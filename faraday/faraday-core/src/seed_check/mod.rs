//! My seed makes my wallet (`docs/FLOWS.md`): what a seed makes at the
//! account a person picks, for the person to hold against the wallet app
//! and the hardware wallet. This is the model: the accounts offered and
//! the Result's rows worked out from a key, whether the check holds it
//! alone ([`faraday_keys::Keys::check_read`]) or the session has it
//! loaded. The key is read only through `faraday-keys`' public reads (an
//! extended public key at a path); nothing here sees a secret.

use crate::explore::{implied_script, parse_path};
use crate::strings::EN;
use faraday_bip::bitcoin::bip32::{DerivationPath, Xpub};
use faraday_bip::keys::{Fingerprint, Network, ScriptType};
use faraday_bip::policy::WalletPolicy;
use faraday_bip::slip132;
use faraday_crypto::work::Compute;
use faraday_keys::Keys;
use faraday_text::Text;

/// The accounts a wallet app makes from one seed, in the order the
/// Account card lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Account {
    /// BIP-84, `m/84h/0h/0h`.
    NativeSegwit,
    /// BIP-86, `m/86h/0h/0h`.
    Taproot,
    /// BIP-49, `m/49h/0h/0h`.
    NestedSegwit,
    /// BIP-44, `m/44h/0h/0h`.
    Legacy,
    /// BIP-48, `m/48h/0h/0h/2h`: a cosigner's key.
    Multisig,
}

/// The rows of the Account card.
pub const ACCOUNTS: [Account; 5] = [
    Account::NativeSegwit,
    Account::Taproot,
    Account::NestedSegwit,
    Account::Legacy,
    Account::Multisig,
];

impl Account {
    /// The account's path below `m/`, at account 0 and coin type
    /// `coin` (0 on mainnet, 1 on the test networks).
    pub fn path(self, coin: u32) -> Text {
        let (purpose, tail) = match self {
            Account::NativeSegwit => (84, ""),
            Account::Taproot => (86, ""),
            Account::NestedSegwit => (49, ""),
            Account::Legacy => (44, ""),
            Account::Multisig => (48, "/2h"),
        };
        Text::formatted(format_args!("{purpose}h/{coin}h/0h{tail}"))
    }

    /// The row's name.
    pub fn name(self) -> &'static str {
        match self {
            Account::NativeSegwit => EN.explore_native_segwit,
            Account::Taproot => EN.explore_taproot,
            Account::NestedSegwit => EN.explore_nested_segwit,
            Account::Legacy => EN.explore_legacy,
            Account::Multisig => EN.catalog_multisig,
        }
    }

    /// The BIP the account is.
    pub fn bip(self) -> &'static str {
        match self {
            Account::NativeSegwit => "BIP-84",
            Account::Taproot => "BIP-86",
            Account::NestedSegwit => "BIP-49",
            Account::Legacy => "BIP-44",
            Account::Multisig => "BIP-48",
        }
    }

    /// How the account's addresses start on `network`, or what a
    /// multisig account gives.
    pub fn starts(self, network: Network) -> Text {
        let main = network == Network::Mainnet;
        let start = match (self, main) {
            (Account::NativeSegwit, true) => "bc1q",
            (Account::NativeSegwit, false) => "tb1q",
            (Account::Taproot, true) => "bc1p",
            (Account::Taproot, false) => "tb1p",
            (Account::NestedSegwit, true) => "3",
            (Account::NestedSegwit, false) => "2",
            (Account::Legacy, true) => "1",
            (Account::Legacy, false) => EN.seed_check_legacy_test_start,
            (Account::Multisig, _) => return Text::of(EN.seed_check_cosigner),
        };
        Text::fill(EN.seed_check_starts, &[&Text::of(start)])
    }
}

/// The coin type of `network`'s paths.
pub fn coin(network: Network) -> u32 {
    u32::from(network != Network::Mainnet)
}

/// Where the key the check reads is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// The seed the check holds alone.
    Held,
    /// The session's key at this place.
    Loaded(usize),
}

