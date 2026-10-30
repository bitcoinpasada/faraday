//! A WIF private key read as Bitcoin Core reads one (`DecodeSecret`),
//! against Core's own vectors (`tools/vectors/wif/`), and the addresses
//! a key pays, against published ones.

use crate::common::{hex, unhex};
use faraday_bip::bitcoin::{Address, Network, NetworkKind};
use faraday_bip::wif::{Error, SingleKey};
use faraday_crypto::work::Compute;
use serde_json::Value;

const VALID: &str = include_str!("../../../../tools/vectors/wif/key_io_valid.json");
const INVALID: &str = include_str!("../../../../tools/vectors/wif/key_io_invalid.json");

fn read(text: &str) -> Result<SingleKey, Error> {
    SingleKey::decode(&Compute::unchecked(), text)
}

/// Every private key in Core's valid file reads to its key's bytes, its
/// form and its chain, as Core's `key_io_valid_parse` checks.
#[test]
fn cores_valid_private_keys_read_with_their_form_and_chain() {
    let all: Vec<Value> = serde_json::from_str(VALID).unwrap();
    let mut seen = 0;
    for v in &all {
        let meta = &v[2];
        if meta["isPrivkey"] != Value::Bool(true) {
            continue;
        }
        let (text, key) = (v[0].as_str().unwrap(), v[1].as_str().unwrap());
        let k = read(text).unwrap_or_else(|e| panic!("{text}: {e:?}"));
        assert_eq!(hex(&k.secret_key().secret_bytes()), key, "{text}");
        assert_eq!(
            k.compressed(),
            meta["isCompressed"] == Value::Bool(true),
            "{text}"
        );
        let main = meta["chain"].as_str() == Some("main");
        assert_eq!(k.network() == NetworkKind::Main, main, "{text}");
        seen += 1;
    }
    assert!(seen >= 16, "{seen} private keys");
}

/// Nothing in Core's invalid file, and no address in its valid file,
/// reads as a private key.
#[test]
fn nothing_else_in_cores_files_reads_as_a_private_key() {
    let invalid: Vec<Value> = serde_json::from_str(INVALID).unwrap();
    for v in &invalid {
        let text = v[0].as_str().unwrap();
        assert!(read(text).is_err(), "{text} read as a key");
    }
    let valid: Vec<Value> = serde_json::from_str(VALID).unwrap();
    for v in valid
        .iter()
        .filter(|v| v[2]["isPrivkey"] != Value::Bool(true))
    {
        let text = v[0].as_str().unwrap();
        assert!(read(text).is_err(), "{text} read as a key");
    }
}

/// Base58check of `payload`: Core's `EncodeBase58Check`, through the
/// `bitcoin` crate, which Faraday did not write.
fn b58(payload: &[u8]) -> String {
    faraday_bip::bitcoin::base58::encode_check(payload)
}

/// A WIF of a valid checksum is refused for each rule `DecodeSecret`
/// holds: its version, its length, its compression flag, and a key of
/// zero or past the curve's order.
#[test]
fn a_wif_breaking_a_rule_core_holds_is_refused() {
    let one = unhex("0000000000000000000000000000000000000000000000000000000000000001");
    let with = |v: u8, key: &[u8], tail: &[u8]| -> String {
        let mut p = vec![v];
        p.extend_from_slice(key);
        p.extend_from_slice(tail);
        b58(&p)
    };
    assert!(read(&with(0x80, &one, &[0x01])).is_ok());
    assert_eq!(
        read(&with(0x81, &one, &[0x01])).err(),
        Some(Error::Version(0x81))
    );
    assert_eq!(
        read(&with(0x80, &one, &[0x02])).err(),
        Some(Error::Flag(0x02))
    );
    assert_eq!(
        read(&with(0x80, &one, &[0x01, 0x01])).err(),
        Some(Error::Length(35))
    );
    assert_eq!(
        read(&with(0x80, &one[1..], &[])).err(),
        Some(Error::Length(32))
    );
    assert_eq!(read(&with(0x80, &[0u8; 32], &[])).err(), Some(Error::Key));
    let order = unhex("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141");
    assert_eq!(read(&with(0xef, &order, &[0x01])).err(), Some(Error::Key));
    let text = with(0x80, &one, &[0x01]);
    let mut bad = text.into_bytes();
    let last = bad.len() - 1;
    bad[last] = if bad[last] == b'2' { b'3' } else { b'2' };
    assert_eq!(
        read(&String::from_utf8(bad).unwrap()).err(),
        Some(Error::Base58)
    );
}

/// The key 1, the curve's generator: its WIFs and the addresses it pays,
/// as published (BIP-173's P2WPKH example is this key) and as any
/// wallet derives them; an uncompressed key pays a legacy address only.
#[test]
fn the_generators_key_pays_its_published_addresses() {
    let address = |s: &faraday_bip::bitcoin::ScriptBuf, n: Network| {
        Address::from_script(s, n).unwrap().to_string()
    };
    let k = read("KwDiBf89QgGbjEhKnhXJuH7LrciVrZi3qYjgd9M7rFU73sVHnoWn").unwrap();
    let got: Vec<String> = k
        .scripts()
        .iter()
        .map(|s| address(s, Network::Bitcoin))
        .collect();
    assert_eq!(
        got,
        [
            "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH",
            "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4",
            "3JvL6Ymt8MVWiCNHC7oWU6nLeHNJKLZGLN",
        ]
    );
    let u = read("5HpHagT65TZzG1PH3CSu63k8DbpvD8s5ip4nEB3kEsreAnchuDf").unwrap();
    assert!(!u.compressed());
    let got: Vec<String> = u
        .scripts()
        .iter()
        .map(|s| address(s, Network::Bitcoin))
        .collect();
    assert_eq!(got, ["1EHNa6Q4Jz2uvNExL497mE43ikXhwF6kZm"]);
    let t = read("cMahea7zqjxrtgAbB7LSGbcQUr1uX1ojuat9jZodMN87JcbXMTcA").unwrap();
    assert_eq!(t.network(), NetworkKind::Test);
    let got: Vec<String> = t
        .scripts()
        .iter()
        .map(|s| address(s, Network::Testnet))
        .collect();
    assert_eq!(
        got,
        [
            "mrCDrCybB6J1vRfbwM5hemdJz73FwDBC8r",
            "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx",
            "2NAUYAHhujozruyzpsFRP63mbrdaU5wnEpN",
        ]
    );
}
