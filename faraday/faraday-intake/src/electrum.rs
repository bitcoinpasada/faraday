//! Electrum's wallet file, unencrypted, as Electrum 4.x and Sparrow
//! write it (`docs/INTEROP.md` §5 I6):
//!
//! ```text
//! {"keystore": {"type": "bip32" | "hardware", "xpub", "derivation",
//!               "root_fingerprint" | "ckcc_xfp", …},
//!  "wallet_type": "standard", "use_encryption": false, …}
//! {"x1/": {…}, "x2/": {…}, …, "wallet_type": "2of3", …}
//! ```
//!
//! Electrum names a wallet's script by its keys' SLIP-132 versions: one
//! key `xpub` is `pkh`, `ypub` `sh(wpkh)`, `zpub` `wpkh`; a multisig wallet
//! `xpub` is `sh`, `Ypub` `sh(wsh)`, `Zpub` `wsh`, always `sortedmulti`,
//! every key of one type. A key's fingerprint is `root_fingerprint` (hex)
//! or Coldcard's `ckcc_xfp`, a decimal number whose four bytes are the
//! fingerprint little-endian. A key with no fingerprint and derivation has
//! no origin: a single-key wallet loads without one (watch-only here); a
//! multisig wallet is refused by name, since a multisig `WalletPolicy`
//! holds no key without one (BIP-388; upstream's `policy::check`).
//!
//! A file whose keystores hold an `xprv`, a `seed` or imported private
//! keys is a secret ([`holds_secret`]); only its public keys are read.

use crate::single_key::path_h;
use faraday_bip::json::{self, Value};
use faraday_bip::policy::{Template, WalletPolicy, Wrapper};
use faraday_bip::slip132::{self, Script};
use faraday_strings::{EN, fill};

/// The most keys a multisig script holds, and so the most keystores read.
const MAX_KEYS: usize = 15;

/// The `seed_version` the file Faraday writes states: Sparrow's. Electrum
/// upgrades the file when it opens it.
pub const SEED_VERSION: u32 = 17;

/// Whether JSON is an Electrum wallet file: it has a `wallet_type` string.
pub fn looks_like(v: &Value) -> bool {
    v.get("wallet_type").and_then(Value::as_str).is_some()
}

/// Whether text is an Electrum wallet file encrypted whole: the base64 of
/// Electrum's `BIE1` envelope.
pub fn encrypted_whole(text: &str) -> bool {
    text.trim_start_matches('\u{feff}')
        .trim_start()
        .starts_with("QklFMQ")
}

/// The keystores of an Electrum file, in order: `keystore` for one key,
/// `x1/`, `x2/` … for a multisig wallet.
fn keystores(v: &Value) -> Vec<&Value> {
    if let Some(k) = v.get("keystore") {
        return vec![k];
    }
    (1..=MAX_KEYS)
        .map_while(|i| v.get(&format!("x{i}/")))
        .collect()
}

/// Whether an Electrum file holds a secret beside its public keys: a
/// keystore's `xprv` or `seed`, or imported private keys (`keypairs`).
pub fn holds_secret(v: &Value) -> bool {
    let filled = |x: Option<&Value>| match x {
        Some(Value::String(s)) => !s.trim().is_empty(),
        Some(Value::Object(m)) => !m.is_empty(),
        _ => false,
    };
    keystores(v)
        .iter()
        .any(|k| filled(k.get("xprv")) || filled(k.get("seed")) || filled(k.get("keypairs")))
}

/// A keystore's master fingerprint as eight hex digits: `root_fingerprint`,
/// or `ckcc_xfp` read little-endian (1130956047 is `0f056943`).
fn fingerprint(k: &Value) -> Result<Option<String>, String> {
    let bad = || fill(EN.wallet_electrum, &[EN.wallet_electrum_fingerprint]);
    if let Some(f) = k.get("root_fingerprint").and_then(Value::as_str) {
        let f = f.trim();
        if f.len() != 8 || !f.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(bad());
        }
        return Ok(Some(f.to_ascii_lowercase()));
    }
    match k.get("ckcc_xfp") {
        None | Some(Value::Null) => Ok(None),
        Some(x) => {
            let n = x
                .as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(bad)?;
            Ok(Some(
                n.to_le_bytes().iter().map(|b| format!("{b:02x}")).collect(),
            ))
        }
    }
}

