//! What this device encrypts under a passphrase the person types: a
//! key's words, a master seed, a note, or a wallet's recovery sheet,
//! each as one self-describing byte string (`docs/PLANNING.md` §16.71
//! decision 4, §16.112 rule 1).
//!
//! The format is `osk-backup`. `docs/BACKUP.md` writes it down byte for
//! byte; what follows is the summary the code is read against.
//!
//! ```text
//!  0   magic "OSKB", 4 bytes
//!  4   version, 1 byte = 3
//!  5   Argon2id memory cost, u32, KiB
//!  9   Argon2id passes, u32
//! 13   Argon2id lanes, u32
//! 17   salt, 32 bytes                    -- 0..49 is the associated data
//! 49   nonce, 24 bytes
//! 73   ciphertext, a multiple of 256 bytes, then a 16-byte tag
//! ```
//!
//! The version-3 plaintext is a kind byte, the payload's length as a
//! little-endian `u16`, the payload, and zero padding to the next
//! multiple of [`STEP`]. The kinds are [`KIND_WORDS`] (the version-2
//! plaintext, unchanged in meaning), [`KIND_SEED`] (a master seed of 16
//! to 64 bytes, for a SLIP-39 or codex32 key), [`KIND_NOTE`] (UTF-8
//! text) and [`KIND_SHEET`] (a wallet's descriptor, its name and a
//! note, each length-prefixed). Padding in steps is what §16.96 asked
//! for, generalised: a file's size says the order of magnitude of what
//! is in it and no more, and words and a seed are one step, so the two
//! kinds a key travels in are one length.
//!
//! Version 2 still opens: its plaintext is 34 bytes and always words.
//! Version 1 is not read; nothing had shipped when it was replaced.
//!
//! The key is `Argon2id(passphrase, salt)` at the cost the header
//! carries, and the associated data is the header, so a backup whose
//! parameters or salt were changed fails the tag rather than costing
//! work under the wrong ones. The memory is the caller's choice, which
//! in OpenSigner is the person's setting; a file made at another
//! device's setting opens here as long as this device can allocate that much, and one
//! that asks for more is refused by [`Error::Memory`] with the MiB it
//! needs, before any work is done. `tools/backup/decrypt.py` reads the
//! same bytes with two published libraries, so the format is not this
//! app's secret.
//!
//! A key's BIP-39 passphrase has no place in the format: a passphrase
//! key is sealed as its parent's words.
//!
//! This file is listed in `tools/lint-secrets.sh` and may hold no heap
//! text.

use alloc::vec::Vec;

use argon2::Params;
use chacha20poly1305::aead::inout::InOutBuf;
use chacha20poly1305::{AeadInOut, KeyInit, Tag, XChaCha20Poly1305, XNonce};
use osk_bip::bip39::{Language, Mnemonic};
use osk_crypto::{Secret, Zeroize};
use zeroize::Zeroizing;

use crate::{Cost, DEVICE_PARAMS, StretchError, argon2id};

/// The first four bytes of every backup.
pub const MAGIC: [u8; 4] = *b"OSKB";

/// The format version these bytes are written at.
pub const VERSION: u8 = 3;

/// The version whose 34-byte words-only plaintext is still read.
pub const VERSION_2: u8 = 2;

/// The Argon2id salt.
const SALT_LEN: usize = 32;
/// XChaCha20-Poly1305's nonce.
const NONCE_LEN: usize = 24;
/// Poly1305's tag.
const TAG_LEN: usize = 16;

/// Magic, version, the three costs and the salt: the associated data.
pub const HEADER_LEN: usize = 4 + 1 + 4 + 4 + 4 + SALT_LEN;
/// Where the nonce starts.
const NONCE_AT: usize = HEADER_LEN;
/// Where the ciphertext starts.
const CIPHERTEXT_AT: usize = NONCE_AT + NONCE_LEN;

/// The step a version-3 plaintext is padded to. A file's length says
/// which step its payload is in and nothing finer.
pub const STEP: usize = 256;

/// The plaintext's header: the kind byte and the payload's length.
const TYPE_LEN: usize = 1 + 2;

