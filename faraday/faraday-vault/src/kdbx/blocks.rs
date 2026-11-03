//! The HMAC block stream that carries the encrypted body
//! (`docs/VAULT.md` §1.1): blocks of an HMAC-SHA-256, a length and the
//! ciphertext, then an empty block.

use alloc::vec::Vec;

use faraday_crypto::Secret;
use zeroize::Zeroizing;

use super::Error;
use super::keys::{block_key, hmac, hmac_ok};

/// The size of the blocks Faraday writes: 1 MiB.
pub(super) const BLOCK_LEN: usize = 1024 * 1024;
/// An HMAC-SHA-256 and a length.
pub(super) const BLOCK_OVERHEAD: usize = 32 + 4;

/// The ciphertext of every block of `stream`, joined, once the stream is
/// well formed and every block's HMAC verifies. Nothing is returned of a
/// stream with one block wrong, a block missing at the end, or bytes after
/// the empty block; the joined bytes are at most `limit`.
pub(super) fn join(
    stream: &[u8],
    base: &Secret<[u8; 64]>,
    limit: usize,
) -> Result<Zeroizing<Vec<u8>>, Error> {
    // First the frame alone: where each block is and how much they hold.
    let mut total = 0usize;
    let mut at = 0usize;
    loop {
        let len = block_len(stream, at)?;
        at += BLOCK_OVERHEAD + len;
        total = total.checked_add(len).ok_or(Error::TooLarge)?;
        if total > limit {
            return Err(Error::TooLarge);
        }
        if len == 0 {
            break;
        }
    }
    if at != stream.len() {
        return Err(Error::Damaged);
    }
    let mut out = Zeroizing::new(Vec::new());
    out.try_reserve_exact(total).map_err(|_| Error::TooLarge)?;
    // Then each block's HMAC, before its bytes are taken.
    let (mut at, mut i) = (0usize, 0u64);
    loop {
        let len = block_len(stream, at)?;
        let tag = &stream[at..at + 32];
        let len_bytes = &stream[at + 32..at + BLOCK_OVERHEAD];
        let data = &stream[at + BLOCK_OVERHEAD..at + BLOCK_OVERHEAD + len];
        if !hmac_ok(
            &block_key(base, i),
            &[&i.to_le_bytes(), len_bytes, data],
            tag,
        ) {
            return Err(Error::Damaged);
        }
        if len == 0 {
            return Ok(out);
        }
        out.extend_from_slice(data);
        at += BLOCK_OVERHEAD + len;
        i += 1;
    }
}

/// The length of the block at `at`, which lies whole inside `stream`.
fn block_len(stream: &[u8], at: usize) -> Result<usize, Error> {
    let len_at = at.checked_add(32).ok_or(Error::Damaged)?;
    let len = u32::from_le_bytes(
        stream
            .get(len_at..len_at + 4)
            .ok_or(Error::Damaged)?
            .try_into()
            .map_err(|_| Error::Damaged)?,
    ) as usize;
    let end = (at + BLOCK_OVERHEAD)
        .checked_add(len)
        .ok_or(Error::Damaged)?;
    if end > stream.len() {
        return Err(Error::Damaged);
    }
    Ok(len)
}

/// The stream for `ciphertext`: blocks of [`BLOCK_LEN`], then the empty
/// block.
pub(super) fn write(ciphertext: &[u8], base: &Secret<[u8; 64]>, out: &mut Vec<u8>) {
    let mut i = 0u64;
    for data in ciphertext.chunks(BLOCK_LEN) {
        put(i, data, base, out);
        i += 1;
    }
    put(i, &[], base, out);
}

/// One block: its HMAC, its length, its ciphertext.
fn put(i: u64, data: &[u8], base: &Secret<[u8; 64]>, out: &mut Vec<u8>) {
    let len = (data.len() as u32).to_le_bytes();
    out.extend_from_slice(&hmac(&block_key(base, i), &[&i.to_le_bytes(), &len, data]));
    out.extend_from_slice(&len);
    out.extend_from_slice(data);
}
