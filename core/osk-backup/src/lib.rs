//! The two encrypted file formats OpenSigner writes, apart from any
//! screen that asks for them (`docs/PLANNING.md` §16.138):
//!
//! - [`oskb`]: `osk-backup`, OpenSigner's own format for a key's words,
//!   a master seed, a note or a wallet's recovery sheet under a
//!   passphrase. `docs/BACKUP.md` writes it down byte for byte, the
//!   vectors in `tools/vectors/backup/` pin it, and
//!   `tools/backup/decrypt.py` reads it with two published libraries.
//! - [`kdbx`]: a KDBX 4 database a KeePass app opens with the same
//!   passphrase.
//!
//! Both stretch the passphrase with Argon2id at a [`Cost`] the caller
//! chooses and the file carries, and both take their salt and nonces
//! from a seed the caller supplies: nothing here draws randomness.
//!
//! `no_std` + `alloc`. The Argon2id working memory is asked for with
//! `try_reserve`, so a cost a device cannot meet is an error rather
//! than an abort.

#![no_std]

extern crate alloc;

use argon2::Params;

pub mod kdbx;
pub mod oskb;

/// What Argon2id costs a guess, as a file carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cost {
    /// Memory in KiB.
    pub memory_kib: u32,
    /// Passes over that memory.
    pub passes: u32,
    /// Lanes.
    pub lanes: u32,
}

/// The Argon2id cost OpenSigner writes the kept-key blob at: 64 MiB,
/// three passes, one lane. Its passes and lanes are also the most an
/// `osk-backup` file may state, so a hostile header cannot ask for more
/// work than the device does for itself.
pub const DEVICE_PARAMS: Cost = Cost {
    memory_kib: 65_536,
    passes: 3,
    lanes: 1,
};

/// Argon2id's own minimum: the cost tests write at, where what they are
/// about is not the cost (`docs/PLANNING.md` §16.119 rule 3). A file
/// written at it is as easy to guess as Argon2id allows.
pub const MIN_PARAMS: Cost = Cost {
    memory_kib: Params::MIN_M_COST,
    passes: Params::MIN_T_COST,
    lanes: Params::MIN_P_COST,
};
