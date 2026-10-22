//! Full-screen attach to a hosted session from the current terminal.
//!
//! Every key goes to the agent except the detach sequence: Ctrl-\ then `d`.
//! Ctrl-\ twice sends one Ctrl-\ to the agent.

use std::io::{IsTerminal, Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow, bail};
use hq_protocol::{Frame, Request, Response, SessionInfo, read_frame, write_frame};

use hq_client::Client;

const DETACH_PREFIX: u8 = 0x1c;
const DETACH_KEY: u8 = b'd';

/// Undoes terminal modes an agent may have left on when the view goes away: the
/// alternate screen, hidden cursor, mouse reporting, bracketed paste, and colours.
const RESTORE_TERMINAL: &[u8] =
    b"\x1b[?1049l\x1b[?25h\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?2004l\x1b[0m\r\n";

pub enum Outcome {
    Detached,
    Ended(Option<u32>),
}

pub fn attach(mut client: Client, session: &SessionInfo) -> Result<Outcome> {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        bail!("attaching needs a terminal; use `hq ls` to see sessions instead");
    }
    let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
    match client.request(&Request::Attach {
        session: session.id.clone(),
        cols,
        rows,
    })? {
        Response::Attached { .. } => {}
        Response::Error { message, .. } => bail!(message),
        other => bail!("unexpected answer to attach: {other:?}"),
    }

    let writer = Arc::new(Mutex::new(client.writer.try_clone()?));
    crossterm::terminal::enable_raw_mode()?;
    let _restore = RestoreOnDrop;

    spawn_input(Arc::clone(&writer));
    spawn_resize(Arc::clone(&writer));

    let mut stdout = std::io::stdout();
    loop {
        match read_frame(&mut client.reader).map_err(|error| anyhow!(error))? {
            Some(Frame::Output(bytes)) => {
                stdout.write_all(&bytes)?;
                stdout.flush()?;
            }
            Some(Frame::Control(bytes)) => match Response::parse(&bytes).map_err(|e| anyhow!(e))? {
                Response::Detached => return Ok(Outcome::Detached),
                Response::Ended { exit_code, .. } => return Ok(Outcome::Ended(exit_code)),
                Response::Error { message, .. } => bail!(message),
                _ => {}
            },
            Some(Frame::Input(_)) => {}
            None => bail!("the Headquarter host closed the connection"),
        }
    }
}

fn send(writer: &Mutex<UnixStream>, frame: &Frame) -> bool {
    let Ok(mut stream) = writer.lock() else {
        return false;
    };
    write_frame(&mut *stream, frame).is_ok()
}

fn spawn_input(writer: Arc<Mutex<UnixStream>>) {
    std::thread::spawn(move || {
        let mut stdin = std::io::stdin();
        let mut buffer = [0u8; 4096];
        let mut pending_prefix = false;
        loop {
            let count = match stdin.read(&mut buffer) {
                Ok(0) | Err(_) => return,
                Ok(count) => count,
            };
            let (keys, detach) = split_detach(&buffer[..count], &mut pending_prefix);
            if !keys.is_empty() && !send(&writer, &Frame::Input(keys)) {
                return;
            }
            if detach {
                send(&writer, &Frame::request(&Request::Detach {}));
                return;
            }
        }
    });
}

/// Separates owner keys from the detach sequence, which may span reads.
fn split_detach(input: &[u8], pending_prefix: &mut bool) -> (Vec<u8>, bool) {
    let mut keys = Vec::with_capacity(input.len());
    for &byte in input {
        if *pending_prefix {
            *pending_prefix = false;
            match byte {
                DETACH_KEY => return (keys, true),
                DETACH_PREFIX => keys.push(DETACH_PREFIX),
                other => keys.extend([DETACH_PREFIX, other]),
            }
        } else if byte == DETACH_PREFIX {
            *pending_prefix = true;
        } else {
            keys.push(byte);
        }
    }
    (keys, false)
}

fn spawn_resize(writer: Arc<Mutex<UnixStream>>) {
    std::thread::spawn(move || {
        let Ok(mut signals) = signal_hook::iterator::Signals::new([signal_hook::consts::SIGWINCH])
        else {
            return;
        };
        for _ in signals.forever() {
            let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
            if !send(&writer, &Frame::request(&Request::Resize { cols, rows })) {
                return;
            }
        }
    });
}

struct RestoreOnDrop;

impl Drop for RestoreOnDrop {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        let mut stdout = std::io::stdout();
        let _ = stdout.write_all(RESTORE_TERMINAL);
        let _ = stdout.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::split_detach;

    #[test]
    fn plain_keys_pass_through() {
        let mut pending = false;
        assert_eq!(
            split_detach(b"hello\r", &mut pending),
            (b"hello\r".to_vec(), false)
        );
    }

    #[test]
    fn detach_sequence_in_one_read() {
        let mut pending = false;
        assert_eq!(
            split_detach(b"ab\x1cd", &mut pending),
            (b"ab".to_vec(), true)
        );
    }

    #[test]
    fn detach_sequence_across_reads() {
        let mut pending = false;
        assert_eq!(split_detach(b"x\x1c", &mut pending), (b"x".to_vec(), false));
        assert!(pending);
        assert_eq!(split_detach(b"d", &mut pending), (vec![], true));
    }

    #[test]
    fn prefix_twice_sends_one_prefix_and_other_keys_are_kept() {
        let mut pending = false;
        assert_eq!(
            split_detach(b"\x1c\x1c\x1cq", &mut pending),
            (vec![0x1c, 0x1c, b'q'], false)
        );
    }
}
