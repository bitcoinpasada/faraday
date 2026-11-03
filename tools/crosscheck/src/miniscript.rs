//! Miniscript as Faraday reads it: a wallet's descriptor
//! (`faraday_bip::policy::WalletPolicy`, which reads `wsh()`,
//! `sh(wsh())` and `tr()` with a tree through `miniscript` and refuses a
//! miniscript that is not sane) and the script it pays to at an index; a
//! bare miniscript read the same way
//! (`faraday_bip::policy::parse_script_descriptor`) with its script; and a
//! witness script read back as `faraday_psbt` reads a PSBT input's
//! (`miniscript`'s `parse_with_ext` under `ExtParams::sane` with raw
//! `pkh`), written out again.

use bitcoin::{PublicKey, ScriptBuf};
use faraday_bip::miniscript::{Descriptor, ExtParams, Miniscript, Segwitv0, Tap};
use faraday_bip::keys::ScriptType;
use faraday_bip::policy::{MiniscriptKind, Template, Wrapper, WalletPolicy, parse_script_descriptor};
use faraday_bip::recovery::{Form, Path, Recovery, RecoveryPolicy};
use faraday_bip::spend::Lock;
use faraday_crypto::work::Compute;
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, flag, hex, num, text};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

/// BIP-341's `H` as a compressed key, the internal key a bare tapscript
/// miniscript is put under to be read as a `tr()` leaf.
const NUMS: &str = "0250929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";

/// The kind of a wallet's descriptor in the words the cross-check shares
/// with btclib's classes: the functions from the outside in, the script
/// inside a `wsh()` or a `tr()` named by what it is.
fn kind(policy: &WalletPolicy) -> String {
    let wrap = |w: Wrapper, inner: &str| match w {
        Wrapper::Sh => format!("sh({inner})"),
        Wrapper::Wsh => format!("wsh({inner})"),
        Wrapper::ShWsh => format!("sh(wsh({inner}))"),
    };
    match policy.template() {
        Template::Multi {
            sorted, wrapper, ..
        } => wrap(wrapper, if sorted { "sortedmulti" } else { "multi" }),
        Template::Single { script } => match script {
            ScriptType::Legacy => "pkh",
            ScriptType::NestedSegwit => "sh(wpkh)",
            ScriptType::NativeSegwit => "wpkh",
            ScriptType::Taproot => "tr",
        }
        .into(),
        Template::MuSig => "tr(musig)".into(),
        Template::Silent => "sp".into(),
        Template::Miniscript { kind } => match kind {
            MiniscriptKind::Wsh => "wsh(miniscript)".into(),
            MiniscriptKind::ShWsh => "sh(wsh(miniscript))".into(),
        },
        Template::Tree => "tr(tree)".into(),
    }
}

/// A lock as a pair of its kind and its number.
fn lock(l: &Lock) -> Value {
    match l {
        Lock::Blocks(n) => json!(["blocks", n]),
        Lock::Intervals(n) => json!(["intervals", n]),
        Lock::Height(n) => json!(["height", n]),
        Lock::Time(n) => json!(["time", n]),
        Lock::Preimage => json!(["preimage", 0]),
    }
}

/// What a wallet is: its kind, the descriptor it writes (no checksum),
/// its keys as `[fingerprint/path]xpub` and the ways its coins move: the
/// keys (indices into that list) that sign and the locks that must have
/// passed.
pub(crate) fn inspect(policy: &WalletPolicy) -> Value {
    let keys: Vec<String> = policy.keys().iter().map(|k| k.key_text()).collect();
    let paths: Vec<Value> = policy
        .spend_paths()
        .iter()
        .map(|p| json!({"keys": p.keys, "locks": p.locks.iter().map(lock).collect::<Vec<_>>()}))
        .collect();
    json!({
        "type": kind(policy),
        "descriptor": policy.to_descriptor(),
        "keys": keys,
        "paths": paths,
    })
}

fn path_of(v: &Value) -> Path {
    let keys: Vec<&str> = v["keys"]
        .as_array()
        .expect("keys")
        .iter()
        .map(|k| k.as_str().expect("key text"))
        .collect();
    Path::multi(v["threshold"].as_u64().expect("threshold") as usize, &keys)
}

