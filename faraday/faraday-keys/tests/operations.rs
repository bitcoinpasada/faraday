//! What the key operations give a person or a wallet, against the vectors
//! the primitive crates are held to: BIP-85's children, a BIP-137 message
//! signature that verifies for the address its published key has, BIP-352's
//! address, and the fingerprint BIP-39 and BIP-32 publish for the
//! "abandon" words.
#![allow(clippy::disallowed_methods)]

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::bitcoin::bip32::DerivationPath;
use faraday_bip::bitcoin::secp256k1::{Secp256k1, SecretKey};
use faraday_bip::bitcoin::{Address, Network as Chain};
use faraday_bip::codex32::Codex32;
use faraday_bip::keys::{MasterKey, Network, ScriptType};
use faraday_bip::silent::Receiver;
use faraday_bip::slip39::{GroupSpec, Share};
use faraday_crypto::work::Compute;
use faraday_keys::operations::WordsForm;
use faraday_keys::{Convert, Keys, Material, Part, Reveal, bip85, identity};
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

/// The mainnet master key of `m` with no passphrase, made directly: tests
/// may build keys themselves.
fn master(c: &Compute, m: &Mnemonic) -> MasterKey {
    MasterKey::from_seed(c, &m.to_seed(c, b"").unwrap(), Network::Mainnet)
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
    let master = master(&c, &m);
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
                receiver.address(&secp).ok().as_ref().map(|a| a.as_str()),
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
                    receiver
                        .labelled_address(&secp, label)
                        .ok()
                        .as_ref()
                        .map(|a| a.as_str()),
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
    let master = master(&c, &m);
    let mut keys = Keys::on(Network::Mainnet);
    keys.add_words(&c, ABANDON, "abandon", None)
        .expect("the words");
    let receiver = keys.silent_receiver(&c, 0, 0).expect("a receiver");
    let secp = Secp256k1::new();
    let spend = master.derive(&c, &faraday_bip::silent::spend_path(Network::Mainnet, 0));
    let scan = master.derive(&c, &faraday_bip::silent::scan_path(Network::Mainnet, 0));
    assert_eq!(
        receiver.spend_public_key(),
        spend.secret_key().public_key(&secp)
    );
    assert_eq!(
        receiver.scan_public_key(),
        scan.secret_key().public_key(&secp)
    );
    assert!(receiver.address(0).is_some_and(|a| a.starts_with("sp1q")));
    // The scan key leaves only for a reason that names it.
    assert!(receiver.scan_key(Reveal::Words).is_none());
    assert!(
        receiver
            .scan_key(Reveal::OutboxFile(Part::ScanKey))
            .is_some_and(|k| k.starts_with("spscan1q"))
    );
}

/// Two of RFC 6238's SHA-1 vectors (secret "12345678901234567890"), six
/// digits: the vault crate holds the whole published table, this holds
/// that the shown code, period and time left are read from it.
#[test]
fn totp_codes_are_the_ones_rfc_6238_publishes() {
    let secret = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
    for (time, code) in [(59, "287082"), (1_111_111_109, "081804")] {
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

#[test]
fn keys_added_from_words_come_back_with_their_fingerprint_and_word_count() {
    use faraday_keys::{Keys, Part, Reveal};
    let c = compute();
    let mut keys = Keys::on(Network::Testnet);
    let fp = keys
        .add_words(&c, ABANDON, "", None)
        .expect("the key loads");
    assert_eq!(fp.0, [0x73, 0xc5, 0xda, 0x0a]);
    let info = keys.info(0).expect("the key is held");
    assert_eq!(info.fingerprint, fp);
    assert_eq!(info.word_count, Some(12));
    assert!(!info.has_passphrase);
    // With no label the fingerprint names it.
    assert_eq!(info.label, "73c5da0a");
    // The words come back as they were loaded, and a key with no
    // passphrase has none to show.
    assert_eq!(&*keys.reveal(0, Reveal::Words), ABANDON);
    assert_eq!(&*keys.reveal(0, Reveal::OutboxFile(Part::Words)), ABANDON);
    assert!(keys.reveal(0, Reveal::Passphrase).is_empty());
    // The same words with a passphrase are another key; the same key twice
    // is refused.
    let other = keys
        .add_words_with(&c, ABANDON, "TREZOR", "With a passphrase", None)
        .expect("the second key loads");
    assert_ne!(other, fp);
    assert!(keys.info(1).is_some_and(|k| k.has_passphrase));
    assert_eq!(&*keys.reveal(1, Reveal::Passphrase), "TREZOR");
    assert!(keys.passphrase_matches(1, "TREZOR"));
    assert!(!keys.passphrase_matches(1, "trezor"));
    assert!(!keys.passphrase_matches(0, ""));
    assert!(keys.add_words(&c, ABANDON, "", None).is_err());
    // The two share their words.
    assert_eq!(keys.with_same_words(&c, 0), vec![0, 1]);
    assert!(keys.same_words(&c, 0, 1));
    // The same entropy in Spanish's list is another wallet: its words are
    // not the English key's.
    let spanish = Mnemonic::parse(
        &c,
        Language::Spanish,
        "ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco abierto",
    )
    .expect("the Spanish words");
    let es = keys
        .add_mnemonic(&c, &spanish, "", "Spanish", None)
        .expect("the Spanish key loads");
    assert_ne!(es, fp);
    assert_eq!(keys.with_same_words(&c, 0), vec![0, 1]);
    assert_eq!(keys.with_same_words(&c, 2), vec![2]);
    keys.remove(2);
    // The key a passphrase would make is told before it is added.
    assert_eq!(keys.fingerprint_with(&c, 0, "TREZOR"), Some(other));
    assert_eq!(keys.position(other), Some(1));
    keys.remove(0);
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].fingerprint, other);
}

#[test]
fn a_key_from_a_master_seed_has_no_words_to_show() {
    use faraday_keys::{Keys, Reveal};
    let c = compute();
    let mut keys = Keys::on(Network::Testnet);
    let seed = [7u8; 32];
    let fp = keys.add_seed(&c, &seed, "A seed").expect("the seed loads");
    let info = keys.info(0).expect("the key is held");
    assert_eq!(info.fingerprint, fp);
    assert_eq!(info.word_count, None);
    assert!(!info.has_words());
    assert!(keys.reveal(0, Reveal::Words).is_empty());
    // A seed longer than the master seeds there are is refused.
    assert!(keys.add_seed(&c, &[1u8; 65], "Too long").is_err());
}

#[test]
fn a_copy_of_the_words_is_compared_by_their_numbers() {
    use faraday_keys::{CopyCheck, Keys};
    let c = compute();
    let mut keys = Keys::on(Network::Testnet);
    keys.add_words(&c, ABANDON, "", None)
        .expect("the key loads");
    let right: Vec<u16> = words(ABANDON).indices().to_vec();
    assert_eq!(keys.words_match(&c, 0, &right), Some(CopyCheck::Matches));
    let mut wrong = right.clone();
    wrong[4] ^= 1;
    assert_eq!(
        keys.words_match(&c, 0, &wrong),
        Some(CopyCheck::WrongWord(5))
    );
    // A copy that stops short is wrong at the word it lacks.
    assert_eq!(
        keys.words_match(&c, 0, &right[..11]),
        Some(CopyCheck::WrongWord(12))
    );
    // Another seed's words, BIP-39's own vector, are another seed.
    let other =
        words("legal winner thank year wave sausage worth useful legal winner thank yellow");
    assert_eq!(
        keys.words_match(&c, 0, other.indices()),
        Some(CopyCheck::AnotherSeed(1))
    );
    // Most words different but no checksum of their own: a copy wrong
    // at its first word.
    let mut mixed = other.indices()[..7].to_vec();
    mixed.extend_from_slice(&right[7..]);
    assert_eq!(
        keys.words_match(&c, 0, &mixed),
        Some(CopyCheck::WrongWord(1))
    );
    assert_eq!(keys.words_match(&c, 3, &right), None);
}

