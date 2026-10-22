//! Observes an agent's terminal output for status signals without changing it.
//!
//! The caller writes every byte it feeds here to the owner's terminal unchanged; the
//! scanner only reports what it recognised. Signal text (titles, notification bodies)
//! is kept in memory for classification and must never be persisted.

use crate::event::NeedsReason;

/// Reply announcing support for the Program Status Protocol (OSC 7501, draft 0.4).
pub const PROGRAM_STATUS_SUPPORT_REPLY: &[u8] =
    b"\x1b]7501;?:states=idle,working,done,blocked,error:kinds=permission,question,auth\x1b\\";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalSignal {
    Title(String),
    Notification { title: Option<String>, body: String },
    Progress { state: u8, percent: Option<u8> },
    ProgramStatus(ProgramStatus),
    ProgramStatusQuery,
    Bell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramState {
    Idle,
    Working,
    Done,
    Blocked,
    Error,
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramStatus {
    pub state: ProgramState,
    pub kind: Option<NeedsReason>,
    /// Record path; `None` is the program's root record. Child records describe
    /// sub-tasks and do not drive the session status.
    pub id: Option<String>,
}

pub struct Scanner {
    parser: vte::Parser,
    collector: Collector,
}

impl Default for Scanner {
    fn default() -> Self {
        Scanner {
            parser: vte::Parser::new(),
            collector: Collector::default(),
        }
    }
}

impl Scanner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses the next chunk of output. Sequences split across chunks are recognised
    /// once their final byte arrives.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<TerminalSignal> {
        self.parser.advance(&mut self.collector, bytes);
        std::mem::take(&mut self.collector.signals)
    }

    /// Writes `chunk` to `out` byte for byte and returns the signals it contained.
    pub fn pass_through(
        &mut self,
        chunk: &[u8],
        out: &mut impl std::io::Write,
    ) -> std::io::Result<Vec<TerminalSignal>> {
        out.write_all(chunk)?;
        Ok(self.feed(chunk))
    }
}

#[derive(Default)]
struct Collector {
    signals: Vec<TerminalSignal>,
}

