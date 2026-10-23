//! Codex32 (BIP 93) through `faraday_bip::codex32`.

use faraday_bip::codex32::{self, Codex32};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, hex, num, text, texts};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

fn strings(req: &Value, key: &str) -> Result<Vec<Codex32>, String> {
    texts(req, key)
        .into_iter()
        .map(|s| Codex32::parse(s).map_err(err))
        .collect()
}

fn four(req: &Value, key: &str) -> [u8; 4] {
    text(req, key)
        .as_bytes()
        .try_into()
        .expect("four characters")
}

/// What a string says about itself: its header, and its seed when it
/// is the secret.
fn describe(s: &Codex32) -> Value {
    let seed = s.to_seed().ok().map(|seed| hex(seed.expose().bytes()));
    json!({
        "string": s.encode().as_str(),
        "threshold": s.threshold(),
        "identifier": String::from_utf8_lossy(&s.identifier()),
        "index": (s.share_index() as char).to_string(),
        "seed": seed,
    })
}

/// Handles `codex32.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    match name {
        "parse" => Ok(describe(&Codex32::parse(text(req, "string")).map_err(err)?)),
        "from_seed" => {
            let s = Codex32::from_seed(
                num(req, "threshold") as u8,
                four(req, "identifier"),
                &bytes(req, "seed"),
            )
            .map_err(err)?;
            Ok(describe(&s))
        }
        "recover" => Ok(describe(
            &Codex32::recover(&strings(req, "shares")?).map_err(err)?,
        )),
        "derive_share" => {
            let index = text(req, "index").as_bytes()[0];
            Ok(describe(
                &Codex32::derive_share(&strings(req, "strings")?, index).map_err(err)?,
            ))
        }
        "split" => {
            let secret = Codex32::parse(text(req, "secret")).map_err(err)?;
            let shares = secret
                .split(
                    num(req, "threshold") as u8,
                    num(req, "shares") as u8,
                    &bytes(req, "fresh"),
                )
                .map_err(err)?;
            Ok(
                json!({"shares": shares.iter().map(|s| s.encode().as_str().to_string()).collect::<Vec<_>>()}),
            )
        }
        "identifier_for" => {
            let fp: [u8; 4] = bytes(req, "fingerprint").try_into().expect("4 bytes");
            Ok(json!({"identifier": String::from_utf8_lossy(&codex32::identifier_for(fp))}))
        }
        _ => Err(format!("unknown codex32 op {name}")),
    }
}
