//! What Unlock, Create and Lock say when the vault crate refuses: its
//! [`Error`] and [`Fault`] are kinds, and their words are here, from the
//! strings file; so are the names Faraday writes into a vault.

use crate::strings::{EN, fill};
use faraday_keys::vault::Names;
use faraday_vault::file::Error;
use faraday_vault::kdbx::Unsupported;
use faraday_vault::kdbx::{Error as Kdbx, items::Fault};

/// The reason `e` gives, in a few words, for the screen.
pub fn error_text(e: &Error) -> String {
    match e {
        Error::Kdbx(k) => kdbx_text(k),
        Error::Xml(_) => EN.vault_error_contents.to_string(),
        Error::Unwritable(_) => EN.vault_error_unwritable.to_string(),
        Error::ReadOnly => EN.vault_error_read_only.to_string(),
        Error::Full { .. } => EN.vault_error_full.to_string(),
        Error::Memory(mib) => fill(EN.vault_error_memory, &[&mib.to_string()]),
    }
}

/// [`error_text`] for a vault `name` whose open or seal stopped, saying the
/// numbers a person can act on: the bytes a seal is over the most a vault
/// holds, and, where `free_mib` (what the computer has free) is known, the
/// memory a vault needs against it.
pub fn stopped_text(e: &Error, name: &str, free_mib: Option<u32>) -> String {
    match (e, free_mib) {
        (Error::Full { over }, _) => fill(EN.vault_cannot_seal, &[name, &over.to_string()]),
        (Error::Memory(need), Some(free)) => fill(
            EN.vaults_needs_this_computer,
            &[&super::mib_text(*need), &super::mib_text(free)],
        ),
        _ => error_text(e),
    }
}

/// The reason a KDBX container is refused, in a few words.
fn kdbx_text(k: &Kdbx) -> String {
    match k {
        Kdbx::NotKdbx => EN.vault_error_not_kdbx.to_string(),
        Kdbx::Older { .. } => EN.vault_error_older.to_string(),
        Kdbx::Newer { major, minor } => fill(EN.vault_error_newer, &[&format!("{major}.{minor}")]),
        Kdbx::UnknownField(n) => fill(EN.vault_error_unknown_field, &[&n.to_string()]),
        Kdbx::Unsupported(u) => unsupported_text(*u).to_string(),
        Kdbx::Cost => EN.vault_error_cost.to_string(),
        Kdbx::Memory(mib) => fill(EN.vault_error_memory, &[&mib.to_string()]),
        Kdbx::Passphrase => EN.vault_error_wrong.to_string(),
        Kdbx::EmptyPassphrase => EN.vault_error_empty_passphrase.to_string(),
        Kdbx::Damaged => EN.vault_error_damaged.to_string(),
        Kdbx::TooLarge => EN.vault_error_too_large.to_string(),
        Kdbx::Full(_) => EN.vault_error_full.to_string(),
        Kdbx::ReadBack => EN.vault_error_read_back.to_string(),
    }
}

/// The setting a file is saved in that this build does not read, named.
fn unsupported_text(u: Unsupported) -> &'static str {
    match u {
        Unsupported::Twofish => EN.vault_error_twofish,
        Unsupported::OtherCipher => EN.vault_error_other_cipher,
        Unsupported::AesKdf => EN.vault_error_aes_kdf,
        Unsupported::OtherKdf => EN.vault_error_other_kdf,
        Unsupported::Compression => EN.vault_error_compression,
        Unsupported::Salsa20 => EN.vault_error_salsa20,
        Unsupported::OtherInnerStream => EN.vault_error_other_inner,
    }
}

/// What is faulty in an item that does not read, in a few words.
pub fn fault_text(f: Fault) -> &'static str {
    match f {
        Fault::UnknownKind => EN.vault_fault_unknown_kind,
        Fault::UnknownField => EN.vault_fault_unknown_field,
        Fault::Missing => EN.vault_fault_missing,
        Fault::Value => EN.vault_fault_value,
        Fault::Twice => EN.vault_fault_twice,
        Fault::Format => EN.vault_fault_format,
        Fault::SameUuid => EN.vault_fault_same_uuid,
        Fault::SameField => EN.vault_fault_same_field,
        Fault::Ambiguous => EN.vault_fault_ambiguous,
    }
}

/// The names Faraday writes into a vault, in the language the app is in
/// (`docs/VAULT.md` §7): one group per kind and the titles of items that
/// have none of their own.
pub fn names() -> Names {
    Names {
        group_keys: EN.vault_group_keys.to_string(),
        group_wallets: EN.vault_group_wallets.to_string(),
        group_notes: EN.vault_group_notes.to_string(),
        group_sheets: EN.vault_group_sheets.to_string(),
        group_gpg: EN.vault_group_gpg.to_string(),
        group_secure_boot: EN.vault_group_secure_boot.to_string(),
        group_wallet_shares: EN.vault_group_wallet_shares.to_string(),
        group_passphrases: EN.vault_group_passphrases.to_string(),
        group_shares: EN.vault_group_shares.to_string(),
        title_key: EN.vault_title_key.to_string(),
        title_note: EN.vault_title_note.to_string(),
        title_passphrase: EN.vault_title_passphrase.to_string(),
        title_share: EN.vault_title_share.to_string(),
        title_secure_boot: EN.vault_title_secure_boot.to_string(),
    }
}
