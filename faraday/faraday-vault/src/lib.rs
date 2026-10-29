//! The Faraday vault (`docs/VAULT.md`): one file, four equal slots, each
//! opened by its own passphrase. A slot no passphrase opens is random
//! bytes of the same length, so the file does not say how many slots are
//! in use.
//!
//! - [`read_header`] checks a file's magic, version, slot size, length
//!   and Argon2id cost before anything is allocated (§1 to §3).
//! - [`create`] writes a new vault from one to four passphrases and the
//!   contents of each (§8).
//! - [`open`] derives the slot key once and tries it against all four
//!   slots, so the work is the same whichever slot opens (§4).
//! - [`Opened::seal`] re-encrypts the open slot under a fresh nonce with
//!   its seal count one higher, and copies the other three byte for byte
//!   (§5).
//! - [`records`] is the slot's plaintext: typed records of fields (§7),
//!   and the seal count ([`Seals`]) this crate alone writes.
//!
//! Every passphrase is put in Unicode NFC before Argon2id, at creation
//! and at opening, so a passphrase typed in either form derives one key
//! (§3). Every slot's plaintext begins with [`COMMIT_LEN`] zero bytes,
//! checked in constant time once its tag verifies, so a slot made to
//! verify under two keys is refused under both (§4).
//!
//! The cryptography is `faraday-backup`'s: Argon2id version 1.3 and
//! XChaCha20-Poly1305, with the working memory asked for rather than
//! taken. Nothing here draws randomness: the caller passes the seed a new
//! vault is made from and the nonce a seal uses.
//!
//! `no_std` + `alloc`.

#![no_std]

extern crate alloc;

pub mod codex32;
pub mod entries;
pub mod layouts;
pub mod records;
pub mod totp;

use alloc::string::String;
use alloc::vec::Vec;
use core::ops::Range;

use argon2::{Algorithm, Argon2, Block, Params, Version};
use chacha20::ChaCha20;
use chacha20::cipher::{KeyIvInit, StreamCipher};
use chacha20poly1305::aead::inout::InOutBuf;
use chacha20poly1305::{AeadInOut, KeyInit, Tag, XChaCha20Poly1305, XNonce};
use faraday_crypto::work::{Compute, Work};
use faraday_crypto::{Secret, Zeroize};
use hmac::{Hmac, KeyInit as HmacInit, Mac};
use sha2::Sha256;
use unicode_normalization::UnicodeNormalization;
use zeroize::Zeroizing;

pub use faraday_backup::Cost;
pub use records::{Contents, Field, Record, Seals};

/// The zero bytes every slot's plaintext begins with, which commit the
/// slot to its key (§4).
pub const COMMIT_LEN: usize = 32;

/// The Unicode version a passphrase is put in NFC under: the
/// normaliser's tables.
pub use unicode_normalization::UNICODE_VERSION;

/// The first four bytes of every vault.
pub const MAGIC: [u8; 4] = faraday_intake::VAULT_MAGIC;
/// The only version this build reads and writes.
pub const VERSION: u8 = 1;
/// The header: magic, version, memory, passes, lanes, slot length, salt.
pub const HEADER_LEN: usize = 4 + 1 + 4 + 4 + 4 + 4 + SALT_LEN;
/// The Argon2id salt.
pub const SALT_LEN: usize = 32;
/// An XChaCha20-Poly1305 nonce.
pub const NONCE_LEN: usize = 24;
/// A Poly1305 tag.
pub const TAG_LEN: usize = 16;
/// Every vault has four slots, whether one passphrase is set or four.
pub const SLOTS: usize = 4;
/// The slot plaintext lengths a reader accepts (§2), smallest first.
pub const SLOT_SIZES: [u32; 4] = [65_536, 262_144, 1_048_576, 4_194_304];
/// The default slot size: keys, wallets, notes and about a hundred
/// entries.
pub const DEFAULT_SLOT: u32 = 262_144;

