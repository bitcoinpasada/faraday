use hmac::{Hmac, KeyInit, Mac};
use sha2::block_api::compress256;
use sha2::{Digest, Sha256, Sha512};

use crate::Secret;
use crate::work::{Compute, Work};

/// SHA-256 of `data`: for small, fixed inputs (a key, a checksum's
/// bytes). A whole file is hashed with [`sha256_whole`].
pub fn sha256(data: &[u8]) -> [u8; 32] {
    Sha256::digest(data).into()
}

/// SHA-512 of `data`: for small inputs (a hash chain's step). The
/// result may be a secret's stretch; the caller wipes it.
pub fn sha512(data: &[u8]) -> [u8; 64] {
    Sha512::digest(data).into()
}

/// SHA-512 of `data` written into `out`, which may be a secret's stretch:
/// no result array is returned, so none is left on the stack. The
/// hasher's state is wiped as it drops (sha2's `zeroize` feature).
pub fn sha512_into(data: &[u8], out: &mut [u8; 64]) {
    let mut hasher = Sha512::new();
    hasher.update(data);
    hasher.finalize_into(out.into());
}

/// SHA-512 of the 64 bytes in `buf`, written over them: a hash chain's
/// next round with no second array.
pub fn sha512_again(buf: &mut [u8; 64]) {
    let mut hasher = Sha512::new();
    hasher.update(&buf[..]);
    hasher.finalize_into(buf.into());
}

/// Costs a SHA-256 pass over every byte: the SHA-256 of a whole file or
/// transaction, `bytes`, counted as [`Work::FileHash`].
pub fn sha256_whole(c: &Compute, bytes: &[u8]) -> [u8; 32] {
    c.count(Work::FileHash);
    Sha256::digest(bytes).into()
}

/// HMAC-SHA-512 of `msg` under `key`. Any key length is accepted, as
/// RFC 2104 specifies.
pub fn hmac_sha512(key: &[u8], msg: &[u8]) -> [u8; 64] {
    let mut mac = Hmac::<Sha512>::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(msg);
    mac.finalize().into_bytes().into()
}

/// HKDF with SHA-256 (RFC 5869): HKDF-Extract, PRK = HMAC-SHA-256(`salt`,
/// `ikm`) with an empty `salt` the same as 32 zero bytes, then
/// [`hkdf_sha256_expand`] into `out`. The pseudorandom key is held in a
/// [`Secret`] and wiped. `None`, with `out` untouched, when `out` is
/// longer than [`HKDF_SHA256_MAX`].
pub fn hkdf_sha256(salt: &[u8], ikm: &[u8], info: &[u8], out: &mut [u8]) -> Option<()> {
    let prk = Secret::new(hmac_sha256(salt, ikm));
    hkdf_sha256_expand(prk.expose(), info, out)
}

/// Costs a PBKDF2: `rounds` HMACs per 64 bytes of `out`. PBKDF2 with
/// HMAC-SHA-512 as the PRF (RFC 8018 §5.2). Fills `out`, which may be any length; BIP-39 uses 64
/// bytes and 2048 rounds.
pub fn pbkdf2_hmac_sha512(c: &Compute, password: &[u8], salt: &[u8], rounds: u32, out: &mut [u8]) {
    c.count(Work::Kdf);
    pbkdf2::pbkdf2_hmac::<Sha512>(password, salt, rounds, out);
}

/// HMAC-SHA-256 of `msg` under `key`. Any key length is accepted, as
/// RFC 2104 specifies.
pub fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(msg);
    mac.finalize().into_bytes().into()
}

/// The most bytes [`hkdf_sha256_expand`] makes: 255 blocks of 32, the
/// bound RFC 5869 §2.3 puts on `L`.
pub const HKDF_SHA256_MAX: usize = 255 * 32;

/// HKDF-Expand with SHA-256 (RFC 5869 §2.3), an HMAC-SHA-256 per 32
/// bytes of `out`: fills `out` with T(1) ‖ T(2) ‖ …, where T(0) is empty
/// and T(i) = HMAC-SHA-256(`prk`, T(i−1) ‖ `info` ‖ i). `prk` is a
/// pseudorandom key of at least 32 bytes: HKDF-Extract's output, or
/// bytes already uniformly random, as §3.3 allows. `None`, with `out`
/// untouched, when `out` is longer than [`HKDF_SHA256_MAX`].
pub fn hkdf_sha256_expand(prk: &[u8], info: &[u8], out: &mut [u8]) -> Option<()> {
    if out.len() > HKDF_SHA256_MAX {
        return None;
    }
    let mut t = zeroize::Zeroizing::new([0u8; 32]);
    let mut previous = 0;
    for (i, chunk) in out.chunks_mut(32).enumerate() {
        let mut mac = Hmac::<Sha256>::new_from_slice(prk).expect("HMAC accepts any key length");
        mac.update(&t[..previous]);
        mac.update(info);
        // At most 255 blocks, checked above.
        mac.update(&[(i + 1) as u8]);
        t.copy_from_slice(&mac.finalize().into_bytes());
        previous = 32;
        chunk.copy_from_slice(&t[..chunk.len()]);
    }
    Some(())
}

/// Costs a PBKDF2: `rounds` HMACs per 32 bytes of `out`. PBKDF2 with
/// HMAC-SHA-256 as the PRF (RFC 8018 §5.2). Fills `out`, which may be any length; SLIP-39's
/// round function uses it with half the master secret's length.
pub fn pbkdf2_hmac_sha256(c: &Compute, password: &[u8], salt: &[u8], rounds: u32, out: &mut [u8]) {
    c.count(Work::Kdf);
    pbkdf2_sha256(password, salt, rounds, out);
}

