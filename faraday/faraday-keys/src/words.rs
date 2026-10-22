//! Where seed words become a [`Mnemonic`]: typed, scanned as a SeedQR,
//! read from text, or made from fresh entropy, and where fresh entropy
//! becomes SLIP-39 shares. The caller holds what comes back as input in
//! hand: to check it, show it for writing down, or hand it to
//! [`Keys`](crate::Keys) (`docs/KEYS.md` §2). Nothing outside this crate
//! makes one (`tools/lint-secrets.sh` rule 4), and nothing outside it can
//! derive a key from one: `to_seed` and the master key are refused there
//! too.

use faraday_bip::bip39::{Error, Language, Mnemonic};
use faraday_bip::slip39;
use faraday_crypto::work::Compute;

/// The words whose indices into `lang`'s list are `indices`, when the
/// count and the checksum hold.
pub fn from_indices(lang: Language, indices: &[u16]) -> Result<Mnemonic, Error> {
    Mnemonic::from_indices(lang, indices)
}

/// Costs a word-list search per word and a hash. The words `text` holds
/// in `lang`, when every one is a word and the checksum holds.
pub fn parse(c: &Compute, lang: Language, text: &str) -> Result<Mnemonic, Error> {
    Mnemonic::parse(c, lang, text)
}

/// The words `entropy` is in `lang`, for 16 to 32 bytes in steps of 4.
pub fn from_entropy(lang: Language, entropy: &[u8]) -> Result<Mnemonic, Error> {
    Mnemonic::from_entropy(lang, entropy)
}

/// The words a standard SeedQR's digits are in `lang`.
pub fn from_seedqr_digits(
    digits: &[u8],
    lang: Language,
) -> Result<Mnemonic, faraday_codec::seedqr::Error> {
    faraday_codec::seedqr::from_digits(digits, lang)
}

/// The words a CompactSeedQR's entropy bytes are in `lang`.
pub fn from_compact_seedqr(
    bytes: &[u8],
    lang: Language,
) -> Result<Mnemonic, faraday_codec::seedqr::Error> {
    faraday_codec::seedqr::from_entropy(bytes, lang)
}

/// Costs SLIP-39's PBKDF2 rounds (the share encryption). Splits the fresh
/// master secret `secret` into one group of `count` shares, `threshold` of
/// them needed, with no passphrase, under `identifier`; `random` fills
/// the random parts. The shares, written into `out`; how many there are.
pub fn slip39_split(
    c: &Compute,
    secret: &[u8],
    identifier: u16,
    threshold: u8,
    count: u8,
    random: &mut dyn FnMut(&mut [u8]),
    out: &mut [slip39::Share],
) -> Result<usize, slip39::Error> {
    let groups = [slip39::GroupSpec { threshold, count }];
    slip39::split(c, secret, b"", identifier, true, 1, 1, &groups, random, out)
}