/// The least Argon2id memory a reader accepts, in KiB: 8 MiB (§3).
pub const MIN_MEMORY_KIB: u32 = 8 * 1024;
/// The most, in KiB: 4 GiB.
pub const MAX_MEMORY_KIB: u32 = 4 * 1024 * 1024;
/// The most passes a reader accepts.
pub const MAX_PASSES: u32 = 10;
/// The least memory Faraday creates a vault with, in KiB: 64 MiB.
pub const CREATE_MIN_MEMORY_KIB: u32 = 64 * 1024;
/// The fewest passes Faraday creates a vault with.
pub const CREATE_MIN_PASSES: u32 = 2;

/// The HKDF info that turns the stretched passphrase into a slot key.
const SLOT_INFO: &[u8] = b"Faraday vault v1 slot";
/// The domain a new vault's keystream is drawn under.
const CREATE_TAG: &[u8] = b"Faraday vault create";

/// Why a vault could not be read, opened, made or sealed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The file does not begin with `FVLT`.
    Magic,
    /// A version this build does not read.
    Version,
    /// A slot length §2 does not list.
    SlotSize,
    /// The file is not 53 + 4 × (L + 40) bytes.
    Length,
    /// The stated Argon2id cost is outside §3's limits.
    Cost,
    /// The Argon2id memory could not be allocated: the MiB it needs.
    Memory(u32),
    /// No slot opens under this passphrase.
    Passphrase,
    /// More than one slot opened, which this format never writes.
    TwoSlots,
    /// A slot's tag verified and its first [`COMMIT_LEN`] bytes are not
    /// zero, which this format never writes: a slot made to verify under
    /// more than one key.
    Commitment,
    /// The opened slot's contents are not what §7 allows.
    Contents(records::Fault),
    /// The contents do not fit in a slot.
    Full,
    /// No passphrase, more than four, or an empty one.
    Passphrases,
    /// Two passphrases are the same, or the same in Unicode NFC.
    SamePassphrase,
    /// The file being sealed is not the file this slot was opened from.
    OtherFile,
}

/// What a vault's header states. None of it is secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// What a guess costs.
    pub cost: Cost,
    /// Each slot's plaintext length.
    pub slot_len: u32,
    /// The Argon2id salt, which with the length is the vault's identity
    /// on a stick (§6).
    pub salt: [u8; SALT_LEN],
}

impl Header {
    fn to_bytes(self) -> [u8; HEADER_LEN] {
        let mut h = [0u8; HEADER_LEN];
        h[0..4].copy_from_slice(&MAGIC);
        h[4] = VERSION;
        h[5..9].copy_from_slice(&self.cost.memory_kib.to_le_bytes());
        h[9..13].copy_from_slice(&self.cost.passes.to_le_bytes());
        h[13..17].copy_from_slice(&self.cost.lanes.to_le_bytes());
        h[17..21].copy_from_slice(&self.slot_len.to_le_bytes());
        h[21..53].copy_from_slice(&self.salt);
        h
    }
}

/// A vault's whole length for a slot length.
pub fn file_len(slot_len: u32) -> usize {
    HEADER_LEN + SLOTS * (slot_len as usize + NONCE_LEN + TAG_LEN)
}

pub use faraday_intake::is_vault;

/// Checks a vault's header and length (§4 step 1).
pub fn read_header(bytes: &[u8]) -> Result<Header, Error> {
    if !bytes.starts_with(&MAGIC) {
        return Err(Error::Magic);
    }
    if bytes.len() < HEADER_LEN {
        return Err(Error::Length);
    }
    if bytes[4] != VERSION {
        return Err(Error::Version);
    }
    let u32_at =
        |at: usize| u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    let cost = Cost {
        memory_kib: u32_at(5),
        passes: u32_at(9),
        lanes: u32_at(13),
    };
    let slot_len = u32_at(17);
    if !SLOT_SIZES.contains(&slot_len) {
        return Err(Error::SlotSize);
    }
    if bytes.len() != file_len(slot_len) {
        return Err(Error::Length);
    }
    check_cost(&cost)?;
    let mut salt = [0u8; SALT_LEN];
    salt.copy_from_slice(&bytes[21..53]);
    Ok(Header {
        cost,
        slot_len,
        salt,
    })
}

