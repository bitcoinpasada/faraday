//! Where Faraday's key material is used.
//!
//! Every function here takes the secret it works on as an argument (a
//! master key, seed words, a passphrase, a record's bytes) and gives back
//! a public result, or a secret wrapped in `Zeroizing` where the caller
//! needs one. The flow around each (state, sheets, toasts, the Outbox,
//! strings) stays in faraday-core. By the later steps of `docs/KEYS.md`
//! the session's keys and the open vault's secrets are held here too,
//! and nothing outside this crate touches the primitive crates' secret
//! types.
//!
//! This crate does not depend on faraday-core.

pub mod bip85;
pub mod carry;
pub mod frost;
pub mod identity;
pub mod lightning;
pub mod message;
pub mod psbt;
pub mod silent;
pub mod texts;
pub mod vanity;

pub use identity::{Material, fingerprint, master_of};
