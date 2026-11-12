//! Creating a wallet (`docs/WALLETS.md` §5): which kind, the quorum, a key
//! for every slot, and the descriptor they make. The keys can be loaded
//! here, made here from fresh entropy, or arrive from a cosigner as an
//! account key in a file. Nothing secret leaves the session.

use crate::strings::EN;
use crate::strings::fill;
use faraday_bip::keys::{MultisigScriptType, Network, ScriptType};
use faraday_bip::policy::{PolicyKey, WalletPolicy};
use faraday_bip::slip132;
use faraday_crypto::work::Compute;

use crate::wallet::{KeyTexts, fp_text};
use crate::{
    Action, Code, Faraday, MAX_VAULTS, Screen, VaultAsk, backup_flow, create, cstep, flow,
    secret_text, wallet,
};
use faraday_shell_api::Key as KeyIn;

/// The kinds a wallet can be created as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NewKind {
    /// `wpkh`, BIP-84.
    #[default]
    NativeSegwit,
    /// `tr`, BIP-86.
    Taproot,
    /// `sh(wpkh)`, BIP-49.
    NestedSegwit,
    /// `pkh`, BIP-44.
    Legacy,
    /// `wsh(sortedmulti)`, BIP-48 type 2.
    Multi,
    /// `sh(wsh(sortedmulti))`, BIP-48 type 1.
    MultiNested,
    /// `sh(sortedmulti)`, at m/45'.
    MultiLegacy,
    /// `tr(NUMS, sortedmulti_a)`, at m/48'/0'/0'/3' (coin type 1 on a test network).
    TapMulti,
    /// MuSig2: every key signs one Taproot key together,
    /// `tr(musig(…)/<0;1>/*)` over the keys' Taproot accounts (BIP-327,
    /// 328, 390).
    MuSig,
}

/// How a single key made for the wallet is backed up: the Backup card's
/// answer (F240), which fixes what its slot's New key makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupForm {
    /// BIP-39 words.
    Words,
    /// Codex32 shares, k of n (`docs/CODEX32.md` §6.9).
    Codex32,
}

impl BackupForm {
    /// The forms, by their number in `Action::CBackupForm`, in the order
    /// the Backup card lists them.
    pub const ALL: [BackupForm; 2] = [BackupForm::Words, BackupForm::Codex32];

    /// The forms the Backup card offers: codex32 only where Make codex32
    /// shares is (`codex32::MAKE_OFFERED`).
    pub fn offered() -> Vec<BackupForm> {
        Self::ALL
            .into_iter()
            .filter(|f| *f == BackupForm::Words || crate::codex32::MAKE_OFFERED)
            .collect()
    }

    /// Its number in `Action::CBackupForm`.
    pub fn index(self) -> u8 {
        Self::ALL.iter().position(|f| *f == self).unwrap_or(0) as u8
    }

    /// The row's name on the Backup card.
    pub fn name(self) -> &'static str {
        match self {
            BackupForm::Words => EN.create_backup_words,
            BackupForm::Codex32 => EN.create_backup_codex32,
        }
    }

    /// The row's second line: a fact of the row.
    pub fn line(self) -> &'static str {
        match self {
            BackupForm::Words => EN.create_backup_words_line,
            BackupForm::Codex32 => EN.create_backup_codex32_line,
        }
    }
}

/// The number a kept flow wrote for Single key, backup in shares, a kind
/// since folded into Single key with a codex32 backup (F240): read as
/// Single key, native SegWit, its Backup card answered Codex32 shares.
pub const OLD_SHARES_CODE: u8 = 10;

/// The kind and the Backup card's answer a kept number names: a kind's
/// own ([`NewKind::from_code`]), or [`OLD_SHARES_CODE`].
pub fn from_kept_code(code: u8) -> Option<(NewKind, Option<BackupForm>)> {
    if code == OLD_SHARES_CODE {
        return Some((NewKind::NativeSegwit, Some(BackupForm::Codex32)));
    }
    NewKind::from_code(code).map(|k| (k, None))
}

/// The create cards a kind shows. Check is the last: its **Back up
/// wallet** opens Back up on the wallet made, with no card of its own
/// (owner, 2026-10-26). A single key asks how its key is backed up, on
/// Backup, where there is more than one form to choose (F240).
pub fn create_steps(kind: NewKind) -> Vec<u8> {
    let mut v = vec![cstep::KIND];
    if kind.multi() {
        v.push(cstep::QUORUM);
    } else if BackupForm::offered().len() > 1 {
        v.push(cstep::BACKUP_FORM);
    }
    v.extend([cstep::WHERE, cstep::KEYS, cstep::CHECK]);
    v
}

/// BIP-341's unspendable internal key.
const NUMS: &str = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";

impl NewKind {
    /// Every kind, by its number in `Action::CKind` and `Action::RKind`:
    /// a new kind is added at the end, never between. A backup flow kept
    /// across a lock writes a kind by [`NewKind::code`] instead. The Kind
    /// card's order is [`NewKind::CREATE_ROWS`].
    pub const ALL: [NewKind; 9] = [
        NewKind::NativeSegwit,
        NewKind::Taproot,
        NewKind::NestedSegwit,
        NewKind::Legacy,
        NewKind::Multi,
        NewKind::MultiNested,
        NewKind::MultiLegacy,
        NewKind::TapMulti,
        NewKind::MuSig,
    ];

    /// The kind's number in a backup flow kept across a lock, which never
    /// changes: 8 was a kind since removed (F127) and is read as none, 10
    /// a kind since folded into Single key ([`OLD_SHARES_CODE`]).
    pub fn code(self) -> u8 {
        match self {
            NewKind::MuSig => 9,
            k => Self::ALL.iter().position(|x| *x == k).unwrap_or(0) as u8,
        }
    }

    /// The kind a kept number names ([`NewKind::code`]).
    pub fn from_code(code: u8) -> Option<NewKind> {
        match code {
            0..=7 => Some(Self::ALL[usize::from(code)]),
            9 => Some(NewKind::MuSig),
            _ => None,
        }
    }

    /// Make a new wallet's Kind rows, by place in [`NewKind::ALL`], in the
    /// order shown: who signs, Single key then Multisig, then the address
    /// types and MuSig2 behind **More kinds** (F240).
    pub(crate) const CREATE_ROWS: [usize; 9] = [0, 4, 1, 2, 3, 5, 6, 7, 8];

    /// How many of [`NewKind::CREATE_ROWS`] show before **More kinds**.
    pub(crate) const CREATE_FIRST: usize = 2;

    /// Make a new wallet's Kind rows as shown, by place in
    /// [`NewKind::ALL`]: the first two, or with `more` every one.
    pub fn create_rows(more: bool) -> Vec<usize> {
        let shown = if more {
            &Self::CREATE_ROWS[..]
        } else {
            &Self::CREATE_ROWS[..Self::CREATE_FIRST]
        };
        shown.to_vec()
    }

