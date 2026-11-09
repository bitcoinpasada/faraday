//! A descriptor that uses one key twice is found from its text, whether
//! or not the wallet loads: the same key under another origin or depth
//! is still the same key, and one key at disjoint derivations
//! (BIP-388's `@0/<0;1>/*` and `@0/<2;3>/*`) is not a repeat.

use std::str::FromStr;

use bitcoin::bip32::Xpub;
use faraday_bip::policy::{WalletPolicy, repeated_key_texts, repeated_keys_in};
use faraday_crypto::work::Compute;

const A: &str = "tpubDFZk17LmJpm9L6gT3oK6r3TWNni6fVLrkZLCsH2psqckyuuZ5239eePW97oLgikCUJ5zQX4MVVKDp9Yj9DC9wtYyGXkPwLuGeGN78U3ykR2";
const B: &str = "tpubDFcsPThm5auRE692Mr7j53V9NY9Lt3XKuUP6fgJjCbDmECdXthX6jv7Yfy5qZAAxFKndTSS22t6RrxaDTvKTV9beG354SxmaXrvEQVJeBsB";
const C: &str = "tpubDFUzvqxLm1V5NfApU15kxXreU4PnLG5nEvhKxw2dAUmfdh1Vfv2V9mxEkU68vF35cMPLBmFkuqRKxUNsGT66VEukLBGRFAuRnHsZjB9xC5U";

fn a() -> String {
    format!("[9a6a2580/48h/1h/0h/3h]{A}")
}
fn b() -> String {
    format!("[5d388376/48h/1h/0h/3h]{B}")
}
fn c() -> String {
    format!("[cf2e083d/48h/1h/0h/3h]{C}")
}

/// The key of `A` as another extended key under another origin: another
/// depth and parent, the same public key and chain code.
fn a_again() -> String {
    let mut x = Xpub::from_str(A).unwrap();
    x.depth = 5;
    x.parent_fingerprint = [1, 2, 3, 4].into();
    format!("[00000000/84h/1h/0h]{x}")
}

fn found(text: &str) -> Vec<(usize, usize)> {
    repeated_keys_in(&Compute::unchecked(), text)
}

#[test]
fn a_key_twice_in_a_sorted_multisig_is_found() {
    let text = format!(
        "wsh(sortedmulti(2,{0}/<0;1>/*,{1}/<0;1>/*,{0}/<0;1>/*))",
        a(),
        b()
    );
    assert_eq!(found(&text), vec![(0, 0)]);
}

#[test]
fn the_same_key_under_another_origin_and_depth_is_found() {
    let text = format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
        a(),
        b(),
        a_again()
    );
    assert_eq!(found(&text), vec![(0, 2)]);
    let two_part = format!(
        "wsh(sortedmulti(2,@0/**,@1/**,@2/**))\n{},{},{}",
        a(),
        b(),
        a_again()
    );
    assert_eq!(found(&two_part), vec![(0, 2)]);
}

#[test]
fn a_loaded_policy_names_the_same_key_under_two_placeholders() {
    let two_part = format!(
        "wsh(sortedmulti(2,@0/**,@1/**,@2/**))\n{},{},{}",
        a(),
        b(),
        a_again()
    );
    let policy = WalletPolicy::parse_any(&Compute::unchecked(), &two_part).expect("loads");
    assert_eq!(policy.repeated_keys(), vec![(0, 2)]);
    let distinct = format!("wsh(sortedmulti(2,@0/**,@1/**))\n{},{}", a(), b());
    let policy = WalletPolicy::parse_any(&Compute::unchecked(), &distinct).expect("loads");
    assert!(policy.repeated_keys().is_empty());
}

#[test]
fn a_key_in_a_tree_leaf_and_as_the_internal_key_is_found() {
    let text = format!(
        "tr({0}/<0;1>/*,multi_a(2,{1}/<0;1>/*,{2}/<0;1>/*))",
        a(),
        b(),
        c()
    );
    assert!(found(&text).is_empty());
    let text = format!(
        "tr({0}/<0;1>/*,multi_a(2,{1}/<0;1>/*,{0}/<0;1>/*))",
        a(),
        b()
    );
    assert_eq!(found(&text), vec![(0, 0)]);
    let text = format!(
        "tr({}/<0;1>/*,sortedmulti_a(2,{}/<0;1>/*,{}/<0;1>/*))",
        a(),
        b(),
        a_again()
    );
    assert_eq!(found(&text), vec![(0, 2)]);
}

#[test]
fn one_key_at_disjoint_derivations_is_not_a_repeat() {
    let at = |first: &str, second: &str| {
        format!(
            "wsh(or_d(multi(2,{0}/{first}/*,{1}/<0;1>/*),and_v(v:multi(2,{0}/{second}/*,{1}/<2;3>/*),older(100))))",
            a(),
            b()
        )
    };
    assert!(found(&at("<0;1>", "<2;3>")).is_empty());
    assert_eq!(found(&at("<0;1>", "<1;2>")), vec![(0, 0)]);
}

#[test]
fn a_single_key_descriptor_is_not_a_repeat() {
    let text = format!("wpkh({}/<0;1>/*)", a());
    assert!(found(&text).is_empty());
    assert!(found("not a descriptor").is_empty());
}

#[test]
fn key_texts_repeat_by_key_not_by_text() {
    assert_eq!(repeated_key_texts(&[a(), b(), a_again()]), vec![(0, 2)]);
    assert!(repeated_key_texts(&[a(), b(), c()]).is_empty());
}
