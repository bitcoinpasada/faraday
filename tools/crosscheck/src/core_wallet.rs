//! A Bitcoin Core descriptor wallet's `wallet.dat`, read whole by
//! `faraday_core::core_wallet::read_file`, as the cross-check compares it
//! with Bitcoin Core's own answers.

use faraday_bip::keys::Network;
use faraday_core::core_wallet::{Passphrase, read_file};
use faraday_crypto::work::Compute;
use faraday_intake::secret_buf::SecretText;
use serde_json::{Value, json};

use crate::Answer;
use crate::json::bytes;

/// Handles `core_wallet.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    match name {
        // The wallet the file holds on regtest, with the passphrase if one
        // is given; a refusal is the answer's error, by its name.
        "read" => {
            let file = bytes(req, "db");
            let pass = req
                .get("passphrase")
                .and_then(Value::as_str)
                .map(SecretText::of);
            let c = Compute::unchecked();
            let w = read_file(
                &c,
                "wallet.dat",
                &file,
                &[],
                Network::Regtest,
                pass.as_ref(),
            )
            .map_err(|e| format!("{e:?}"))?;
            let listing: Vec<Value> = w
                .descriptors
                .iter()
                .map(|d| {
                    let active = w.active.iter().find(|a| a.id == d.id);
                    json!({
                        "text": d.text,
                        "listed": d.listed,
                        "timestamp": d.creation_time,
                        "next_index": d.next_index,
                        "range": d.range.map(|(first, last)| [first, last]),
                        "active": active.is_some(),
                        "internal": active.map(|a| a.internal),
                    })
                })
                .collect();
            Ok(json!({
                "flags": w.flags,
                "encrypted": w.is_encrypted(),
                "whole": w.is_whole(),
                "passphrase": match w.passphrase {
                    Passphrase::NotNeeded => "not-needed",
                    Passphrase::NotGiven => "not-given",
                    Passphrase::Opened => "opened",
                    Passphrase::Wrong => "wrong",
                },
                "failures": w.failures.iter()
                    .map(|f| json!([f.record, format!("{:?}", f.fault)]))
                    .collect::<Vec<_>>(),
                "failures_more": w.failures_more,
                "descriptors": listing,
                "with_private_keys": w.descriptors.iter()
                    .filter(|d| d.private_keys + d.encrypted_keys > 0)
                    .count(),
                "keys_passed": w.keys_passed,
                "keys_unchecked": w.keys_unchecked,
                "tx_count": w.tx_count,
            }))
        }
        _ => Err(format!("unknown core_wallet op {name}")),
    }
}
