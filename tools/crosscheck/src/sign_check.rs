//! The signature check of a PSBT: `faraday_psbt::verify::verify_signatures`
//! (each signature in a partial-signature field or a final scriptSig or
//! witness, checked against its sighash) and
//! `faraday_psbt::verify::repeated_nonces` (one `r` or `R` under any keys
//! in more than one place), as the Spend screen's signature check and the
//! nonce check call them. No key is held, so no signature is "ours".

use faraday_bip::keys::Network;
use faraday_crypto::work::Compute;
use faraday_psbt::verify::{
    Verdict, final_invalid, not_evaluated, repeated_nonces, verify_signatures, witness_mismatch,
};
use faraday_psbt::{Context, Psbt, SigKind};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, hex, num, text};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

fn kind(k: SigKind) -> &'static str {
    match k {
        SigKind::Ecdsa => "ecdsa",
        SigKind::Schnorr => "schnorr",
    }
}

/// Handles `sign-check.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let c = Compute::unchecked();
    match name {
        "verify" | "reuse" => {
            let network = Network::from_name(text(req, "network")).expect("a network name");
            let ctx = Context {
                network,
                keys: &[],
                wallets: &[],
                musig_session: None,
            };
            let psbt = Psbt::parse_bytes(&c, &bytes(req, "psbt")).map_err(err)?;
            if name == "reuse" {
                let reused: Vec<Value> = repeated_nonces(&psbt, &ctx)
                    .iter()
                    .map(|r| {
                        let mut keys: Vec<String> = r.keys.iter().map(|k| k.hex()).collect();
                        keys.sort();
                        json!({
                            "kind": kind(r.kind),
                            "keys": keys,
                            "nonce": r.nonce,
                            "inputs": r.inputs,
                        })
                    })
                    .collect();
                return Ok(json!({"count": reused.len(), "reused": reused}));
            }
            // "checked" comes before "signatures" in the answer, which is
            // the order the self-test's wrong answer reaches: a verdict.
            let inputs: Vec<Value> = verify_signatures(&psbt, &ctx)
                .iter()
                .map(|input| {
                    let verdicts: Vec<String> = input
                        .signatures
                        .iter()
                        .map(|s| match s.verdict {
                            Verdict::Valid => "valid".to_string(),
                            Verdict::Invalid => "invalid".to_string(),
                            Verdict::Unchecked(why) => format!("unchecked {why}"),
                        })
                        .collect();
                    let signatures: Vec<Value> = input
                        .signatures
                        .iter()
                        .map(|s| {
                            json!({
                                "kind": kind(s.kind),
                                "key": s.key.hex(),
                                "signature": hex(&s.bytes),
                                "finalized": s.finalized,
                                "leaf": s.leaf.map(|l| format!("{l}")),
                            })
                        })
                        .collect();
                    json!({"checked": verdicts, "signatures": signatures})
                })
                .collect();
            Ok(json!({"inputs": inputs}))
        }
        // The verdict on input `index`'s final fields, as the Spend screen
        // reads it: "mismatch" (`witness_mismatch`), else "invalid"
        // (`final_invalid`), else "unjudged" (`not_evaluated`), else "ok".
        "input" => {
            let psbt = Psbt::parse_bytes(&c, &bytes(req, "psbt")).map_err(err)?;
            let index = usize::try_from(num(req, "index")).map_err(err)?;
            let inner = psbt.inner();
            let verdict = if witness_mismatch(inner, index) {
                "mismatch"
            } else if final_invalid(inner, index) {
                "invalid"
            } else if not_evaluated(inner, index) {
                "unjudged"
            } else {
                "ok"
            };
            Ok(json!({ "verdict": verdict }))
        }
        _ => Err(format!("unknown sign-check op {name}")),
    }
}
