//! The integration tests of faraday-psbt, one module per subject.

// The examples are standalone programs that include the shared builder;
// the tests include the same files, so a file is loaded as a module more
// than once on purpose.
#![allow(clippy::duplicate_mod)]

mod common;
mod fixtures;
mod inspect;
mod message;
mod miniscript;
mod multisig;
mod musig;
mod nonce;
mod sign;
mod tapmulti;
mod threshold;
mod transaction;
mod vectors;
mod verify;
mod wallet_fixtures;
