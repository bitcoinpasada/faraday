//! Reading a request's fields and writing hex. A malformed request is a
//! harness bug, not a refusal, so it panics and the answer says so.

use bitcoin::secp256k1::{PublicKey, SecretKey};
use serde_json::Value;

/// Lowercase hex of `bytes`.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The bytes a hex string stands for.
pub fn unhex(s: &str) -> Vec<u8> {
    assert!(s.len().is_multiple_of(2), "odd-length hex {s}");
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

fn field<'a>(req: &'a Value, key: &str) -> &'a Value {
    req.get(key)
        .unwrap_or_else(|| panic!("request has no {key}"))
}

/// A string field.
pub fn text<'a>(req: &'a Value, key: &str) -> &'a str {
    field(req, key)
        .as_str()
        .unwrap_or_else(|| panic!("{key} is not text"))
}

/// A hex field.
pub fn bytes(req: &Value, key: &str) -> Vec<u8> {
    unhex(text(req, key))
}

/// A hex field that may be `null`.
pub fn opt_bytes(req: &Value, key: &str) -> Option<Vec<u8>> {
    req.get(key).and_then(Value::as_str).map(unhex)
}

/// A field of 32 hex bytes.
pub fn b32(req: &Value, key: &str) -> [u8; 32] {
    bytes(req, key)
        .try_into()
        .unwrap_or_else(|_| panic!("{key} is not 32 bytes"))
}

/// A list of strings.
pub fn texts<'a>(req: &'a Value, key: &str) -> Vec<&'a str> {
    field(req, key)
        .as_array()
        .unwrap_or_else(|| panic!("{key} is not a list"))
        .iter()
        .map(|v| v.as_str().expect("a list of text"))
        .collect()
}

/// A list of hex strings.
pub fn bytes_list(req: &Value, key: &str) -> Vec<Vec<u8>> {
    texts(req, key).into_iter().map(unhex).collect()
}

/// A non-negative integer field.
pub fn num(req: &Value, key: &str) -> u64 {
    field(req, key)
        .as_u64()
        .unwrap_or_else(|| panic!("{key} is not a number"))
}

/// A list of non-negative integers.
pub fn nums(req: &Value, key: &str) -> Vec<u64> {
    field(req, key)
        .as_array()
        .unwrap_or_else(|| panic!("{key} is not a list"))
        .iter()
        .map(|v| v.as_u64().expect("a list of numbers"))
        .collect()
}

/// A boolean field.
pub fn flag(req: &Value, key: &str) -> bool {
    field(req, key)
        .as_bool()
        .unwrap_or_else(|| panic!("{key} is not a boolean"))
}

/// Compressed public keys; a key that does not parse is a refusal.
pub fn pubkeys(req: &Value, key: &str) -> Result<Vec<PublicKey>, String> {
    bytes_list(req, key)
        .iter()
        .enumerate()
        .map(|(i, b)| PublicKey::from_slice(b).map_err(|_| format!("{key}[{i}] is not a key")))
        .collect()
}

/// One compressed public key.
pub fn pubkey(req: &Value, key: &str) -> Result<PublicKey, String> {
    PublicKey::from_slice(&bytes(req, key)).map_err(|_| format!("{key} is not a key"))
}

/// One secret key; out of range is a refusal.
pub fn seckey(req: &Value, key: &str) -> Result<SecretKey, String> {
    SecretKey::from_slice(&bytes(req, key)).map_err(|_| format!("{key} is out of range"))
}

/// Tweaks as `[[hex, x_only], ...]`; a tweak that is not 32 bytes is a
/// refusal, as Faraday's type cannot hold it.
pub fn tweaks(req: &Value) -> Result<Vec<faraday_bip::musig::Tweak>, String> {
    field(req, "tweaks")
        .as_array()
        .expect("tweaks is a list")
        .iter()
        .map(|t| {
            Ok(faraday_bip::musig::Tweak {
                bytes: unhex(t[0].as_str().expect("tweak hex"))
                    .try_into()
                    .map_err(|_| "a tweak is not 32 bytes".to_string())?,
                x_only: t[1].as_bool().expect("x_only"),
            })
        })
        .collect()
}
