//! PSBTs: version 0 read and written by `faraday_psbt::Psbt` (over
//! `bitcoin`), version 2 by `faraday_psbt::v2::PsbtV2` (Faraday's own
//! reader of BIP-370's maps), with the transaction and locktime a
//! version 2 PSBT gives.

use faraday_crypto::work::Compute;
use faraday_psbt::Psbt;
use faraday_psbt::v2::PsbtV2;
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, hex};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

/// Handles `psbt.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let c = Compute::unchecked();
    match name {
        "v0" => {
            let p = Psbt::parse_bytes(&c, &bytes(req, "psbt")).map_err(err)?;
            Ok(json!({
                "bytes": hex(&p.to_bytes()),
                "txid": p.unsigned_tx().compute_txid().to_string(),
            }))
        }
        "v2" => {
            let p = PsbtV2::parse_bytes(&c, &bytes(req, "psbt")).map_err(err)?;
            let tx = p
                .unsigned_tx()
                .ok()
                .map(|t| hex(&bitcoin::consensus::serialize(&t)));
            Ok(json!({
                "bytes": hex(&p.to_bytes()),
                "locktime": p.locktime().ok(),
                "tx": tx,
            }))
        }
        _ => Err(format!("unknown psbt op {name}")),
    }
}
