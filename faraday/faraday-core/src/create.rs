//! Creating a wallet (`docs/WALLETS.md` §5): which kind, the quorum, a key
//! for every slot, and the descriptor they make. The keys can be loaded
//! here, made here from fresh entropy, or arrive from a cosigner as an
//! account key in a file. Nothing secret leaves the session.

use crate::strings::EN;
use crate::strings::fill;
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::{MasterKey, MultisigScriptType, ScriptType};
use osk_bip::policy::{PolicyKey, WalletPolicy};
use osk_bip::slip132;
use osk_crypto::work::Compute;

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
    /// FROST: any m of n shares sign one Taproot key, dealt here
    /// (`docs/PLANNING.md` §16.103).
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
        use osk_bip::policy::{Template, Wrapper};
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
    /// as OpenSigner deals (`docs/PLANNING.md` §16.103), fifteen keys
    /// for a multisig.
    pub fn max_keys(self) -> usize {
        if self.threshold() { 5 } else { 15 }
    }

    /// The path a key of this kind is taken at, from its master, on
    /// `network`.
    pub fn path(self, network: osk_bip::keys::Network) -> String {
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
    pub fn path_at(self, network: osk_bip::keys::Network, account: u32) -> String {
        let path = self.path(network);
        let mut steps: Vec<String> = path.split('/').map(str::to_string).collect();
        if steps.len() >= 4 {
            steps[3] = format!("{account}'");
        }
        steps.join("/")
    }

    /// `[fingerprint/path]xpub` of a master key, at this kind's path.
    /// Costs a key derivation.
    pub fn key_text(self, c: &Compute, master: &MasterKey) -> Result<String, String> {
        self.key_text_account(c, master, 0)
    }

    /// Whether the kind's path has an account to choose: every kind but
    /// `m/45'` and the shares dealt here.
    pub fn has_account(self) -> bool {
        !matches!(self, NewKind::MultiLegacy | NewKind::Threshold)
    }

    /// `[fingerprint/path]xpub` of a master key, at this kind's path with
    /// account `account` in place of 0 (`docs/NEW-WALLET.md` §16.2).
    /// Costs a key derivation.
    pub fn key_text_account(
        self,
        c: &Compute,
        master: &MasterKey,
        account: u32,
    ) -> Result<String, String> {
        let (xpub, fp, path) = match self {
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
                let a = master
                    .account_xpub(c, t, account)
                    .map_err(|e| e.to_string())?;
                (*a.xpub(), a.master_fingerprint(), a.path().clone())
            }
            NewKind::Multi | NewKind::MultiNested => {
                let t = if self == NewKind::Multi {
                    MultisigScriptType::NativeSegwit
                } else {
                    MultisigScriptType::NestedSegwit
                };
                let a = master
                    .multisig_account_xpub(c, t, account)
                    .map_err(|e| e.to_string())?;
                (*a.xpub(), a.master_fingerprint(), a.path().clone())
            }
            NewKind::MultiLegacy | NewKind::TapMulti => {
                let path: osk_bip::bitcoin::bip32::DerivationPath = self
                    .path_at(master.network(), account)
                    .parse()
                    .map_err(|e| format!("{e}"))?;
                let d = master.derive(c, &path);
                (d.to_xpub(c), master.fingerprint(), path)
            }
        };
        let shown = format!("{path}")
            .trim_start_matches("m/")
            .replace('\'', "h");
        Ok(format!("[{fp}/{shown}]{xpub}"))
    }

    /// A `wsh` multisig key as the `ur:crypto-account` SeedSigner shows
    /// for one; `None` for any other kind. Costs a key derivation.
    pub fn account_ur(self, c: &Compute, master: &MasterKey) -> Option<String> {
        self.account_ur_at(c, master, 0)
    }

    /// [`NewKind::account_ur`] at account `account`. Costs a key
    /// derivation.
    pub fn account_ur_at(self, c: &Compute, master: &MasterKey, account: u32) -> Option<String> {
        if self != NewKind::Multi {
            return None;
        }
        let a = master
            .multisig_account_xpub(c, MultisigScriptType::NativeSegwit, account)
            .ok()?;
        let path: Vec<_> = a.path().into_iter().copied().collect();
        let cbor =
            faraday_qr::registry::account_cbor(a.master_fingerprint().0, &path, a.xpub(), false);
        Some(osk_codec::ur::encode_single("crypto-account", &cbor))
    }

    /// A key's BIP 129 key record, signed by the account key itself:
    /// what Coldcard, Sparrow, Nunchuk and Keystone take from a signer.
    /// For the multisig kinds BIP 129 covers (`wsh` and `sh(wsh)`), at
    /// account `account` of the kind's path. Costs a key derivation and a
    /// signature.
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
        let record = master
            .with_multisig_account_secret(c, script, account, |secret, view| {
                osk_bip::bsms::signer_record(master.secp(), view, secret, "00", description)
            })
            .ok()?
            .ok()?;
        Some(record.to_text())
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
    use osk_bip::bitcoin::bip32::ChildNumber;
    policy
        .keys()
        .first()
        .and_then(|k| k.path())
        .and_then(|p| p.into_iter().nth(2).copied())
        .map_or(0, |c| match c {
            ChildNumber::Hardened { index } | ChildNumber::Normal { index } => index,
        })
}

