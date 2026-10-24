//! Uniform Resources (`faraday_codec::ur`, BCR-2020-005 and -012): a
//! single-part UR written and read, the parts of a multi-part one as the
//! sender animates them, and a set of parts put together as the scanner
//! reads frames.

use faraday_codec::ur::{self, Decoder, Encoder, Message};
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
        _ => Err(format!("unknown ur op {name}")),
    }
}
