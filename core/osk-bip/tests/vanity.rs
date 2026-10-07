//! The vanity grinder against EntropyLab, which is the other
//! implementation of the same counter (`tools/reference/vanity/README.md`,
//! `docs/PLANNING.md` §16.117).
//!
//! The two vectors are EntropyLab's own output, from running its
//! `vanity-wasm` once over BIP-39's zero-entropy words: the passphrase
//! dial's first match at counter 8 and the account dial's at account 31,
//! for the prefix `bc1qq` on native SegWit.

use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::{MasterKey, Network, ScriptType};
use osk_bip::vanity::{self, Key, Method};

/// BIP-39's zero entropy: "abandon … about".
fn words() -> Mnemonic {
    Mnemonic::from_entropy(Language::English, &[0u8; 16]).expect("twelve words")
}

fn master(passphrase: &[u8]) -> MasterKey {
    let seed = words().to_seed(passphrase).expect("a seed");
    MasterKey::from_seed(&seed, Network::Mainnet)
}

#[test]
fn the_passphrase_counter_reaches_the_address_entropylab_reaches() {
    let words = words();
    let out = vanity::grind(
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
    let master = master(b"");
    let out = vanity::grind(
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
}

#[test]
fn the_passphrase_a_find_names_is_the_passphrase_that_makes_the_address() {
    let words = words();
    let find = vanity::grind(
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
        .account_xpub(ScriptType::NativeSegwit, 0)
        .expect("an account");
    let address = account.address(false, 0).expect("the first address");
    assert_eq!(alloc_string(&address), find.address.as_str());
}

#[test]
fn the_account_a_find_names_is_the_account_that_makes_the_address() {
    let master = master(b"");
    let find = vanity::grind(
        &Key::Master(&master),
        &Method::Account,
        ScriptType::NativeSegwit,
        "bc1qq",
        Network::Mainnet,
        0,
        64,
    )
    .find
    .expect("a find");
    let account = master
        .account_xpub(ScriptType::NativeSegwit, find.account)
        .expect("an account");
    let address = account.address(false, 0).expect("the first address");
    assert_eq!(alloc_string(&address), find.address.as_str());
}

#[test]
fn a_grind_that_stops_and_goes_on_reaches_the_same_candidate() {
    let words = words();
    let mut from = 0;
    let mut find = None;
    while from < 62 && find.is_none() {
        let out = vanity::grind(
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
    let master = master(b"");
    let out = vanity::grind(
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

/// An address as text. `Address` is public data and formats to a
/// `String`; the grinder's own buffer is compared against it.
fn alloc_string(address: &osk_bip::bitcoin::Address) -> String {
    format!("{address}")
}
