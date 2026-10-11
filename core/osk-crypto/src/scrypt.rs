//! scrypt (RFC 7914), written here rather than taken as a crate.
//!
//! The tree already has PBKDF2-HMAC-SHA-256, which is the whole of
//! scrypt's outer structure; what is left is Salsa20/8, `BlockMix` and
//! `ROMix`, some sixty lines between them
//! (`docs/PLANNING.md` §16.116). One caller: LND's aezeed, which
//! stretches the passphrase with `N = 32768`, `r = 8`, `p = 1` into the
//! 32-byte AEZ key.
//!
//! The one allocation is `ROMix`'s `V`, which is `128 · r · N` bytes —
//! 32 MiB at aezeed's parameters. It is zeroized before it is freed.

use alloc::vec;
use zeroize::Zeroize;

use crate::hash::pbkdf2_hmac_sha256;

/// Why scrypt could not run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// `n` is not a power of two above 1, or `r` or `p` is zero, or the
    /// parameters ask for more memory than `usize` can address.
    Parameters,
}

/// scrypt with cost `n` (a power of two), block size `r` and
/// parallelism `p`, filling `out`.
///
/// `n` is the cost itself, not its logarithm: aezeed's parameters are
/// `n = 32768`, `r = 8`, `p = 1`.
pub fn scrypt(
    password: &[u8],
    salt: &[u8],
    n: u32,
    r: u32,
    p: u32,
    out: &mut [u8],
) -> Result<(), Error> {
    if n < 2 || !n.is_power_of_two() || r == 0 || p == 0 || out.is_empty() {
        return Err(Error::Parameters);
    }
    let r = r as usize;
    let p = p as usize;
    let n = n as usize;
    let block = 128usize.checked_mul(r).ok_or(Error::Parameters)?;
    let total = block.checked_mul(p).ok_or(Error::Parameters)?;
    // `V` is `n` blocks; on a 32-bit target this is where an absurd
    // cost is refused rather than aborting on allocation.
    let v_len = block.checked_mul(n).ok_or(Error::Parameters)?;

    let mut b = vec![0u8; total];
    pbkdf2_hmac_sha256(password, salt, 1, &mut b);

    let mut v = vec![0u8; v_len];
    let mut scratch = vec![0u8; 256 * r];
    for chunk in b.chunks_exact_mut(block) {
        ro_mix(chunk, n, r, &mut v, &mut scratch);
    }
    v.zeroize();
    scratch.zeroize();

    pbkdf2_hmac_sha256(password, &b, 1, out);
    b.zeroize();
    Ok(())
}

/// RFC 7914 §5, `scryptROMix`, in place over one `128 · r`-byte block.
fn ro_mix(b: &mut [u8], n: usize, r: usize, v: &mut [u8], scratch: &mut [u8]) {
    let block = 128 * r;
    for i in 0..n {
        v[i * block..(i + 1) * block].copy_from_slice(b);
        block_mix(b, r, scratch);
    }
    for _ in 0..n {
        // `Integerify`: the last 64-byte half-block, read as a
        // little-endian integer, modulo `n`, which is a power of two.
        let mut word = [0u8; 8];
        word.copy_from_slice(&b[block - 64..block - 56]);
        let j = (u64::from_le_bytes(word) as usize) & (n - 1);
        for (x, y) in b.iter_mut().zip(&v[j * block..(j + 1) * block]) {
            *x ^= y;
        }
        block_mix(b, r, scratch);
    }
}

/// RFC 7914 §4, `scryptBlockMix`, in place. `scratch` is `256 · r`
/// bytes: the `2r` output blocks, then the running `X`.
fn block_mix(b: &mut [u8], r: usize, scratch: &mut [u8]) {
    let (out, x) = scratch.split_at_mut(128 * r);
    x[..64].copy_from_slice(&b[128 * r - 64..]);
    for i in 0..2 * r {
        for (a, c) in x[..64].iter_mut().zip(&b[i * 64..i * 64 + 64]) {
            *a ^= c;
        }
        salsa20_8(&mut x[..64]);
        // The even blocks go to the first half of the output and the
        // odd ones to the second: `Y_0, Y_2, …, Y_1, Y_3, …`.
        let at = if i % 2 == 0 { i / 2 } else { r + i / 2 };
        out[at * 64..at * 64 + 64].copy_from_slice(&x[..64]);
    }
    b.copy_from_slice(&out[..128 * r]);
}

/// RFC 7914 §3, `Salsa20/8 Core`, in place over 64 bytes.
fn salsa20_8(block: &mut [u8]) {
    let mut x = [0u32; 16];
    for (i, word) in x.iter_mut().enumerate() {
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&block[i * 4..i * 4 + 4]);
        *word = u32::from_le_bytes(bytes);
    }
    let start = x;
    for _ in 0..4 {
        // Columns.
        quarter(&mut x, 4, 0, 12, 8);
        quarter(&mut x, 9, 5, 1, 13);
        quarter(&mut x, 14, 10, 6, 2);
        quarter(&mut x, 3, 15, 11, 7);
        // Rows.
        quarter(&mut x, 1, 0, 3, 2);
        quarter(&mut x, 6, 5, 4, 7);
        quarter(&mut x, 11, 10, 9, 8);
        quarter(&mut x, 12, 15, 14, 13);
    }
    for i in 0..16 {
        let sum = x[i].wrapping_add(start[i]);
        block[i * 4..i * 4 + 4].copy_from_slice(&sum.to_le_bytes());
    }
    x.zeroize();
}

/// One line of the Salsa20 core: `x[a] ^= rotl(x[b] + x[c], k)` for the
/// four rotations 7, 9, 13, 18.
fn quarter(x: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    x[a] ^= x[b].wrapping_add(x[c]).rotate_left(7);
    x[d] ^= x[a].wrapping_add(x[b]).rotate_left(9);
    x[c] ^= x[d].wrapping_add(x[a]).rotate_left(13);
    x[b] ^= x[c].wrapping_add(x[d]).rotate_left(18);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 7914 §12, the first and second published vectors. The third
    /// and fourth run at `N = 16384` and `N = 1048576`; the aezeed
    /// tests in `core/osk-bip` run scrypt at aezeed's own `N = 32768`
    /// against LND's vectors.
    #[test]
    fn rfc_7914_vectors() {
        let mut out = [0u8; 64];
        scrypt(b"", b"", 16, 1, 1, &mut out).unwrap();
        assert_eq!(out[..8], [0x77, 0xd6, 0x57, 0x62, 0x38, 0x65, 0x7b, 0x20]);
        assert_eq!(out[56..], [0xcf, 0x35, 0xe2, 0x0c, 0x38, 0xd1, 0x89, 0x06]);

        scrypt(b"password", b"NaCl", 1024, 8, 16, &mut out).unwrap();
        assert_eq!(out[..8], [0xfd, 0xba, 0xbe, 0x1c, 0x9d, 0x34, 0x72, 0x00]);
        assert_eq!(out[56..], [0x83, 0x60, 0xcb, 0xdf, 0xa2, 0xcc, 0x06, 0x40]);
    }
}
