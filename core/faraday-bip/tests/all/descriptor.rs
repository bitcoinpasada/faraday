//! BIP-380 descriptor checksum against published vectors.

use faraday_bip::descriptor::{descriptor_checksum, verify_checksum};

fn checksum(desc: &str) -> String {
    String::from_utf8(descriptor_checksum(desc).unwrap().to_vec()).unwrap()
}

/// Vectors with known checksums: the BIP-380 example, rust-miniscript's
/// `descriptor/checksum.rs` tests, and Bitcoin Core's
/// `descriptor_tests.cpp` (the two `sh(multi(…))` cases).
const KNOWN: &[(&str, &str)] = &[
    ("raw(deadbeef)", "89f8spxm"),
    (
        "wpkh(tprv8ZgxMBicQKsPdpkqS7Eair4YxjcuuvDPNYmKX3sCniCf16tHEVrjjiSXEkFRnUH77yXc6ZcwHHcLNfjdi5qUvw3VDfgYiH5mNsj5izuiu2N/1/2/*)",
        "tqz0nc62",
    ),
    (
        "pkh(tpubD6NzVbkrYhZ4XHndKkuB8FifXm8r5FQHwrN6oZuWCz13qb93rtgKvD4PQsqC4HP4yhV3tA2fqr2RbY5mNXfM7RxXUoeABoDtsFUq2zJq6YK/44'/1'/0'/0/*)",
        "lasegmfs",
    ),
    (
        "sh(multi(2,[00000000/111'/222]xprvA1RpRA33e1JQ7ifknakTFpgNXPmW2YvmhqLQYMmrj4xJXXWYpDPS3xz7iAxn8L39njGVyuoseXzU6rcxFLJ8HFsTjSyQbLYnMpCqE2VbFWc,xprv9uPDJpEQgRQfDcW7BkF7eTya6RPxXeJCqCJGHuCJ4GiRVLzkTXBAJMu2qaMWPrS7AANYqdq6vcBcBUdJCVVFceUvJFjaPdGZ2y9WACViL4L/0))",
        "ggrsrxfy",
    ),
    (
        "sh(multi(2,[00000000/111'/222]xpub6ERApfZwUNrhLCkDtcHTcxd75RbzS1ed54G1LkBUHQVHQKqhMkhgbmJbZRkrgZw4koxb5JaHWkY4ALHY2grBGRjaDMzQLcgJvLJuZZvRcEL,xpub68NZiKmJWnxxS6aaHmn81bvJeTESw724CRDs6HbuccFQN9Ku14VQrADWgqbhhTHBaohPX4CjNLf9fq9MYo6oDaPPLPxSb7gwQN3ih19Zm4Y/0))",
        "tjg09x5t",
    ),
];

#[test]
fn known_checksums() {
    for &(desc, expected) in KNOWN {
        assert_eq!(checksum(desc), expected, "{desc}");
        assert!(verify_checksum(&format!("{desc}#{expected}")));
    }
}

#[test]
fn a_checksum_that_is_not_the_descriptors_is_refused() {
    assert!(!verify_checksum("raw(deadbeef)#89f8spxmx"), "too long");
    assert!(!verify_checksum("raw(deadbeef)#89f8spx"), "too short");
    assert!(!verify_checksum("raw(deedbeef)#89f8spxm"), "payload error");
    assert!(!verify_checksum("raw(deadbeef)#89f8spxn"), "checksum error");
    assert!(
        !verify_checksum("raw(deadbeef)#89f8spxb"),
        "b is not bech32"
    );
    assert!(!verify_checksum(""));
    assert!(!verify_checksum("#89f8spxm"));
}

