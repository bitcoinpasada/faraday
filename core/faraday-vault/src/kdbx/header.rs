//! The outer header: the signatures, the version, the header fields and
//! the KDF parameters' VariantDictionary (`docs/VAULT.md` §1.1, §7.1).
//!
//! A field this reader does not know is refused, as is every field a
//! KDBX 3.1 file carries that 4 does not; the cost the KDF parameters
//! state is checked against §3's limits here, before anything is
//! allocated for it.

use alloc::vec::Vec;

use crate::Cost;

use super::{Error, Unsupported};

/// The first four bytes of every KDBX file.
pub(super) const SIGNATURE_1: u32 = 0x9AA2_D903;
/// The second four, which say it is KDBX rather than KDB.
pub(super) const SIGNATURE_2: u32 = 0xB54B_FB67;
/// Major 4, minor 0: the version Faraday writes.
const WRITTEN_VERSION: u32 = 0x0004_0000;

/// Outer header field ids.
const FIELD_END: u8 = 0;
const FIELD_CIPHER: u8 = 2;
const FIELD_COMPRESSION: u8 = 3;
const FIELD_MASTER_SEED: u8 = 4;
const FIELD_ENCRYPTION_IV: u8 = 7;
const FIELD_KDF_PARAMS: u8 = 11;
const FIELD_PUBLIC_CUSTOM_DATA: u8 = 12;

/// ChaCha20 (RFC 8439) as the file's cipher.
const CIPHER_CHACHA20: [u8; 16] = [
    0xD6, 0x03, 0x8A, 0x2B, 0x8B, 0x6F, 0x4C, 0xB5, 0xA5, 0x24, 0x33, 0x9A, 0x31, 0xDB, 0xB5, 0x9A,
];
/// AES-256 in CBC mode.
const CIPHER_AES256: [u8; 16] = [
    0x31, 0xC1, 0xF2, 0xE6, 0xBF, 0x71, 0x43, 0x50, 0xBE, 0x58, 0x05, 0x21, 0x6A, 0xFC, 0x5A, 0xFF,
];
/// Twofish in CBC mode, which KeePass 2 plugins write; named and refused.
const CIPHER_TWOFISH: [u8; 16] = [
    0xAD, 0x68, 0xF2, 0x9F, 0x57, 0x6F, 0x4B, 0xB9, 0xA3, 0x6A, 0xD4, 0x7A, 0xF9, 0x65, 0x34, 0x6C,
];

/// Argon2id as the key derivation function.
const KDF_ARGON2ID: [u8; 16] = [
    0x9E, 0x29, 0x8B, 0x19, 0x56, 0xDB, 0x47, 0x73, 0xB2, 0x3D, 0xFC, 0x3E, 0xC6, 0xF0, 0xA1, 0xE6,
];
/// Argon2d as the key derivation function.
const KDF_ARGON2D: [u8; 16] = [
    0xEF, 0x63, 0x6D, 0xDF, 0x8C, 0x29, 0x44, 0x4B, 0x91, 0xF7, 0xA9, 0xA4, 0x03, 0xE3, 0x0A, 0x0C,
];
/// AES-KDF, KDBX 3's, which KDBX 4 still allows; named and refused.
const KDF_AES: [u8; 16] = [
    0xC9, 0xD9, 0xF3, 0x9A, 0x62, 0x8A, 0x44, 0x60, 0xBF, 0x74, 0x0D, 0x08, 0xC1, 0x8A, 0x4F, 0xEA,
];

/// VariantDictionary value types.
const VD_U32: u8 = 0x04;
const VD_U64: u8 = 0x05;
const VD_BOOL: u8 = 0x08;
const VD_I32: u8 = 0x0C;
const VD_I64: u8 = 0x0D;
const VD_STRING: u8 = 0x18;
const VD_BYTES: u8 = 0x42;

/// The master seed's length.
pub(super) const MASTER_SEED_LEN: usize = 32;
/// ChaCha20's nonce, which is the encryption IV's length under it.
const CHACHA_IV_LEN: usize = 12;
/// AES's block, which is the encryption IV's length under it.
const AES_IV_LEN: usize = 16;
/// The shortest and longest Argon2 salt a reader takes (RFC 9106 asks
/// for at least 8 bytes).
const SALT_RANGE: core::ops::RangeInclusive<usize> = 8..=64;

