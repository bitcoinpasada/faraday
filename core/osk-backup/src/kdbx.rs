//! The KDBX 4 database this device writes so that a KeePass app opens
//! what it exported (`docs/PLANNING.md` §16.112 rule 3, pass E2).
//!
//! This is a writer. Nothing here reads a KDBX file someone else made:
//! the only reader is [`verify`], which checks the authentication on
//! bytes this module wrote and parses no XML.
//!
//! The format is KeePass's own, published at
//! <https://keepass.info/help/kb/kdbx.html> and
//! <https://keepass.info/help/kb/kdbx_4.html>; both pages are in
//! `tools/reference/kdbx4/` with the date they were fetched and their
//! SHA-256. `docs/BACKUP.md` writes down what this module puts in one.
//! The version written is 4.0, because 4.1 adds only XML elements this
//! writer has no use for and 4.0 is what every KDBX 4 reader takes.
//!
//! ```text
//!  0   signature 1, u32 = 0x9AA2D903
//!  4   signature 2, u32 = 0xB54BFB67
//!  8   version, u32 = 0x00040000
//! 12   header fields: cipher ChaCha20, compression none, master seed,
//!      encryption IV, the KDF parameters as a VariantDictionary, end
//!  ..   SHA-256 of the header, 32 bytes
//!  ..   HMAC-SHA-256 of the header, 32 bytes
//!  ..   the HMAC block stream over the encrypted body
//! ```
//!
//! The keys follow the specification:
//!
//! ```text
//! R = SHA-256(SHA-256(passphrase))         -- the composite key
//! T = Argon2id(R, salt, cost)              -- 32 bytes
//! encryption key = SHA-256(master_seed ‖ T)
//! base           = SHA-512(master_seed ‖ T ‖ 0x01)
//! header HMAC key = SHA-512(0xFFFFFFFFFFFFFFFF ‖ base)
//! block i's key   = SHA-512(i ‖ base)
//! ```
//!
//! The composite key is the passphrase alone: this writer takes no key
//! file, and a KeePass app opens the file with the passphrase the
//! person typed here and nothing else. The Argon2id cost is the
//! device's [`Cost`], the same one `osk-backup` is written at, so the
//! two forms of an export cost a guess the same.
//!
//! The body is the inner header — inner random stream ChaCha20 with its
//! key, no binaries — and one XML document: a root group named
//! `OpenSigner` with one entry per item, each carrying Title, Notes and
//! a Password field marked `Protected` and encrypted with the inner
//! stream. There is no compression: gzip over a handful of secrets buys
//! nothing and costs a second format in the path.
//!
//! The inner stream is KeePass's process-memory protection and no part
//! of the file's security, which is the outer ChaCha20 and the HMACs.
//! What a KDBX file's security is, is the passphrase over it: a key
//! exported this way is as safe as that passphrase, which is what Learn
//! says and why the export asks for a strong one.
//!
//! This file is listed in `tools/lint-secrets.sh` and may hold no heap
//! text: every byte of the document is built in a buffer that zeroizes,
//! because the words are in it in the clear until the cipher runs.

use alloc::vec::Vec;

use argon2::Params;
use chacha20::ChaCha20;
use chacha20::cipher::{KeyIvInit, StreamCipher};
use hmac::{Hmac, KeyInit, Mac};
use osk_crypto::{Secret, Zeroize};
use sha2::{Digest, Sha256, Sha512};
use zeroize::Zeroizing;

use crate::{Cost, argon2id};

/// The first four bytes of every KDBX file.
const SIGNATURE_1: u32 = 0x9AA2_D903;
/// The second four, which say it is KDBX rather than KDB.
const SIGNATURE_2: u32 = 0xB54B_FB67;
/// Major 4, minor 0.
const FORMAT_VERSION: u32 = 0x0004_0000;

/// ChaCha20 as the file's cipher (RFC 8439).
const CIPHER_CHACHA20: [u8; 16] = [
    0xD6, 0x03, 0x8A, 0x2B, 0x8B, 0x6F, 0x4C, 0xB5, 0xA5, 0x24, 0x33, 0x9A, 0x31, 0xDB, 0xB5, 0x9A,
];
/// Argon2id as the key derivation function.
const KDF_ARGON2ID: [u8; 16] = [
    0x9E, 0x29, 0x8B, 0x19, 0x56, 0xDB, 0x47, 0x73, 0xB2, 0x3D, 0xFC, 0x3E, 0xC6, 0xF0, 0xA1, 0xE6,
];