#[test]
fn the_vault_record_of_a_key_holds_its_words_and_passphrase_when_asked() {
    use faraday_keys::{Keys, Part, Reveal};
    use faraday_vault::records::{field, kind};
    let c = compute();
    let mut keys = Keys::on(Network::Testnet);
    keys.add_words_with(&c, ABANDON, "TREZOR", "Key", None)
        .expect("the key loads");
    let plain = keys
        .key_record(&c, 0, None, false, &[], Reveal::VaultEntry(Part::Words))
        .expect("a record");
    assert_eq!(plain.kind, kind::KEY);
    assert!(plain.field(field::KEY_PASSPHRASE).is_none());
    assert_eq!(plain.field(field::KEY_LABEL), Some(&b"Key"[..]));
    let with = keys
        .key_record(&c, 0, Some(1), true, &[], Reveal::VaultEntry(Part::Words))
        .expect("a record");
    assert_eq!(with.field(field::KEY_PASSPHRASE), Some(&b"TREZOR"[..]));
    assert_eq!(with.field(field::KEY_FLAGS), Some(&[1u8][..]));
    // The same words alone, without the passphrase, are the key BIP-32
    // publishes for them.
    assert_eq!(
        keys.words_identity(&c, 0).map(|(f, _)| f.0),
        Some([0x73, 0xc5, 0xda, 0x0a])
    );
    assert_ne!(
        keys.info(0).map(|k| k.fingerprint.0),
        Some([0x73, 0xc5, 0xda, 0x0a])
    );
}

/// What Explore's eyes show of a key, against published vectors: the root
/// extended private key of BIP-32's test vector 1 and of BIP 93's codex32
/// vectors, the account extended private key BIP-84 publishes for the
/// "abandon" words, the private key of the first address there (it pays
/// to the address BIP-84 publishes), a codex32 key's seed, secret and
/// shares as BIP 93 writes them; and that each reason gives only its own
/// secret.
mod explore {
    use std::str::FromStr;

    use super::{ABANDON, compute, unhex};
    use faraday_bip::bitcoin::bip32::DerivationPath;
    use faraday_bip::bitcoin::secp256k1::Secp256k1;
    use faraday_bip::bitcoin::{
        Address, CompressedPublicKey, Network as Chain, PrivateKey, base58,
    };
    use faraday_bip::keys::Network;
    use faraday_crypto::work::Compute;
    use faraday_keys::{Keys, Part, Reveal};
    use serde_json::Value;

    const BIP93: &str = include_str!("../../../tools/vectors/bip93/vectors.json");
    fn path(p: &str) -> DerivationPath {
        DerivationPath::from_str(p).unwrap()
    }

    /// Every reason but the three Explore's eyes name.
    const OTHERS: [Reveal; 14] = [
        Reveal::Words,
        Reveal::Passphrase,
        Reveal::SeedQr,
        Reveal::CopyByHand,
        Reveal::PaperForm,
        Reveal::Bip85Child,
        Reveal::LightningKey,
        Reveal::GpgPaperkey,
        Reveal::VaultEntry(Part::Words),
        Reveal::OutboxFile(Part::Words),
        Reveal::BackupStick(Part::Words),
        Reveal::VaultEntry(Part::Passphrase),
        Reveal::BackupStick(Part::Codex32),
        Reveal::KefCode,
    ];

    fn abandon(c: &Compute) -> Keys {
        let mut keys = Keys::on(Network::Mainnet);
        keys.add_words_with(c, ABANDON, "", "abandon", None)
            .expect("the abandon words");
        keys
    }

    #[test]
    fn the_root_extended_private_key_is_bip32s_vector_1() {
        let c = compute();
        let mut keys = Keys::on(Network::Mainnet);
        keys.add_seed(&c, &unhex("000102030405060708090a0b0c0d0e0f"), "")
            .unwrap();
        let root = keys
            .xprv_at(&c, 0, &DerivationPath::master(), Reveal::ExtendedPrivateKey)
            .unwrap();
        assert_eq!(
            root.as_str(),
            "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi"
        );
        let at = keys
            .xprv_at(&c, 0, &path("m/0h/1"), Reveal::ExtendedPrivateKey)
            .unwrap();
        assert_eq!(
            at.as_str(),
            "xprv9wTYmMFdV23N2TdNG573QoEsfRrWKQgWeibmLntzniatZvR9BmLnvSxqu53Kw1UmYPxLgboyZQaXwTCg8MSY3H2EU4pWcQDnRnrVA1xe8fs"
        );
    }

    #[test]
    fn the_account_extended_private_key_is_bip84s() {
        let c = compute();
        let keys = abandon(&c);
        let shown = keys
            .xprv_at(&c, 0, &path("m/84h/0h/0h"), Reveal::ExtendedPrivateKey)
            .unwrap();
        // BIP-84 publishes it as a zprv; Faraday writes the xprv form, the
        // same 78 bytes under BIP-32's version.
        let zprv = "zprvAdG4iTXWBoARxkkzNpNh8r6Qag3irQB8PzEMkAFeTRXxHpbF9z4QgEvBRmfvqWvGp42t42nvgGpNgYSJA9iefm1yYNZKEm7z6qUWCroSQnE";
        let published = base58::decode_check(zprv).unwrap();
        let ours = base58::decode_check(shown.as_str()).unwrap();
        assert_eq!(ours[..4], [0x04, 0x88, 0xad, 0xe4]);
        assert_eq!(ours[4..], published[4..]);
    }

    #[test]
    fn the_first_addresss_private_key_pays_to_bip84s_first_address() {
        let c = compute();
        let keys = abandon(&c);
        let wif = keys
            .wif_at(&c, 0, &path("m/84h/0h/0h/0/0"), Reveal::AddressKey)
            .unwrap();
        let key = PrivateKey::from_wif(wif.as_str()).unwrap();
        assert!(key.compressed);
        assert_eq!(key.network, Chain::Bitcoin.into());
        let public = CompressedPublicKey::from_private_key(&Secp256k1::new(), &key).unwrap();
        assert_eq!(
            Address::p2wpkh(&public, Chain::Bitcoin).to_string(),
            "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu"
        );
        // BIP-84's published private key of that address.
        assert_eq!(
            wif.as_str(),
            "KyZpNDKnfs94vbrwhJneDi77V6jF64PWPF8x5cdJb8ifgg2DUc9d"
        );
    }

