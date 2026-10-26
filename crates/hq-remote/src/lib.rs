//! Remote control's connection between a phone and the host (add-remote-gateway): the
//! iroh endpoint both sides use, the pairing code the owner shows, and today's
//! `hq-protocol` frames carried over a QUIC stream. Shared by the host's gateway and,
//! later, the phone's core.

use std::fmt;
use std::net::SocketAddr;

use hq_protocol::{Frame, FrameError, MAX_PAYLOAD};
use iroh::endpoint::presets;
use iroh::{Endpoint, EndpointAddr, PublicKey, RelayMap, RelayMode, RelayUrl, SecretKey};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// The application protocol both sides speak over iroh.
pub const ALPN: &[u8] = b"hq-remote/1";

/// How long a pairing code works (D4).
pub const PAIRING_VALID_SECS: u64 = 120;

/// Which relays the endpoint uses (D7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Relays {
    /// n0's public relays.
    Default,
    /// The owner's own relays instead.
    Custom(Vec<RelayUrl>),
    /// None: direct addresses only (tests, `HQ_REMOTE_RELAYS=none`).
    Disabled,
}

impl Relays {
    /// From the settings file's `[remote] relays`; `HQ_REMOTE_RELAYS=none` turns relays off
    /// for tests.
    pub fn from_settings(relays: &[String]) -> Result<Relays, String> {
        if std::env::var("HQ_REMOTE_RELAYS").is_ok_and(|value| value == "none") {
            return Ok(Relays::Disabled);
        }
        if relays.is_empty() {
            return Ok(Relays::Default);
        }
        relays
            .iter()
            .map(|url| {
                url.parse::<RelayUrl>()
                    .map_err(|error| format!("relay {url}: {error}"))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Relays::Custom)
    }

    fn mode(&self) -> RelayMode {
        match self {
            Relays::Default => RelayMode::Default,
            Relays::Custom(urls) => RelayMode::Custom(RelayMap::from_iter(urls.iter().cloned())),
            Relays::Disabled => RelayMode::Disabled,
        }
    }
}

/// Binds an endpoint with `secret`, using `relays`. The host passes [`ALPN`] in `accept`
/// to take phone connections; a phone passes nothing. No address lookup service is used:
/// a pairing code carries what a phone needs to dial (D7).
pub async fn bind(
    secret: SecretKey,
    relays: &Relays,
    accept: &[&[u8]],
) -> Result<Endpoint, String> {
    Endpoint::builder(presets::N0)
        .clear_address_lookup()
        .secret_key(secret)
        .relay_mode(relays.mode())
        .alpns(accept.iter().map(|alpn| alpn.to_vec()).collect())
        .bind()
        .await
        .map_err(|error| error.to_string())
}

/// What the owner shows to pair a phone (D4, D13):
/// `hqp2|<endpoint id, base32>|<relay url>|<addr,...>|<secret, base32>`. Addresses are left
/// out when a relay is known, so the QR code stays small enough for a phone camera.
#[derive(Clone, PartialEq, Eq)]
pub struct PairingCode {
    pub host: PublicKey,
    pub relay: Option<RelayUrl>,
    pub addrs: Vec<SocketAddr>,
    /// 128 random bits, as 26 lowercase base32 characters.
    pub secret: String,
}

impl fmt::Debug for PairingCode {
    // The secret stays out of logs and panics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PairingCode")
            .field("host", &self.host)
            .field("relay", &self.relay)
            .field("addrs", &self.addrs)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeError(pub &'static str);

impl fmt::Display for CodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl PairingCode {
    /// A code for `host` with a fresh secret; `addrs` are kept only without a relay.
    pub fn new(host: PublicKey, relay: Option<RelayUrl>, addrs: Vec<SocketAddr>) -> PairingCode {
        let addrs = if relay.is_some() { Vec::new() } else { addrs };
        PairingCode {
            host,
            relay,
            addrs,
            secret: new_secret(),
        }
    }

    pub fn format(&self) -> String {
        let relay = self
            .relay
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default();
        let addrs: Vec<String> = self.addrs.iter().map(ToString::to_string).collect();
        format!(
            "hqp2|{}|{relay}|{}|{}",
            base32(self.host.as_bytes()),
            addrs.join(","),
            self.secret
        )
    }

    pub fn parse(text: &str) -> Result<PairingCode, CodeError> {
        let parts: Vec<&str> = text.trim().split('|').collect();
        let [kind, host, relay, addrs, secret] = parts.as_slice() else {
            return Err(CodeError("not a Headquarter pairing code"));
        };
        if *kind != "hqp2" {
            return Err(CodeError("not a Headquarter pairing code"));
        }
        let host = unbase32(host)
            .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
            .and_then(|bytes| PublicKey::from_bytes(&bytes).ok())
            .ok_or(CodeError("the code's computer key is not valid"))?;
        let relay = match *relay {
            "" => None,
            url => Some(
                url.parse()
                    .map_err(|_| CodeError("the code's relay is not a URL"))?,
            ),
        };
        let addrs = addrs
            .split(',')
            .filter(|addr| !addr.is_empty())
            .map(|addr| {
                addr.parse()
                    .map_err(|_| CodeError("the code's address is not valid"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if secret.len() != 26 || unbase32(secret).is_none_or(|bytes| bytes.len() != 16) {
            return Err(CodeError("the code's secret is not valid"));
        }
        Ok(PairingCode {
            host,
            relay,
            addrs,
            secret: secret.to_ascii_lowercase(),
        })
    }

    /// Where to dial the host.
    pub fn addr(&self) -> EndpointAddr {
        let mut addr = EndpointAddr::new(self.host);
        if let Some(relay) = self.relay.clone() {
            addr = addr.with_relay_url(relay);
        }
        for ip in &self.addrs {
            addr = addr.with_ip_addr(*ip);
        }
        addr
    }
}

fn new_secret() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("the operating system provides random bytes");
    base32(&bytes)
}

const BASE32: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";

/// RFC 4648 base32, lowercase, without padding.
fn base32(bytes: &[u8]) -> String {
    let mut out = String::new();
    let (mut buffer, mut bits) = (0u32, 0u32);
    for &byte in bytes {
        buffer = (buffer << 8) | u32::from(byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(BASE32[((buffer >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(BASE32[((buffer << (5 - bits)) & 31) as usize] as char);
    }
    out
}

fn unbase32(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let (mut buffer, mut bits) = (0u32, 0u32);
    for c in text.bytes() {
        let value = BASE32
            .iter()
            .position(|&known| known == c.to_ascii_lowercase())? as u32;
        buffer = (buffer << 5) | value;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    Some(out)
}

/// Compares two secrets in time that does not depend on where they differ.
pub fn same_secret(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

/// Reads one frame; `None` at a clean end of the stream.
pub async fn read_frame(input: &mut (impl AsyncRead + Unpin)) -> Result<Option<Frame>, FrameError> {
    let mut header = [0u8; 5];
    match input.read_exact(&mut header).await {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(FrameError::Io(error)),
    }
    let length = u32::from_be_bytes([header[1], header[2], header[3], header[4]]) as usize;
    if length > MAX_PAYLOAD {
        return Err(FrameError::TooLarge(length));
    }
    let mut frame = Vec::with_capacity(5 + length);
    frame.extend_from_slice(&header);
    frame.resize(5 + length, 0);
    input
        .read_exact(&mut frame[5..])
        .await
        .map_err(FrameError::Io)?;
    // The synchronous reader checks the kind and builds the frame.
    hq_protocol::read_frame(&mut frame.as_slice())
}

/// Writes one frame.
pub async fn write_frame(
    output: &mut (impl AsyncWrite + Unpin),
    frame: &Frame,
) -> Result<(), FrameError> {
    let mut bytes = Vec::new();
    hq_protocol::write_frame(&mut bytes, frame)?;
    output.write_all(&bytes).await.map_err(FrameError::Io)?;
    output.flush().await.map_err(FrameError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host() -> PublicKey {
        SecretKey::from_bytes(&[7; 32]).public()
    }

    #[test]
    fn codes_round_trip_short_without_showing_the_secret_in_debug() {
        let addrs = vec![
            "192.168.1.5:41000".parse().unwrap(),
            "[::1]:41001".parse().unwrap(),
        ];
        let code = PairingCode::new(
            host(),
            Some("https://aps1-1.relay.n0.iroh.link./".parse().unwrap()),
            addrs.clone(),
        );
        assert!(code.addrs.is_empty(), "the relay finds direct paths");
        let text = code.format();
        assert!(text.starts_with("hqp2|"));
        assert!(text.len() < 130, "{} characters: {text}", text.len());
        assert_eq!(PairingCode::parse(&text).unwrap(), code);
        assert!(!format!("{code:?}").contains(&code.secret));
        assert_eq!(code.secret.len(), 26);
        assert_ne!(code.secret, PairingCode::new(host(), None, vec![]).secret);
        let local = PairingCode::new(host(), None, addrs);
        assert_eq!(local.addrs.len(), 2, "kept without a relay");
        assert_eq!(PairingCode::parse(&local.format()).unwrap(), local);
    }

    #[test]
    fn base32_round_trips() {
        for bytes in [&b""[..], b"f", b"fo", b"foo", b"foob", b"fooba", b"foobar"] {
            assert_eq!(unbase32(&base32(bytes)).unwrap(), bytes);
        }
        assert_eq!(base32(b"foobar"), "mzxw6ytboi");
        assert_eq!(unbase32("1"), None, "not in the alphabet");
    }

    #[test]
    fn malformed_codes_are_refused() {
        let good = PairingCode::new(host(), None, vec![]).format();
        for bad in [
            "",
            "hqs1|x|y|z",
            &good.replacen("hqp2", "hqp1", 1),
            &good.replacen(&base32(host().as_bytes()), "not-a-key", 1),
            &format!("{}1", &good[..good.len() - 1]),
            &format!("{good}a"),
            "hqp2|a|b|c|d|e",
        ] {
            assert!(PairingCode::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn secrets_compare_by_content() {
        assert!(same_secret("abcd", "abcd"));
        assert!(!same_secret("abcd", "abce"));
        assert!(!same_secret("abcd", "abc"));
    }

    #[tokio::test]
    async fn frames_cross_a_stream() {
        let frames = [
            Frame::Control(br#"{"type":"list"}"#.to_vec()),
            Frame::Input(b"typed\r".to_vec()),
            Frame::Output(vec![0x1b, b'[', b'H']),
        ];
        let mut bytes = Vec::new();
        for frame in &frames {
            write_frame(&mut bytes, frame).await.unwrap();
        }
        let mut input = bytes.as_slice();
        for frame in &frames {
            assert_eq!(read_frame(&mut input).await.unwrap().as_ref(), Some(frame));
        }
        assert_eq!(read_frame(&mut input).await.unwrap(), None, "clean end");
        let mut too_big = vec![0u8];
        too_big.extend_from_slice(&u32::try_from(MAX_PAYLOAD + 1).unwrap().to_be_bytes());
        assert!(matches!(
            read_frame(&mut too_big.as_slice()).await,
            Err(FrameError::TooLarge(_))
        ));
    }

    #[test]
    fn relays_from_settings() {
        assert_eq!(Relays::from_settings(&[]).unwrap(), Relays::Default);
        assert!(matches!(
            Relays::from_settings(&["https://relay.example.com".into()]).unwrap(),
            Relays::Custom(urls) if urls.len() == 1
        ));
        assert!(Relays::from_settings(&["not a url".into()]).is_err());
    }
}
