//! The shapes of Faraday's samples (F81, `docs/MODES.md` §5): public
//! dummies a person can load in Practice. The data is in [`crate::SAMPLES`]; this module holds
//! the types it fills in.
//!
//! Every field is public data of `&'static` text or bytes. Nothing here
//! parses or derives; the fingerprints are literals that this crate's
//! tests check against the code that derives them.

use faraday_bip::keys::Fingerprint;
use faraday_forms::{Form, Passphrase};
use faraday_text::Text;

/// The passphrase of every sample that has one: the backup, the vault, and
/// the second fingerprint of each key that takes a passphrase
/// (`docs/MODES.md` §6.2). All lowercase.
pub const PASSPHRASE: &str = "faraday";

/// The form a key sample is read in (`docs/MODES.md` §5.1): a form of the
/// table, or one the table does not hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleForm {
    /// A form of the table: BIP-39 words in any of the lists, SLIP-39
    /// shares, codex32 strings (a secret, or the shares of a set) or Seed
    /// XOR parts, which make the key's words.
    Form(Form),
    /// A master seed, which has no words (a 64-byte hex seed).
    Seed,
    /// An LND cipher seed, for the Lightning node key only.
    Aezeed,
}

impl SampleForm {
    /// Whether a key of this form takes a passphrase (BIP-39, SLIP-39, and
    /// Seed XOR, whose result is BIP-39 words), so is public twice: with
    /// none and with `faraday`.
    pub fn takes_passphrase(self) -> bool {
        match self {
            SampleForm::Form(f) => match f.passphrase() {
                Passphrase::Bip39 | Passphrase::Slip39 => true,
                Passphrase::None => false,
            },
            SampleForm::Seed | SampleForm::Aezeed => false,
        }
    }
}

/// How big a key sample is, as its type line says it (`docs/MODES.md`): what the form counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleSize {
    /// BIP-39 words (or an LND cipher seed's).
    Words(u8),
    /// BIP-39 words of the Japanese list.
    JapaneseWords(u8),
    /// This many shares, typed or scanned.
    Shares(u8),
    /// SLIP-39 shares, `need` of the `total` the set was dealt in.
    Threshold {
        /// How many shares recover the key.
        need: u8,
        /// How many the set holds.
        total: u8,
    },
    /// SLIP-39 shares of a set in groups, this many of them.
    GroupShares(u8),
    /// A codex32 secret, which is the key itself.
    Secret,
    /// Seed XOR parts.
    Parts(u8),
    /// A master seed of this many bytes.
    Bytes(u8),
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
    /// How big it is: what the form counts.
    pub size: SampleSize,
    /// Whether the Load samples sheet lists it. A key a wallet needs and
    /// the table of §5.1 does not list (the 3 of 5's) is not.
    pub listed: bool,
    /// What is typed or scanned, one text per part: the phrase, the
    /// shares (as many as load the key), the codex32 strings, the Seed XOR
    /// parts' words, the hex master seed, the cipher seed's words.
    pub parts: &'static [&'static str],
    /// For Seed XOR: the words of the key the parts make; else "".
    pub combined: &'static str,
    /// The key's fingerprint with no passphrase; `None` for a cipher
    /// seed, which loads no key.
    pub plain: Option<[u8; 4]>,
    /// Its fingerprint with the passphrase `faraday`, for a form that
    /// takes one.
    pub with_passphrase: Option<[u8; 4]>,
    /// For a cipher seed: the Lightning node id it gives.
    pub node: Option<[u8; 33]>,
}

/// What a sample wallet is, apart from its script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleKind {
    /// One key.
    Single,
    /// `need` signatures of `total` keys.
    Multisig {
        /// Signatures required.
        need: u8,
        /// Keys.
        total: u8,
    },
    /// A MuSig2 aggregate of this many keys.
    MuSig(u8),
    /// A key, and a second that spends after a delay.
    Timelock,
}

/// The script a sample wallet pays to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleScript {
    /// Pay to public key hash, or to script hash.
    Legacy,
    /// SegWit nested in script hash.
    NestedSegwit,
    /// Native SegWit.
    NativeSegwit,
    /// Taproot.
    Taproot,
    /// Taproot with a tree of scripts.
    TaprootTree,
}