    /// The kind's name on the card.
    pub fn name(self) -> &'static str {
        match self {
            NewKind::NativeSegwit => EN.create_single_key_native,
            NewKind::Taproot => EN.create_single_key_taproot,
            NewKind::NestedSegwit => EN.create_single_key_nested,
            NewKind::Legacy => EN.create_single_key_legacy,
            NewKind::Multi => EN.create_multisig_native,
            NewKind::MultiNested => EN.create_multisig_nested,
            NewKind::MultiLegacy => EN.create_multisig_legacy,
            NewKind::TapMulti => EN.create_multisig_taproot,
            NewKind::MuSig => EN.create_all_keys_musig2,
        }
    }

    /// One line on what it is for.
    pub fn line(self) -> &'static str {
        match self {
            NewKind::NativeSegwit => EN.create_bc1q_addresses,
            NewKind::Taproot => EN.create_bc1p_addresses,
            NewKind::NestedSegwit => EN.create_3_addresses,
            NewKind::Legacy => EN.create_1_addresses,
            NewKind::Multi => EN.create_several_keys,
            NewKind::MultiNested => EN.create_multisig_nested_line,
            NewKind::MultiLegacy => EN.create_multisig_legacy_line,
            NewKind::TapMulti => EN.create_multisig_taproot_line,
            NewKind::MuSig => EN.create_every_key_signs,
        }
    }

    /// The second line of the kind's row on Make a new wallet's Kind card:
    /// a fact of the row, with the counts the creation holds now, `m` of
    /// `n` for a multisig.
    pub fn create_line(self, (m, n): (usize, usize)) -> String {
        match self {
            NewKind::NativeSegwit => EN.create_single_key_line.to_string(),
            NewKind::Multi => fill(EN.create_multisig_line, &[&n.to_string(), &m.to_string()]),
            other => other.line().to_string(),
        }
    }

    /// The kind a wallet of this policy is, when it is one Create makes:
    /// what names its keys and signs their BIP 129 records.
    pub fn of(policy: &WalletPolicy) -> Option<NewKind> {
        use faraday_bip::policy::{Template, Wrapper};
        Some(match policy.template() {
            Template::Single { script } => match script {
                ScriptType::NativeSegwit => NewKind::NativeSegwit,
                ScriptType::Taproot => NewKind::Taproot,
                ScriptType::NestedSegwit => NewKind::NestedSegwit,
                ScriptType::Legacy => NewKind::Legacy,
            },
            Template::Multi { wrapper, .. } => match wrapper {
                Wrapper::Wsh => NewKind::Multi,
                Wrapper::ShWsh => NewKind::MultiNested,
                Wrapper::Sh => NewKind::MultiLegacy,
            },
            Template::Tree if policy.tapscript_quorum().is_some() => NewKind::TapMulti,
            Template::MuSig => NewKind::MuSig,
            _ => return None,
        })
    }

    /// Whether it has more than one key.
    pub fn multi(self) -> bool {
        matches!(
            self,
            NewKind::Multi
                | NewKind::MultiNested
                | NewKind::MultiLegacy
                | NewKind::TapMulti
                | NewKind::MuSig
        )
    }

    /// Whether every key must sign: MuSig2, n of n.
    pub(crate) fn all_sign(self) -> bool {
        self == NewKind::MuSig
    }

    /// The most keys a wallet of this kind has: fifteen keys for a
    /// multisig.
    pub(crate) fn max_keys(self) -> usize {
        15
    }

    /// The path a key of this kind is taken at, from its master, on
    /// `network`.
    pub fn path(self, network: faraday_bip::keys::Network) -> String {
        let c = network.coin_type();
        match self {
            NewKind::NativeSegwit => format!("m/84'/{c}'/0'"),
            NewKind::Taproot => format!("m/86'/{c}'/0'"),
            NewKind::NestedSegwit => format!("m/49'/{c}'/0'"),
            NewKind::Legacy => format!("m/44'/{c}'/0'"),
            NewKind::Multi => format!("m/48'/{c}'/0'/2'"),
            NewKind::MultiNested => format!("m/48'/{c}'/0'/1'"),
            NewKind::MultiLegacy => "m/45'".to_string(),
            NewKind::TapMulti => format!("m/48'/{c}'/0'/3'"),
            NewKind::MuSig => format!("m/86'/{c}'/0'"),
        }
    }

    /// The kind's path with account `account` in place of 0: the
    /// account is BIP-44's third step, BIP-48's too. `m/45'` has none
    /// and stays as it is.
    pub fn path_at(self, network: faraday_bip::keys::Network, account: u32) -> String {
        self.path_with(network, account, false)
    }

    /// Whether the kind can take BIP-87's path, `m/87'/coin'/account'`,
    /// in place of its own: the four multisig kinds Create makes with a
    /// descriptor, whose script the descriptor says (`docs/WALLETS.md`).
    pub fn has_bip87(self) -> bool {
        matches!(
            self,
            NewKind::Multi | NewKind::MultiNested | NewKind::MultiLegacy | NewKind::TapMulti
        )
    }

    /// [`NewKind::path_at`], or with `bip87` and a kind that
    /// [`NewKind::has_bip87`], BIP-87's `m/87'/coin'/account'`.
    pub fn path_with(
        self,
        network: faraday_bip::keys::Network,
        account: u32,
        bip87: bool,
    ) -> String {
        if bip87 && self.has_bip87() {
            return format!("m/87'/{}'/{account}'", network.coin_type());
        }
        let path = self.path(network);
        let mut steps: Vec<String> = path.split('/').map(str::to_string).collect();
        if steps.len() >= 4 {
            steps[3] = format!("{account}'");
        }
        steps.join("/")
    }

    /// Whether the kind's path has an account to choose: every kind but
    /// `m/45'`.
    pub(crate) fn has_account(self) -> bool {
        self != NewKind::MultiLegacy
    }

    /// The card that carries the kind's Account row (F98): Quorum, at its
    /// foot, for a kind that has one; else Keys, at its head. `None` for
    /// a kind with no account.
    pub(crate) fn account_card(self) -> Option<u8> {
        if !self.has_account() {
            return None;
        }
        Some(if crate::create_steps(self).contains(&cstep::QUORUM) {
            cstep::QUORUM
        } else {
            cstep::KEYS
        })
    }

    /// Which key text this kind is at `account` on `network`: its script's
    /// account key, or its path (`docs/history/NEW-WALLET.md` §16.2).
    pub fn spec(self, network: Network, account: u32) -> Result<faraday_keys::Spec, String> {
        use faraday_keys::Spec;
        Ok(match self {
            NewKind::NativeSegwit => Spec::Single(ScriptType::NativeSegwit, account),
            // A MuSig2 participant's key is its Taproot account.
            NewKind::Taproot | NewKind::MuSig => Spec::Single(ScriptType::Taproot, account),
            NewKind::NestedSegwit => Spec::Single(ScriptType::NestedSegwit, account),
            NewKind::Legacy => Spec::Single(ScriptType::Legacy, account),
            NewKind::Multi => Spec::Multisig(MultisigScriptType::NativeSegwit, account),
            NewKind::MultiNested => Spec::Multisig(MultisigScriptType::NestedSegwit, account),
            NewKind::MultiLegacy | NewKind::TapMulti => Spec::Path(
                self.path_at(network, account)
                    .parse()
                    .map_err(|e| format!("{e}"))?,
            ),
        })
    }

    /// Costs a key derivation. A `wsh` multisig key of session key `i` as
    /// the `ur:crypto-account` SeedSigner shows for one; `None` for any
    /// other kind.
    pub(crate) fn account_ur(
        self,
        c: &Compute,
        keys: &faraday_keys::Keys,
        i: usize,
        bip87: bool,
    ) -> Option<String> {
        self.account_ur_at(c, keys, i, 0, bip87)
    }

    /// Costs a key derivation. [`NewKind::account_ur`] at account
    /// `account`, on BIP-87's path when `bip87`.
    pub(crate) fn account_ur_at(
        self,
        c: &Compute,
        keys: &faraday_keys::Keys,
        i: usize,
        account: u32,
        bip87: bool,
    ) -> Option<String> {
        if self != NewKind::Multi {
            return None;
        }
        let a = keys
            .multisig_account(c, i, MultisigScriptType::NativeSegwit, account, bip87)?
            .ok()?;
        let path: Vec<_> = a.path.into_iter().copied().collect();
        let cbor = faraday_qr::registry::account_cbor(a.fingerprint.0, &path, &a.xpub, false);
        Some(faraday_codec::ur::encode_single("crypto-account", &cbor))
    }

    /// Costs a key derivation and a signature. Session key `i`'s BIP 129
    /// key record, signed by the account key itself: what Coldcard,
    /// Sparrow, Nunchuk and Keystone take from a signer. For the multisig
    /// kinds BIP 129 covers (`wsh` and `sh(wsh)`), at account `account` of
    /// the kind's path (BIP-87's with `bip87`), described by the key's
    /// fingerprint text `fp`.
    pub fn bsms_record(
        self,
        c: &Compute,
        keys: &faraday_keys::Keys,
        i: usize,
        account: u32,
        bip87: bool,
        fp: &str,
    ) -> Option<String> {
        let script = match self {
            NewKind::Multi => MultisigScriptType::NativeSegwit,
            NewKind::MultiNested => MultisigScriptType::NestedSegwit,
            _ => return None,
        };
        keys.bsms_record(c, i, script, account, bip87, &format!("Faraday key {fp}"))
    }

    /// The descriptor over these keys.
    pub fn descriptor(self, m: usize, keys: &[String]) -> String {
        let list = keys
            .iter()
            .map(|k| format!("{k}/<0;1>/*"))
            .collect::<Vec<_>>()
            .join(",");
        match self {
            NewKind::NativeSegwit => format!("wpkh({list})"),
            NewKind::Taproot => format!("tr({list})"),
            NewKind::NestedSegwit => format!("sh(wpkh({list}))"),
            NewKind::Legacy => format!("pkh({list})"),
            NewKind::Multi => format!("wsh(sortedmulti({m},{list}))"),
            NewKind::MultiNested => format!("sh(wsh(sortedmulti({m},{list})))"),
            NewKind::MultiLegacy => format!("sh(sortedmulti({m},{list}))"),
            NewKind::TapMulti => format!("tr({NUMS},sortedmulti_a({m},{list}))"),
            // BIP-390: the wallet's derivation is on the aggregate, not on
            // each participant.
            NewKind::MuSig => format!("tr(musig({})/<0;1>/*)", keys.join(",")),
        }
    }
}

/// The account a wallet's keys are at: the third step of its first key's
/// path, BIP-44's and BIP-48's account; 0 for a key with no such step.
pub fn account_of(policy: &WalletPolicy) -> u32 {
    use faraday_bip::bitcoin::bip32::ChildNumber;
    policy
        .keys()
        .first()
        .and_then(|k| k.path())
        .and_then(|p| p.into_iter().nth(2).copied())
        .map_or(0, |c| match c {
            ChildNumber::Hardened { index } | ChildNumber::Normal { index } => index,
        })
}

/// Costs what `derive` and `to_xpub` cost. The key text at `path`, written
/// `m/48'/0'/0'/2'` or `48h/0h/0h/2h`, as a [`faraday_keys::Spec`]: what
/// [`NewKind::spec`] gives at a kind's own path, at any other.
pub fn path_spec(path: &str) -> Result<faraday_keys::Spec, String> {
    let path = normal_path(path).ok_or_else(|| fill(EN.create_not_a_derivation, &[path]))?;
    let parsed: faraday_bip::bitcoin::bip32::DerivationPath =
        path.parse().map_err(|e| format!("{e}"))?;
    Ok(faraday_keys::Spec::Path(parsed))
}

/// A derivation path as typed, `m/48'/0'/0'/2'`, `48h/0h/0h/2h` or with
/// spaces, as `m/48'/0'/0'/2'`; `None` when a step is not a number.
pub(crate) fn normal_path(text: &str) -> Option<String> {
    let t: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let t = t.trim_start_matches(['m', 'M']).trim_start_matches('/');
    let mut out = String::from("m");
    for step in t.split('/').filter(|s| !s.is_empty()) {
        let (num, hard) = match step.strip_suffix(['\'', 'h', 'H']) {
            Some(n) => (n, true),
            None => (step, false),
        };
        let n: u32 = num.parse().ok()?;
        if n >= 1 << 31 {
            return None;
        }
        out.push_str(&format!("/{n}{}", if hard { "'" } else { "" }));
    }
    Some(out)
}

/// Where a slot's key comes from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Source {
    /// Not chosen yet.
    #[default]
    Empty,
    /// A key loaded in this session, by fingerprint.
    Here([u8; 4]),
    /// A cosigner's account key, `[fingerprint/path]xpub`, from a file.
    Cosigner(String),
    /// Left for a cosigner, whose key file comes later.
    Later,
}

/// What a creation started from a loaded wallet keeps of that wallet
/// (`docs/history/NEW-WALLET.md` §16.2): its name, kind and account, and, by
/// slot, the cosigners' keys it has only as xpubs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FromWallet {
    /// The wallet's name.
    pub name: String,
    /// Its kind.
    pub kind: NewKind,
    /// The account its keys are at.
    pub account: u32,
    /// Whether its keys are at BIP-87's path (purpose 87), not the
    /// kind's own.
    pub bip87: bool,
    /// A cosigner's `[fingerprint/path]xpub` for each slot held only as
    /// an xpub; `None` for a key held here or in a vault.
    pub xpubs: Vec<Option<String>>,
}

impl FromWallet {
    /// What slot `slot` holds while the creation is a `kind` wallet at
    /// `account`, when the wallet's key there is a cosigner's xpub: that
    /// xpub at the wallet's own account, kind and path choice (`bip87`,
    /// BIP-87's or the kind's own), which it gives and no other, and only
    /// when the xpub's own origin is at that path; left for the cosigner
    /// otherwise. `None` for a slot whose key is held.
    pub(crate) fn xpub_slot(
        &self,
        slot: usize,
        kind: NewKind,
        account: u32,
        bip87: bool,
    ) -> Option<Source> {
        let text = self.xpubs.get(slot)?.as_ref()?;
        Some(
            if kind == self.kind
                && account == self.account
                && bip87 == self.bip87
                && at_kind(kind, text, bip87)
            {
                Source::Cosigner(text.clone())
            } else {
                Source::Later
            },
        )
    }

    /// The name proposed for the new wallet: the wallet's, or the kind's
    /// once the kind is changed, with the account.
    pub fn name_at(&self, kind: NewKind, account: u32) -> String {
        let base = if kind == self.kind {
            self.name.as_str()
        } else {
            kind.name()
        };
        fill(EN.create_at_account, &[base, &account.to_string()])
    }
}

/// The account key in a file, if the file is one: the first of
/// [`read_keys`].
pub fn read_key(text: &str) -> Option<String> {
    read_keys(text).into_iter().next()
}

/// Every account key a file offers, as `[fingerprint/path]xpub`: a BIP 129
/// key record whose signature is the key's; Coldcard's multisig key file
/// (`ccxp-….json`, which Passport and Unchained also write: flat `p2wsh`,
/// `p2sh_p2wsh` and `p2sh` fields, each with its `_deriv`); Coldcard's
/// generic export, one object per account; a single-key file (Cobo's,
/// Keystone's, Coldcard's for Wasabi); or `[fingerprint/path]xpub` on
/// lines of their own.
pub fn read_keys(text: &str) -> Vec<String> {
    if faraday_bip::bsms::is_signer_record(text) {
        return faraday_bip::bsms::SignerRecord::parse(text)
            .ok()
            .filter(|r| r.verify().is_ok())
            .map(|r| r.key.key_text())
            .into_iter()
            .collect();
    }
    if text.trim_start().starts_with('{') {
        // A single-key file (Cobo, Keystone, Coldcard's for Wasabi): its
        // one key.
        if let Ok(v) = faraday_bip::json::parse(text)
            && let Some(read) = faraday_intake::single_key::read(&v)
        {
            return read
                .ok()
                .and_then(|k| PolicyKey::parse(&k.key).ok())
                .map(|k| k.key_text())
                .into_iter()
                .collect();
        }
        // Coldcard's multisig key file or generic export.
        return faraday_bip::coldcard::cosigner_keys(text);
    }
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| match slip132::key_line(l) {
            Some(read) => PolicyKey::parse(&read.ok()?.text).ok(),
            None => PolicyKey::parse(l).ok(),
        })
        .map(|k| k.key_text())
        .collect()
}