/// A key's words: the version-2 plaintext, unchanged in meaning.
pub const KIND_WORDS: u8 = 1;
/// A master seed of 16 to 64 bytes, which is what a SLIP-39 or a
/// codex32 key is.
pub const KIND_SEED: u8 = 2;
/// A note, as UTF-8 text.
pub const KIND_NOTE: u8 = 3;
/// A wallet's recovery sheet: its descriptor, its name and a note.
pub const KIND_SHEET: u8 = 4;

/// The plaintext's two leading bytes: word count and language.
const PREFIX_LEN: usize = 2;
/// The shortest entropy a mnemonic carries.
const MIN_ENTROPY: usize = 16;
/// The longest, and the room the words payload always makes.
const MAX_ENTROPY: usize = 32;

/// The words payload, the same length for every key.
const WORDS_LEN: usize = PREFIX_LEN + MAX_ENTROPY;

/// The version-2 plaintext, which is the words payload alone.
const V2_PLAIN_LEN: usize = WORDS_LEN;

/// The whole of a version-2 backup.
pub const V2_LEN: usize = CIPHERTEXT_AT + V2_PLAIN_LEN + TAG_LEN;

/// The shortest master seed a [`KIND_SEED`] payload carries.
pub const MIN_SEED: usize = 16;
/// The longest, and the room that payload always makes.
pub const MAX_SEED: usize = 64;

/// The seed payload, the same length for every seed: its length, then
/// 64 bytes zero-padded on the right. It shares a step with the words
/// payload, so a key travels at one file length whichever kind it is.
const SEED_LEN_PAYLOAD: usize = 1 + MAX_SEED;

/// The most a payload of any kind may hold. A note and a sheet are the
/// two that vary; this is the ceiling the reader allocates against.
pub const MAX_PAYLOAD: usize = 16 * 1024;

/// The whole of a backup that holds a key, whether as words or as a
/// seed: one step of plaintext.
pub const LEN: usize = CIPHERTEXT_AT + STEP + TAG_LEN;

/// The most Argon2id memory this build will ever try to allocate, in
/// KiB. A file above it is refused by its stated cost alone, so a
/// header claiming terabytes never reaches an allocator; a file below
/// it is refused only if the allocation itself fails, which is what
/// makes "what this device can allocate" the bound rather than what
/// this device writes.
pub const MAX_MEMORY_KIB: u32 = 4 * 1024 * 1024;

/// The random bytes one backup needs: the salt and the nonce.
pub const SEED_LEN: usize = SALT_LEN + NONCE_LEN;

/// Why a backup could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The bytes are not an `osk-backup` this build reads: the magic,
    /// the version, the length or the Argon2id cost is wrong.
    Format,
    /// The header is fine and the tag did not verify.
    Passphrase,
    /// The file asks for more Argon2id memory than this device can
    /// allocate. The value is the MiB it asks for.
    Memory(u32),
}

/// What a backup held.
pub enum Opened {
    /// A key's words.
    Words(Mnemonic),
    /// A master seed, 16 to 64 bytes.
    Seed(Zeroizing<Vec<u8>>),
    /// A note's UTF-8 bytes.
    Note(Zeroizing<Vec<u8>>),
    /// A wallet's recovery sheet.
    Sheet(Sheet),
}

/// A recovery sheet's three parts, as they were sealed.
pub struct Sheet {
    /// The wallet's descriptor.
    pub descriptor: Zeroizing<Vec<u8>>,
    /// What the person calls the wallet, empty where it has no name.
    pub name: Zeroizing<Vec<u8>>,
    /// The sheet's note, empty until one is written.
    pub note: Zeroizing<Vec<u8>>,
}

/// Whether `bytes` begin with the magic, whatever follows it.
pub fn is_backup(bytes: &[u8]) -> bool {
    bytes.starts_with(&MAGIC)
}