/// Outer header field ids.
const FIELD_END: u8 = 0;
const FIELD_CIPHER: u8 = 2;
const FIELD_COMPRESSION: u8 = 3;
const FIELD_MASTER_SEED: u8 = 4;
const FIELD_ENCRYPTION_IV: u8 = 7;
const FIELD_KDF_PARAMS: u8 = 11;

/// Inner header field ids.
const INNER_END: u8 = 0;
const INNER_STREAM_ID: u8 = 1;
const INNER_STREAM_KEY: u8 = 2;

/// ChaCha20 as the inner random stream.
const INNER_STREAM_CHACHA20: u32 = 3;

/// VariantDictionary value types.
const VD_U32: u8 = 0x04;
const VD_U64: u8 = 0x05;
const VD_BYTES: u8 = 0x42;

/// The master seed, which is part of every key the file uses.
const MASTER_SEED_LEN: usize = 32;
/// ChaCha20's nonce, which is what the encryption IV holds.
const IV_LEN: usize = 12;
/// The Argon2id salt, at the size KeePass recommends.
const KDF_SALT_LEN: usize = 32;
/// The inner random stream's key, at the size ChaCha20 wants.
const INNER_KEY_LEN: usize = 64;

/// The random bytes one database needs: the master seed, the encryption
/// IV, the Argon2id salt and the inner stream's key, in that order.
pub const SEED_LEN: usize = MASTER_SEED_LEN + IV_LEN + KDF_SALT_LEN + INNER_KEY_LEN;

/// SHA-256 and HMAC-SHA-256, which are 32 bytes each.
const HASH_LEN: usize = 32;

/// The block index the header's own HMAC is computed under.
const HEADER_BLOCK: u64 = u64::MAX;

/// The most entries one database holds. An export is one item or a
/// handful, never a list.
pub const MAX_ITEMS: usize = 8;

/// The most any one field of an item may be, which is the payload
/// `osk-backup` takes.
pub const MAX_FIELD: usize = 16 * 1024;

/// The most Argon2id memory this build will allocate for a KDBX file,
/// in KiB — the same bound [`crate::oskb`] puts on a file's stated
/// cost.
const MAX_MEMORY_KIB: u32 = 4 * 1024 * 1024;

/// One entry of the database: what a KeePass app shows as a row.
pub struct Item<'a> {
    /// The entry's Title: a key's fingerprint, a note's first line, a
    /// wallet's name.
    pub title: &'a [u8],
    /// The entry's Notes: the word count and wordlist, a note's text, a
    /// wallet's descriptor and the sheet's note.
    pub notes: &'a [u8],
    /// The entry's Password, which is the secret field: the words, a
    /// seed as hex, or nothing where the item has no secret.
    pub secret: &'a [u8],
}

/// The KDBX 4 database holding `items`, under `passphrase`, at `cost`,
/// with the master seed, the IV, the Argon2id salt and the inner key
/// taken from `seed`.
///
/// `seed` is the caller's randomness and must be fresh for every file:
/// a salt reused across two passphrases costs an attacker one Argon2id
/// run for both.
pub fn write(
    items: &[Item<'_>],
    passphrase: &[u8],
    cost: Cost,
    seed: &[u8; SEED_LEN],
) -> Option<Vec<u8>> {
    if items.is_empty() || items.len() > MAX_ITEMS {
        return None;
    }
    for item in items {
        if item.title.len() > MAX_FIELD
            || item.notes.len() > MAX_FIELD
            || item.secret.len() > MAX_FIELD
        {
            return None;
        }
    }
    let master_seed = &seed[..MASTER_SEED_LEN];
    let iv = &seed[MASTER_SEED_LEN..MASTER_SEED_LEN + IV_LEN];
    let kdf_salt = &seed[MASTER_SEED_LEN + IV_LEN..MASTER_SEED_LEN + IV_LEN + KDF_SALT_LEN];
    let inner_key = &seed[MASTER_SEED_LEN + IV_LEN + KDF_SALT_LEN..];

    let header = header(master_seed, iv, kdf_salt, cost)?;
    let keys = keys(master_seed, kdf_salt, passphrase, cost)?;

    let mut body: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::new());
    inner_header(inner_key, &mut body);
    document(items, inner_key, master_seed, &mut body);

    // The body is encrypted in place, so what is left in the buffer is
    // ciphertext and the plaintext is gone before the buffer is.
    let mut stream = chacha20(&keys.encryption, iv);
    stream.apply_keystream(&mut body);

    let mut out: Vec<u8> = Vec::new();
    out.try_reserve_exact(header.len() + 2 * HASH_LEN + body.len() + 2 * (HASH_LEN + 4))
        .ok()?;
    out.extend_from_slice(&header);
    out.extend_from_slice(Sha256::digest(&header).as_slice());
    out.extend_from_slice(&hmac(&block_key(&keys.hmac_base, HEADER_BLOCK), &[&header]));
    put_block(0, &body, &keys.hmac_base, &mut out);
    put_block(1, &[], &keys.hmac_base, &mut out);
    Some(out)
}

