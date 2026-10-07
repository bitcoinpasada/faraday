//! BIP 387's test vectors: every valid `multi_a` and `sortedmulti_a`
//! descriptor pays to the script the BIP lists, and every invalid one is
//! refused.
//!
//! The BIP writes its keys as raw public keys, WIF private keys and
//! extended private keys, which no wallet of this tree loads; what the
//! vectors check is the reader underneath — `miniscript` for `multi_a`,
//! and `osk_bip::tapmulti` for the sorted form `miniscript` has no
//! parser for.

use std::str::FromStr;

use bitcoin::secp256k1::Secp256k1;
use osk_bip::miniscript::Descriptor;
use osk_bip::miniscript::descriptor::DescriptorPublicKey;

/// The BIP, verbatim (`tools/vectors/bip387/README.md`).
const BIP: &str = include_str!("../../../tools/vectors/bip387/bip-0387.mediawiki");

/// The output script a descriptor pays to at `index`, read the way this
/// tree reads one: `miniscript` for `multi_a`, and the sorted script
/// rebuilt from the derived keys for `sortedmulti_a`.
fn script_at(text: &str, index: u32) -> Option<String> {
    let secp = Secp256k1::new();
    if hardened_wildcard(text) {
        return None;
    }
    let sorted = text.contains("sortedmulti_a(");
    let unsorted = text.replacen("sortedmulti_a(", "multi_a(", 1);
    let (descriptor, _) = Descriptor::parse_descriptor(&secp, &unsorted).ok()?;
    let derived = descriptor
        .at_derivation_index(index)
        .ok()?
        .derived_descriptor(&secp)
        .ok()?;
    if !sorted {
        return Some(derived.script_pubkey().to_hex_string());
    }
    let threshold: usize = unsorted
        .split_once("multi_a(")?
        .1
        .split_once(',')?
        .0
        .parse()
        .ok()?;
    let (internal, keys) = osk_bip::tapmulti::derived_keys(&derived)?;
    let leaf = osk_bip::tapmulti::sorted_multi_a_script(threshold, &keys);
    Some(osk_bip::tapmulti::output_script(&secp, internal, &leaf)?.to_hex_string())
}

/// Whether a key's wildcard is hardened (`/*'`). Such a descriptor
/// derives only from the private key, so a device holding the extended
/// public keys of its cosigners cannot produce its addresses at all —
/// which is why no wallet writes one, and why the one BIP 387 vector
/// that does is read but not derived here.
fn hardened_wildcard(text: &str) -> bool {
    text.contains("*'") || text.contains("*h")
}

/// The text between `<tt>` and `</tt>` on a line, which is how the BIP
/// writes both a descriptor and a script.
fn tagged(line: &str) -> Option<&str> {
    line.split_once("<tt>")?
        .1
        .split_once("</tt>")
        .map(|(inner, _)| inner)
}

/// The valid vectors: each descriptor with the scripts it pays to at
/// index 0, 1, 2 …
fn valid() -> Vec<(String, Vec<String>)> {
    let body = BIP
        .split_once("==Test Vectors==")
        .expect("the BIP has test vectors")
        .1
        .split_once("Invalid descriptors")
        .expect("the vectors end at the invalid ones")
        .0;
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if let Some(script) = line.strip_prefix("** ").and_then(tagged) {
            out.last_mut()
                .expect("a script follows its descriptor")
                .1
                .push(String::from(script));
        } else if let Some(descriptor) = line.strip_prefix("* ").and_then(tagged) {
            out.push((String::from(descriptor), Vec::new()));
        }
    }
    out
}

/// The invalid vectors, each with the reason the BIP gives for it.
fn invalid() -> Vec<(String, String)> {
    let body = BIP
        .split_once("Invalid descriptors")
        .expect("the BIP lists invalid descriptors")
        .1
        .split_once("==Backwards Compatibility==")
        .expect("the list ends at the next section")
        .0;
    body.lines()
        .filter_map(|line| {
            let line = line.trim().strip_prefix("* ")?;
            let descriptor = tagged(line)?;
            let reason = line.split_once(':').map(|(r, _)| r).unwrap_or(line);
            Some((String::from(descriptor), String::from(reason)))
        })
        .collect()
}

#[test]
fn every_valid_descriptor_pays_to_its_published_script() {
    let vectors = valid();
    assert_eq!(vectors.len(), 6, "BIP 387 lists six valid descriptors");
    for (descriptor, scripts) in vectors {
        let secp = Secp256k1::new();
        assert!(
            Descriptor::parse_descriptor(
                &secp,
                &descriptor.replacen("sortedmulti_a(", "multi_a(", 1)
            )
            .is_ok(),
            "{descriptor}"
        );
        if hardened_wildcard(&descriptor) {
            continue;
        }
        for (index, script) in scripts.iter().enumerate() {
            let index = u32::try_from(index).expect("three indices at most");
            assert_eq!(
                script_at(&descriptor, index).as_deref(),
                Some(script.as_str()),
                "{descriptor} at {index}"
            );
        }
    }
}

#[test]
fn every_invalid_descriptor_is_refused() {
    let vectors = invalid();
    assert_eq!(vectors.len(), 7, "BIP 387 lists seven invalid descriptors");
    for (descriptor, reason) in vectors {
        assert_eq!(script_at(&descriptor, 0), None, "{reason}: {descriptor}");
    }
}

/// The sorted form and the unsorted form of the same keys are different
/// wallets: BIP 387's own pair of vectors differ in the script they pay
/// to, and sorting is of the derived keys, so a wallet that sorted once
/// at the account level would agree at one index and disagree at the
/// next.
#[test]
fn sorting_is_of_the_derived_keys() {
    let secp = Secp256k1::new();
    let text = "tr(50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0,\
                sortedmulti_a(2,\
                xpub6ERApfZwUNrhLCkDtcHTcxd75RbzS1ed54G1LkBUHQVHQKqhMkhgbmJbZRkrgZw4koxb5JaHWkY4ALHY2grBGRjaDMzQLcgJvLJuZZvRcEL/*,\
                xpub68NZiKmJWnxxS6aaHmn81bvJeTESw724CRDs6HbuccFQN9Ku14VQrADWgqbhhTHBaohPX4CjNLf9fq9MYo6oDaPPLPxSb7gwQN3ih19Zm4Y/0/0/*))";
    let unsorted = text.replacen("sortedmulti_a(", "multi_a(", 1);
    let descriptor = Descriptor::<DescriptorPublicKey>::from_str(&unsorted).unwrap();
    let mut orders = Vec::new();
    for index in 0..3u32 {
        let derived = descriptor
            .at_derivation_index(index)
            .unwrap()
            .derived_descriptor(&secp)
            .unwrap();
        let (_, keys) = osk_bip::tapmulti::derived_keys(&derived).unwrap();
        orders.push(keys[0].serialize() < keys[1].serialize());
    }
    assert!(
        orders.contains(&true) && orders.contains(&false),
        "these two keys change order between indices"
    );
}
