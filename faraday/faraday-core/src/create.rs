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
    /// FROST: any m of n shares sign one Taproot key, dealt here.
    Threshold,
    /// MuSig2: every key signs one Taproot key together,
    /// `tr(musig(…)/<0;1>/*)` over the keys' Taproot accounts (BIP-327,
    /// 328, 390).
    MuSig,
    /// Single key, backup in shares (`docs/CODEX32.md` §6.9): `wpkh`,
    /// BIP-84, as Single key is on chain; its key is made as k of n
    /// codex32 shares, the scheme asked on Quorum.
    Shares,
}

/// Whether Make codex32 shares is offered, and with it Make a new
/// wallet's Single key, backup in shares (`docs/CODEX32.md` §6.9). The
/// same switch as `codex32::MAKE_OFFERED`, which this one joins.
pub const MAKE_OFFERED: bool = true;

/// The create cards a kind shows.
pub fn create_steps(kind: NewKind) -> Vec<u8> {
    let mut v = vec![cstep::KIND];
    if kind.multi() || kind.shares() {
        v.push(cstep::QUORUM);
    }
    v.extend([cstep::WHERE, cstep::KEYS, cstep::CHECK, cstep::BACKUP]);
    v
}

/// BIP-341's unspendable internal key.
const NUMS: &str = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";

impl NewKind {
    /// Every kind, by its number in `Action::CKind` and `Action::RKind`
    /// (and in a backup flow kept across a lock): a new kind is added at
    /// the end, never between. The Kind card's order is
    /// [`NewKind::CREATE_ROWS`].
    pub const ALL: [NewKind; 11] = [
        NewKind::NativeSegwit,
        NewKind::Taproot,
        NewKind::NestedSegwit,
        NewKind::Legacy,
        NewKind::Multi,
        NewKind::MultiNested,
        NewKind::MultiLegacy,
        NewKind::TapMulti,
        NewKind::Threshold,
        NewKind::MuSig,
        NewKind::Shares,
    ];

    /// Make a new wallet's Kind rows, by place in [`NewKind::ALL`], in the
    /// order shown: Single key backup in shares, the default, then Single
    /// key and Multisig (`docs/CODEX32.md` §6.9, owner 2026-10-24), then
    /// the others behind **More kinds**.
    /// FROST (8) is not among them: Create makes no threshold wallet
    /// (F109); the kind stays in [`NewKind::ALL`] for what reads one.
    pub const CREATE_ROWS: [usize; 10] = [10, 0, 4, 1, 2, 3, 5, 6, 7, 9];

    /// How many of [`NewKind::CREATE_ROWS`] show before **More kinds**.
    pub const CREATE_FIRST: usize = 3;

