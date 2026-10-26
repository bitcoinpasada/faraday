//! Another paper form of a key held here, beside its words:
//! a Seed XOR split of its words
//! into parts that are each BIP-39 words, every part needed. The
//! arithmetic is `faraday-keys`' (`Keys::seed_xor`);
//! the randomness is fresh from the system. The parts are shown to copy by
//! hand and never written to a file; only the seed itself goes into a
//! vault or, past the secret sheet, a file (`crate::secrets`).

use zeroize::Zeroizing;

/// A key's other paper form, on screen.
pub enum PaperForm {
    /// Seed XOR parts, each a line of words; every one is needed.
    Xor(faraday_keys::Held<Vec<Zeroizing<String>>>),
}