/// The key in a file for a slot of a `kind` wallet ([`key_for_slot`]),
/// when there is one.
pub fn key_for(kind: NewKind, text: &str) -> Option<String> {
    key_for_slot(kind, false, text).ok()
}

/// One key or wallet file of the Inbox as Make a new wallet's Keys card
/// lists it: the key it offers a slot, or why it offers none.
#[derive(Clone, Debug)]
pub struct KeyFile {
    /// Its place in the Inbox.
    pub at: usize,
    /// Its name.
    pub name: String,
    /// [`key_for_slot`] for the kind being made.
    pub key: Result<String, String>,
}

impl Faraday {
    /// Costs [`key_for_slot`] for each key or wallet file of the Inbox.
    /// The files Make a new wallet's Keys card lists, offered or refused
    /// ([`crate::drawing::View::create_key_files`]); none for a kind whose
    /// keys are not files.
    pub(crate) fn create_key_files_now(&self, _c: &Compute) -> std::rc::Rc<Vec<KeyFile>> {
        let Some((kind, bip87)) = self.create.state().map(|c| (c.kind, c.at_bip87())) else {
            return std::rc::Rc::default();
        };
        std::rc::Rc::new(
            self.inbox
                .iter()
                .enumerate()
                .filter(|(_, it)| {
                    matches!(it.kind, wallet::FileKind::Key | wallet::FileKind::Wallet)
                })
                .map(|(at, it)| KeyFile {
                    at,
                    name: it.name.clone(),
                    key: key_for_slot(kind, bip87, &String::from_utf8_lossy(&it.bytes)),
                })
                .collect(),
        )
    }
}

/// Costs a read of the file's keys (a base58 decode each). The key in a
/// file for a slot of a `kind` wallet, or why there is none: of the keys the file offers, the first with its origin at the
/// kind's derivation (any account; with `bip87`, BIP-87's instead), its path's coin type that of its
/// xpub's network, and in a file of one key a SLIP-132 version, where it
/// has one, naming the kind's script type. A key with no origin, or at another path, is never taken
/// for the slot: its path would be a guess. None for the kinds whose
/// keys are dealt here.
pub fn key_for_slot(kind: NewKind, bip87: bool, text: &str) -> Result<String, String> {
    let none = || fill(EN.create_scan_no_key, &[kind.name()]);
    if path_shape(&kind.path(Network::Mainnet)).is_none() {
        return Err(none());
    }
    let keys = read_keys(text);
    if let Some(k) = keys
        .iter()
        .find(|k| at_kind(kind, k, bip87) && coin_agrees(k))
    {
        if keys.len() == 1 && !named_agrees(kind, text) {
            let path = origin_path(k).unwrap_or_default();
            return Err(fill(EN.create_scan_not_for, &[path, kind.name()]));
        }
        return Ok(k.clone());
    }
    match keys.as_slice() {
        [k] => Err(match origin_path(k) {
            None => EN.create_scan_no_origin.to_string(),
            Some(path) if !at_kind(kind, k, bip87) => {
                fill(EN.create_scan_not_for, &[path, kind.name()])
            }
            Some(_) => EN.create_scan_coin.to_string(),
        }),
        _ => Err(none()),
    }
}

/// Whether an account key `[fingerprint/path]xpub`'s path has the coin
/// type of its xpub's network: 0 for an `xpub`, 1 for a `tpub`. `m/45'`
/// has no coin type. A key with no origin has none to agree.
fn coin_agrees(key: &str) -> bool {
    let Some(path) = origin_path(key) else {
        return false;
    };
    if path_shape(path).as_deref() == Some("45") {
        return true;
    }
    let xpub = key
        .get(key.find(']').map_or(0, |i| i + 1)..)
        .unwrap_or_default();
    let coin = if xpub.starts_with("xpub") {
        "0"
    } else if xpub.starts_with("tpub") {
        "1"
    } else {
        return false;
    };
    path.split('/')
        .nth(1)
        .map(|s| s.trim_end_matches(['h', '\'', 'H']))
        == Some(coin)
}

/// Purpose, and for BIP-48 the script type: what a derivation path says
/// apart from its coin type and account.
fn path_shape(path: &str) -> Option<String> {
    let steps: Vec<String> = path
        .trim_start_matches('m')
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_end_matches(['h', '\'', 'H']).to_string())
        .collect();
    match steps.first().map(String::as_str) {
        Some("48") => steps.get(3).map(|t| format!("48/{t}")),
        Some(p) => Some(p.to_string()),
        None => None,
    }
}

/// Whether a wallet's own keys are at BIP-87's path (purpose 87).
pub fn bip87_of(policy: &WalletPolicy) -> bool {
    policy.keys().first().is_some_and(bip87_key)
}

/// Whether one key's own origin path is at BIP-87's purpose (87').
pub fn bip87_key(key: &faraday_bip::policy::PolicyKey) -> bool {
    use faraday_bip::bitcoin::bip32::ChildNumber;
    key.path().and_then(|p| p.into_iter().next().copied())
        == Some(ChildNumber::Hardened { index: 87 })
}

/// Costs a key derivation the first time. Whether wallet key `pk` is
/// session key `key` at Create's own account for `kind`, on BIP-48's path
/// or BIP-87's as `pk`'s own path is: what its BIP 129 record and its
/// account code are made at. A key elsewhere has neither.
pub fn at_own_account(
    c: &Compute,
    keys: &faraday_keys::Keys,
    key: usize,
    kind: NewKind,
    pk: &PolicyKey,
) -> bool {
    let text = if kind.has_bip87() && bip87_key(pk) {
        keys.text_at(c, key, &kind.path_with(keys.network(), 0, true))
    } else {
        keys.account_text(c, key, kind, 0)
    };
    text.ok()
        .and_then(|t| PolicyKey::parse(&t).ok())
        .is_some_and(|t| t.xpub() == pk.xpub() && t.path() == pk.path())
}

/// The path of an account key `[fingerprint/path]xpub`, as its origin
/// writes it; `None` for a key with no origin.
fn origin_path(key: &str) -> Option<&str> {
    key.get(key.find('/')? + 1..key.find(']')?)
}

/// Whether an account key `[fingerprint/path]xpub` is at a `kind`
/// wallet's derivation, any account and any network, or, when the
/// creation takes BIP-87's path (`bip87`, [`CreateState::at_bip87`]),
/// at BIP-87's and no other: its path leaves the script to the
/// descriptor, and every key of the wallet is at it. A key with no origin
/// is at none.
pub(crate) fn at_kind(kind: NewKind, key: &str, bip87: bool) -> bool {
    let want = path_shape(&kind.path(Network::Mainnet));
    let shape = origin_path(key).and_then(path_shape);
    want.is_some()
        && if bip87 && kind.has_bip87() {
            shape.as_deref() == Some("87")
        } else {
            shape == want
        }
}

/// The script type a `kind` wallet's keys are written under.
fn kind_script(kind: NewKind) -> Option<ScriptType> {
    match kind {
        NewKind::NativeSegwit | NewKind::Multi => Some(ScriptType::NativeSegwit),
        NewKind::NestedSegwit | NewKind::MultiNested => Some(ScriptType::NestedSegwit),
        NewKind::Legacy | NewKind::MultiLegacy => Some(ScriptType::Legacy),
        NewKind::Taproot | NewKind::TapMulti | NewKind::MuSig => Some(ScriptType::Taproot),
    }
}

/// The one key a wallet description offers for a slot of a `kind`
/// wallet, as `[fingerprint/path]xpub`, with the network its xpub is on
/// (`current` naming which test network a `tpub` is), or why it does
/// not: the description names exactly one key, with its fingerprint and
/// path, under the kind's script type at the kind's derivation (any
/// account), its path's coin type that of its network. A description of
/// several keys does not choose one. A lookup over the policy read when
/// its file came in.
pub(crate) fn key_of_wallet(
    kind: NewKind,
    bip87: bool,
    policy: &WalletPolicy,
    current: Network,
) -> Result<(String, Network), String> {
    let [key] = policy.keys() else {
        return Err(fill(
            EN.create_scan_keys,
            &[&policy.keys().len().to_string()],
        ));
    };
    let text = key.key_text();
    let (Some(_), Some(path)) = (key.fingerprint(), origin_path(&text)) else {
        return Err(EN.create_scan_no_origin.into());
    };
    if kind_script(kind) != Some(policy.script_type()) || !at_kind(kind, &text, bip87) {
        return Err(fill(EN.create_scan_not_for, &[path, kind.name()]));
    }
    let net = wallet::network_of_key(key, current);
    let coin = path
        .split('/')
        .nth(1)
        .map(|s| s.trim_end_matches(['h', '\'', 'H']));
    if path_shape(path).as_deref() != Some("45")
        && coin != Some(net.coin_type().to_string().as_str())
    {
        return Err(EN.create_scan_coin.into());
    }
    Ok((text, net))
}

/// Whether every key in a file whose version is SLIP-132's names the
/// script type of a `kind` wallet: a `zpub` fills a native SegWit slot
/// and no other. Costs a base58 decode per line.
fn named_agrees(kind: NewKind, text: &str) -> bool {
    use slip132::Script;
    let around = match kind {
        NewKind::NativeSegwit => Script::NativeSegwit,
        NewKind::NestedSegwit => Script::NestedSegwit,
        NewKind::Legacy => Script::Legacy,
        NewKind::Multi => Script::NativeMultisig,
        NewKind::MultiNested => Script::NestedMultisig,
        NewKind::MultiLegacy => Script::LegacyMultisig,
        NewKind::Taproot | NewKind::TapMulti | NewKind::MuSig => Script::Taproot,
    };
    text.lines()
        .filter_map(slip132::key_line)
        .filter_map(Result::ok)
        .all(|k| slip132::agrees(k.named, around))
}

/// Costs a parse of the description (through `parse_any`). Builds and
/// reads back the wallet a creation describes.
pub fn build(
    c: &Compute,
    kind: NewKind,
    m: usize,
    keys: &[String],
) -> Result<WalletPolicy, String> {
    WalletPolicy::parse_any(c, &kind.descriptor(m, keys))
        .map_err(|e| faraday_strings::bip::policy_error(&e))
}

