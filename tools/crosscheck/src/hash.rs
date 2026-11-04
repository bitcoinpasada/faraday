//! The Hashes and Encodings calculators: SHA-256, SHA-256d and HASH160
//! (`faraday_bip::hashes`), the text encodings (`faraday_codec::encodings`)
//! the SHA-256 of a whole file (`faraday_core::file_menu::sha256`), and
//! the one HKDF (`faraday_crypto::hkdf_sha256` and `hkdf_sha256_expand`).

use faraday_bip::hashes::hashes;
use faraday_codec::encodings::{self, Encoding};
use faraday_crypto::work::Compute;
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, hex, text};

fn encoding_name(e: Encoding) -> &'static str {
    match e {
        Encoding::Base58Check => "base58check",
        Encoding::Base58 => "base58",
        Encoding::Bech32 => "bech32",
        Encoding::Bech32m => "bech32m",
        Encoding::Hex => "hex",
    }
}

fn len(req: &Value) -> usize {
    req["len"].as_u64().expect("len") as usize
}

/// Handles `hash.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    match name {
        // The three hashes of some bytes.
        "hashes" => {
            let h = hashes(&bytes(req, "bytes"));
            Ok(json!({
                "len": h.len,
                "sha256": hex(&h.sha256),
                "sha256d": hex(&h.sha256d),
                "hash160": hex(&h.hash160),
            }))
        }
        // The SHA-256 of a whole file, as the Files screen shows it.
        "file" => {
            let c = Compute::unchecked();
            Ok(json!(hex(&faraday_core::file_menu::sha256(
                &c,
                &bytes(req, "bytes")
            ))))
        }
        // Which spelling a text is, and what it carries.
        "read" => {
            let r = encodings::read(text(req, "text")).ok_or("no encoding")?;
            Ok(json!({
                "encoding": encoding_name(r.encoding),
                "bytes": hex(&r.bytes),
                "version": r.version,
                "hrp": r.hrp,
                "checksum": r.checksum,
            }))
        }
        // The same bytes written in one spelling.
        "write" => {
            let b = bytes(req, "bytes");
            let written = match text(req, "kind") {
                "base58check" => Some(encodings::base58check(
                    req["version"].as_u64().ok_or("no version")? as u8,
                    &b,
                )),
                "base58" => Some(encodings::base58(&b)),
                "bech32" => encodings::bech32(text(req, "hrp"), &b),
                "bech32m" => encodings::bech32m(text(req, "hrp"), &b),
                other => return Err(format!("unknown kind {other}")),
            };
            Ok(json!(written.ok_or("not written")?))
        }
        // The bytes of a text of hex digits.
        "from_hex" => {
            let b = encodings::from_hex(text(req, "text")).ok_or("not hex")?;
            Ok(json!(hex(&b)))
        }
        // HKDF-Expand from a pseudorandom key: `len` bytes, or a refusal.
        "hkdf_expand" => {
            let mut out = vec![0u8; len(req)];
            faraday_crypto::hkdf_sha256_expand(&bytes(req, "prk"), &bytes(req, "info"), &mut out)
                .ok_or("longer than 255 blocks")?;
            Ok(json!(hex(&out)))
        }
        // The whole HKDF: extract under the salt, then expand.
        "hkdf" => {
            let mut out = vec![0u8; len(req)];
            faraday_crypto::hkdf_sha256(
                &bytes(req, "salt"),
                &bytes(req, "ikm"),
                &bytes(req, "info"),
                &mut out,
            )
            .ok_or("longer than 255 blocks")?;
            Ok(json!(hex(&out)))
        }
        _ => Err(format!("unknown hash op {name}")),
    }
}
