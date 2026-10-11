//! AEZ v5, the wide-block robust AEAD LND's aezeed enciphers a cipher
//! seed with (`docs/PLANNING.md` §16.116).
//!
//! This is a port of the AEZ v5 reference code as Yawning Angel's Go
//! implementation holds it, which is what LND itself calls; the file
//! read and its digest are in `tools/reference/aezeed/README.md`. The
//! primitive is here rather than in a crate because no Rust AEZ crate
//! is maintained, and because AEZ is a few hundred lines over the AES
//! round function and nothing else.
//!
//! What is built on it is [`crate::aezeed`], and nothing else: this
//! device never enciphers an aezeed, and [`encrypt`] exists so that the
//! published vectors — which are stated as encryptions — can be run
//! against it.
//!
//! # What AEZ is
//!
//! A key, a nonce, a vector of associated data and an expansion `tau`
//! name a length-preserving enciphering of `plaintext ‖ 0^tau`. There
//! is no separate tag: the authentication is that a decipherment whose
//! last `tau` bytes are not zero is a forgery. The key is first
//! `Extract`ed to 48 bytes — copied when it is already 48, hashed with
//! BLAKE2b when it is not, which is the case aezeed is in — and those
//! 48 bytes are the three subkeys `I`, `J`, `L` that the four- and
//! ten-round AES calls run over.

use alloc::vec;
use alloc::vec::Vec;
use osk_crypto::{Zeroize, blake2b};

/// AEZ's block, which is AES's.
const BLOCK: usize = 16;
/// The extracted key: `I ‖ J ‖ L`.
const EXTRACTED: usize = 3 * BLOCK;

/// AES's S-box.
const SBOX: [u8; 256] = [
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
];

/// Multiplication by `x` in GF(2⁸), AES's field.
fn xtime(a: u8) -> u8 {
    (a << 1) ^ (0x1b & ((a >> 7) & 1).wrapping_neg())
}

/// One AES round over the whole state — SubBytes, ShiftRows,
/// MixColumns, AddRoundKey — which is what AEZ's `AES4` and `AES10`
/// apply. MixColumns runs in every round, the last included, and the
/// key is never added before the first: AEZ is not AES, it is AES's
/// round function used as a public permutation.
fn round(state: &mut [u8; BLOCK], key: &[u8; BLOCK]) {
    let b = *state;
    for c in 0..4 {
        let a0 = SBOX[b[4 * c] as usize];
        let a1 = SBOX[b[4 * ((c + 1) % 4) + 1] as usize];
        let a2 = SBOX[b[4 * ((c + 2) % 4) + 2] as usize];
        let a3 = SBOX[b[4 * ((c + 3) % 4) + 3] as usize];
        state[4 * c] = xtime(a0) ^ xtime(a1) ^ a1 ^ a2 ^ a3 ^ key[4 * c];
        state[4 * c + 1] = a0 ^ xtime(a1) ^ xtime(a2) ^ a2 ^ a3 ^ key[4 * c + 1];
        state[4 * c + 2] = a0 ^ a1 ^ xtime(a2) ^ xtime(a3) ^ a3 ^ key[4 * c + 2];
        state[4 * c + 3] = xtime(a0) ^ a0 ^ a1 ^ a2 ^ xtime(a3) ^ key[4 * c + 3];
    }
}

/// `dst = a ^ b`, over one block.
fn xor16(a: &[u8], b: &[u8]) -> [u8; BLOCK] {
    let mut out = [0u8; BLOCK];
    for i in 0..BLOCK {
        out[i] = a[i] ^ b[i];
    }
    out
}

/// Doubling in GF(2¹²⁸), AEZ's `2 · x`.
fn double(p: &mut [u8; BLOCK]) {
    let top = p[0];
    for i in 0..15 {
        p[i] = (p[i] << 1) | (p[i + 1] >> 7);
    }
    p[15] = (p[15] << 1) ^ (135 & (top >> 7).wrapping_neg());
}

