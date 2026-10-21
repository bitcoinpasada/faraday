//! What the key operations give a person or a wallet, against the vectors
//! the primitive crates are held to: BIP-85's children, a BIP-137 message
//! signature that verifies for the address its published key has, BIP-352's
//! address, and the fingerprint BIP-39 and BIP-32 publish for the
//! "abandon" words.

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::bitcoin::bip32::DerivationPath;
use faraday_bip::bitcoin::secp256k1::{Secp256k1, SecretKey};
use faraday_bip::bitcoin::{Address, Network as Chain};
use faraday_bip::keys::{MasterKey, Network, ScriptType};
use faraday_bip::silent::Receiver;
use faraday_crypto::work::Compute;
use faraday_keys::{Material, bip85, identity};
use serde_json::Value;

const BIP85: &str = include_str!("../../../tools/vectors/bip85/vectors.json");
const BIP352: &str =
    include_str!("../../../tools/vectors/bip352/send_and_receive_test_vectors.json");
const BIP137: &str = include_str!("../../../tools/vectors/message/bip137.txt");

/// A token for the tests' key work (`docs/COMPUTE.md` §3), counting
/// nothing: faraday-core's `testkit::compute` is out of this crate's
/// reach. Cheap.
#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-keys' tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_| {})
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn words(text: &str) -> Mnemonic {
    Mnemonic::parse(&compute(), Language::English, text).expect("the words")
}

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

#[test]
fn the_abandon_words_have_the_fingerprint_the_bips_publish() {
    let c = compute();
    let fp = identity::fingerprint_of_words(&c, ABANDON, "").expect("a fingerprint");
    assert_eq!(fp.0, [0x73, 0xc5, 0xda, 0x0a]);
    // Another passphrase is another key; the words typed with extra
    // spaces are the same words.
    let spaced = ABANDON.replace(' ', "  ");
    assert_eq!(identity::fingerprint_of_words(&c, &spaced, ""), Ok(fp));
    let other = identity::fingerprint_of_words(&c, ABANDON, "TREZOR").expect("a fingerprint");
    assert_ne!(other, fp);
    // Words that are no list's are refused with a reason.
    assert!(identity::fingerprint_of_words(&c, "abandon abandon", "").is_err());
}

#[test]
fn a_master_key_is_told_from_another_by_all_of_its_identity() {
    let c = compute();
    let m = words(ABANDON);
    let (fp, id) =
        identity::identity(&c, Material::Words(&m), "", Network::Mainnet).expect("an identity");
    assert_eq!(fp.0, [0x73, 0xc5, 0xda, 0x0a]);
    // The same on every network; another passphrase gives another.
    let (_, on_test) =
        identity::identity(&c, Material::Words(&m), "", Network::Testnet).expect("an identity");
    assert_eq!(id, on_test);
    let (_, with) =
        identity::identity(&c, Material::Words(&m), "x", Network::Mainnet).expect("an identity");
    assert_ne!(id, with);
}

#[test]
fn bip85_children_are_the_ones_the_bip_publishes() {
    let c = compute();
    let v: Value = serde_json::from_str(BIP85).unwrap();
    let master = MasterKey::decode(&c, v["master"].as_str().unwrap()).expect("the BIP's master");
    let apps = &v["applications"];
    let child = |app, index| bip85::child(&c, &master, app, index).map(|t| t.to_string());
    for case in apps["bip39"].as_array().unwrap() {
        let n = case["words"].as_u64().unwrap() as usize;
        let i = case["index"].as_u64().unwrap() as u32;
        assert_eq!(
            child(bip85::App::Words(n), i).as_deref(),
            case["mnemonic"].as_str(),
            "{}",
            case["section"]
        );
    }
    let first = |app: &str| apps[app][0].clone();
    assert_eq!(
        child(bip85::App::Wif, 0).as_deref(),
        first("wif")["wif"].as_str()
    );
    assert_eq!(
        child(bip85::App::Xprv, 0).as_deref(),
        first("xprv")["xprv"].as_str()
    );
    let hex = first("hex");
    let bytes = hex["bytes"].as_u64().unwrap() as usize;
    assert_eq!(
        child(bip85::App::Hex(bytes), 0).as_deref(),
        hex["entropy"].as_str().map(|e| &e[..bytes * 2])
    );
    for (name, app) in [
        ("base64", bip85::App::Base64 as fn(usize) -> bip85::App),
        ("base85", bip85::App::Base85),
    ] {
        let case = first(name);
        let n = case["length"].as_u64().unwrap() as usize;
        assert_eq!(
            child(app(n), 0).as_deref(),
            case["password"].as_str(),
            "{name}"
        );
    }
    // A length BIP-85 does not define gives nothing.
    assert!(child(bip85::App::Words(13), 0).is_none());
}

/// The text of the BIP-137 record named `case`, as `key: value` pairs.
fn record(case: &str) -> impl Fn(&str) -> &'static str + use<> {
    let block = BIP137
        .split("\n\n")
        .find(|b| b.contains(&format!("case: {case}\n")))
        .expect("the record");
    move |key: &str| {
        let prefix = format!("{key}: ");
        block
            .lines()
            .find_map(|l| l.strip_prefix(prefix.as_str()))
            .expect("a field")
    }
}

