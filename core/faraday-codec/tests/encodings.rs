//! What a string is encoded in, and what bytes it names.

use faraday_codec::encodings::{Encoding, ReadAs, from_hex, read, read_input};

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
    let text = read_input("dead", ReadAs::Text).unwrap();
    let bytes = read_input("dead", ReadAs::Hex).unwrap();
    assert_eq!(text.len(), 4);
    assert_eq!(bytes, [0xde, 0xad]);
    // With no mode forced, a string that spells hex is read as hex.
    assert_eq!(read_input("dead", ReadAs::Auto).unwrap(), bytes);
    assert_eq!(read_input("dea", ReadAs::Auto).unwrap().len(), 3);
}

#[test]
fn a_segwit_address_reads_as_its_witness_version_and_program() {
    // BIP-173's and BIP-350's valid addresses, with the script each gives.
    for (address, encoding, hrp, version, program) in [
        (
            "BC1QW508D6QEJXTDG4Y5R3ZARVARY0C5XW7KV8F3T4",
            Encoding::Bech32,
            "bc",
            0,
            "751e76e8199196d454941c45d1b3a323f1433bd6",
        ),
        (
            "tb1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3q0sl5k7",
            Encoding::Bech32,
            "tb",
            0,
            "1863143c14c5166804bd19203356da136c985678cd4d27a1b8c6329604903262",
        ),
        (
            "bc1p0xlxvlhemja6c4dqv22uapctqupfhlxm9h8z3k2e72q4k9hcz7vqzk5jj0",
            Encoding::Bech32m,
            "bc",
            1,
            "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
        ),
        ("BC1SW50QGDZ25J", Encoding::Bech32m, "bc", 16, "751e"),
    ] {
        let read = read(address).expect(address);
        assert_eq!(read.encoding, encoding, "{address}");
        assert_eq!(read.hrp.as_deref(), Some(hrp), "{address}");
        assert_eq!(read.version, Some(version), "{address}");
        assert_eq!(read.bytes, from_hex(program).expect("hex"), "{address}");
    }
}

#[test]
fn bech32_text_that_is_not_a_segwit_address_has_no_version() {
    // Another prefix, and a version 1 address under the bech32 checksum,
    // which BIP-350 lists as invalid.
    for text in [
        "abcdef1qpzry9x8gf2tvdw0s3jn54khce6mua7lmqqqxw",
        "bc1p0xlxvlhemja6c4dqv22uapctqupfhlxm9h8z3k2e72q4k9hcz7vqh2y7hd",
    ] {
        assert_eq!(read(text).map(|r| r.version), Some(None), "{text}");
    }
}
