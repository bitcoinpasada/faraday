//! A `tr()` wallet with a script tree states who can spend by its key
//! path (`docs/decisions/F132.md`): nobody (BIP 341's NUMS point), a
//! key of the wallet, or another key. Only a wallet nobody can spend by
//! the key path reads as a "k of n" tapscript multisig.

use std::str::FromStr;

use bitcoin::bip32::{ChainCode, ChildNumber, Xpub};
use bitcoin::secp256k1::PublicKey;
use faraday_bip::policy::{KeyPath, Template, WalletPolicy};
use faraday_crypto::work::Compute;

const NUMS: &str = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";
/// The generator's x coordinate: a valid key that is not `H`.
const OTHER: &str = "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";

const A: &str = "[9a6a2580/48h/1h/0h/3h]tpubDFZk17LmJpm9L6gT3oK6r3TWNni6fVLrkZLCsH2psqckyuuZ5239eePW97oLgikCUJ5zQX4MVVKDp9Yj9DC9wtYyGXkPwLuGeGN78U3ykR2/<0;1>/*";
const B: &str = "[5d388376/48h/1h/0h/3h]tpubDFcsPThm5auRE692Mr7j53V9NY9Lt3XKuUP6fgJjCbDmECdXthX6jv7Yfy5qZAAxFKndTSS22t6RrxaDTvKTV9beG354SxmaXrvEQVJeBsB/<0;1>/*";
const C: &str = "[cf2e083d/48h/1h/0h/3h]tpubDFUzvqxLm1V5NfApU15kxXreU4PnLG5nEvhKxw2dAUmfdh1Vfv2V9mxEkU68vF35cMPLBmFkuqRKxUNsGT66VEukLBGRFAuRnHsZjB9xC5U/<0;1>/*";

fn read(text: &str) -> WalletPolicy {
    WalletPolicy::from_descriptor(&Compute::unchecked(), text).expect("the wallet reads")
}

fn multi_a(internal: &str, sorted: bool) -> String {
    let leaf = if sorted { "sortedmulti_a" } else { "multi_a" };
    format!("tr({internal},{leaf}(2,{A},{B},{C}))")
}

/// An extended key whose public key is `H`, with no origin: the
/// internal key Liana and this tree write so that a child of it is
/// unspendable at every index.
fn nums_xpub() -> String {
    let key = PublicKey::from_str(&format!("02{NUMS}")).unwrap();
    let xpub = Xpub {
        network: bitcoin::NetworkKind::Test,
        depth: 0,
        parent_fingerprint: [0u8; 4].into(),
        child_number: ChildNumber::from_normal_idx(0).unwrap(),
        public_key: key,
        chain_code: ChainCode::from([7u8; 32]),
    };
    format!("{xpub}/<0;1>/*")
}

#[test]
fn a_nums_internal_key_reads_as_k_of_n_with_no_key_path() {
    for sorted in [false, true] {
        let policy = read(&multi_a(NUMS, sorted));
        assert_eq!(policy.key_path(), Some(&KeyPath::Unspendable));
        assert!(policy.key_path_unspendable());
        assert_eq!(policy.tapscript_quorum(), Some((2, 3)), "sorted {sorted}");
    }
}

#[test]
fn another_internal_key_is_stated_and_has_no_quorum() {
    for sorted in [false, true] {
        let policy = read(&multi_a(OTHER, sorted));
        assert_eq!(policy.key_path(), Some(&KeyPath::Other(OTHER.to_string())));
        assert!(!policy.key_path_unspendable());
        assert_eq!(policy.tapscript_quorum(), None, "sorted {sorted}");
        assert_eq!(policy.template(), Template::Tree);
        // The tree's own paths are still listed, each over the
        // wallet's three keys.
        let paths = policy.spend_paths();
        assert_eq!(paths.len(), 3);
        assert!(
            paths
                .iter()
                .all(|p| p.keys.len() == 2 && p.keys.iter().all(|&k| k < 3))
        );
    }
}

#[test]
fn a_wallet_key_as_internal_key_spends_alone_and_has_no_quorum() {
    let policy = read(&multi_a(A, false));
    assert_eq!(policy.key_path(), Some(&KeyPath::WalletKey(0)));
    assert_eq!(policy.tapscript_quorum(), None);
    // The key path is the first way to spend: key 0 alone.
    assert!(
        policy
            .spend_paths()
            .iter()
            .any(|p| p.keys == [0] && p.locks.is_empty())
    );
}