/// A wallet's derivation paths as Make a new wallet's Check shows them
/// (`docs/history/NEW-WALLET.md` §17): the one path when every key has it, else
/// each key's fingerprint and path. A key that came with no origin
/// reads as such.
pub(crate) fn paths_text(policy: &WalletPolicy) -> String {
    let shown = |k: &PolicyKey| match k.path() {
        Some(p) => format!("m/{p}"),
        None => EN.create_path_not_given.to_string(),
    };
    let keys = policy.keys();
    let paths: Vec<String> = keys.iter().map(shown).collect();
    match paths.first() {
        None => String::new(),
        Some(first) if paths.iter().all(|p| p == first) => first.clone(),
        Some(_) => keys
            .iter()
            .zip(&paths)
            .map(|(k, p)| {
                let fp = k.fingerprint().or(Some(k.identifier_fingerprint()));
                format!("{} {p}", crate::wallet_backup::fp_of(fp))
            })
            .collect::<Vec<_>>()
            .join(" · "),
    }
}

/// What Make a new wallet keeps of the codex32 set a single key was made
/// as (`docs/CODEX32.md` §6.9, §9.4): plain facts, no string. Check shows
/// it, and Back up is handed it for the wallet made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharesSet {
    /// The key the set rebuilds.
    pub fingerprint: [u8; 4],
    /// The set's four-character identifier, lower case.
    pub identifier: String,
    /// How many strings rebuild the key.
    pub k: u8,
    /// How many strings were made.
    pub n: u8,
    /// Characters per string, `ms1` included: 48 or 74.
    pub length: usize,
}

impl SharesSet {
    /// The facts of the set `make` made, for the key `fingerprint`.
    pub fn of(fingerprint: [u8; 4], make: &crate::codex32::Codex32Make) -> SharesSet {
        let (k, n) = make.k_n();
        SharesSet {
            fingerprint,
            identifier: make.identifier(),
            k,
            n,
            length: make.length().chars(),
        }
    }

    /// Check's row: "codex32 w0za · 2 of 3".
    pub fn line(&self) -> String {
        fill(
            EN.create_shares_set,
            &[&self.identifier, &self.k.to_string(), &self.n.to_string()],
        )
    }
}

/// Creating a wallet: the cards' state.
#[derive(Default)]
pub struct CreateState {
    /// The open card.
    pub open: Option<u8>,
    /// Open from Home: Back goes to Home, not Wallets.
    pub from_home: bool,
    /// Cards closed as done.
    pub done: [bool; cstep::COUNT],
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// The kind.
    pub kind: NewKind,
    /// No kind is ticked yet, `kind` only standing in: Make a new wallet
    /// opened by Move money, where the person chooses (owner,
    /// 2026-10-24; `docs/CODEX32.md` §6.9). A row chosen clears it; Next
    /// does nothing until then.
    pub kind_unchosen: bool,
    /// Signatures needed.
    pub m: usize,
    /// Keys.
    pub n: usize,
    /// A single key's Backup card: how its key is backed up, `None` until
    /// a row is pressed (F240). What its slot's New key makes.
    pub backup: Option<BackupForm>,
    /// The codex32 set a single key's slot was made as, from Make codex32
    /// shares' Lock in; Check's Backup row, and what Back up is handed.
    pub shares_set: Option<SharesSet>,
    /// Where each key comes from.
    pub slots: Vec<Source>,
    /// The wallet, once made, by index into the session.
    pub built: Option<usize>,
    /// The last refusal.
    pub error: Option<String>,
    /// The Kind card's **More kinds** is expanded: the other eight kinds
    /// are shown too.
    pub more_kinds: bool,
    /// New keys made for this wallet.
    pub made: u32,
    /// The slot whose passphrase fields are open (`docs/history/NEW-WALLET.md`
    /// §3.6).
    pub pass_slot: Option<u8>,
    /// The passphrase typed there.
    pub pass: secret_text::SecretText,
    /// The same passphrase typed again.
    pub pass2: secret_text::SecretText,
    /// Which of the two fields typing goes to, if either.
    pub pass_focus: Option<u8>,
    /// Why the last Lock in did nothing.
    pub pass_note: Option<String>,
    /// Keys locked in with a passphrase here, by fingerprint: their slot
    /// shows **Locked in**.
    pub pass_locked: Vec<[u8; 4]>,
    /// Whether vaults are part of the backup and how many; with yes, the
    /// sticks and the vaults come before the keys.
    pub vaults: VaultAsk,
    /// The number the wallet's backup flow is tied to (`Faraday::owner_seq`).
    pub owner: u64,
    /// The account every key is used at (`docs/history/NEW-WALLET.md` §16.2): 0
    /// unless the creation was started from a wallet.
    pub account: u32,
    /// The Account row's − n + stepper is open, by its **Change** or by
    /// Check's Path **Change** (F98); closed, the row shows the account.
    pub account_open: bool,
    /// The multisig path row's two-segment choice is open, by its
    /// **Change**; closed, the row shows the path chosen. False whenever
    /// the kind changes.
    pub path_open: bool,
    /// A multisig kind's keys are taken at BIP-87's `m/87'/coin'/account'`,
    /// not at the kind's own path (BIP-48's, or `m/45'`): the choice on the
    /// card that carries the Account row. False at entry and whenever the
    /// kind changes; only a kind that [`NewKind::has_bip87`] reads it.
    pub bip87: bool,
    /// The wallet the creation was started from, by **New wallet from
    /// this one**.
    pub from: Option<FromWallet>,
    /// The made wallet's derivation paths as Check shows them, worked out
    /// once when it is made (`docs/history/NEW-WALLET.md` §17).
    pub path: String,
    /// The made wallet was added to the session by this creation, not
    /// found there already: taking it back removes it.
    pub added: bool,
    /// The made wallet's first key recorded as a BIP-85 child seed, by
    /// fingerprint: what **A BIP-85 child seed** names (F166). Set once,
    /// when the wallet is made.
    pub child: Option<[u8; 4]>,
    /// The creation's backup flow as it stood before the made wallet was
    /// tied to it, by index: what taking the wallet back puts back.
    pub before_made: Option<(usize, Box<backup_flow::BackupFlow>)>,
}

impl CreateState {
    /// Whether the keys are at BIP-87's path ([`CreateState::bip87`]).
    pub fn at_bip87(&self) -> bool {
        self.bip87 && self.kind.has_bip87()
    }

    /// Whether the path has an account to choose: the kind's, or any
    /// BIP-87 path.
    pub fn has_account(&self) -> bool {
        self.kind.has_account() || self.at_bip87()
    }

    /// The card that carries the Account row ([`NewKind::account_card`]),
    /// `m/45'` included once BIP-87 is chosen.
    pub fn account_card(&self) -> Option<u8> {
        if self.at_bip87() && !self.kind.has_account() {
            return Some(cstep::QUORUM);
        }
        self.kind.account_card()
    }

    /// The person answered yes to using vaults.
    pub fn into_vaults(&self) -> bool {
        self.vaults.answer == Some(true)
    }

    /// Closes a slot's passphrase fields, forgetting what was typed.
    pub(crate) fn close_pass(&mut self) {
        self.pass_slot = None;
        self.pass.clear();
        self.pass2.clear();
        self.pass_focus = None;
        self.pass_note = None;
    }
}

/// What a made wallet taken back leaves for the caller to undo.
pub(crate) struct TakenBack {
    /// The wallet that was made.
    pub(crate) wallet: usize,
    /// Whether the creation added it to the session.
    pub(crate) added: bool,
    /// The flow that was open before it was made, to put back.
    pub(crate) before: Option<(usize, Box<backup_flow::BackupFlow>)>,
}

/// Create's area: the wallet being made, while the flow is open.
#[derive(Default)]
pub(crate) struct CreateArea {
    state: Option<CreateState>,
}

impl CreateArea {
    /// The flow's state, while it is open.
    pub(crate) fn state(&self) -> Option<&CreateState> {
        self.state.as_ref()
    }

    /// Opens card `open` (none closes them all) and has the column follow
    /// it.
    pub(crate) fn open_card(&mut self, open: Option<u8>) {
        if let Some(c) = self.state.as_mut() {
            c.open = open;
            c.scroll.follow = true;
        }
    }

    /// As [`CreateArea::open_card`], where the wallet is not yet made.
    pub(crate) fn open_card_unbuilt(&mut self, open: Option<u8>) {
        if self.state.as_ref().is_some_and(|c| c.built.is_none()) {
            self.open_card(open);
        }
    }

    /// Sets the column's scroll, as the drawing pass noted it.
    pub(crate) fn set_scroll(&mut self, to: flow::Scroll) {
        if let Some(c) = self.state.as_mut() {
            c.scroll = to;
        }
    }

    /// How far the column is scrolled, units, while the flow is open.
    pub(crate) fn scroll_y(&self) -> Option<f32> {
        self.state.as_ref().map(|c| c.scroll.y)
    }

    /// The column scrolled to `y` units, while the flow is open.
    pub(crate) fn set_scroll_y(&mut self, y: f32) {
        if let Some(c) = self.state.as_mut() {
            c.scroll.y = y;
        }
    }

    /// Puts `source` in slot `slot`, for the tests' kit.
    #[cfg(feature = "testkit")]
    pub(crate) fn set_slot(&mut self, slot: usize, source: Source) {
        if let Some(c) = self.state.as_mut() {
            c.slots[slot] = source;
        }
    }

    /// Notes the child of the keys' seeds the creation's notice is about.
    pub(crate) fn set_child(&mut self, child: Option<[u8; 4]>) {
        if let Some(c) = self.state.as_mut() {
            c.child = child;
        }
    }

    /// Lets go of the passphrase fields' focus.
    pub(crate) fn clear_pass_focus(&mut self) {
        if let Some(c) = self.state.as_mut() {
            c.pass_focus = None;
        }
    }

    /// Fills passphrase field `i` with `text` as the demo does: the first
    /// field fills its repeat too, where that is empty.
    pub(crate) fn fill_pass(&mut self, i: u8, text: &str) {
        if let Some(c) = self.state.as_mut() {
            let both: &[u8] = if i == 0 { &[0, 1] } else { &[1] };
            for &j in both {
                let t = if j == 0 { &mut c.pass } else { &mut c.pass2 };
                if j == i || t.is_empty() {
                    t.set(text);
                }
            }
            c.pass_focus = Some(i);
            c.pass_note = None;
        }
    }

    /// Notes that the Where card is done with no vault set up.
    pub(crate) fn where_done_no_vault(&mut self) {
        if let Some(c) = self.state.as_mut() {
            c.done[cstep::WHERE as usize] = true;
            c.vaults.answer = Some(false);
        }
    }

    /// Puts the vault's key `fp` in slot `slot`, where the slot is there;
    /// its passphrase fields close if they were on that slot.
    pub(crate) fn fill_slot_from_vault(&mut self, slot: u8, fp: [u8; 4]) {
        if let Some(c) = self.state.as_mut()
            && let Some(s) = c.slots.get_mut(usize::from(slot))
        {
            *s = Source::Here(fp);
            if c.pass_slot == Some(slot) {
                c.close_pass();
            }
        }
    }

    /// Puts key `fp` in `slot`, counting it as made here when `made`.
    pub(crate) fn fill_slot(&mut self, slot: u8, fp: [u8; 4], made: bool) {
        if let Some(c) = self.state.as_mut() {
            if made {
                c.made += 1;
            }
            if let Some(s) = c.slots.get_mut(usize::from(slot)) {
                *s = Source::Here(fp);
            }
        }
    }

    /// Whether a single-key creation is still to be made.
    pub(crate) fn single_key_unbuilt(&self) -> bool {
        self.state
            .as_ref()
            .is_some_and(|c| !c.kind.multi() && c.built.is_none())
    }

