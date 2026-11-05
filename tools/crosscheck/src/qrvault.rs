//! The QR vault, `faraday_vault::qr` (`docs/VAULT.md` §1.2, §2.1): one item
//! sealed into an OpenPGP message under a passphrase, and a message opened
//! into its item. The other programs' side (Sequoia's `sq`, rPGP) is the
//! area's own Python and `tools/crosscheck/rpgp/`.
//!
//! An item travels as JSON: `{"kind": "words", "text": "<the words>"}`,
//! `{"kind": "codex32", "text": …, "for": "<8 hex>"}`, `{"kind":
//! "passphrase", "text": …, "for": …}` or `{"kind": "note", "text": …}` (an
//! empty note is the empty vault). A passphrase is hex of its UTF-8.

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_crypto::work::Compute;
use faraday_pgp::message::RANDOM_LEN;
use faraday_vault::layouts::{self, Opened};
use faraday_vault::records::{field, kind};
use faraday_vault::{Record, qr};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, hex, num};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

fn passphrase(req: &Value) -> Result<String, String> {
    String::from_utf8(bytes(req, "passphrase")).map_err(err)
}

fn text<'a>(item: &'a Value, key: &str) -> &'a str {
    item[key].as_str().unwrap_or_else(|| panic!("no {key}"))
}

fn record(c: &Compute, item: &Value) -> Result<Record, String> {
    Ok(match text(item, "kind") {
        "words" => {
            let m = Mnemonic::parse(c, Language::English, text(item, "text")).map_err(err)?;
            Record::new(kind::KEY).with(field::KEY, &layouts::words_payload(&m))
        }
        "codex32" => Record::new(kind::CODEX32_SHARE)
            .with(field::CODEX32_STRING, text(item, "text").as_bytes())
            .with(field::CODEX32_KEY, &bytes(item, "for")),
        "passphrase" => Record::new(kind::PASSPHRASE)
            .with(field::PASSPHRASE_KEY, &bytes(item, "for"))
            .with(field::PASSPHRASE, text(item, "text").as_bytes()),
        "note" => Record::new(kind::NOTE).with(field::NOTE, text(item, "text").as_bytes()),
        other => return Err(format!("unknown kind {other}")),
    })
}

fn lossy(b: Option<&[u8]>) -> String {
    String::from_utf8_lossy(b.unwrap_or(&[])).into_owned()
}

fn item(r: &Record) -> Result<Value, String> {
    Ok(match r.kind {
        kind::KEY => {
            let Some(Opened::Words(m)) =
                layouts::payload_of(layouts::KIND_WORDS, r.field(field::KEY).unwrap_or(&[]))
            else {
                return Err("a key that is not words".into());
            };
            let words: Vec<&str> = m.indices().iter().map(|i| m.language().word(*i)).collect();
            json!({"kind": "words", "text": words.join(" ")})
        }
        kind::CODEX32_SHARE => json!({
            "kind": "codex32",
            "text": lossy(r.field(field::CODEX32_STRING)),
            "for": hex(r.field(field::CODEX32_KEY).unwrap_or(&[])),
        }),
        kind::PASSPHRASE => json!({
            "kind": "passphrase",
            "text": lossy(r.field(field::PASSPHRASE)),
            "for": hex(r.field(field::PASSPHRASE_KEY).unwrap_or(&[])),
        }),
        kind::NOTE => json!({"kind": "note", "text": lossy(r.field(field::NOTE))}),
        _ => return Err("an item of no QR vault".into()),
    })
}

/// Handles `qrvault.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let c = Compute::unchecked();
    match name {
        // The message `qr::seal` makes of the item under the passphrase.
        "seal" => {
            let random: [u8; RANDOM_LEN] = bytes(req, "random")
                .try_into()
                .map_err(|_| "the random bytes' length")?;
            let exponent = u8::try_from(num(req, "exponent")).map_err(err)?;
            let r = record(&c, &req["item"])?;
            let file = qr::seal(&c, &passphrase(req)?, exponent, &random, &r).map_err(err)?;
            Ok(json!({"file": hex(&file)}))
        }
        // The item `qr::open` finds in the message, or its refusal.
        "open" => {
            let r = qr::open(&c, &passphrase(req)?, &bytes(req, "file")).map_err(err)?;
            Ok(json!({"item": item(&r)?}))
        }
        _ => Err(format!("unknown qrvault op {name}")),
    }
}