/// `[fingerprint/path]xpub` of a master key at `path`, written
/// `m/48'/0'/0'/2'` or `48h/0h/0h/2h`: what [`NewKind::key_text`] gives
/// at a kind's own path, at any other.
pub fn key_text_at(c: &Compute, master: &MasterKey, path: &str) -> Result<String, String> {
    let path = normal_path(path).ok_or_else(|| fill(EN.create_not_a_derivation, &[path]))?;
    let parsed: osk_bip::bitcoin::bip32::DerivationPath =
        path.parse().map_err(|e| format!("{e}"))?;
    let d = master.derive(c, &parsed);
    let shown = path
        .trim_start_matches('m')
        .trim_start_matches('/')
        .replace('\'', "h");
    if shown.is_empty() {
        return Ok(format!("[{}]{}", master.fingerprint(), d.to_xpub(c)));
    }
    Ok(format!(
        "[{}/{shown}]{}",
        master.fingerprint(),
        d.to_xpub(c)
    ))
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
/// (`docs/NEW-WALLET.md` §16.2): its name, kind and account, and, by
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

/// A new 24-word seed from 32 bytes of entropy.
pub fn new_words(entropy: &[u8; 32]) -> Result<String, String> {
    let m = Mnemonic::from_entropy(Language::English, entropy).map_err(|e| e.to_string())?;
    let lang = Language::English;
    Ok(m.indices()
        .iter()
        .map(|&i| lang.word(i))
        .collect::<Vec<_>>()
        .join(" "))
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
    if osk_bip::bsms::is_signer_record(text) {
        return osk_bip::bsms::SignerRecord::parse(text)
            .ok()
            .filter(|r| r.verify().is_ok())
            .map(|r| r.key.key_text())
            .into_iter()
            .collect();
    }
    if text.trim_start().starts_with('{') {
        // A single-key file (Cobo, Keystone, Coldcard's for Wasabi): its
        // one key.
        if let Ok(v) = osk_bip::json::parse(text)
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
        return osk_bip::coldcard::cosigner_keys(text);
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
    let want = shape(&kind.path(osk_bip::keys::Network::Mainnet))?;
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

/// Builds and reads back the wallet a creation describes.
pub fn build(kind: NewKind, m: usize, keys: &[String]) -> Result<WalletPolicy, String> {
    WalletPolicy::parse_any(&kind.descriptor(m, keys)).map_err(|e| e.to_string())
}

/// A wallet's derivation paths as Make a new wallet's Check shows them
/// (`docs/NEW-WALLET.md` §17): the one path when every key has it, else
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
                format!("{} {p}", crate::backup::fp_of(fp))
            })
            .collect::<Vec<_>>()
            .join(" · "),
    }
}
