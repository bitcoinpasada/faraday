//! What a person who kept an extended private key or a master seed in hex
//! would notice when it is read back (F300, `docs/KEY-FORMS.md` §6),
//! against BIP 32's published test vectors 1 to 3 (`vectors.json`) and
//! vector 5 (`invalid-keys.txt`), both in `tools/vectors/bip32/`.
#![allow(clippy::disallowed_methods)]

use faraday_bip::keys::{Fingerprint, Network};
use faraday_crypto::work::Compute;
use faraday_keys::xprv::{HexSeedRefusal, XprvError, XprvRefusal, parse_hex_seed};
use faraday_keys::{Keys, Reveal};
use serde_json::Value;

const BIP32: &str = include_str!("../../../tools/vectors/bip32/vectors.json");
const BIP32_INVALID: &str = include_str!("../../../tools/vectors/bip32/invalid-keys.txt");

/// The master fingerprints of BIP 32's test vectors 1, 2 and 3 (the parent
/// fingerprint its first child's serialisation carries).
const FINGERPRINTS: [[u8; 4]; 3] = [
    [0x34, 0x42, 0x19, 0x3e],
    [0xbd, 0x16, 0xbe, 0xe5],
    [0x41, 0xd6, 0x3b, 0x50],
];

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-keys' tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(faraday_crypto::work::Hooks {
        count: |_| {},
        stop: || false,
        progress: |_, _| {},
    })
}

/// Vector `n` (0 to 2): its seed in hex, its master xprv, and its first
/// child's (path, xprv).
fn vector(n: usize) -> (String, String, (String, String)) {
    let v: Value = serde_json::from_str(BIP32).unwrap();
    let v = &v[n];
    let chain = |i: usize, k: &str| v["chains"][i][k].as_str().unwrap().to_string();
    (
        v["seed"].as_str().unwrap().to_string(),
        chain(0, "xprv"),
        (chain(1, "path"), chain(1, "xprv")),
    )
}

/// `xprv` re-written under another four-byte version, checksum made again.
fn with_version(xprv: &str, version: [u8; 4]) -> String {
    let read = faraday_codec::encodings::read(xprv).unwrap();
    let mut payload = version[1..].to_vec();
    payload.extend_from_slice(&read.bytes[3..]);
    faraday_codec::encodings::base58check(version[0], &payload)
}

fn expect_read(r: Result<Fingerprint, XprvError>) -> Fingerprint {
    r.unwrap_or_else(|e| panic!("refused: {e:?}"))
}

/// Vectors 1 to 3 load, give their published fingerprints and write the
/// published master xprv again; the key holds no words, seed or
/// passphrase.
#[test]
fn the_master_keys_of_vectors_1_to_3_load() {
    let c = compute();
    for (n, fp) in FINGERPRINTS.iter().enumerate() {
        let (_, xprv, _) = vector(n);
        let mut keys = Keys::on(Network::Mainnet);
        assert_eq!(expect_read(keys.add_xprv(&c, &xprv, "")), Fingerprint(*fp));
        let info = keys.info(0).unwrap();
        assert_eq!(info.word_count, None);
        assert_eq!(info.seed_len, None);
        assert!(!info.has_passphrase);
        let back = keys
            .master_xprv(&c, 0, Reveal::PaperForm)
            .unwrap()
            .release();
        assert_eq!(back.as_str(), xprv);
    }
}

/// An origin before the key is taken when it names the key's own
/// fingerprint and no path, and refused otherwise.
#[test]
fn an_origin_is_taken_only_as_the_keys_own_fingerprint() {
    let c = compute();
    let (_, xprv, _) = vector(0);
    let mut keys = Keys::on(Network::Mainnet);
    expect_read(keys.add_xprv(&c, &format!("[3442193e]{xprv}"), ""));
    let refused = |text: String| {
        Keys::on(Network::Mainnet)
            .add_xprv(&c, &text, "")
            .unwrap_err()
    };
    assert_eq!(
        refused(format!("[bd16bee5]{xprv}")),
        XprvError::Read(XprvRefusal::OriginDiffers {
            named: Fingerprint([0xbd, 0x16, 0xbe, 0xe5]),
            own: Fingerprint([0x34, 0x42, 0x19, 0x3e]),
        })
    );
    assert_eq!(
        refused(format!("[3442193e/0h]{xprv}")),
        XprvError::Read(XprvRefusal::OriginPath)
    );
    assert_eq!(
        refused(format!("[3442193e{xprv}")),
        XprvError::Read(XprvRefusal::OriginUnreadable)
    );
}

/// Every key of vector 5 is refused.
#[test]
fn every_key_of_vector_5_is_refused() {
    let c = compute();
    let mut seen = 0;
    for line in BIP32_INVALID.lines() {
        let key = line.split(' ').next().unwrap();
        seen += 1;
        let mut keys = Keys::on(Network::Mainnet);
        assert!(keys.add_xprv(&c, key, "").is_err(), "{line}");
        assert!(keys.is_empty(), "{line}");
    }
    assert_eq!(seen, 16);
}

/// A key that is not a master key is refused with its depth, its parent's
/// fingerprint and its child number: vector 1's m/0H.
#[test]
fn a_key_below_the_master_is_refused_with_where_it_is() {
    let c = compute();
    let (_, _, (path, xprv)) = vector(0);
    assert_eq!(path, "m/0h");
    let mut keys = Keys::on(Network::Mainnet);
    assert_eq!(
        keys.add_xprv(&c, &xprv, "").unwrap_err(),
        XprvError::Read(XprvRefusal::NotMaster {
            depth: 1,
            parent: Fingerprint(FINGERPRINTS[0]),
            index: 0,
            hardened: true,
        })
    );
    assert!(keys.is_empty());
}

