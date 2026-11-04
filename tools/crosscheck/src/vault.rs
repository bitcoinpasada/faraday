//! What of the vault the cross-check still asks of Faraday's code
//! (`docs/VAULT.md`): Argon2id through `faraday_backup::argon2id`, a
//! passphrase's NFC form and the Unicode version `faraday_vault` makes it
//! under, and the payload layouts of §7 through `faraday_vault::layouts`.
//!
//! Passphrases travel as JSON text.

use faraday_backup::Cost;
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_crypto::work::Compute;
use faraday_vault::layouts::{self, Opened};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, hex, num};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

fn cost(req: &Value) -> Result<Cost, String> {
    let n = |k: &str| u32::try_from(num(req, k)).map_err(err);
    Ok(Cost {
        memory_kib: n("memory_kib")?,
        passes: n("passes")?,
        lanes: n("lanes")?,
    })
}

/// Handles `vault.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let c = Compute::unchecked();
    match name {
        // Argon2id, version 1.3, 32 bytes.
        "argon2id" => {
            let k = faraday_backup::argon2id(
                &c,
                &cost(req)?,
                &bytes(req, "password"),
                &bytes(req, "salt"),
            )
            .map_err(err)?;
            Ok(json!({"key": hex(k.expose())}))
        }
        // The Unicode version the passphrase is put in NFC under.
        "unicode_version" => {
            let (a, b, v) = faraday_vault::UNICODE_VERSION;
            Ok(json!({"version": format!("{a}.{b}.{v}")}))
        }
        // A passphrase's NFC form, the bytes Argon2id stretches (§3).
        "passphrase_nfc" => {
            let p = req["passphrase"].as_str().ok_or("no passphrase")?;
            let nfc = faraday_vault::kdbx::passphrase_bytes(p).map_err(err)?;
            Ok(json!({"nfc": hex(&nfc)}))
        }
        _ => layout(name, req),
    }
}

/// Handles the payload layouts of §7: `vault.words_payload`,
/// `seed_payload`, `sheet_payload` and `payload_of`.
fn layout(name: &str, req: &Value) -> Answer {
    match name {
        "words_payload" => {
            let lang = *Language::ALL
                .get(num(req, "language") as usize)
                .ok_or("no such language")?;
            let m = Mnemonic::from_entropy(lang, &bytes(req, "entropy")).map_err(err)?;
            Ok(json!({"payload": hex(&layouts::words_payload(&m))}))
        }
        "seed_payload" => {
            let p = layouts::seed_payload(&bytes(req, "seed")).ok_or("not 16 to 64 bytes")?;
            Ok(json!({"payload": hex(&p)}))
        }
        "sheet_payload" => {
            let p = layouts::sheet_payload(
                &bytes(req, "descriptor"),
                &bytes(req, "name"),
                &bytes(req, "note"),
            )
            .ok_or("too long")?;
            Ok(json!({"payload": hex(&p)}))
        }
        "payload_of" => {
            let kind = u8::try_from(num(req, "kind")).map_err(err)?;
            match layouts::payload_of(kind, &bytes(req, "payload")).ok_or("not that layout")? {
                Opened::Words(m) => {
                    let lang = Language::ALL
                        .iter()
                        .position(|l| *l == m.language())
                        .expect("a listed language");
                    Ok(json!({"words": {
                        "language": lang,
                        "entropy": hex(m.entropy().expose().as_bytes()),
                    }}))
                }
                Opened::Seed(s) => Ok(json!({"seed": hex(&s)})),
                Opened::Sheet(s) => Ok(json!({"sheet": [
                    hex(&s.descriptor), hex(&s.name), hex(&s.note)
                ]})),
            }
        }
        _ => Err(format!("unknown vault op {name}")),
    }
}