/// Whether `bytes` are a KDBX 4 file whose header hash, header HMAC and
/// every block HMAC verify under `passphrase`.
///
/// This is the check on what [`write`] wrote, not an importer: it reads
/// the header, runs the key derivation the header states and verifies
/// the authentication. It parses no XML and returns nothing from the
/// file.
pub fn verify(bytes: &[u8], passphrase: &[u8]) -> bool {
    let Some((header_len, master_seed, kdf_salt, cost)) = read_header(bytes) else {
        return false;
    };
    let header = &bytes[..header_len];
    if bytes.len() < header_len + 2 * HASH_LEN {
        return false;
    }
    if Sha256::digest(header).as_slice() != &bytes[header_len..header_len + HASH_LEN] {
        return false;
    }
    let Some(keys) = keys(&master_seed, &kdf_salt, passphrase, cost) else {
        return false;
    };
    let stated = &bytes[header_len + HASH_LEN..header_len + 2 * HASH_LEN];
    if hmac(&block_key(&keys.hmac_base, HEADER_BLOCK), &[header]).as_slice() != stated {
        return false;
    }
    blocks_verify(&bytes[header_len + 2 * HASH_LEN..], &keys.hmac_base)
}

/// The Argon2id cost `bytes` state in their header, or `None` where
/// they are not a KDBX 4 file this module would have written. What a
/// KeePass app opening the file will pay, read without the passphrase
/// because the header states it in the clear.
pub fn cost_of(bytes: &[u8]) -> Option<Cost> {
    read_header(bytes).map(|(_, _, _, cost)| cost)
}

// ---------------------------------------------------------------------------
// The keys.
// ---------------------------------------------------------------------------

/// The two keys a file needs: the one the body is encrypted under and
/// the base every HMAC key is derived from.
struct Keys {
    /// ChaCha20's key over the body.
    encryption: Secret<[u8; 32]>,
    /// `SHA-512(master_seed ‖ T ‖ 0x01)`.
    hmac_base: Secret<[u8; 64]>,
}

/// The keys of a file with this master seed, salt, passphrase and cost.
fn keys(master_seed: &[u8], kdf_salt: &[u8], passphrase: &[u8], cost: Cost) -> Option<Keys> {
    // R, the composite key: the SHA-256 of the concatenated components,
    // and the one component is the SHA-256 of the passphrase.
    let mut composite = Secret::new(<[u8; 32]>::from(Sha256::digest(Sha256::digest(passphrase))));
    let transformed = stretch(&composite, kdf_salt, cost);
    composite.zeroize();
    let mut transformed = transformed?;

    let mut encryption = Sha256::new();
    encryption.update(master_seed);
    encryption.update(transformed.expose());
    let encryption = Secret::new(<[u8; 32]>::from(encryption.finalize()));

    let mut base = Sha512::new();
    base.update(master_seed);
    base.update(transformed.expose());
    base.update([1u8]);
    let hmac_base = Secret::new(<[u8; 64]>::from(base.finalize()));
    transformed.zeroize();
    Some(Keys {
        encryption,
        hmac_base,
    })
}

/// `Argon2id(composite, salt)`, 32 bytes, at `cost`: [`argon2id`],
/// refused above [`MAX_MEMORY_KIB`].
fn stretch(composite: &Secret<[u8; 32]>, salt: &[u8], cost: Cost) -> Option<Secret<[u8; 32]>> {
    if cost.memory_kib > MAX_MEMORY_KIB {
        return None;
    }
    argon2id(&cost, composite.expose(), salt).ok()
}

