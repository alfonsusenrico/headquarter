//! A stand-in phone for trying remote control from a terminal until the Android app
//! exists: `cargo run -p hq-remote --example phone -- '<pairing code>' [name]` pairs with
//! the code `hq remote pair` shows, lists the sessions, then stays connected until
//! Ctrl-C. Each run is a new phone (a fresh key).

use hq_protocol::{Frame, PROTOCOL_VERSION, Request, Response};
use hq_remote::{ALPN, PairingCode, Relays};

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let code = args.next().expect("usage: phone '<pairing code>' [name]");
    let name = args.next().unwrap_or_else(|| "Terminal phone".to_owned());
    let code = PairingCode::parse(&code).unwrap_or_else(|error| panic!("{error}"));
    let relays = match &code.relay {
        Some(url) => Relays::Custom(vec![url.clone()]),
        None => Relays::Disabled,
    };
    let endpoint = hq_remote::bind(iroh::SecretKey::generate(), &relays, &[])
        .await
        .expect("endpoint");
    let started = std::time::Instant::now();
    let connection = endpoint.connect(code.addr(), ALPN).await.expect("connect");
    let (mut send, mut recv) = connection.open_bi().await.expect("stream");
    let request = |request: &Request| Frame::request(request);
    hq_remote::write_frame(
        &mut send,
        &request(&Request::Pair {
            secret: code.secret.clone(),
            name: name.clone(),
        }),
    )
    .await
    .expect("send");
    match next(&mut recv).await {
        Some(Response::Paired { host }) => println!(
            "Paired with {host} as {name} in {} ms.",
            started.elapsed().as_millis()
        ),
        other => panic!("not paired: {other:?}"),
    }
    hq_remote::write_frame(
        &mut send,
        &request(&Request::Hello {
            protocol: PROTOCOL_VERSION,
        }),
    )
    .await
    .expect("send");
    next(&mut recv).await;
    hq_remote::write_frame(&mut send, &request(&Request::List {}))
        .await
        .expect("send");
    if let Some(Response::Sessions { sessions, .. }) = next(&mut recv).await {
        println!("{} sessions:", sessions.len());
        for session in sessions.iter().filter(|session| session.in_sidebar) {
            println!(
                "  {} ({:?})",
                hq_protocol::sidebar::display_title(session),
                session.status
            );
        }
    }
    println!("Connected; Ctrl-C to leave.");
    let _ = tokio::signal::ctrl_c().await;
    connection.close(0u32.into(), b"bye");
    endpoint.close().await;
}

async fn next(recv: &mut iroh::endpoint::RecvStream) -> Option<Response> {
    loop {
        match hq_remote::read_frame(recv).await.ok()?? {
            Frame::Control(bytes) => return Response::parse(&bytes).ok(),
            _ => continue,
        }
    }
}
