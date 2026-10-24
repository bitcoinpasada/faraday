//! SeedQR and CompactSeedQR (`faraday_codec::seedqr`) and the QR encoder
//! they are drawn with (`faraday_codec::qr`), and Codex32QR: the digits and bytes a
//! key's words become, the words read back from them, and every module
//! of the codes.

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_codec::qr::{self, Ecc, Payload, QrMatrix};
use faraday_codec::seedqr;
use faraday_crypto::work::Compute;
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, hex, num, text};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

fn language(req: &Value) -> Result<Language, String> {
    Language::ALL
        .get(num(req, "language") as usize)
        .copied()
        .ok_or_else(|| "no such language".to_string())
}

const ECC: [Ecc; 4] = [Ecc::Low, Ecc::Medium, Ecc::Quartile, Ecc::High];

/// A code as its version and its rows, `1` a dark module.
fn matrix(m: &QrMatrix) -> Value {
    let rows: Vec<String> = (0..m.size())
        .map(|y| {
            (0..m.size())
                .map(|x| if m.module(x, y) { '1' } else { '0' })
                .collect()
        })
        .collect();
    json!({"version": m.version(), "rows": rows})
}

fn words(m: &Mnemonic) -> Value {
    json!({"indices": m.indices(), "entropy": hex(m.entropy().expose().as_bytes())})
}

/// Handles `seedqr.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let c = Compute::unchecked();
    match name {
        // A key's two codes, from its entropy.
        "encode" => {
            let m = Mnemonic::from_entropy(language(req)?, &bytes(req, "entropy")).map_err(err)?;
            let digits = seedqr::to_digits(&m);
            let compact = seedqr::to_entropy(&m);
            Ok(json!({
                "digits": String::from_utf8(digits.expose().as_bytes().to_vec()).expect("digits"),
                "compact": hex(compact.expose().as_bytes()),
                "seedqr": matrix(&seedqr::encode_seedqr(&c, &m).map_err(err)?),
                "compact_qr": matrix(&seedqr::encode_compact(&c, &m).map_err(err)?),
            }))
        }
        // The words a SeedQR's digits name.
        "digits" => {
            let m =
                seedqr::from_digits(text(req, "digits").as_bytes(), language(req)?).map_err(err)?;
            Ok(words(&m))
        }
        // The words a CompactSeedQR's bytes are.
        "compact" => {
            let m = seedqr::from_entropy(&bytes(req, "bytes"), language(req)?).map_err(err)?;
            Ok(words(&m))
        }
        // Any payload in one mode at one level, in the smallest version
        // up to `max_version` that holds it.
        "qr" => {
            let data = bytes(req, "data");
            let payload = match text(req, "mode") {
                "numeric" => Payload::Numeric(&data),
                "alphanumeric" => Payload::Alphanumeric(&data),
                _ => Payload::Bytes(&data),
            };
            let ecc = ECC[num(req, "ecc") as usize];
            let max = num(req, "max_version") as u8;
            Ok(matrix(
                &qr::encode_bounded(&c, payload, ecc, max).map_err(err)?,
            ))
        }
        // A codex32 string's Codex32QR code as Faraday shows it, and the
        // picture it writes of that code on a stick, with a label.
        "codex32qr" => {
            let m = qr::codex32qr(&c, text(req, "string")).ok_or("not a Codex32QR string")?;
            let lines = [text(req, "line").to_string()];
            let png = faraday_core::picture::labelled_png(&c, &m, text(req, "title"), &lines);
            Ok(json!({"code": matrix(&m), "png": hex(&png)}))
        }
        _ => Err(format!("unknown seedqr op {name}")),
    }
}