fn check_cost(cost: &Cost) -> Result<(), Error> {
    if !(MIN_MEMORY_KIB..=MAX_MEMORY_KIB).contains(&cost.memory_kib)
        || !(1..=MAX_PASSES).contains(&cost.passes)
        || cost.lanes != 1
    {
        return Err(Error::Cost);
    }
    Ok(())
}

/// A slot key: HKDF-SHA256 of the stretched passphrase. Wiped on drop.
pub struct SlotKey(Secret<[u8; 32]>);

/// Costs an Argon2id at the header's cost, then HKDF-SHA256 with an
/// empty salt and the slot info: derives the slot key a passphrase
/// gives under `header` (§3), from the passphrase's NFC form.
pub fn derive(c: &Compute, header: &Header, passphrase: &str) -> Result<SlotKey, Error> {
    let nfc = nfc(passphrase)?;
    let k = stretch(c, &header.cost, nfc.as_bytes(), &header.salt)?;
    Ok(SlotKey(hkdf_sha256(k.expose(), SLOT_INFO)))
}

/// A passphrase in Unicode Normalization Form C (UAX #15), in a buffer
/// sized once and wiped on drop. Nothing else is done to it: no character
/// is mapped or refused. The normaliser's own buffer of combining marks
/// is not wiped.
fn nfc(passphrase: &str) -> Result<Zeroizing<String>, Error> {
    let len: usize = passphrase.nfc().map(char::len_utf8).sum();
    let mut out = Zeroizing::new(String::new());
    out.try_reserve_exact(len)
        .map_err(|_| Error::Memory((len / (1024 * 1024)) as u32 + 1))?;
    // The capacity is the whole form, so the buffer never moves and
    // leaves no copy behind.
    for ch in passphrase.nfc() {
        out.push(ch);
    }
    Ok(out)
}

/// Costs an Argon2id run: `cost.memory_kib` of memory filled
/// `cost.passes` times, asked for so that a cost this machine cannot
/// meet is an error. Argon2id(passphrase, salt) at `cost`, 32 bytes.
fn stretch(
    c: &Compute,
    cost: &Cost,
    passphrase: &[u8],
    salt: &[u8],
) -> Result<Secret<[u8; 32]>, Error> {
    c.count(Work::Kdf);
    let params =
        Params::new(cost.memory_kib, cost.passes, cost.lanes, Some(32)).map_err(|_| Error::Cost)?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params.clone());
    let blocks = params.block_count();
    let mut memory: Vec<Block> = Vec::new();
    memory
        .try_reserve_exact(blocks)
        .map_err(|_| Error::Memory(cost.memory_kib / 1024))?;
    memory.resize(blocks, Block::default());
    let mut derived = Secret::new([0u8; 32]);
    let hashed = argon
        .hash_password_into_with_memory(passphrase, salt, derived.expose_mut(), &mut memory)
        .is_ok();
    // What the memory holds is derived from the passphrase.
    memory.fill(Block::default());
    core::hint::black_box(&memory);
    if hashed {
        Ok(derived)
    } else {
        Err(Error::Cost)
    }
}

/// HKDF-SHA256 (RFC 5869) with an empty salt, 32 bytes of output: one
/// extract and the first expand block.
fn hkdf_sha256(ikm: &[u8], info: &[u8]) -> Secret<[u8; 32]> {
    // An empty salt is HashLen zeros, which HMAC pads to the same key.
    let mut extract =
        <Hmac<Sha256> as HmacInit>::new_from_slice(&[0u8; 32]).expect("any key length");
    extract.update(ikm);
    let prk = Secret::new(<[u8; 32]>::from(extract.finalize().into_bytes()));
    let mut expand =
        <Hmac<Sha256> as HmacInit>::new_from_slice(prk.expose()).expect("any key length");
    expand.update(info);
    expand.update(&[1]);
    Secret::new(<[u8; 32]>::from(expand.finalize().into_bytes()))
}

/// Where slot `i` starts.
fn slot_at(slot_len: u32, i: usize) -> usize {
    HEADER_LEN + i * (slot_len as usize + NONCE_LEN + TAG_LEN)
}

/// The bytes of slot `i` in a vault whose slots are `slot_len` long: its
/// nonce, ciphertext and tag, 40 + *L* bytes (§1).
pub fn slot_range(slot_len: u32, i: usize) -> Range<usize> {
    let at = slot_at(slot_len, i);
    at..at + NONCE_LEN + slot_len as usize + TAG_LEN
}

