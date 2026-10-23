//! Helpers shared by the integration tests.
#![allow(dead_code)]

use faraday_bip::json::{self, Value};
use faraday_crypto::work::Compute;

/// A token for the tests' key work (`docs/COMPUTE.md` §3), counting
/// nothing: faraday-core's `testkit::compute` is out of this crate's
/// reach. Cheap.
#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-intake's tests, which cannot reach faraday-core's testkit"
)]
pub fn compute() -> Compute {
    Compute::new(|_| {})
}

/// A vector file under the repository's `tools/vectors`.
pub fn tools_vector(path: &str) -> String {
    let full = format!("{}/../../tools/vectors/{path}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("{full}: {e}"))
}

/// A file other wallets write, from `faraday-core`'s interop set
/// (`faraday/faraday-core/tests/vectors/interop/README.md`).
pub fn interop(path: &str) -> String {
    let full = format!(
        "{}/../faraday-core/tests/vectors/interop/{path}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("{full}: {e}"))
}

pub fn parse(text: &str) -> Value {
    json::parse(text).unwrap_or_else(|e| panic!("not JSON: {e:?}"))
}

pub fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// The English rows of the BIP-39 vectors: entropy hex, words, seed hex,
/// xprv.
pub fn bip39_english() -> Vec<[String; 4]> {
    let v = parse(&tools_vector("bip39/vectors.json"));
    v.get("english")
        .and_then(Value::as_array)
        .expect("english")
        .iter()
        .map(|row| {
            let r = row.as_array().expect("row");
            std::array::from_fn(|i| r[i].as_str().expect("string").to_string())
        })
        .collect()
}