/// The cost `bytes` were written at, or why they cannot be opened here.
///
/// The three Argon2id parameters are checked before anything is
/// allocated or run, because the header is only associated data: a
/// changed cost can do no more than fail the tag, and the memory has
/// been allocated and the passes run by then.
pub fn read_header(bytes: &[u8]) -> Result<Cost, Error> {
    if !is_backup(bytes) || bytes.len() < CIPHERTEXT_AT + TAG_LEN {
        return Err(Error::Format);
    }
    let sealed = bytes.len() - CIPHERTEXT_AT - TAG_LEN;
    let length_fits = match bytes[4] {
        VERSION_2 => sealed == V2_PLAIN_LEN,
        VERSION => sealed > 0 && sealed.is_multiple_of(STEP) && sealed <= MAX_PAYLOAD + STEP,
        _ => false,
    };
    if !length_fits {
        return Err(Error::Format);
    }
    let u32_at =
        |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    let cost = Cost {
        memory_kib: u32_at(5),
        passes: u32_at(9),
        lanes: u32_at(13),
    };
    if !(Params::MIN_T_COST..=DEVICE_PARAMS.passes).contains(&cost.passes)
        || !(Params::MIN_P_COST..=DEVICE_PARAMS.lanes).contains(&cost.lanes)
        || cost.memory_kib < Params::MIN_M_COST
    {
        return Err(Error::Format);
    }
    if cost.memory_kib > MAX_MEMORY_KIB {
        return Err(Error::Memory(cost.memory_kib / 1024));
    }
    Ok(cost)
}

/// The cost `bytes` were written at, or `None` where they are not a
/// backup this build reads at all.
pub fn cost_of(bytes: &[u8]) -> Option<Cost> {
    read_header(bytes).ok()
}

/// The backup of `m` under `passphrase`, at `cost`, with the salt and
/// the nonce taken from `seed`. `None` where Argon2id refuses the cost.
///
/// `seed` is the caller's randomness and must be fresh for every
/// backup: a salt reused across two passphrases costs an attacker one
/// Argon2id run for both.
pub fn seal(m: &Mnemonic, passphrase: &[u8], cost: Cost, seed: &[u8; SEED_LEN]) -> Option<Vec<u8>> {
    let mut entropy = m.entropy();
    let n = entropy.expose().as_bytes().len();
    let mut payload = Secret::new([0u8; WORDS_LEN]);
    payload.expose_mut()[0] = m.word_count() as u8;
    payload.expose_mut()[1] = language_index(m.language())?;
    payload.expose_mut()[PREFIX_LEN..PREFIX_LEN + n].copy_from_slice(entropy.expose().as_bytes());
    entropy.zeroize();
    let out = seal_payload(KIND_WORDS, payload.expose(), passphrase, cost, seed);
    payload.zeroize();
    out
}

/// The backup of a master seed — what a SLIP-39 or a codex32 key is —
/// under `passphrase`.
pub fn seal_seed(
    master: &[u8],
    passphrase: &[u8],
    cost: Cost,
    seed: &[u8; SEED_LEN],
) -> Option<Vec<u8>> {
    if !(MIN_SEED..=MAX_SEED).contains(&master.len()) {
        return None;
    }
    let mut payload = Secret::new([0u8; SEED_LEN_PAYLOAD]);
    payload.expose_mut()[0] = master.len() as u8;
    payload.expose_mut()[1..1 + master.len()].copy_from_slice(master);
    let out = seal_payload(KIND_SEED, payload.expose(), passphrase, cost, seed);
    payload.zeroize();
    out
}

/// The backup of a note's UTF-8 bytes.
pub fn seal_note(
    text: &[u8],
    passphrase: &[u8],
    cost: Cost,
    seed: &[u8; SEED_LEN],
) -> Option<Vec<u8>> {
    if text.len() > MAX_PAYLOAD {
        return None;
    }
    seal_payload(KIND_NOTE, text, passphrase, cost, seed)
}

/// The backup of a wallet's recovery sheet: its descriptor, its name
/// and its note, each written as a little-endian `u16` length and that
/// many bytes, in that order.
pub fn seal_sheet(
    descriptor: &[u8],
    name: &[u8],
    note: &[u8],
    passphrase: &[u8],
    cost: Cost,
    seed: &[u8; SEED_LEN],
) -> Option<Vec<u8>> {
    let payload = sheet_payload(descriptor, name, note)?;
    seal_payload(KIND_SHEET, &payload, passphrase, cost, seed)
}

