//! Pair two fake devices through a running Worker and exchange E2E messages.
//! Skipped unless `BATON_RELAY_URL` is set (CI: `wrangler dev`, see ci-server.yml).
use baton_cli::{
    device::Device,
    relay::{pair_via_rendezvous, Relay},
};
use baton_core::pb;

fn clip(text: &str) -> pb::Payload {
    pb::Payload {
        kind: Some(pb::payload::Kind::Clip(pb::Clip {
            clip_id: "c1".into(),
            body: Some(pb::clip::Body::Text(text.into())),
            char_count: text.len() as u32,
            expires_at_ms: 0,
        })),
    }
}

#[tokio::test]
async fn pair_and_exchange_through_worker() {
    let Ok(base) = std::env::var("BATON_RELAY_URL") else {
        eprintln!("BATON_RELAY_URL not set; skipping");
        return;
    };
    let phone = Device::generate("phone").unwrap();
    let desk = Device::generate("desk").unwrap();
    let (pr, dr) = (Relay::new(&base, &phone), Relay::new(&base, &desk));
    pr.register().await.unwrap();
    dr.register().await.unwrap();

    let rid = "e2e-rendezvous-".to_string() + &hex::encode(rand_bytes());
    let psk = [9u8; 32];
    let desk_pub = desk.x25519.public_bytes();
    let (b1, b2, r1, r2) = (base.clone(), base.clone(), rid.clone(), rid.clone());
    let phone_ref = &phone;
    let desk_ref = &desk;
    let dp = desk_pub.clone();
    let (a, b) = tokio::join!(
        pair_via_rendezvous(&b1, &r1, phone_ref, true, &dp, &psk),
        pair_via_rendezvous(&b2, &r2, desk_ref, false, &[], &psk),
    );
    let ((phone_est, desk_as_peer), (desk_est, phone_as_peer)) = (a.unwrap(), b.unwrap());
    assert_eq!(phone_est.sas(), desk_est.sas(), "SAS must match");
    assert_eq!(desk_as_peer.id, desk.id());

    pr.grant(&desk_as_peer).await.unwrap();
    dr.grant(&phone_as_peer).await.unwrap();

    let cont = pb::Payload {
        kind: Some(pb::payload::Kind::Continue(pb::Continue {
            item_id: "i1".into(),
            url: "https://secret.example/article".into(),
            title: "Secret Title".into(),
            ..Default::default()
        })),
    };
    dr.send(&phone_as_peer, &cont, pb::PushHint::None)
        .await
        .unwrap();
    pr.send(&desk_as_peer, &clip("copied text"), pb::PushHint::None)
        .await
        .unwrap();

    assert_eq!(
        pr.receive(std::slice::from_ref(&desk_as_peer))
            .await
            .unwrap(),
        vec![cont]
    );
    assert_eq!(
        dr.receive(std::slice::from_ref(&phone_as_peer))
            .await
            .unwrap(),
        vec![clip("copied text")]
    );

    // An unpaired device is rejected by the Worker.
    let stranger = Device::generate("stranger").unwrap();
    let sr = Relay::new(&base, &stranger);
    sr.register().await.unwrap();
    assert!(sr
        .send(&desk_as_peer.clone(), &clip("x"), pb::PushHint::None)
        .await
        .is_err());
}

fn rand_bytes() -> [u8; 8] {
    use rand::RngCore;
    let mut b = [0u8; 8];
    rand::rngs::OsRng.fill_bytes(&mut b);
    b
}