/// A sample wallet (`docs/MODES.md` §5.2).
#[derive(Debug, Clone, Copy)]
pub struct SampleWallet {
    /// The name of its string in `faraday-strings` (`sample_savings`, ...).
    pub name: &'static str,
    /// What kind of wallet it is.
    pub kind: SampleKind,
    /// The script it pays to.
    pub script: SampleScript,
    /// What `Session::add_wallet` takes: its descriptor.
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
    /// A vault file (`.kdbx`) under the passphrase `faraday`.
    pub vault: &'static [u8],
    /// A signed message, in the text form a person pastes: made by sample
    /// Spending's first receive address.
    pub signed_message: &'static str,
    /// That address.
    pub address: &'static str,
    /// A TOTP setup code (`otpauth://` text).
    pub totp: &'static str,
    /// Check a signer's Sent: sample Spending's unsigned spend.
    pub signer_sent: &'static [u8],
    /// Its Returned, signed by sample key 1: clean.
    pub signer_returned: &'static [u8],
    /// That Returned with one proprietary pair added.
    pub signer_returned_proprietary: &'static [u8],
    /// Check a signer's second pair, signed elsewhere: Sent is
    /// `tools/vectors/psbt/demo-regtest.psbt`.
    pub signer_elsewhere_sent: &'static [u8],
    /// Its Returned, `signer-random-nonce.psbt`: signed by the "abandon"
    /// key, which Practice does not load at the start. Both are fixed
    /// data that `regenerate` copies.
    pub signer_elsewhere_returned: &'static [u8],
    /// Compare's first card: Bitcoin Core's `psbtbumpfee` original
    /// (`tools/vectors/psbt/bump-core-original.psbt`), fixed data that
    /// `regenerate` does not remake.
    pub compare_first: &'static [u8],
    /// Its second: Core's replacement (`bump-core-replacement.psbt`).
    pub compare_second: &'static [u8],
    /// A Krux encrypted code (KEF) of the first English words sample
    /// (`sample_guide`), under the passphrase `faraday`. It is fixed data:
    /// made once, by `Faraday::kef_make_seed` at the defaults of
    /// `docs/KEF.md` §10 (version 20, an 8-byte random id, 100,000
    /// rounds), and `regenerate` does not remake it, since a remake would
    /// draw a new id and IV. Empty where a set has none.
    pub krux_code: &'static [u8],
    /// Core Lightning's `hsm_secret` samples (`docs/MODES.md` §6), in the
    /// order plain, encrypted under the passphrase `faraday`, and the
    /// words form of the first English words sample. Empty where a set has
    /// none.
    pub hsm: &'static [SampleHsm],
    /// Check a timestamp: the proof (`.ots`) the US Commerce Department's
    /// timestamp of a press-release file made, which reaches its block
    /// through SHA-256 only. Empty where a set has none.
    pub timestamp_proof: &'static [u8],
    /// The SHA-256 of the file that proof stamps, in hex: the file is not
    /// kept, the tile takes its hash.
    pub timestamp_hash: &'static str,
    /// New key's dice entry: the rolls of the guide's key, as direct word
    /// selection takes them at twelve words (`tools/vectors/entropy/dice.json`).
    /// Empty where a set has none.
    pub dice_rolls: &'static str,
    /// The 80-byte header of the block the proof's Bitcoin attestation
    /// names, in hex.
    pub timestamp_header: &'static str,
    /// Read a Bitcoin Core wallet: the descriptor wallet's `wallet.dat` of
    /// Bitcoin Core's regtest, made by Bitcoin Core (`tools/vectors/core-wallet/`),
    /// in the order plain, then encrypted under the passphrase `faraday`.
    /// Empty where a set has none.
    pub core_wallet: &'static [&'static [u8]],
    /// The Core Lightning node of every English words sample, with no
    /// passphrase and under `faraday`. Empty where a set has none.
    pub hsm_words_nodes: &'static [[u8; 33]],
}

/// One `hsm_secret` sample file and the Lightning node id it gives.
#[derive(Debug, Clone, Copy)]
pub struct SampleHsm {
    /// The file's bytes.
    pub bytes: &'static [u8],
    /// The node id it gives.
    pub node: [u8; 33],
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
pub fn repeated(word: &str, count: u8) -> Text {
    let mut out = Text::new();
    for i in 0..count {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}

impl Samples {
    /// Every fingerprint Practice takes: each key's with none, and each key
    /// that takes a passphrase with `faraday` too. Allocates a list.
    pub fn public(&self) -> Vec<Fingerprint> {
        self.keys
            .iter()
            .flat_map(|k| [k.plain, k.with_passphrase])
            .flatten()
            .map(Fingerprint)
            .collect()
    }

    /// The Lightning node ids Practice takes. Allocates a short list.
    pub fn nodes(&self) -> Vec<[u8; 33]> {
        self.keys
            .iter()
            .filter_map(|k| k.node)
            .chain(self.inputs.hsm.iter().map(|h| h.node))
            .chain(self.inputs.hsm_words_nodes.iter().copied())
            .collect()
    }
}
