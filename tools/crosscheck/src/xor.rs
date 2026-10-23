//! Seed XOR through `faraday_entropy::SeedXor`, with the words read and
//! written by `faraday_bip::bip39`.

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_crypto::work::Compute;
use faraday_entropy::SeedXor;
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, bytes_list, hex, num, texts};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

fn language(req: &Value) -> Result<Language, String> {
    Language::ALL
        .get(num(req, "language") as usize)
        .copied()
        .ok_or_else(|| "no such language".to_string())
}

/// The XOR of the pushed parts, as entropy and as word indices.
fn result(xor: &SeedXor, lang: Language) -> Answer {
    let entropy = xor.entropy().map_err(err)?;
    let m = Mnemonic::from_entropy(lang, entropy.as_bytes()).map_err(err)?;
    Ok(json!({"entropy": hex(entropy.as_bytes()), "indices": m.indices()}))
}

/// Handles `xor.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let lang = language(req)?;
    let mut xor = SeedXor::new();
    match name {
        // Every part's words, read as BIP-39 and XORed.
        "combine" => {
            let c = Compute::unchecked();
            for words in texts(req, "parts") {
                let m = Mnemonic::parse(&c, lang, words).map_err(err)?;
                xor.push(m.entropy().expose().as_bytes()).map_err(err)?;
            }
            result(&xor, lang)
        }
        // The last part of a split: the seed XOR the random parts.
        "split" => {
            for part in bytes_list(req, "random_parts") {
                xor.push(&part).map_err(err)?;
            }
            xor.push(&bytes(req, "seed")).map_err(err)?;
            result(&xor, lang)
        }
        _ => Err(format!("unknown xor op {name}")),
    }
}
