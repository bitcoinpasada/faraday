//! Helpers shared by the integration tests.
#![allow(dead_code)]

pub mod builder;

use std::path::PathBuf;

use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::{MasterKey, Network};

/// BIP-39 vector seed: "abandon" ×11 "about".
pub const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
/// BIP-39 vector seed: "zoo" ×11 "wrong".
pub const ZOO: &str = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong";
/// BIP-39 vector seed.
pub const LEGAL: &str =
    "legal winner thank year wave sausage worth useful legal winner thank yellow";

pub fn master(words: &str, network: Network) -> MasterKey {
    let m = Mnemonic::parse(Language::English, words).expect("test mnemonic");
    MasterKey::from_seed(&m.to_seed(b"").expect("ascii passphrase"), network)
}

pub fn unhex(s: &str) -> Vec<u8> {
    assert!(s.len().is_multiple_of(2), "odd hex length");
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// One record of a `tools/vectors/psbt/*.txt` file: `key: value` lines,
/// keys may repeat.
#[derive(Debug, Clone, Default)]
pub struct Record(pub Vec<(String, String)>);

impl Record {
    pub fn get(&self, key: &str) -> &str {
        self.all(key)
            .next()
            .unwrap_or_else(|| panic!("record has no `{key}`"))
    }

    pub fn opt(&self, key: &str) -> Option<&str> {
        self.all(key).next()
    }

    pub fn all<'a>(&'a self, key: &str) -> impl Iterator<Item = &'a str> + 'a {
        let key = key.to_string();
        self.0
            .iter()
            .filter(move |(k, _)| *k == key)
            .map(|(_, v)| v.as_str())
    }
}

/// Reads a vector file relative to the repository root.
pub fn records(path: &str) -> Vec<Record> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let text = std::fs::read_to_string(root.join(path)).expect("vector file");
    let mut out = Vec::new();
    let mut current = Record::default();
    for line in text.lines() {
        if line.is_empty() {
            if !current.0.is_empty() {
                out.push(std::mem::take(&mut current));
            }
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let (k, v) = line
            .split_once(": ")
            .unwrap_or((line.trim_end_matches(':'), ""));
        current.0.push((k.to_string(), v.to_string()));
    }
    if !current.0.is_empty() {
        out.push(current);
    }
    out
}