/// Slot `i`'s associated data: the header, then the slot number.
fn associated(header: &[u8], i: usize) -> [u8; HEADER_LEN + 1] {
    let mut ad = [0u8; HEADER_LEN + 1];
    ad[..HEADER_LEN].copy_from_slice(&header[..HEADER_LEN]);
    ad[HEADER_LEN] = i as u8;
    ad
}

/// Encrypts the commitment block and `body` (the slot length less
/// [`COMMIT_LEN`], as [`Contents::encode`] writes it) into slot `i` of
/// `file`. The plaintext is wiped once encrypted.
fn encrypt_slot(
    file: &mut [u8],
    key: &SlotKey,
    i: usize,
    nonce: &[u8; NONCE_LEN],
    body: &[u8],
) -> Result<(), Error> {
    let len = COMMIT_LEN + body.len();
    let mut plain: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::new());
    plain
        .try_reserve_exact(len)
        .map_err(|_| Error::Memory((len / (1024 * 1024)) as u32 + 1))?;
    plain.resize(COMMIT_LEN, 0);
    plain.extend_from_slice(body);
    let ad = associated(file, i);
    let cipher = XChaCha20Poly1305::new_from_slice(key.0.expose()).expect("32-byte key");
    let tag = cipher
        .encrypt_inout_detached(&XNonce::from(*nonce), &ad, InOutBuf::from(&mut plain[..]))
        .expect("a slot is far below the cipher's limit");
    let at = slot_at(len as u32, i);
    file[at..at + NONCE_LEN].copy_from_slice(nonce);
    file[at + NONCE_LEN..at + NONCE_LEN + len].copy_from_slice(&plain);
    file[at + NONCE_LEN + len..at + NONCE_LEN + len + TAG_LEN].copy_from_slice(&tag);
    Ok(())
}

/// What trying one slot under a key gave.
enum Tried {
    /// The tag failed.
    Shut,
    /// The tag verified and the commitment block is zero: the slot's
    /// whole plaintext.
    Opened(Zeroizing<Vec<u8>>),
    /// The tag verified and the commitment block is not zero.
    Uncommitted,
}

/// Decrypts slot `i` of `file` in a buffer of its own and checks its
/// commitment block. The buffer is wiped unless the slot opens.
fn decrypt_slot(file: &[u8], header: &Header, key: &SlotKey, i: usize) -> Result<Tried, Error> {
    let len = header.slot_len as usize;
    let at = slot_at(header.slot_len, i);
    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&file[at..at + NONCE_LEN]);
    let mut tag = [0u8; TAG_LEN];
    tag.copy_from_slice(&file[at + NONCE_LEN + len..at + NONCE_LEN + len + TAG_LEN]);
    let mut plain: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::new());
    plain
        .try_reserve_exact(len)
        .map_err(|_| Error::Memory((len / (1024 * 1024)) as u32 + 1))?;
    plain.extend_from_slice(&file[at + NONCE_LEN..at + NONCE_LEN + len]);
    let ad = associated(file, i);
    let cipher = XChaCha20Poly1305::new_from_slice(key.0.expose()).expect("32-byte key");
    let ok = cipher
        .decrypt_inout_detached(
            &XNonce::from(nonce),
            &ad,
            InOutBuf::from(&mut plain[..]),
            &Tag::from(tag),
        )
        .is_ok();
    if !ok {
        plain.zeroize();
        return Ok(Tried::Shut);
    }
    if committed(&plain[..COMMIT_LEN]) {
        Ok(Tried::Opened(plain))
    } else {
        plain.zeroize();
        Ok(Tried::Uncommitted)
    }
}

/// Whether every byte of the commitment block is zero: an OR of every
/// byte, compared once, so the time does not depend on which byte is not.
fn committed(block: &[u8]) -> bool {
    let any = block.iter().fold(0u8, |acc, &b| acc | b);
    core::hint::black_box(any) == 0
}

