use baton_cli::{
    device::{Device, Peer},
    lan::{send_with_fallback, LanLink, LanListener},
};
use baton_core::{crypto, pb};
use prost::Message;
use std::{
    cell::Cell,
    time::{Duration, Instant},
};

/// Pair two devices as if the pairing handshake had run (shared SAS is covered in relay_e2e / core tests).
fn paired() -> (Device, Device, Peer, Peer) {
    let a = Device::generate("a").unwrap();
    let b = Device::generate("b").unwrap();
    let peer = |d: &Device| Peer {
        id: d.id(),
        ed25519_pub: d.ed25519_pub(),
        x25519_pub: d.x25519.public_bytes(),
    };
    let (pa, pb_) = (peer(&a), peer(&b));
    (a, b, pa, pb_)
}

fn env(from: &Device, to: &Peer) -> Vec<u8> {
    let p = pb::Payload {
        kind: Some(pb::payload::Kind::Withdraw(pb::Withdraw {
            item_id: "i".into(),
        })),
    };
    crypto::seal(
        &p,
        &crypto::Header {
            sender: from.id(),
            recipient: to.id,
            sent_at_ms: 1,
            ttl_s: 60,
            push: pb::PushHint::None,
            collapse_key: [0; 16],
        },
        &from.x25519,
        &to.x25519_pub,
    )
    .unwrap()
    .encode_to_vec()
}

#[tokio::test]
async fn lan_link_carries_envelope_and_rejects_unknown_peer() {
    let (a, b, pa, pb_) = paired();
    let listener = LanListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let b_peers = vec![pa.clone()];
    let server = tokio::spawn(async move {
        let (mut link, who) = listener.accept(&b, &b_peers).await.unwrap();
        assert_eq!(who.id, pa.id);
        let got = link.recv().await.unwrap();
        // A second connection from a stranger is dropped.
        assert!(listener.accept(&b, &b_peers).await.is_err());
        got
    });

    let mut link = LanLink::connect(&addr, &a, &pb_).await.unwrap();
    let sent = env(&a, &pb_);
    link.send(&sent).await.unwrap();

    let stranger = Device::generate("mallory").unwrap();
    assert!(LanLink::connect(&addr, &stranger, &pb_).await.is_err());

    assert_eq!(server.await.unwrap(), sent);
}

#[tokio::test]
async fn killed_lan_link_falls_back_to_relay_within_two_seconds() {
    let (a, b, pa, pb_) = paired();
    let listener = LanListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let peers = vec![pa];
    let server = tokio::spawn(async move {
        let (mut link, _) = listener.accept(&b, &peers).await.unwrap();
        let _ = link.recv().await; // first message acked
                                   // then the "network" dies: listener and link are dropped here
    });

    let mut link = LanLink::connect(&addr, &a, &pb_).await.unwrap();
    let relayed = Cell::new(0);
    let e = env(&a, &pb_);
    assert!(send_with_fallback(Some(&mut link), &e, || async {
        relayed.set(relayed.get() + 1);
        Ok(())
    })
    .await
    .unwrap());
    server.await.unwrap();

    let t = Instant::now();
    let via_lan = send_with_fallback(Some(&mut link), &e, || async {
        relayed.set(relayed.get() + 1);
        Ok(())
    })
    .await
    .unwrap();
    assert!(!via_lan, "dead link must not report LAN delivery");
    assert_eq!(relayed.get(), 1);
    assert!(
        t.elapsed() < Duration::from_secs(2),
        "fallback took {:?}",
        t.elapsed()
    );
}