/// [`pbkdf2_hmac_sha256`] uncounted, for scrypt's own two single-round
/// passes: scrypt counts itself once.
pub(crate) fn pbkdf2_sha256(password: &[u8], salt: &[u8], rounds: u32, out: &mut [u8]) {
    pbkdf2::pbkdf2_hmac::<Sha256>(password, salt, rounds, out);
}

/// SHA-256's initial state (FIPS 180-4 §5.3.3).
const SHA256_IV: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

/// How many rounds [`pbkdf2_hmac_sha256_block`] runs between two calls to
/// its `between`.
pub const PBKDF2_STEP: u32 = 5_000;

/// Costs a PBKDF2 of `rounds` HMACs: PBKDF2 with HMAC-SHA-256 as the PRF
/// (RFC 8018 §5.2) for the one 32-byte block, `T_1`, that can be stopped
/// and counted. It is [`pbkdf2_hmac_sha256`] for a 32-byte `out`, run in
/// steps of [`PBKDF2_STEP`] rounds; before the first and after each step
/// `between` is called with the rounds done so far, and the work stops,
/// returning `None`, when it returns `false`. `None` also for 0 `rounds`.
/// The key is in a [`Secret`] and the sum is too. The HMAC is spelled out
/// (RFC 2104) on SHA-256's compression function: the states after the key
/// xor ipad and the key xor opad are worked out once, and each round is
/// then two compressions from them (the inner one of `u`, the outer one of
/// that hash, each in a block padded once), with nothing cloned and nothing
/// wiped per round. The `hmac` crate's states wipe on drop (with sha2's
/// `zeroize`); what it leaves on the stack is the padded key block and the
/// inner hash; this function's own copies of them, and its states and
/// blocks, are in [`Zeroizing`](zeroize::Zeroizing), wiped once, at the
/// end. `sha2`'s software compression keeps each block's message words in
/// its own frame, unwiped.
pub fn pbkdf2_hmac_sha256_block(
    c: &Compute,
    password: &[u8],
    salt: &[u8],
    rounds: u32,
    between: &mut dyn FnMut(u32) -> bool,
) -> Option<Secret<[u8; 32]>> {
    if rounds == 0 || !between(0) {
        return None;
    }
    c.count(Work::Kdf);
    // The key, hashed if longer than a block, padded with zeros to one.
    let mut block = zeroize::Zeroizing::new([0u8; 64]);
    if password.len() > 64 {
        let mut hasher = Sha256::new();
        hasher.update(password);
        hasher.finalize_into((&mut block[..32]).try_into().expect("32 bytes"));
    } else {
        block[..password.len()].copy_from_slice(password);
    }
    let mut ipad = zeroize::Zeroizing::new([0u8; 64]);
    let mut opad = zeroize::Zeroizing::new([0u8; 64]);
    for ((i, o), k) in ipad.iter_mut().zip(opad.iter_mut()).zip(block.iter()) {
        *i = k ^ 0x36;
        *o = k ^ 0x5c;
    }
    // U_1 = PRF(P, S || INT(1)), with the salt of any length, through the
    // buffered hasher: once.
    let mut u = zeroize::Zeroizing::new([0u8; 32]);
    {
        let mut inner = Sha256::new();
        inner.update(ipad.as_slice());
        inner.update(salt);
        inner.update(1u32.to_be_bytes());
        let mut mid = zeroize::Zeroizing::new([0u8; 32]);
        inner.finalize_into((&mut *mid).into());
        let mut outer = Sha256::new();
        outer.update(opad.as_slice());
        outer.update(mid.as_slice());
        outer.finalize_into((&mut *u).into());
    }
    let mut t = Secret::new(*u);
    // The states after one block of each pad, and the two blocks each round
    // fills: 32 bytes, 0x80 and the length, (64 + 32) * 8 bits, which are
    // the same every round.
    let mut inner_state = zeroize::Zeroizing::new(SHA256_IV);
    let mut outer_state = zeroize::Zeroizing::new(SHA256_IV);
    compress256(&mut inner_state, core::slice::from_ref(&*ipad));
    compress256(&mut outer_state, core::slice::from_ref(&*opad));
    let mut inner_block = zeroize::Zeroizing::new([0u8; 64]);
    let mut outer_block = zeroize::Zeroizing::new([0u8; 64]);
    for blk in [&mut inner_block, &mut outer_block] {
        blk[32] = 0x80;
        blk[56..].copy_from_slice(&768u64.to_be_bytes());
    }
    inner_block[..32].copy_from_slice(&*u);
    let mut work = zeroize::Zeroizing::new([0u32; 8]);
    let mut done = 1u32;
    while done < rounds {
        let stop = done.saturating_add(PBKDF2_STEP).min(rounds);
        for _ in done..stop {
            // The inner block holds U_i; the next U is the outer hash.
            work.copy_from_slice(&*inner_state);
            compress256(&mut work, core::slice::from_ref(&*inner_block));
            for (w, bytes) in work.iter().zip(outer_block.chunks_exact_mut(4)) {
                bytes.copy_from_slice(&w.to_be_bytes());
            }
            work.copy_from_slice(&*outer_state);
            compress256(&mut work, core::slice::from_ref(&*outer_block));
            for (w, bytes) in work.iter().zip(inner_block.chunks_exact_mut(4)) {
                bytes.copy_from_slice(&w.to_be_bytes());
            }
            for (a, b) in t.expose_mut().iter_mut().zip(inner_block.iter()) {
                *a ^= b;
            }
        }
        done = stop;
        if done < rounds && !between(done) {
            return None;
        }
    }
    between(rounds);
    Some(t)
}