/// A keystore's key as `[fingerprint/path]xpub` (or the key alone, with
/// no origin), its version BIP-32's own, with the type its version names.
/// Costs a base58 decode.
fn key(k: &Value) -> Result<(slip132::KeyLine, bool), String> {
    let bad = |why: &str| fill(EN.wallet_electrum, &[why]);
    match k.get("type").and_then(Value::as_str) {
        Some("bip32" | "hardware") => {}
        Some(other) => {
            return Err(bad(&fill(EN.wallet_electrum_keystore_type, &[other])));
        }
        None => return Err(bad(EN.wallet_electrum_no_key)),
    }
    let xpub = k
        .get("xpub")
        .and_then(Value::as_str)
        .ok_or_else(|| bad(EN.wallet_electrum_no_key))?;
    let fp = fingerprint(k)?;
    let derivation = k.get("derivation").and_then(Value::as_str);
    let origin = match (fp, derivation) {
        (Some(fp), Some(d)) => {
            let d = d.trim();
            let path = if matches!(d, "" | "m" | "m/" | "M" | "M/") {
                String::new()
            } else {
                path_h(d).ok_or_else(|| bad(EN.wallet_electrum_derivation))?
            };
            Some(if path.is_empty() {
                fp
            } else {
                format!("{fp}/{path}")
            })
        }
        _ => None,
    };
    let line = match &origin {
        Some(o) => format!("[{o}]{}", xpub.trim()),
        None => xpub.trim().to_string(),
    };
    let read = slip132::key_line(&line)
        .ok_or_else(|| bad(EN.wallet_electrum_no_key))?
        .map_err(|m| crate::wallet::mismatch_text(&m))?;
    Ok((read, origin.is_some()))
}

/// The descriptor an Electrum file describes; the refusal's text when it
/// is encrypted, of a kind not read, or its keys disagree. Costs a base58
/// decode per key.
pub fn descriptor(v: &Value) -> Result<String, String> {
    let bad = |why: &str| fill(EN.wallet_electrum, &[why]);
    if matches!(v.get("use_encryption"), Some(Value::Bool(true))) {
        return Err(EN.wallet_electrum_encrypted.to_string());
    }
    let kind = v.get("wallet_type").and_then(Value::as_str).unwrap_or("");
    if kind == "standard" {
        let k = v
            .get("keystore")
            .ok_or_else(|| bad(EN.wallet_electrum_no_key))?;
        let (read, _) = key(k)?;
        let wrap = match read.named {
            None => "pkh(K/<0;1>/*)",
            Some(Script::NestedSegwit) => "sh(wpkh(K/<0;1>/*))",
            Some(Script::NativeSegwit) => "wpkh(K/<0;1>/*)",
            Some(other) => {
                return Err(bad(&fill(
                    EN.wallet_electrum_key_type,
                    &[crate::wallet::script_name(other)],
                )));
            }
        };
        return Ok(wrap.replace('K', &read.text));
    }
    // `2of3`: two signatures of three keys.
    let quorum = kind.split_once("of").and_then(|(m, n)| {
        let all_digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
        (all_digits(m) && all_digits(n))
            .then(|| Some((m.parse::<usize>().ok()?, n.parse::<usize>().ok()?)))
            .flatten()
    });
    let Some((m, n)) = quorum else {
        return Err(bad(&fill(EN.wallet_electrum_wallet_type, &[kind])));
    };
    let stores = keystores(v);
    if m == 0 || m > n || n > MAX_KEYS || stores.len() != n {
        return Err(bad(EN.wallet_electrum_quorum));
    }
    let mut keys = Vec::with_capacity(n);
    let mut named = None;
    for k in stores {
        let (read, origin) = key(k)?;
        if !origin {
            let xpub = k.get("xpub").and_then(Value::as_str).unwrap_or("");
            return Err(bad(&fill(EN.wallet_electrum_no_origin, &[&short(xpub)])));
        }
        if keys.is_empty() {
            named = read.named;
        } else if read.named != named {
            return Err(bad(EN.wallet_electrum_mixed));
        }
        keys.push(format!("{}/<0;1>/*", read.text));
    }
    let wrap = match named {
        None => "sh(K)",
        Some(Script::NestedMultisig) => "sh(wsh(K))",
        Some(Script::NativeMultisig) => "wsh(K)",
        Some(other) => {
            return Err(bad(&fill(
                EN.wallet_electrum_key_type,
                &[crate::wallet::script_name(other)],
            )));
        }
    };
    Ok(wrap.replace('K', &format!("sortedmulti({m},{})", keys.join(","))))
}

