//! Creating a wallet (`docs/WALLETS.md` §5): which kind, the quorum, a key
//! for every slot, and the descriptor they make. The keys can be loaded
//! here, made here from fresh entropy, or arrive from a cosigner as an
//! account key in a file. Nothing secret leaves the session.

use crate::strings::EN;
use crate::strings::fill;
use faraday_bip::keys::{MasterKey, MultisigScriptType, ScriptType};
use faraday_bip::policy::{PolicyKey, WalletPolicy};
use faraday_bip::slip132;
use faraday_crypto::work::Compute;

use crate::{
    Action, Code, Faraday, MAX_VAULTS, Screen, VaultAsk, backup_flow, create, cstep, flow,
    secret_text, wallet,
};

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
}

/// BIP-341's unspendable internal key.
const NUMS: &str = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";

impl NewKind {
    /// Every kind, in the order the Kind card lists them.
    pub const ALL: [NewKind; 10] = [
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
    ];

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
            NewKind::MultiNested => EN.create_multisig_with_3,
            NewKind::MultiLegacy => EN.create_multisig_with,
            NewKind::TapMulti => EN.create_multisig_on,
            NewKind::Threshold => EN.create_any_m_of_n_shares,
            NewKind::MuSig => EN.create_every_key_signs,
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
            NewKind::NativeSegwit => format!("m/84'/{c}'/0'"),
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

    /// Costs a key derivation. `[fingerprint/path]xpub` of a master key, at
    /// this kind's path.
    pub fn key_text(self, c: &Compute, master: &MasterKey) -> Result<String, String> {
        self.key_text_account(c, master, 0)
    }

    /// Whether the kind's path has an account to choose: every kind but
    /// `m/45'` and the shares dealt here.
    pub fn has_account(self) -> bool {
        !matches!(self, NewKind::MultiLegacy | NewKind::Threshold)
    }

    /// Costs a key derivation. `[fingerprint/path]xpub` of a master key, at
    /// this kind's path with account `account` in place of 0
    /// (`docs/history/NEW-WALLET.md` §16.2).
    pub fn key_text_account(
        self,
        c: &Compute,
        master: &MasterKey,
        account: u32,
    ) -> Result<String, String> {
        let key = match self {
            NewKind::Threshold => return Err(EN.create_a_share_has.to_string()),
            NewKind::NativeSegwit
            | NewKind::Taproot
            | NewKind::NestedSegwit
            | NewKind::Legacy
            | NewKind::MuSig => {
                let t = match self {
                    NewKind::NativeSegwit => ScriptType::NativeSegwit,
                    // A MuSig2 participant's key is its Taproot account.
                    NewKind::Taproot | NewKind::MuSig => ScriptType::Taproot,
                    NewKind::NestedSegwit => ScriptType::NestedSegwit,
                    _ => ScriptType::Legacy,
                };
                faraday_keys::texts::single_account(c, master, t, account)?
            }
            NewKind::Multi | NewKind::MultiNested => {
                let t = if self == NewKind::Multi {
                    MultisigScriptType::NativeSegwit
                } else {
                    MultisigScriptType::NestedSegwit
                };
                faraday_keys::texts::multisig_account(c, master, t, account)?
            }
            NewKind::MultiLegacy | NewKind::TapMulti => {
                let path: faraday_bip::bitcoin::bip32::DerivationPath = self
                    .path_at(master.network(), account)
                    .parse()
                    .map_err(|e| format!("{e}"))?;
                faraday_keys::texts::account_at(c, master, &path)
            }
        };
        let shown = format!("{}", key.path)
            .trim_start_matches("m/")
            .replace('\'', "h");
        Ok(format!("[{}/{shown}]{}", key.fingerprint, key.xpub))
    }

    /// Costs a key derivation. A `wsh` multisig key as the
    /// `ur:crypto-account` SeedSigner shows for one; `None` for any other
    /// kind.
    pub fn account_ur(self, c: &Compute, master: &MasterKey) -> Option<String> {
        self.account_ur_at(c, master, 0)
    }

