//! Electrum's wallet file, unencrypted, as Electrum 4.x and Sparrow
//! write it (`docs/INTEROP.md` §5, Electrum wallet files):
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
//! no origin, and its wallet is refused, a multisig by the key's name:
//! Faraday holds no key without the fingerprint and path it has under the
//! device's key (BIP-388; upstream's `policy::check`).
//!
//! A file whose keystores hold an `xprv`, a `seed` or imported private
//! keys is a secret ([`holds_secret`]); only its public keys are read.

use crate::caravan::short;
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

/// What a value is, without keeping it: a string says whether it has
/// anything but white space, an object or array whether it has a member;
/// nothing else of it is read. serde_json hands a string over as a
/// borrow of its own buffer, so no copy of a secret is made here.
#[derive(Default)]
struct Glance {
    string: bool,
    filled: bool,
}

impl<'de> serde::Deserialize<'de> for Glance {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = Glance;

            fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str("json")
            }

            fn visit_bool<E>(self, _: bool) -> Result<Glance, E> {
                Ok(Glance::default())
            }

            fn visit_i64<E>(self, _: i64) -> Result<Glance, E> {
                Ok(Glance::default())
            }

            fn visit_u64<E>(self, _: u64) -> Result<Glance, E> {
                Ok(Glance::default())
            }

            fn visit_f64<E>(self, _: f64) -> Result<Glance, E> {
                Ok(Glance::default())
            }

            fn visit_unit<E>(self) -> Result<Glance, E> {
                Ok(Glance::default())
            }

            fn visit_str<E>(self, v: &str) -> Result<Glance, E> {
                Ok(Glance {
                    string: true,
                    filled: !v.trim().is_empty(),
                })
            }

            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut a: A) -> Result<Glance, A::Error> {
                while a.next_element::<serde::de::IgnoredAny>()?.is_some() {}
                Ok(Glance::default())
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut a: A) -> Result<Glance, A::Error> {
                let mut filled = false;
                while a
                    .next_entry::<serde::de::IgnoredAny, serde::de::IgnoredAny>()?
                    .is_some()
                {
                    filled = true;
                }
                Ok(Glance {
                    string: false,
                    filled,
                })
            }
        }
        d.deserialize_any(V)
    }
}

/// A keystore, as far as [`text_holds_secret`] reads it: whether its
/// `xprv`, `seed` or `keypairs` is filled. A key named twice is an error.
struct Keystore(bool);

impl<'de> serde::Deserialize<'de> for Keystore {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = Keystore;

            fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str("keystore")
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut a: A,
            ) -> Result<Keystore, A::Error> {
                use serde::de::Error as _;
                let mut seen: [Option<Glance>; 3] = [None, None, None];
                while let Some(k) = a.next_key::<String>()? {
                    let at = match k.as_str() {
                        "xprv" => Some(0),
                        "seed" => Some(1),
                        "keypairs" => Some(2),
                        _ => None,
                    };
                    match at {
                        Some(i) => {
                            if seen[i].replace(a.next_value()?).is_some() {
                                return Err(A::Error::duplicate_field("secret"));
                            }
                        }
                        None => {
                            a.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }
                Ok(Keystore(seen.iter().flatten().any(|g| g.filled)))
            }

            // A keystore that is no object holds nothing to read.
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> Result<Keystore, A::Error> {
                while a.next_element::<serde::de::IgnoredAny>()?.is_some() {}
                Ok(Keystore(false))
            }

            fn visit_str<E>(self, _: &str) -> Result<Keystore, E> {
                Ok(Keystore(false))
            }

            fn visit_bool<E>(self, _: bool) -> Result<Keystore, E> {
                Ok(Keystore(false))
            }

            fn visit_i64<E>(self, _: i64) -> Result<Keystore, E> {
                Ok(Keystore(false))
            }

            fn visit_u64<E>(self, _: u64) -> Result<Keystore, E> {
                Ok(Keystore(false))
            }

            fn visit_f64<E>(self, _: f64) -> Result<Keystore, E> {
                Ok(Keystore(false))
            }

            fn visit_unit<E>(self) -> Result<Keystore, E> {
                Ok(Keystore(false))
            }
        }
        d.deserialize_any(V)
    }
}