/// The other network's version and SLIP-132's private spellings are
/// refused by name.
#[test]
fn another_networks_version_and_slip_132_are_refused_by_name() {
    let c = compute();
    let (_, xprv, _) = vector(0);
    let mut main = Keys::on(Network::Mainnet);
    let mut test = Keys::on(Network::Testnet);
    expect_read(test.add_seed(&c, &[7; 16], "").map_err(XprvError::Session));
    let tprv = test
        .master_xprv(&c, 0, Reveal::PaperForm)
        .unwrap()
        .release();
    assert_eq!(
        main.add_xprv(&c, tprv.as_str(), "").unwrap_err(),
        XprvError::Read(XprvRefusal::WrongNetwork {
            found: "tprv",
            wanted: "xprv"
        })
    );
    assert_eq!(
        Keys::on(Network::Testnet)
            .add_xprv(&c, &xprv, "")
            .unwrap_err(),
        XprvError::Read(XprvRefusal::WrongNetwork {
            found: "xprv",
            wanted: "tprv"
        })
    );
    // Vector 1's master key written with each of SLIP-132's eight private
    // versions: the zprv, yprv and Zprv strings are published ones.
    let main_versions: [(&str, [u8; 4]); 4] = [
        ("yprv", [0x04, 0x9d, 0x78, 0x78]),
        ("Yprv", [0x02, 0x95, 0xb0, 0x05]),
        ("zprv", [0x04, 0xb2, 0x43, 0x0c]),
        ("Zprv", [0x02, 0xaa, 0x7a, 0x99]),
    ];
    let test_versions: [(&str, [u8; 4]); 4] = [
        ("uprv", [0x04, 0x4a, 0x4e, 0x28]),
        ("Uprv", [0x02, 0x42, 0x85, 0xb5]),
        ("vprv", [0x04, 0x5f, 0x18, 0xbc]),
        ("Vprv", [0x02, 0x57, 0x50, 0x48]),
    ];
    for (network, versions) in [
        (Network::Mainnet, main_versions),
        (Network::Testnet, test_versions),
    ] {
        for (name, version) in versions {
            let text = with_version(&xprv, version);
            assert_eq!(
                Keys::on(network).add_xprv(&c, &text, "").unwrap_err(),
                XprvError::Read(XprvRefusal::Slip132(name)),
                "{name} on {network:?}"
            );
        }
    }
    for (name, text) in [
        (
            "zprv",
            "zprvAWgYBBk7JR8GjzqSzmunMCS7dAbwpYTCs1YUMDXqduMA5JFHZ3iX5s2UkAR6vBdcCYYa1S5o1fVLrKsrnpCQ4WpUd6aVUWP1bS2Yy5DoaKv",
        ),
        (
            "yprv",
            "yprvABrGsX5C9jantheLAR8A97LcTCTVsvThwu2FZpdxFtyH2CS4JPYxToNLixTWvGygnuRmFxVEZ18ny3GJ57nPGH8skkt4tbZXKhxuaUFh6jt",
        ),
        (
            "Zprv",
            "ZprvAhadJRUYsNgeAZzpwSNmBGmvLxeD2u8oBHC9GUoP1gBZhUpCKT6oAytQYsNaUcrWS1cYt25utssqyVVd63MMCzw9TZgtsur1VAJRYzwAEqZ",
        ),
    ] {
        assert_eq!(
            Keys::on(Network::Mainnet)
                .add_xprv(&c, text, "")
                .unwrap_err(),
            XprvError::Read(XprvRefusal::Slip132(name))
        );
    }
}

/// Vectors 1 to 3's seeds written in hex load and give the published
/// fingerprints, in either case and with spaces.
#[test]
fn the_seeds_of_vectors_1_to_3_load_from_hex() {
    let c = compute();
    for (n, fp) in FINGERPRINTS.iter().enumerate() {
        let (hex, xprv, _) = vector(n);
        let spaced: String = hex
            .to_uppercase()
            .as_bytes()
            .chunks(4)
            .map(|p| std::str::from_utf8(p).unwrap())
            .collect::<Vec<_>>()
            .join(" ");
        for text in [hex.as_str(), spaced.as_str()] {
            let seed = parse_hex_seed(text).unwrap();
            let mut keys = Keys::on(Network::Mainnet);
            assert_eq!(keys.add_seed(&c, &seed, "").unwrap(), Fingerprint(*fp));
            let master = keys
                .master_xprv(&c, 0, Reveal::PaperForm)
                .unwrap()
                .release();
            assert_eq!(master.as_str(), xprv);
        }
    }
}

/// A seed of 15 or 65 bytes and an odd count of digits are refused.
#[test]
fn a_hex_seed_of_another_length_is_refused() {
    assert_eq!(
        parse_hex_seed(&"ab".repeat(15)).unwrap_err(),
        HexSeedRefusal::Length(15)
    );
    assert_eq!(
        parse_hex_seed(&"ab".repeat(65)).unwrap_err(),
        HexSeedRefusal::Length(65)
    );
    assert_eq!(
        parse_hex_seed(&format!("{}a", "ab".repeat(16))).unwrap_err(),
        HexSeedRefusal::OddDigits(33)
    );
    assert_eq!(
        parse_hex_seed(&format!("{}zz", "ab".repeat(15))).unwrap_err(),
        HexSeedRefusal::NotHex
    );
    assert!(parse_hex_seed(&"ab".repeat(16)).is_ok());
    assert!(parse_hex_seed(&"ab".repeat(64)).is_ok());
}
