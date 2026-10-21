//! `faraday-intake`'s JSON reader over any bytes (`docs/INTEROP.md` §5
//! I2), then the wallet reader over the same text: a single-key file or a
//! Caravan config is another wallet's file, whose every byte its writer
//! chose. Text that reads as JSON is written back and read again.

#![no_main]

use faraday_intake::json::{self, Value};
use libfuzzer_sys::fuzz_target;

/// The value written as JSON again.
fn write(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.clone(),
        Value::String(s) => json::quote(s),
        Value::Array(items) => {
            let items: Vec<String> = items.iter().map(write).collect();
            format!("[{}]", items.join(","))
        }
        Value::Object(members) => {
            let members: Vec<String> = members
                .iter()
                .map(|(k, v)| format!("{}:{}", json::quote(k), write(v)))
                .collect();
            format!("{{{}}}", members.join(","))
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    if let Ok(v) = json::parse(text) {
        assert_eq!(json::parse(&write(&v)), Ok(v));
    }
    let _ = faraday_intake::wallet::read_wallet(text);
});
