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
use faraday_bip::policy::{WalletPolicy, parse_script_descriptor};
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
