//! BIP standards for OpenSignerKit: BIP-39 mnemonics, BIP-32 keys,
//! BIP-44/49/84/86 accounts and addresses, SLIP-132 and BIP-380
//! descriptor checksums, and BIP-85 child mnemonics.
//!
//! Design (see `docs/PLANNING.md` §16.1 and §16.2):
//!
//! - **Word indices, not text.** A committed mnemonic word is a `u16` in
//!   `0..2048`. [`bip39::Mnemonic`] is a fixed-size, stack-allocated,
//!   zeroizing value; no heap text type ever holds a mnemonic, seed or
//!   entropy. Text appears only at the edges: `&'static str` from the
//!   embedded wordlists for display, and `&str` typed input for parsing.
//! - **In-house SLIP-39 too.** [`slip39`] writes Shamir backup the same
//!   way, over its own 1024-word list. What it recovers is a master
//!   secret, which is the BIP-32 seed itself and not BIP-39 entropy.
//! - **In-house BIP-39.** The algorithm is small, and the explorer tools
//!   need intermediate values (checksum bits, final-word candidates) that a
//!   library hides. The `bip39` crate is a dev-dependency used to
//!   cross-check every language and vector.
//! - **No Unicode tables on the device.** Wordlists are embedded already
//!   NFKD-normalized, and passphrases are restricted to printable ASCII,
//!   which NFKD leaves unchanged. Seed derivation therefore needs no
//!   normalization code.
//! - **rust-bitcoin for keys and addresses.** [`keys`] wraps
//!   `bitcoin::bip32` so that private material never gains `Debug`,
//!   `Clone` or a text form and is erased on drop. Extended public keys,
//!   addresses and descriptors are not secrets and are ordinary `String`s
//!   in [`account`], [`slip132`] and [`descriptor`].
//!
//! - **Multi-party signing by hand.** [`musig`] writes BIP-327's rounds
//!   and [`frost`] BIP 445's over the same curve primitives, because no
//!   `bitcoin` release this tree can pin carries either module. Both
//!   are checked against the published vectors; [`frost`] adds the
//!   trusted dealer that makes a threshold wallet's shares.
//!
//! - **Codex32 in house.** [`codex32`] writes BIP 93's checksum,
//!   header and GF(32) interpolation over the same fixed-size zeroizing
//!   types, so a master seed and its shares never reach a heap string.
//!
//! The crate is `no_std`; `bitcoin` needs `alloc`, so heap allocation
//! exists but is confined to non-secret data. `tools/lint-secrets.sh`
//! keeps `bip39.rs`, `slip39.rs`, `keys.rs` and `frost.rs` free of heap
//! text.

#![no_std]

extern crate alloc;

pub mod account;
pub mod address;
pub mod aez;
pub mod aezeed;
pub mod base64;
pub mod bip39;
pub mod bip85;
pub mod bsms;
pub mod codex32;
pub mod coldcard;
pub mod compile;
pub mod core_import;
pub mod descriptor;
pub mod diceware;
pub mod frost;
pub mod hangul;
pub mod hashes;
pub mod kana;
pub mod keys;
pub mod multisig_config;
pub mod musig;
pub mod policy;
pub mod recovery;
pub mod silent;
pub mod silent_wallet;
pub mod slip132;
pub mod slip39;
pub mod spend;
pub mod tapmulti;
pub mod threshold;
pub mod vanity;
pub mod wordlists;
pub mod xkey;

/// The rust-bitcoin version this crate is built against, re-exported so
/// callers use the same types without a second dependency.
pub use bitcoin;

/// The rust-miniscript version this crate is built against, re-exported
/// for the same reason.
pub use miniscript;
