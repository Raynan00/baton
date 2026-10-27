//! mDNS needs multicast; opt in with `BATON_TEST_MDNS=1` (CI runners and most LANs allow it).
use baton_cli::{device::Device, mdns};
use std::time::Duration;

#[test]
fn advertised_device_is_found_by_hint() {
    if std::env::var("BATON_TEST_MDNS").is_err() {
        eprintln!("BATON_TEST_MDNS not set; skipping");
        return;
    }
    let d = Device::generate("desk").unwrap();
    let _advert = mdns::advertise(&d.id(), 4242).unwrap();
    let addr = mdns::discover(&d.id(), Duration::from_secs(8)).unwrap();
    assert!(addr.ends_with(":4242"), "{addr}");
    // A different device id must not match.
    assert!(mdns::discover(&[1; 16], Duration::from_secs(2)).is_err());
}
