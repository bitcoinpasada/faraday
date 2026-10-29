//! The known answers of Faraday's own procedures, read from
//! `tools/vectors/` for the tests: the cards (`tools/vectors/cards/`),
//! the camera (`tools/vectors/camera/`) and a mix (`tools/vectors/mix/`),
//! each folder's README giving the procedure and a few lines of Python
//! that reproduce its answers. Test-only; the secret-handling files take
//! bytes and numbers from here and hold no text of their own.

extern crate std;

use crate::Strength;
use std::vec::Vec;

pub(crate) const CARDS: &str = include_str!("../../../tools/vectors/cards/vectors.json");
pub(crate) const CAMERA: &str = include_str!("../../../tools/vectors/camera/vectors.json");
pub(crate) const MIX: &str = include_str!("../../../tools/vectors/mix/vectors.json");

/// A vector file, parsed.
pub(crate) fn json(text: &str) -> serde_json::Value {
    serde_json::from_str(text).expect("the vectors are JSON")
}

/// The bytes a hex string in the file names.
pub(crate) fn bytes(v: &serde_json::Value) -> Vec<u8> {
    let s = v.as_str().expect("a hex string");
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

/// A count in the file.
pub(crate) fn count(v: &serde_json::Value) -> usize {
    usize::try_from(v.as_u64().expect("a number")).unwrap()
}

/// A strength in bits in the file.
pub(crate) fn strength(v: &serde_json::Value) -> Strength {
    match v.as_u64() {
        Some(128) => Strength::Bits128,
        Some(160) => Strength::Bits160,
        Some(256) => Strength::Bits256,
        other => panic!("no strength of {other:?} bits here"),
    }
}
