//! Uniform Resources (`faraday_codec::ur`, BCR-2020-005 and -012): a
//! single-part UR written and read, the parts of a multi-part one as the
//! sender animates them, and a set of parts put together as the scanner
//! reads frames.

use faraday_bip::bitcoin::bip32::{ChildNumber, Xpub};
use faraday_codec::ur::{self, Decoder, Encoder, Message};
use faraday_qr::cbor::Value as Cbor;
use faraday_qr::registry::{self, Read};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, hex, num, text, texts};

fn err<E: core::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn message(m: &Message) -> Value {
    match m {
        Message::Psbt(d) => json!({"type": ur::PSBT_TYPE, "data": hex(d)}),
        Message::Bytes(d) => json!({"type": ur::BYTES_TYPE, "data": hex(d)}),
        Message::Other { ur_type, cbor } => json!({"type": ur_type, "cbor": hex(cbor)}),
    }
}

/// A CBOR item as a tree the harness compares: `["u", n]`, `["n", n]`
/// (the value is `-1 - n`), `["b", hex]`, `["t", text]`, `["a", items]`,
/// `["m", [[key, value], ...]]`, `["g", tag, item]`, `["f", bool]`, `["z"]`.
fn tree(v: &Cbor) -> Value {
    match v {
        Cbor::Uint(n) => json!(["u", n]),
        Cbor::Neg(n) => json!(["n", n]),
        Cbor::Bytes(b) => json!(["b", hex(b)]),
        Cbor::Text(t) => json!(["t", t]),
        Cbor::Array(a) => json!(["a", a.iter().map(tree).collect::<Vec<_>>()]),
        Cbor::Map(m) => json!([
            "m",
            m.iter()
                .map(|(k, v)| json!([tree(k), tree(v)]))
                .collect::<Vec<_>>()
        ]),
        Cbor::Tag(t, i) => json!(["g", t, tree(i)]),
        Cbor::Bool(b) => json!(["f", b]),
        Cbor::Null => json!(["z"]),
    }
}

/// Handles `ur.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    match name {
        // A message's single-part UR and the first `count` parts of its
        // animation.
        "encode" => {
            let data = bytes(req, "data");
            let psbt = text(req, "kind") == "psbt";
            let single = if psbt {
                ur::encode_psbt(&data)
            } else {
                ur::encode_bytes(&data)
            };
            let len = num(req, "fragment_len") as usize;
            let mut enc = if psbt {
                Encoder::psbt(&data, len)
            } else {
                Encoder::bytes(&data, len)
            }
            .map_err(err)?;
            let parts: Vec<String> = (0..num(req, "count")).map(|_| enc.next_part()).collect();
            Ok(json!({"single": single, "fragments": enc.fragment_count(), "parts": parts}))
        }
        // A single-part UR of any type read.
        "single" => Ok(message(&ur::decode_single(text(req, "text")).map_err(err)?)),
        // A UR of any type written from its CBOR.
        "write" => Ok(json!({"ur": ur::encode_single(text(req, "type"), &bytes(req, "cbor"))})),
        // Parts read one after another into one decoder.
        "decode" => {
            let mut d = Decoder::new();
            for p in texts(req, "parts") {
                d.receive(p).map_err(err)?;
            }
            Ok(json!({
                "complete": d.is_complete(),
                "message": d.message().map(message),
            }))
        }
        // The one CRC-32 (`faraday_crypto::crc32`) over bytes.
        "crc32" => Ok(json!({"crc": faraday_crypto::crc32(&bytes(req, "data"))})),
        // The reader's one CBOR item, with nothing after it.
        "cbor" => Ok(tree(
            &Cbor::decode(&bytes(req, "cbor")).map_err(|_| "malformed CBOR".to_string())?,
        )),
        // A registry type read as the key lines or descriptor it holds.
        "registry" => {
            let read = registry::read(text(req, "type"), &bytes(req, "cbor"))
                .map_err(|e| format!("{e:?}"))?;
            Ok(match read {
                None => Value::Null,
                Some(Read::Psbt(b)) => json!({"psbt": hex(&b)}),
                Some(Read::Keys(k)) => json!({"keys": k}),
                Some(Read::Descriptor(d)) => json!({"descriptor": d}),
            })
        }
        // The `crypto-account` Faraday writes for one key.
        "account" => {
            let master: [u8; 4] = bytes(req, "master").try_into().expect("4 bytes");
            let path: Vec<ChildNumber> = texts(req, "path")
                .iter()
                .map(|c| c.parse().expect("child number"))
                .collect();
            let xpub: Xpub = text(req, "xpub").parse().expect("xpub");
            let cbor =
                registry::account_cbor(master, &path, &xpub, req["wpkh"].as_bool().expect("wpkh"));
            Ok(json!({"cbor": hex(&cbor)}))
        }
        _ => Err(format!("unknown ur op {name}")),
    }
}