/// The memory a reader accepts, in KiB: 8 MiB to 1 GiB (§3).
pub const MEMORY_KIB: core::ops::RangeInclusive<u32> = 8 * 1024..=1024 * 1024;
/// The passes a reader accepts.
pub const PASSES: core::ops::RangeInclusive<u32> = 1..=100;
/// The lanes a reader accepts.
pub const LANES: core::ops::RangeInclusive<u32> = 1..=64;

/// How the body is encrypted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cipher {
    /// ChaCha20, what Faraday writes.
    ChaCha20,
    /// AES-256 in CBC mode with PKCS #7 padding.
    Aes256,
}

/// Whether the body is compressed before it is encrypted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    /// Not compressed, what Faraday writes.
    None,
    /// gzip.
    Gzip,
}

/// The memory-hard function that stretches the passphrase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kdf {
    /// Argon2id, what Faraday writes.
    Argon2id,
    /// Argon2d, which KeePass writes by default.
    Argon2d,
}

/// What a file's header states. None of it is secret.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::disallowed_methods, reason = "a derived Clone of public data")]
pub struct Header {
    /// 0 or 1: the minor version, 4.0 or 4.1.
    pub minor: u16,
    /// The body's cipher.
    pub cipher: Cipher,
    /// The body's compression.
    pub compression: Compression,
    /// The key derivation function.
    pub kdf: Kdf,
    /// What a guess costs.
    pub cost: Cost,
    /// The header's public custom data (a VariantDictionary other programs
    /// keep outside the encryption), as it was read, to be written back.
    pub public_custom_data: Option<Vec<u8>>,
}

/// A header read: what it states and the values the keys come from.
pub(super) struct Parsed {
    pub header: Header,
    /// The header's length, signatures to the end field.
    pub len: usize,
    pub master_seed: [u8; MASTER_SEED_LEN],
    pub iv: Vec<u8>,
    pub salt: Vec<u8>,
}

/// Four bytes at `at` as a little-endian number.
fn le_u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

/// What the first twelve bytes of `bytes` say of the file.
pub(super) fn minor_of(bytes: &[u8]) -> Result<u16, Error> {
    if le_u32_at(bytes, 0) != Some(SIGNATURE_1) || le_u32_at(bytes, 4) != Some(SIGNATURE_2) {
        return Err(Error::NotKdbx);
    }
    let version = le_u32_at(bytes, 8).ok_or(Error::NotKdbx)?;
    let (major, minor) = ((version >> 16) as u16, (version & 0xFFFF) as u16);
    match (major, minor) {
        (4, 0 | 1) => Ok(minor),
        (0..=3, _) => Err(Error::Older { major }),
        _ => Err(Error::Newer { major, minor }),
    }
}

/// Reads the header at the start of `bytes`: the version, each field,
/// the KDF parameters and their cost against the limits (§3). No work.
pub(super) fn read_fields(bytes: &[u8]) -> Result<Parsed, Error> {
    let minor = minor_of(bytes)?;
    let mut at = 12usize;
    let mut cipher = None;
    let mut compression = None;
    let mut master_seed: Option<[u8; MASTER_SEED_LEN]> = None;
    let mut iv: Option<Vec<u8>> = None;
    let mut kdf = None;
    let mut public_custom_data = None;
    loop {
        let id = *bytes.get(at).ok_or(Error::Damaged)?;
        let len = le_u32_at(bytes, at + 1).ok_or(Error::Damaged)? as usize;
        at += 5;
        let end = at.checked_add(len).ok_or(Error::Damaged)?;
        let value = bytes.get(at..end).ok_or(Error::Damaged)?;
        at = end;
        let seen = match id {
            FIELD_END => break,
            FIELD_CIPHER => cipher.replace(cipher_of(value)?).is_some(),
            FIELD_COMPRESSION => compression.replace(compression_of(value)?).is_some(),
            FIELD_MASTER_SEED => master_seed
                .replace(value.try_into().map_err(|_| Error::Damaged)?)
                .is_some(),
            FIELD_ENCRYPTION_IV => {
                #[expect(
                    clippy::disallowed_methods,
                    reason = "the cipher IV, which the file header holds in the clear"
                )]
                let value = value.to_vec();
                iv.replace(value).is_some()
            }
            FIELD_KDF_PARAMS => kdf.replace(kdf_of(value)?).is_some(),
            FIELD_PUBLIC_CUSTOM_DATA => {
                #[expect(
                    clippy::disallowed_methods,
                    reason = "the public custom data, which the file header holds in the clear"
                )]
                let value = value.to_vec();
                public_custom_data.replace(value).is_some()
            }
            other => return Err(Error::UnknownField(other)),
        };
        if seen {
            return Err(Error::Damaged);
        }
    }
    let cipher = cipher.ok_or(Error::Damaged)?;
    let iv = iv.ok_or(Error::Damaged)?;
    let want = match cipher {
        Cipher::ChaCha20 => CHACHA_IV_LEN,
        Cipher::Aes256 => AES_IV_LEN,
    };
    if iv.len() != want {
        return Err(Error::Damaged);
    }
    let (kdf, cost, salt) = kdf.ok_or(Error::Damaged)?;
    Ok(Parsed {
        header: Header {
            minor,
            cipher,
            compression: compression.ok_or(Error::Damaged)?,
            kdf,
            cost,
            public_custom_data,
        },
        len: at,
        master_seed: master_seed.ok_or(Error::Damaged)?,
        iv,
        salt,
    })
}

