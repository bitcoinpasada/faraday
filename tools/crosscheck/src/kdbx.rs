//! The KDBX 4 export, `faraday_backup::kdbx`: a database written from
//! items, a passphrase, an Argon2id cost and the caller's randomness,
//! and the check the writer makes of its own file.

use faraday_backup::Cost;
use faraday_backup::kdbx::{self, Item, SEED_LEN};
use faraday_crypto::work::Compute;
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, hex, num};

/// Handles `kdbx.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let c = Compute::unchecked();
    match name {
        "write" => {
            let fields: Vec<[Vec<u8>; 3]> = req["items"]
                .as_array()
                .expect("items is a list")
                .iter()
                .map(|i| [bytes(i, "title"), bytes(i, "notes"), bytes(i, "secret")])
                .collect();
            let items: Vec<Item<'_>> = fields
                .iter()
                .map(|[title, notes, secret]| Item {
                    title,
                    notes,
                    secret,
                })
                .collect();
            let cost = Cost {
                memory_kib: num(req, "memory_kib") as u32,
                passes: num(req, "passes") as u32,
                lanes: num(req, "lanes") as u32,
            };
            let seed: [u8; SEED_LEN] = bytes(req, "seed").try_into().expect("the seed's length");
            let file =
                kdbx::write(&c, &items, &bytes(req, "passphrase"), cost, &seed).ok_or("refused")?;
            Ok(json!({"file": hex(&file)}))
        }
        "verify" => Ok(json!({
            "valid": kdbx::verify(&c, &bytes(req, "file"), &bytes(req, "passphrase"))
        })),
        _ => Err(format!("unknown kdbx op {name}")),
    }
}