/// What the seed makes at one account: the Result card's rows, worked
/// out once. Public values.
#[derive(Clone)]
pub struct Made {
    /// The key's fingerprint, the passphrase applied.
    pub fingerprint: Fingerprint,
    /// The same words' fingerprint with no passphrase, where one was
    /// given.
    pub without: Option<Fingerprint>,
    /// The account's path, `m/84h/0h/0h`.
    pub path: Text,
    /// The account's script, in DESIGN.md §4.6's words.
    pub script: &'static str,
    /// The account's origin, `[73c5da0a/84h/0h/0h]`.
    pub origin: Text,
    /// The extended public key, `xpub` or `tpub`.
    pub plain: Text,
    /// SLIP-132's spelling of it where one names the script (zpub, ypub,
    /// Zpub, Ypub and their test twins).
    pub spelled: Option<Text>,
    /// Address 0 of the receive chain; none for a multisig account.
    pub receive: Option<Text>,
    /// Address 0 of the change chain; none for a multisig account.
    pub change: Option<Text>,
    /// The extended public key itself.
    pub xpub: Xpub,
    /// The account's path.
    pub derivation: DerivationPath,
    /// The script the account's addresses are of, where it has any.
    pub kind: ScriptType,
    /// Whether the account is a multisig cosigner's (BIP-48).
    pub cosigner: bool,
}

/// Why no result: the path is not one, or the key gave nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// The path as typed, with the reader's line.
    Path(&'static str),
    /// The key is gone or the path longer than any wallet's.
    Key,
}

/// The path text as the Account card takes it: without `m/`, `'` read as
/// `h`.
fn read_path(text: &str) -> Result<DerivationPath, &'static str> {
    let t = text.trim();
    let t = t
        .strip_prefix("m/")
        .unwrap_or(if t == "m" { "" } else { t });
    let t = Text::replaced(t, "'", "h");
    parse_path(&t).map_err(|e| e.message(&EN))
}

/// The script type a path and the chosen script give: the purpose's, or
/// the one asked for. A BIP-48 path is a multisig account, whose script
/// type is its last step's.
fn script_of(path: &DerivationPath, asked: ScriptType) -> (ScriptType, Option<slip132::Script>) {
    let multi = slip132::Script::of_path(&Text::formatted(format_args!("{path}"))).filter(|s| {
        matches!(
            s,
            slip132::Script::NativeMultisig | slip132::Script::NestedMultisig
        )
    });
    let single = implied_script(path).unwrap_or(asked);
    (single, multi)
}

/// Costs a key derivation, an extended public key and two address
/// derivations (none for a multisig). What the key at `source` makes at
/// `path` (below `m/`) on `network`, `asked` being the script for a path
/// whose purpose names none.
pub fn make(
    c: &Compute,
    keys: &Keys,
    source: Source,
    path: &str,
    asked: ScriptType,
    network: Network,
) -> Result<Made, Refused> {
    let path = read_path(path).map_err(Refused::Path)?;
    let (script, multi) = script_of(&path, asked);
    let (fingerprint, without, xpub) = match source {
        Source::Held => {
            let facts = keys.check_facts().ok_or(Refused::Key)?;
            let x = keys.check_xpub_at(c, &path).ok_or(Refused::Key)?;
            (facts.fingerprint, facts.without, x)
        }
        Source::Loaded(i) => {
            let info = keys.all().nth(i).ok_or(Refused::Key)?;
            let x = keys.xpub_at(c, i, &path).ok_or(Refused::Key)?;
            let without = (info.has_passphrase)
                .then(|| keys.fingerprint_with(c, i, ""))
                .flatten();
            (info.fingerprint, without, x)
        }
    };
    let plain = Text::formatted(format_args!("{xpub}"));
    let spelled_as = multi.unwrap_or(match script {
        ScriptType::NativeSegwit => slip132::Script::NativeSegwit,
        ScriptType::NestedSegwit => slip132::Script::NestedSegwit,
        ScriptType::Taproot => slip132::Script::Taproot,
        ScriptType::Legacy => slip132::Script::Legacy,
    });
    let spelled = slip132::encode(&plain, spelled_as).filter(|s| **s != *plain);
    let (receive, change) = if multi.is_some() {
        (None, None)
    } else {
        let policy = WalletPolicy::from_xpub(xpub, script).map_err(|_| Refused::Key)?;
        let at = |change| {
            policy
                .address_at(c, network, change, 0)
                .ok()
                .map(|a| Text::formatted(format_args!("{a}")))
        };
        (at(false), at(true))
    };
    let shown = Text::replaced(&Text::formatted(format_args!("{path}")), "'", "h");
    // The master key's own path is empty: `[fp]` and `m`.
    let (origin, shown_path) = if shown.as_str().is_empty() {
        (
            Text::formatted(format_args!("[{fingerprint}]")),
            Text::formatted(format_args!("m")),
        )
    } else {
        (
            Text::formatted(format_args!("[{fingerprint}/{}]", shown.as_str())),
            Text::formatted(format_args!("m/{}", shown.as_str())),
        )
    };
    Ok(Made {
        fingerprint,
        without,
        path: shown_path,
        script: match (multi, script) {
            (Some(slip132::Script::NestedMultisig), _) => EN.seed_check_multisig_nested,
            (Some(_), _) => EN.seed_check_multisig_native,
            (None, ScriptType::NativeSegwit) => EN.explore_native_segwit,
            (None, ScriptType::Taproot) => EN.explore_taproot,
            (None, ScriptType::NestedSegwit) => EN.explore_nested_segwit,
            (None, ScriptType::Legacy) => EN.explore_legacy,
        },
        origin,
        plain,
        spelled,
        receive,
        change,
        xpub,
        derivation: path,
        kind: script,
        cosigner: multi.is_some(),
    })
}

