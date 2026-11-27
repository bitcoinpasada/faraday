//! The vanity grinder against EntropyLab, which is the other
//! implementation of the same counter (`tools/reference/vanity/README.md`).
//!
//! The two vectors are EntropyLab's own output, from running its
//! `vanity-wasm` once over BIP-39's zero-entropy words: the passphrase
//! dial's first match at counter 8 and the account dial's at account 31,
//! for the prefix `bc1qq` on native SegWit.

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::keys::{MasterKey, Network, ScriptType};
use faraday_bip::silent::Receiver;
use faraday_bip::vanity::{self, Key, Kind, Method};
use faraday_crypto::work::Compute;

/// BIP-39's zero entropy: "abandon … about".
fn words() -> Mnemonic {
    Mnemonic::from_entropy(Language::English, &[0u8; 16]).expect("twelve words")
}

fn master(passphrase: &[u8]) -> MasterKey {
    let c = Compute::unchecked();
    let seed = words().to_seed(&c, passphrase).expect("a seed");
    MasterKey::from_seed(&c, &seed, Network::Mainnet)
}

#[test]
fn the_passphrase_counter_reaches_the_address_entropylab_reaches() {
    let c = Compute::unchecked();
    let words = words();
    let out = vanity::grind(
        &c,
        &Key::Words(&words),
        &Method::Passphrase { base: b"" },
        ScriptType::NativeSegwit,
        "bc1qq",
        Network::Mainnet,
        0,
        62,
    );
    let find = out.find.expect("a find inside the first 62 candidates");
    assert_eq!(find.counter, 8);
    assert_eq!(find.suffix.as_str(), "i");
    assert_eq!(
        find.address.as_str(),
        "bc1qqv2kx2d9tv4d3epztk59dc5kymkl8pk8scu6xz"
    );
}

#[test]
fn the_account_index_reaches_the_address_entropylab_reaches() {
    let c = Compute::unchecked();
    let master = master(b"");
    let out = vanity::grind(
        &c,
        &Key::Master(&master),
        &Method::Account,
        ScriptType::NativeSegwit,
        "bc1qq",
        Network::Mainnet,
        0,
        64,
    );
    let find = out.find.expect("a find inside the first 64 accounts");
    assert_eq!(find.account, 31);
    assert_eq!(find.suffix.as_str(), "");
    assert_eq!(
        find.address.as_str(),
        "bc1qqpat9khft6dnm9qp0nnrvpyvmyg2ytshn7gglv"
    );
    // The account the find names is the account that makes the address.
    let account = master
        .account_xpub(&c, ScriptType::NativeSegwit, find.account)
        .expect("an account");
    let address = account.address(false, 0).expect("the first address");
    assert_eq!(alloc_string(&address), find.address.as_str());
}

#[test]
fn the_passphrase_a_find_names_is_the_passphrase_that_makes_the_address() {
    let c = Compute::unchecked();
    let words = words();
    let find = vanity::grind(
        &c,
        &Key::Words(&words),
        &Method::Passphrase { base: b"" },
        ScriptType::NativeSegwit,
        "bc1qq",
        Network::Mainnet,
        0,
        62,
    )
    .find
    .expect("a find");
    let opened = master(find.suffix.as_bytes());
    let account = opened
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .expect("an account");
    let address = account.address(false, 0).expect("the first address");
    assert_eq!(alloc_string(&address), find.address.as_str());
}

#[test]
fn a_grind_that_stops_and_goes_on_reaches_the_same_candidate() {
    let c = Compute::unchecked();
    let words = words();
    let mut from = 0;
    let mut find = None;
    while from < 62 && find.is_none() {
        let out = vanity::grind(
            &c,
            &Key::Words(&words),
            &Method::Passphrase { base: b"" },
            ScriptType::NativeSegwit,
            "bc1qq",
            Network::Mainnet,
            from,
            3,
        );
        from += out.tested;
        find = out.find;
    }
    let find = find.expect("a find");
    assert_eq!(find.counter, 8);
    assert_eq!(find.suffix.as_str(), "i");
}