/// One open slot: which slot, under which key, of which file, and its
/// seal count. The key stays for sealing and is wiped with this.
pub struct Opened {
    /// The slot that opened, 0 to 3. Never shown: §9 has nothing on
    /// screen count or name slots.
    slot: usize,
    key: SlotKey,
    header: Header,
    /// The seal count the slot was opened with, or the last seal of this
    /// opening wrote.
    seals: Seals,
}

impl Opened {
    /// The header of the file this slot was opened from.
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// The slot's seal count: as it was opened, or as the last
    /// [`Opened::seal`] of this opening wrote it.
    pub fn seals(&self) -> Seals {
        self.seals
    }

    /// The slot's position in the file, 0 to 3, for telling copies of one
    /// slot apart ([`slot_range`]). Never shown: nothing on screen counts
    /// or names slots (§9).
    pub fn position(&self) -> usize {
        self.slot
    }

    /// Costs one XChaCha20-Poly1305 decryption of a slot (up to 4 MiB),
    /// counted as a file hash. The seal count of this slot's position in
    /// `file`, another copy of the vault (the same header), read with the
    /// slot key held, with no Argon2id (§5.3 of the vault hardening
    /// spec). `Error::Passphrase` where the slot does not open under the
    /// key (a damaged copy), `Error::OtherFile` for another vault's file.
    pub fn copy_seals(&self, c: &Compute, file: &[u8]) -> Result<Seals, Error> {
        c.count(Work::FileHash);
        let header = read_header(file)?;
        if header != self.header {
            return Err(Error::OtherFile);
        }
        match decrypt_slot(file, &header, &self.key, self.slot)? {
            Tried::Opened(plain) => {
                let (_, seals) = Contents::decode(&plain[COMMIT_LEN..]).map_err(Error::Contents)?;
                Ok(seals)
            }
            Tried::Uncommitted => Err(Error::Commitment),
            Tried::Shut => Err(Error::Passphrase),
        }
    }

    /// The vault with this slot holding `contents` under a fresh nonce,
    /// its seal count one higher with the time `now` (Unix seconds, `None`
    /// when the clock is not set), and every other slot copied from `file`
    /// byte for byte (§5). The new count is kept, so two seals of one
    /// opening count two.
    pub fn seal(
        &mut self,
        file: &[u8],
        contents: &Contents,
        nonce: &[u8; NONCE_LEN],
        now: Option<u64>,
    ) -> Result<Vec<u8>, Error> {
        let header = read_header(file)?;
        if header != self.header {
            return Err(Error::OtherFile);
        }
        let count =
            self.seals
                .count
                .checked_add(1)
                .ok_or(Error::Contents(records::Fault::Value(
                    records::kind::SEALS,
                    records::field::SEAL_COUNT,
                )))?;
        let seals = Seals { count, last: now };
        let body = contents.encode(header.slot_len as usize - COMMIT_LEN, seals)?;
        let mut out: Vec<u8> = Vec::new();
        out.try_reserve_exact(file.len())
            .map_err(|_| Error::Memory((file.len() / (1024 * 1024)) as u32 + 1))?;
        out.extend_from_slice(file);
        encrypt_slot(&mut out, &self.key, self.slot, nonce, &body)?;
        self.seals = seals;
        Ok(out)
    }
}

/// Costs what `derive` costs. Opens the slot `passphrase` opens (§4):
/// derives the key once from the passphrase's NFC form, tries all four
/// slots, and parses the one that opens.
pub fn open(c: &Compute, file: &[u8], passphrase: &str) -> Result<(Opened, Contents), Error> {
    let header = read_header(file)?;
    let key = derive(c, &header, passphrase)?;
    open_with(file, &header, key)
}

