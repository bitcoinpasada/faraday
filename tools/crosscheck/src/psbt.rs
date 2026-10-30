//! PSBTs: version 0 read and written by `faraday_psbt::Psbt` (over
//! `bitcoin`), with what `bitcoin` writes of it, version 2 by
//! `faraday_psbt::v2::PsbtV2` (Faraday's own reader of BIP-370's maps),
//! with the transaction and locktime a version 2 PSBT gives and its
//! version 0 form.

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
            // `canonical`: the PSBT as read, written by `bitcoin` (what
            // `to_bytes` writes for a field that changed), which Bitcoin
            // Core must read as it reads the PSBT itself. `v2`: the
            // version 2 PSBT of it (`PsbtV2::from_v0`), HWI's
            // `convert_to_v2`'s to compare; `back`: that turned back to
            // version 0 (`PsbtV2::to_v0`).
            let v2 = PsbtV2::from_v0(&c, &p);
            let back = v2.to_v0(&c).ok().map(|v| hex(&v.to_bytes()));
            Ok(json!({
                "bytes": hex(&p.to_bytes()),
                "canonical": hex(&p.inner().serialize()),
                "v2": hex(&v2.to_bytes()),
                "back": back,
                "txid": p.unsigned_tx().compute_txid().to_string(),
            }))
        }
        "v2" => {
            let p = PsbtV2::parse_bytes(&c, &bytes(req, "psbt")).map_err(err)?;
            let tx = p
                .unsigned_tx()
                .ok()
                .map(|t| hex(&bitcoin::consensus::serialize(&t)));
            // `v0`: the version 0 PSBT the signer is given, none where
            // there is none.
            let v0 = p.to_v0(&c).ok().map(|v| hex(&v.to_bytes()));
            Ok(json!({
                "bytes": hex(&p.to_bytes()),
                "locktime": p.locktime().ok(),
                "tx": tx,
                "v0": v0,
            }))
        }
        _ => Err(format!("unknown psbt op {name}")),
    }
}