/// A [`KIND_SHEET`] payload: the descriptor, the name and the note,
/// each a little-endian `u16` length and that many bytes, in that
/// order. The blob's notes record holds a kept sheet as these same
/// bytes (`docs/PLANNING.md` §16.112 pass E3), so one encoder writes
/// both and [`payload_of`] reads both.
pub fn sheet_payload(descriptor: &[u8], name: &[u8], note: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
    let total = 6 + descriptor.len() + name.len() + note.len();
    if total > MAX_PAYLOAD {
        return None;
    }
    let mut payload: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::new());
    payload.try_reserve_exact(total).ok()?;
    for part in [descriptor, name, note] {
        let n = u16::try_from(part.len()).ok()?;
        payload.extend_from_slice(&n.to_le_bytes());
        payload.extend_from_slice(part);
    }
    Some(payload)
}

/// One payload of `kind`, sealed. The plaintext is the kind, the
/// payload's length, the payload and zero padding to the next
/// [`STEP`].
fn seal_payload(
    kind: u8,
    payload: &[u8],
    passphrase: &[u8],
    cost: Cost,
    seed: &[u8; SEED_LEN],
) -> Option<Vec<u8>> {
    let plain_len = (TYPE_LEN + payload.len()).div_ceil(STEP) * STEP;
    if payload.len() > MAX_PAYLOAD {
        return None;
    }
    let mut out: Vec<u8> = Vec::new();
    out.try_reserve_exact(CIPHERTEXT_AT + plain_len + TAG_LEN)
        .ok()?;
    out.resize(CIPHERTEXT_AT + plain_len + TAG_LEN, 0);
    out[..4].copy_from_slice(&MAGIC);
    out[4] = VERSION;
    out[5..9].copy_from_slice(&cost.memory_kib.to_le_bytes());
    out[9..13].copy_from_slice(&cost.passes.to_le_bytes());
    out[13..17].copy_from_slice(&cost.lanes.to_le_bytes());
    out[17..HEADER_LEN].copy_from_slice(&seed[..SALT_LEN]);
    out[NONCE_AT..CIPHERTEXT_AT].copy_from_slice(&seed[SALT_LEN..]);

    let mut plain: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::new());
    plain.try_reserve_exact(plain_len).ok()?;
    plain.resize(plain_len, 0);
    plain[0] = kind;
    plain[1..3].copy_from_slice(&(payload.len() as u16).to_le_bytes());
    plain[TYPE_LEN..TYPE_LEN + payload.len()].copy_from_slice(payload);

    let mut aad = [0u8; HEADER_LEN];
    aad.copy_from_slice(&out[..HEADER_LEN]);
    let key = stretch(&cost, passphrase, &seed[..SALT_LEN]).ok()?;
    let cipher = XChaCha20Poly1305::new_from_slice(key.expose()).expect("32-byte key");
    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&seed[SALT_LEN..]);
    let tag = cipher
        .encrypt_inout_detached(&XNonce::from(nonce), &aad, InOutBuf::from(&mut plain[..]))
        .ok()?;
    out[CIPHERTEXT_AT..CIPHERTEXT_AT + plain_len].copy_from_slice(&plain);
    out[CIPHERTEXT_AT + plain_len..].copy_from_slice(&tag);
    Some(out)
}

/// The key `bytes` hold, under `passphrase`. A backup of any other kind
/// is [`Error::Format`] here; [`open_payload`] is what reads one.
pub fn open(bytes: &[u8], passphrase: &[u8]) -> Result<Mnemonic, Error> {
    match open_payload(bytes, passphrase)? {
        Opened::Words(m) => Ok(m),
        _ => Err(Error::Format),
    }
}

