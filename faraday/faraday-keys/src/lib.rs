//! Where Faraday's key material is used.
//!
//! Every function here takes the secret it works on as an argument (a
//! master key, seed words, a passphrase, a record's bytes) and gives back
//! a public result, or a secret wrapped in `Zeroizing` where the caller
//! needs one. The flow around each (state, sheets, toasts, the Outbox,
//! strings) stays in faraday-core. The session's keys and the open
//! vaults' slot keys and records are held here too (`docs/KEYS.md` steps
//! 2 and 3), and nothing outside this crate touches the primitive
//! crates' secret types.
//!
//! This crate does not depend on faraday-core.

pub mod apk;
pub mod bip85;
pub mod carry;
pub mod frost;
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

pub use identity::{Material, fingerprint, fp_text};
pub use operations::{Dial, Paper, codex32_seed, slip39_seed};
pub use session::{
    CopyCheck, Held, Infos, KEY_TEXTS_KEPT, KeyId, KeyInfo, Keys, Part, Reveal, Revealed, WordsId,
    compare_words, words_id,
};
pub use texts::Spec;
