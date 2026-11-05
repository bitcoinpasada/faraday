//! A key or descriptor text far longer than any key can be is refused, not
//! worked through: the base58 decoder is quadratic, so each long case here
//! is long enough that reading it would not end in any useful time.

use faraday_bip::bitcoin::base58;
use faraday_bip::descriptor::{descriptor_checksum, reads_as_descriptor};
use faraday_bip::miniscript::{Descriptor, DescriptorPublicKey};
use faraday_bip::policy::{WalletPolicy, parse_script_descriptor};
use faraday_bip::recovery::{Form, Path, Recovery, RecoveryPolicy};
use faraday_bip::{slip132, xkey};
use faraday_crypto::work::Compute;

const XPUB: &str = "xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGhePY2gZ29ESFjqJoCu1Rupje8YtGqsefD265TMg7usUDFdp6W1EGMcet8";
const XPUB2: &str = "xpub68Gmy5EdvgibQVfPdqkBBCHxA5htiqg55crXYuXoQRKfDBFA1WEjWgP6LHhwBZeNK1VTsfTFUHCdrfp1bgwQ9xv5ski8PX9rL2dZXvgGDnw";
/// The generator point, compressed and uncompressed.
const G66: &str = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
const G130: &str = "0479be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798483ada7726a3c4655da4fbfc0e1108a8fd17b448a68554199c47d08ffb10d4b8";
const LONG: usize = 16 << 20;

fn c() -> Compute {
    Compute::unchecked()
}

fn long_xpub() -> String {
    format!("xpub{}", "F".repeat(LONG))
}

fn with_checksum(body: &str) -> String {
    format!(
        "{body}#{}",
        String::from_utf8(descriptor_checksum(body).unwrap().to_vec()).unwrap()
    )
}

#[test]
fn a_descriptor_with_a_key_far_longer_than_any_key_is_refused() {
    let key = long_xpub();
    for text in [
        format!("wsh(pk({key}))"),
        format!("wpkh({key})"),
        format!("wpkh([00000000/84h/0h/0h]{key}/<0;1>/*)"),
        format!("tr({key})"),
        format!("wsh(multi(2,{XPUB}/<0;1>/*,{key}/<0;1>/*))"),
    ] {
        assert!(!reads_as_descriptor(&c(), &text));
        assert!(WalletPolicy::from_descriptor(&c(), &text).is_err());
        assert!(parse_script_descriptor::<DescriptorPublicKey>(&c(), &text).is_err());
    }
}

#[test]
fn a_wallet_policy_with_a_key_far_longer_than_any_key_is_refused() {
    let key = long_xpub();
    let two_part = format!(
        "wsh(sortedmulti(1,@0/**,@1/**))\n[00000000/48h/0h/0h/2h]{XPUB}\n[00000000/48h/0h/0h/2h]{key}\n"
    );
    assert!(WalletPolicy::parse(&c(), &two_part).is_err());
    assert!(xkey::decode_xpub(&key).is_err());
    assert!(xkey::decode_xpriv(&key).is_err());
    assert!(slip132::decode_xpub(&key).is_err());
}

/// Every key form a descriptor holds still reads: extended keys with and
/// without origin and multipath, compressed and uncompressed raw keys,
/// x-only keys, a hash, and a checksum.
#[test]
fn every_key_form_a_descriptor_holds_still_reads() {
    let xonly = &G66[2..];
    let sha = "ab".repeat(32);
    for body in [
        format!("wpkh({XPUB})"),
        format!("wpkh([deadbeef/84h/0h/0h]{XPUB}/<0;1>/*)"),
        format!("pkh({G66})"),
        format!("pkh({G130})"),
        format!("tr({xonly})"),
        format!("wsh(multi(2,[deadbeef/48h/0h/0h/2h]{XPUB}/<0;1>/*,{XPUB2}/<0;1>/*))"),
        format!("wsh(and_v(v:pk({G66}),sha256({sha})))"),
    ] {
        assert!(reads_as_descriptor(&c(), &body), "{body}");
        assert!(reads_as_descriptor(&c(), &with_checksum(&body)), "{body}");
    }
    let wallet = format!(
        "wsh(sortedmulti(2,[deadbeef/48h/0h/0h/2h]{XPUB}/<0;1>/*,[cafebabe/48h/0h/0h/2h]{XPUB2}/<0;1>/*))"
    );
    assert!(WalletPolicy::from_descriptor(&c(), &with_checksum(&wallet)).is_ok());
}

