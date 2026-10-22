//! The shapes of Faraday's samples (F81, `docs/MODES.md` §5): public
//! dummies a person can load in Demo. The data is in `faraday-samples`,
//! a crate only the desktop app depends on; this module holds the types
//! it fills in and what the session does with them, so core never links
//! the data (`tools/lint-keys-tests.sh` holds that).
//!
//! Every field is public data of `&'static` text or bytes. Nothing here
//! parses or derives; the fingerprints are literals that the samples
//! crate's tests check against the code that derives them.

use faraday_bip::keys::Fingerprint;
use faraday_keys::Handout;

/// The form a key sample is read in (`docs/MODES.md` §5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleForm {
    /// BIP-39 words, in any of the lists.
    Words,
    /// SLIP-39 shares.
    Slip39,
    /// codex32 strings: a secret, or the shares of a set.
    Codex32,
    /// Seed XOR parts, which make the key's words.
    Xor,
    /// A master seed, which has no words (a 64-byte hex seed).
    Seed,
    /// An `.oskb` backup holding a master seed.
    Backup,
    /// An LND cipher seed, for the Lightning node key only.
    Aezeed,
}

impl SampleForm {
    /// Whether a key of this form takes a passphrase (BIP-39, SLIP-39, and
    /// Seed XOR, whose result is BIP-39 words), so is public twice: with
    /// none and with `password`.
    pub fn takes_passphrase(self) -> bool {
        matches!(
            self,
            SampleForm::Words | SampleForm::Slip39 | SampleForm::Xor
        )
    }
}

/// A sample key.
#[derive(Debug, Clone, Copy)]
pub struct SampleKey {
    /// The name of its string in `faraday-strings` (`sample_words`, ...).
    pub name: &'static str,
    /// For a words sample made of one word repeated: the word; else "".
    pub word: &'static str,
    /// For such a sample: how many words; else 0.
    pub count: u8,
    /// The form it is read in.
    pub form: SampleForm,
    /// Whether the Load samples sheet lists it. A key a wallet needs and
    /// the table of §5.1 does not list (the 3 of 5's) is not.
    pub listed: bool,
    /// What is typed or scanned, one text per part: the phrase, the
    /// shares (as many as load the key), the codex32 strings, the Seed XOR
    /// parts' words, the hex master seed, the cipher seed's words.
    /// Empty for a backup.
    pub parts: &'static [&'static str],
    /// For Seed XOR: the words of the key the parts make; else "".
    pub combined: &'static str,
    /// For a backup: the `.oskb` file; else empty.
    pub file: &'static [u8],
    /// The key's fingerprint with no passphrase; `None` for a cipher
    /// seed, which loads no key.
    pub plain: Option<[u8; 4]>,
    /// Its fingerprint with the passphrase `password`, for a form that
    /// takes one.
    pub with_password: Option<[u8; 4]>,
    /// For a cipher seed: the Lightning node id it gives.
    pub node: Option<[u8; 33]>,
}

/// A sample wallet (`docs/MODES.md` §5.2).
#[derive(Debug, Clone, Copy)]
pub struct SampleWallet {
    /// The name of its string in `faraday-strings` (`sample_savings`, ...).
    pub name: &'static str,
    /// What `Session::add_wallet` takes: its descriptor, or for the FROST
    /// wallet its record text.
    pub text: &'static str,
    /// Its unsigned spend (PSBT bytes); empty where there is none.
    pub spend: &'static [u8],
    /// The fingerprints of the sample keys it uses, in its own order:
    /// what the tick loads with it.
    pub keys: &'static [[u8; 4]],
}

/// The other inputs a flow takes (`docs/MODES.md` §5.3).
#[derive(Debug, Clone, Copy)]
pub struct SampleInputs {
    /// A vault file (`.ofv`) under the passphrase `password`.
    pub vault: &'static [u8],
    /// A signed message, in the text form a person pastes: made by sample
    /// Spending's first receive address.
    pub signed_message: &'static str,
    /// That address.
    pub address: &'static str,
    /// A TOTP setup code (`otpauth://` text).
    pub totp: &'static str,
}

/// Every sample, as the samples crate holds them.
#[derive(Debug, Clone, Copy)]
pub struct Samples {
    /// The keys, in the order the sheet lists them.
    pub keys: &'static [SampleKey],
    /// The wallets, in the order the sheet lists them.
    pub wallets: &'static [SampleWallet],
    /// The other inputs.
    pub inputs: SampleInputs,
}

/// The phrase of a words sample made of one word: `word` `count` times.
pub fn repeated(word: &str, count: u8) -> String {
    vec![word; usize::from(count)].join(" ")
}

impl Samples {
    /// What New key hands out: the words samples made of one word (any
    /// length), in the order of `keys`. Allocates a short list.
    pub fn handouts(&self) -> Vec<Handout> {
        self.keys
            .iter()
            .filter(|k| k.form == SampleForm::Words && !k.word.is_empty())
            .filter_map(|k| {
                Some(Handout {
                    name: k.word,
                    words: k.count,
                    plain: Fingerprint(k.plain?),
                    with_password: Fingerprint(k.with_password?),
                })
            })
            .collect()
    }

    /// Every fingerprint Demo takes: each key's with none, and each key
    /// that takes a passphrase with `password` too. Allocates a list.
    pub fn public(&self) -> Vec<Fingerprint> {
        self.keys
            .iter()
            .flat_map(|k| [k.plain, k.with_password])
            .flatten()
            .map(Fingerprint)
            .collect()
    }

    /// The Lightning node ids Demo takes. Allocates a short list.
    pub fn nodes(&self) -> Vec<[u8; 33]> {
        self.keys.iter().filter_map(|k| k.node).collect()
    }
}