/// AEZ's `x · src` by double-and-add. `x` is a small constant of the
/// scheme, never a secret, so the loop's shape leaks nothing.
fn mult(x: u32, src: &[u8; BLOCK]) -> [u8; BLOCK] {
    let mut t = *src;
    let mut r = [0u8; BLOCK];
    let mut x = x;
    while x != 0 {
        if x & 1 != 0 {
            r = xor16(&r, &t);
        }
        double(&mut t);
        x >>= 1;
    }
    t.zeroize();
    r
}

/// AEZ's `Extract`: the key as the three 16-byte subkeys `I`, `J`, `L`.
/// A 48-byte key is those subkeys already; any other length is hashed
/// to 48 bytes with BLAKE2b, which is the path LND's 32-byte scrypt key
/// takes.
///
/// Public so that the reference implementation's `extract.json` can be
/// run against it; nothing outside the vectors calls it.
pub fn extract(key: &[u8]) -> [u8; EXTRACTED] {
    let mut out = [0u8; EXTRACTED];
    if key.len() == EXTRACTED {
        out.copy_from_slice(key);
    } else {
        blake2b(key, &mut out);
    }
    out
}

/// The key schedule and the doubled subkeys one AEZ key gives.
struct State {
    /// `1I`, `2I`.
    i: [[u8; BLOCK]; 2],
    /// `1J`, `2J`, `4J`.
    j: [[u8; BLOCK]; 3],
    /// `0L` through `7L`.
    l: [[u8; BLOCK]; 8],
    /// The four round keys `AES4` applies: `J`, `I`, `L`, `0`.
    key4: [[u8; BLOCK]; 4],
    /// The ten `AES10` applies: `I`, `J`, `L` three times over, then
    /// `I`.
    key10: [[u8; BLOCK]; 10],
}

impl Drop for State {
    fn drop(&mut self) {
        for b in self
            .i
            .iter_mut()
            .chain(self.j.iter_mut())
            .chain(self.l.iter_mut())
            .chain(self.key4.iter_mut())
            .chain(self.key10.iter_mut())
        {
            b.zeroize();
        }
    }
}

impl State {
    fn new(key: &[u8]) -> State {
        let mut extracted = extract(key);
        let mut sub = [[0u8; BLOCK]; 3];
        for (n, block) in sub.iter_mut().enumerate() {
            block.copy_from_slice(&extracted[n * BLOCK..(n + 1) * BLOCK]);
        }
        extracted.zeroize();
        let (ik, jk, lk) = (sub[0], sub[1], sub[2]);

        let mut i = [ik, ik];
        double(&mut i[1]);
        let mut j = [jk, jk, jk];
        double(&mut j[1]);
        j[2] = j[1];
        double(&mut j[2]);

        let mut l = [[0u8; BLOCK]; 8];
        l[1] = lk;
        l[2] = lk;
        double(&mut l[2]);
        l[3] = xor16(&l[2], &l[1]);
        l[4] = l[2];
        double(&mut l[4]);
        l[5] = xor16(&l[4], &l[1]);
        l[6] = l[3];
        double(&mut l[6]);
        l[7] = xor16(&l[6], &l[1]);

        let key4 = [jk, ik, lk, [0u8; BLOCK]];
        let key10 = [ik, jk, lk, ik, jk, lk, ik, jk, lk, ik];
        State {
            i,
            j,
            l,
            key4,
            key10,
        }
    }

    /// `E^{j}_{i}` with four rounds: the three tweak blocks are XORed
    /// into the source and the round function runs four times.
    fn aes4(&self, j: &[u8; BLOCK], i: &[u8; BLOCK], l: &[u8; BLOCK], src: &[u8]) -> [u8; BLOCK] {
        let mut state = [0u8; BLOCK];
        for n in 0..BLOCK {
            state[n] = j[n] ^ i[n] ^ l[n] ^ src[n];
        }
        for key in &self.key4 {
            round(&mut state, key);
        }
        state
    }

    /// `E^{-1}_{i}` with ten rounds.
    fn aes10(&self, l: &[u8; BLOCK], src: &[u8]) -> [u8; BLOCK] {
        let mut state = [0u8; BLOCK];
        for n in 0..BLOCK {
            state[n] = l[n] ^ src[n];
        }
        for key in &self.key10 {
            round(&mut state, key);
        }
        state
    }