/// `SHA-512(i ‖ base)`: the key block `i`'s HMAC is computed under, and
/// the header's when `i` is [`HEADER_BLOCK`].
fn block_key(base: &Secret<[u8; 64]>, i: u64) -> Secret<[u8; 64]> {
    let mut h = Sha512::new();
    h.update(i.to_le_bytes());
    h.update(base.expose());
    Secret::new(<[u8; 64]>::from(h.finalize()))
}

/// `HMAC-SHA-256(key, parts concatenated)`.
fn hmac(key: &Secret<[u8; 64]>, parts: &[&[u8]]) -> [u8; 32] {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(key.expose()).expect("HMAC accepts any key length");
    for part in parts {
        mac.update(part);
    }
    mac.finalize().into_bytes().into()
}

/// ChaCha20 under `key` and a 12-byte nonce.
fn chacha20(key: &Secret<[u8; 32]>, nonce: &[u8]) -> ChaCha20 {
    ChaCha20::new_from_slices(key.expose(), nonce).expect("a 32-byte key and a 12-byte nonce")
}

// ---------------------------------------------------------------------------
// The outer header.
// ---------------------------------------------------------------------------

/// The outer header, from the signatures to the end-of-header field.
fn header(master_seed: &[u8], iv: &[u8], kdf_salt: &[u8], cost: Cost) -> Option<Vec<u8>> {
    if cost.memory_kib > MAX_MEMORY_KIB {
        return None;
    }
    Params::new(cost.memory_kib, cost.passes, cost.lanes, Some(32)).ok()?;
    let mut out: Vec<u8> = Vec::new();
    out.try_reserve_exact(256).ok()?;
    out.extend_from_slice(&SIGNATURE_1.to_le_bytes());
    out.extend_from_slice(&SIGNATURE_2.to_le_bytes());
    out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    put_field(FIELD_CIPHER, &CIPHER_CHACHA20, &mut out);
    put_field(FIELD_COMPRESSION, &0u32.to_le_bytes(), &mut out);
    put_field(FIELD_MASTER_SEED, master_seed, &mut out);
    put_field(FIELD_ENCRYPTION_IV, iv, &mut out);

    let mut kdf: Vec<u8> = Vec::new();
    kdf.extend_from_slice(&0x0100u16.to_le_bytes());
    put_vd_bytes(b"$UUID", &KDF_ARGON2ID, &mut kdf);
    put_vd_bytes(b"S", kdf_salt, &mut kdf);
    put_vd_u64(b"I", u64::from(cost.passes), &mut kdf);
    put_vd_u64(b"M", u64::from(cost.memory_kib) * 1024, &mut kdf);
    put_vd_u32(b"P", cost.lanes, &mut kdf);
    put_vd_u32(b"V", 0x13, &mut kdf);
    kdf.push(0);
    put_field(FIELD_KDF_PARAMS, &kdf, &mut out);

    put_field(FIELD_END, &[0x0D, 0x0A, 0x0D, 0x0A], &mut out);
    Some(out)
}

/// One outer or inner header field: the id, the length, the value.
fn put_field(id: u8, value: &[u8], out: &mut Vec<u8>) {
    out.push(id);
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value);
}

/// One VariantDictionary item of a given type.
fn put_vd(kind: u8, name: &[u8], value: &[u8], out: &mut Vec<u8>) {
    out.push(kind);
    out.extend_from_slice(&(name.len() as u32).to_le_bytes());
    out.extend_from_slice(name);
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value);
}

fn put_vd_bytes(name: &[u8], value: &[u8], out: &mut Vec<u8>) {
    put_vd(VD_BYTES, name, value, out);
}

fn put_vd_u32(name: &[u8], value: u32, out: &mut Vec<u8>) {
    put_vd(VD_U32, name, &value.to_le_bytes(), out);
}

fn put_vd_u64(name: &[u8], value: u64, out: &mut Vec<u8>) {
    put_vd(VD_U64, name, &value.to_le_bytes(), out);
}