    /// Marks Keys and Check done and opens Check, once the wallet is made.
    pub(crate) fn check_opened_after_make(&mut self) {
        if let Some(c) = self.state.as_mut()
            && c.built.is_some()
        {
            c.done[usize::from(cstep::KEYS)] = true;
            c.done[usize::from(cstep::CHECK)] = true;
            c.open = Some(cstep::CHECK);
            c.scroll.follow = true;
        }
    }

    /// Fills the slots a wallet this was started from can fill, at the
    /// kind and account now chosen, where the wallet is not made: an empty
    /// or deferred slot, or a cosigner's still the wallet's own xpub.
    pub(crate) fn refresh_from(&mut self) {
        let Some(c) = self.state.as_mut() else {
            return;
        };
        if c.built.is_some() {
            return;
        }
        let Some(f) = c.from.as_ref() else {
            return;
        };
        let (kind, account, bip87) = (c.kind, c.account, c.at_bip87());
        for (i, s) in c.slots.iter_mut().enumerate() {
            let Some(want) = f.xpub_slot(i, kind, account, bip87) else {
                continue;
            };
            let auto = match &*s {
                Source::Empty | Source::Later => true,
                Source::Cosigner(t) => f.xpubs.get(i).and_then(|x| x.as_ref()) == Some(t),
                Source::Here(_) => false,
            };
            if auto {
                *s = want;
            }
        }
    }

    /// The Account row's step by `d`, where the wallet is not made and
    /// the kind has an account.
    pub(crate) fn step_account(&mut self, d: i8) {
        if let Some(c) = self.state.as_mut()
            && c.built.is_none()
            && c.has_account()
        {
            c.account_open = true;
            c.account =
                (i64::from(c.account) + i64::from(d)).clamp(0, i64::from(u32::MAX >> 1)) as u32;
        }
    }

    /// Opens the multisig path row's choice, where it applies.
    pub(crate) fn open_path_choice(&mut self) {
        if let Some(c) = self.state.as_mut()
            && c.built.is_none()
            && c.kind.has_bip87()
        {
            c.path_open = true;
        }
    }

    /// Chooses the multisig path (`bip87` or the kind's own); a cosigner's
    /// key at the other path no longer fits and its slot is emptied.
    pub(crate) fn choose_path(&mut self, bip87: bool) {
        if let Some(c) = self.state.as_mut()
            && c.built.is_none()
            && c.kind.has_bip87()
            && c.bip87 != bip87
        {
            c.bip87 = bip87;
            let (kind, at) = (c.kind, c.at_bip87());
            for s in &mut c.slots {
                if matches!(s, Source::Cosigner(t) if !at_kind(kind, t, at)) {
                    *s = Source::Empty;
                }
            }
        }
    }

    /// Takes the made wallet back: Keys and Check are no longer done, the
    /// card that carries the Account row opens with its stepper open, and
    /// the made wallet's marks go. Returns what the caller undoes.
    pub(crate) fn take_back(&mut self) -> Option<TakenBack> {
        let c = self.state.as_mut()?;
        let wallet = c.built.take()?;
        c.done[cstep::KEYS as usize] = false;
        c.done[cstep::CHECK as usize] = false;
        c.open = Some(c.account_card().unwrap_or(cstep::KIND));
        c.account_open = true;
        c.scroll.follow = true;
        c.path.clear();
        c.error = None;
        let added = std::mem::take(&mut c.added);
        let before = c.before_made.take();
        Some(TakenBack {
            wallet,
            added,
            before,
        })
    }

    /// Leaves the new creation's Kind with no row ticked, as Move money
    /// opens it (owner, 2026-10-24): only while nothing past Kind is done.
    pub(crate) fn leave_kind_unchosen(&mut self) {
        if let Some(c) = self.state.as_mut().filter(|c| {
            c.built.is_none()
                && !c.done[cstep::KIND as usize]
                && c.slots.iter().all(|s| *s == Source::Empty)
        }) {
            c.kind_unchosen = true;
            c.open = Some(cstep::KIND);
        }
    }

    /// Starts the flow on `state`.
    pub(crate) fn set(&mut self, state: CreateState) {
        self.state = Some(state);
    }

    /// Forgets the flow.
    pub(crate) fn clear(&mut self) {
        self.state = None;
    }

    /// A press that changes only the flow's state: a card, the kind, the
    /// quorum, a slot, a passphrase field.
    pub(crate) fn edit(&mut self, action: Action) {
        match action {
            Action::CStep(k) => {
                if let Some(c) = self.state.as_mut() {
                    c.open = if c.open == Some(k) { None } else { Some(k) };
                    c.scroll.follow = true;
                }
            }
            Action::CKind(i) => {
                if let Some(c) = self.state.as_mut()
                    && c.built.is_none()
                {
                    // Ticks the kind, whether chosen by hand on an open
                    // Kind card or by a Tools tile opening Create with
                    // its kind already ticked; `create_act` then moves on
                    // (a single choice moves on by itself).
                    c.kind = create::NewKind::ALL[i as usize % create::NewKind::ALL.len()];
                    c.kind_unchosen = false;
                    c.bip87 = false;
                    c.path_open = false;
                    if c.kind.multi() {
                        c.n = c.n.clamp(2, c.kind.max_keys());
                        c.m = if c.kind.all_sign() { c.n } else { c.m.min(c.n) };
                    }
                    let n = if c.kind.multi() { c.n } else { 1 };
                    c.slots.resize(n, create::Source::Empty);
                    c.slots.truncate(n);
                    // A cosigner's key at the old kind's path no longer
                    // fits the new one: its slot is emptied, as when the
                    // path choice flips.
                    let kind = c.kind;
                    for s in &mut c.slots {
                        if matches!(s, create::Source::Cosigner(t) if !create::at_kind(kind, t, false))
                        {
                            *s = create::Source::Empty;
                        }
                    }
                }
            }
            Action::CMoreKinds => {
                if let Some(c) = self.state.as_mut() {
                    c.more_kinds = true;
                }
            }
            // The Backup card's row: how the single key is backed up.
            Action::CBackupForm(i) => {
                if let Some(c) = self.state.as_mut()
                    && c.built.is_none()
                    && !c.kind.multi()
                    && let Some(&f) = BackupForm::ALL.get(usize::from(i))
                {
                    c.backup = Some(f);
                }
            }
            Action::CM(d) => {
                if let Some(c) = self.state.as_mut() {
                    c.m = if c.kind.all_sign() {
                        c.n
                    } else {
                        (c.m as i64 + i64::from(d)).clamp(1, c.n as i64) as usize
                    };
                }
            }
            Action::CN(d) => {
                if let Some(c) = self.state.as_mut()
                    && c.kind.multi()
                {
                    c.n = (c.n as i64 + i64::from(d)).clamp(2, c.kind.max_keys() as i64) as usize;
                    c.m = if c.kind.all_sign() { c.n } else { c.m.min(c.n) };
                    c.slots.resize(c.n, create::Source::Empty);
                }
            }
            _ => self.edit_slot(action),
        }
    }

    /// [`CreateArea::edit`] for the slots and the passphrase fields.
    fn edit_slot(&mut self, action: Action) {
        match action {
            Action::CSlotHere(k, fp) => {
                if let Some(c) = self.state.as_mut()
                    && let Some(s) = c.slots.get_mut(k as usize)
                {
                    *s = create::Source::Here(fp);
                    if c.pass_slot == Some(k) {
                        c.close_pass();
                    }
                }
            }
            Action::CSlotLater(k) => {
                if let Some(c) = self.state.as_mut()
                    && let Some(s) = c.slots.get_mut(k as usize)
                {
                    *s = create::Source::Later;
                }
            }
            Action::CSlotClear(k) => {
                if let Some(c) = self.state.as_mut()
                    && let Some(s) = c.slots.get_mut(k as usize)
                {
                    *s = create::Source::Empty;
                    if c.pass_slot == Some(k) {
                        c.close_pass();
                    }
                }
            }
            Action::CPassOpen(k) => {
                if let Some(c) = self.state.as_mut() {
                    if c.pass_slot == Some(k) {
                        c.close_pass();
                    } else if c.built.is_none() {
                        c.close_pass();
                        c.pass_slot = Some(k);
                        c.pass_focus = Some(0);
                    }
                }
            }
            Action::CPassField(f) => {
                if let Some(c) = self.state.as_mut()
                    && c.pass_slot.is_some()
                    && f < 2
                {
                    c.pass_focus = Some(f);
                    c.pass_note = None;
                }
            }
            _ => {}
        }
    }
}

impl Faraday {
    /// The Create flow's state, while it is open.
    pub fn create(&self) -> Option<&CreateState> {
        self.create.state()
    }

