//! Where Faraday's key material is held and used (`docs/KEYS.md`, F70).
//!
//! - **Held.** [`Keys`] holds the session's keys (words, passphrase,
//!   master key) and [`vault::OpenVaults`] the open vaults' slot keys and
//!   decrypted records, in private fields. Callers address a key by its
//!   place and a record by its vault and index, and read public values.
//! - **Used.** Every operation on a secret is here: signing
//!   ([`psbt`], [`message`]), derivation and key texts ([`texts`],
//!   [`identity`]), [`bip85`], [`lightning`], [`silent`],
//!   [`vanity`], paper forms ([`operations`], and [`codex32`], the work
//!   set of a codex32 set being made), [`gpg`],
//!   [`secure_boot`], [`totp`], and seed words made into a `Mnemonic` ([`words`]). Each gives back a
//!   public result. The flow around each (state, sheets, toasts, the
//!   Outbox, strings) stays in faraday-core.
//! - **Leaving.** A secret leaves only for a [`Reveal`] reason, lent as
//!   [`Revealed`] or held as [`Held`], neither `Clone` nor `Debug`, wiped
//!   when dropped. Types that hold a secret ([`silent::Receiver`],
//!   [`lightning::Node`], [`vault::MadeKey`]) give public values, and their secret only for a
//!   reason.
//! - **Held to.** `tools/lint-secrets.sh` rule 4 refuses key material and
//!   the primitive crates' secret-taking functions anywhere else under
//!   `faraday/`.
//!
//! This crate does not depend on faraday-core.

pub mod bip85;
pub mod codex32;
pub mod gpg;
pub mod identity;
pub mod lightning;
pub mod message;
pub mod operations;
pub mod psbt;
pub mod secure_boot;
pub mod session;
pub mod silent;
pub mod texts;
pub mod totp;
pub mod vanity;
pub mod vault;
pub mod words;

pub use codex32::{Making, StringCheck, WorkSet};
pub use identity::{Material, fingerprint, fp_text};
pub use operations::{
    Dial, Paper, codex32_random_len, codex32_seed, seed_xor_random_len, slip39_seed,
};
pub use session::{
    CopyCheck, Handout, Held, Infos, KEY_TEXTS_KEPT, KeyId, KeyInfo, Keys, Part, Reveal, Revealed,
    WordsId, compare_words, words_id,
};
pub use texts::Spec;
