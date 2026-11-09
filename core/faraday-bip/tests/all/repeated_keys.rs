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

/// The account key of a fixed seed and the key `path` below it, made by
/// `bitcoin::bip32`.
fn derived(path: &str) -> Xpub {
    use bitcoin::bip32::{DerivationPath, Xpriv};
    use bitcoin::secp256k1::Secp256k1;
    let secp = Secp256k1::new();
    let master = Xpriv::new_master(bitcoin::Network::Testnet, &[7u8; 32]).unwrap();
    let path = DerivationPath::from_str(&format!("m/48h/1h/0h/2h{path}")).unwrap();
    Xpub::from_priv(&secp, &master.derive_priv(&secp, &path).unwrap())
}

fn wsh(a: &str, b: &str) -> String {
    format!("wsh(multi(1,{a},{b}))")
}

#[test]
fn a_wildcard_and_a_fixed_index_it_reaches_are_a_repeat() {
    let p = derived("");
    let found_in = |a: &str, b: &str| found(&wsh(&format!("{p}/{a}"), &format!("{p}/{b}")));
    assert_eq!(found_in("0/*", "0/5"), vec![(0, 0)]);
    assert_eq!(found_in("<0;1>/*", "1/7"), vec![(0, 0)]);
    assert!(found_in("0/*", "1/5").is_empty());
    assert!(found_in("0/*", "0/5h").is_empty());
    assert!(found_in("0/*", "0/5/1").is_empty());
    assert_eq!(found_in("0/*h", "0/5h"), vec![(0, 0)]);
    assert!(found_in("0/*h", "0/5").is_empty());
}

#[test]
fn a_parent_and_its_child_are_a_repeat() {
    let parent = derived("");
    let child = derived("/0");
    let text = wsh(&format!("{parent}/0/*"), &format!("{child}/*"));
    assert_eq!(found(&text), vec![(0, 1)]);
    let other = derived("/1");
    assert!(found(&wsh(&format!("{parent}/0/*"), &format!("{other}/*"))).is_empty());
    // The plain key at P/0/5 is the key P/0/5 reaches.
    let leaf = derived("/0/5").public_key;
    let text = wsh(&format!("{parent}/0/5"), &leaf.to_string());
    assert_eq!(found(&text), vec![(0, 1)]);
    let text = wsh(&format!("{parent}/0/6"), &leaf.to_string());
    assert!(found(&text).is_empty());
}

#[test]
fn a_two_part_policy_with_other_multipaths_is_read() {
    let (a, b) = (a(), b());
    let policy = |x: &str, y: &str| {
        format!(
            "wsh(or_d(multi(2,@0/{x}/*,@1/**),and_v(v:multi(2,@0/{y}/*,@1/<2;3>/*),older(100))))\n{a},{b}"
        )
    };
    assert!(found(&policy("<0;1>", "<2;3>")).is_empty());
    assert_eq!(found(&policy("<2;3>", "<3;4>")), vec![(0, 0)]);
    let loads = format!("wsh(sortedmulti(2,@0/<2;3>/*,@1/<2;3>/*))\n{a},{b}");
    assert!(WalletPolicy::parse_any(&Compute::unchecked(), &loads).is_err());
}

#[test]
fn keys_are_numbered_as_they_are_read_plain_ones_too() {
    let nums = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";
    let text = format!("tr({nums},{{pk({0}/<0;1>/*),pk({0}/<0;1>/*)}})", a());
    assert_eq!(found(&text), vec![(1, 1)]);
    let p = derived("");
    let plain = derived("/0/5").public_key;
    let text = format!("wsh(multi(1,{plain},{}/0/*,{p}/0/5))", a());
    assert_eq!(found(&text), vec![(0, 2)]);
}

#[test]
fn a_multipath_alternative_reaches_a_child_or_a_plain_key() {
    let p = derived("");
    let child = derived("/1");
    let text = wsh(&format!("{p}/<0;1>/*"), &format!("{child}/*"));
    assert_eq!(found(&text), vec![(0, 1)]);
    let plain = derived("/1/5").public_key;
    let text = wsh(&format!("{p}/<0;1>/5"), &plain.to_string());
    assert_eq!(found(&text), vec![(0, 1)]);
    let text = wsh(&format!("{p}/<0;2>/5"), &plain.to_string());
    assert!(found(&text).is_empty());
}

#[test]
fn a_key_without_an_origin_is_numbered_where_it_is_written() {
    let bare = format!("{}/<0;1>/*", C);
    let text = format!("wsh(multi(1,{}/<0;1>/*,{B}/<0;1>/*,{bare}))", c());
    assert_eq!(found(&text), vec![(0, 2)]);
    let p = derived("/0/5").public_key.to_string();
    let text = format!("wsh(multi(1,[cf2e083d/48h]{p},{B}/<0;1>/*,{p}))");
    assert_eq!(found(&text), vec![(0, 2)]);
}

#[test]
fn a_two_part_policy_numbers_its_list_of_keys() {
    let (a, b) = (a(), b());
    let text = format!("wsh(multi(1,@0/**,@1/**,@2/**,@3/**))\n{a},{a},{b},{b}");
    assert_eq!(found(&text), vec![(0, 1), (2, 3)]);
    let nums = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";
    let text = format!("tr({nums},{{pk(@0/<0;1>/*),pk(@0/<0;1>/*)}})\n{a}");
    assert_eq!(found(&text), vec![(0, 0)]);
    let text = format!("tr({nums},{{pk(@0/<0;1>/*),pk({nums})}})\n{a}");
    assert!(found(&text).is_empty());
}

#[test]
fn an_origin_on_a_key_in_the_template_is_not_a_number() {
    let a = a();
    let lit = format!("[fa1b2c3d/48h]{B}/<0;1>/*");
    let text = format!("wsh(multi(2,@0/**,{lit},{lit}))\n{a}");
    assert!(found(&text).is_empty());
    let text = format!("wsh(multi(2,@0/**,[fa000000/48h]{A}/<0;1>/*))\n{a}");
    assert!(found(&text).is_empty());
}