    /// Costs what the actions below cost, the keys' and wallets' work among
    /// it: handles the Create wallet flow's actions (`Action::C*`, `CreateWallet`, `CreateOver`, `PlanAddPass`, `NewWalletFrom`).
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub(crate) fn create_act(&mut self, action: Action, c: &Compute) {
        self.create_past_kind(action);
        match action {
            Action::CreateWallet
                if self.create_unfinished()
                    || self.create.state.as_ref().is_some_and(|c| {
                        self.setup_pending(c.owner, c.into_vaults(), c.built.is_some())
                    }) =>
            {
                // Resumed, its own way back stays where it was first
                // opened from: the trail lands where the person was when
                // they resumed it (F77), and a Home reached on the way, as
                // a setup's Leave lands it, does not make Back go Home.
                self.screen = Screen::Create;
            }
            Action::CreateWallet | Action::CreateOver => {
                // Create opens on Kind, open, with no row ticked until one
                // is pressed (owner, 2026-11-09; `docs/CODEX32.md` §6.9).
                // Back goes where it was opened from, Home or Wallets.
                let done = [false; cstep::COUNT];
                let owner = self.fresh_owner();
                self.create.state = Some(CreateState {
                    owner,
                    done,
                    open: Some(cstep::KIND),
                    kind: NewKind::NativeSegwit,
                    kind_unchosen: true,
                    // Start over keeps it.
                    from_home: self.screen == Screen::Home
                        || (self.screen == Screen::Create
                            && self.create.state.as_ref().is_some_and(|c| c.from_home)),
                    m: 2,
                    n: 3,
                    slots: vec![create::Source::Empty; 1],
                    ..CreateState::default()
                });
                self.screen = Screen::Create;
            }
            // Keys stay closed while the vaults' setup is to do.
            Action::CStep(k)
                if k == cstep::KEYS
                    && self.create.state.as_ref().is_some_and(|c| {
                        self.setup_pending(c.owner, c.into_vaults(), c.built.is_some())
                    }) => {}
            // Keys' Continue makes the wallet, which opens Check; until
            // it is made, a refusal keeps Keys open with the reason.
            Action::CNext(k)
                if k == cstep::KEYS
                    && self
                        .create
                        .state
                        .as_ref()
                        .is_some_and(|c| c.built.is_none()) =>
            {
                self.create_make();
            }
            // Kind with no row ticked waits for one (Move money).
            Action::CNext(k)
                if k == cstep::KIND && self.create.state().is_some_and(|c| c.kind_unchosen) => {}
            // The Backup card with no row ticked waits for one.
            Action::CNext(k)
                if k == cstep::BACKUP_FORM
                    && self.create.state().is_some_and(|c| c.backup.is_none()) => {}
            // Keys stays closed until a single key's backup is chosen: its
            // slot's New key makes the key in that form (F240).
            Action::CStep(k) if k == cstep::KEYS && self.create_backup_unanswered() => {}
            // The info button after "codex32" shows its tooltip at once
            // (F112, F240).
            Action::CBackupAbout => self.pointer.tip_show(Action::CBackupAbout),
            // The question of vaults waits for its answer.
            Action::CNext(k)
                if k == cstep::WHERE
                    && self
                        .create
                        .state()
                        .is_some_and(|c| c.vaults.answer.is_none()) => {}
            // How many vaults: Continue opens the setup, which brings in
            // each vault's stick and opens or makes the vault; it comes
            // back to Keys.
            Action::CNext(k)
                if k == cstep::HOW_MANY
                    && self
                        .create
                        .state()
                        .is_some_and(|c| self.flows.layout_blocked(c.owner)) => {}
            Action::CNext(k) if k == cstep::HOW_MANY => {
                if let Some(c) = self.create.state.as_mut() {
                    c.done[k as usize] = true;
                    let owner = c.owner;
                    self.wallet_flow_begin(owner);
                }
            }
            Action::CNext(k) => {
                if let Some(c) = self.create.state.as_mut() {
                    c.done[k as usize] = true;
                }
                // It matches keeps Check open: its foot is Back up wallet
                // now (owner, 2026-10-26).
                if k == cstep::CHECK {
                    if let Some(c) = self.create.state.as_mut() {
                        c.open = Some(k);
                    }
                } else {
                    self.create_open_next();
                }
                // Check's own Continue: this wallet is checked this
                // power-on (`docs/WALLETS.md` §4).
                if k == cstep::CHECK
                    && let Some(w) = self.create.state.as_ref().and_then(|c| c.built)
                    && let Some(wallet) = self.session.wallets.get(w)
                {
                    let checksum = wallet.checksum().to_string();
                    self.mark_checked(&checksum);
                }
            }
            Action::CBackup => {
                if let Some(i) = self.create.state.as_ref().and_then(|c| c.built) {
                    self.act(Action::Backup(i));
                    self.backup.set_from_create();
                }
            }
            Action::CVaults(_) | Action::CVaultN(_) | Action::CSticks(_)
                if self
                    .create
                    .state()
                    .is_some_and(|c| c.built.is_none() && self.setup_vault_open(c.owner)) =>
            {
                self.toast(EN.create_vaults_locked);
            }
            // Yes or No moves on by itself, and so does the answer already
            // given, pressed again: Yes to How many, No to Keys.
            Action::CVaults(yes) => {
                let Some(c) = self.create.state.as_mut().filter(|c| c.built.is_none()) else {
                    return;
                };
                if c.vaults.answer != Some(yes) {
                    let owner = c.owner;
                    c.vaults.answer = Some(yes);
                    c.done[cstep::HOW_MANY as usize] = false;
                    self.wallet_vaults_changed(owner);
                }
                if let Some(c) = self.create.state.as_mut() {
                    c.done[cstep::WHERE as usize] = true;
                }
                self.create_open_next();
            }
            Action::CVaultN(n) => {
                if let Some(c) = self.create.state.as_mut()
                    && c.built.is_none()
                {
                    let owner = c.owner;
                    c.vaults.count = usize::from(n).clamp(1, MAX_VAULTS);
                    c.done[cstep::HOW_MANY as usize] = false;
                    self.wallet_vaults_changed(owner);
                }
            }
            Action::CSticks(n) => {
                if let Some(c) = self.create.state.as_mut()
                    && c.built.is_none()
                {
                    let owner = c.owner;
                    c.done[cstep::HOW_MANY as usize] = false;
                    self.flows.layout_sticks(owner, usize::from(n));
                    self.wallet_vaults_changed(owner);
                }
            }
            Action::PlanAddPass => self.plan_add_passphrase(),
            // NEW-WALLET §16 (F45): keys from a vault in Create; a new
            // wallet from one held.
            Action::CSlotVault(k, v, r) => self.create_slot_vault(k, v, r),
            Action::CAccount(d) => self.create_account_step(d),
            Action::CMultiPath(bip87) => self.create_multi_path(bip87),
            Action::CMultiPathChange => self.create_multi_path_change(),
            Action::CPathChange => self.create_take_back(),
            Action::NewWalletFrom(i) => self.create_from_wallet(i),
            Action::CSlotFile(k, i) => {
                let Some(text) = self.inbox.get(i).map(|it| {
                    zeroize::Zeroizing::new(String::from_utf8_lossy(&it.bytes).into_owned())
                }) else {
                    return;
                };
                if let Err(e) = self.create_slot_file_key(k, &text)
                    && let Some(c) = self.create.state.as_mut()
                {
                    c.error = Some(e);
                }
            }
            Action::CKeyOut(k) | Action::CKeyQr(k) => {
                let qr = matches!(action, Action::CKeyQr(_));
                match self.create_key_text(c, k) {
                    Ok((fp, text)) => {
                        let kind = self.create.state.as_ref().map_or("", |c| c.kind.name());
                        if qr {
                            let view = self.code_view(c, Code::Key(k));
                            self.open_qr(view);
                        } else {
                            let name = format!("xpub-{fp}.txt");
                            let name = self.put_outbox_beside(
                                &name,
                                format!("# Account xpub {fp}, {kind}\n{text}\n").into_bytes(),
                            );
                            self.toast_out(&name);
                        }
                    }
                    Err(e) => self.toast(&e),
                }
            }
            Action::CKeyBsms(k) => match self.create_key_bsms(c, k) {
                Some((fpt, text)) => {
                    let name = format!("xpub-{fpt}-bsms.txt");
                    let name = self.put_outbox_beside(&name, text.into_bytes());
                    self.toast_out(&name);
                }
                None => self.toast(EN.act_bip_129_covers),
            },
            // A kind moves on by itself, and so does the kind already
            // ticked, pressed again: that is how the default is taken.
            Action::CKind(_) => {
                self.create.edit(action);
                self.create_from_refresh();
                if let Some(c) = self.create.state.as_mut().filter(|c| c.built.is_none()) {
                    c.done[cstep::KIND as usize] = true;
                    self.create_open_next();
                }
            }
            // A backup form moves on by itself, as a kind does (F103); one
            // not offered is not taken.
            Action::CBackupForm(i)
                if !BackupForm::ALL
                    .get(usize::from(i))
                    .is_some_and(|f| BackupForm::offered().contains(f)) => {}
            Action::CBackupForm(_) => {
                self.create.edit(action);
                if let Some(c) = self.create.state.as_mut().filter(|c| c.built.is_none()) {
                    c.done[cstep::BACKUP_FORM as usize] = true;
                    self.create_open_next();
                }
            }
            Action::CN(_) => {
                self.create.edit(action);
                self.create_from_refresh();
            }
            Action::CStep(_)
            | Action::CMoreKinds
            | Action::CM(_)
            | Action::CSlotHere(..)
            | Action::CSlotLater(_)
            | Action::CSlotClear(_)
            | Action::CPassOpen(_)
            | Action::CPassField(_) => {
                self.create.edit(action);
            }
            Action::CPassLock => self.create_pass_lock(),
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }

    /// Kind, closed on its default, is taken as answered once the person
    /// answers a card after it: the press is not on Kind, opens no card
    /// and starts nothing over.
    fn create_past_kind(&mut self, action: Action) {
        if matches!(
            action,
            Action::CreateWallet
                | Action::CreateOver
                | Action::CStep(_)
                | Action::CMoreKinds
                | Action::CBackupAbout
                | Action::CKind(_)
        ) {
            return;
        }
        if let Some(c) = self
            .create
            .state
            .as_mut()
            .filter(|c| c.open.is_some_and(|k| k != cstep::KIND) && !c.kind_unchosen)
        {
            c.done[cstep::KIND as usize] = true;
        }
    }

    /// A creation that has keys chosen and is not made yet: Create comes
    /// back to it.
    pub(crate) fn create_unfinished(&self) -> bool {
        self.create.state.as_ref().is_some_and(|c| {
            c.built.is_none() && c.slots.iter().any(|s| *s != create::Source::Empty)
        })
    }

    /// Costs what `create_vaults_now` costs. Whether Make a new wallet's Keys card would
    /// offer one way on only: a single key's slot, empty, whose one choice
    /// is its key flow, **New key** (no key in the session, no key or wallet
    /// file in the Inbox, offered or refused, no open vault's key and no
    /// locked vault). Such a card is passed over: the key flow opens in
    /// its place.
    pub(crate) fn create_keys_one_way(&self, c: &Compute) -> bool {
        let Some(cs) = self.create.state() else {
            return false;
        };
        if cs.built.is_some()
            || cs.kind.multi()
            || cs.slots.iter().any(|s| *s != create::Source::Empty)
        {
            return false;
        }
        if !self.session.keys.is_empty() {
            return false;
        }
        // A file the card lists, offered or refused: the card shows, so
        // the person sees why a refused file is not taken.
        let file = self
            .inbox
            .iter()
            .any(|it| matches!(it.kind, wallet::FileKind::Key | wallet::FileKind::Wallet));
        if file {
            return false;
        }
        let vaults = self.create_vaults_now(c);
        vaults.open.is_empty() && vaults.locked.is_empty()
    }

    /// The card Make a new wallet shows before Keys: where Cancel in a
    /// key flow opened in place of Keys returns to.
    pub(crate) fn create_card_before_keys(&self) -> Option<u8> {
        let cards = self.create_cards();
        let at = cards.iter().position(|&k| k == cstep::KEYS)?;
        at.checked_sub(1).and_then(|i| cards.get(i).copied())
    }

    /// Keys, just opened by a Continue on the card before it: where it
    /// offers one way on only ([`Faraday::create_keys_one_way`]), that key
    /// flow opens in its place, for slot 1, as its button would, and
    /// Make a new wallet waits on the card before Keys. Decides from the
    /// state as it is.
    pub(crate) fn create_keys_opened(&mut self) {
        let c = self.compute();
        if self.screen != Screen::Create
            || self.create.state().and_then(|c| c.open) != Some(cstep::KEYS)
            || !self.create_keys_one_way(&c)
        {
            return;
        }
        let before = self.create_card_before_keys();
        self.keygen_open(Some(0));
        if self.screen == Screen::KeyGen {
            self.create.open_card(before);
        }
    }

    /// Make a new wallet shown on Keys, its vaults set up: or, where Keys
    /// offers one way on only, its key flow ([`Faraday::create_keys_opened`]).
    pub(crate) fn create_open_keys(&mut self) {
        self.create.open_card(Some(cstep::KEYS));
        self.screen = Screen::Create;
        self.create_keys_opened();
    }

    /// Slots left for a cosigner's key, in the creation under way.
    pub fn create_waiting(&self) -> usize {
        self.create.state.as_ref().map_or(0, |c| {
            c.slots
                .iter()
                .filter(|s| **s == create::Source::Later)
                .count()
        })
    }

    /// Costs what `account_text` costs. The account key expression for a
    /// slot held here, with its fingerprint, for the cosigners.
    pub(crate) fn create_key_text(
        &self,
        c: &Compute,
        slot: u8,
    ) -> Result<(String, String), String> {
        let cs = self
            .create
            .state
            .as_ref()
            .ok_or(EN.create_key_text_nothing)?;
        let Some(create::Source::Here(fp)) = cs.slots.get(slot as usize) else {
            return Err(EN.create_key_text_not_held.into());
        };
        let key = self
            .session
            .keys
            .position_of(*fp)
            .ok_or(EN.create_key_text_a_chosen)?;
        Ok((
            fp_text(faraday_bip::keys::Fingerprint(*fp)),
            self.create_slot_key_text(c, key, cs)?,
        ))
    }

    /// Costs what `account_text` costs. Session key `key` as the creation
    /// `cs` takes it: at the kind's path, or at BIP-87's when that is
    /// chosen ([`CreateState::at_bip87`]).
    pub(crate) fn create_slot_key_text(
        &self,
        c: &Compute,
        key: usize,
        cs: &CreateState,
    ) -> Result<String, String> {
        if cs.at_bip87() {
            let path = cs
                .kind
                .path_with(self.session.keys.network(), cs.account, true);
            return self.session.keys.text_at(c, key, &path);
        }
        self.session.keys.account_text(c, key, cs.kind, cs.account)
    }

    /// Costs a key derivation (through `account_ur_at`). A slot's key as
    /// `ur:crypto-account`, for a `wsh` multisig, the one script that type
    /// writes a cosigner's key under.
    pub(crate) fn create_key_account(&self, c: &Compute, slot: u8) -> Option<String> {
        let cs = self.create.state.as_ref()?;
        let Some(create::Source::Here(fp)) = cs.slots.get(slot as usize) else {
            return None;
        };
        let key = self.session.keys.position_of(*fp)?;
        cs.kind
            .account_ur_at(c, &self.session.keys, key, cs.account, cs.at_bip87())
    }

    /// Costs a key derivation (through `bsms_record`). A slot's key as a
    /// signed BIP 129 key record, with its fingerprint: for a `wsh` or
    /// `sh(wsh)` multisig only.
    pub(crate) fn create_key_bsms(&self, c: &Compute, slot: u8) -> Option<(String, String)> {
        let cs = self.create.state.as_ref()?;
        let Some(create::Source::Here(fp)) = cs.slots.get(slot as usize) else {
            return None;
        };
        let key = self.session.keys.position_of(*fp)?;
        let fpt = fp_text(faraday_bip::keys::Fingerprint(*fp));
        cs.kind
            .bsms_record(c, &self.session.keys, key, cs.account, cs.at_bip87(), &fpt)
            .map(|r| (fpt, r))
    }

    /// Costs what `account_text` costs. The key texts of a creation's
    /// slots, or why they are not ready.
    pub fn create_keys(&self, c: &Compute) -> Result<Vec<String>, String> {
        let cs = self.create.state.as_ref().ok_or(EN.create_keys_nothing)?;
        let mut out = Vec::new();
        for (i, s) in cs.slots.iter().enumerate() {
            match s {
                create::Source::Empty => {
                    return Err(fill(EN.create_keys_key_has, &[&(i + 1).to_string()]));
                }
                create::Source::Later => {
                    return Err(fill(
                        EN.create_keys_key_waits,
                        &[&(i + 1).to_string(), self.medium.a(), &(i + 1).to_string()],
                    ));
                }
                create::Source::Here(fp) => {
                    let key = self
                        .session
                        .keys
                        .position_of(*fp)
                        .ok_or(EN.create_keys_a_chosen)?;
                    out.push(self.create_slot_key_text(c, key, cs)?);
                }
                create::Source::Cosigner(text) => out.push(text.clone()),
            }
        }
        let mut seen = out.clone();
        seen.sort();
        seen.dedup();
        if seen.len() != out.len() || !faraday_bip::policy::repeated_key_texts(&out).is_empty() {
            return Err(EN.create_keys_the_same.into());
        }
        Ok(out)
    }

    /// The slot a cosigner's key goes in, while a multisig is being made:
    /// one marked for later, else the first with no key.
    pub(crate) fn create_waiting_slot(&self) -> Option<u8> {
        let c = self.create.state.as_ref()?;
        if c.built.is_some() || !c.kind.multi() {
            return None;
        }
        c.slots
            .iter()
            .position(|s| *s == create::Source::Later)
            .or_else(|| c.slots.iter().position(|s| *s == create::Source::Empty))
            .map(|i| i as u8)
    }

    /// Puts a cosigner's account key in slot `k`, the wallet then made on
    /// the key's network `net`; why not, when that network is refused.
    fn create_slot_cosigner(
        &mut self,
        k: u8,
        key: Option<String>,
        net: Option<Network>,
    ) -> Result<(), String> {
        self.session.follow(net).map_err(|e| e.text())?;
        if let Some(c) = self.create.state.as_mut()
            && let Some(s) = c.slots.get_mut(k as usize)
            && let Some(key) = key
        {
            *s = create::Source::Cosigner(key);
        }
        Ok(())
    }

    /// Costs a read of the file's keys (a base58 decode each). Puts the
    /// key a file offers for slot `k` ([`key_for_slot`]) in it, the wallet
    /// then made on the key's network; why not, when the file has no key
    /// for the slot or its network is refused.
    fn create_slot_file_key(&mut self, k: u8, text: &str) -> Result<(), String> {
        let (kind, bip87) = self
            .create
            .state
            .as_ref()
            .map(|s| (s.kind, s.at_bip87()))
            .unwrap_or_default();
        let key = key_for_slot(kind, bip87, text)?;
        // A cosigner's key sets the network the wallet is made on.
        let net = PolicyKey::parse(&key)
            .ok()
            .map(|p| wallet::network_of_key(&p, self.session.network()));
        self.create_slot_cosigner(k, Some(key), net)
    }

    /// Costs a read of an account key file (a base58 decode) or, for a
    /// scanned text that reads as a descriptor but not as a wallet, a parse
    /// (through `read_wallet`) for its reason. Whether the
    /// scanned Inbox file `at`, named `name`, went to the slot waiting
    /// for a cosigner's key while a wallet is being made: an account key
    /// fills it, as does a descriptor of one key that can be the slot's
    /// ([`key_of_wallet`]); a descriptor that cannot stays in Files, and
    /// the toast says why. False when no slot waits, or for any other
    /// file.
    pub(crate) fn create_scan_arrived(&mut self, c: &Compute, at: usize, name: &str) -> bool {
        let Some(slot) = self.create_waiting_slot() else {
            return false;
        };
        let Some(item) = self.inbox.get(at) else {
            return false;
        };
        let (kind, bip87) = self
            .create
            .state
            .as_ref()
            .map(|s| (s.kind, s.at_bip87()))
            .unwrap_or_default();
        let current = self.session.network();
        let taken = match (item.kind, item.wallet()) {
            (wallet::FileKind::Key, _) => {
                let text =
                    zeroize::Zeroizing::new(String::from_utf8_lossy(&item.bytes).into_owned());
                self.create_slot_file_key(slot, &text)
            }
            // A file of account keys (Coldcard's export, a single-key
            // file) as Keys offers it, else a descriptor of one key.
            (wallet::FileKind::Wallet, Some(policy)) => {
                let text =
                    zeroize::Zeroizing::new(String::from_utf8_lossy(&item.bytes).into_owned());
                if key_for_slot(kind, bip87, &text).is_ok() {
                    self.create_slot_file_key(slot, &text)
                } else {
                    key_of_wallet(kind, bip87, policy, current).and_then(|(key, net)| {
                        self.create_slot_cosigner(slot, Some(key), Some(net))
                    })
                }
            }
            // A descriptor that does not read: its checksum, its keys.
            (wallet::FileKind::Text, _) => {
                let text = String::from_utf8_lossy(&item.bytes);
                if faraday_bip::descriptor::script_type(text.trim()).is_none() {
                    return false;
                }
                match wallet::read_wallet(c, &text) {
                    Err(e) => Err(e.text()),
                    Ok(_) => return false,
                }
            }
            _ => return false,
        };
        let number = (slot + 1).to_string();
        match taken {
            Ok(()) => self.toast(&fill(EN.scanned_is_key, &[name, &number])),
            Err(why) => self.toast(&fill(EN.create_scan_not_in_slot, &[name, &number, &why])),
        }
        self.screen = Screen::Create;
        true
    }

    /// Whether the key in a Create slot can take a passphrase there: one
    /// loaded here from BIP-39 words without one
    /// (`docs/history/NEW-WALLET.md` §3.6).
    pub(crate) fn create_pass_offered(&self, slot: usize) -> bool {
        let Some(c) = self.create.state.as_ref() else {
            return false;
        };
        let Some(create::Source::Here(fp)) = c.slots.get(slot) else {
            return false;
        };
        c.built.is_none()
            && self
                .session
                .keys
                .all()
                .find(|k| k.fingerprint.0 == *fp)
                .is_some_and(|k| k.has_words() && !k.has_passphrase)
    }

    /// Lock in under a Create slot: adds the key the slot's words and the
    /// passphrase typed make, and puts it in the slot in place of the
    /// first, which stays loaded (`docs/history/NEW-WALLET.md` §3.6).
    pub(crate) fn create_pass_lock(&mut self) {
        let c = self.compute();
        if !self.selftest_passed() {
            return;
        }
        let Some(slot) = self.create.state.as_ref().and_then(|cs| cs.pass_slot) else {
            return;
        };
        if !self.create_pass_offered(usize::from(slot)) {
            return;
        }
        let Some(cs) = self.create.state.as_mut() else {
            return;
        };
        if cs.pass.as_str() != cs.pass2.as_str() {
            return;
        }
        // Empty means none: the key stays as it is.
        if cs.pass.as_str().is_empty() {
            cs.close_pass();
            return;
        }
        let Some(create::Source::Here(first)) = cs.slots.get(usize::from(slot)).cloned() else {
            return;
        };
        let mut pass = secret_text::room();
        pass.push_str(cs.pass.as_str());
        let Some(place) = self.session.keys.position_of(first) else {
            return;
        };
        let key = &self.session.keys[place];
        if !key.has_words() {
            return;
        }
        let label = fill(EN.create_pass_lock_passphrase, &[&key.label.to_string()]);
        let (added, new) = match self
            .session
            .keys
            .add_with_passphrase(&c, place, &pass, &label)
        {
            Ok(fp) => (Ok(fp), true),
            // Loaded already with this passphrase: that key is the one.
            Err(wallet::Refusal::Duplicate(_)) => (
                self.session
                    .keys
                    .fingerprint_with(&c, place, &pass)
                    .ok_or_else(|| wallet::Refusal::Words(String::new()).text()),
                false,
            ),
            Err(e) => (Err(e.text()), false),
        };
        // The passphrase exists nowhere else: the key is made here
        // (`docs/history/NEW-WALLET.md` §14.3).
        if new && let Ok(fp) = &added {
            self.mark_made_here(*fp);
        }
        let Some(cs) = self.create.state.as_mut() else {
            return;
        };
        match added {
            Ok(fp) => {
                if let Some(s) = cs.slots.get_mut(usize::from(slot)) {
                    *s = create::Source::Here(fp.0);
                }
                if !cs.pass_locked.contains(&fp.0) {
                    cs.pass_locked.push(fp.0);
                }
                cs.close_pass();
                self.refresh_spend();
            }
            Err(e) => cs.pass_note = Some(e),
        }
    }

    /// Keys typed while a Create slot's passphrase fields have focus.
    /// Returns whether they took the key.
    pub(crate) fn create_pass_key(&mut self, key: KeyIn) -> bool {
        if self.screen != Screen::Create || self.sheet.is_some() {
            return false;
        }
        let Some(c) = self.create.state.as_mut() else {
            return false;
        };
        let (Some(_), Some(f)) = (c.pass_slot, c.pass_focus) else {
            return false;
        };
        let field = if f == 0 { &mut c.pass } else { &mut c.pass2 };
        match key {
            KeyIn::Char(ch) if (' '..='~').contains(&ch) => field.push(ch),
            // Anything else is refused, and the slot says so.
            KeyIn::Char(ch) if !ch.is_control() => {
                c.pass_note = Some(crate::state::ascii_refused());
                return true;
            }
            KeyIn::Backspace => {
                field.pop();
            }
            KeyIn::Tab => c.pass_focus = Some(1 - f.min(1)),
            KeyIn::Escape => c.pass_focus = None,
            KeyIn::Enter => {
                self.create_pass_lock();
                return true;
            }
            _ => return false,
        }
        c.pass_note = None;
        true
    }

    /// A single key's Backup card, while the wallet is not made: the form
    /// its slot's New key makes (F240). `None` for a multisig, before the
    /// card is answered, and with no creation open.
    pub(crate) fn create_slot_backup(&self) -> Option<BackupForm> {
        self.create
            .state()
            .filter(|c| !c.kind.multi() && c.built.is_none())
            .and_then(|c| c.backup)
    }

    /// Whether a single key's Backup card is still to answer: Keys waits
    /// for it.
    pub(crate) fn create_backup_unanswered(&self) -> bool {
        self.create
            .state()
            .is_some_and(|c| c.built.is_none() && c.backup.is_none())
            && self.create_cards().contains(&cstep::BACKUP_FORM)
    }

    /// Make codex32 shares' Lock in for a single key's slot, whatever its
    /// address type: the set's facts are kept with the creation, for Check
    /// and for Back up. Called before the slot is filled.
    pub(crate) fn create_codex32_made(&mut self, set: SharesSet) {
        if let Some(c) = self
            .create
            .state
            .as_mut()
            .filter(|c| !c.kind.multi() && c.built.is_none())
        {
            c.shares_set = Some(set);
        }
    }

    /// The codex32 set of wallet `wallet`, when this creation made it from
    /// a key made here as codex32 shares, the key in its slot: Check's
    /// Backup row, and what Back up opens on (`docs/CODEX32.md` §6.9,
    /// §9.5).
    pub fn create_shares_set(&self, wallet: usize) -> Option<&SharesSet> {
        // A single key's only: a kind changed after the set was made (Check's
        // Path, Change) keeps the set and the key in slot 0.
        let c = self
            .create
            .state()
            .filter(|c| c.built == Some(wallet) && !c.kind.multi())?;
        let set = c.shares_set.as_ref()?;
        (c.slots.first() == Some(&create::Source::Here(set.fingerprint))).then_some(set)
    }

    /// A wallet just made with a key recorded as a BIP-85 child seed
    /// (from its vault record or this session) opens **A BIP-85 child
    /// seed**, once for this creation (F166). A lookup over the slots.
    fn create_child_notice(&mut self) {
        let child = self.create.state().and_then(|c| {
            c.slots.iter().find_map(|s| match s {
                create::Source::Here(fp) => self
                    .session
                    .keys
                    .all()
                    .any(|k| k.fingerprint.0 == *fp && k.child_of.is_some())
                    .then_some(*fp),
                _ => None,
            })
        });
        self.create.set_child(child);
        if child.is_some() {
            self.sheet = Some(crate::Sheet::ChildNotice);
        }
    }

    pub(crate) fn create_make(&mut self) {
        let k = self.compute();
        let keys = match self.create_keys(&k) {
            Ok(k) => k,
            Err(e) => {
                if let Some(c) = self.create.state.as_mut() {
                    c.error = Some(e);
                }
                return;
            }
        };
        let Some(c) = self.create.state.as_ref() else {
            return;
        };
        let descriptor = c.kind.descriptor(c.m, &keys);
        // A wallet made from another is named after it, with the account
        // (`docs/history/NEW-WALLET.md` §16.2).
        let name = match c.from.as_ref() {
            Some(f) => f.name_at(c.kind, c.account),
            None => fill(
                EN.create_make_new_wallet,
                &[&(self.session.wallets.len() + 1).to_string()],
            ),
        };
        let before = self.session.wallets.len();
        match self.session.add_wallet(
            &k,
            &name,
            &descriptor,
            EN.create_make_made_here,
            wallet::Origin::Statement,
        ) {
            Ok(i) => {
                self.glance.moved_to(&self.session, i);
                self.wallets.set_wallet(i);
                let path = create::paths_text(self.session.wallets[i].policy());
                let owner = self.create.state.as_ref().map(|c| c.owner);
                let before_made = owner
                    .and_then(|o| self.wallet_flow_pre_of(o))
                    .map(|f| (f, Box::new(self.flows[f].clone())));
                if let Some(c) = self.create.state.as_mut() {
                    c.built = Some(i);
                    c.error = None;
                    c.done[cstep::KEYS as usize] = true;
                    c.open = Some(cstep::CHECK);
                    c.scroll.follow = true;
                    c.path = path;
                    c.added = i == before;
                    c.before_made = before_made;
                }
                self.create_child_notice();
                // The flow that decided where its keys go knows the wallet
                // now.
                if let Some(owner) = owner {
                    self.wallet_flow_attach(i, owner);
                }
            }
            Err(e) => {
                if let Some(c) = self.create.state.as_mut() {
                    c.error = Some(e.text());
                }
            }
        }
    }
}

/// The Sticks and vaults card of Make a new wallet and Restore on screen
/// (`docs/STICK-SETUP.md` §2): its board's presses and drags go to the
/// area, `backup_flow/layout.rs`; anything else is the backup board's.
impl Faraday {
    /// The Make a new wallet or Restore whose card is on screen and takes
    /// changes.
    fn layout_owner(&self) -> Option<u64> {
        let (owner, locked) = match self.screen {
            Screen::Create => self.create.state().map(|c| (c.owner, c.built.is_some()))?,
            Screen::Restore => self
                .restore
                .state()
                .map(|r| (r.owner, r.done[crate::rstep::SEEDS as usize]))?,
            _ => return None,
        };
        self.flows.layout_of(owner)?;
        (!locked && !self.setup_vault_open(owner)).then_some(owner)
    }