    /// AEZ-hash: the tweak `delta` over `tau` (in bits), the nonce and
    /// each element of the associated data.
    fn hash(&self, nonce: &[u8], ad: &[&[u8]], tau_bits: u32) -> [u8; BLOCK] {
        let mut buf = [0u8; BLOCK];
        buf[12..].copy_from_slice(&tau_bits.to_be_bytes());
        let j = xor16(&self.j[0], &self.j[1]);
        let mut sum = self.aes4(&j, &self.i[1], &self.l[1], &buf);

        // The nonce, then every element, each in its own tweak space.
        self.hash_element(nonce, &self.j[2], &mut sum);
        for (k, element) in ad.iter().enumerate() {
            self.hash_element(element, &mult(5 + k as u32, &self.j[0]), &mut sum);
        }
        sum
    }

    /// One string of [`hash`](Self::hash): every whole block under a
    /// rising tweak, then the one-zero-padded remainder, which an empty
    /// string also has.
    fn hash_element(&self, data: &[u8], tweak: &[u8; BLOCK], sum: &mut [u8; BLOCK]) {
        let mut i = self.i[1];
        let mut rest = data;
        let mut n = 1usize;
        while rest.len() >= BLOCK {
            let out = self.aes4(tweak, &i, &self.l[n % 8], &rest[..BLOCK]);
            *sum = xor16(sum, &out);
            rest = &rest[BLOCK..];
            if n.is_multiple_of(8) {
                double(&mut i);
            }
            n += 1;
        }
        if !rest.is_empty() || data.is_empty() {
            let mut pad = [0u8; BLOCK];
            pad[..rest.len()].copy_from_slice(rest);
            pad[rest.len()] = 0x80;
            let out = self.aes4(tweak, &self.i[0], &self.l[0], &pad);
            *sum = xor16(sum, &out);
        }
    }

    /// AEZ-prf: the `tau` bytes a zero-length message enciphers to.
    fn prf(&self, delta: &[u8; BLOCK], out: &mut [u8]) {
        let mut ctr = [0u8; BLOCK];
        for chunk in out.chunks_mut(BLOCK) {
            let buf = self.aes10(&self.l[3], &xor16(delta, &ctr));
            chunk.copy_from_slice(&buf[..chunk.len()]);
            // ctr += 1, big-endian.
            for byte in ctr.iter_mut().rev() {
                *byte = byte.wrapping_add(1);
                if *byte != 0 {
                    break;
                }
            }
        }
    }