#[test]
fn a_prefix_asks_only_for_characters_an_address_of_that_kind_can_carry() {
    let bech32 = [
        ("bc1qq", true),
        ("bc1qzz", true),
        ("bc1q1", false),
        ("bc1qb", false),
        ("bc1qi", false),
        ("bc1qo", false),
        ("bc1pq", false),
    ];
    for (prefix, allowed) in bech32 {
        assert_eq!(
            vanity::can_begin(prefix, ScriptType::NativeSegwit, Network::Mainnet),
            allowed,
            "{prefix}"
        );
    }
    let base58 = [
        ("1A", true),
        ("1z", true),
        ("10", false),
        ("1O", false),
        ("1I", false),
        ("1l", false),
        ("3A", false),
    ];
    for (prefix, allowed) in base58 {
        assert_eq!(
            vanity::can_begin(prefix, ScriptType::Legacy, Network::Mainnet),
            allowed,
            "{prefix}"
        );
    }
}

#[test]
fn a_legacy_address_on_a_test_network_begins_with_m_or_n() {
    for (prefix, allowed) in [("m", true), ("n", true), ("1", false), ("mA", true)] {
        assert_eq!(
            vanity::can_begin(prefix, ScriptType::Legacy, Network::Testnet),
            allowed,
            "{prefix}"
        );
    }
    assert_eq!(
        vanity::fixed_prefix(ScriptType::Legacy, Network::Testnet),
        ""
    );
}

#[test]
fn the_expected_count_is_one_over_the_chance_of_an_address_beginning_that_way() {
    let cases = [
        ("bc1q", ScriptType::NativeSegwit, Network::Mainnet, 1),
        ("bc1qq", ScriptType::NativeSegwit, Network::Mainnet, 32),
        ("bc1qab", ScriptType::NativeSegwit, Network::Mainnet, 1024),
        ("bc1pxyz", ScriptType::Taproot, Network::Mainnet, 32768),
        ("1A", ScriptType::Legacy, Network::Mainnet, 58),
        ("1Ab", ScriptType::Legacy, Network::Mainnet, 3364),
        ("3A", ScriptType::NestedSegwit, Network::Mainnet, 58),
        ("m", ScriptType::Legacy, Network::Testnet, 2),
        ("mA", ScriptType::Legacy, Network::Testnet, 116),
    ];
    for (prefix, script, network, expected) in cases {
        assert_eq!(
            vanity::expected_candidates(prefix, script, network),
            expected,
            "{prefix}"
        );
    }
}

#[test]
fn the_expected_time_is_the_count_over_the_rate() {
    assert_eq!(vanity::expected_seconds(32, 32.0), Some(1));
    assert_eq!(vanity::expected_seconds(1024, 2.0), Some(512));
    assert_eq!(vanity::expected_seconds(1024, 0.0), None);
}

#[test]
fn a_dial_the_key_cannot_turn_tests_nothing() {
    let c = Compute::unchecked();
    let master = master(b"");
    let out = vanity::grind(
        &c,
        &Key::Master(&master),
        &Method::Passphrase { base: b"" },
        ScriptType::NativeSegwit,
        "bc1qq",
        Network::Mainnet,
        0,
        10,
    );
    assert_eq!(out.tested, 0);
    assert!(out.find.is_none());
}

const BIP352_VECTORS: &str =
    include_str!("../../../../tools/vectors/bip352/send_and_receive_test_vectors.json");

