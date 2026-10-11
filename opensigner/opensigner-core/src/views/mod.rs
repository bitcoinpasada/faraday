//! Widget trees for every screen, one module per area. Each is an `impl
//! OpenSigner` block that reads state and returns a [`Node`]; nothing
//! here mutates.
//!
//! [`Node`]: osk_ui::Node

mod backup;
mod bip85;
pub(crate) mod build;
pub(crate) mod calculators;
mod codex32;
pub(crate) mod create;
mod detail;
mod dice;
pub(crate) mod explore;
pub(crate) mod export;
mod files;
mod finish;
mod home;
mod inspect;
mod keep;
mod keys;
mod learn;
pub(crate) mod lightning;
pub(crate) mod load;
mod lock;
pub(crate) mod message;
mod notes;
mod overlay;
pub(crate) mod quiz;
mod scan;
mod secure_boot;
mod selftest;
mod settings;
mod shares;
pub(crate) mod sign;
mod silent;
mod tools;
mod vanity;
mod verify;
mod wallet;
pub(crate) mod wordlist;
pub(crate) mod words;
