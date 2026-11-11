//! A [`Cost`] for Argon2id that the caller chooses and a file carries, and
//! the function that runs it (a wrapper over `faraday_crypto::argon2`). The
//! self-test checks it against a known answer and the cross-check's
//! `vault.argon2id` judges it. The vault (`docs/VAULT.md`) takes this
//! crate's [`Cost`] and stretches its passphrases through
//! `faraday_crypto::argon2` itself, which this function wraps. Nothing here
//! draws randomness.
//!
//! `no_std` + `alloc`. The Argon2id working memory is asked for with
//! `try_reserve`, so a cost a device cannot meet is an error rather
//! than an abort.

#![no_std]

extern crate alloc;

use argon2::{Algorithm, Params};
use faraday_crypto::Secret;
use faraday_crypto::work::Compute;

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

/// The device's own Argon2id cost: 64 MiB,
/// three passes, one lane. Its passes and lanes are also the most a
/// KDBX file may state, so a hostile header cannot ask for more
/// work than the device does for itself.
pub const DEVICE_PARAMS: Cost = Cost {
    memory_kib: 65_536,
    passes: 3,
    lanes: 1,
};

/// Argon2id's own minimum: the cost tests write at, where what they are
/// about is not the cost. A file
/// written at it is as easy to guess as Argon2id allows.
pub const MIN_PARAMS: Cost = Cost {
    memory_kib: Params::MIN_M_COST,
    passes: Params::MIN_T_COST,
    lanes: Params::MIN_P_COST,
};

/// Why [`argon2id`] gave no key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StretchError {
    /// The cost is not one Argon2id accepts.
    Params,
    /// The working memory the cost asks for could not be had.
    Memory,
}

/// Costs an Argon2id run: `cost.memory_kib` of memory filled
/// `cost.passes` times. `Argon2id(password, salt)`, version 0x13, 32
/// bytes, at `cost`.
///
/// The working memory is asked for with `try_reserve` rather than
/// taken, so a device that cannot spare what a cost asks for gets
/// [`StretchError::Memory`] instead of aborting on the allocation. It is
/// overwritten before it is given back, since what it holds is derived
/// from the password. (The `argon2` crate's own allocating helper needs
/// a feature that brings in its string format.)
pub fn argon2id(
    c: &Compute,
    cost: &Cost,
    password: &[u8],
    salt: &[u8],
) -> Result<Secret<[u8; 32]>, StretchError> {
    let mut derived = Secret::new([0u8; 32]);
    faraday_crypto::argon2(
        c,
        Algorithm::Argon2id,
        password,
        salt,
        faraday_crypto::Argon2Cost {
            memory_kib: cost.memory_kib,
            passes: cost.passes,
            lanes: cost.lanes,
        },
        &mut derived,
    )
    .map_err(|e| match e {
        faraday_crypto::Argon2Error::Memory => StretchError::Memory,
        faraday_crypto::Argon2Error::Params => StretchError::Params,
    })?;
    Ok(derived)
}