    /// AEZ-core, for a message of 32 bytes or more. `d` is 0 to
    /// encipher and 1 to decipher; the buffer is rewritten in place.
    fn core(&self, delta: &[u8; BLOCK], buf: &mut [u8], d: usize) {
        let sz = buf.len();
        let frag = sz % 32;
        let initial = sz - frag - 32;
        let zero = [0u8; BLOCK];

        let mut x = [0u8; BLOCK];
        // Pass 1 over everything but the fragment and the last two
        // blocks, leaving its intermediate values in place.
        let mut i_block = self.i[1];
        for n in 1..=initial / 32 {
            let at = (n - 1) * 32;
            let tmp = self.aes4(&self.j[0], &i_block, &self.l[n % 8], &buf[at + 16..at + 32]);
            let first = xor16(&buf[at..at + 16], &tmp);
            buf[at..at + 16].copy_from_slice(&first);
            let tmp = self.aes4(&zero, &self.i[0], &self.l[0], &first);
            let second = xor16(&buf[at + 16..at + 32], &tmp);
            buf[at + 16..at + 32].copy_from_slice(&second);
            x = xor16(&x, &second);
            if n.is_multiple_of(8) {
                double(&mut i_block);
            }
        }

        // The fragment's contribution to X.
        if frag >= BLOCK {
            let tmp = self.aes4(&zero, &self.i[1], &self.l[4], &buf[initial..initial + 16]);
            x = xor16(&x, &tmp);
            let mut pad = [0u8; BLOCK];
            pad[..frag - BLOCK].copy_from_slice(&buf[initial + 16..initial + frag]);
            pad[frag - BLOCK] = 0x80;
            let tmp = self.aes4(&zero, &self.i[1], &self.l[5], &pad);
            x = xor16(&x, &tmp);
        } else if frag > 0 {
            let mut pad = [0u8; BLOCK];
            pad[..frag].copy_from_slice(&buf[initial..initial + frag]);
            pad[frag] = 0x80;
            let tmp = self.aes4(&zero, &self.i[1], &self.l[4], &pad);
            x = xor16(&x, &tmp);
        }

        // S, from the last two blocks.
        let tail = sz - 32;
        let tmp = self.aes4(
            &zero,
            &self.i[1],
            &self.l[(1 + d) % 8],
            &buf[tail + 16..tail + 32],
        );
        let mut first = xor16(&x, &buf[tail..tail + 16]);
        first = xor16(&first, delta);
        first = xor16(&first, &tmp);
        buf[tail..tail + 16].copy_from_slice(&first);
        let tmp = self.aes10(&self.l[(1 + d) % 8], &first);
        let second = xor16(&buf[tail + 16..tail + 32], &tmp);
        buf[tail + 16..tail + 32].copy_from_slice(&second);
        let s = xor16(&first, &second);

        // Pass 2 over the intermediate values pass 1 left.
        let mut y = [0u8; BLOCK];
        let mut i_block = self.i[1];
        for n in 1..=initial / 32 {
            let at = (n - 1) * 32;
            let tmp = self.aes4(&self.j[1], &i_block, &self.l[n % 8], &s);
            let mut first = xor16(&buf[at..at + 16], &tmp);
            let mut second = xor16(&buf[at + 16..at + 32], &tmp);
            y = xor16(&y, &first);
            let tmp = self.aes4(&zero, &self.i[0], &self.l[0], &second);
            first = xor16(&first, &tmp);
            let tmp = self.aes4(&self.j[0], &i_block, &self.l[n % 8], &first);
            second = xor16(&second, &tmp);
            // The two blocks change places.
            buf[at..at + 16].copy_from_slice(&second);
            buf[at + 16..at + 32].copy_from_slice(&first);
            if n.is_multiple_of(8) {
                double(&mut i_block);
            }
        }

        // The fragment itself, and its contribution to Y.
        if frag >= BLOCK {
            let tmp = self.aes10(&self.l[4], &s);
            let block = xor16(&buf[initial..initial + 16], &tmp);
            buf[initial..initial + 16].copy_from_slice(&block);
            let tmp = self.aes4(&zero, &self.i[1], &self.l[4], &block);
            y = xor16(&y, &tmp);

            let rest = frag - BLOCK;
            let tmp = self.aes10(&self.l[5], &s);
            let mut pad = [0u8; BLOCK];
            for n in 0..rest {
                pad[n] = buf[initial + 16 + n] ^ tmp[n];
            }
            buf[initial + 16..initial + frag].copy_from_slice(&pad[..rest]);
            pad[rest] = 0x80;
            let tmp = self.aes4(&zero, &self.i[1], &self.l[5], &pad);
            y = xor16(&y, &tmp);
        } else if frag > 0 {
            let tmp = self.aes10(&self.l[4], &s);
            let mut pad = [0u8; BLOCK];
            for n in 0..frag {
                pad[n] = buf[initial + n] ^ tmp[n];
            }
            buf[initial..initial + frag].copy_from_slice(&pad[..frag]);
            pad[frag] = 0x80;
            let tmp = self.aes4(&zero, &self.i[1], &self.l[4], &pad);
            y = xor16(&y, &tmp);
        }

        // The last two blocks, which also change places.
        let tmp = self.aes10(&self.l[(2 - d) % 8], &buf[tail + 16..tail + 32]);
        let first = xor16(&buf[tail..tail + 16], &tmp);
        let tmp = self.aes4(&zero, &self.i[1], &self.l[(2 - d) % 8], &first);
        let mut second = xor16(&tmp, &buf[tail + 16..tail + 32]);
        second = xor16(&second, delta);
        second = xor16(&second, &y);
        buf[tail..tail + 16].copy_from_slice(&second);
        buf[tail + 16..tail + 32].copy_from_slice(&first);
    }