/// Whether a text is an Electrum file ([`looks_like`]) that holds a
/// secret ([`holds_secret`]), read without keeping any value: the
/// keystores' `xprv`, `seed` and `keypairs` are looked at as they go by
/// and dropped. A text that is not JSON, or names one of those twice,
/// holds none here (the import refuses it when it reads the file).
/// Costs a pass over the text.
pub fn text_holds_secret(text: &str) -> bool {
    struct File {
        electrum: bool,
        secret: bool,
    }
    impl<'de> serde::Deserialize<'de> for File {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            struct V;
            impl<'de> serde::de::Visitor<'de> for V {
                type Value = File;

                fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.write_str("electrum")
                }

                fn visit_map<A: serde::de::MapAccess<'de>>(
                    self,
                    mut a: A,
                ) -> Result<File, A::Error> {
                    use serde::de::Error as _;
                    let mut electrum = false;
                    let mut single: Option<Keystore> = None;
                    let mut multi: [Option<Keystore>; MAX_KEYS] = core::array::from_fn(|_| None);
                    while let Some(k) = a.next_key::<String>()? {
                        if k == "wallet_type" {
                            electrum |= a.next_value::<Glance>()?.string;
                        } else if k == "keystore" {
                            if single.replace(a.next_value()?).is_some() {
                                return Err(A::Error::duplicate_field("keystore"));
                            }
                        } else if let Some(i) = k
                            .strip_prefix('x')
                            .and_then(|r| r.strip_suffix('/'))
                            .and_then(|n| n.parse::<usize>().ok())
                            .filter(|i| (1..=MAX_KEYS).contains(i) && k == format!("x{i}/"))
                        {
                            if multi[i - 1].replace(a.next_value()?).is_some() {
                                return Err(A::Error::duplicate_field("x/"));
                            }
                        } else {
                            a.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                    // `keystore` for one key, else `x1/`, `x2/` … while
                    // each is there ([`keystores`]).
                    let secret = match single {
                        Some(k) => k.0,
                        None => multi.iter().map_while(Option::as_ref).any(|k| k.0),
                    };
                    Ok(File { electrum, secret })
                }
            }
            d.deserialize_map(V)
        }
    }
    let body = text.trim_start_matches('\u{feff}');
    serde_json::from_str::<File>(body).is_ok_and(|f| f.electrum && f.secret)
}

/// A keystore's master fingerprint as eight hex digits: `root_fingerprint`,
/// or `ckcc_xfp` read little-endian (1130956047 is `0f056943`).
fn fingerprint(k: &Value) -> Result<Option<String>, String> {
    let bad = || fill(EN.wallet_electrum, &[EN.wallet_electrum_fingerprint]);
    let root = match k.get("root_fingerprint").and_then(Value::as_str) {
        Some(f) => {
            let f = f.trim();
            if f.len() != 8 || !f.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(bad());
            }
            Some(f.to_ascii_lowercase())
        }
        None => None,
    };
    let xfp = match k.get("ckcc_xfp") {
        None | Some(Value::Null) => None,
        Some(x) => {
            let n = x
                .as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(bad)?;
            Some(String::from(&*faraday_crypto::hex::encode_string(
                &n.to_le_bytes(),
            )))
        }
    };
    // Two fingerprints that disagree are an ambiguous input.
    match (root, xfp) {
        (Some(a), Some(b)) if a != b => Err(bad()),
        (a, b) => Ok(a.or(b)),
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
        .map_err(|m| String::from(&*crate::wallet::mismatch_text(&m)))?;
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
        keys.push(format!("{}/<0;1>/*", read.text.as_str()));
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
        let fp: String = String::from(&*faraday_crypto::hex::encode_string(&k.fingerprint()?.0));
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
