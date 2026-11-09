//! What makes a file a vault by its contents (`docs/VAULT.md` §1.1 and §2,
//! `docs/VAULT.md` §1.1): KDBX's signature, major version 4 and
//! a length a vault can have, whatever its name.

/// KDBX's two signatures, little-endian, as KDBX 2 to 4 begin.
pub const SIGNATURE: [u8; 8] = [0x03, 0xD9, 0xA2, 0x9A, 0x67, 0xFB, 0x4B, 0xB5];
/// The bytes that say what a file is: the signatures and the version.
pub const HEAD_LEN: usize = 12;

/// The header, its SHA-256 and its HMAC as Faraday writes them
/// (`docs/VAULT.md` §1.1).
const FARADAY_HEADER: usize = 249 + 32 + 32;
/// A block's HMAC and length.
const BLOCK_OVERHEAD: usize = 32 + 4;
/// The most a vault's plaintext holds: 128 MiB (`docs/VAULT.md` §2).
const MAX_PLAIN: usize = 128 * 1024 * 1024;
/// The most a stored stream takes: the plaintext as a compressing program
/// may have left it, and an eighth of slack that covers the frames.
const MAX_STORED: usize = MAX_PLAIN + MAX_PLAIN / 8;

/// The KDBX major version of the first twelve bytes, if they begin with
/// KDBX's signatures.
pub fn kdbx_major(bytes: &[u8]) -> Option<u16> {
    if bytes.len() < HEAD_LEN || !bytes.starts_with(&SIGNATURE) {
        return None;
    }
    Some(u16::from_le_bytes([bytes[10], bytes[11]]))
}

/// The longest file a vault has: its header and the most a plaintext
/// takes with the slack for its frames.
pub const MAX_FILE_LEN: u64 = (FARADAY_HEADER + MAX_STORED) as u64;

/// Whether `len` is a length a vault can have: the header, its hash and
/// its HMAC and the stream's empty block at least, and at most what the
/// largest plaintext takes with the slack for its frames.
pub fn is_vault_len(len: u64) -> bool {
    let least = (FARADAY_HEADER + BLOCK_OVERHEAD) as u64;
    (least..=MAX_FILE_LEN).contains(&len)
}

/// Whether a whole file is a vault by its contents: KDBX's signatures,
/// major version 4 and a vault's length. Says nothing of whether the rest
/// is sound.
pub fn is_vault_file(bytes: &[u8]) -> bool {
    kdbx_major(bytes) == Some(4) && is_vault_len(bytes.len() as u64)
}

/// Whether the bytes begin as a KeePass database of version 3 or older,
/// which Faraday names and does not open.
pub fn is_older_kdbx(bytes: &[u8]) -> bool {
    kdbx_major(bytes).is_some_and(|m| m <= 3)
}
