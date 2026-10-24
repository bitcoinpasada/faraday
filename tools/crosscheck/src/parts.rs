//! Files split across QR codes, as `faraday_qr` writes and reads them:
//! BBQr (`faraday_qr::bbqr`) and the numbered text parts Specter writes
//! and SeedSigner reads (`faraday_qr::specter`). A set of parts is read
//! as the scanner reads frames, one at a time into one collector, and
//! the answer is where the set stands after the last, or the first
//! refusal.

use faraday_qr::{bbqr, specter};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, hex, num, text, texts};

/// Handles `bbqr.<name>`.
pub fn handle_bbqr(name: &str, req: &Value) -> Answer {
    match name {
        "encode" => {
            let t = text(req, "type").chars().next().expect("a file type");
            let parts = bbqr::encode(t, &bytes(req, "data"), num(req, "part_bytes") as usize)
                .ok_or("refused")?;
            Ok(json!({"parts": parts}))
        }
        "join" => {
            let mut c = bbqr::Collector::default();
            let mut last = Value::Null;
            for p in texts(req, "parts") {
                last = match c.add(p).map_err(|e| e.reason())? {
                    bbqr::Got::Part {
                        have,
                        total,
                        missing,
                    } => {
                        json!({"part": {"have": have, "total": total, "missing": missing}})
                    }
                    bbqr::Got::Done { file_type, data } => {
                        json!({"done": {"type": file_type.to_string(), "data": hex(&data)}})
                    }
                };
            }
            Ok(last)
        }
        _ => Err(format!("unknown bbqr op {name}")),
    }
}

/// Handles `specter.<name>`.
pub fn handle_specter(name: &str, req: &Value) -> Answer {
    match name {
        "parse" => {
            let (n, m, body) = specter::parse(text(req, "text")).ok_or("not a numbered part")?;
            Ok(json!({"n": n, "m": m, "body": body}))
        }
        "join" => {
            let mut c = specter::Collector::default();
            let mut last = Value::Null;
            for p in texts(req, "parts") {
                let (n, m, body) = specter::parse(p).ok_or("not a numbered part")?;
                last = match c.add(n, m, body) {
                    specter::Got::Part(have, total, missing) => {
                        json!({"part": {"have": have, "total": total, "missing": missing}})
                    }
                    specter::Got::Done(text) => json!({"done": text}),
                    specter::Got::Refused(why) => return Err(why.to_string()),
                };
            }
            Ok(last)
        }
        "encode" => Ok(json!({
            "parts": specter::encode(text(req, "text"), num(req, "chunk") as usize)
        })),
        _ => Err(format!("unknown specter op {name}")),
    }
}
