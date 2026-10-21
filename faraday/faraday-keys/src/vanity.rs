//! Vanity grinding: a key's first address made to begin with characters
//! the person chose, by one of two dials.

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::keys::{MasterKey, Network, ScriptType};
use faraday_bip::vanity::{self as engine, Key, Method};
use faraday_crypto::work::Compute;

pub use faraday_bip::vanity::Outcome;

/// What a grind turns.
pub enum Dial<'a> {
    /// The account index, below a master key.
    Account(&'a MasterKey),
    /// A suffix on the BIP-39 passphrase, over a key's words: `base` is
    /// the passphrase the key already has.
    Passphrase {
        /// The BIP-39 list the words are from.
        language: Language,
        /// The key's words, when it has them.
        words: Option<&'a str>,
        /// The key's own passphrase, to which the suffix is added.
        base: &'a [u8],
    },
}

/// Costs `budget` candidates, each a key derivation or a PBKDF2 and an
/// address. Tests a budget of candidates from `cursor` for `prefix`;
/// `None` for the passphrase dial when the key has no words that read.
#[allow(clippy::too_many_arguments)]
pub fn grind(
    c: &Compute,
    dial: Dial<'_>,
    script: ScriptType,
    prefix: &str,
    network: Network,
    cursor: u64,
    budget: u64,
) -> Option<Outcome> {
    Some(match dial {
        Dial::Account(master) => engine::grind(
            c,
            &Key::Master(master),
            &Method::Account,
            script,
            prefix,
            network,
            cursor,
            budget,
        ),
        Dial::Passphrase {
            language,
            words,
            base,
        } => {
            let m = Mnemonic::parse(c, language, words?).ok()?;
            engine::grind(
                c,
                &Key::Words(&m),
                &Method::Passphrase { base },
                script,
                prefix,
                network,
                cursor,
                budget,
            )
        }
    })
}