#[test]
fn changing_the_internal_key_changes_who_spends_by_the_key_path() {
    let nums = read(&multi_a(NUMS, false));
    let other = read(&multi_a(OTHER, false));
    let own = read(&multi_a(A, false));
    assert_ne!(nums.key_path(), other.key_path());
    assert_ne!(nums.key_path(), own.key_path());
    assert_ne!(other.key_path(), own.key_path());
}

#[test]
fn an_extended_internal_key_over_nums_is_unspendable() {
    let policy = read(&multi_a(&nums_xpub(), false));
    assert_eq!(policy.key_path(), Some(&KeyPath::Unspendable));
}

#[test]
fn a_sorted_leaf_over_fewer_keys_than_the_wallet_has_no_quorum() {
    // The extended internal key is one of the wallet's four keys and
    // the leaf holds three of them: "2 of 4" would be untrue.
    let policy = read(&multi_a(&nums_xpub(), true));
    assert_eq!(policy.keys().len(), 4);
    assert_eq!(policy.tapscript_quorum(), None);
}

#[test]
fn a_key_with_no_origin_is_refused_but_as_the_internal_key_over_nums() {
    let bare = "tpubD6NzVbkrYhZ4XgHkCEtfpuZPJDLaLPxu5ZBEtAbub9GcUX1mTS2t3eCnBaYxKDfyvecpxgGsf4MtTC3gyZz9FD4qq6Km68VSDnpD5vpWm3e/<0;1>/*";
    let c = Compute::unchecked();
    // As the internal key over another point.
    assert_eq!(
        WalletPolicy::from_descriptor(&c, &multi_a(bare, false)),
        Err(faraday_bip::policy::Error::Key)
    );
    // As a leaf's key, under the NUMS point.
    let leaf = format!("tr({NUMS},multi_a(2,{bare},{B},{C}))");
    assert_eq!(
        WalletPolicy::from_descriptor(&c, &leaf),
        Err(faraday_bip::policy::Error::Key)
    );
}

#[test]
fn a_leaf_states_its_quorum_whoever_holds_the_key_path() {
    assert_eq!(read(&multi_a(OTHER, false)).leaf_quorum(), Some((2, 3)));
    assert_eq!(read(&multi_a(OTHER, true)).leaf_quorum(), Some((2, 3)));
    assert_eq!(read(&multi_a(NUMS, false)).leaf_quorum(), Some((2, 3)));
}

#[test]
fn a_single_key_or_segwit_policy_has_no_key_path_line() {
    let policy = read(&format!("wsh(sortedmulti(2,{A},{B},{C}))"));
    assert_eq!(policy.key_path(), None);
}

#[test]
fn the_timelocked_tree_sample_names_its_key_path_holder() {
    let policy = read(
        "tr([9a6a2580/86h/1h/1h]tpubDC5hTcigUKbZJjL6T8LpSMmJaEtoCmT38hVGQQVfWmxpXN6qpwL5fZnw4SevkDDSYt9n3jeLFqcZeSse8rjc6p7Cu9ehKRxNnBVvg174hXk/<0;1>/*,{and_v(v:pk([5d388376/86h/1h/1h]tpubDCEp3dYAyqnXuYsEwQgtXnCrvCzqtie4vCZwa93fDqxY5HLA4NqUf2Z5esZ1PFtGgoiiNtAKtSduA68T1X7N48N76paJTNQXrUmTqvNRBXM/<0;1>/*),older(4320)),pk([cf2e083d/86h/1h/1h]tpubDCcnPaKYdcUBgJLr244A7HRVfieRT4muW9RL3tumt2wtcxnZAMH8DA5AFuDLZgPs5KceFfoN9bnMPscK6JQZ92uC3B5Ha15QWF9MrMiz9kH/<0;1>/*)})",
    );
    assert_eq!(policy.key_path(), Some(&KeyPath::WalletKey(0)));
    assert_eq!(
        policy.keys()[0].fingerprint().map(|f| f.to_string()),
        Some("9a6a2580".into())
    );
}