/// What `bytes` hold, under `passphrase`.
pub fn open_payload(bytes: &[u8], passphrase: &[u8]) -> Result<Opened, Error> {
    let cost = read_header(bytes)?;
    let plain_len = bytes.len() - CIPHERTEXT_AT - TAG_LEN;
    let key = stretch(&cost, passphrase, &bytes[17..HEADER_LEN])?;
    let cipher = XChaCha20Poly1305::new_from_slice(key.expose()).expect("32-byte key");
    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&bytes[NONCE_AT..CIPHERTEXT_AT]);
    let mut tag = [0u8; TAG_LEN];
    tag.copy_from_slice(&bytes[CIPHERTEXT_AT + plain_len..]);
    let mut plain: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::new());
    plain
        .try_reserve_exact(plain_len)
        .map_err(|_| Error::Memory((plain_len / (1024 * 1024)) as u32))?;
    plain.extend_from_slice(&bytes[CIPHERTEXT_AT..CIPHERTEXT_AT + plain_len]);
    let opened = cipher
        .decrypt_inout_detached(
            &XNonce::from(nonce),
            &bytes[..HEADER_LEN],
            InOutBuf::from(&mut plain[..]),
            &Tag::from(tag),
        )
        .is_ok();
    if !opened {
        return Err(Error::Passphrase);
    }
    if bytes[4] == VERSION_2 {
        return words_of(&plain).ok_or(Error::Format);
    }
    let kind = plain[0];
    let len = usize::from(u16::from_le_bytes([plain[1], plain[2]]));
    if len > plain_len - TYPE_LEN || plain[TYPE_LEN + len..].iter().any(|&b| b != 0) {
        return Err(Error::Format);
    }
    // The payload must be in the last step the plaintext could have
    // been padded to, or the file was padded further than the format
    // allows and is not one this build wrote.
    if (TYPE_LEN + len).div_ceil(STEP) * STEP != plain_len {
        return Err(Error::Format);
    }
    payload_of(kind, &plain[TYPE_LEN..TYPE_LEN + len]).ok_or(Error::Format)
}

/// What a payload of `kind` holds, whether it came out of a file or out
/// of a slot of the blob's notes record. `None` where the bytes are not
/// that kind's.
pub fn payload_of(kind: u8, payload: &[u8]) -> Option<Opened> {
    match kind {
        KIND_WORDS => words_of(payload),
        KIND_SEED => seed_of(payload),
        KIND_NOTE => text_of(payload).map(Opened::Note),
        KIND_SHEET => sheet_of(payload),
        _ => None,
    }
}

/// The words a [`KIND_WORDS`] payload holds. The count says how many of
/// the 32 bytes are the key's; the rest are the pad, and a pad that is
/// not zero is not a backup.
fn words_of(payload: &[u8]) -> Option<Opened> {
    if payload.len() != WORDS_LEN {
        return None;
    }
    let words = usize::from(payload[0]);
    let lang = Language::ALL.get(usize::from(payload[1])).copied()?;
    let entropy = words * 4 / 3;
    if words * 4 != entropy * 3
        || !(MIN_ENTROPY..=MAX_ENTROPY).contains(&entropy)
        || payload[PREFIX_LEN + entropy..].iter().any(|&b| b != 0)
    {
        return None;
    }
    Mnemonic::from_entropy(lang, &payload[PREFIX_LEN..PREFIX_LEN + entropy])
        .ok()
        .map(Opened::Words)
}

/// The master seed a [`KIND_SEED`] payload holds.
fn seed_of(payload: &[u8]) -> Option<Opened> {
    if payload.len() != SEED_LEN_PAYLOAD {
        return None;
    }
    let n = usize::from(payload[0]);
    if !(MIN_SEED..=MAX_SEED).contains(&n) || payload[1 + n..].iter().any(|&b| b != 0) {
        return None;
    }
    let mut out: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::new());
    out.try_reserve_exact(n).ok()?;
    out.extend_from_slice(&payload[1..1 + n]);
    Some(Opened::Seed(out))
}

/// One length-prefixed field's bytes, kept in a buffer of its own.
fn text_of(bytes: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
    core::str::from_utf8(bytes).ok()?;
    let mut out: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::new());
    out.try_reserve_exact(bytes.len()).ok()?;
    out.extend_from_slice(bytes);
    Some(out)
}