/// A descriptor of many short tokens reads: the limit is on one token, not
/// on the text.
#[test]
fn a_descriptor_of_many_keys_reads() {
    let keys: Vec<String> = (0..20).map(|_| format!("{XPUB}/<0;1>/*")).collect();
    let body = format!("wsh(multi(2,{}))", keys.join(","));
    assert!(reads_as_descriptor(&c(), &body));
    assert!(body.parse::<Descriptor<DescriptorPublicKey>>().is_ok());
}

/// What only looks long still reads, as it did: a run of wrappers, and path
/// indexes with leading zeros, in an origin, a path and a multipath step.
#[test]
fn long_wrappers_and_long_path_indexes_still_read() {
    let wrappers = "n".repeat(131);
    let zeros = "0".repeat(2000);
    for body in [
        format!("wsh({wrappers}:pk({XPUB}))"),
        format!("wpkh([deadbeef/{zeros}84h/0h/0h]{XPUB})"),
        format!("wpkh({XPUB}/{zeros}1/*)"),
        format!("wpkh({XPUB}/<{zeros};1>/*)"),
    ] {
        assert!(reads_as_descriptor(&c(), &body), "{}", &body[..40]);
        assert!(
            parse_script_descriptor::<DescriptorPublicKey>(&c(), &body).is_ok(),
            "{}",
            &body[..40]
        );
    }
}

/// White space round a `tr()` of one `sortedmulti_a` leaf is trimmed before
/// it is read, so however much of it there is, the text reads.
#[test]
fn a_sorted_taproot_leaf_with_long_padding_still_reads() {
    let body = format!("tr({XPUB},sortedmulti_a(1,{XPUB2},{}))", &G66[2..]);
    assert!(reads_as_descriptor(&c(), &body));
    let padding = " ".repeat(2000);
    assert!(reads_as_descriptor(&c(), &format!("{body}{padding}")));
    assert!(reads_as_descriptor(&c(), &format!("{padding}{body}")));
}

/// A key that stops being base58 early is as far from reading as a long one,
/// and the decoder would still size its work from the whole of it, whichever
/// character follows the valid start.
#[test]
fn a_key_with_a_long_tail_is_refused() {
    let start = "2".repeat(1020);
    for tail in [".", "<", ">", ";", "[", "]", "#", ":", "{", "}"] {
        let tail = tail.repeat(LONG);
        for text in [
            format!("wsh(pk(xpub{start}{tail}))"),
            format!("wsh(pk([deadbeef/0h]xpub{start}{tail}/0))"),
        ] {
            assert!(!reads_as_descriptor(&c(), &text));
            assert!(parse_script_descriptor::<DescriptorPublicKey>(&c(), &text).is_err());
        }
    }
}

/// A recovery wallet's key is read alone, not as part of a descriptor, so
/// no character of a long key divides it.
#[test]
fn a_recovery_wallet_with_a_long_key_is_refused() {
    let start = "2".repeat(1020);
    for key in [
        long_xpub(),
        format!("xpub{start}{}", "(".repeat(LONG)),
        format!("xpub{start}{}", ",".repeat(LONG)),
    ] {
        let policy = RecoveryPolicy {
            primary: Path::single(&key),
            recovery: vec![Recovery {
                delay: 1000,
                path: Path::single(XPUB2),
            }],
        };
        assert!(policy.to_wallet_policy(&c(), Form::SegWit).is_err());
    }
}

/// A text over 112 characters whose base58 check holds is refused as a bad
/// key; it is no longer read to learn its length.
#[test]
fn a_checked_payload_too_long_to_be_a_key_is_refused() {
    let text = base58::encode_check(&[0x42; 100]);
    assert!(text.len() > 112);
    assert!(xkey::decode_xpub(&text).is_err());
}
