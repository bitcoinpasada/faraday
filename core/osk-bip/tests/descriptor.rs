//! BIP-380 descriptor checksum against published vectors.

use osk_bip::descriptor::{descriptor_checksum, verify_checksum};

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
    assert_eq!(descriptor_checksum("raw(\u{dc})"), None);
    assert_eq!(descriptor_checksum("tab\there"), None);
    // The empty descriptor has a checksum too; verify_checksum needs a body.
    let empty = checksum("");
    assert!(verify_checksum(&format!("#{empty}")));
}

#[test]
fn group_boundaries() {
    // Lengths 1, 2 and 3 exercise the partial-group tail cases.
    for desc in ["a", "ab", "abc", "abcd", "abcde", "abcdef"] {
        let sum = checksum(desc);
        assert!(verify_checksum(&format!("{desc}#{sum}")));
        assert!(!verify_checksum(&format!("{desc}x#{sum}")));
    }
}
