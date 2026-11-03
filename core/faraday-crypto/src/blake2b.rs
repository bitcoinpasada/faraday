//! BLAKE2b (RFC 7693), unkeyed, at any digest length from 1 to 64
//! bytes, over the `blake2` crate's block core.
//!
//! It is here for one caller: AEZ's `Extract` step hashes a key that is
//! not already 48 bytes into 48 bytes with BLAKE2b, and LND's aezeed
//! hands AEZ a 32-byte scrypt key, so every aezeed decode runs it.

use blake2::Blake2bVarCore;
use blake2::digest::Output;
use blake2::digest::block_api::{Buffer, UpdateCore, VariableOutputCore};
use zeroize::Zeroize;

/// The longest digest BLAKE2b produces.
pub const MAX_OUT: usize = 64;

/// BLAKE2b of `data`, unkeyed, filling `out`.
///
/// `out` must be 1 to [`MAX_OUT`] bytes long; the digest length is part
/// of the parameter block, so a 48-byte digest is not a truncation of a
/// 64-byte one.
///
/// # Panics
///
/// If `out` is empty or longer than [`MAX_OUT`].
pub fn blake2b(data: &[u8], out: &mut [u8]) {
    assert!(
        !out.is_empty() && out.len() <= MAX_OUT,
        "BLAKE2b digest length is 1 to 64 bytes"
    );
    let mut core = Blake2bVarCore::new(out.len()).expect("the length is checked above");
    let mut buffer = Buffer::<Blake2bVarCore>::default();
    buffer.digest_blocks(data, |blocks| core.update_blocks(blocks));
    let mut digest = Output::<Blake2bVarCore>::default();
    core.finalize_variable_core(&mut buffer, &mut digest);
    out.copy_from_slice(&digest[..out.len()]);
    // The core and the buffer wipe themselves on drop; the full digest
    // is as secret as what was hashed.
    digest.as_mut_slice().zeroize();
}
