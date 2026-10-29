//! Sending silent payments from a PSBT: BIP-374 proofs through
//! `faraday_bip::dleq`, BIP-375's fields and scripts through
//! `faraday_psbt::silent`.

use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
use faraday_bip::dleq;
use faraday_crypto::work::Compute;
use faraday_psbt::silent;
use faraday_psbt::v2::PsbtV2;
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{b32, bytes, flag, hex, opt_bytes, seckey, text, unhex};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

/// A point the request names; `INFINITY` is a point no `PublicKey` can
/// hold, so naming it is a refusal, as the BIP's vectors expect.
fn pubkey(req: &Value, key: &str) -> Result<PublicKey, String> {
    if text(req, key) == "INFINITY" {
        return Err(format!("{key} is the point at infinity"));
    }
    crate::json::pubkey(req, key)
}

/// The optional 32-byte message.
fn message(req: &Value) -> Result<Option<[u8; 32]>, String> {
    opt_bytes(req, "m")
        .map(|m| m.try_into().map_err(|_| "m is not 32 bytes".to_string()))
        .transpose()
}

/// The generator, secp256k1's own when the request names none.
fn generator(req: &Value) -> Result<PublicKey, String> {
    if req.get("g").is_some() {
        pubkey(req, "g")
    } else {
        PublicKey::from_slice(&unhex(
            "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
        ))
        .map_err(err)
    }
}

/// What a check found, as the request's answer.
fn report(r: &silent::Report) -> Value {
    json!({
        "complete": r.complete(),
        "outputs": r.outputs.iter().map(|o| json!({
            "index": o.index,
            "k": o.k,
            "stated": o.stated,
            "script": o.script.as_ref().map(|s| hex(s)),
        })).collect::<Vec<_>>(),
    })
}

/// Handles `sps.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let secp = Secp256k1::new();
    let c = Compute::unchecked();
    match name {
        "dleq_generate" => {
            let proof = dleq::generate_proof_with(
                &secp,
                &seckey(req, "a")?,
                &pubkey(req, "b")?,
                &b32(req, "r"),
                &generator(req)?,
                message(req)?.as_ref(),
            )
            .map_err(err)?;
            Ok(json!({"proof": hex(&proof)}))
        }
        "dleq_verify" => {
            let proof: [u8; 64] = bytes(req, "proof")
                .try_into()
                .map_err(|_| "proof is not 64 bytes".to_string())?;
            let valid = dleq::verify_proof_with(
                &secp,
                &pubkey(req, "a")?,
                &pubkey(req, "b")?,
                &pubkey(req, "c")?,
                &proof,
                &generator(req)?,
                message(req)?.as_ref(),
            );
            Ok(json!({"valid": valid}))
        }
        "check" => {
            let psbt = PsbtV2::parse_base64(&c, text(req, "psbt")).map_err(err)?;
            if flag(req, "inputs_only") {
                silent::check_inputs(&psbt).map_err(err)?;
                return Ok(json!({"inputs": true}));
            }
            Ok(report(&silent::check(&c, &secp, &psbt).map_err(err)?))
        }
        "shares" => {
            let mut psbt = PsbtV2::parse_base64(&c, text(req, "psbt")).map_err(err)?;
            let keys = req["keys"]
                .as_array()
                .expect("keys is a list")
                .iter()
                .map(|k| {
                    let key =
                        SecretKey::from_slice(&unhex(k[1].as_str().expect("key"))).map_err(err)?;
                    Ok((k[0].as_u64().expect("index") as usize, key))
                })
                .collect::<Result<Vec<_>, String>>()?;
            let refs: Vec<(usize, &SecretKey)> = keys.iter().map(|(i, k)| (*i, k)).collect();
            let r = silent::add_shares_for_keys(&c, &secp, &mut psbt, &refs, &b32(req, "aux"))
                .map_err(err)?;
            let mut out = report(&r);
            out["psbt"] = json!(psbt.to_base64());
            Ok(out)
        }
        _ => Err(format!("unknown sps op {name}")),
    }
}