/// Opens with a slot key already derived. A slot whose tag verifies and
/// whose commitment block is not zero refuses the file, as two slots
/// opening do.
pub fn open_with(file: &[u8], header: &Header, key: SlotKey) -> Result<(Opened, Contents), Error> {
    let mut found: Option<(usize, Zeroizing<Vec<u8>>)> = None;
    let mut twice = false;
    let mut uncommitted = false;
    // Every slot is tried, whichever opens, so the work does not say
    // which slot a passphrase opens.
    for i in 0..SLOTS {
        match decrypt_slot(file, header, &key, i)? {
            Tried::Shut => {}
            Tried::Uncommitted => uncommitted = true,
            Tried::Opened(plain) => {
                if found.is_some() {
                    twice = true;
                } else {
                    found = Some((i, plain));
                }
            }
        }
    }
    if uncommitted {
        return Err(Error::Commitment);
    }
    if twice {
        return Err(Error::TwoSlots);
    }
    let (slot, plain) = found.ok_or(Error::Passphrase)?;
    let (contents, seals) = Contents::decode(&plain[COMMIT_LEN..]).map_err(Error::Contents)?;
    Ok((
        Opened {
            slot,
            key,
            header: *header,
            seals,
        },
        contents,
    ))
}

/// Costs what `derive` costs. Makes a vault (§8): one to four
/// passphrases, each with the contents of its own slot, at `cost` with
/// slots of `slot_len`, every used slot with seal count 1. Everything
/// random (the salt, which slots the passphrases take, the nonces and the
/// unused slots) is drawn from a ChaCha20 keystream keyed by `seed`. Two
/// passphrases with one NFC form are the same passphrase.
pub fn create(
    c: &Compute,
    slot_len: u32,
    cost: Cost,
    passphrases: &[&str],
    contents: &[Contents],
    seed: &[u8; 32],
) -> Result<Vec<u8>, Error> {
    if !SLOT_SIZES.contains(&slot_len) {
        return Err(Error::SlotSize);
    }
    check_cost(&cost)?;
    if passphrases.is_empty()
        || passphrases.len() > SLOTS
        || passphrases.len() != contents.len()
        || passphrases.iter().any(|p| p.is_empty())
    {
        return Err(Error::Passphrases);
    }
    let forms: Vec<Zeroizing<String>> = passphrases
        .iter()
        .map(|p| nfc(p))
        .collect::<Result<_, _>>()?;
    for (i, a) in forms.iter().enumerate() {
        if forms[i + 1..].contains(a) {
            return Err(Error::SamePassphrase);
        }
    }
    // Every slot's plaintext is checked to fit before any work is done.
    let mut plains: Vec<Zeroizing<Vec<u8>>> = Vec::new();
    for c in contents {
        plains.push(c.encode(slot_len as usize - COMMIT_LEN, Seals::FIRST)?);
    }

    let mut stream = keystream(seed);
    let mut draw = |out: &mut [u8]| {
        out.fill(0);
        stream.apply_keystream(out);
    };
    let mut salt = [0u8; SALT_LEN];
    draw(&mut salt);
    // Which slot each passphrase takes: a uniform permutation of 0..4.
    let mut order = [0usize, 1, 2, 3];
    for i in (1..SLOTS).rev() {
        let j = loop {
            let mut b = [0u8; 1];
            draw(&mut b);
            // Rejection keeps every position equally likely.
            let limit = 256 - 256 % (i + 1);
            if (b[0] as usize) < limit {
                break b[0] as usize % (i + 1);
            }
        };
        order.swap(i, j);
    }
    let header = Header {
        cost,
        slot_len,
        salt,
    };
    let mut file: Vec<u8> = Vec::new();
    let len = file_len(slot_len);
    file.try_reserve_exact(len)
        .map_err(|_| Error::Memory((len / (1024 * 1024)) as u32 + 1))?;
    file.extend_from_slice(&header.to_bytes());
    // Unused slots are a nonce and L + 16 bytes from the keystream: the
    // same distribution as a used slot. Used slots are overwritten below.
    file.resize(len, 0);
    draw(&mut file[HEADER_LEN..]);
    for (k, passphrase) in forms.iter().enumerate() {
        let key = derive(c, &header, passphrase)?;
        let mut nonce = [0u8; NONCE_LEN];
        draw(&mut nonce);
        encrypt_slot(&mut file, &key, order[k], &nonce, &plains[k])?;
    }
    Ok(file)
}

fn keystream(seed: &[u8; 32]) -> ChaCha20 {
    let key = hkdf_sha256(seed, CREATE_TAG);
    ChaCha20::new_from_slices(key.expose(), &[0u8; 12]).expect("a 32-byte key and a 12-byte nonce")
}

#[cfg(test)]
mod tests;