#[test]
fn a_message_signed_by_the_published_key_verifies_for_its_address() {
    let c = compute();
    let get = record("trezor p2wpkh mainnet");
    let m = words(get("words"));
    let master = identity::master_of(&c, Material::Words(&m), "", Network::Mainnet).unwrap();
    let path: DerivationPath = get("path").parse().unwrap();
    let signed = faraday_keys::message::sign(
        &c,
        &master,
        &path,
        ScriptType::NativeSegwit,
        Network::Mainnet,
        get("message"),
        faraday_psbt::message::Format::Bip137,
    )
    .expect("a signature");
    assert_eq!(signed.address, get("address"));
    let address = signed
        .address
        .parse::<Address<_>>()
        .unwrap()
        .require_network(Chain::Bitcoin)
        .unwrap();
    assert!(faraday_psbt::message::verify(&address, get("message"), &signed.signature).is_ok());
    // The signature published for the same key and message verifies too.
    let published = faraday_psbt::base64::encode(&unhex(get("signature-hex")));
    assert!(faraday_psbt::message::verify(&address, get("message"), &published).is_ok());
    // And not for another message.
    assert!(faraday_psbt::message::verify(&address, "another", &signed.signature).is_err());
}

#[test]
fn a_silent_payment_address_is_the_one_bip352_publishes() {
    let secp = Secp256k1::new();
    let cases: Vec<Value> = serde_json::from_str(BIP352).unwrap();
    let mut checked = 0;
    for case in &cases {
        for test in case["receiving"].as_array().unwrap() {
            let given = &test["given"];
            let key = |name: &str| {
                SecretKey::from_slice(&unhex(given["key_material"][name].as_str().unwrap()))
                    .unwrap()
            };
            let receiver = Receiver::new(
                key("scan_priv_key"),
                key("spend_priv_key").public_key(&secp),
                Network::Mainnet,
            );
            let published = test["expected"]["addresses"].as_array().unwrap();
            assert_eq!(
                faraday_keys::silent::address(&receiver, 0).as_deref(),
                published[0].as_str()
            );
            // Label 0 is the change label, which no address is handed out
            // under: `address` gives the unlabelled one for it.
            for (i, label) in given["labels"].as_array().unwrap().iter().enumerate() {
                let label = label.as_u64().unwrap() as u32;
                if label == 0 {
                    continue;
                }
                assert_eq!(
                    faraday_keys::silent::address(&receiver, label).as_deref(),
                    published[i + 1].as_str()
                );
            }
            checked += 1;
        }
    }
    assert!(checked > 0);
}

#[test]
fn a_silent_wallet_comes_from_the_paths_the_bip_names() {
    let c = compute();
    let m = words(ABANDON);
    let master = identity::master_of(&c, Material::Words(&m), "", Network::Mainnet).unwrap();
    let receiver = faraday_keys::silent::receiver(&c, &master, 0);
    let secp = Secp256k1::new();
    let spend = master.derive(&c, &faraday_bip::silent::spend_path(Network::Mainnet, 0));
    assert_eq!(
        receiver.spend_public_key(),
        spend.secret_key().public_key(&secp)
    );
    assert!(faraday_keys::silent::address(&receiver, 0).is_some_and(|a| a.starts_with("sp1q")));
}

#[test]
fn a_deal_gives_a_record_and_the_words_of_the_shares_left_to_write_down() {
    let c = compute();
    let share_words = |byte: u8| {
        let m = Mnemonic::from_entropy(Language::English, &[byte; 32]).unwrap();
        m.indices()
            .iter()
            .map(|&i| Language::English.word(i))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let (a, b) = (share_words(7), share_words(9));
    let dealers = [&a, &b].map(|w| faraday_keys::frost::Dealer {
        language: Language::English,
        words: Some(w.as_str()),
    });
    let dealt =
        faraday_keys::frost::make_group(&c, &dealers, 3, 2, Network::Testnet).expect("a deal");
    assert!(faraday_bip::threshold::ThresholdRecord::parse(&c, &dealt.record).is_ok());
    assert_eq!(dealt.words.len(), 1);
    assert_eq!(dealt.words[0].0, 2);
    assert_eq!(dealt.words[0].1.split(' ').count(), 24);
    // A key with no words has no share to deal from.
    let none = [faraday_keys::frost::Dealer {
        language: Language::English,
        words: None,
    }];
    assert!(matches!(
        faraday_keys::frost::make_group(&c, &none, 3, 2, Network::Testnet),
        Err(faraday_keys::frost::DealError::NoShare)
    ));
}

/// RFC 6238's SHA-1 vectors (secret "12345678901234567890"), six digits.
#[test]
fn totp_codes_are_the_ones_rfc_6238_publishes() {
    let secret = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
    for (time, code) in [
        (59, "287082"),
        (1_111_111_109, "081804"),
        (1_234_567_890, "005924"),
        (2_000_000_000, "279037"),
    ] {
        let shown = faraday_keys::totp::code_at(secret, time).expect("a TOTP key");
        assert_eq!(shown.grouped.replace(' ', ""), code, "at {time}");
        assert_eq!(shown.period, 30);
        assert_eq!(u64::from(shown.left), 30 - time % 30);
    }
    // A key read but no time to show it at.
    assert!(matches!(faraday_keys::totp::shown(secret, None), Ok(None)));
    // A counter-based key is no TOTP key.
    assert!(matches!(
        faraday_keys::totp::code_at("otpauth://hotp/x?secret=GEZDGNBVGY3TQOJQ&counter=1", 0),
        Err(faraday_keys::totp::NotTotp::Hotp)
    ));
}
