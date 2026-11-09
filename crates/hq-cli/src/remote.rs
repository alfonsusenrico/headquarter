//! `hq remote` (add-remote-gateway D9): pair a phone or another computer's app with a
//! code, list and remove paired devices. Remote control runs while a device is paired,
//! with no switch (remote-follows-paired-phones D3, add-remote-desktop-client D1).

use std::io::IsTerminal;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use hq_client::{Client, expect_ok};
use hq_daemon::paths::HostPaths;
use hq_protocol::{RemoteDevice, Request, Response};

#[derive(Subcommand)]
pub enum RemoteCommand {
    /// Show whether remote control runs and how the computer is reached.
    Status,
    /// Show a code that pairs one phone (QR) or one computer's app (pasted into Connect to
    /// a Host); it works once, for 2 minutes.
    Pair,
    /// List paired phones and computers.
    Devices {
        #[arg(long)]
        json: bool,
    },
    /// Remove a paired device by name or id; it is disconnected at once, and remote control
    /// stops with the last one.
    Remove { device: String },
}

pub fn run(paths: &HostPaths, command: RemoteCommand) -> Result<ExitCode> {
    match command {
        RemoteCommand::Status => {
            let mut client = Client::connect_or_start(paths)?;
            print!(
                "{}",
                status_text(&client.request(&Request::RemoteStatus {})?)
            );
            Ok(ExitCode::SUCCESS)
        }
        RemoteCommand::Pair => pair(paths),
        RemoteCommand::Devices { json } => {
            let mut client = Client::connect_or_start(paths)?;
            let Response::RemoteDevices { devices } =
                expect_ok(client.request(&Request::RemoteDevices {})?)?
            else {
                bail!("unexpected answer from the host");
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&devices)?);
            } else {
                print!("{}", devices_text(&devices, hq_signals::now_ms()));
            }
            Ok(ExitCode::SUCCESS)
        }
        RemoteCommand::Remove { device } => {
            let mut client = Client::connect_or_start(paths)?;
            expect_ok(client.request(&Request::RemoteRemove {
                device: device.clone(),
            })?)?;
            println!("Removed {device}.");
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn pair(paths: &HostPaths) -> Result<ExitCode> {
    let mut client = Client::connect_or_start(paths)?;
    let Response::RemotePairing {
        code,
        expires_in_secs,
    } = expect_ok(client.request(&Request::RemotePair {})?)?
    else {
        bail!("unexpected answer from the host");
    };
    let qr = qrcode::QrCode::new(code.as_bytes()).context("cannot draw the QR code")?;
    let (cols, rows) = qr_size(qr.width());
    match crossterm::terminal::size() {
        Ok((width, height)) if width < cols || height < rows + 4 => println!(
            "Make this window at least {cols} columns by {} rows to show the QR code, or use \
             Headquarter ▸ Remote Control… in the app.\n",
            rows + 4
        ),
        _ => {
            let drawn = qr
                .render::<qrcode::render::unicode::Dense1x2>()
                .quiet_zone(true)
                .build();
            let coloured =
                std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
            println!("{}", paint_qr(&drawn, coloured));
        }
    }
    println!(
        "Scan the QR code with Headquarter on your phone, or paste the code into Connect to a \
         Host in the Headquarter app on another computer:\n{code}\n"
    );
    println!("Get the phone app: {PHONE_APP_LINK}\n");
    println!(
        "Waiting for a device; the code works once, for {} minutes.",
        expires_in_secs.div_ceil(60)
    );
    loop {
        match client.next_response()? {
            Response::RemotePairUsed { device, .. } => {
                println!("Paired {device}.");
                return Ok(ExitCode::SUCCESS);
            }
            Response::RemotePairExpired { reason } => {
                eprintln!("hq: no phone paired: {reason}");
                return Ok(ExitCode::FAILURE);
            }
            _ => continue,
        }
    }
}

/// Columns and rows a QR code of `modules` takes in a terminal: two rows of modules per
/// line, with a 4-module quiet zone around it.
/// The latest stable phone app, as the Remote Control window links it.
const PHONE_APP_LINK: &str = "https://github.com/alfonsusenrico/headquarter/releases/latest/download/headquarter-android.apk";

/// The QR code dark on light whatever the terminal's theme: on a dark theme the blocks would
/// otherwise draw light on dark, which many camera apps cannot read (fix-remote-polish D2).
fn paint_qr(drawn: &str, coloured: bool) -> String {
    if !coloured {
        return drawn.to_owned();
    }
    drawn
        .lines()
        .map(|line| format!("\x1b[30;107m{line}\x1b[0m"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn qr_size(modules: usize) -> (u16, u16) {
    let side = modules + 8;
    (side as u16, side.div_ceil(2) as u16)
}

fn status_text(status: &Response) -> String {
    let Response::RemoteStatus {
        enabled,
        running,
        endpoint,
        relay,
        problem,
    } = status
    else {
        return "unexpected answer from the host\n".to_owned();
    };
    let mut text = String::new();
    match (enabled, running) {
        (_, true) => text.push_str("Remote control is on.\n"),
        (true, false) => text.push_str("A device is paired but remote control is not running.\n"),
        (false, false) => {
            text.push_str(
                "Remote control is off: no device is paired. `hq remote pair` pairs one.\n",
            );
        }
    }
    if let Some(problem) = problem {
        text.push_str(&format!("Problem: {problem}\n"));
    }
    if *running {
        match relay {
            Some(relay) => text.push_str(&format!("Reachable through {relay}\n")),
            None => text.push_str("Not connected to a relay yet; devices on this network only.\n"),
        }
        if let Some(endpoint) = endpoint {
            text.push_str(&format!("Computer key: {endpoint}\n"));
        }
    }
    text
}

fn devices_text(devices: &[RemoteDevice], now_ms: u64) -> String {
    if devices.is_empty() {
        return "No paired devices. `hq remote pair` pairs a phone or a computer.\n".to_owned();
    }
    let width = devices
        .iter()
        .map(|device| device.name.chars().count())
        .max()
        .unwrap_or(0);
    let mut text = String::new();
    for device in devices {
        let seen = match (device.connected, device.last_seen_at) {
            (true, _) => "connected now".to_owned(),
            (false, Some(at)) => format!(
                "seen {} ago",
                hq_protocol::sidebar::age(now_ms.saturating_sub(at.max(0) as u64))
            ),
            (false, None) => "never connected".to_owned(),
        };
        let id: String = device.id.chars().take(10).collect();
        text.push_str(&format!(
            "  {:width$}  {:<7}  {seen:<16}  {id}\n",
            device.name,
            device.kind.as_str()
        ));
    }
    text
}

#[cfg(test)]
mod tests {
    use hq_protocol::DeviceKind;

    use super::*;

    #[test]
    fn the_qr_code_needs_its_size() {
        // A 45-module code (an hqp2 code with a relay) needs 53 columns and 27 rows.
        assert_eq!(qr_size(45), (53, 27));
        assert_eq!(qr_size(57), (65, 33));
    }

    #[test]
    fn the_qr_code_is_dark_on_light_in_a_terminal() {
        let drawn = "\u{2588}\u{2580}\n \u{2584}";
        assert_eq!(
            paint_qr(drawn, true),
            "\x1b[30;107m\u{2588}\u{2580}\x1b[0m\n\x1b[30;107m \u{2584}\x1b[0m"
        );
        assert_eq!(
            paint_qr(drawn, false),
            drawn,
            "plain when piped or NO_COLOR"
        );
    }

    #[test]
    fn devices_read_plainly() {
        let devices = [
            RemoteDevice {
                id: "abcdef0123456789".into(),
                name: "Pixel 8".into(),
                kind: DeviceKind::Phone,
                paired_at: 0,
                last_seen_at: Some(0),
                connected: true,
            },
            RemoteDevice {
                id: "0011223344556677".into(),
                name: "Old tablet".into(),
                kind: DeviceKind::Desktop,
                paired_at: 0,
                last_seen_at: Some(1_000),
                connected: false,
            },
        ];
        let text = devices_text(&devices, 3 * 3_600_000);
        assert!(
            text.contains("Pixel 8     phone    connected now"),
            "{text}"
        );
        assert!(text.contains("Old tablet  desktop  seen 2h ago"), "{text}");
        assert!(text.contains("abcdef0123"), "{text}");
        assert!(devices_text(&[], 0).contains("hq remote pair"));
    }

    #[test]
    fn status_says_how_the_computer_is_reached() {
        let on = status_text(&Response::RemoteStatus {
            enabled: true,
            running: true,
            endpoint: Some("key".into()),
            relay: Some("https://relay.example.com/".into()),
            problem: None,
        });
        assert!(on.starts_with("Remote control is on."), "{on}");
        assert!(
            on.contains("Reachable through https://relay.example.com/"),
            "{on}"
        );
        let broken = status_text(&Response::RemoteStatus {
            enabled: true,
            running: false,
            endpoint: None,
            relay: None,
            problem: Some("no network".into()),
        });
        assert!(
            broken.contains("not running") && broken.contains("no network"),
            "{broken}"
        );
    }
}