/// Every silent payment code BIP-352's vectors publish.
fn bip352_codes() -> Vec<String> {
    fn walk(v: &serde_json::Value, out: &mut Vec<String>) {
        match v {
            serde_json::Value::String(t) if t.starts_with("sp1q") => out.push(t.clone()),
            serde_json::Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            serde_json::Value::Object(o) => o.values().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let v: serde_json::Value = serde_json::from_str(BIP352_VECTORS).expect("vectors");
    let mut out = Vec::new();
    walk(&v, &mut out);
    out
}

#[test]
fn every_silent_payment_code_in_bip_352s_vectors_begins_as_the_rule_says() {
    let codes = bip352_codes();
    assert!(codes.len() >= 10, "{}", codes.len());
    let fixed = vanity::fixed_prefix(Kind::Silent, Network::Mainnet);
    assert_eq!(fixed, "sp1qq");
    let free = vanity::first_free(Kind::Silent, Network::Mainnet).expect("a first place");
    for code in &codes {
        assert!(code.starts_with(fixed), "{code}");
        let second = code[fixed.len()..].chars().next().expect("a character");
        assert!(free.contains(second), "{code}");
        // Every length of its beginning is a prefix a person may ask for,
        // upper case included.
        for n in fixed.len()..=code.len() {
            assert!(vanity::can_begin(
                &code[..n],
                Kind::Silent,
                Network::Mainnet
            ));
            assert!(vanity::can_begin(
                &code[..n].to_ascii_uppercase(),
                Kind::Silent,
                Network::Mainnet
            ));
        }
    }
    assert!(!vanity::can_begin("sp1qqa", Kind::Silent, Network::Mainnet));
    assert!(!vanity::can_begin("sp1qp", Kind::Silent, Network::Mainnet));
    assert!(!vanity::can_begin("bc1qq", Kind::Silent, Network::Mainnet));
    assert_eq!(
        vanity::fixed_prefix(Kind::Silent, Network::Testnet),
        "tsp1qq"
    );
    assert!(vanity::can_begin("tsp1qqg", Kind::Silent, Network::Testnet));
    // Eight characters stand second, thirty-two in each place after.
    for (prefix, expected) in [
        ("sp1qq", 1),
        ("sp1qqg", 8),
        ("sp1qqgz", 256),
        ("sp1qqgzz9", 262_144),
    ] {
        assert_eq!(
            vanity::expected_candidates(prefix, Kind::Silent, Network::Mainnet),
            expected,
            "{prefix}"
        );
    }
}

#[test]
fn a_silent_payment_code_a_find_names_is_the_wallets_own_at_that_account() {
    let c = Compute::unchecked();
    let master = master(b"");
    let out = vanity::grind(
        &c,
        &Key::Master(&master),
        &Method::Account,
        Kind::Silent,
        "sp1qqgz",
        Network::Mainnet,
        0,
        100_000,
    );
    let find = out.find.expect("a find");
    assert_eq!(find.counter, u64::from(find.account));
    assert_eq!(out.tested, u64::from(find.account) + 1);
    let receiver = Receiver::derive(&c, &master, find.account);
    let code = receiver.address(master.secp()).expect("a code");
    assert_eq!(code.as_str(), find.address.as_str());
    assert!(code.as_str().to_ascii_lowercase().starts_with("sp1qqgz"));
    // No account before it began that way.
    for account in 0..find.account {
        let other = Receiver::derive(&c, &master, account);
        let other = other.address(master.secp()).expect("a code");
        assert!(!other.as_str().starts_with("sp1qqgz"), "{account}");
    }
}

#[test]
fn a_silent_payment_code_on_the_passphrase_dial_is_the_wallet_of_the_key_with_that_passphrase() {
    let c = Compute::unchecked();
    let words = words();
    let find = vanity::grind(
        &c,
        &Key::Words(&words),
        &Method::Passphrase { base: b"" },
        Kind::Silent,
        "sp1qqgz",
        Network::Mainnet,
        0,
        100_000,
    )
    .find
    .expect("a find");
    let opened = master(find.suffix.as_bytes());
    let receiver = Receiver::derive(&c, &opened, 0);
    let code = receiver.address(opened.secp()).expect("a code");
    assert_eq!(code.as_str(), find.address.as_str());
    assert_eq!(find.account, 0);
}

/// An address as text. `Address` is public data and formats to a
/// `String`; the grinder's own buffer is compared against it.
fn alloc_string(address: &faraday_bip::bitcoin::Address) -> String {
    format!("{address}")
}