/// The three parts a [`KIND_SHEET`] payload holds.
fn sheet_of(payload: &[u8]) -> Option<Opened> {
    let mut at = 0usize;
    let mut parts: [Option<Zeroizing<Vec<u8>>>; 3] = [None, None, None];
    for part in &mut parts {
        if at + 2 > payload.len() {
            return None;
        }
        let n = usize::from(u16::from_le_bytes([payload[at], payload[at + 1]]));
        at += 2;
        if at + n > payload.len() {
            return None;
        }
        *part = Some(text_of(&payload[at..at + n])?);
        at += n;
    }
    if at != payload.len() {
        return None;
    }
    let [descriptor, name, note] = parts;
    Some(Opened::Sheet(Sheet {
        descriptor: descriptor?,
        name: name?,
        note: note?,
    }))
}

/// A language's place in [`Language::ALL`], which is what the words
/// payload carries.
fn language_index(lang: Language) -> Option<u8> {
    Language::ALL
        .into_iter()
        .position(|l| l == lang)
        .map(|i| i as u8)
}

/// `Argon2id(passphrase, salt)`, 32 bytes, at `cost`: [`argon2id`],
/// with a cost the device cannot spare the memory for named in MiB.
fn stretch(cost: &Cost, passphrase: &[u8], salt: &[u8]) -> Result<Secret<[u8; 32]>, Error> {
    argon2id(cost, passphrase, salt).map_err(|e| match e {
        StretchError::Memory => Error::Memory(cost.memory_kib / 1024),
        StretchError::Params => Error::Format,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cost no slower than a test wants; the device writes the
    /// person's setting.
    const CHEAP: Cost = Cost {
        memory_kib: 32,
        passes: 1,
        lanes: 1,
    };

    fn seed() -> [u8; SEED_LEN] {
        let mut s = [0u8; SEED_LEN];
        for (i, b) in s.iter_mut().enumerate() {
            *b = i as u8;
        }
        s
    }

    #[test]
    fn a_backup_opens_to_the_same_key() {
        let m = Mnemonic::from_entropy(Language::English, &[7u8; 32]).unwrap();
        let bytes = seal(&m, b"correct horse", CHEAP, &seed()).unwrap();
        assert_eq!(bytes.len(), LEN);
        let back = open(&bytes, b"correct horse").unwrap();
        assert_eq!(back.indices(), m.indices());
        assert_eq!(back.language(), m.language());
    }

    #[test]
    fn another_passphrase_and_a_changed_header_open_nothing() {
        let m = Mnemonic::from_entropy(Language::Japanese, &[3u8; 16]).unwrap();
        let bytes = seal(&m, b"correct horse", CHEAP, &seed()).unwrap();
        assert_eq!(bytes.len(), LEN);
        assert_eq!(open(&bytes, b"correct hors").err(), Some(Error::Passphrase));

        let mut flipped = bytes.clone();
        flipped[20] ^= 1;
        assert_eq!(
            open(&flipped, b"correct horse").err(),
            Some(Error::Passphrase)
        );

        let mut version = bytes.clone();
        version[4] = 1;
        assert_eq!(open(&version, b"correct horse").err(), Some(Error::Format));

        let mut magic = bytes;
        magic[0] = b'X';
        assert_eq!(open(&magic, b"correct horse").err(), Some(Error::Format));
    }

    /// A 12-word key's pad is 16 zero bytes. One of them set is a
    /// backup this build will not open, however good the passphrase.
    #[test]
    fn a_backup_whose_pad_is_not_zero_is_refused() {
        let m = Mnemonic::from_entropy(Language::English, &[3u8; 16]).unwrap();
        let mut payload = [0u8; WORDS_LEN];
        payload[0] = 12;
        payload[PREFIX_LEN..PREFIX_LEN + 16].copy_from_slice(&[3u8; 16]);
        payload[WORDS_LEN - 1] = 1;
        let bytes = reseal(&m, b"correct horse", KIND_WORDS, &payload);
        assert_eq!(open(&bytes, b"correct horse").err(), Some(Error::Format));
    }

    /// One backup whose payload is `payload` of `kind` rather than what
    /// [`seal`] would have written, sealed so that its tag verifies.
    fn reseal(m: &Mnemonic, passphrase: &[u8], kind: u8, payload: &[u8]) -> Vec<u8> {
        let mut bytes = seal(m, passphrase, CHEAP, &seed()).unwrap();
        let plain_len = bytes.len() - CIPHERTEXT_AT - TAG_LEN;
        let key = stretch(&CHEAP, passphrase, &seed()[..SALT_LEN]).unwrap();
        let cipher = XChaCha20Poly1305::new_from_slice(key.expose()).unwrap();
        let mut nonce = [0u8; NONCE_LEN];
        nonce.copy_from_slice(&seed()[SALT_LEN..]);
        let mut buf = alloc::vec![0u8; plain_len];
        buf[0] = kind;
        buf[1..3].copy_from_slice(&(payload.len() as u16).to_le_bytes());
        buf[TYPE_LEN..TYPE_LEN + payload.len()].copy_from_slice(payload);
        let aad: Vec<u8> = bytes[..HEADER_LEN].to_vec();
        let tag = cipher
            .encrypt_inout_detached(&XNonce::from(nonce), &aad, InOutBuf::from(&mut buf[..]))
            .unwrap();
        bytes[CIPHERTEXT_AT..CIPHERTEXT_AT + plain_len].copy_from_slice(&buf);
        bytes[CIPHERTEXT_AT + plain_len..].copy_from_slice(&tag);
        bytes
    }

    #[test]
    fn an_unreadable_cost_is_refused_before_any_work() {
        let m = Mnemonic::from_entropy(Language::English, &[0u8; 16]).unwrap();
        let mut bytes = seal(&m, b"correct horse", CHEAP, &seed()).unwrap();
        bytes[5..9].copy_from_slice(&67_108_864u32.to_le_bytes());
        assert_eq!(
            open(&bytes, b"correct horse").err(),
            Some(Error::Memory(65_536))
        );
    }

    /// A master seed, a note and a sheet each come back as they went
    /// in, and a key's two kinds are one file length.
    #[test]
    fn every_kind_comes_back_whole() {
        let master: Vec<u8> = (0..32u8).collect();
        let bytes = seal_seed(&master, b"correct horse", CHEAP, &seed()).unwrap();
        assert_eq!(bytes.len(), LEN);
        match open_payload(&bytes, b"correct horse").unwrap() {
            Opened::Seed(out) => assert_eq!(out.as_slice(), master.as_slice()),
            _ => panic!("a seed"),
        }

        let note = b"keys in the safe, words with the notary";
        let bytes = seal_note(note, b"correct horse", CHEAP, &seed()).unwrap();
        match open_payload(&bytes, b"correct horse").unwrap() {
            Opened::Note(out) => assert_eq!(out.as_slice(), note.as_slice()),
            _ => panic!("a note"),
        }

        let bytes = seal_sheet(
            b"wpkh([73c5da0a/84h/0h/0h]xpub.../0/*)",
            b"Cold",
            b"two of three",
            b"correct horse",
            CHEAP,
            &seed(),
        )
        .unwrap();
        match open_payload(&bytes, b"correct horse").unwrap() {
            Opened::Sheet(sheet) => {
                assert_eq!(sheet.name.as_slice(), b"Cold");
                assert_eq!(sheet.note.as_slice(), b"two of three");
            }
            _ => panic!("a sheet"),
        }
    }

    /// A note long enough to need a second step of padding makes a
    /// longer file, and one short enough for the first step makes the
    /// same file length as a key does.
    #[test]
    fn a_files_length_says_only_which_step_it_is_in() {
        let short = seal_note(&[b'x'; 100], b"correct horse", CHEAP, &seed()).unwrap();
        let long = seal_note(&[b'x'; 300], b"correct horse", CHEAP, &seed()).unwrap();
        assert_eq!(short.len(), LEN);
        assert_eq!(long.len(), LEN + STEP);
    }
}
