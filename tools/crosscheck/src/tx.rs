//! Transactions and signature hashes: a raw transaction read by
//! `faraday_psbt::transaction::read_raw` with its txid, wtxid and
//! serialisation, and the four signature hashes `faraday_psbt::sign`
//! asks `bitcoin`'s `SighashCache` for (legacy, BIP-143 for P2WPKH and
//! P2WSH, BIP-341's key path and script path).

use bitcoin::hashes::Hash;
use bitcoin::sighash::{Prevouts, SighashCache};
use bitcoin::taproot::{LeafVersion, TapLeafHash};
use bitcoin::{Amount, EcdsaSighashType, ScriptBuf, TapSighashType, TxOut, consensus};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, hex, num, text};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

fn index(req: &Value) -> Result<usize, String> {
    usize::try_from(num(req, "index")).map_err(err)
}

/// Handles `tx.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    match name {
        "read" => {
            let tx = faraday_psbt::transaction::read_raw(text(req, "raw").as_bytes())
                .ok_or("not a transaction")?;
            Ok(json!({
                "txid": tx.compute_txid().to_string(),
                "wtxid": tx.compute_wtxid().to_string(),
                "raw": hex(&consensus::serialize(&tx)),
                "weight": tx.weight().to_wu(),
            }))
        }
        "sighash" => sighash(req),
        _ => Err(format!("unknown tx op {name}")),
    }
}

/// A signature hash of the kind `mode` names.
fn sighash(req: &Value) -> Answer {
    let tx: bitcoin::Transaction = consensus::deserialize(&bytes(req, "tx")).map_err(err)?;
    let mut cache = SighashCache::new(&tx);
    let i = index(req)?;
    let ty = u32::try_from(num(req, "hash_type")).map_err(err)?;
    let hash: [u8; 32] = match text(req, "mode") {
        "legacy" => {
            let code = ScriptBuf::from_bytes(bytes(req, "script_code"));
            cache
                .legacy_signature_hash(i, &code, ty)
                .map_err(err)?
                .to_byte_array()
        }
        "p2wpkh" | "p2wsh" => {
            let script = ScriptBuf::from_bytes(bytes(req, "script_code"));
            let amount = Amount::from_sat(num(req, "amount"));
            let ty = EcdsaSighashType::from_standard(ty).map_err(err)?;
            if text(req, "mode") == "p2wpkh" {
                cache
                    .p2wpkh_signature_hash(i, &script, amount, ty)
                    .map_err(err)?
            } else {
                cache
                    .p2wsh_signature_hash(i, &script, amount, ty)
                    .map_err(err)?
            }
            .to_byte_array()
        }
        mode @ ("taproot_key" | "taproot_script") => {
            let prevouts: Vec<TxOut> = req["prevouts"]
                .as_array()
                .expect("prevouts")
                .iter()
                .map(|p| TxOut {
                    value: Amount::from_sat(p[0].as_u64().expect("an amount")),
                    script_pubkey: ScriptBuf::from_bytes(crate::json::unhex(
                        p[1].as_str().expect("a script"),
                    )),
                })
                .collect();
            let ty =
                TapSighashType::from_consensus_u8(u8::try_from(ty).map_err(err)?).map_err(err)?;
            // Under ANYONECANPAY only the input's own output is
            // committed to, as `faraday_psbt::sign` hands it on.
            let one;
            let prevouts = if (ty as u8) & 0x80 != 0 {
                one = prevouts.get(i).cloned().ok_or("no prevout for the input")?;
                Prevouts::One(i, one)
            } else {
                Prevouts::All(&prevouts)
            };
            if mode == "taproot_key" {
                cache
                    .taproot_key_spend_signature_hash(i, &prevouts, ty)
                    .map_err(err)?
                    .to_byte_array()
            } else {
                let leaf = ScriptBuf::from_bytes(bytes(req, "leaf"));
                let leaf_hash = TapLeafHash::from_script(&leaf, LeafVersion::TapScript);
                cache
                    .taproot_script_spend_signature_hash(i, &prevouts, leaf_hash, ty)
                    .map_err(err)?
                    .to_byte_array()
            }
        }
        other => return Err(format!("unknown sighash mode {other}")),
    };
    Ok(json!({"sighash": hex(&hash)}))
}