    #[test]
    fn a_codex32_keys_seed_is_shown_as_bip93_writes_it() {
        let c = compute();
        let v: Value = serde_json::from_str(BIP93).unwrap();
        let set = v["sets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["vector"] == "2")
            .unwrap();
        let mut keys = Keys::on(Network::Mainnet);
        keys.add_seed(&c, &unhex(set["seed"].as_str().unwrap()), "")
            .unwrap();
        // No set known: the seed itself, in hex.
        let hex = keys.seed_shown(0, Reveal::SeedShown).unwrap();
        assert_eq!(hex.len(), 1);
        assert_eq!(hex[0].as_str(), set["seed"].as_str().unwrap());
        // The set known and its two shares typed back: the seed, the secret
        // they recover as BIP 93 writes it, then the shares by index.
        assert!(keys.set_codex32(
            0,
            Some(faraday_vault::codex32::SetFacts {
                threshold: 2,
                identifier: *b"name",
                length: 48,
                unknown: 0,
                strings: Vec::new(),
            })
        ));
        let shares: Vec<String> = set["shares"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_lowercase())
            .collect();
        let refs: Vec<&str> = shares.iter().map(String::as_str).rev().collect();
        assert_eq!(keys.keep_codex32(&c, 0, &refs), 2);
        let shown = keys.seed_shown(0, Reveal::SeedShown).unwrap();
        let shown: Vec<&str> = shown.iter().map(|s| s.as_str()).collect();
        let secret = set["secret"].as_str().unwrap().to_lowercase();
        assert_eq!(
            shown,
            [
                set["seed"].as_str().unwrap(),
                secret.as_str(),
                &shares[0],
                &shares[1]
            ]
        );
        // One share alone recovers nothing: the seed and that share.
        let facts = faraday_vault::codex32::SetFacts {
            threshold: 2,
            identifier: *b"name",
            length: 48,
            unknown: 0,
            strings: Vec::new(),
        };
        assert!(keys.set_codex32(0, None));
        assert!(keys.set_codex32(0, Some(facts)));
        // A share alone cannot be shown to be of the seed: not kept.
        assert_eq!(keys.keep_codex32(&c, 0, &[shares[1].as_str()]), 0);
        let shown = keys.seed_shown(0, Reveal::SeedShown).unwrap();
        let shown: Vec<&str> = shown.iter().map(|s| s.as_str()).collect();
        assert_eq!(shown, [set["seed"].as_str().unwrap()]);
        // Its root extended private key is the vector's.
        let root = keys
            .xprv_at(&c, 0, &DerivationPath::master(), Reveal::ExtendedPrivateKey)
            .unwrap();
        assert_eq!(root.as_str(), set["xprv"].as_str().unwrap());
    }

    /// A key made from BIP 93's vector 2 seed, its set facts known, and a
    /// share of another seed with that set's header, at index `a`.
    fn vector_two_and_a_stranger(c: &Compute) -> (Keys, Vec<String>, String) {
        let v: Value = serde_json::from_str(BIP93).unwrap();
        let set = v["sets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["vector"] == "2")
            .unwrap();
        let mut keys = Keys::on(Network::Mainnet);
        keys.add_seed(c, &unhex(set["seed"].as_str().unwrap()), "")
            .unwrap();
        assert!(keys.set_codex32(
            0,
            Some(faraday_vault::codex32::SetFacts {
                threshold: 2,
                identifier: *b"name",
                length: 48,
                unknown: 0,
                strings: Vec::new(),
            })
        ));
        let shares: Vec<String> = set["shares"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_lowercase())
            .collect();
        let other = faraday_bip::codex32::Codex32::from_seed(2, *b"name", &[7u8; 16]).unwrap();
        let stranger = other
            .split(2, 3, &[9u8; 64])
            .unwrap()
            .into_iter()
            .find(|s| s.share_index() == b'a')
            .unwrap()
            .encode()
            .as_str()
            .to_string();
        assert_ne!(stranger, shares[0]);
        (keys, shares, stranger)
    }

    #[test]
    fn a_share_of_another_seed_is_not_kept_for_a_key() {
        let c = compute();
        let (mut keys, shares, stranger) = vector_two_and_a_stranger(&c);
        assert_eq!(keys.keep_codex32(&c, 0, &[stranger.as_str()]), 0);
        assert_eq!(
            keys.keep_codex32(&c, 0, &[stranger.as_str(), shares[1].as_str()]),
            0
        );
        let shown = keys.seed_shown(0, Reveal::SeedShown).unwrap();
        assert!(shown.iter().all(|s| s.as_str() != stranger));
    }

    #[test]
    fn the_right_shares_are_kept_beside_a_wrong_one_at_the_same_index() {
        let c = compute();
        let (mut keys, shares, stranger) = vector_two_and_a_stranger(&c);
        let given = [stranger.as_str(), shares[0].as_str(), shares[1].as_str()];
        assert_eq!(keys.keep_codex32(&c, 0, &given), 2);
        assert!(keys.holds_codex32(0, b'a'));
        assert!(keys.holds_codex32(0, b'c'));
        let shown = keys.seed_shown(0, Reveal::SeedShown).unwrap();
        let shown: Vec<&str> = shown.iter().map(|s| s.as_str()).collect();
        assert!(shown.contains(&shares[0].as_str()));
        assert!(shown.contains(&shares[1].as_str()));
        assert!(!shown.contains(&stranger.as_str()));
    }

    #[test]
    fn each_reason_gives_only_its_own_secret() {
        let c = compute();
        let mut keys = abandon(&c);
        keys.add_seed(&c, &unhex("000102030405060708090a0b0c0d0e0f"), "")
            .unwrap();
        let at = path("m/84h/0h/0h/0/0");
        for why in OTHERS {
            assert!(keys.xprv_at(&c, 0, &at, why).is_none(), "{why:?}");
            assert!(keys.wif_at(&c, 0, &at, why).is_none(), "{why:?}");
            assert!(keys.seed_shown(1, why).is_none(), "{why:?}");
        }
        // Explore's reasons lend no stored text: never the words, never the
        // passphrase, of a key or of a vault record.
        for why in [
            Reveal::SeedShown,
            Reveal::ExtendedPrivateKey,
            Reveal::AddressKey,
        ] {
            assert_eq!(&*keys.reveal(0, why), "", "{why:?}");
            assert!(keys.xprv_at(&c, 0, &at, Reveal::AddressKey).is_none());
            assert!(
                keys.wif_at(&c, 0, &at, Reveal::ExtendedPrivateKey)
                    .is_none()
            );
        }
        // A key of words has no seed to show: its words are lent instead.
        assert!(keys.seed_shown(0, Reveal::SeedShown).is_none());
        assert_eq!(&*keys.reveal(0, Reveal::Words), ABANDON);
    }
}

const BIP39: &str = include_str!("../../../tools/vectors/bip39/vectors.json");
const BIP93: &str = include_str!("../../../tools/vectors/bip93/vectors.json");
const SLIP39: &str = include_str!("../../../tools/vectors/slip39/vectors.json");
const R: usize = 8 * 103;

/// The master seeds, and the xprv where the vector gives one, of SLIP-39's
/// valid vectors of one master secret (16 and 32 bytes) and of BIP 93's
/// (16, 20, 24, 28, 32 and 64 bytes).
fn seed_vectors(set: &str) -> Vec<(Vec<u8>, Option<String>)> {
    let v: Value = serde_json::from_str(set).unwrap();
    if let Some(list) = v.as_array() {
        return list
            .iter()
            .filter(|x| !x[2].as_str().unwrap().is_empty())
            .filter(|x| x[1].as_array().unwrap().len() == 1)
            .map(|x| {
                (
                    unhex(x[2].as_str().unwrap()),
                    Some(x[3].as_str().unwrap().into()),
                )
            })
            .collect();
    }
    v["secrets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| {
            (
                unhex(x["seed"].as_str().unwrap()),
                x["xprv"].as_str().map(String::from),
            )
        })
        .collect()
}

fn seed_key(c: &Compute, network: Network, seed: &[u8]) -> Keys {
    let mut keys = Keys::on(network);
    keys.add_seed(c, seed, "").unwrap();
    keys
}

/// Every conversion of key 0, in the order: hex, xprv, SLIP-39, codex32.
fn all(c: &Compute, keys: &Keys, why: Reveal) -> [Result<String, Convert>; 4] {
    let text = |r: Result<faraday_keys::Held<zeroize::Zeroizing<String>>, Convert>| {
        r.map(|h| h.release().as_str().to_string())
    };
    [
        text(keys.master_seed_hex(c, 0, why)),
        text(keys.master_xprv(c, 0, why)),
        text(keys.slip39_one_of_one(c, 0, why)),
        keys.codex32_converted(c, 0, 0, 1, (&[], &[]), why)
            .map(|h| h.release()[0].as_str().to_string()),
    ]
}

/// BIP-39's published vector, with the passphrase TREZOR: the seed and the
/// xprv; SLIP-39 and codex32 are not made from words.
#[test]
fn a_key_of_words_gives_its_seed_and_xprv_and_no_slip39_or_codex32() {
    let c = compute();
    let v: Value = serde_json::from_str(BIP39).unwrap();
    let row = &v["english"][0];
    let mut keys = Keys::on(Network::Mainnet);
    keys.add_words_with(&c, row[1].as_str().unwrap(), "TREZOR", "", None)
        .unwrap();
    let [hex, xprv, slip, codex] = all(&c, &keys, Reveal::PaperForm);
    assert_eq!(hex.unwrap(), row[2].as_str().unwrap());
    assert_eq!(xprv.unwrap(), row[3].as_str().unwrap());
    assert_eq!(slip, Err(Convert::Words));
    assert_eq!(codex, Err(Convert::Words));
}

/// A key loaded from a published master secret writes it back as hex and
/// its published xprv; its 1 of 1 and codex32 each load as a new key with
/// the same fingerprint.
#[test]
fn a_key_of_seed_bytes_converts_to_the_same_wallet() {
    let c = compute();
    let mut sets = seed_vectors(SLIP39);
    sets.extend(seed_vectors(BIP93));
    assert!(sets.len() >= 8);
    let mut lengths = Vec::new();
    for (seed, xprv) in sets {
        let keys = seed_key(&c, Network::Mainnet, &seed);
        let want = keys.info(0).unwrap().fingerprint;
        let [hex, x, slip, codex] = all(&c, &keys, Reveal::PaperForm);
        assert_eq!(unhex(&hex.unwrap()), seed);
        if let Some(xprv) = xprv {
            assert_eq!(x.unwrap(), xprv);
        }
        match slip {
            Ok(words) => {
                assert!(seed.len() == 16 || seed.len() == 32);
                let share = Share::parse(&words).unwrap();
                let back = faraday_keys::slip39_seed(&c, &[share], b"").unwrap();
                let fresh = seed_key(&c, Network::Mainnet, &back);
                assert_eq!(fresh.info(0).unwrap().fingerprint, want);
            }
            Err(e) => {
                assert!(seed.len() != 16 && seed.len() != 32);
                assert_eq!(e, Convert::Length);
                lengths.push(seed.len());
            }
        }
        let parts = [Codex32::parse(&codex.unwrap()).unwrap()];
        let back = faraday_keys::codex32_seed(&parts).unwrap();
        assert_eq!(*back, seed);
        let fresh = seed_key(&c, Network::Mainnet, &back);
        assert_eq!(fresh.info(0).unwrap().fingerprint, want);
    }
    for n in [20, 24, 28] {
        assert!(lengths.contains(&n), "{n} bytes reached Convert::Length");
    }
}

/// Every share of a codex32 set of a key reads, and the last `k` recover
/// the key as the first `k` do.
#[test]
fn a_codex32_set_of_a_key_recovers_the_same_wallet() {
    let c = compute();
    let seed = unhex("ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100");
    let keys = seed_key(&c, Network::Mainnet, &seed);
    let want = keys.info(0).unwrap().fingerprint;
    let random: Vec<u8> = (0..R).map(|i| (i * 7 + 3) as u8).collect();
    let shares = keys
        .codex32_converted(&c, 0, 2, 3, (&random, &[]), Reveal::PaperForm)
        .unwrap()
        .release();
    assert_eq!(shares.len(), 3);
    let parts: Vec<Codex32> = shares.iter().map(|s| Codex32::parse(s).unwrap()).collect();
    for part in [&parts[..2], &parts[1..]] {
        let back = faraday_keys::codex32_seed(part).unwrap();
        let fresh = seed_key(&c, Network::Mainnet, &back);
        assert_eq!(fresh.info(0).unwrap().fingerprint, want);
    }
}

/// A codex32 set made for a key takes none of the identifiers other sets
/// on the device carry, whatever the randomness drew.
#[test]
fn a_codex32_set_of_a_key_never_takes_an_identifier_already_taken() {
    let c = compute();
    let seed = unhex("ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100");
    let keys = seed_key(&c, Network::Mainnet, &seed);
    let random: Vec<u8> = (0..R).map(|i| (i * 7 + 3) as u8).collect();
    let id_of = |taken: &[[u8; 4]]| {
        let shares = keys
            .codex32_converted(&c, 0, 2, 3, (&random, taken), Reveal::PaperForm)
            .unwrap()
            .release();
        Codex32::parse(&shares[0])
            .unwrap()
            .identifier()
            .map(|b| b.to_ascii_lowercase())
    };
    let first = id_of(&[]);
    let second = id_of(&[first]);
    assert_ne!(second, first);
    let third = id_of(&[first, second]);
    assert!(third != first && third != second);
}

/// Where every identifier that shares the draw's first three characters is
/// taken, the set still gets one that is none of them.
#[test]
fn a_codex32_set_finds_a_free_identifier_when_a_whole_run_of_them_is_taken() {
    let c = compute();
    let seed = unhex("ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100");
    let keys = seed_key(&c, Network::Mainnet, &seed);
    let random: Vec<u8> = (0..R).map(|i| (i * 7 + 3) as u8).collect();
    let id_of = |taken: &[[u8; 4]]| {
        let shares = keys
            .codex32_converted(&c, 0, 2, 3, (&random, taken), Reveal::PaperForm)
            .unwrap()
            .release();
        Codex32::parse(&shares[0])
            .unwrap()
            .identifier()
            .map(|b| b.to_ascii_lowercase())
    };
    let first = id_of(&[]);
    let run: Vec<[u8; 4]> = (0..32)
        .map(|k| {
            let mut id = first;
            id[3] = faraday_bip::codex32::CHARSET[k];
            id
        })
        .collect();
    let got = id_of(&run);
    assert!(!run.contains(&got), "{got:?}");
}

/// A codex32 set of a key takes its identifier from the randomness: never
/// the fingerprint's nor the key's own set's, a different one for other
/// randomness, and shares of two such sets do not recover a seed together.
/// The secret alone keeps the fingerprint's, and a draw too short for the
/// identifier is refused.
#[test]
fn a_codex32_set_of_a_key_has_an_identifier_of_its_own_each_time() {
    let c = compute();
    let seed = unhex("000102030405060708090a0b0c0d0e0f");
    let mut keys = seed_key(&c, Network::Mainnet, &seed);
    let fp = keys.info(0).unwrap().fingerprint;
    let own = faraday_bip::codex32::identifier_for(fp.0);
    let set = |keys: &Keys, base: usize| {
        let random: Vec<u8> = (0..faraday_keys::codex32_random_len(2))
            .map(|i| (i * 11 + base) as u8)
            .collect();
        keys.codex32_converted(&c, 0, 2, 3, (&random, &[]), Reveal::PaperForm)
            .map(|h| h.release())
    };
    let parse = |set: &[zeroize::Zeroizing<String>]| -> Vec<Codex32> {
        set.iter().map(|s| Codex32::parse(s).unwrap()).collect()
    };
    let (sa, sb) = (set(&keys, 3).unwrap(), set(&keys, 9).unwrap());
    let (a, b) = (parse(&sa), parse(&sb));
    assert_ne!(a[0].identifier(), b[0].identifier());
    assert!(a[0].identifier() != own && b[0].identifier() != own);
    assert!(a.iter().all(|s| s.identifier() == a[0].identifier()));
    assert!(faraday_keys::codex32_seed(&parse(&[sa[0].clone(), sb[1].clone()])).is_err());
    assert_eq!(*faraday_keys::codex32_seed(&a[1..]).unwrap(), seed);
    // The key's own set's identifier is not drawn either: randomness that
    // would give it is stepped on to another.
    let drawn: Vec<u8> = a[0].identifier().to_vec();
    keys.set_codex32(
        0,
        Some(faraday_vault::codex32::SetFacts {
            threshold: 2,
            identifier: a[0].identifier(),
            length: 48,
            unknown: 0,
            strings: Vec::new(),
        }),
    );
    let again = parse(&set(&keys, 3).unwrap());
    assert_ne!(again[0].identifier().to_vec(), drawn);
    assert_ne!(again[0].identifier(), own);
    // The secret alone is fixed by the fingerprint and draws nothing.
    let alone = keys
        .codex32_converted(&c, 0, 0, 1, (&[], &[]), Reveal::PaperForm)
        .unwrap()
        .release();
    assert_eq!(Codex32::parse(&alone[0]).unwrap().identifier(), own);
    // Too little randomness for the identifier is refused.
    // A 16-byte seed's share takes its symbols, then four for the identifier.
    let short = vec![1u8; faraday_bip::codex32::symbols_for(16) + 3];
    assert_eq!(
        keys.codex32_converted(&c, 0, 2, 3, (&short, &[]), Reveal::PaperForm)
            .map(|h| h.release()),
        Err(Convert::Refused)
    );
}

/// Seed XOR is given for a key on the English list only: a published
/// Japanese BIP-39 vector key is refused, whatever randomness is given.
#[test]
fn seed_xor_is_not_given_for_a_key_on_another_list() {
    let c = compute();
    let jp: Value = serde_json::from_str(include_str!(
        "../../../tools/vectors/bip39/test_JP_BIP39.json"
    ))
    .unwrap();
    let words = jp[0]["mnemonic"].as_str().unwrap();
    let mut keys = Keys::on(Network::Mainnet);
    keys.add_words_with(&c, words, "", "", None).unwrap();
    assert_eq!(
        keys.seed_xor_converted(&c, 0, 3, &[7u8; 96], Reveal::PaperForm)
            .map(|h| h.release()),
        Err(Convert::NotGiven)
    );
}

/// A SLIP-39 set of a key, as SLIP-39's decoder reads it: the key's seed
/// from the shares at `at` of `shares`.
fn slip39_back(c: &Compute, shares: &[zeroize::Zeroizing<String>], at: &[usize]) -> Vec<u8> {
    let parts: Vec<Share> = at
        .iter()
        .map(|&i| Share::parse(&shares[i]).unwrap())
        .collect();
    faraday_keys::slip39_seed(c, &parts, b"").unwrap().to_vec()
}

fn slip39_set(
    c: &Compute,
    keys: &Keys,
    gt: u8,
    shape: &[(u8, u8)],
) -> Result<Vec<zeroize::Zeroizing<String>>, Convert> {
    let groups: Vec<GroupSpec> = shape
        .iter()
        .map(|&(threshold, count)| GroupSpec { threshold, count })
        .collect();
    let random: Vec<u8> = (0..faraday_keys::slip39_random_len(
        keys.info(0).and_then(|k| k.seed_len).unwrap_or(32),
        gt,
        &groups,
    ))
        .map(|i| (i * 13 + 5) as u8)
        .collect();
    keys.slip39_converted(c, 0, gt, &groups, &random, Reveal::PaperForm)
        .map(|h| h.release())
}

/// Every two of a 2 of 3 set of each published master secret (16 and 32
/// bytes) recover that secret through SLIP-39's decoder.
#[test]
fn a_slip39_two_of_three_of_a_key_recovers_the_seed_from_any_two() {
    let c = compute();
    let mut lengths = Vec::new();
    for (seed, _) in seed_vectors(SLIP39) {
        let keys = seed_key(&c, Network::Mainnet, &seed);
        let shares = slip39_set(&c, &keys, 1, &[(2, 3)]).unwrap();
        assert_eq!(shares.len(), 3);
        for at in [[0, 1], [0, 2], [1, 2], [2, 1]] {
            assert_eq!(slip39_back(&c, &shares, &at), seed);
        }
        lengths.push(seed.len());
    }
    assert!(lengths.contains(&16) && lengths.contains(&32));
}

/// A groups shape: every pair of groups, with each group's threshold of
/// members, recovers the seed; one share short of a group does not.
#[test]
fn a_slip39_set_of_groups_recovers_the_seed_from_every_qualifying_subset() {
    let c = compute();
    let seed = unhex("ffeeddccbbaa99887766554433221100");
    let keys = seed_key(&c, Network::Mainnet, &seed);
    let shares = slip39_set(&c, &keys, 2, &[(2, 3), (2, 3), (1, 1)]).unwrap();
    assert_eq!(shares.len(), 7);
    let groups = [[0, 1, 2], [3, 4, 5], [6, 6, 6]];
    let pairs = |g: &[usize; 3]| -> Vec<Vec<usize>> {
        if g[0] == g[1] {
            vec![vec![g[0]]]
        } else {
            vec![vec![g[0], g[1]], vec![g[0], g[2]], vec![g[1], g[2]]]
        }
    };
    for (a, b) in [(0, 1), (0, 2), (1, 2)] {
        for x in pairs(&groups[a]) {
            for y in pairs(&groups[b]) {
                let at: Vec<usize> = x.iter().chain(&y).copied().collect();
                assert_eq!(slip39_back(&c, &shares, &at), seed, "{at:?}");
            }
        }
    }
    // One group alone is not enough.
    let parts: Vec<Share> = [0, 1]
        .iter()
        .map(|&i| Share::parse(&shares[i]).unwrap())
        .collect();
    assert!(faraday_keys::slip39_seed(&c, &parts, b"").is_err());
    for share in &shares {
        let share = Share::parse(share).unwrap();
        assert_eq!(share.group_threshold(), 2);
        assert_eq!(share.group_count(), 3);
    }
}

/// What the set refuses: a key of words, another length, another reason, a
/// shape SLIP-39 does not take and too little randomness; and the same
/// randomness gives the same set.
#[test]
fn a_slip39_set_is_refused_for_the_wrong_key_reason_shape_or_randomness() {
    let c = compute();
    let keys = seed_key(
        &c,
        Network::Mainnet,
        &unhex("000102030405060708090a0b0c0d0e0f"),
    );
    let again = slip39_set(&c, &keys, 1, &[(2, 3)]).unwrap();
    assert_eq!(again, slip39_set(&c, &keys, 1, &[(2, 3)]).unwrap());
    for (gt, shape) in [
        (1, vec![(1, 3)]),
        (1, vec![(4, 3)]),
        (2, vec![(1, 1)]),
        (1, vec![(2, 3), (2, 3)]),
        (0, vec![(2, 3)]),
        (1, vec![(2, 3); 17]),
        (1, vec![(2, 17)]),
    ] {
        assert_eq!(
            slip39_set(&c, &keys, gt, &shape),
            Err(Convert::Refused),
            "{gt} {shape:?}"
        );
    }
    let groups = [GroupSpec {
        threshold: 2,
        count: 3,
    }];
    assert_eq!(
        keys.slip39_converted(&c, 0, 1, &groups, &[1; 5], Reveal::PaperForm)
            .err(),
        Some(Convert::Refused)
    );
    for why in [
        Reveal::Words,
        Reveal::CopyByHand,
        Reveal::SeedShown,
        Reveal::ExtendedPrivateKey,
    ] {
        assert_eq!(
            keys.slip39_converted(&c, 0, 1, &groups, &[1; 200], why)
                .err(),
            Some(Convert::NotGiven)
        );
    }
    let mut words = Keys::on(Network::Mainnet);
    words
        .add_words(&c, "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about", "", None)
        .unwrap();
    assert_eq!(
        words
            .slip39_converted(&c, 0, 1, &groups, &[1; 200], Reveal::PaperForm)
            .err(),
        Some(Convert::Words)
    );
    let long = seed_key(&c, Network::Mainnet, &[7; 24]);
    assert_eq!(
        long.slip39_converted(&c, 0, 1, &groups, &[1; 200], Reveal::PaperForm)
            .err(),
        Some(Convert::Length)
    );
}

/// On a test network the master key is a tprv.
#[test]
fn a_test_network_gives_a_tprv() {
    let c = compute();
    let keys = seed_key(
        &c,
        Network::Testnet,
        &unhex("000102030405060708090a0b0c0d0e0f"),
    );
    let x = keys
        .master_xprv(&c, 0, Reveal::PaperForm)
        .unwrap()
        .release();
    assert!(x.starts_with("tprv"));
}

/// Any reason but the paper form gives nothing, from any of the four.
#[test]
fn another_reason_gives_nothing() {
    let c = compute();
    let keys = seed_key(
        &c,
        Network::Mainnet,
        &unhex("000102030405060708090a0b0c0d0e0f"),
    );
    for why in [
        Reveal::Words,
        Reveal::CopyByHand,
        Reveal::SeedShown,
        Reveal::ExtendedPrivateKey,
    ] {
        for r in all(&c, &keys, why) {
            assert_eq!(r, Err(Convert::NotGiven));
        }
    }
    let mut words = Keys::on(Network::Mainnet);
    words
        .add_words(&c, "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about", "", None)
        .unwrap();
    for r in all(&c, &words, Reveal::Words) {
        assert_eq!(r, Err(Convert::NotGiven));
    }
}

fn written(keys: &Keys, c: &Compute, form: WordsForm) -> Result<String, Convert> {
    keys.words_written(c, 0, form, Reveal::PaperForm)
        .map(|h| h.release().as_str().to_string())
}

/// BIP-39's vectors for 12 and 24 English words: each way of writing the
/// words, against the vector's entropy and words.
#[test]
fn a_key_of_words_is_written_as_the_words_the_entropy_the_numbers_and_the_starts() {
    let c = compute();
    let v: Value = serde_json::from_str(BIP39).unwrap();
    for at in [0, 8, 3, 11] {
        let row = &v["english"][at];
        let words = row[1].as_str().unwrap();
        let mut keys = Keys::on(Network::Mainnet);
        keys.add_words_with(&c, words, "", "", None).unwrap();
        assert_eq!(written(&keys, &c, WordsForm::Words).unwrap(), words);
        assert_eq!(
            written(&keys, &c, WordsForm::HexEntropy).unwrap(),
            row[0].as_str().unwrap()
        );
        let m = Mnemonic::parse(&c, Language::English, words).unwrap();
        let numbers: Vec<String> = m
            .indices()
            .iter()
            .map(|&n| format!("{} ({n})", n + 1))
            .collect();
        assert_eq!(
            written(&keys, &c, WordsForm::Numbers).unwrap(),
            numbers.join("\n")
        );
        let starts: Vec<String> = words
            .split(' ')
            .map(|w| w.chars().take(4).collect())
            .collect();
        assert_eq!(
            written(&keys, &c, WordsForm::Starts).unwrap(),
            starts.join(" ")
        );
    }
}

/// SeedQR and CompactSeedQR of BIP-39's vectors: the matrix is the one this
/// crate's encoder makes of the words, with the word count; the digits are
/// each word's number, four to a word. That a scanner reads the matrix is
/// the flow's check, in `faraday-core`'s `convert` tests.
#[test]
fn a_key_of_words_is_written_as_a_seedqr_and_a_compact_seedqr() {
    let c = compute();
    let v: Value = serde_json::from_str(BIP39).unwrap();
    for at in [0, 8] {
        let words = v["english"][at][1].as_str().unwrap();
        let mut keys = Keys::on(Network::Mainnet);
        keys.add_words_with(&c, words, "", "", None).unwrap();
        let m = Mnemonic::parse(&c, Language::English, words).unwrap();
        for compact in [false, true] {
            let (code, count) = keys
                .seedqr_written(&c, 0, compact, Reveal::PaperForm)
                .unwrap();
            assert_eq!(count, words.split(' ').count());
            let own = if compact {
                faraday_codec::seedqr::encode_compact(&c, &m)
            } else {
                faraday_codec::seedqr::encode_seedqr(&c, &m)
            }
            .unwrap();
            assert_eq!(
                code.release().to_luma(4, 2),
                own.to_luma(4, 2),
                "compact {compact}"
            );
        }
    }
    // The digits are each word's number on the list, four digits a word.
    let m = Mnemonic::parse(&c, Language::English, ABANDON).unwrap();
    let digits: String = m.indices().iter().map(|n| format!("{n:04}")).collect();
    assert_eq!(
        faraday_codec::seedqr::to_digits(&m).expose().as_bytes(),
        digits.as_bytes()
    );
}

/// SeedQR is defined on 12 and 24 words: a key of 15, 18 or 21 words is not
/// given one, and says so rather than "not the same wallet".
#[test]
fn a_key_of_another_length_than_12_or_24_words_has_no_seedqr() {
    let c = compute();
    for bytes in [20usize, 24, 28] {
        let m = Mnemonic::from_entropy(Language::English, &vec![7u8; bytes]).unwrap();
        let list = Language::English.words();
        let words: Vec<&str> = m.indices().iter().map(|&n| list[usize::from(n)]).collect();
        let mut keys = Keys::on(Network::Mainnet);
        keys.add_words_with(&c, &words.join(" "), "", "", None)
            .unwrap();
        for compact in [false, true] {
            assert_eq!(
                keys.seedqr_written(&c, 0, compact, Reveal::PaperForm).err(),
                Some(Convert::NotGiven),
                "{} words, compact {compact}",
                words.len()
            );
        }
    }
}

/// A key on another list is written as its words, entropy and numbers,
/// but has no SeedQR and no starts; a key with no words has none of them.
#[test]
fn another_list_gives_words_and_numbers_but_no_codes_or_starts_and_seed_bytes_give_none() {
    let c = compute();
    let jp: Value = serde_json::from_str(include_str!(
        "../../../tools/vectors/bip39/test_JP_BIP39.json"
    ))
    .unwrap();
    let row = &jp[0];
    let words = row["mnemonic"].as_str().unwrap();
    let mut keys = Keys::on(Network::Mainnet);
    keys.add_words_with(&c, words, "", "", None).unwrap();
    let spaced: Vec<&str> = words.split('\u{3000}').collect();
    let m = Mnemonic::parse(&c, Language::Japanese, &spaced.join(" ")).unwrap();
    let listed: Vec<&str> = m
        .indices()
        .iter()
        .map(|&n| Language::Japanese.word_display(n))
        .collect();
    assert_eq!(
        written(&keys, &c, WordsForm::Words).unwrap(),
        listed.join(" ")
    );
    assert_eq!(
        written(&keys, &c, WordsForm::HexEntropy).unwrap(),
        row["entropy"].as_str().unwrap()
    );
    assert!(
        written(&keys, &c, WordsForm::Numbers)
            .unwrap()
            .starts_with("1 (0)\n1 (0)")
    );
    assert_eq!(
        written(&keys, &c, WordsForm::Starts),
        Err(Convert::NotGiven)
    );
    for compact in [false, true] {
        assert!(matches!(
            keys.seedqr_written(&c, 0, compact, Reveal::PaperForm),
            Err(Convert::NotGiven)
        ));
    }
    let mut bytes = Keys::on(Network::Mainnet);
    bytes.add_seed(&c, &[7; 16], "").unwrap();
    for form in [
        WordsForm::Words,
        WordsForm::HexEntropy,
        WordsForm::Numbers,
        WordsForm::Starts,
    ] {
        assert_eq!(written(&bytes, &c, form), Err(Convert::NotGiven));
    }
    assert!(matches!(
        bytes.seedqr_written(&c, 0, false, Reveal::PaperForm),
        Err(Convert::NotGiven)
    ));
}

const XOR: &str = include_str!("../../../tools/vectors/xor/coldcard.json");

/// Coldcard's published Seed XOR examples: with the first parts given as
/// the random ones, the last part is the one Coldcard publishes, and every
/// part is words on the list.
#[test]
fn seed_xor_gives_coldcards_published_parts() {
    let c = compute();
    let v: Value = serde_json::from_str(XOR).unwrap();
    for e in v["examples"].as_array().unwrap() {
        let parts: Vec<&str> = e["parts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p.as_str().unwrap())
            .collect();
        let mut random = Vec::new();
        for p in &parts[..parts.len() - 1] {
            random.extend_from_slice(words(p).entropy().expose().as_bytes());
        }
        let mut keys = Keys::on(Network::Mainnet);
        keys.add_words_with(&c, e["seed"].as_str().unwrap(), "", "", None)
            .unwrap();
        let made = keys
            .seed_xor_converted(&c, 0, parts.len() as u8, &random, Reveal::PaperForm)
            .unwrap()
            .release();
        let made: Vec<&str> = made.iter().map(|p| p.as_str()).collect();
        assert_eq!(made, parts, "{}", e["name"]);
    }
}

/// Seed XOR is made only for the paper form, 2 to 4 parts, a key of words
/// and enough randomness; each refusal says which.
#[test]
fn seed_xor_is_refused_for_the_wrong_reason() {
    let c = compute();
    let mut keys = Keys::on(Network::Mainnet);
    keys.add_words_with(&c, ABANDON, "", "", None).unwrap();
    let random = [7u8; 96];
    let xor = |keys: &Keys, parts, random: &[u8], why| {
        keys.seed_xor_converted(&c, 0, parts, random, why)
            .map(|h| h.release())
    };
    assert_eq!(
        xor(&keys, 3, &random, Reveal::Words),
        Err(Convert::NotGiven)
    );
    assert_eq!(
        xor(&keys, 5, &random, Reveal::PaperForm),
        Err(Convert::Refused)
    );
    assert_eq!(
        xor(&keys, 1, &random, Reveal::PaperForm),
        Err(Convert::Refused)
    );
    assert_eq!(
        xor(&keys, 3, &random[..16], Reveal::PaperForm),
        Err(Convert::Refused)
    );
    let seed = seed_key(&c, Network::Mainnet, &[9u8; 16]);
    assert_eq!(
        xor(&seed, 2, &random, Reveal::PaperForm),
        Err(Convert::NotGiven)
    );
}

/// A key's seed length is known without its bytes: none for words.
#[test]
fn a_key_knows_how_long_its_seed_is() {
    let c = compute();
    let mut words_key = Keys::on(Network::Mainnet);
    words_key.add_words_with(&c, ABANDON, "", "", None).unwrap();
    assert_eq!(words_key.info(0).unwrap().seed_len, None);
    for n in [16usize, 20, 32, 64] {
        let k = seed_key(&c, Network::Mainnet, &vec![3u8; n]);
        assert_eq!(k.info(0).unwrap().seed_len, Some(n));
    }
}

mod aezeed_source {
    //! An aezeed read for Convert is held in the store, never as a key,
    //! and converts as the seed LND takes its 16 bytes of entropy to be
    //! (LND's `cipherseed_test.go`, `tools/reference/aezeed/README.md`).

    use super::*;
    use faraday_keys::lightning::AEZEED_WORDS;

    const WORDS: &str = "above judge emerge veteran reform crunch system all snap please \
        shoulder vault hurt city quarter cover enlist swear success suggest drink wagon enrich body";
    const WITH_PASSPHRASE: &str = "absorb century submit father path glove gloom super divert \
        garden ice mirror wisdom grass dice kit ugly castle success suggest drink monster \
        congress flight";
    const ENTROPY: &str = "81b637d86359e6960de795e41e0b4cfd";

    fn indices(words: &str) -> [u16; AEZEED_WORDS] {
        let mut out = [0u16; AEZEED_WORDS];
        for (slot, w) in out.iter_mut().zip(words.split_whitespace()) {
            *slot = Language::English.index_of(w).unwrap();
        }
        out
    }

    fn hold(keys: &mut Keys, words: &str, pass: &str) -> Result<(), String> {
        let c = compute();
        let pass = faraday_intake::secret_buf::SecretText::of(pass);
        let read = keys.aezeed_seed_job(&indices(words), pass).run(&c);
        keys.aezeed_hold(read).map(|_| ())
    }

    #[test]
    fn the_entropy_is_the_bip_32_seed_and_each_target_reads_back_to_it() {
        let c = compute();
        let mut keys = Keys::default();
        hold(&mut keys, WORDS, "").unwrap();
        assert_eq!(keys.len(), 0);
        let i = Keys::AEZEED_SOURCE;
        let why = Reveal::PaperForm;
        let entropy = unhex(ENTROPY);
        let master = MasterKey::from_seed_bytes(
            &c,
            &faraday_crypto::Secret::new(faraday_crypto::SeedBytes::new(&entropy).unwrap()),
            Network::Mainnet,
        );
        assert_eq!(
            keys.source_info(i).unwrap().fingerprint,
            master.fingerprint()
        );

        assert_eq!(
            &**keys.master_seed_hex(&c, i, why).unwrap().release(),
            ENTROPY
        );
        assert_eq!(
            keys.master_xprv(&c, i, why).unwrap().release().as_str(),
            master.xpriv_ascii().as_str()
        );
        let words = keys.slip39_one_of_one(&c, i, why).unwrap().release();
        let share = Share::parse(&words).unwrap();
        assert_eq!(
            &faraday_keys::slip39_seed(&c, &[share], b"").unwrap()[..],
            &entropy[..]
        );
        let codex = keys
            .codex32_converted(&c, i, 0, 1, (&[], &[]), why)
            .unwrap()
            .release();
        assert_eq!(codex[0].len(), 48);
        let parsed = Codex32::parse(&codex[0]).unwrap();
        assert_eq!(parsed.to_seed().unwrap().expose().bytes(), &entropy[..]);
        let groups = [GroupSpec {
            threshold: 2,
            count: 3,
        }];
        let random = [7u8; 200];
        let set = keys
            .slip39_converted(&c, i, 1, &groups, &random, why)
            .unwrap();
        let shares: Vec<Share> = set
            .release()
            .iter()
            .map(|s| Share::parse(s).unwrap())
            .collect();
        assert_eq!(shares.len(), 3);
        assert_eq!(
            &faraday_keys::slip39_seed(&c, &shares[1..], b"").unwrap()[..],
            &entropy[..]
        );
        // Only for the paper form.
        assert_eq!(
            keys.master_xprv(&c, i, Reveal::LightningKey).err(),
            Some(Convert::NotGiven)
        );
    }

    #[test]
    fn the_birthday_is_the_one_the_vector_carries() {
        let mut keys = Keys::default();
        hold(&mut keys, WITH_PASSPHRASE, "!very_safe_55345_password*").unwrap();
        assert_eq!(keys.aezeed_birthday(), Some((2018, 3, 22)));
    }

    #[test]
    fn a_wrong_passphrase_or_a_bad_checksum_holds_nothing() {
        let mut keys = Keys::default();
        assert!(hold(&mut keys, WORDS, "wrong").is_err());
        let mut words: Vec<&str> = WORDS.split(' ').collect();
        words[23] = "abandon";
        assert!(hold(&mut keys, &words.join(" "), "").is_err());
        assert!(keys.source_info(Keys::AEZEED_SOURCE).is_none());
        assert!(!keys.holds_secret());
    }

    #[test]
    fn a_stick_attached_holds_nothing() {
        let mut keys = Keys::default();
        keys.set_stick_attached(Some("stick"));
        assert!(hold(&mut keys, WORDS, "").is_err());
        assert!(keys.aezeed_birthday().is_none());
    }

    /// The held aezeed gives the conversions and nothing else: no public
    /// key is derived from it.
    #[test]
    fn only_the_conversions_reach_it() {
        let c = compute();
        let mut keys = Keys::default();
        hold(&mut keys, WORDS, "").unwrap();
        let path = "m/84'/0'/0'".parse().unwrap();
        assert!(keys.xpub_at(&c, Keys::AEZEED_SOURCE, &path).is_none());
        assert!(
            keys.master_seed_hex(&c, Keys::AEZEED_SOURCE, Reveal::PaperForm)
                .is_ok()
        );
    }

    #[test]
    fn forgetting_or_clearing_drops_it() {
        let c = compute();
        let mut keys = Keys::default();
        hold(&mut keys, WORDS, "").unwrap();
        assert!(keys.holds_secret());
        keys.aezeed_forget();
        assert!(!keys.holds_secret());
        assert_eq!(
            keys.master_seed_hex(&c, Keys::AEZEED_SOURCE, Reveal::PaperForm)
                .err(),
            Some(Convert::NotGiven)
        );
        hold(&mut keys, WORDS, "").unwrap();
        keys.clear();
        assert!(keys.aezeed_birthday().is_none());
    }
}

/// A WIF is told from other text, and a text far longer than any key is
/// not one.
#[test]
fn is_wif_reads_a_wif_and_refuses_a_very_long_text() {
    for wif in [
        "5HueCGU8rMjxEXxiPuD5BDku4MkFqeZyd4dZ1jvhTVqvbTLvyTJ",
        "L1aW4aubDFB7yfras2S1mN3bqg9nwySY8nkoLmJebSLD5BWv3ENZ",
    ] {
        assert!(identity::is_wif(wif), "{wif}");
    }
    assert!(!identity::is_wif("not a key"));
    assert!(!identity::is_wif(&"5".repeat(16 << 20)));
    assert!(!identity::is_wif(&format!(
        "5HueCGU8rMjxEXxiPuD5BDku4MkFqeZyd4dZ1jvhTVqvbTLvyTJ{}",
        "1".repeat(16 << 20)
    )));
}

/// A seed sealed as a Krux code (`docs/KEF.md` §6): the entropy of 12 or
/// 24 English words, and nothing else.
mod kef_entropy {
    use super::{ABANDON, compute, unhex};
    use faraday_bip::keys::Network;
    use faraday_keys::{Keys, Reveal};

    const TWELVE: &str =
        "legal winner thank year wave sausage worth useful legal winner thank yellow";
    const TWENTY_FOUR: &str = "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth title";
    const FIFTEEN: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon address";

    fn with(words: &str) -> Keys {
        let c = compute();
        let mut keys = Keys::on(Network::Mainnet);
        keys.add_words_with(&c, words, "", "seed", None)
            .expect("a valid seed");
        keys
    }

    #[test]
    fn a_12_and_a_24_word_seed_give_16_and_32_bytes_of_entropy() {
        let c = compute();
        let twelve = with(TWELVE);
        let held = twelve.kef_entropy(&c, 0, Reveal::KefCode).unwrap();
        assert_eq!(&held[..], &unhex(&"7f".repeat(16))[..]);
        let big = with(TWENTY_FOUR);
        let held = big.kef_entropy(&c, 0, Reveal::KefCode).unwrap();
        assert_eq!(&held[..], &unhex(&"7f".repeat(32))[..]);
        let held = with(ABANDON).kef_entropy(&c, 0, Reveal::KefCode).unwrap();
        assert_eq!(&held[..], &[0u8; 16][..]);
    }

    #[test]
    fn a_seed_of_15_words_is_refused() {
        let c = compute();
        assert!(with(FIFTEEN).kef_entropy(&c, 0, Reveal::KefCode).is_none());
    }

    #[test]
    fn a_key_with_no_words_or_no_such_key_gives_nothing() {
        let c = compute();
        let mut keys = Keys::on(Network::Mainnet);
        keys.add_seed(&c, &unhex("000102030405060708090a0b0c0d0e0f"), "")
            .unwrap();
        assert!(keys.kef_entropy(&c, 0, Reveal::KefCode).is_none());
        assert!(keys.kef_entropy(&c, 7, Reveal::KefCode).is_none());
    }

    #[test]
    fn another_reason_gets_nothing_and_this_one_lends_no_text() {
        let c = compute();
        let keys = with(TWELVE);
        for why in [
            Reveal::Words,
            Reveal::SeedShown,
            Reveal::PaperForm,
            Reveal::SeedQr,
        ] {
            assert!(keys.kef_entropy(&c, 0, why).is_none(), "{why:?}");
        }
        assert_eq!(&*keys.reveal(0, Reveal::KefCode), "");
    }
}

#[test]
fn a_job_view_holds_the_masters_and_no_words() {
    let c = compute();
    let mut keys = Keys::on(Network::Testnet);
    keys.add_words_with(&c, ABANDON, "TREZOR", "With a passphrase", None)
        .expect("the words");
    let view = keys.job_view();
    assert_eq!(view.len(), 1);
    assert_eq!(view.info(0), keys.info(0));
    let path: DerivationPath = "m/84'/1'/0'".parse().unwrap();
    assert_eq!(
        view.xpub_at(&c, 0, &path).map(|x| x.to_string()),
        keys.xpub_at(&c, 0, &path).map(|x| x.to_string())
    );
    assert!(view.xpub_at(&c, 0, &path).is_some());
    for why in [Reveal::Words, Reveal::Passphrase, Reveal::PaperForm] {
        assert!(view.reveal(0, why).is_empty());
    }
    assert!(!keys.reveal(0, Reveal::Words).is_empty());
}
