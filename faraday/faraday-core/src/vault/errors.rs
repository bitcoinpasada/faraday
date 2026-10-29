//! What Unlock and Create a vault say when the vault crate refuses: its
//! [`Error`] and [`Fault`] are kinds, and their words are here, from the
//! strings file.

use crate::strings::{EN, fill};
use faraday_vault::Error;
use faraday_vault::records::{Fault, kind};

/// The reason `e` gives, in a few words, for the screen.
pub fn error_text(e: Error) -> String {
    match e {
        Error::Magic => EN.vault_error_magic.to_string(),
        Error::Version => EN.vault_error_version.to_string(),
        Error::SlotSize => EN.vault_error_slot_size.to_string(),
        Error::Length => EN.vault_error_length.to_string(),
        Error::Cost => EN.vault_error_cost.to_string(),
        Error::Memory(mib) => fill(EN.vault_error_memory, &[&mib.to_string()]),
        Error::Passphrase => EN.vault_error_passphrase.to_string(),
        Error::TwoSlots => EN.vault_error_two_slots.to_string(),
        Error::Commitment => EN.vault_error_commitment.to_string(),
        Error::Contents(f) => fill(EN.vault_error_contents, &[&fault_text(f)]),
        Error::Full => EN.vault_error_full.to_string(),
        Error::Passphrases => EN.vault_error_passphrases.to_string(),
        Error::SamePassphrase => EN.vault_error_same_passphrase.to_string(),
        Error::OtherFile => EN.vault_error_other_file.to_string(),
    }
}

/// What is faulty in a slot's contents, in a few words.
fn fault_text(f: Fault) -> String {
    match f {
        Fault::Format => EN.vault_fault_format.to_string(),
        Fault::UnknownType(t) => fill(EN.vault_fault_unknown_type, &[&t.to_string()]),
        Fault::Truncated => EN.vault_fault_truncated.to_string(),
        Fault::Trailing => EN.vault_fault_trailing.to_string(),
        Fault::UnknownField(t, f) => fill(
            EN.vault_fault_unknown_field,
            &[&f.to_string(), &t.to_string()],
        ),
        Fault::Missing(t, f) => fill(EN.vault_fault_missing, &[&t.to_string(), &f.to_string()]),
        Fault::Repeated(t, f) => fill(EN.vault_fault_repeated, &[&f.to_string(), &t.to_string()]),
        Fault::Value(t, f) => fill(EN.vault_fault_value, &[&f.to_string(), &t.to_string()]),
        Fault::TwoLabels => EN.vault_fault_two_labels.to_string(),
        Fault::Seals => EN.vault_fault_seals.to_string(),
        Fault::Duplicate { kind: ty, of } => {
            let of = format!("{:08x}", u32::from_be_bytes(of));
            if ty == kind::PASSPHRASE {
                fill(EN.vault_fault_two_passphrases, &[&of])
            } else {
                fill(EN.vault_fault_two_amounts, &[&of])
            }
        }
    }
}
