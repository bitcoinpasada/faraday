//! BLAKE2b (RFC 7693), unkeyed, at any digest length from 1 to 64
//! bytes.
//!
//! It is here for one caller: AEZ's `Extract` step hashes a key that is
//! not already 48 bytes into 48 bytes with BLAKE2b, and LND's aezeed
//! hands AEZ a 32-byte scrypt key, so every aezeed decode runs it
//! (`docs/PLANNING.md` §16.116). RFC 7693's compression function is
//! twelve rounds of a four-word mixing step over a 128-byte block; the
//! whole of it is below.

/// BLAKE2b's IV, which is SHA-512's: the fractional parts of the square
/// roots of the first eight primes.
const IV: [u64; 8] = [
    0x6a09_e667_f3bc_c908,
    0xbb67_ae85_84ca_a73b,
    0x3c6e_f372_fe94_f82b,
    0xa54f_f53a_5f1d_36f1,
    0x510e_527f_ade6_82d1,
    0x9b05_688c_2b3e_6c1f,
    0x1f83_d9ab_fb41_bd6b,
    0x5be0_cd19_137e_2179,
];

/// The message-word schedule, one permutation per round.
const SIGMA: [[usize; 16]; 12] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
];

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
    let mut h = IV;
    // Parameter block word 0: digest length, key length 0, fanout 1,
    // depth 1.
    h[0] ^= 0x0101_0000 ^ (out.len() as u64);

    // Every block but the last is compressed with the final flag clear;
    // the last is zero-padded, and the empty message is one padded
    // block of zeros.
    let mut counter: u128 = 0;
    let full = data.len().saturating_sub(1) / 128;
    for i in 0..full {
        counter += 128;
        compress(&mut h, &data[i * 128..i * 128 + 128], counter, false);
    }
    let rest = &data[full * 128..];
    let mut last = [0u8; 128];
    last[..rest.len()].copy_from_slice(rest);
    counter += rest.len() as u128;
    compress(&mut h, &last, counter, true);

    let mut digest = [0u8; MAX_OUT];
    for (i, word) in h.iter().enumerate() {
        digest[i * 8..i * 8 + 8].copy_from_slice(&word.to_le_bytes());
    }
    out.copy_from_slice(&digest[..out.len()]);
    // The state, the last block and the full digest are as secret as
    // what was hashed.
    zeroize::Zeroize::zeroize(&mut h);
    zeroize::Zeroize::zeroize(&mut last);
    zeroize::Zeroize::zeroize(&mut digest);
}

/// RFC 7693 §3.2, `F`: twelve rounds over one 128-byte block.
fn compress(h: &mut [u64; 8], block: &[u8], counter: u128, last: bool) {
    let mut m = [0u64; 16];
    for (i, word) in m.iter_mut().enumerate() {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&block[i * 8..i * 8 + 8]);
        *word = u64::from_le_bytes(bytes);
    }

    let mut v = [0u64; 16];
    v[..8].copy_from_slice(h);
    v[8..].copy_from_slice(&IV);
    v[12] ^= counter as u64;
    v[13] ^= (counter >> 64) as u64;
    if last {
        v[14] = !v[14];
    }

    for s in &SIGMA {
        mix(&mut v, 0, 4, 8, 12, m[s[0]], m[s[1]]);
        mix(&mut v, 1, 5, 9, 13, m[s[2]], m[s[3]]);
        mix(&mut v, 2, 6, 10, 14, m[s[4]], m[s[5]]);
        mix(&mut v, 3, 7, 11, 15, m[s[6]], m[s[7]]);
        mix(&mut v, 0, 5, 10, 15, m[s[8]], m[s[9]]);
        mix(&mut v, 1, 6, 11, 12, m[s[10]], m[s[11]]);
        mix(&mut v, 2, 7, 8, 13, m[s[12]], m[s[13]]);
        mix(&mut v, 3, 4, 9, 14, m[s[14]], m[s[15]]);
    }

    for i in 0..8 {
        h[i] ^= v[i] ^ v[i + 8];
    }
    zeroize::Zeroize::zeroize(&mut m);
    zeroize::Zeroize::zeroize(&mut v);
}

/// RFC 7693 §3.1, `G`.
#[allow(clippy::too_many_arguments)]
fn mix(v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize, x: u64, y: u64) {
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(x);
    v[d] = (v[d] ^ v[a]).rotate_right(32);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(24);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(y);
    v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(63);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 7693 appendix A: BLAKE2b-512 of "abc".
    #[test]
    fn rfc_7693_abc() {
        let mut out = [0u8; 64];
        blake2b(b"abc", &mut out);
        let expected = [
            0xba, 0x80, 0xa5, 0x3f, 0x98, 0x1c, 0x4d, 0x0d, 0x6a, 0x27, 0x97, 0xb6, 0x9f, 0x12,
            0xf6, 0xe9, 0x4c, 0x21, 0x2f, 0x14, 0x68, 0x5a, 0xc4, 0xb7, 0x4b, 0x12, 0xbb, 0x6f,
            0xdb, 0xff, 0xa2, 0xd1, 0x7d, 0x87, 0xc5, 0x39, 0x2a, 0xab, 0x79, 0x2d, 0xc2, 0x52,
            0xd5, 0xde, 0x45, 0x33, 0xcc, 0x95, 0x18, 0xd3, 0x8a, 0xa8, 0xdb, 0xf1, 0x92, 0x5a,
            0xb9, 0x23, 0x86, 0xed, 0xd4, 0x00, 0x99, 0x23,
        ];
        assert_eq!(out, expected);
    }

    /// The empty message, BLAKE2b-512, from RFC 7693's own test suite.
    #[test]
    fn empty_message() {
        let mut out = [0u8; 64];
        blake2b(b"", &mut out);
        assert_eq!(
            out[..8],
            [0x78, 0x6a, 0x02, 0xf7, 0x42, 0x01, 0x59, 0x03],
            "BLAKE2b-512 of the empty string begins 786a02f742015903"
        );
    }

    /// A digest length is part of the parameter block, so a shorter
    /// digest is not a prefix of a longer one.
    #[test]
    fn length_is_not_truncation() {
        let mut short = [0u8; 48];
        let mut long = [0u8; 64];
        blake2b(b"abc", &mut short);
        blake2b(b"abc", &mut long);
        assert_ne!(short[..], long[..48]);
    }
}