    /// A press of a board: the card's, or the backup board's.
    pub(crate) fn board_press(&mut self, p: crate::board::BoardPress) {
        let Some(owner) = self.layout_owner() else {
            return self.board_act(p);
        };
        let anchor = match p {
            crate::board::BoardPress::Menu(_) => self
                .pointer
                .rect_of(Action::Board(p))
                .map(|r| (r.x, r.y, r.w, r.h)),
            _ => None,
        };
        let opens = matches!(
            p,
            crate::board::BoardPress::Menu(_) | crate::board::BoardPress::MenuVerb(_)
        );
        let open = self.flows.layout_press(owner, p, anchor);
        if open && opens {
            self.sheet = Some(crate::Sheet::BoardMenu);
            self.scroll.reset_sheet();
        } else if !open && self.sheet == Some(crate::Sheet::BoardMenu) {
            self.sheet = None;
        }
    }

    /// The card's ⋮ menu, while it is open.
    pub(crate) fn layout_menu(&self) -> Option<crate::board::Menu> {
        self.flows.layout_menu(self.layout_owner()?)
    }

    /// A finger down: on a chip of the card or of the backup board.
    pub(crate) fn board_press_down(&mut self, found: Option<Action>, x: i32, y: i32) -> bool {
        match self.layout_owner() {
            Some(o) => {
                let still = self.compact || self.sheet.is_some();
                self.flows.layout_down(o, found, (x, y), still)
            }
            None => self.board_down(found, x, y),
        }
    }

