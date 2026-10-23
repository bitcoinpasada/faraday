//! Other paper forms of a key held here, beside its words:
//! a Seed XOR split of its words
//! into parts that are each BIP-39 words, every part needed; and codex32
//! shares of its BIP-32 seed (BIP 93), any `k` of `n` enough. The
//! arithmetic is `faraday-keys`' (`Keys::seed_xor`, `Keys::codex32_shares`);
//! the randomness is fresh from the system. The parts are shown to copy by
//! hand and never written to a file; only the seed itself goes into a
//! vault or, past the secret sheet, a file (`crate::secrets`).

use zeroize::Zeroizing;

/// A key's other paper form, on screen.
pub enum PaperForm {
    /// Seed XOR parts, each a line of words; every one is needed.
    Xor(faraday_keys::Held<Vec<Zeroizing<String>>>),
    /// codex32 shares; any `k` of them make the seed.
    Codex32 {
        /// How many make the seed.
        k: u8,
        /// The strings.
        strings: faraday_keys::Held<Vec<Zeroizing<String>>>,
    },
}
