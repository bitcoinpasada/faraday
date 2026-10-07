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
//! - [`Opened::seal`] re-encrypts the open slot under a fresh nonce and
//!   copies the other three byte for byte (§5).
//! - [`records`] is the slot's plaintext: typed records of fields (§7).
//!
//! The cryptography is `osk-backup`'s: Argon2id version 1.3 and
//! XChaCha20-Poly1305, with the working memory asked for rather than
//! taken. Nothing here draws randomness: the caller passes the seed a new
//! vault is made from and the nonce a seal uses.
//!
//! `no_std` + `alloc`.

#![no_std]

extern crate alloc;

pub mod entries;
pub mod records;

use alloc::vec::Vec;

use argon2::{Algorithm, Argon2, Block, Params, Version};
use chacha20::ChaCha20;
use chacha20::cipher::{KeyIvInit, StreamCipher};
use chacha20poly1305::aead::inout::InOutBuf;
use chacha20poly1305::{AeadInOut, KeyInit, Tag, XChaCha20Poly1305, XNonce};
use hmac::{Hmac, KeyInit as HmacInit, Mac};
use osk_crypto::{Secret, Zeroize};
use sha2::Sha256;
use zeroize::Zeroizing;

pub use osk_backup::Cost;
pub use records::{Contents, Field, Record};

/// The first four bytes of every vault.
pub const MAGIC: [u8; 4] = *b"OFVT";
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
    /// The file does not begin with `OFVT`.
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
    /// The opened slot's contents are not what §7 allows.
    Contents(records::Fault),
    /// The contents do not fit in a slot.
    Full,
    /// No passphrase, more than four, or an empty one.
    Passphrases,
    /// Two passphrases are the same.
    SamePassphrase,
    /// The file being sealed is not the file this slot was opened from.
    OtherFile,
}