    /// The finger moved with a chip pressed.
    pub(crate) fn board_press_move(&mut self, x: i32, y: i32) -> bool {
        let slop = crate::motion::SLOP * self.f;
        let Some(o) = self.layout_owner() else {
            return self.board_move(x, y);
        };
        match self.flows.layout_drag(o, (x, y), slop) {
            Some(took) => {
                if took {
                    self.pointer.set_pressed(None);
                }
                took
            }
            None => self.board_move(x, y),
        }
    }

    /// The finger lifted: a chip dragged is dropped or goes back.
    pub(crate) fn board_press_up(&mut self, x: i32, y: i32) -> bool {
        let Some(o) = self.layout_owner() else {
            return self.board_up(x, y);
        };
        let Some(press) = self.flows.layout_lift(o, (x, y), self.pointer.hits()) else {
            return self.board_up(x, y);
        };
        self.pointer.set_pressed(None);
        self.board_press(press);
        true
    }

    /// Costs what `place_setup` costs. The setup of `owner` follows its
    /// card.
    pub(crate) fn layout_apply(&mut self, owner: u64) {
        if let Some(i) = self.wallet_flow_pre_of(owner) {
            let c = self.compute();
            self.flows.layout_apply(i, owner, &c);
        }
    }

    /// Costs what `standard` costs. The answers Back up's board opens on
    /// for wallet `w`: The standard way, with the setup's sticks placed
    /// where the wallet's flow set up vaults before its keys and has not
    /// locked (`docs/STICK-SETUP.md` §6.1).
    pub(crate) fn wallet_start_answers(
        &self,
        c: &Compute,
        w: usize,
        shape: &crate::plan::Shape,
    ) -> crate::plan::Answers {
        let sets = self.c32_sets(w);
        let flow = self.session.wallets.get(w).and_then(|wl| {
            let f = &self.flows[self.wallet_flow_active(wl.checksum())?];
            f.wallet
                .as_ref()
                .filter(|wf| !wf.setup && !f.made && wf.shape.vaults > 0)
        });
        match flow {
            Some(wf) => {
                crate::plan::codex32::standard_over(c, &wf.shape, &sets, wf.answers.clone())
            }
            None => crate::plan::codex32::standard(c, shape, &sets),
        }
    }
}

/// Make a new wallet's cards, plugged into Back and Forward (F104).
pub(crate) struct CreateSteps;

/// The one [`CreateSteps`].
pub(crate) static CREATE_STEPS: CreateSteps = CreateSteps;

impl crate::trail::StepFlow for CreateSteps {
    fn steps(&self, app: &crate::Faraday) -> Option<crate::trail::Steps> {
        let c = app.create.state()?;
        let mut s = crate::trail::Steps::new(app.create_cards(), c.open);
        // The wallet made stays made: Back goes no further than Check.
        if c.built.is_some() {
            s.first = Some(cstep::CHECK);
        }
        Some(s)
    }

    fn open(&self, _: &crate::Faraday, k: u8) -> crate::Action {
        crate::Action::CStep(k)
    }

    fn name(&self, app: &crate::Faraday, k: u8) -> &'static str {
        match k {
            cstep::STICKS => app.medium.bring_in(),
            cstep::HOW_MANY => app.medium.and_vaults(),
            k => crate::screens::CSTEPS[usize::from(k)],
        }
    }
}
