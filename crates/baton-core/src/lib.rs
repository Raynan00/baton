//! Baton core: protocol, crypto, transport routing, restore-URL building.
#![forbid(unsafe_code)]

pub mod crypto;
pub mod discovery;
pub mod error;
pub mod noise;
pub mod outbox;
pub mod pairing_link;
pub mod restore_url;
pub mod router;
pub mod sas;

/// Generated protobuf types (`proto/baton/v1/baton.proto`).
pub mod pb {
    include!(concat!(env!("OUT_DIR"), "/baton.v1.rs"));
}

pub use error::{Error, Result};
