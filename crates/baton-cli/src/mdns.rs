//! mDNS advertise/browse using the rotating hint (BATON.md §3.2).
use anyhow::{anyhow, Result};
use baton_core::discovery::{hint_matches, lan_hint, week, SERVICE};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn now_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Keep the returned daemon alive for as long as the advert should run.
pub fn advertise(device_id: &[u8; 16], port: u16) -> Result<ServiceDaemon> {
    let d = ServiceDaemon::new()?;
    let instance = hex::encode(rand_bytes());
    let host = format!("{instance}.local.");
    let props = [("h", lan_hint(device_id, week(now_s())))];
    d.register(
        ServiceInfo::new(SERVICE, &instance, &host, "", port, &props[..])?.enable_addr_auto(),
    )?;
    Ok(d)
}

fn rand_bytes() -> [u8; 6] {
    use rand::RngCore;
    let mut b = [0u8; 6];
    rand::rngs::OsRng.fill_bytes(&mut b);
    b
}

/// Find a paired peer on the LAN; returns `ip:port`.
pub fn discover(peer_id: &[u8; 16], wait: Duration) -> Result<String> {
    let d = ServiceDaemon::new()?;
    let rx = d.browse(SERVICE)?;
    let end = Instant::now() + wait;
    while let Some(left) = end.checked_duration_since(Instant::now()) {
        if let Ok(ServiceEvent::ServiceResolved(info)) = rx.recv_timeout(left) {
            let h = info.get_property_val_str("h").unwrap_or_default();
            if hint_matches(h, peer_id, now_s()) {
                if let Some(ip) = info.get_addresses().iter().next() {
                    return Ok(format!("{ip}:{}", info.get_port()));
                }
            }
        }
    }
    Err(anyhow!("peer not found via mDNS"))
}
