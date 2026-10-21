//! Caravan's multisig wallet config, which Unchained also writes and
//! reads, and Sparrow and BlueWallet read (`docs/INTEROP.md` §5 I5):
//!
//! ```text
//! {"name", "addressType": "P2WSH" | "P2SH-P2WSH" | "P2SH", "network",
//!  "client": {"type": "public"}, "quorum": {"requiredSigners", "totalSigners"},
//!  "extendedPublicKeys": [{"name", "bip32Path", "xpub", "xfp", "method"?}],
//!  "startingAddressIndex": 0}
//! ```
//!
//! Read as `sortedmulti` (BIP 67), as Caravan builds its addresses.
//! Fields not listed are ignored. A key whose `bip32Path` is `Unknown`, or
//! with no `xfp`, has no origin, and is refused by name: a multisig
//! `WalletPolicy` holds no key without one (BIP-388; upstream's
//! `policy::check`).

use crate::slip132::{self, Script};
use crate::wallet::network_name;
use faraday_strings::{EN, fill};
use osk_bip::json::{self, Value};
use osk_bip::keys::Network;
use osk_bip::policy::{Template, WalletPolicy, Wrapper};

/// Whether JSON is a Caravan config: it has `extendedPublicKeys` and
/// `quorum`.
pub fn looks_like(v: &Value) -> bool {
    v.get("extendedPublicKeys").is_some() && v.get("quorum").is_some()
}

/// The descriptor a Caravan config describes, every key's version checked
/// against `addressType` and the purpose of its path (`slip132`), and its
/// network against `network`; the refusal's text otherwise. Costs a
/// base58 decode per key.
pub fn descriptor(v: &Value) -> Result<String, String> {
    let bad = |why: &str| fill(EN.wallet_caravan, &[why]);
    let (wrap, around) = match v.get("addressType").and_then(Value::as_str) {
        Some(t) if t.eq_ignore_ascii_case("P2WSH") => ("wsh(K)", Script::NativeMultisig),
        Some(t) if t.eq_ignore_ascii_case("P2SH-P2WSH") => ("sh(wsh(K))", Script::NestedMultisig),
        Some(t) if t.eq_ignore_ascii_case("P2SH") => ("sh(K)", Script::LegacyMultisig),
        _ => return Err(bad(EN.wallet_caravan_address_type)),
    };
    let test = match v.get("network").and_then(Value::as_str) {
        Some("mainnet") => false,
        Some("testnet" | "signet" | "regtest") => true,
        _ => return Err(bad(EN.wallet_caravan_network)),
    };
    if v.get("startingAddressIndex")
        .is_some_and(|i| i.as_u64() != Some(0))
    {
        return Err(bad(EN.wallet_caravan_starting_index));
    }
    let keys = v
        .get("extendedPublicKeys")
        .and_then(Value::as_array)
        .ok_or_else(|| bad(EN.wallet_caravan_keys))?;
    let quorum = v.get("quorum");
    let m = quorum
        .and_then(|q| q.get("requiredSigners"))
        .and_then(Value::as_u64);
    let n = quorum
        .and_then(|q| q.get("totalSigners"))
        .and_then(Value::as_u64);
    // A quorum whose total is not the keys listed would build another
    // wallet than the one the file was written from (Sparrow refuses it
    // too).
    let (Some(m), Some(n)) = (m, n) else {
        return Err(bad(EN.wallet_caravan_quorum));
    };
    if m == 0 || m > n || usize::try_from(n).ok() != Some(keys.len()) {
        return Err(bad(EN.wallet_caravan_quorum));
    }
    let mut written = Vec::with_capacity(keys.len());
    for k in keys {
        let xpub = k
            .get("xpub")
            .and_then(Value::as_str)
            .ok_or_else(|| bad(EN.wallet_caravan_keys))?;
        let path = k.get("bip32Path").and_then(Value::as_str);
        let xfp = k.get("xfp").and_then(Value::as_str);
        let origin = match (path, xfp) {
            (Some(p), Some(f)) if !p.eq_ignore_ascii_case("unknown") => {
                let f = f.trim();
                if f.len() != 8 || !f.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err(bad(EN.wallet_caravan_xfp));
                }
                let p = p.trim().trim_start_matches('m').trim_start_matches('/');
                Some(if p.is_empty() {
                    f.to_string()
                } else {
                    format!("{f}/{p}")
                })
            }
            _ => None,
        };
        let Some(o) = origin else {
            return Err(bad(&fill(EN.wallet_caravan_no_origin, &[&short(xpub)])));
        };
        let line = format!("[{o}]{xpub}");
        let read = slip132::key_line(&line)
            .ok_or_else(|| bad(EN.wallet_caravan_keys))?
            .map_err(|m| m.text())?;
        if let Some(named) = read.named
            && !slip132::agrees(Some(named), around)
        {
            let m = slip132::Mismatch {
                key: xpub.to_string(),
                named,
                around,
            };
            return Err(m.text());
        }
        // The key's network against the file's.
        let key = read.text.rsplit(']').next().unwrap_or("");
        if key.starts_with("tpub") != test {
            let on = if test {
                Network::Mainnet
            } else {
                Network::Testnet
            };
            let says = v.get("network").and_then(Value::as_str).unwrap_or("");
            return Err(bad(&fill(
                EN.wallet_caravan_key_network,
                &[&short(xpub), network_name(on), says],
            )));
        }
        written.push(format!("{}/<0;1>/*", read.text));
    }
    Ok(wrap.replace('K', &format!("sortedmulti({m},{})", written.join(","))))
}

