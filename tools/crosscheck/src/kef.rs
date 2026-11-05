//! Krux's encrypted code, `faraday_kef` and `faraday_codec::kef`
//! (`docs/KEF.md`): sealed and opened under a key the area works out, or
//! under a passphrase at a small count, and its envelope taken apart.

use faraday_codec::kef::{self, Mode};
use faraday_crypto::work::Compute;
use faraday_kef::{Sealing, open, open_with_key, seal, seal_with_key, suggest_versions};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{b32, bytes, hex, num, opt_bytes, text};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

fn count(req: &Value) -> Result<u32, String> {
    u32::try_from(num(req, "count")).map_err(err)
}

fn version(req: &Value) -> Result<u8, String> {
    u8::try_from(num(req, "version")).map_err(err)
}

fn mode(req: &Value) -> Mode {
    match text(req, "mode") {
        "ecb" => Mode::Ecb,
        "cbc" => Mode::Cbc,
        "ctr" => Mode::Ctr,
        "gcm" => Mode::Gcm,
        other => panic!("unknown mode {other}"),
    }
}

/// The passphrase as typed: text, or the bytes of `passphrase_hex` when the
/// request gives them (a key that is not text, entered as hex).
fn passphrase(req: &Value) -> Vec<u8> {
    opt_bytes(req, "passphrase_hex").unwrap_or_else(|| text(req, "passphrase").as_bytes().to_vec())
}

/// Handles `kef.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let c = Compute::unchecked();
    match name {
        // The envelope `seal_with_key` writes under a key of the area's.
        "seal" => {
            let e = seal_with_key(
                &b32(req, "key"),
                &bytes(req, "plaintext"),
                version(req)?,
                &bytes(req, "ident"),
                count(req)?,
                &bytes(req, "iv"),
            )
            .map_err(err)?;
            Ok(json!({"envelope": hex(&e)}))
        }
        // The envelope `seal` writes: the key is worked out from the
        // passphrase, the id and the count.
        "seal_pass" => {
            let id = bytes(req, "ident");
            let iv = bytes(req, "iv");
            let how = Sealing {
                version: version(req)?,
                id: &id,
                count: count(req)?,
                iv: &iv,
            };
            let e = seal(
                &c,
                &passphrase(req),
                &bytes(req, "plaintext"),
                &how,
                &mut |_| true,
            )
            .map_err(err)?;
            Ok(json!({"envelope": hex(&e)}))
        }
        // The contents of an envelope under a key of the area's.
        "open" => {
            let envelope = kef::unwrap(&bytes(req, "envelope")).map_err(err)?;
            let o = open_with_key(&b32(req, "key"), &envelope).map_err(err)?;
            Ok(json!({"plaintext": hex(&o.plaintext)}))
        }
        // The contents of an envelope under a passphrase.
        "open_pass" => {
            let envelope = kef::unwrap(&bytes(req, "envelope")).map_err(err)?;
            let o = open(&c, &passphrase(req), &envelope, &mut |_| true).map_err(err)?;
            Ok(json!({"plaintext": hex(&o.plaintext)}))
        }
        // The parts of an envelope.
        "unwrap" => {
            let e = kef::unwrap(&bytes(req, "envelope")).map_err(err)?;
            Ok(json!({
                "id": hex(&e.id),
                "version": e.version,
                "count": e.count,
                "payload": hex(&e.payload),
            }))
        }
        // The versions suggested for a plaintext in a mode.
        "suggest" => Ok(json!({"versions": suggest_versions(&bytes(req, "plaintext"), mode(req))})),
        _ => Err(format!("unknown kef op {name}")),
    }
}