impl Error {
    /// The reason in a few words, for the screen.
    pub fn reason(self) -> alloc::string::String {
        use alloc::string::ToString;
        match self {
            Error::Magic => "Not a Faraday vault".to_string(),
            Error::Version => "A vault version this build does not read".to_string(),
            Error::SlotSize => "A slot size this build does not read".to_string(),
            Error::Length => "The file is cut short or has bytes added".to_string(),
            Error::Cost => "Its unlock cost is outside what Faraday reads".to_string(),
            Error::Memory(mib) => {
                alloc::format!("Needs {mib} MiB, which this computer cannot spare")
            }
            Error::Passphrase => "That passphrase opens nothing in this vault".to_string(),
            Error::TwoSlots => {
                "Two slots opened under one passphrase; the file is not sound".to_string()
            }
            Error::Contents(f) => alloc::format!(
                "The slot opened but its contents are faulty: {}",
                f.reason()
            ),
            Error::Full => "The contents do not fit in this vault's slot size".to_string(),
            Error::Passphrases => "One to four passphrases, none of them empty".to_string(),
            Error::SamePassphrase => "Two passphrases are the same. Each must differ".to_string(),
            Error::OtherFile => "That slot was opened from another file".to_string(),
        }
    }
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
    /// The memory a machine needs free to open this vault, in MiB: the
    /// Argon2id memory, the file, one slot's plaintext and the running
    /// system (§3.1).
    pub fn memory_to_open_mib(&self) -> u32 {
        self.cost.memory_kib / 1024 + 100
    }

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

/// Whether the bytes begin as a vault does. Says nothing of whether the
/// rest is sound.
pub fn is_vault(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && bytes[0..4] == MAGIC
}

/// Checks a vault's header and length (§4 step 1).
pub fn read_header(bytes: &[u8]) -> Result<Header, Error> {
    if !is_vault(bytes) {
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

/// Derives the slot key a passphrase gives under `header` (§3): Argon2id
/// at the header's cost, then HKDF-SHA256 with an empty salt and the
/// slot info.
pub fn derive(header: &Header, passphrase: &[u8]) -> Result<SlotKey, Error> {
    let k = stretch(&header.cost, passphrase, &header.salt)?;
    Ok(SlotKey(hkdf_sha256(k.expose(), SLOT_INFO)))
}

/// Argon2id(passphrase, salt) at `cost`, 32 bytes, with the working
/// memory asked for so that a cost this machine cannot meet is an error.
fn stretch(cost: &Cost, passphrase: &[u8], salt: &[u8]) -> Result<Secret<[u8; 32]>, Error> {
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

/// Slot `i`'s associated data: the header, then the slot number.
fn associated(header: &[u8], i: usize) -> [u8; HEADER_LEN + 1] {
    let mut ad = [0u8; HEADER_LEN + 1];
    ad[..HEADER_LEN].copy_from_slice(&header[..HEADER_LEN]);
    ad[HEADER_LEN] = i as u8;
    ad
}

/// Encrypts `plain` (exactly the slot length) into slot `i` of `file`.
fn encrypt_slot(
    file: &mut [u8],
    key: &SlotKey,
    i: usize,
    nonce: &[u8; NONCE_LEN],
    plain: &mut [u8],
) {
    let slot_len = plain.len() as u32;
    let ad = associated(file, i);
    let cipher = XChaCha20Poly1305::new_from_slice(key.0.expose()).expect("32-byte key");
    let tag = cipher
        .encrypt_inout_detached(&XNonce::from(*nonce), &ad, InOutBuf::from(&mut plain[..]))
        .expect("a slot is far below the cipher's limit");
    let at = slot_at(slot_len, i);
    file[at..at + NONCE_LEN].copy_from_slice(nonce);
    file[at + NONCE_LEN..at + NONCE_LEN + plain.len()].copy_from_slice(plain);
    file[at + NONCE_LEN + plain.len()..at + NONCE_LEN + plain.len() + TAG_LEN]
        .copy_from_slice(&tag);
    plain.zeroize();
}

/// Decrypts slot `i` of `file` in a buffer of its own, or `None` when
/// the tag fails, in which case the buffer is wiped.
fn decrypt_slot(
    file: &[u8],
    header: &Header,
    key: &SlotKey,
    i: usize,
) -> Result<Option<Zeroizing<Vec<u8>>>, Error> {
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
    if ok {
        Ok(Some(plain))
    } else {
        plain.zeroize();
        Ok(None)
    }
}

/// One open slot: which slot, under which key, of which file. The key
/// stays for sealing and is wiped with this.
pub struct Opened {
    /// The slot that opened, 0 to 3. Never shown: §9 has nothing on
    /// screen count or name slots.
    slot: usize,
    key: SlotKey,
    header: Header,
}

impl Opened {
    /// The header of the file this slot was opened from.
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// The vault with this slot holding `contents` under a fresh nonce,
    /// and every other slot copied from `file` byte for byte (§5).
    pub fn seal(
        &self,
        file: &[u8],
        contents: &Contents,
        nonce: &[u8; NONCE_LEN],
    ) -> Result<Vec<u8>, Error> {
        let header = read_header(file)?;
        if header != self.header {
            return Err(Error::OtherFile);
        }
        let mut plain = contents.encode(header.slot_len)?;
        let mut out: Vec<u8> = Vec::new();
        out.try_reserve_exact(file.len())
            .map_err(|_| Error::Memory((file.len() / (1024 * 1024)) as u32 + 1))?;
        out.extend_from_slice(file);
        encrypt_slot(&mut out, &self.key, self.slot, nonce, &mut plain);
        Ok(out)
    }
}

/// Opens the slot `passphrase` opens (§4): derives the key once, tries
/// all four slots, and parses the one that opens.
pub fn open(file: &[u8], passphrase: &[u8]) -> Result<(Opened, Contents), Error> {
    let header = read_header(file)?;
    let key = derive(&header, passphrase)?;
    open_with(file, &header, key)
}

/// Opens with a slot key already derived.
pub fn open_with(file: &[u8], header: &Header, key: SlotKey) -> Result<(Opened, Contents), Error> {
    let mut found: Option<(usize, Zeroizing<Vec<u8>>)> = None;
    let mut twice = false;
    // Every slot is tried, whichever opens, so the work does not say
    // which slot a passphrase opens.
    for i in 0..SLOTS {
        if let Some(plain) = decrypt_slot(file, header, &key, i)? {
            if found.is_some() {
                twice = true;
            } else {
                found = Some((i, plain));
            }
        }
    }
    if twice {
        return Err(Error::TwoSlots);
    }
    let (slot, plain) = found.ok_or(Error::Passphrase)?;
    let contents = Contents::decode(&plain).map_err(Error::Contents)?;
    Ok((
        Opened {
            slot,
            key,
            header: *header,
        },
        contents,
    ))
}

/// Makes a vault (§8): one to four passphrases, each with the contents of
/// its own slot, at `cost` with slots of `slot_len`. Everything random
/// (the salt, which slots the passphrases take, the nonces and the
/// unused slots) is drawn from a ChaCha20 keystream keyed by `seed`.
pub fn create(
    slot_len: u32,
    cost: Cost,
    passphrases: &[&[u8]],
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
    for (i, a) in passphrases.iter().enumerate() {
        if passphrases[i + 1..].contains(a) {
            return Err(Error::SamePassphrase);
        }
    }
    // Every slot's plaintext is checked to fit before any work is done.
    let mut plains: Vec<Zeroizing<Vec<u8>>> = Vec::new();
    for c in contents {
        plains.push(c.encode(slot_len)?);
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
    for (k, passphrase) in passphrases.iter().enumerate() {
        let key = derive(&header, passphrase)?;
        let mut nonce = [0u8; NONCE_LEN];
        draw(&mut nonce);
        encrypt_slot(&mut file, &key, order[k], &nonce, &mut plains[k]);
    }
    Ok(file)
}

fn keystream(seed: &[u8; 32]) -> ChaCha20 {
    let key = hkdf_sha256(seed, CREATE_TAG);
    ChaCha20::new_from_slices(key.expose(), &[0u8; 12]).expect("a 32-byte key and a 12-byte nonce")
}

#[cfg(test)]
mod tests;