    /// AEZ-tiny, for a message shorter than 32 bytes: a balanced
    /// Feistel network whose round count grows as the message shrinks.
    fn tiny(&self, delta: &[u8; BLOCK], buf: &mut [u8], d: usize) {
        let n = buf.len();
        let zero = [0u8; BLOCK];
        let (i, rounds) = match n {
            1 => (7usize, 24i32),
            2 => (7, 16),
            _ if n < BLOCK => (7, 10),
            _ => (6, 8),
        };

        let half = n.div_ceil(2);
        let mut left = [0u8; BLOCK];
        let mut right = [0u8; BLOCK];
        left[..half].copy_from_slice(&buf[..half]);
        right[..half].copy_from_slice(&buf[n / 2..n / 2 + half]);
        let (mut mask, mut pad) = (0u8, 0x80u8);
        if n & 1 != 0 {
            // An odd length ends in a nibble: the right half is shifted
            // half a byte left so that both halves start on a byte.
            for k in 0..n / 2 {
                right[k] = (right[k] << 4) | (right[k + 1] >> 4);
            }
            right[n / 2] <<= 4;
            pad = 0x08;
            mask = 0xf0;
        }

        let mut j: i32 = 0;
        let step: i32 = if d != 0 {
            if n < BLOCK {
                let mut tmp = [0u8; BLOCK];
                tmp[..n].copy_from_slice(buf);
                tmp[0] |= 0x80;
                let tmp = xor16(delta, &tmp);
                let out = self.aes4(&zero, &self.i[1], &self.l[3], &tmp);
                left[0] ^= out[0] & 0x80;
            }
            j = rounds - 1;
            -1
        } else {
            1
        };

        for _ in 0..rounds / 2 {
            let mut tmp = [0u8; BLOCK];
            tmp[..half].copy_from_slice(&right[..half]);
            tmp[n / 2] = (tmp[n / 2] & mask) | pad;
            let mut tmp = xor16(&tmp, delta);
            tmp[15] ^= j as u8;
            let out = self.aes4(&zero, &self.i[1], &self.l[i], &tmp);
            left = xor16(&left, &out);

            let mut tmp = [0u8; BLOCK];
            tmp[..half].copy_from_slice(&left[..half]);
            tmp[n / 2] = (tmp[n / 2] & mask) | pad;
            let mut tmp = xor16(&tmp, delta);
            tmp[15] ^= (j + step) as u8;
            let out = self.aes4(&zero, &self.i[1], &self.l[i], &tmp);
            right = xor16(&right, &out);

            j += 2 * step;
        }
        let mut out = [0u8; 2 * BLOCK];
        out[..n / 2].copy_from_slice(&right[..n / 2]);
        out[n / 2..n / 2 + half].copy_from_slice(&left[..half]);
        if n & 1 != 0 {
            for k in (n / 2 + 1..n).rev() {
                out[k] = (out[k] >> 4) | (out[k - 1] << 4);
            }
            out[n / 2] = (left[0] >> 4) | (right[n / 2] & 0xf0);
        }
        buf.copy_from_slice(&out[..n]);
        if n < BLOCK && d == 0 {
            let mut tmp = [0u8; BLOCK];
            tmp[..n].copy_from_slice(&out[..n]);
            tmp[0] |= 0x80;
            let tmp = xor16(delta, &tmp);
            let last = self.aes4(&zero, &self.i[1], &self.l[3], &tmp);
            buf[0] ^= last[0] & 0x80;
        }
        left.zeroize();
        right.zeroize();
        out.zeroize();
    }

    /// Enciphers or deciphers `buf` in place under the tweak `delta`.
    fn cipher(&self, delta: &[u8; BLOCK], buf: &mut [u8], d: usize) {
        if buf.is_empty() {
            return;
        }
        if buf.len() < 2 * BLOCK {
            self.tiny(delta, buf, d);
        } else {
            self.core(delta, buf, d);
        }
    }
}

/// AEZ-hash: the tweak a key, a nonce, the associated data and `tau`
/// (in bits) give. Public for the reference implementation's
/// `hash.json`.
pub fn hash(key: &[u8], nonce: &[u8], ad: &[&[u8]], tau_bits: u32) -> [u8; BLOCK] {
    State::new(key).hash(nonce, ad, tau_bits)
}

