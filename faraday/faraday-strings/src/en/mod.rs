//! Faraday's English wording: every string Faraday's screens, the stick's
//! device check and the Starter show (F68), one file per area.
//!
//! The struct and the static come from one list, kept in the area files.
//! Each file defines a `words_<area>` macro that takes the names of the
//! macros still to run, adds its own entries to the list handed on and calls
//! the next one; the last one is `strings!`, which writes the struct and the
//! static. The order below is the order of that chain.

/// The struct of every string and the English static, from one list:
/// each key's doc, then its value, so the two are read together. A key
/// whose doc is followed by `#[cfg(…)]` is in that build only. It is the
/// end of the chain: its list is every area's entries, handed on whole.
macro_rules! strings {
    ([] $( $(#[doc = $doc:expr])* $(#[cfg($cfg:meta)])? $name:ident: $value:expr, )*) => {
        /// Every string Faraday shows, grouped by screen.
        pub struct Strings {
            $( $(#[doc = $doc])* $(#[cfg($cfg)])? pub $name: &'static str, )*
        }

        /// The English wording.
        pub static EN: Strings = Strings {
            $( $(#[cfg($cfg)])? $name: $value, )*
        };
    };
}

#[macro_use]
mod common;
#[macro_use]
mod backups;
#[macro_use]
mod bip85;
#[macro_use]
mod bip_refusal;
#[macro_use]
mod boot_import;
#[macro_use]
mod catalog;
#[macro_use]
mod codex32;
#[macro_use]
mod compact;
#[macro_use]
mod compare;
#[macro_use]
mod create;
#[macro_use]
mod decode;
#[macro_use]
mod explore;
#[macro_use]
mod forms;
#[macro_use]
mod glance;
#[macro_use]
mod gpg;
#[macro_use]
mod invoice;
#[macro_use]
mod keygen;
#[macro_use]
mod lightning;
#[macro_use]
mod medium;
#[macro_use]
mod own_randomness;
#[macro_use]
mod paper;
#[macro_use]
mod plan;
#[macro_use]
mod psbt_refusal;
#[macro_use]
mod samples;
#[macro_use]
mod scan;
#[macro_use]
mod screens_backup;
#[macro_use]
mod screens_create;
#[macro_use]
mod screens_entry;
#[macro_use]
mod screens_files;
#[macro_use]
mod screens_home;
#[macro_use]
mod screens_parts;
#[macro_use]
mod screens_restore;
#[macro_use]
mod screens_settings;
#[macro_use]
mod screens_sheets;
#[macro_use]
mod screens_sidebar;
#[macro_use]
mod screens_spend;
#[macro_use]
mod screens_visit;
#[macro_use]
mod screens_wallets;
#[macro_use]
mod seeds;
#[macro_use]
mod silent;
#[macro_use]
mod sources;
#[macro_use]
mod spend_backup;
#[macro_use]
mod starter;
#[macro_use]
mod stick;
#[macro_use]
mod stick_files;
#[macro_use]
mod sweep;
#[macro_use]
mod tools;
#[macro_use]
mod transfer;
#[macro_use]
mod vanity;
#[macro_use]
mod vault;
#[macro_use]
mod vm_names;
#[macro_use]
mod walkthrough;
#[macro_use]
mod wallet;

words_common! {
    [words_backups words_bip85 words_bip_refusal words_boot_import words_catalog words_codex32 words_compact words_compare words_create words_decode words_explore words_forms words_glance words_gpg words_invoice words_keygen words_lightning words_medium words_own_randomness words_paper words_plan words_psbt_refusal words_samples words_scan words_screens_backup words_screens_create words_screens_entry words_screens_files words_screens_home words_screens_parts words_screens_restore words_screens_settings words_screens_sheets words_screens_sidebar words_screens_spend words_screens_visit words_screens_wallets words_seeds words_silent words_sources words_spend_backup words_starter words_stick words_stick_files words_sweep words_tools words_transfer words_vanity words_vault words_vm_names words_walkthrough words_wallet strings]
}
