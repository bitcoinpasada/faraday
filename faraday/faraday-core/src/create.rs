//! Creating a wallet (`docs/WALLETS.md` §5): which kind, the quorum, a key
//! for every slot, and the descriptor they make. The keys can be loaded
//! here, made here from fresh entropy, or arrive from a cosigner as an
//! account key in a file. Nothing secret leaves the session.

use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::{MasterKey, MultisigScriptType, ScriptType};
use osk_bip::policy::{PolicyKey, WalletPolicy};

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
            NewKind::NativeSegwit => "Single key · native SegWit",
            NewKind::Taproot => "Single key · Taproot",
            NewKind::NestedSegwit => "Single key · nested SegWit",
            NewKind::Legacy => "Single key · legacy",
            NewKind::Multi => "Multisig · native SegWit",
            NewKind::MultiNested => "Multisig · nested SegWit",
            NewKind::MultiLegacy => "Multisig · legacy",
            NewKind::TapMulti => "Multisig · Taproot",
            NewKind::Threshold => "Threshold · FROST",
            NewKind::MuSig => "All keys · MuSig2",
        }
    }

    /// One line on what it is for.
    pub fn line(self) -> &'static str {
        match self {
            NewKind::NativeSegwit => "bc1q addresses; what most wallets make",
            NewKind::Taproot => "bc1p addresses; the smallest signatures",
            NewKind::NestedSegwit => "3… addresses, for services that cannot pay bc1",
            NewKind::Legacy => "1… addresses, for old software only",
            NewKind::Multi => "Several keys, a quorum signs; Sparrow, Coldcard, SeedSigner",
            NewKind::MultiNested => "Multisig with 3… addresses",
            NewKind::MultiLegacy => "Multisig with 3… addresses, old style; avoid unless required",
            NewKind::TapMulti => "Multisig on Taproot, one script leaf; Sparrow and Nunchuk",
            NewKind::Threshold => "Any m of n shares sign one Taproot key; single-key on chain",
            NewKind::MuSig => "Every key signs together, one Taproot key; single-key on chain",
        }
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
    pub fn key_text(self, master: &MasterKey) -> Result<String, String> {
        let (xpub, fp, path) = match self {
            NewKind::Threshold => return Err("A share has no account xpub".to_string()),
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
                let a = master.account_xpub(t, 0).map_err(|e| e.to_string())?;
                (*a.xpub(), a.master_fingerprint(), a.path().clone())
            }
            NewKind::Multi | NewKind::MultiNested => {
                let t = if self == NewKind::Multi {
                    MultisigScriptType::NativeSegwit
                } else {
                    MultisigScriptType::NestedSegwit
                };
                let a = master
                    .multisig_account_xpub(t, 0)
                    .map_err(|e| e.to_string())?;
                (*a.xpub(), a.master_fingerprint(), a.path().clone())
            }
            NewKind::MultiLegacy | NewKind::TapMulti => {
                let path: osk_bip::bitcoin::bip32::DerivationPath = self
                    .path(master.network())
                    .parse()
                    .map_err(|e| format!("{e}"))?;
                let d = master.derive(&path);
                (d.to_xpub(), master.fingerprint(), path)
            }
        };
        let shown = format!("{path}")
            .trim_start_matches("m/")
            .replace('\'', "h");
        Ok(format!("[{fp}/{shown}]{xpub}"))
    }

    /// A `wsh` multisig key as the `ur:crypto-account` SeedSigner shows
    /// for one; `None` for any other kind.
    pub fn account_ur(self, master: &MasterKey) -> Option<String> {
        if self != NewKind::Multi {
            return None;
        }
        let a = master
            .multisig_account_xpub(MultisigScriptType::NativeSegwit, 0)
            .ok()?;
        let path: Vec<_> = a.path().into_iter().copied().collect();
        let cbor =
            faraday_qr::registry::account_cbor(a.master_fingerprint().0, &path, a.xpub(), false);
        Some(faraday_qr::ur::single("crypto-account", &cbor))
    }

    /// A key's BIP 129 key record, signed by the account key itself:
    /// what Coldcard, Sparrow, Nunchuk and Keystone take from a signer.
    /// For the multisig kinds BIP 129 covers (`wsh` and `sh(wsh)`).
    pub fn bsms_record(self, master: &MasterKey, description: &str) -> Option<String> {
        let script = match self {
            NewKind::Multi => MultisigScriptType::NativeSegwit,
            NewKind::MultiNested => MultisigScriptType::NestedSegwit,
            _ => return None,
        };
        let record = master
            .with_multisig_account_secret(script, 0, |secret, view| {
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

/// `[fingerprint/path]xpub` of a master key at `path`, written
/// `m/48'/0'/0'/2'` or `48h/0h/0h/2h`: what [`NewKind::key_text`] gives
/// at a kind's own path, at any other.
pub fn key_text_at(master: &MasterKey, path: &str) -> Result<String, String> {
    let path = normal_path(path).ok_or_else(|| format!("Not a derivation path: {path}"))?;
    let parsed: osk_bip::bitcoin::bip32::DerivationPath =
        path.parse().map_err(|e| format!("{e}"))?;
    let d = master.derive(&parsed);
    let shown = path
        .trim_start_matches('m')
        .trim_start_matches('/')
        .replace('\'', "h");
    if shown.is_empty() {
        return Ok(format!("[{}]{}", master.fingerprint(), d.to_xpub()));
    }
    Ok(format!("[{}/{shown}]{}", master.fingerprint(), d.to_xpub()))
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
/// generic export, one object per account; or `[fingerprint/path]xpub` on
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
        return coldcard_keys(text);
    }
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| PolicyKey::parse(l).ok().map(|k| k.key_text()))
        .collect()
}

/// The account keys in a Coldcard JSON file, each rooted at the file's
/// top-level `xfp`, the master's. Coldcard writes the flat multisig keys
/// in SLIP-132 form (`Zpub`, `Ypub`, `Vpub`, `Upub`), which a descriptor
/// takes as `xpub` or `tpub`.
fn coldcard_keys(text: &str) -> Vec<String> {
    use crate::wallet::{json_object, json_string};
    let Some(master) = json_string(text, "xfp") else {
        return Vec::new();
    };
    let master = master.to_ascii_lowercase();
    let key = |deriv: &str, xpub: &str| {
        let path = deriv.trim_start_matches('m').trim_start_matches('/');
        let xpub = crate::wallet::plain_xpub(xpub)?;
        PolicyKey::parse(&format!("[{master}/{path}]{xpub}"))
            .ok()
            .map(|k| k.key_text())
    };
    let mut keys = Vec::new();
    // The multisig key file: flat fields. Coldcard has written the nested
    // one as `p2wsh_p2sh` and as `p2sh_p2wsh`.
    for field in ["p2wsh", "p2sh_p2wsh", "p2wsh_p2sh", "p2sh"] {
        let flat = text.find(&format!("\"{field}\"")).is_some_and(|at| {
            // A field at the top level, not inside an account object.
            text[..at].matches('{').count() == text[..at].matches('}').count() + 1
        });
        if !flat {
            continue;
        }
        if let (Some(d), Some(x)) = (
            json_string(text, &format!("{field}_deriv")),
            json_string(text, field),
        ) && let Some(k) = key(&d, &x)
        {
            keys.push(k);
        }
    }
    // The generic export: one object per account.
    for field in [
        "bip48_2", "bip48_1", "bip45", "bip86", "bip84", "bip49", "bip44",
    ] {
        if let Some(account) = json_object(text, field)
            && let (Some(d), Some(x)) =
                (json_string(account, "deriv"), json_string(account, "xpub"))
            && let Some(k) = key(&d, &x)
        {
            keys.push(k);
        }
    }
    keys
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
            (keys.len() == 1 && !text.trim_start().starts_with('{')).then(|| keys[0].clone())
        })
}

/// Builds and reads back the wallet a creation describes.
pub fn build(kind: NewKind, m: usize, keys: &[String]) -> Result<WalletPolicy, String> {
    WalletPolicy::parse_any(&kind.descriptor(m, keys)).map_err(|e| e.to_string())
}