/// The header's length, its master seed, its Argon2id salt and its
/// cost, or `None` where the bytes are not a KDBX 4 header this module
/// would have written.
#[allow(clippy::type_complexity)]
fn read_header(bytes: &[u8]) -> Option<(usize, [u8; MASTER_SEED_LEN], [u8; KDF_SALT_LEN], Cost)> {
    if bytes.len() < 12
        || u32::from_le_bytes(bytes[0..4].try_into().ok()?) != SIGNATURE_1
        || u32::from_le_bytes(bytes[4..8].try_into().ok()?) != SIGNATURE_2
        || u32::from_le_bytes(bytes[8..12].try_into().ok()?) != FORMAT_VERSION
    {
        return None;
    }
    let mut at = 12usize;
    let mut master_seed: Option<[u8; MASTER_SEED_LEN]> = None;
    let mut kdf: Option<([u8; KDF_SALT_LEN], Cost)> = None;
    loop {
        if at + 5 > bytes.len() {
            return None;
        }
        let id = bytes[at];
        let len = u32::from_le_bytes(bytes[at + 1..at + 5].try_into().ok()?) as usize;
        at += 5;
        if at + len > bytes.len() {
            return None;
        }
        let value = &bytes[at..at + len];
        at += len;
        match id {
            FIELD_CIPHER if value != CIPHER_CHACHA20 => return None,
            FIELD_COMPRESSION if value != 0u32.to_le_bytes() => return None,
            FIELD_MASTER_SEED => master_seed = Some(value.try_into().ok()?),
            FIELD_KDF_PARAMS => kdf = Some(read_kdf(value)?),
            FIELD_END => break,
            _ => {}
        }
    }
    let (salt, cost) = kdf?;
    Some((at, master_seed?, salt, cost))
}

/// The Argon2id salt and cost a KDF VariantDictionary states.
fn read_kdf(bytes: &[u8]) -> Option<([u8; KDF_SALT_LEN], Cost)> {
    if bytes.len() < 3 || bytes[1] != 1 {
        return None;
    }
    let mut at = 2usize;
    let mut salt: Option<[u8; KDF_SALT_LEN]> = None;
    let mut passes: Option<u32> = None;
    let mut memory_kib: Option<u32> = None;
    let mut lanes: Option<u32> = None;
    let mut uuid_ok = false;
    while at < bytes.len() && bytes[at] != 0 {
        let kind = bytes[at];
        at += 1;
        let name_len = u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?) as usize;
        at += 4;
        let name = bytes.get(at..at + name_len)?;
        at += name_len;
        let value_len = u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?) as usize;
        at += 4;
        let value = bytes.get(at..at + value_len)?;
        at += value_len;
        match (kind, name) {
            (VD_BYTES, b"$UUID") => uuid_ok = value == KDF_ARGON2ID,
            (VD_BYTES, b"S") => salt = value.try_into().ok(),
            (VD_U64, b"I") => {
                passes = u32::try_from(u64::from_le_bytes(value.try_into().ok()?)).ok()
            }
            (VD_U64, b"M") => {
                memory_kib = u32::try_from(u64::from_le_bytes(value.try_into().ok()?) / 1024).ok();
            }
            (VD_U32, b"P") => lanes = Some(u32::from_le_bytes(value.try_into().ok()?)),
            (VD_U32, b"V") if u32::from_le_bytes(value.try_into().ok()?) != 0x13 => return None,
            _ => {}
        }
    }
    if !uuid_ok || at >= bytes.len() {
        return None;
    }
    let cost = Cost {
        memory_kib: memory_kib?,
        passes: passes?,
        lanes: lanes?,
    };
    if cost.memory_kib > MAX_MEMORY_KIB {
        return None;
    }
    Some((salt?, cost))
}

// ---------------------------------------------------------------------------
// The HMAC block stream.
// ---------------------------------------------------------------------------

/// One block of the stream: its HMAC, its length and its ciphertext.
fn put_block(i: u64, data: &[u8], base: &Secret<[u8; 64]>, out: &mut Vec<u8>) {
    let len = (data.len() as u32).to_le_bytes();
    let tag = hmac(&block_key(base, i), &[&i.to_le_bytes(), &len, data]);
    out.extend_from_slice(&tag);
    out.extend_from_slice(&len);
    out.extend_from_slice(data);
}