    /// Make a new wallet's Kind rows as shown, by place in
    /// [`NewKind::ALL`]: the first three, or with `more` every one, and
    /// Single key, backup in shares only where `shares_offered`
    /// ([`MAKE_OFFERED`]).
    pub fn create_rows(more: bool, shares_offered: bool) -> Vec<usize> {
        let shown = if more {
            &Self::CREATE_ROWS[..]
        } else {
            &Self::CREATE_ROWS[..Self::CREATE_FIRST]
        };
        shown
            .iter()
            .copied()
            .filter(|&i| shares_offered || !Self::ALL[i].shares())
            .collect()
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
            NewKind::Threshold => EN.create_threshold_frost,
            NewKind::MuSig => EN.create_all_keys_musig2,
            NewKind::Shares => EN.create_single_key_shares,
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
            NewKind::Threshold => EN.create_any_m_of_n_shares,
            NewKind::MuSig => EN.create_every_key_signs,
            NewKind::Shares => EN.create_shares_line_plain,
        }
    }

    /// The second line of the kind's row on Make a new wallet's Kind card
    /// (`docs/CODEX32.md` §6.9): a fact of the row, with the counts the
    /// creation holds now: `m` of `n` for a multisig, `k` of `shares` for
    /// the backup in shares.
    pub fn create_line(self, (m, n): (usize, usize), (k, shares): (u8, u8)) -> String {
        match self {
            NewKind::NativeSegwit => EN.create_single_key_line.to_string(),
            NewKind::Shares => fill(
                EN.create_shares_line,
                &[&shares.to_string(), &k.to_string()],
            ),
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
            Template::Threshold { .. } => NewKind::Threshold,
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
                | NewKind::Threshold
                | NewKind::MuSig
        )
    }

    /// Whether the one key is made as codex32 shares, its scheme asked on
    /// Quorum (`docs/CODEX32.md` §6.9).
    pub fn shares(self) -> bool {
        self == NewKind::Shares
    }

    /// Whether every key must sign: MuSig2, n of n.
    pub fn all_sign(self) -> bool {
        self == NewKind::MuSig
    }

    /// Whether the keys are shares dealt here rather than account keys.
    pub fn threshold(self) -> bool {
        self == NewKind::Threshold
    }

    /// The most keys a wallet of this kind has: five shares for FROST,
    /// fifteen keys for a multisig.
    pub fn max_keys(self) -> usize {
        if self.threshold() { 5 } else { 15 }
    }

    /// The path a key of this kind is taken at, from its master, on
    /// `network`.
    pub fn path(self, network: faraday_bip::keys::Network) -> String {
        let c = network.coin_type();
        match self {
            NewKind::NativeSegwit | NewKind::Shares => format!("m/84'/{c}'/0'"),
            NewKind::Taproot => format!("m/86'/{c}'/0'"),
            NewKind::NestedSegwit => format!("m/49'/{c}'/0'"),
            NewKind::Legacy => format!("m/44'/{c}'/0'"),
            NewKind::Multi => format!("m/48'/{c}'/0'/2'"),
            NewKind::MultiNested => format!("m/48'/{c}'/0'/1'"),
            NewKind::MultiLegacy => "m/45'".to_string(),
            NewKind::TapMulti => format!("m/48'/{c}'/0'/3'"),
            NewKind::Threshold => String::new(),
            NewKind::MuSig => format!("m/86'/{c}'/0'"),
        }
    }

    /// The kind's path with account `account` in place of 0: the
    /// account is BIP-44's third step, BIP-48's too. `m/45'` has none
    /// and stays as it is.
    pub fn path_at(self, network: faraday_bip::keys::Network, account: u32) -> String {
        let path = self.path(network);
        let mut steps: Vec<String> = path.split('/').map(str::to_string).collect();
        if steps.len() >= 4 {
            steps[3] = format!("{account}'");
        }
        steps.join("/")
    }

    /// Whether the kind's path has an account to choose: every kind but
    /// `m/45'` and the shares dealt here.
    pub fn has_account(self) -> bool {
        !matches!(self, NewKind::MultiLegacy | NewKind::Threshold)
    }

    /// The card that carries the kind's Account row (F98): Quorum, at its
    /// foot, for a kind that has one; else Keys, at its head. `None` for
    /// a kind with no account.
    pub fn account_card(self) -> Option<u8> {
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
    /// account key, or its path (`docs/history/NEW-WALLET.md` §16.2). The
    /// shares dealt here have none.
    pub fn spec(self, network: Network, account: u32) -> Result<faraday_keys::Spec, String> {
        use faraday_keys::Spec;
        Ok(match self {
            NewKind::Threshold => return Err(EN.create_a_share_has.to_string()),
            NewKind::NativeSegwit | NewKind::Shares => {
                Spec::Single(ScriptType::NativeSegwit, account)
            }
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
    pub fn account_ur(self, c: &Compute, keys: &faraday_keys::Keys, i: usize) -> Option<String> {
        self.account_ur_at(c, keys, i, 0)
    }

    /// Costs a key derivation. [`NewKind::account_ur`] at account
    /// `account`.
    pub fn account_ur_at(
        self,
        c: &Compute,
        keys: &faraday_keys::Keys,
        i: usize,
        account: u32,
    ) -> Option<String> {
        if self != NewKind::Multi {
            return None;
        }
        let a = keys
            .multisig_account(c, i, MultisigScriptType::NativeSegwit, account)?
            .ok()?;
        let path: Vec<_> = a.path.into_iter().copied().collect();
        let cbor = faraday_qr::registry::account_cbor(a.fingerprint.0, &path, &a.xpub, false);
        Some(faraday_codec::ur::encode_single("crypto-account", &cbor))
    }

    /// Costs a key derivation and a signature. Session key `i`'s BIP 129
    /// key record, signed by the account key itself: what Coldcard,
    /// Sparrow, Nunchuk and Keystone take from a signer. For the multisig
    /// kinds BIP 129 covers (`wsh` and `sh(wsh)`), at account `account` of
    /// the kind's path, described by the key's fingerprint text `fp`.
    pub fn bsms_record(
        self,
        c: &Compute,
        keys: &faraday_keys::Keys,
        i: usize,
        account: u32,
        fp: &str,
    ) -> Option<String> {
        let script = match self {
            NewKind::Multi => MultisigScriptType::NativeSegwit,
            NewKind::MultiNested => MultisigScriptType::NestedSegwit,
            _ => return None,
        };
        keys.bsms_record(c, i, script, account, &format!("Faraday key {fp}"))
    }

    /// The descriptor over these keys.
    pub fn descriptor(self, m: usize, keys: &[String]) -> String {
        let list = keys
            .iter()
            .map(|k| format!("{k}/<0;1>/*"))
            .collect::<Vec<_>>()
            .join(",");
        match self {
            NewKind::NativeSegwit | NewKind::Shares => format!("wpkh({list})"),
            NewKind::Taproot => format!("tr({list})"),
            NewKind::NestedSegwit => format!("sh(wpkh({list}))"),
            NewKind::Legacy => format!("pkh({list})"),
            NewKind::Multi => format!("wsh(sortedmulti({m},{list}))"),
            NewKind::MultiNested => format!("sh(wsh(sortedmulti({m},{list})))"),
            NewKind::MultiLegacy => format!("sh(sortedmulti({m},{list}))"),
            NewKind::TapMulti => format!("tr({NUMS},sortedmulti_a({m},{list}))"),
            // The deal makes a threshold wallet's record; it has no
            // descriptor over account keys.
            NewKind::Threshold => String::new(),
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
pub fn normal_path(text: &str) -> Option<String> {
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
    /// A cosigner's `[fingerprint/path]xpub` for each slot held only as
    /// an xpub; `None` for a key held here or in a vault.
    pub xpubs: Vec<Option<String>>,
}

impl FromWallet {
    /// What slot `slot` holds while the creation is a `kind` wallet at
    /// `account`, when the wallet's key there is a cosigner's xpub: that
    /// xpub at the wallet's own account and kind, which it gives and no
    /// other; left for the cosigner otherwise. `None` for a slot whose
    /// key is held.
    pub fn xpub_slot(&self, slot: usize, kind: NewKind, account: u32) -> Option<Source> {
        let text = self.xpubs.get(slot)?.as_ref()?;
        Some(if kind == self.kind && account == self.account {
            Source::Cosigner(text.clone())
        } else {
            Source::Later
        })
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

/// The key in a file for a slot of a `kind` wallet: of the keys the file
/// offers, the one at the kind's derivation (any account, any network);
/// or, from a file that holds one key only, that key, as the person chose
/// the file for it. None for Single key, backup in shares, whose key is
/// made here as codex32 shares.
pub fn key_for(kind: NewKind, text: &str) -> Option<String> {
    if kind.shares() {
        return None;
    }
    let keys = read_keys(text);
    // Purpose, and for BIP-48 the script type: what the kind's path says
    // apart from its coin type and account.
    let shape = |path: &str| {
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
    };
    let want = shape(&kind.path(faraday_bip::keys::Network::Mainnet))?;
    let at = |k: &String| {
        let path = k.get(k.find('/')? + 1..k.find(']')?)?;
        shape(path)
    };
    keys.iter()
        .find(|k| at(k).as_deref() == Some(want.as_str()))
        .cloned()
        .or_else(|| {
            (keys.len() == 1 && !text.trim_start().starts_with('{') && named_agrees(kind, text))
                .then(|| keys[0].clone())
        })
}

/// Whether the one key in a file, when its version is SLIP-132's, names
/// the script type of a `kind` wallet: a `zpub` fills a native SegWit
/// slot and no other. Costs a base58 decode per line.
fn named_agrees(kind: NewKind, text: &str) -> bool {
    use slip132::Script;
    let around = match kind {
        NewKind::NativeSegwit | NewKind::Shares => Script::NativeSegwit,
        NewKind::NestedSegwit => Script::NestedSegwit,
        NewKind::Legacy => Script::Legacy,
        NewKind::Multi => Script::NativeMultisig,
        NewKind::MultiNested => Script::NestedMultisig,
        NewKind::MultiLegacy => Script::LegacyMultisig,
        NewKind::Taproot | NewKind::TapMulti | NewKind::Threshold | NewKind::MuSig => {
            Script::Taproot
        }
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
    WalletPolicy::parse_any(c, &kind.descriptor(m, keys)).map_err(|e| e.to_string())
}

/// A wallet's derivation paths as Make a new wallet's Check shows them
/// (`docs/history/NEW-WALLET.md` §17): the one path when every key has it, else
/// each key's fingerprint and path. A key that came with no origin
/// reads as such.
pub fn paths_text(policy: &WalletPolicy) -> String {
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

/// What Make a new wallet keeps of the codex32 set its key was made as
/// (`docs/CODEX32.md` §6.9, §9.4): plain facts, no string. Check shows
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
    /// Single key, backup in shares: how many strings rebuild the key,
    /// 2 to 9 (`docs/CODEX32.md` §6.9).
    pub share_k: u8,
    /// Single key, backup in shares: how many strings are made, `share_k`
    /// to 31.
    pub share_n: u8,
    /// The codex32 set the slot's key was made as, from Make codex32
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
    /// The wallet the creation was started from, by **New wallet from
    /// this one**.
    pub from: Option<FromWallet>,
    /// The made wallet's derivation paths as Check shows them, worked out
    /// once when it is made (`docs/history/NEW-WALLET.md` §17).
    pub path: String,
    /// The made wallet was added to the session by this creation, not
    /// found there already: taking it back removes it.
    pub added: bool,
    /// The creation's backup flow as it stood before the made wallet was
    /// tied to it, by index: what taking the wallet back puts back.
    pub before_made: Option<(usize, Box<backup_flow::BackupFlow>)>,
}

impl CreateState {
    /// The person answered yes to using vaults.
    pub fn into_vaults(&self) -> bool {
        self.vaults.answer == Some(true)
    }

    /// Closes a slot's passphrase fields, forgetting what was typed.
    pub fn close_pass(&mut self) {
        self.pass_slot = None;
        self.pass.clear();
        self.pass2.clear();
        self.pass_focus = None;
        self.pass_note = None;
    }
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

    /// The flow's state, for the writers in other areas, until they are
    /// narrowed.
    pub(crate) fn state_mut(&mut self) -> Option<&mut CreateState> {
        self.state.as_mut()
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
                    if c.kind.multi() {
                        c.n = c.n.clamp(2, c.kind.max_keys());
                        c.m = if c.kind.all_sign() { c.n } else { c.m.min(c.n) };
                    }
                    if c.kind.shares() && c.share_k < 2 {
                        (c.share_k, c.share_n) = (2, 3);
                    }
                    let n = if c.kind.multi() { c.n } else { 1 };
                    c.slots.resize(n, create::Source::Empty);
                    c.slots.truncate(n);
                }
            }
            Action::CMoreKinds => {
                if let Some(c) = self.state.as_mut() {
                    c.more_kinds = true;
                }
            }
            // Single key, backup in shares: the scheme's two steppers,
            // How many to rebuild it (2 to 9) and How many shares (k to
            // 31), as Make codex32 shares' Other has them.
            Action::CM(d) if self.state.as_ref().is_some_and(|c| c.kind.shares()) => {
                if let Some(c) = self.state.as_mut() {
                    let max = faraday_bip::codex32::MAX_THRESHOLD;
                    c.share_k =
                        (i16::from(c.share_k) + i16::from(d)).clamp(2, i16::from(max)) as u8;
                    c.share_n = c.share_n.max(c.share_k);
                }
            }
            Action::CN(d) if self.state.as_ref().is_some_and(|c| c.kind.shares()) => {
                if let Some(c) = self.state.as_mut() {
                    let max = faraday_bip::codex32::MAX_SHARES;
                    c.share_n = (i16::from(c.share_n) + i16::from(d))
                        .clamp(i16::from(c.share_k), i16::from(max))
                        as u8;
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
                // Create opens on Kind, open, with Single key, backup in
                // shares ticked where Make codex32 shares is offered, else
                // Single key (owner, 2026-10-24; `docs/CODEX32.md` §6.9):
                // the ticked row pressed again takes it. Back goes where
                // it was opened from, Home or Wallets.
                let done = [false; cstep::COUNT];
                let owner = self.fresh_owner();
                self.create.state = Some(CreateState {
                    owner,
                    done,
                    open: Some(cstep::KIND),
                    kind: if MAKE_OFFERED {
                        NewKind::Shares
                    } else {
                        NewKind::NativeSegwit
                    },
                    // Start over keeps it.
                    from_home: self.screen == Screen::Home
                        || (self.screen == Screen::Create
                            && self.create.state.as_ref().is_some_and(|c| c.from_home)),
                    m: 2,
                    n: 3,
                    share_k: 2,
                    share_n: 3,
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
            // The info button after "codex32" shows its tooltip at once
            // (F112).
            Action::CKindAbout => self.pointer.tip_show(Action::CKindAbout),
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
                self.create_open_next();
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
            Action::CVaults(_) | Action::CVaultN(_)
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
            Action::PlanAddPass => self.plan_add_passphrase(),
            // NEW-WALLET §16 (F45): keys from a vault in Create; a new
            // wallet from one held.
            Action::CSlotVault(k, v, r) => self.create_slot_vault(k, v, r),
            Action::CAccount(d) => self.create_account_step(d),
            Action::CPathChange => self.create_take_back(),
            Action::NewWalletFrom(i) => self.create_from_wallet(i),
            Action::CSlotFile(k, i) => {
                let kind = self
                    .create
                    .state
                    .as_ref()
                    .map(|c| c.kind)
                    .unwrap_or_default();
                let key = self
                    .inbox
                    .get(i)
                    .and_then(|it| create::key_for(kind, &String::from_utf8_lossy(&it.bytes)));
                // A cosigner's key sets the network the wallet is made on.
                let net = key
                    .as_deref()
                    .and_then(|k| faraday_bip::policy::PolicyKey::parse(k).ok())
                    .map(|k| wallet::network_of_key(&k, self.session.network()));
                if let Err(e) = self.session.follow(net) {
                    if let Some(c) = self.create.state.as_mut() {
                        c.error = Some(e.text());
                    }
                    return;
                }
                if let Some(c) = self.create.state.as_mut()
                    && let Some(s) = c.slots.get_mut(k as usize)
                    && let Some(key) = key
                {
                    *s = create::Source::Cosigner(key);
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
                            self.put_outbox(
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
                    self.put_outbox(&name, text.into_bytes());
                    self.toast_out(&name);
                }
                None => self.toast(EN.act_bip_129_covers),
            },
            // A kind moves on by itself, and so does the kind already
            // ticked, pressed again: that is how the default is taken.
            // FROST is not made here (F109): a press for it, as the Tools
            // tile's, chooses nothing.
            // Nor the backup in shares while Make codex32 shares is not
            // offered.
            Action::CKind(i)
                if create::NewKind::ALL
                    .get(usize::from(i))
                    .is_some_and(|k| k.threshold() || (k.shares() && !MAKE_OFFERED)) => {}
            Action::CKind(_) => {
                self.create.edit(action);
                self.create_from_refresh();
                if let Some(c) = self.create.state.as_mut().filter(|c| c.built.is_none()) {
                    c.done[cstep::KIND as usize] = true;
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
                | Action::CKindAbout
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
    pub fn create_unfinished(&self) -> bool {
        self.create.state.as_ref().is_some_and(|c| {
            c.built.is_none() && c.slots.iter().any(|s| *s != create::Source::Empty)
        })
    }

    /// Costs what `create_vaults_now` costs, and a read of each Inbox file
    /// for a key of the kind. Whether Make a new wallet's Keys card would
    /// offer one way on only: a single key's slot, empty, whose one choice
    /// is its key flow, **New key** (no key in the session, no key file in
    /// the Inbox, no open vault's key and no locked vault) or **Make
    /// codex32 shares** for a backup in shares, which takes no other key.
    /// Such a card is passed over: the key flow opens in its place.
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
        if cs.kind.shares() {
            return true;
        }
        if !self.session.keys.is_empty() {
            return false;
        }
        let file = self.inbox.iter().any(|it| {
            matches!(it.kind, wallet::FileKind::Key | wallet::FileKind::Wallet)
                && create::key_for(cs.kind, &String::from_utf8_lossy(&it.bytes)).is_some()
        });
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
        if self.screen == Screen::KeyGen
            && let Some(c) = self.create.state_mut()
        {
            c.open = before;
            c.scroll.follow = true;
        }
    }

    /// Make a new wallet shown on Keys, its vaults set up: or, where Keys
    /// offers one way on only, its key flow ([`Faraday::create_keys_opened`]).
    pub(crate) fn create_open_keys(&mut self) {
        if let Some(c) = self.create.state_mut() {
            c.open = Some(cstep::KEYS);
            c.scroll.follow = true;
        }
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
            self.session
                .keys
                .account_text(c, key, cs.kind, cs.account)?,
        ))
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
            .account_ur_at(c, &self.session.keys, key, cs.account)
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
            .bsms_record(c, &self.session.keys, key, cs.account, &fpt)
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
                    out.push(
                        self.session
                            .keys
                            .account_text(c, key, cs.kind, cs.account)?,
                    );
                }
                create::Source::Cosigner(text) => out.push(text.clone()),
            }
        }
        let mut seen = out.clone();
        seen.sort();
        seen.dedup();
        if seen.len() != out.len() {
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

    /// Whether the key in a Create slot can take a passphrase there: one
    /// loaded here from BIP-39 words without one
    /// (`docs/history/NEW-WALLET.md` §3.6).
    pub fn create_pass_offered(&self, slot: usize) -> bool {
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
            cs.pass_note = Some(EN.create_pass_lock_mismatch.to_string());
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

    /// Single key, backup in shares: the scheme its Quorum chose, `(k,
    /// n)`, while the wallet is not made; what its slot opens Make codex32
    /// shares with (`docs/CODEX32.md` §6.9). `None` for any other kind.
    pub fn create_codex32_scheme(&self) -> Option<(u8, u8)> {
        self.create
            .state()
            .filter(|c| c.kind.shares() && c.built.is_none())
            .map(|c| (c.share_k, c.share_n))
    }

    /// Make codex32 shares' Lock in for a slot of a backup-in-shares
    /// wallet: the set's facts are kept with the creation, for Check and
    /// for Back up. Called before the slot is filled.
    pub fn create_codex32_made(&mut self, set: SharesSet) {
        if let Some(c) = self
            .create
            .state
            .as_mut()
            .filter(|c| c.kind.shares() && c.built.is_none())
        {
            c.shares_set = Some(set);
        }
    }

    /// The codex32 set of wallet `wallet`, when it is the one this
    /// creation made as Single key, backup in shares: what Back up opens
    /// on (`docs/CODEX32.md` §6.9, §9.5).
    pub fn create_shares_set(&self, wallet: usize) -> Option<&SharesSet> {
        self.create
            .state()
            .filter(|c| c.kind.shares() && c.built == Some(wallet))
            .and_then(|c| c.shares_set.as_ref())
    }

    /// Single key, backup in shares: why its slot's key does not make the
    /// wallet, unless it is the key a codex32 set was made as here.
    fn create_shares_refusal(&self) -> Option<String> {
        let c = self.create.state().filter(|c| c.kind.shares())?;
        let Some(create::Source::Here(fp)) = c.slots.first() else {
            return None;
        };
        match &c.shares_set {
            Some(set) if set.fingerprint == *fp => None,
            _ => Some(EN.create_shares_key_needed.to_string()),
        }
    }

    pub(crate) fn create_make(&mut self) {
        let k = self.compute();
        if let Some(e) = self.create_shares_refusal() {
            if let Some(c) = self.create.state.as_mut() {
                c.error = Some(e);
            }
            return;
        }
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
                self.glance.work_mut().moved_to(&self.session, i);
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
