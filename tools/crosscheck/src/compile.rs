//! The policy compiler as Faraday's screen runs it
//! (`faraday_bip::compile::compile`): a concrete policy and the tokens
//! standing for its keys in, the descriptor out with the keys it names,
//! the ways it can be spent and, when the descriptor is a wallet, the
//! script it pays to at an index of one chain.

use faraday_bip::compile::{PolicyScript, Refusal, compile};
use faraday_bip::spend::Lock;
use faraday_crypto::work::Compute;
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{flag, num, text};

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

/// Handles `compile.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let c = Compute::unchecked();
    match name {
        "compile" => {
            let script = match text(req, "script") {
                "segwit" => PolicyScript::Segwit,
                "taproot" => PolicyScript::Taproot,
                other => return Err(format!("unknown script {other}")),
            };
            let tokens: Vec<(String, String)> = req["keys"]
                .as_array()
                .expect("keys")
                .iter()
                .map(|pair| {
                    (
                        pair[0].as_str().expect("token").to_string(),
                        pair[1].as_str().expect("expression").to_string(),
                    )
                })
                .collect();
            let facts = compile(&c, text(req, "policy"), script, &tokens).map_err(|r| match r {
                Refusal::Empty => "empty".to_string(),
                Refusal::Policy(m) => format!("policy: {m}"),
                Refusal::Script(m) => format!("script: {m}"),
                Refusal::LoneKeyTaproot => "lone key under taproot".to_string(),
            })?;
            let at = match &facts.wallet {
                Some(wallet) => {
                    let index = u32::try_from(num(req, "index")).map_err(|e| format!("{e:?}"))?;
                    let script = wallet
                        .script_at(&c, flag(req, "change"), index)
                        .map_err(|e| format!("{e:?}"))?;
                    json!(crate::json::hex(script.as_bytes()))
                }
                None => Value::Null,
            };
            let paths: Vec<Value> = facts
                .paths
                .iter()
                .map(|p| json!({"keys": p.keys, "locks": p.locks.iter().map(lock).collect::<Vec<_>>()}))
                .collect();
            Ok(json!({
                "descriptor": facts.descriptor,
                "keys": facts.keys,
                "paths": paths,
                "script_pubkey": at,
            }))
        }
        _ => Err(format!("unknown compile op {name}")),
    }
}
