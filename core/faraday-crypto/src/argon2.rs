//! Argon2 (RFC 9106), the one call the backup's, the KeePass file's and the
//! OpenPGP message's derivations make.
//!
//! The working memory is asked for with `try_reserve` rather than taken,
//! so a device that cannot spare what a cost asks for gets
//! [`Error::Memory`] instead of aborting on the allocation, and it is
//! overwritten before it is given back, since what it holds is derived
//! from the password. (The `argon2` crate's own allocating helper needs a
//! feature that brings in its string format.)

use alloc::vec::Vec;
use argon2::{Algorithm, Argon2, Block, Params, Version};
use zeroize::Zeroize;

use crate::secret::Secret;
use crate::work::{Compute, Work};

/// Why [`argon2`] gave no key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The working memory the cost asks for could not be had.
    Memory,
    /// The cost, or the salt or password, is not one Argon2 accepts.
    Params,
}

/// What an Argon2 derivation costs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cost {
    /// Memory in KiB.
    pub memory_kib: u32,
    /// Passes over that memory.
    pub passes: u32,
    /// Lanes.
    pub lanes: u32,
}

/// Costs an Argon2 derivation: `cost.memory_kib` KiB of memory filled
/// `cost.passes` times. Argon2id, or for a KeePass file read Argon2d,
/// version 0x13, of `password` and `salt` at `cost`, 32 bytes, into `out`.
///
/// The derivation is counted before the cost is checked, as the callers
/// did. `out` is written only when this returns `Ok`.
pub fn argon2(
    c: &Compute,
    algorithm: Algorithm,
    password: &[u8],
    salt: &[u8],
    cost: Cost,
    out: &mut Secret<[u8; 32]>,
) -> Result<(), Error> {
    c.count(Work::Kdf);
    let params = Params::new(cost.memory_kib, cost.passes, cost.lanes, Some(32))
        .map_err(|_| Error::Params)?;
    let argon = Argon2::new(algorithm, Version::V0x13, params.clone());
    let blocks = params.block_count();
    let mut memory: Vec<Block> = Vec::new();
    memory
        .try_reserve_exact(blocks)
        .map_err(|_| Error::Memory)?;
    memory.resize(blocks, Block::default());
    let mut derived = Secret::new([0u8; 32]);
    let hashed = argon
        .hash_password_into_with_memory(password, salt, derived.expose_mut(), &mut memory)
        .is_ok();
    memory.zeroize();
    if hashed {
        *out.expose_mut() = *derived.expose();
        Ok(())
    } else {
        Err(Error::Params)
    }
}
