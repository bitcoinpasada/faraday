//! A wallet description that names one key twice, keys of two networks,
//! a key with no origin, an origin BIP-380 does not write, or a header
//! line twice is refused, whichever way it arrives.

use std::str::FromStr;

use bitcoin::bip32::Xpub;
use faraday_bip::bsms::{self, DescriptorRecord, SignerRecord};
use faraday_bip::keys::Network;
use faraday_bip::multisig_config;
use faraday_bip::policy::{self, WalletPolicy};
use faraday_crypto::work::Compute;

const KEYS: [&str; 3] = [
    "xpub6ErVmcYYHmavsMgxEcTZyzN5sqth1ZyRpFNJC26ij1wYGC2SBKYrgt9yariSbn7HLRoZUvhUhmPfsRTPrdhhGFscpPZzmch6UTdmRP1aZUj",
    "xpub6Du5Jn6eYZE96ccmAc1ZTFPzdnzrvqfG4mpamDun2qZYKywoiQJMCbS3kWWMr6U3XW6s125RLsaPABWgv2yA749ieaMe67FxkTjMsbcxCch",
    "xpub6Ex81KopPkEt9hJiWHabYy8LNsSR4A7sUQoFBk9dR8XxHrr4p9HrYWN3NCf5uwfopHnQkCG7FYnZMztKbtRtbh6tzZC4xtHPbmVVxRSN7ic",
];
const FP: [&str; 3] = ["793cc70b", "b3118e52", "842bd2ed"];
const TPUB: &str = "tpubDFZk17LmJpm9L6gT3oK6r3TWNni6fVLrkZLCsH2psqckyuuZ5239eePW97oLgikCUJ5zQX4MVVKDp9Yj9DC9wtYyGXkPwLuGeGN78U3ykR2";

/// The second key again with another depth and parent: the same public
/// key and chain code under another header.
fn second_again() -> String {
    let mut x = Xpub::from_str(KEYS[1]).unwrap();
    x.depth = 5;
    x.parent_fingerprint = [1, 2, 3, 4].into();
    x.to_string()
}

fn origin(i: usize) -> String {
    format!("[{}/48h/0h/0h/2h]", FP[i])
}

fn descriptor_with_repeat() -> String {
    format!(
        "wsh(sortedmulti(2,{}{}/<0;1>/*,{}{}/<0;1>/*,{}{}/<0;1>/*))",
        origin(0),
        KEYS[0],
        origin(1),
        KEYS[1],
        origin(2),
        second_again()
    )
}

#[test]
fn a_descriptor_with_one_key_twice_under_another_header_is_refused() {
    let c = Compute::unchecked();
    assert_eq!(
        WalletPolicy::from_descriptor(&c, &descriptor_with_repeat()),
        Err(policy::Error::RepeatedKey)
    );
}

#[test]
fn a_descriptor_record_with_one_key_twice_is_refused() {
    let c = Compute::unchecked();
    let text = descriptor_with_repeat().replace("/<0;1>/*", "/**");
    let record = format!("BSMS 1.0\n{text}\n/0/*,/1/*\nbc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4");
    assert_eq!(
        DescriptorRecord::parse(&c, &record),
        Err(bsms::Error::Policy(policy::Error::RepeatedKey))
    );
}

fn config(keys: &[(&str, String)]) -> String {
    let mut s = format!(
        "Name: Demo\nPolicy: 2 of {}\nFormat: P2WSH\nDerivation: m/48'/0'/0'/2'\n\n",
        keys.len()
    );
    for (fp, key) in keys {
        s.push_str(&format!("{fp}: {key}\n"));
    }
    s
}

#[test]
fn a_multisig_config_with_one_key_twice_is_refused() {
    let c = Compute::unchecked();
    let text = config(&[
        (FP[0], KEYS[0].into()),
        (FP[1], KEYS[1].into()),
        (FP[2], second_again()),
    ]);
    assert_eq!(
        multisig_config::parse(&c, &text),
        Err(policy::Error::RepeatedKey)
    );
}

#[test]
fn a_descriptor_of_mainnet_and_test_keys_is_refused() {
    let c = Compute::unchecked();
    let text = format!(
        "wsh(sortedmulti(2,{}{}/<0;1>/*,{}{TPUB}/<0;1>/*))",
        origin(0),
        KEYS[0],
        origin(1),
    );
    assert_eq!(
        WalletPolicy::from_descriptor(&c, &text),
        Err(policy::Error::MixedNetwork)
    );
}

fn twin_record(network: Network) -> String {
    let c = Compute::unchecked();
    let keys: Vec<_> = FP
        .iter()
        .zip(KEYS)
        .map(|(f, k)| (*f, k.to_string()))
        .collect();
    let policy = multisig_config::parse(&c, &config(&keys)).unwrap();
    let address = policy.address_at(&c, network, false, 0).unwrap();
    format!(
        "BSMS 1.0\n{}\n/0/*,/1/*\n{address}",
        &*policy.to_descriptor_template()
    )
}

#[test]
fn a_descriptor_record_of_mainnet_keys_stating_a_test_address_is_refused() {
    let c = Compute::unchecked();
    assert!(DescriptorRecord::parse(&c, &twin_record(Network::Mainnet)).is_ok());
    assert_eq!(
        DescriptorRecord::parse(&c, &twin_record(Network::Testnet)),
        Err(bsms::Error::Address)
    );
}

#[test]
fn a_key_record_whose_key_has_no_origin_is_refused() {
    let text = "BSMS 1.0\na54044308ceac9b7\nxpub6ErVmcYYHmavsMgxEcTZyzN5sqth1ZyRpFNJC26ij1wYGC2SBKYrgt9yariSbn7HLRoZUvhUhmPfsRTPrdhhGFscpPZzmch6UTdmRP1aZUj\nSigner 1 key\nIHEUwiVEv4zVrOFi3OjCsYBLbs0UjsdFmWgaXAq5J7QWHNmWL3vMo0W3o/9OLeAyAXP2MAdWC9ZmRKNvYAWdmlk=";
    assert_eq!(SignerRecord::parse(text), Err(bsms::Error::Key));
}

#[test]
fn an_origin_with_an_m_head_or_an_empty_path_is_refused() {
    let c = Compute::unchecked();
    for bad in ["[793cc70b/m/48h/0h/0h/2h]", "[793cc70b/]"] {
        let text = format!(
            "wsh(sortedmulti(1,{bad}{}/<0;1>/*,{}{}/<0;1>/*))",
            KEYS[0],
            origin(1),
            KEYS[1]
        );
        assert_eq!(
            WalletPolicy::from_descriptor(&c, &text),
            Err(policy::Error::Key),
            "{bad}"
        );
    }
}

#[test]
fn a_multisig_config_with_derivation_m_is_refused() {
    let c = Compute::unchecked();
    let text =
        config(&[(FP[0], KEYS[0].into()), (FP[1], KEYS[1].into())]).replace("m/48'/0'/0'/2'", "m");
    assert!(multisig_config::parse(&c, &text).is_err());
}

#[test]
fn a_multisig_config_with_a_header_twice_is_refused() {
    let c = Compute::unchecked();
    let base = config(&[(FP[0], KEYS[0].into()), (FP[1], KEYS[1].into())]);
    for (header, extra) in [
        ("Policy: 2 of 2", "Policy: 1 of 2"),
        ("Format: P2WSH", "Format: P2SH"),
        ("Name: Demo", "Name: Other"),
    ] {
        let text = base.replace(header, &format!("{header}\n{extra}"));
        assert!(multisig_config::parse(&c, &text).is_err(), "{extra}");
    }
}
