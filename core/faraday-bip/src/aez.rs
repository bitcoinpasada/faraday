//! AEZ v5, the wide-block robust AEAD LND's aezeed enciphers a cipher
//! seed with.
//!
//! This is a port of the AEZ v5 reference code as Yawning Angel's Go
//! implementation holds it, which is what LND itself calls; the file
//! read and its digest are in `tools/reference/aezeed/README.md`. The
//! primitive is here rather than in a crate because no Rust AEZ crate
//! is maintained, and because AEZ is a few hundred lines over the AES
//! round function and nothing else; that round is the `aes` crate's.
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
use faraday_crypto::{Zeroize, blake2b};

/// AEZ's block, which is AES's.
const BLOCK: usize = 16;
/// The extracted key: `I ‖ J ‖ L`.
const EXTRACTED: usize = 3 * BLOCK;

/// One AES round over the whole state — SubBytes, ShiftRows,
/// MixColumns, AddRoundKey — which is what AEZ's `AES4` and `AES10`
/// apply. MixColumns runs in every round, the last included, and the
/// key is never added before the first: AEZ is not AES, it is AES's
/// round function used as a public permutation.
///
/// The round is the `aes` crate's (RustCrypto), FIPS 197's round as
/// AES-NI's `AESENC` runs it: the processor's AES instructions where it
/// has them, a fixsliced round without tables where it does not, so the
/// seed's bytes choose no memory read.
fn round(state: &mut [u8; BLOCK], key: &[u8; BLOCK]) {
    let mut block = aes::hazmat::Block::from(*state);
    aes::hazmat::cipher_round(&mut block, &aes::hazmat::Block::from(*key));
    state.copy_from_slice(&block);
    block.as_mut_slice().zeroize();
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
        let (mut ik, mut jk, mut lk) = (sub[0], sub[1], sub[2]);
        sub.zeroize();

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
        ik.zeroize();
        jk.zeroize();
        lk.zeroize();
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
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
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
    /// integration test in `core/faraday-bip/tests/all/aez.rs` runs; this one
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