/// AEZ-prf: the bytes a tweak stretches to, which is what an empty
/// message enciphers to. Public for the reference implementation's
/// `prf.json`.
pub fn prf(key: &[u8], delta: &[u8; BLOCK], out: &mut [u8]) {
    State::new(key).prf(delta, out);
}

/// AEZ encryption: `plaintext ‖ 0^tau` enciphered under `key`, the
/// nonce and every element of `ad`, written to `out`, which is
/// `plaintext.len() + tau` bytes long.
///
/// # Panics
///
/// If `out` is not exactly `plaintext.len() + tau` bytes.
pub fn encrypt(
    key: &[u8],
    nonce: &[u8],
    ad: &[&[u8]],
    tau: usize,
    plaintext: &[u8],
    out: &mut [u8],
) {
    assert_eq!(
        out.len(),
        plaintext.len() + tau,
        "AEZ expands a message by exactly tau bytes"
    );
    let state = State::new(key);
    let delta = state.hash(nonce, ad, (tau * 8) as u32);
    if plaintext.is_empty() {
        state.prf(&delta, out);
        return;
    }
    out[..plaintext.len()].copy_from_slice(plaintext);
    out[plaintext.len()..].fill(0);
    state.cipher(&delta, out, 0);
}

/// AEZ decryption. `out` is `ciphertext.len() - tau` bytes; the return
/// is whether the last `tau` bytes of the decipherment were zero, which
/// is the whole of AEZ's authentication. On `false` nothing of value is
/// left in `out`.
///
/// # Panics
///
/// If `ciphertext` is shorter than `tau`, or `out` is not exactly
/// `ciphertext.len() - tau` bytes.
pub fn decrypt(
    key: &[u8],
    nonce: &[u8],
    ad: &[&[u8]],
    tau: usize,
    ciphertext: &[u8],
    out: &mut [u8],
) -> bool {
    assert!(
        ciphertext.len() >= tau,
        "a ciphertext is at least tau bytes"
    );
    assert_eq!(
        out.len(),
        ciphertext.len() - tau,
        "AEZ shrinks a ciphertext by exactly tau bytes"
    );
    let state = State::new(key);
    let delta = state.hash(nonce, ad, (tau * 8) as u32);
    let mut buf: Vec<u8> = vec![0u8; ciphertext.len()];
    let ok = if ciphertext.len() == tau {
        state.prf(&delta, &mut buf);
        let mut sum = 0u8;
        for (a, b) in buf.iter().zip(ciphertext) {
            sum |= a ^ b;
        }
        sum == 0
    } else {
        buf.copy_from_slice(ciphertext);
        state.cipher(&delta, &mut buf, 1);
        let mut sum = 0u8;
        for byte in &buf[ciphertext.len() - tau..] {
            sum |= byte;
        }
        if sum == 0 {
            out.copy_from_slice(&buf[..out.len()]);
        }
        sum == 0
    };
    buf.zeroize();
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The AES round function AEZ uses is AES's own with the last
    /// round's MixColumns kept and no whitening key, so a ten-round run
    /// under an all-zero key schedule is a fixed permutation. What it
    /// is worth checking against is AEZ's own vectors, which the
    /// integration test in `core/osk-bip/tests/aez.rs` runs; this one
    /// states that enciphering and deciphering are inverse over every
    /// length the two branches divide at.
    #[test]
    fn every_length_round_trips() {
        let key = [7u8; 32];
        let ad: [&[u8]; 1] = [b"data"];
        for len in [0usize, 1, 2, 3, 15, 16, 17, 31, 32, 33, 47, 64, 65, 96, 129] {
            let message: Vec<u8> = (0..len).map(|i| i as u8).collect();
            for tau in [0usize, 4, 16] {
                let mut sealed = vec![0u8; len + tau];
                encrypt(&key, b"nonce", &ad, tau, &message, &mut sealed);
                let mut opened = vec![0u8; len];
                assert!(decrypt(&key, b"nonce", &ad, tau, &sealed, &mut opened));
                assert_eq!(opened, message, "length {len}, tau {tau}");
                if tau > 0 {
                    sealed[0] ^= 1;
                    assert!(!decrypt(&key, b"nonce", &ad, tau, &sealed, &mut opened));
                }
            }
        }
    }
}