fn cipher_of(value: &[u8]) -> Result<Cipher, Error> {
    match <[u8; 16]>::try_from(value).map_err(|_| Error::Damaged)? {
        CIPHER_CHACHA20 => Ok(Cipher::ChaCha20),
        CIPHER_AES256 => Ok(Cipher::Aes256),
        CIPHER_TWOFISH => Err(Error::Unsupported(Unsupported::Twofish)),
        _ => Err(Error::Unsupported(Unsupported::OtherCipher)),
    }
}

fn compression_of(value: &[u8]) -> Result<Compression, Error> {
    match u32::from_le_bytes(value.try_into().map_err(|_| Error::Damaged)?) {
        0 => Ok(Compression::None),
        1 => Ok(Compression::Gzip),
        _ => Err(Error::Unsupported(Unsupported::Compression)),
    }
}

/// The KDF, its cost and its salt from the parameters' VariantDictionary.
/// The cost is checked here, before any caller allocates for it.
#[expect(
    clippy::disallowed_methods,
    reason = "the key derivation salt, which the file header holds in the clear"
)]
fn kdf_of(bytes: &[u8]) -> Result<(Kdf, Cost, Vec<u8>), Error> {
    // The dictionary's version: major 1, whatever the minor.
    if bytes.len() < 3 || bytes[1] != 1 {
        return Err(Error::Damaged);
    }
    let mut at = 2usize;
    let mut uuid: Option<[u8; 16]> = None;
    let mut salt: Option<Vec<u8>> = None;
    let (mut passes, mut memory, mut lanes, mut version) = (None, None, None, None);
    let mut extra = false;
    loop {
        let kind = *bytes.get(at).ok_or(Error::Damaged)?;
        at += 1;
        if kind == 0 {
            break;
        }
        let name_len = le_u32_at(bytes, at).ok_or(Error::Damaged)? as usize;
        at += 4;
        let name = bytes
            .get(at..at.checked_add(name_len).ok_or(Error::Damaged)?)
            .ok_or(Error::Damaged)?;
        at += name_len;
        let value_len = le_u32_at(bytes, at).ok_or(Error::Damaged)? as usize;
        at += 4;
        let value = bytes
            .get(at..at.checked_add(value_len).ok_or(Error::Damaged)?)
            .ok_or(Error::Damaged)?;
        at += value_len;
        let fixed = match kind {
            VD_U32 | VD_I32 => 4,
            VD_U64 | VD_I64 => 8,
            VD_BOOL => 1,
            VD_STRING | VD_BYTES => value.len(),
            _ => return Err(Error::Damaged),
        };
        if value.len() != fixed {
            return Err(Error::Damaged);
        }
        match (kind, name) {
            (VD_BYTES, b"$UUID") => uuid = Some(value.try_into().map_err(|_| Error::Damaged)?),
            (VD_BYTES, b"S") => salt = Some(value.to_vec()),
            (VD_U64, b"I") => passes = Some(le_u64(value)),
            (VD_U64, b"M") => memory = Some(le_u64(value)),
            (VD_U32, b"P") => lanes = Some(le_u32_at(value, 0).ok_or(Error::Damaged)?),
            (VD_U32, b"V") => version = Some(le_u32_at(value, 0).ok_or(Error::Damaged)?),
            // A secret key or associated data is not read (§3), unless
            // empty, which is what none is.
            (VD_BYTES, b"K" | b"A") => extra |= !value.is_empty(),
            (_, b"$UUID" | b"S" | b"I" | b"M" | b"P" | b"V" | b"K" | b"A") => {
                return Err(Error::Damaged);
            }
            _ => {}
        }
    }
    // The dictionary ends the field.
    if at != bytes.len() {
        return Err(Error::Damaged);
    }
    let kdf = match uuid.ok_or(Error::Damaged)? {
        KDF_ARGON2ID => Kdf::Argon2id,
        KDF_ARGON2D => Kdf::Argon2d,
        KDF_AES => return Err(Error::Unsupported(Unsupported::AesKdf)),
        _ => return Err(Error::Unsupported(Unsupported::OtherKdf)),
    };
    let salt = salt.ok_or(Error::Damaged)?;
    if !SALT_RANGE.contains(&salt.len()) {
        return Err(Error::Damaged);
    }
    // KeePass and KeePassXC refuse a dictionary without its version.
    let version = version.ok_or(Error::Damaged)?;
    if extra || version != 0x13 {
        return Err(Error::Cost);
    }
    // Memory in bytes, a whole number of KiB.
    let memory = memory.ok_or(Error::Damaged)?;
    let memory_kib = u32::try_from(memory / 1024).map_err(|_| Error::Cost)?;
    let passes = u32::try_from(passes.ok_or(Error::Damaged)?).map_err(|_| Error::Cost)?;
    let cost = Cost {
        memory_kib,
        passes,
        lanes: lanes.ok_or(Error::Damaged)?,
    };
    cost_in_limits(&cost)?;
    if memory % 1024 != 0 {
        return Err(Error::Cost);
    }
    Ok((kdf, cost, salt))
}