/// Whether every block of `bytes` verifies and the stream ends with the
/// empty block the format terminates on.
fn blocks_verify(bytes: &[u8], base: &Secret<[u8; 64]>) -> bool {
    let mut at = 0usize;
    let mut i = 0u64;
    loop {
        if at + HASH_LEN + 4 > bytes.len() {
            return false;
        }
        let tag = &bytes[at..at + HASH_LEN];
        let len_bytes = &bytes[at + HASH_LEN..at + HASH_LEN + 4];
        let len = u32::from_le_bytes(match len_bytes.try_into() {
            Ok(v) => v,
            Err(_) => return false,
        }) as usize;
        at += HASH_LEN + 4;
        if at + len > bytes.len() {
            return false;
        }
        let data = &bytes[at..at + len];
        at += len;
        if hmac(&block_key(base, i), &[&i.to_le_bytes(), len_bytes, data]).as_slice() != tag {
            return false;
        }
        if len == 0 {
            return at == bytes.len();
        }
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// The body: the inner header and the XML document.
// ---------------------------------------------------------------------------

/// The inner header: the stream cipher, its key, and the end. No
/// binaries, because this writer stores no attachments.
fn inner_header(inner_key: &[u8], out: &mut Vec<u8>) {
    put_field(INNER_STREAM_ID, &INNER_STREAM_CHACHA20.to_le_bytes(), out);
    put_field(INNER_STREAM_KEY, inner_key, out);
    put_field(INNER_END, &[], out);
}

/// The time every element carries: zero seconds from the format's
/// epoch. The device has no clock, and a date it made up would be a
/// statement of fact that is not one.
const ZERO_TIME: &[u8] = b"AAAAAAAAAAA=";

/// The XML document: one group, one entry per item, the secret field
/// protected by the inner stream.
fn document(items: &[Item<'_>], inner_key: &[u8], master_seed: &[u8], out: &mut Vec<u8>) {
    let mut h = Sha512::new();
    h.update(inner_key);
    let mut wide = Secret::new(<[u8; 64]>::from(h.finalize()));
    let mut key = Secret::new([0u8; 32]);
    key.expose_mut().copy_from_slice(&wide.expose()[..32]);
    let mut stream = chacha20(&key, &wide.expose()[32..44]);
    key.zeroize();
    wide.zeroize();

    out.extend_from_slice(b"<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"yes\"?>");
    out.extend_from_slice(b"<KeePassFile><Meta><Generator>OpenSigner</Generator>");
    out.extend_from_slice(b"<DatabaseName>OpenSigner</DatabaseName>");
    out.extend_from_slice(b"<MemoryProtection><ProtectTitle>False</ProtectTitle>");
    out.extend_from_slice(b"<ProtectUserName>False</ProtectUserName>");
    out.extend_from_slice(b"<ProtectPassword>True</ProtectPassword>");
    out.extend_from_slice(b"<ProtectURL>False</ProtectURL>");
    out.extend_from_slice(b"<ProtectNotes>False</ProtectNotes></MemoryProtection></Meta>");
    out.extend_from_slice(b"<Root><Group>");
    put_uuid(master_seed, 0xFF, out);
    out.extend_from_slice(b"<Name>OpenSigner</Name>");
    put_times(out);
    for (i, item) in items.iter().enumerate() {
        out.extend_from_slice(b"<Entry>");
        put_uuid(master_seed, i as u8, out);
        out.extend_from_slice(b"<IconID>0</IconID>");
        put_times(out);
        put_string(b"Title", item.title, out);
        put_string(b"UserName", b"", out);
        put_protected(b"Password", item.secret, &mut stream, out);
        put_string(b"Notes", item.notes, out);
        out.extend_from_slice(b"</Entry>");
    }
    out.extend_from_slice(b"</Group></Root></KeePassFile>");
}

/// One entry's or group's UUID, derived from the master seed so that
/// nothing outside the file decides it and no two entries share one.
fn put_uuid(master_seed: &[u8], i: u8, out: &mut Vec<u8>) {
    let mut h = Sha256::new();
    h.update(master_seed);
    h.update(b"osk-kdbx-uuid");
    h.update([i]);
    let digest = h.finalize();
    out.extend_from_slice(b"<UUID>");
    base64(&digest[..16], out);
    out.extend_from_slice(b"</UUID>");
}

/// The Times block, the same on every element.
fn put_times(out: &mut Vec<u8>) {
    out.extend_from_slice(b"<Times><CreationTime>");
    out.extend_from_slice(ZERO_TIME);
    out.extend_from_slice(b"</CreationTime><LastModificationTime>");
    out.extend_from_slice(ZERO_TIME);
    out.extend_from_slice(b"</LastModificationTime><LastAccessTime>");
    out.extend_from_slice(ZERO_TIME);
    out.extend_from_slice(b"</LastAccessTime><ExpiryTime>");
    out.extend_from_slice(ZERO_TIME);
    out.extend_from_slice(b"</ExpiryTime><Expires>False</Expires><UsageCount>0</UsageCount>");
    out.extend_from_slice(b"<LocationChanged>");
    out.extend_from_slice(ZERO_TIME);
    out.extend_from_slice(b"</LocationChanged></Times>");
}

/// One plain string field.
fn put_string(key: &[u8], value: &[u8], out: &mut Vec<u8>) {
    out.extend_from_slice(b"<String><Key>");
    out.extend_from_slice(key);
    out.extend_from_slice(b"</Key><Value>");
    escape(value, out);
    out.extend_from_slice(b"</Value></String>");
}

/// One protected field: the value is encrypted with the inner stream,
/// in the order the document states it, and written in Base64.
fn put_protected(key: &[u8], value: &[u8], stream: &mut ChaCha20, out: &mut Vec<u8>) {
    let mut buf: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::new());
    buf.extend_from_slice(value);
    stream.apply_keystream(&mut buf);
    out.extend_from_slice(b"<String><Key>");
    out.extend_from_slice(key);
    out.extend_from_slice(b"</Key><Value Protected=\"True\">");
    base64(&buf, out);
    out.extend_from_slice(b"</Value></String>");
}

/// `text` as XML character data. The five markup characters are
/// escaped; a control character XML 1.0 has no representation for is
/// dropped, because a document that cannot be parsed is worse than a
/// note without its bell character.
fn escape(text: &[u8], out: &mut Vec<u8>) {
    for &b in text {
        match b {
            b'&' => out.extend_from_slice(b"&amp;"),
            b'<' => out.extend_from_slice(b"&lt;"),
            b'>' => out.extend_from_slice(b"&gt;"),
            b'"' => out.extend_from_slice(b"&quot;"),
            b'\'' => out.extend_from_slice(b"&apos;"),
            b'\t' | b'\n' | b'\r' => out.push(b),
            0x00..=0x1F | 0x7F => {}
            _ => out.push(b),
        }
    }
}

/// Base64, as the XML stores bytes.
fn base64(bytes: &[u8], out: &mut Vec<u8>) {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    for chunk in bytes.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = chunk.get(1).copied().map_or(0, u32::from);
        let b2 = chunk.get(2).copied().map_or(0, u32::from);
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(n >> 18) as usize & 0x3F]);
        out.push(ALPHABET[(n >> 12) as usize & 0x3F]);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 0x3F]
        } else {
            b'='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 0x3F]
        } else {
            b'='
        });
    }
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

    fn one() -> Vec<u8> {
        write(
            &[Item {
                title: b"73c5da0a",
                notes: b"12 words, English",
                secret: b"abandon abandon about",
            }],
            b"correct horse",
            CHEAP,
            &seed(),
        )
        .expect("a database")
    }

    /// The file this writer makes authenticates: the header's hash and
    /// HMAC verify under the passphrase, and the block stream verifies
    /// with it.
    #[test]
    fn a_written_database_verifies_under_its_passphrase() {
        let bytes = one();
        assert!(verify(&bytes, b"correct horse"));
        assert!(!verify(&bytes, b"correct hors"));
    }

    /// One byte changed anywhere in the file — the header, its hash,
    /// its HMAC, a block's HMAC, the ciphertext — fails the check.
    #[test]
    fn one_changed_byte_anywhere_fails() {
        let bytes = one();
        for at in 0..bytes.len() {
            let mut flipped = bytes.clone();
            flipped[at] ^= 1;
            assert!(!verify(&flipped, b"correct horse"), "a changed byte passed");
        }
        let mut short = bytes.clone();
        short.pop();
        assert!(!verify(&short, b"correct horse"));
        let mut long = bytes;
        long.push(0);
        assert!(!verify(&long, b"correct horse"));
    }

    /// The cost the person chose is what the file's KDF parameters
    /// state, so a KeePass app opening it pays what this device paid.
    #[test]
    fn the_file_states_the_cost_it_was_written_at() {
        let cost = Cost {
            memory_kib: 64,
            passes: 2,
            lanes: 1,
        };
        let bytes = write(
            &[Item {
                title: b"a",
                notes: b"b",
                secret: b"c",
            }],
            b"correct horse",
            cost,
            &seed(),
        )
        .expect("a database");
        assert_eq!(cost_of(&bytes), Some(cost));
        assert!(verify(&bytes, b"correct horse"));
    }
}
