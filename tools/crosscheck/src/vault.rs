//! The vault (`docs/VAULT.md`) through `faraday_vault`, and its two
//! primitives as Faraday calls them: Argon2id through
//! `faraday_backup::argon2id`, and XChaCha20-Poly1305 through the
//! `chacha20poly1305` crate's detached calls, which `faraday_vault`
//! seals and opens every slot with.
//!
//! The slot key's HKDF and the keystream a new vault is drawn from are
//! private to `faraday_vault`; they are checked through `vault.create`,
//! whose every byte depends on both.
//!
//! Records travel as `[[type, [[field, hex], ...]], ...]`.

use chacha20poly1305::aead::inout::InOutBuf;
use chacha20poly1305::{AeadInOut, KeyInit, Tag, XChaCha20Poly1305, XNonce};
use faraday_backup::Cost;
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_crypto::work::Compute;
use faraday_vault::layouts::{self, Opened};
use faraday_vault::{Contents, Record};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{b32, bytes, bytes_list, hex, num};

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

/// A list of records as the request writes them.
fn contents(v: &Value) -> Contents {
    let records = v
        .as_array()
        .expect("contents is a list")
        .iter()
        .map(|r| {
            let kind = u8::try_from(r[0].as_u64().expect("a type")).expect("a type byte");
            let mut record = Record::new(kind);
            for f in r[1].as_array().expect("fields") {
                let number = u8::try_from(f[0].as_u64().expect("a field")).expect("a field byte");
                record.push(number, &crate::json::unhex(f[1].as_str().expect("hex")));
            }
            record
        })
        .collect();
    Contents { records }
}

fn records(c: &Contents) -> Value {
    Value::Array(
        c.records
            .iter()
            .map(|r| {
                let fields: Vec<Value> = r
                    .fields
                    .iter()
                    .map(|f| json!([f.number, hex(&f.bytes)]))
                    .collect();
                json!([r.kind, fields])
            })
            .collect(),
    )
}

fn nonce(req: &Value) -> [u8; 24] {
    bytes(req, "nonce").try_into().expect("a nonce is 24 bytes")
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
        // XChaCha20-Poly1305 with a detached tag, as a slot is sealed.
        "aead_seal" => {
            let cipher = XChaCha20Poly1305::new_from_slice(&b32(req, "key")).map_err(err)?;
            let mut buf = bytes(req, "plaintext");
            let tag = cipher
                .encrypt_inout_detached(
                    &XNonce::from(nonce(req)),
                    &bytes(req, "aad"),
                    InOutBuf::from(&mut buf[..]),
                )
                .map_err(err)?;
            Ok(json!({"ciphertext": hex(&buf), "tag": hex(&tag)}))
        }
        "aead_open" => {
            let cipher = XChaCha20Poly1305::new_from_slice(&b32(req, "key")).map_err(err)?;
            let mut buf = bytes(req, "ciphertext");
            let tag: [u8; 16] = bytes(req, "tag").try_into().expect("a 16-byte tag");
            cipher
                .decrypt_inout_detached(
                    &XNonce::from(nonce(req)),
                    &bytes(req, "aad"),
                    InOutBuf::from(&mut buf[..]),
                    &Tag::from(tag),
                )
                .map_err(err)?;
            Ok(json!({"plaintext": hex(&buf)}))
        }
        // A new vault from the seed every random byte is drawn from.
        "create" => {
            let passphrases = bytes_list(req, "passphrases");
            let refs: Vec<&[u8]> = passphrases.iter().map(Vec::as_slice).collect();
            let all: Vec<Contents> = req["contents"]
                .as_array()
                .expect("contents is a list")
                .iter()
                .map(contents)
                .collect();
            let slot_len = u32::try_from(num(req, "slot_len")).map_err(err)?;
            let file =
                faraday_vault::create(&c, slot_len, cost(req)?, &refs, &all, &b32(req, "seed"))
                    .map_err(err)?;
            Ok(json!({"file": hex(&file)}))
        }
        "open" => {
            let (_, opened) =
                faraday_vault::open(&c, &bytes(req, "file"), &bytes(req, "passphrase"))
                    .map_err(err)?;
            Ok(json!({"records": records(&opened)}))
        }
        // Opens the slot a passphrase opens and seals new contents into
        // it under the nonce given.
        "seal" => {
            let file = bytes(req, "file");
            let (opened, _) =
                faraday_vault::open(&c, &file, &bytes(req, "passphrase")).map_err(err)?;
            let sealed = opened
                .seal(&file, &contents(&req["contents"]), &nonce(req))
                .map_err(err)?;
            Ok(json!({"file": hex(&sealed)}))
        }
        // A slot's plaintext, and its reading, with no cipher around it.
        "encode" => {
            let slot_len = u32::try_from(num(req, "slot_len")).map_err(err)?;
            let plain = contents(&req["contents"]).encode(slot_len).map_err(err)?;
            Ok(json!({"plain": hex(&plain)}))
        }
        "decode" => {
            let c = Contents::decode(&bytes(req, "plain")).map_err(err)?;
            Ok(json!({"records": records(&c)}))
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