fn le_u64(value: &[u8]) -> u64 {
    let mut b = [0u8; 8];
    b.copy_from_slice(value);
    u64::from_le_bytes(b)
}

/// Whether `cost` is within §3's limits.
pub(super) fn cost_in_limits(cost: &Cost) -> Result<(), Error> {
    if MEMORY_KIB.contains(&cost.memory_kib)
        && PASSES.contains(&cost.passes)
        && LANES.contains(&cost.lanes)
    {
        Ok(())
    } else {
        Err(Error::Cost)
    }
}

/// The header Faraday writes: 4.0, ChaCha20, no compression, Argon2id.
pub(super) fn write(
    master_seed: &[u8],
    iv: &[u8],
    salt: &[u8],
    cost: Cost,
    public_custom_data: Option<&[u8]>,
) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(256);
    out.extend_from_slice(&SIGNATURE_1.to_le_bytes());
    out.extend_from_slice(&SIGNATURE_2.to_le_bytes());
    out.extend_from_slice(&WRITTEN_VERSION.to_le_bytes());
    put_field(FIELD_CIPHER, &CIPHER_CHACHA20, &mut out);
    put_field(FIELD_COMPRESSION, &0u32.to_le_bytes(), &mut out);
    put_field(FIELD_MASTER_SEED, master_seed, &mut out);
    put_field(FIELD_ENCRYPTION_IV, iv, &mut out);

    let mut kdf: Vec<u8> = Vec::with_capacity(160);
    kdf.extend_from_slice(&0x0100u16.to_le_bytes());
    put_vd(VD_BYTES, b"$UUID", &KDF_ARGON2ID, &mut kdf);
    put_vd(VD_BYTES, b"S", salt, &mut kdf);
    put_vd(
        VD_U64,
        b"I",
        &u64::from(cost.passes).to_le_bytes(),
        &mut kdf,
    );
    put_vd(
        VD_U64,
        b"M",
        &(u64::from(cost.memory_kib) * 1024).to_le_bytes(),
        &mut kdf,
    );
    put_vd(VD_U32, b"P", &cost.lanes.to_le_bytes(), &mut kdf);
    put_vd(VD_U32, b"V", &0x13u32.to_le_bytes(), &mut kdf);
    kdf.push(0);
    put_field(FIELD_KDF_PARAMS, &kdf, &mut out);
    if let Some(data) = public_custom_data {
        put_field(FIELD_PUBLIC_CUSTOM_DATA, data, &mut out);
    }
    put_field(FIELD_END, &[0x0D, 0x0A, 0x0D, 0x0A], &mut out);
    out
}

/// One header field: the id, the length, the value.
pub(super) fn put_field(id: u8, value: &[u8], out: &mut Vec<u8>) {
    out.push(id);
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value);
}

/// One VariantDictionary item.
fn put_vd(kind: u8, name: &[u8], value: &[u8], out: &mut Vec<u8>) {
    out.push(kind);
    out.extend_from_slice(&(name.len() as u32).to_le_bytes());
    out.extend_from_slice(name);
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value);
}
