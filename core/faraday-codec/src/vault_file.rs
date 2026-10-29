//! What makes a file a vault by its contents (`docs/VAULT-HARDENING.md`
//! §2.1): its first four bytes and its length, whatever its name.

/// The first four bytes of every Faraday vault (`docs/VAULT.md` §1).
pub const MAGIC: [u8; 4] = *b"FVLT";
/// The slot plaintext lengths a reader accepts (`docs/VAULT.md` §2),
/// smallest first.
pub const SLOT_SIZES: [u32; 4] = [65_536, 262_144, 1_048_576, 4_194_304];
/// Every vault has four slots.
pub const SLOTS: usize = 4;
/// The header: magic, version, memory, passes, lanes, slot length, salt.
pub const HEADER_LEN: usize = 4 + 1 + 4 + 4 + 4 + 4 + 32;
/// A slot's nonce and tag, beside its plaintext.
const SLOT_OVERHEAD: usize = 24 + 16;

/// A vault's whole length for a slot length.
pub const fn file_len(slot_len: u32) -> usize {
    HEADER_LEN + SLOTS * (slot_len as usize + SLOT_OVERHEAD)
}

/// Whether `len` is the length of a vault of one of the four sizes.
pub fn is_vault_len(len: u64) -> bool {
    SLOT_SIZES.iter().any(|&s| file_len(s) as u64 == len)
}

/// Whether the bytes begin with the vault magic.
pub fn has_magic(bytes: &[u8]) -> bool {
    bytes.starts_with(&MAGIC)
}

/// Whether a whole file is a vault by its contents: the magic and one of
/// the four vault lengths. Says nothing of whether the rest is sound.
pub fn is_vault_file(bytes: &[u8]) -> bool {
    has_magic(bytes) && is_vault_len(bytes.len() as u64)
}
