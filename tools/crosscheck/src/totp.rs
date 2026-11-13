//! TOTP codes (`faraday_vault::totp`): a field 6 value read (`parse`) and
//! the code, the seconds left and the period (`Totp::code`, `left`,
//! `period`) at a Unix time. KeePass's fields (`from_fields`) and
//! `normalise` have no outside reader and stay with the crate's tests.

use faraday_vault::totp::{self, NotTotp, Totp};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{num, nums, text};

/// What `parse` refuses with, as the answer's refusal.
fn refusal(e: NotTotp) -> String {
    match e {
        NotTotp::Hotp => "hotp".to_string(),
        NotTotp::Unreadable => "unreadable".to_string(),
    }
}

fn read(req: &Value) -> Result<Totp, String> {
    totp::parse(text(req, "field")).map_err(refusal)
}

/// Handles `totp.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    match name {
        // One code, the seconds left in its period and the period.
        "code" => {
            let t = read(req)?;
            let unix = num(req, "time");
            Ok(json!({
                "code": &*t.code(unix),
                "left": t.left(unix),
                "period": t.period(),
            }))
        }
        // What a value reads as: its period and the codes at some times.
        "read" => {
            let t = read(req)?;
            let codes: Vec<String> = nums(req, "times")
                .into_iter()
                .map(|u| String::from(&*t.code(u)))
                .collect();
            Ok(json!({"codes": codes, "period": t.period()}))
        }
        _ => Err(format!("unknown op totp.{name}")),
    }
}