impl vte::Perform for Collector {
    fn execute(&mut self, byte: u8) {
        if byte == 0x07 {
            self.signals.push(TerminalSignal::Bell);
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if let Some(signal) = classify_osc(params) {
            self.signals.push(signal);
        }
    }
}

fn text(parts: &[&[u8]]) -> String {
    let joined: Vec<u8> = parts.join(&b';');
    String::from_utf8_lossy(&joined).into_owned()
}

fn classify_osc(params: &[&[u8]]) -> Option<TerminalSignal> {
    let (&code, rest) = params.split_first()?;
    match code {
        b"0" | b"2" => Some(TerminalSignal::Title(text(rest))),
        b"9" => match rest {
            [b"4", state, more @ ..] => Some(TerminalSignal::Progress {
                state: parse_number(state)?,
                percent: more.first().and_then(|value| parse_number(value)),
            }),
            _ => Some(TerminalSignal::Notification {
                title: None,
                body: text(rest),
            }),
        },
        b"777" => match rest {
            [b"notify", title, body @ ..] => Some(TerminalSignal::Notification {
                title: Some(text(&[title])),
                body: text(body),
            }),
            _ => None,
        },
        // Kitty desktop notifications: OSC 99 ; metadata ; payload
        b"99" => Some(TerminalSignal::Notification {
            title: None,
            body: text(rest.get(1..).unwrap_or_default()),
        }),
        b"7501" => classify_program_status(rest),
        _ => None,
    }
}

fn parse_number(bytes: &[u8]) -> Option<u8> {
    std::str::from_utf8(bytes).ok()?.trim().parse().ok()
}

fn classify_program_status(rest: &[&[u8]]) -> Option<TerminalSignal> {
    let body = text(rest);
    if body.trim() == "?" {
        return Some(TerminalSignal::ProgramStatusQuery);
    }
    let mut state = None;
    let mut kind = None;
    let mut id = None;
    for pair in body.split(':') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "state" => state = Some(value.to_owned()),
            "kind" => kind = Some(value.to_owned()),
            "id" => id = Some(value.to_owned()),
            _ => {}
        }
    }
    // An unrecognised state discards the whole report (spec 0.4).
    let state = match state?.as_str() {
        "idle" => ProgramState::Idle,
        "working" => ProgramState::Working,
        "done" => ProgramState::Done,
        "blocked" => ProgramState::Blocked,
        "error" => ProgramState::Error,
        "clear" => ProgramState::Clear,
        _ => return None,
    };
    let kind = match (state, kind.as_deref()) {
        (ProgramState::Blocked, Some("permission")) => Some(NeedsReason::Permission),
        (ProgramState::Blocked, Some("question")) => Some(NeedsReason::Question),
        (ProgramState::Blocked, Some("auth")) => Some(NeedsReason::Auth),
        (ProgramState::Blocked, _) => Some(NeedsReason::Unknown),
        _ => None,
    };
    Some(TerminalSignal::ProgramStatus(ProgramStatus {
        state,
        kind,
        id,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(bytes: &[u8]) -> Vec<TerminalSignal> {
        Scanner::new().feed(bytes)
    }

    #[test]
    fn titles_with_both_terminators() {
        assert_eq!(
            scan(b"\x1b]0;one\x07\x1b]2;two\x1b\\"),
            vec![
                TerminalSignal::Title("one".into()),
                TerminalSignal::Title("two".into())
            ]
        );
    }

    #[test]
    fn notification_sequences() {
        assert_eq!(
            scan(b"\x1b]9;Approval requested\x07"),
            vec![TerminalSignal::Notification {
                title: None,
                body: "Approval requested".into()
            }]
        );
        assert_eq!(
            scan(b"\x1b]777;notify;Claude;needs you\x07"),
            vec![TerminalSignal::Notification {
                title: Some("Claude".into()),
                body: "needs you".into()
            }]
        );
        assert_eq!(
            scan(b"\x1b]99;i=1;hello\x1b\\"),
            vec![TerminalSignal::Notification {
                title: None,
                body: "hello".into()
            }]
        );
    }

    #[test]
    fn progress_sequence() {
        assert_eq!(
            scan(b"\x1b]9;4;1;40\x07"),
            vec![TerminalSignal::Progress {
                state: 1,
                percent: Some(40)
            }]
        );
    }

    #[test]
    fn program_status_reports_and_query() {
        assert_eq!(
            scan(b"\x1b]7501;?\x1b\\"),
            vec![TerminalSignal::ProgramStatusQuery]
        );
        assert_eq!(
            scan(b"\x1b]7501;state=blocked:kind=permission\x07"),
            vec![TerminalSignal::ProgramStatus(ProgramStatus {
                state: ProgramState::Blocked,
                kind: Some(NeedsReason::Permission),
                id: None
            })]
        );
        assert_eq!(
            scan(b"\x1b]7501; state = working : id=root/task1 :junk\x07"),
            vec![TerminalSignal::ProgramStatus(ProgramStatus {
                state: ProgramState::Working,
                kind: None,
                id: Some("root/task1".into())
            })]
        );
    }

    #[test]
    fn program_status_with_unknown_state_is_discarded() {
        assert!(scan(b"\x1b]7501;state=sleeping\x07").is_empty());
        assert!(scan(b"\x1b]7501;kind=permission\x07").is_empty());
    }

    #[test]
    fn bell_is_reported() {
        assert_eq!(scan(b"ding\x07"), vec![TerminalSignal::Bell]);
    }

    #[test]
    fn unknown_and_plain_output_yield_nothing() {
        assert!(scan(b"\x1b]1337;SetMark\x07\x1b[31mred\x1b[0m plain text\r\n").is_empty());
    }

    #[test]
    fn pass_through_leaves_bytes_unchanged() {
        let input: &[u8] =
            b"\x1b[?1049h\x1b]0;\xe2\xa0\x8b proj\x07text\x1b]7501;state=working\x1b\\\x1b]1337;x\x07\xff\x00end";
        let mut scanner = Scanner::new();
        let mut out = Vec::new();
        let mut signals = Vec::new();
        for chunk in input.chunks(5) {
            signals.extend(scanner.pass_through(chunk, &mut out).unwrap());
        }
        assert_eq!(out, input);
        assert_eq!(signals.len(), 2);
    }

    #[test]
    fn sequences_split_across_chunks_are_recognised() {
        let mut scanner = Scanner::new();
        assert!(scanner.feed(b"\x1b]7501;sta").is_empty());
        assert_eq!(
            scanner.feed(b"te=done\x07"),
            vec![TerminalSignal::ProgramStatus(ProgramStatus {
                state: ProgramState::Done,
                kind: None,
                id: None
            })]
        );
    }
}