/// A key shortened for a refusal.
fn short(k: &str) -> String {
    if k.len() > 16 && k.is_ascii() {
        format!("{}…{}", &k[..8], &k[k.len() - 6..])
    } else {
        k.to_string()
    }
}

/// The wallet type Electrum gives the wallet and the script type its keys'
/// versions name, when Electrum can hold it: `pkh`, `sh(wpkh)` or `wpkh`
/// of one key, `sortedmulti` in `sh`, `sh(wsh)` or `wsh`; every key with
/// its origin.
fn shape(policy: &WalletPolicy) -> Option<(String, Script)> {
    use faraday_bip::keys::ScriptType;
    if !policy
        .keys()
        .iter()
        .all(|k| k.fingerprint().is_some() && k.path().is_some())
    {
        return None;
    }
    match policy.template() {
        Template::Single { script } => {
            let s = match script {
                ScriptType::Legacy => Script::Legacy,
                ScriptType::NestedSegwit => Script::NestedSegwit,
                ScriptType::NativeSegwit => Script::NativeSegwit,
                ScriptType::Taproot => return None,
            };
            Some(("standard".to_string(), s))
        }
        Template::Multi {
            threshold,
            sorted: true,
            wrapper,
        } => {
            let s = match wrapper {
                Wrapper::Sh => Script::LegacyMultisig,
                Wrapper::ShWsh => Script::NestedMultisig,
                Wrapper::Wsh => Script::NativeMultisig,
            };
            let n = policy.keys().len();
            (n <= MAX_KEYS).then(|| (format!("{threshold}of{n}"), s))
        }
        _ => None,
    }
}

/// Whether [`file`] writes an Electrum file for the wallet.
pub fn writes(policy: &WalletPolicy) -> bool {
    shape(policy).is_some()
}

/// The wallet as an Electrum wallet file, in the shape Sparrow writes for
/// Electrum: each keystore `bip32` with its key in the SLIP-132 version
/// that names the wallet's script, no `xprv`, its derivation and
/// `root_fingerprint`. `None` for a wallet Electrum cannot hold
/// ([`writes`]). Costs a base58 decode and encode per key.
pub fn file(policy: &WalletPolicy) -> Option<String> {
    let (wallet_type, script) = shape(policy)?;
    let mut stores = Vec::new();
    for k in policy.keys() {
        let fp: String = k
            .fingerprint()?
            .0
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let path = k.path()?.to_string();
        let path = path.trim_start_matches('m').trim_start_matches('/');
        let derivation = if path.is_empty() {
            "m".to_string()
        } else {
            format!("m/{}", path.trim_end_matches('/').replace('h', "'"))
        };
        let xpub = slip132::encode(&k.xpub().to_string(), script)?;
        stores.push(format!(
            "{{\n        \"type\": \"bip32\",\n        \"xpub\": {},\n        \"xprv\": null,\n        \"derivation\": {},\n        \"root_fingerprint\": {},\n        \"label\": \"\"\n    }}",
            json::quote(&xpub),
            json::quote(&derivation),
            json::quote(&fp),
        ));
    }
    let mut out = String::from("{\n");
    if let [one] = &stores[..] {
        out.push_str(&format!("    \"keystore\": {one},\n"));
    } else {
        for (i, s) in stores.iter().enumerate() {
            out.push_str(&format!("    \"x{}/\": {s},\n", i + 1));
        }
    }
    out.push_str(&format!(
        "    \"seed_version\": {SEED_VERSION},\n    \"use_encryption\": false,\n    \"wallet_type\": {}\n}}\n",
        json::quote(&wallet_type)
    ));
    Some(out)
}