    /// Costs a key derivation. [`NewKind::account_ur`] at account
    /// `account`.
    pub fn account_ur_at(self, c: &Compute, master: &MasterKey, account: u32) -> Option<String> {
        if self != NewKind::Multi {
            return None;
        }
        let a = faraday_keys::texts::multisig_account(
            c,
            master,
            MultisigScriptType::NativeSegwit,
            account,
        )
        .ok()?;
        let path: Vec<_> = a.path.into_iter().copied().collect();
        let cbor = faraday_qr::registry::account_cbor(a.fingerprint.0, &path, &a.xpub, false);
        Some(faraday_codec::ur::encode_single("crypto-account", &cbor))
    }

    /// Costs a key derivation and a signature. A key's BIP 129 key record,
    /// signed by the account key itself: what Coldcard, Sparrow, Nunchuk
    /// and Keystone take from a signer. For the multisig kinds BIP 129
    /// covers (`wsh` and `sh(wsh)`), at account `account` of the kind's
    /// path.
    pub fn bsms_record(
        self,
        c: &Compute,
        master: &MasterKey,
        account: u32,
        description: &str,
    ) -> Option<String> {
        let script = match self {
            NewKind::Multi => MultisigScriptType::NativeSegwit,
            NewKind::MultiNested => MultisigScriptType::NestedSegwit,
            _ => return None,
        };
        faraday_keys::texts::bsms_record(c, master, script, account, description)
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

/// Costs what `derive` and `to_xpub` cost. `[fingerprint/path]xpub` of
/// a master key at `path`, written `m/48'/0'/0'/2'` or `48h/0h/0h/2h`:
/// what [`NewKind::key_text`] gives at a kind's own path, at any other.
pub fn key_text_at(c: &Compute, master: &MasterKey, path: &str) -> Result<String, String> {
    let path = normal_path(path).ok_or_else(|| fill(EN.create_not_a_derivation, &[path]))?;
    let parsed: faraday_bip::bitcoin::bip32::DerivationPath =
        path.parse().map_err(|e| format!("{e}"))?;
    let key = faraday_keys::texts::account_at(c, master, &parsed);
    let shown = path
        .trim_start_matches('m')
        .trim_start_matches('/')
        .replace('\'', "h");
    if shown.is_empty() {
        return Ok(format!("[{}]{}", key.fingerprint, key.xpub));
    }
    Ok(format!("[{}/{shown}]{}", key.fingerprint, key.xpub))
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
/// the file for it.
pub fn key_for(kind: NewKind, text: &str) -> Option<String> {
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
        NewKind::NativeSegwit => Script::NativeSegwit,
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

/// Creating a wallet: the cards' state.
#[derive(Default)]
pub struct CreateState {
    /// The open card.
    pub open: Option<u8>,
    /// Opened from Home: Back goes to Home, not Wallets.
    pub from_home: bool,
    /// Cards closed as done.
    pub done: [bool; cstep::COUNT],
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// The kind.
    pub kind: NewKind,
    /// Signatures needed.
    pub m: usize,
    /// Keys.
    pub n: usize,
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

impl Faraday {
    /// Costs what the actions below cost, the keys' and wallets' work among
    /// it: handles the Create wallet flow's actions (`Action::C*`, `CreateWallet`, `CreateOver`, `PlanAddPass`, `NewWalletFrom`).
    pub(crate) fn create_act(&mut self, action: Action, c: &Compute) {
        match action {
            Action::CreateWallet
                if self.create_unfinished()
                    || self.create.as_ref().is_some_and(|c| {
                        self.setup_pending(c.owner, c.into_vaults(), c.built.is_some())
                    }) =>
            {
                let from_home = self.screen == Screen::Home;
                if let Some(c) = self.create.as_mut() {
                    c.from_home = from_home;
                }
                self.screen = Screen::Create;
            }
            Action::CreateWallet | Action::CreateOver => {
                // Create opens with Kind open, whatever opened it
                // (`docs/history/NEW-WALLET.md` §2.1); Back goes where it was
                // opened from, Home or Wallets.
                // Where the keys will be kept is done only once the person
                // has pressed its own Continue, whatever is ticked to begin
                // with.
                let done = [false; cstep::COUNT];
                let owner = self.fresh_owner();
                self.create = Some(CreateState {
                    owner,
                    done,
                    open: Some(cstep::KIND),
                    // Start over keeps it.
                    from_home: self.screen == Screen::Home
                        || (self.screen == Screen::Create
                            && self.create.as_ref().is_some_and(|c| c.from_home)),
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
                    && self.create.as_ref().is_some_and(|c| {
                        self.setup_pending(c.owner, c.into_vaults(), c.built.is_some())
                    }) => {}
            Action::CStep(k) => {
                if let Some(c) = self.create.as_mut() {
                    c.open = if c.open == Some(k) { None } else { Some(k) };
                    c.scroll.follow = true;
                }
            }
            // Keys' Continue makes the wallet, which opens Check; until
            // it is made, a refusal keeps Keys open with the reason.
            Action::CNext(k)
                if k == cstep::KEYS && self.create.as_ref().is_some_and(|c| c.built.is_none()) =>
            {
                self.create_make();
            }
            // The question of vaults waits for its answer.
            Action::CNext(k)
                if k == cstep::WHERE
                    && self
                        .create
                        .as_ref()
                        .is_some_and(|c| c.vaults.answer.is_none()) => {}
            // How many vaults: Continue opens the setup, which brings in
            // each vault's stick and opens or makes the vault; it comes
            // back to Keys.
            Action::CNext(k) if k == cstep::HOW_MANY => {
                if let Some(c) = self.create.as_mut() {
                    c.done[k as usize] = true;
                    let owner = c.owner;
                    self.wallet_flow_begin(owner);
                }
            }
            Action::CNext(k) => {
                if let Some(c) = self.create.as_mut() {
                    c.done[k as usize] = true;
                }
                self.create_open_next();
                // Check's own Continue: this wallet is checked this
                // power-on (`docs/WALLETS.md` §4).
                if k == cstep::CHECK
                    && let Some(w) = self.create.as_ref().and_then(|c| c.built)
                    && let Some(wallet) = self.session.wallets.get(w)
                {
                    let checksum = wallet.checksum().to_string();
                    self.mark_checked(&checksum);
                }
            }
            Action::CKind(i) => {
                if let Some(c) = self.create.as_mut()
                    && c.built.is_none()
                {
                    // Ticks the kind, whether chosen by hand on an open
                    // Kind card or by a Tools tile opening Create with
                    // its kind already ticked (`docs/history/NEW-WALLET.md`
                    // §2.1): either way Kind stays open until its own
                    // Continue closes it.
                    c.kind = create::NewKind::ALL[i as usize % create::NewKind::ALL.len()];
                    if c.kind.multi() {
                        c.n = c.n.clamp(2, c.kind.max_keys());
                        c.m = if c.kind.all_sign() { c.n } else { c.m.min(c.n) };
                    }
                    // A threshold wallet's slots are the shares chosen;
                    // the deal computes the rest.
                    let n = match c.kind {
                        k if k.threshold() => c.m,
                        k if k.multi() => c.n,
                        _ => 1,
                    };
                    if c.kind.threshold() {
                        c.slots.retain(|s| matches!(s, create::Source::Here(_)));
                    }
                    c.slots.resize(n, create::Source::Empty);
                    c.slots.truncate(n);
                }
                self.create_from_refresh();
            }
            Action::CBackup => {
                if let Some(i) = self.create.as_ref().and_then(|c| c.built) {
                    self.act(Action::Backup(i));
                    if let Some(b) = self.backup.as_mut() {
                        b.from_create = true;
                    }
                }
            }
            Action::CVaults(_) | Action::CVaultN(_)
                if self
                    .create
                    .as_ref()
                    .is_some_and(|c| c.built.is_none() && self.setup_vault_open(c.owner)) =>
            {
                self.toast(EN.create_vaults_locked);
            }
            Action::CVaults(yes) => {
                if let Some(c) = self.create.as_mut()
                    && c.built.is_none()
                    && c.vaults.answer != Some(yes)
                {
                    let owner = c.owner;
                    c.vaults.answer = Some(yes);
                    c.done[cstep::WHERE as usize] = false;
                    c.done[cstep::HOW_MANY as usize] = false;
                    self.wallet_vaults_changed(owner);
                }
            }
            Action::CVaultN(n) => {
                if let Some(c) = self.create.as_mut()
                    && c.built.is_none()
                {
                    let owner = c.owner;
                    c.vaults.count = usize::from(n).clamp(1, MAX_VAULTS);
                    c.done[cstep::HOW_MANY as usize] = false;
                    self.wallet_vaults_changed(owner);
                }
            }
            Action::PlanAddPass => self.plan_add_passphrase(),
            Action::CMoreKinds => {
                if let Some(c) = self.create.as_mut() {
                    c.more_kinds = true;
                }
            }
            Action::CM(d) => {
                if let Some(c) = self.create.as_mut() {
                    c.m = if c.kind.all_sign() {
                        c.n
                    } else {
                        (c.m as i64 + i64::from(d)).clamp(1, c.n as i64) as usize
                    };
                    if c.kind.threshold() {
                        c.slots.resize(c.m, create::Source::Empty);
                    }
                }
            }
            Action::CN(d) => {
                if let Some(c) = self.create.as_mut()
                    && c.kind.multi()
                {
                    c.n = (c.n as i64 + i64::from(d)).clamp(2, c.kind.max_keys() as i64) as usize;
                    c.m = if c.kind.all_sign() { c.n } else { c.m.min(c.n) };
                    let slots = if c.kind.threshold() { c.m } else { c.n };
                    c.slots.resize(slots, create::Source::Empty);
                }
                self.create_from_refresh();
            }
            // NEW-WALLET §16 (F45): keys from a vault in Create; a new
            // wallet from one held.
            Action::CSlotVault(k, v, r) => self.create_slot_vault(k, v, r),
            Action::CAccount(d) => self.create_account_step(d),
            Action::CPathChange => self.create_take_back(),
            Action::NewWalletFrom(i) => self.create_from_wallet(i),
            Action::CSlotHere(k, fp) => {
                if let Some(c) = self.create.as_mut()
                    && let Some(s) = c.slots.get_mut(k as usize)
                {
                    *s = create::Source::Here(fp);
                    if c.pass_slot == Some(k) {
                        c.close_pass();
                    }
                }
            }
            Action::CSlotFile(k, i) => {
                let kind = self.create.as_ref().map(|c| c.kind).unwrap_or_default();
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
                    if let Some(c) = self.create.as_mut() {
                        c.error = Some(e.text());
                    }
                    return;
                }
                if let Some(c) = self.create.as_mut()
                    && let Some(s) = c.slots.get_mut(k as usize)
                    && let Some(key) = key
                {
                    *s = create::Source::Cosigner(key);
                }
            }
            Action::CSlotLater(k) => {
                if let Some(c) = self.create.as_mut()
                    && let Some(s) = c.slots.get_mut(k as usize)
                {
                    *s = create::Source::Later;
                }
            }
            Action::CKeyOut(k) | Action::CKeyQr(k) => {
                let qr = matches!(action, Action::CKeyQr(_));
                match self.create_key_text(c, k) {
                    Ok((fp, text)) => {
                        let kind = self.create.as_ref().map_or("", |c| c.kind.name());
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
            Action::CSlotClear(k) => {
                if let Some(c) = self.create.as_mut()
                    && let Some(s) = c.slots.get_mut(k as usize)
                {
                    *s = create::Source::Empty;
                    if c.pass_slot == Some(k) {
                        c.close_pass();
                    }
                }
            }
            Action::CPassOpen(k) => {
                if let Some(c) = self.create.as_mut() {
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
                if let Some(c) = self.create.as_mut()
                    && c.pass_slot.is_some()
                    && f < 2
                {
                    c.pass_focus = Some(f);
                    c.pass_note = None;
                }
            }
            Action::CPassLock => self.create_pass_lock(),
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }
}
