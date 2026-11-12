//! Read a Bitcoin Core wallet (F229): the reader is the crate
//! `faraday-core-wallet`, re-exported here; this module holds the tool and
//! its screen.

pub use faraday_core_wallet::*;

pub(crate) mod screen;
pub mod tool;