/// A key shortened for a refusal.
fn short(k: &str) -> String {
    if k.len() > 16 && k.is_ascii() {
        format!("{}…{}", &k[..8], &k[k.len() - 6..])
    } else {
        k.to_string()
    }
}

/// Whether [`file`] writes a config for the wallet: a `sortedmulti`
/// wallet in `wsh`, `sh(wsh)` or `sh` whose keys all have origins.
pub fn writes(policy: &WalletPolicy) -> bool {
    matches!(policy.template(), Template::Multi { sorted: true, .. })
        && policy.keys().iter().all(|k| k.fingerprint().is_some())
}

/// The wallet as a Caravan config, in the shape Sparrow writes it, each
/// key named by its fingerprint; `None` for a wallet Caravan cannot hold
/// ([`writes`]).
pub fn file(name: &str, policy: &WalletPolicy, network: Network) -> Option<String> {
    let Template::Multi {
        threshold,
        sorted: true,
        wrapper,
    } = policy.template()
    else {
        return None;
    };
    let address_type = match wrapper {
        Wrapper::Wsh => "P2WSH",
        Wrapper::ShWsh => "P2SH-P2WSH",
        Wrapper::Sh => "P2SH",
    };
    let net = match network {
        Network::Mainnet => "mainnet",
        Network::Testnet => "testnet",
        Network::Signet => "signet",
        Network::Regtest => "regtest",
    };
    let mut keys = Vec::new();
    for k in policy.keys() {
        let fp: String = k
            .fingerprint()?
            .0
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let path = k.path()?.to_string();
        let path = format!("m/{}", path.trim_start_matches('m').trim_start_matches('/'));
        let path = path.trim_end_matches('/').replace('h', "'");
        keys.push(format!(
            "    {{\n      \"name\": {},\n      \"bip32Path\": {},\n      \"xpub\": {},\n      \"xfp\": {}\n    }}",
            json::quote(&fp),
            json::quote(&path),
            json::quote(&k.xpub().to_string()),
            json::quote(&fp),
        ));
    }
    Some(format!(
        "{{\n  \"name\": {},\n  \"addressType\": \"{address_type}\",\n  \"network\": \"{net}\",\n  \"client\": {{\n    \"type\": \"public\"\n  }},\n  \"quorum\": {{\n    \"requiredSigners\": {threshold},\n    \"totalSigners\": {}\n  }},\n  \"extendedPublicKeys\": [\n{}\n  ],\n  \"startingAddressIndex\": 0\n}}\n",
        json::quote(name),
        policy.keys().len(),
        keys.join(",\n"),
    ))
}
