//! Faraday's samples (F81, `docs/MODES.md` §5): the public dummies Practice
//! takes, as data. Keys in every form Faraday reads (a repeated word at
//! every length, test vectors of BIP-39, SLIP-39, codex32 and Seed XOR, an
//! LND cipher seed), the testkit's wallets with an
//! unsigned spend each, a vault, a signed message, an address and a TOTP
//! setup code.
//!
//! Public data only: strings, bytes and fingerprints, in the shapes
//! `faraday_core::samples` defines. This crate parses nothing and derives
//! nothing, so no key material is in it or reaches it. Its tests check
//! every value against the testkit, the published vectors and another
//! implementation; `examples/regenerate.rs` writes `src/data.rs` and
//! `data/` again when one of those changes.
//!
//! Only the desktop app depends on this crate. The stick, the Pi, the disk
//! and grant helpers, the boot report and the Starter must not
//! (`tools/lint-keys-tests.sh`).
//!
//! The `name` of a sample is the name of its string in
//! `faraday-strings`; the strings are not here.

mod data;

pub use data::SAMPLES;