#[test]
fn single_symbol_errors_are_detected() {
    for &(desc, expected) in KNOWN {
        let full = format!("{desc}#{expected}");
        let bytes = full.as_bytes();
        for i in 0..bytes.len() {
            let mut edited = bytes.to_vec();
            // Substitute a different charset character at position i.
            edited[i] = if bytes[i] == b'(' { b')' } else { b'(' };
            let edited = String::from_utf8(edited).unwrap();
            assert!(
                !verify_checksum(&edited),
                "undetected edit at {i}: {edited}"
            );
        }
    }
}

#[test]
fn every_charset_character_is_accepted() {
    let charset = "0123456789()[],'/*abcdefgh@:$%{}IJKLMNOPQRSTUVWXYZ&+-.;<=>?!^_|~ijklmnopqrstuvwxyzABCDEFGH`#\"\\ ";
    assert_eq!(charset.len(), 95, "all printable ASCII");
    let sum = descriptor_checksum(charset).unwrap();
    assert!(
        sum.iter()
            .all(|c| b"qpzry9x8gf2tvdw0s3jn54khce6mua7l".contains(c))
    );
    assert!(verify_checksum(&format!(
        "{charset}#{}",
        std::str::from_utf8(&sum).unwrap()
    )));
    assert_eq!(descriptor_checksum("raw(\u{dc})"), None, "non-charset char");
    assert_eq!(descriptor_checksum("tab\there"), None);
    assert!(!verify_checksum("raw(\u{dc})#00000000"));
    // The empty descriptor has a checksum too; verify_checksum needs a body.
    let empty = checksum("");
    assert!(verify_checksum(&format!("#{empty}")));
}

/// What `key_facts` says of a descriptor's keys: Bitcoin Core writes a
/// master key's descriptor with no brackets, and that key is its own origin.
#[test]
fn key_facts_say_which_keys_have_an_origin_and_of_which_network() {
    use faraday_bip::descriptor::key_facts;
    use faraday_crypto::work::Compute;
    let c = Compute::unchecked();
    let tpub = "tpubDDdxeK56gpJTxGcCbxZFWUrM9YMqGsW9xJCNxozZBATiW2ConR2Cf7EzxHA2SyN5Eb9wZxqcphbaYWyQLsa1nogJu3zeYehQJcivKjWevAP";
    let master = "xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGhePY2gZ29ESFjqJoCu1Rupje8YtGqsefD265TMg7usUDFdp6W1EGMcet8";

    let with = key_facts(&c, &format!("wpkh([17746d02/84h/1h/0h]{tpub}/0/*)")).unwrap();
    assert_eq!(
        (with.keys, with.without_origin, with.main, with.test),
        (1, 0, 0, 1)
    );
    let without = key_facts(&c, &format!("wpkh({tpub}/0/*)")).unwrap();
    assert_eq!((without.keys, without.without_origin), (1, 1));
    let top = key_facts(&c, &format!("wpkh({master}/0/*)")).unwrap();
    assert_eq!(
        (top.keys, top.without_origin, top.main, top.test),
        (1, 0, 1, 0)
    );
    assert!(key_facts(&c, "wpkh(nothing)").is_none());
}

/// BIP 32 test vector 5 says a depth-0 key with a parent fingerprint or a
/// child number is invalid: such a key is not a master key and carries no
/// origin of its own.
#[test]
fn a_depth_zero_key_with_a_parent_or_an_index_has_no_origin() {
    use faraday_bip::descriptor::key_facts;
    use faraday_crypto::work::Compute;
    let c = Compute::unchecked();
    let invalid = include_str!("../../../../tools/vectors/bip32/invalid-keys.txt");
    let mut seen = 0;
    for line in invalid.lines().filter(|l| l.starts_with("xpub")) {
        let (key, why) = line.split_once(' ').expect("key then reason");
        if !why.starts_with("zero depth") {
            continue;
        }
        let facts = key_facts(&c, &format!("wpkh({key}/0/*)")).expect("reads");
        assert_eq!(facts.without_origin, 1, "{why}");
        seen += 1;
    }
    assert_eq!(seen, 2);
}