mod act;
mod area;
mod compare;
pub(crate) mod screen;

pub(crate) use act::{
    SEED_STEPS, act, compare_sample_text, drop_for_check, enter_is_slow, field, fill_demo_pass,
    form_done, here, key, not_a_seed, open_for_check, put_compare_sample, put_sample_words,
    scanned, seed_sample_open, stick_attached, take_part, tidy,
};
pub(crate) use area::SeedState;
pub use area::{SeedArea, step};
pub(crate) use compare::{
    Answer, CompareDone, CompareWork, SeedIs, compare_scanned, done as compare_done, held_in,
};

/// The forms of Add a key, other than words, that the Seed card takes, in
/// the order its rows list them: the one list, so that a form taken later
/// (the Krux code) is one more entry here and its `Form` variant.
pub const FORMS: [crate::forms::Form; 3] = [
    crate::forms::Form::Slip39,
    crate::forms::Form::Codex32,
    crate::forms::Form::Xor,
];

/// A press in the flow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeedAction {
    /// The Tools tile.
    Open,
    /// A card's header: open it, or take it again.
    Step(u8),
    /// A card opened as it was, by Back or Forward: nothing is taken
    /// again.
    Show(u8),
    /// Type the words.
    Type,
    /// The typed words are taken.
    Words,
    /// A form of Add a key's other than words, at this place of
    /// [`FORMS`]: its parts are typed on the Seed card.
    Form(u8),
    /// The parts typed are put together.
    FormDone,
    /// The key loaded at this place.
    Loaded(usize),
    /// No passphrase.
    NoPass,
    /// Add a passphrase: its two fields.
    AddPass,
    /// Typing goes to passphrase field 0 or 1.
    PassField(u8),
    /// Use the passphrase typed twice.
    UsePass,
    /// The account a row names.
    Pick(Account),
    /// Another account or path: the path field.
    Other,
    /// Typing goes to the path.
    PathField,
    /// The script a path names none of, by its place in
    /// `ScriptType::ALL`.
    Script(u8),
    /// Use the path typed.
    UsePath,
    /// The xpub's spelling: plain (true) or SLIP-132's.
    Spelling(bool),
    /// The code of the xpub (0) or the receive address (1).
    Qr(u8),
    /// Another account of the same key.
    Another,
    /// Another seed.
    AnotherSeed,
    /// Compare with the wallet app's: its field opens.
    Compare,
    /// Typing goes to the comparison's field.
    CompareField,
    /// Scan a value for the comparison.
    CompareScan,
    /// The comparison's value is the file at this place in Files.
    CompareFile(usize),
    /// Compare the value typed with the account picked.
    CompareGo,
    /// Practice's sample seed, taken as typed words.
    Sample,
    /// Practice's sample for the comparison's field.
    CompareSample,
    /// Keep this key.
    Keep,
    /// Done: the key is dropped.
    Done,
    /// Cancel: the key is dropped.
    Cancel,
}
