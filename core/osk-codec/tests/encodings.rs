//! What a string is encoded in, and what bytes it names.

use osk_codec::encodings::{Encoding, ReadAs, read, read_input};

/// BIP-173 and BIP-350: the strings the standards say are valid and the
/// ones they say must be rejected.
#[test]
fn the_bech32_vectors_are_read_the_way_the_standards_say() {
    for valid in [
        "A12UEL5L",
        "a12uel5l",
        "abcdef1qpzry9x8gf2tvdw0s3jn54khce6mua7lmqqqxw",
        "?1ezyfcl",
    ] {
        assert_eq!(
            read(valid).map(|r| r.encoding),
            Some(Encoding::Bech32),
            "{valid}"
        );
    }
    for valid in [
        "A1LQFN3A",
        "abcdef1l7aum6echk45nj3s0wdvt2fg8x9yrzpqzd3ryx",
        "?1v759aa",
    ] {
        assert_eq!(
            read(valid).map(|r| r.encoding),
            Some(Encoding::Bech32m),
            "{valid}"
        );
    }
    for invalid in [
        // An empty human-readable part.
        "1pzry9x0s0muk",
        // A character outside the data charset.
        "x1b4n0q5v",
        // The checksum does not hold.
        "li1dgmt3",
        // Mixed case.
        "A1G7SGD8",
    ] {
        assert!(
            !matches!(
                read(invalid).map(|r| r.encoding),
                Some(Encoding::Bech32 | Encoding::Bech32m)
            ),
            "{invalid} was read as bech32"
        );
    }
}

/// The mode decides what the text means: "dead" is four characters of
/// text or two bytes of hex.
#[test]
fn the_mode_row_decides_whether_the_field_is_text_or_hex() {
    let text = read_input("dead", ReadAs::Text);
    let bytes = read_input("dead", ReadAs::Hex);
    assert_eq!(text.len(), 4);
    assert_eq!(bytes, [0xde, 0xad]);
    // With no mode forced, a string that spells hex is read as hex.
    assert_eq!(read_input("dead", ReadAs::Auto), bytes);
    assert_eq!(read_input("dea", ReadAs::Auto).len(), 3);
}
