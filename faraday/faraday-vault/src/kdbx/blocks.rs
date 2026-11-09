//! The HMAC block stream that carries the encrypted body
//! (`docs/VAULT.md` §1.1): blocks of an HMAC-SHA-256, a length and the
//! ciphertext, then an empty block.

use alloc::vec::Vec;

use chacha20::ChaCha20;
use chacha20::cipher::StreamCipher;
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
    out.try_reserve_exact(total)
        .map_err(|_| super::no_memory(total))?;
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

/// Writes the plaintext of a file as the stream, encrypted and framed in
/// place in the output (F206 point 6): each piece handed to [`Framer::put`]
/// is appended, encrypted where it stands and sealed by its block's HMAC
/// once the block is full, so no copy of the plaintext is ever made.
pub(super) struct Framer<'a> {
    out: &'a mut Vec<u8>,
    cipher: ChaCha20,
    base: &'a Secret<[u8; 64]>,
    index: u64,
    /// Where the open block's frame begins in `out`, if one is open.
    frame_at: Option<usize>,
}

impl<'a> Framer<'a> {
    /// A stream appended to `out`, encrypted by `cipher`.
    pub(super) fn new(out: &'a mut Vec<u8>, cipher: ChaCha20, base: &'a Secret<[u8; 64]>) -> Self {
        Self {
            out,
            cipher,
            base,
            index: 0,
            frame_at: None,
        }
    }

    /// Appends plaintext. `out` must have room for the whole file: the
    /// caller reserves it, so nothing here allocates.
    pub(super) fn put(&mut self, mut data: &[u8]) {
        while !data.is_empty() {
            let at = *self.frame_at.get_or_insert_with(|| {
                let at = self.out.len();
                self.out.resize(at + BLOCK_OVERHEAD, 0);
                at
            });
            let filled = self.out.len() - at - BLOCK_OVERHEAD;
            let n = data.len().min(BLOCK_LEN - filled);
            let from = self.out.len();
            self.out.extend_from_slice(&data[..n]);
            self.cipher.apply_keystream(&mut self.out[from..]);
            data = &data[n..];
            if filled + n == BLOCK_LEN {
                self.close();
            }
        }
    }

    /// Closes the open block, if any: its length and its HMAC.
    fn close(&mut self) {
        let Some(at) = self.frame_at.take() else {
            return;
        };
        let len = ((self.out.len() - at - BLOCK_OVERHEAD) as u32).to_le_bytes();
        let tag = hmac(
            &block_key(self.base, self.index),
            &[
                &self.index.to_le_bytes(),
                &len,
                &self.out[at + BLOCK_OVERHEAD..],
            ],
        );
        self.out[at..at + 32].copy_from_slice(&tag);
        self.out[at + 32..at + BLOCK_OVERHEAD].copy_from_slice(&len);
        self.index += 1;
    }

    /// Closes the last block and writes the empty block after it.
    pub(super) fn finish(mut self) {
        self.close();
        let len = 0u32.to_le_bytes();
        let tag = hmac(
            &block_key(self.base, self.index),
            &[&self.index.to_le_bytes(), &len, &[]],
        );
        self.out.extend_from_slice(&tag);
        self.out.extend_from_slice(&len);
    }
}
