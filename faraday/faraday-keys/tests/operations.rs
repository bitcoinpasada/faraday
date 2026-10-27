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
use faraday_keys::{Keys, Material, Part, Reveal, bip85, identity};
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
    assert_eq!(keys.with_same_words(0), vec![0, 1]);
    assert!(keys.same_words(&c, 0, 1));
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
        .key_record(&c, 0, None, false, Reveal::VaultEntry(Part::Words))
        .expect("a record");
    assert_eq!(plain.kind, kind::KEY);
    assert!(plain.field(field::KEY_PASSPHRASE).is_none());
    assert_eq!(plain.field(field::KEY_LABEL), Some(&b"Key"[..]));
    let with = keys
        .key_record(&c, 0, Some(1), true, Reveal::VaultEntry(Part::Words))
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
    const OTHERS: [Reveal; 13] = [
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
        assert_eq!(keys.keep_codex32(0, &refs), 2);
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
        assert_eq!(keys.keep_codex32(0, &[shares[1].as_str()]), 1);
        let shown = keys.seed_shown(0, Reveal::SeedShown).unwrap();
        let shown: Vec<&str> = shown.iter().map(|s| s.as_str()).collect();
        assert_eq!(shown, [set["seed"].as_str().unwrap(), &shares[1]]);
        // Its root extended private key is the vector's.
        let root = keys
            .xprv_at(&c, 0, &DerivationPath::master(), Reveal::ExtendedPrivateKey)
            .unwrap();
        assert_eq!(root.as_str(), set["xprv"].as_str().unwrap());
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