/// Handles `miniscript.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let c = Compute::unchecked();
    match name {
        // A wallet's descriptor and the script it pays to at one index
        // of one chain.
        "wallet" => {
            let policy = WalletPolicy::from_descriptor(&c, text(req, "descriptor")).map_err(err)?;
            let index = u32::try_from(num(req, "index")).map_err(err)?;
            let script = policy
                .script_at(&c, flag(req, "change"), index)
                .map_err(err)?;
            Ok(json!({"script_pubkey": hex(script.as_bytes())}))
        }
        // A wallet read, said as `inspect` says it.
        "inspect" => {
            let policy = WalletPolicy::from_descriptor(&c, text(req, "descriptor")).map_err(err)?;
            Ok(inspect(&policy))
        }
        // A recovery wallet built from its parts (`RecoveryPolicy`): what
        // it is, the script it pays at one index, and the parts read back
        // out of it.
        "recovery" => {
            let recovery: Vec<Recovery> = req["recovery"]
                .as_array()
                .expect("recovery")
                .iter()
                .map(|r| Recovery {
                    delay: u32::try_from(r["delay"].as_u64().expect("delay")).expect("delay"),
                    path: path_of(r),
                })
                .collect();
            let policy = RecoveryPolicy {
                primary: path_of(&req["primary"]),
                recovery,
            };
            let form = match text(req, "form") {
                "segwit" => Form::SegWit,
                "taproot" => Form::Taproot,
                other => return Err(format!("unknown form {other}")),
            };
            let wallet = policy.to_wallet_policy(&c, form).map_err(err)?;
            let index = u32::try_from(num(req, "index")).map_err(err)?;
            let script = wallet
                .script_at(&c, flag(req, "change"), index)
                .map_err(err)?;
            let back = RecoveryPolicy::from_wallet_policy(&wallet);
            let mut out = inspect(&wallet);
            out["script_pubkey"] = json!(hex(script.as_bytes()));
            out["back"] = match back {
                None => Value::Null,
                Some(b) => json!({
                    "primary": {"threshold": b.primary.threshold, "keys": b.primary.keys},
                    "recovery": b.recovery.iter().map(|r| json!({
                        "delay": r.delay, "threshold": r.path.threshold, "keys": r.path.keys,
                    })).collect::<Vec<_>>(),
                }),
            };
            Ok(out)
        }
        // A miniscript with raw keys: its P2WSH witness script, or its
        // tapscript leaf.
        "script" => {
            let ms = text(req, "ms");
            let script = match text(req, "ctx") {
                "wsh" => {
                    let d: Descriptor<PublicKey> =
                        parse_script_descriptor(&c, &format!("wsh({ms})")).map_err(err)?;
                    d.explicit_script().map_err(err)?
                }
                "tap" => {
                    let d: Descriptor<PublicKey> =
                        parse_script_descriptor(&c, &format!("tr({NUMS},{ms})")).map_err(err)?;
                    let Descriptor::Tr(tr) = d else {
                        return Err("not a tr()".into());
                    };
                    let mut leaves = tr.iter_scripts();
                    let (_, leaf) = leaves.next().ok_or("no leaf")?;
                    leaf.encode()
                }
                other => return Err(format!("unknown context {other}")),
            };
            Ok(json!({"script": hex(script.as_bytes())}))
        }
        // A script read back as a miniscript and written out again.
        "from_script" => {
            let script = ScriptBuf::from_bytes(bytes(req, "script"));
            let ext = ExtParams {
                raw_pkh: true,
                ..ExtParams::sane()
            };
            let again = match text(req, "ctx") {
                "wsh" => Miniscript::<PublicKey, Segwitv0>::parse_with_ext(&script, &ext)
                    .map_err(err)?
                    .encode(),
                "tap" => Miniscript::<bitcoin::secp256k1::XOnlyPublicKey, Tap>::parse_with_ext(
                    &script, &ext,
                )
                .map_err(err)?
                .encode(),
                other => return Err(format!("unknown context {other}")),
            };
            Ok(json!({"script": hex(again.as_bytes())}))
        }
        _ => Err(format!("unknown miniscript op {name}")),
    }
}
