//! BOLT #11 invoices: every example of the specification, valid and invalid
//! (`tools/vectors/lightning-bolt11`), read as a person pasting or scanning
//! an invoice would see it.

use faraday_bip::keys::Network;
use faraday_codec::bolt11::{Error, Fallback, decode};

const VECTORS: &str = include_str!("../../../tools/vectors/lightning-bolt11/invoice-test.json");
const NODE: &str = "03e7156ae33b0a208d0744199163177e909e80176e55d97a2f221ede0f934dd9ad";

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn vectors() -> Vec<(String, bool, String)> {
    let v: serde_json::Value = serde_json::from_str(VECTORS).unwrap();
    v.as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e["description"].as_str().unwrap().to_string(),
                e["valid"].as_bool().unwrap(),
                e["invoice"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn invoice(containing: &str) -> faraday_codec::bolt11::Invoice {
    let (_, _, text) = vectors()
        .into_iter()
        .find(|(d, _, _)| d.contains(containing))
        .unwrap();
    decode(&text).unwrap()
}

#[test]
fn every_valid_example_is_read_and_every_invalid_one_refused() {
    for (what, valid, text) in vectors() {
        let got = decode(&text);
        assert_eq!(got.is_ok(), valid, "{what}: {got:?}");
    }
}

#[test]
fn donation_of_any_amount() {
    let i = invoice("donation of any amount");
    assert_eq!(i.network, Network::Mainnet);
    assert_eq!(i.amount_msat, None);
    assert_eq!(i.timestamp, 1_496_314_658);
    assert_eq!(
        hex(&i.payment_hash),
        "0001020304050607080900010203040506070809000102030405060708090102"
    );
    assert_eq!(i.payment_secret, [0x11; 32]);
    assert_eq!(
        i.description.as_deref(),
        Some("Please consider supporting this project")
    );
    assert_eq!(hex(&i.payee), NODE);
    assert!(!i.payee_from_field);
    assert_eq!(i.features, vec![8, 14]);
    assert_eq!(i.expiry, 3600);
}

#[test]
fn amounts_and_japanese_description() {
    assert_eq!(invoice("$3 for a cup").amount_msat, Some(250_000_000));
    let i = invoice("nonsense");
    assert_eq!(i.description.as_deref(), Some("ナンセンス 1杯"));
    assert_eq!(i.expiry, 60);
    let i = invoice("0.00967878534 BTC");
    assert_eq!(i.amount_msat, Some(967_878_534));
    assert_eq!(i.expiry, 604_800);
    assert_eq!(i.min_final_cltv_expiry, 10);
    assert_eq!(i.route_hints.len(), 1);
    let hop = &i.route_hints[0][0];
    assert_eq!(hop.short_channel_id, (589_390 << 40) | (3312 << 16) | 1);
    assert_eq!(hop.fee_base_msat, 1000);
    assert_eq!(hop.fee_proportional_millionths, 2500);
    assert_eq!(hop.cltv_expiry_delta, 40);
    assert_eq!(
        hex(&hop.node_id),
        "03d06758583bb5154774a6eb221b1276c9e82d65bbaceca806d90e20c108f4b1c7"
    );
}

#[test]
fn hashed_description_and_fallbacks() {
    let i = invoice("list of things (hashed)");
    assert_eq!(i.amount_msat, Some(2_000_000_000));
    assert_eq!(i.description, None);
    assert_eq!(
        hex(&i.description_hash.unwrap()),
        "3925b6f67e2c340036ed12093dd44e0368df1b6ea26c53dbe4811f58fd5db8c1"
    );
    let t = invoice("on testnet");
    assert_eq!(t.network, Network::Testnet);
    assert_eq!(
        t.fallbacks[0].address(t.network),
        "mk2QpYatsKicvFVuTAQLBryyccRXMUaGHP"
    );
    for (what, address) in [
        (
            "fallback address 1Rusty",
            "1RustyRX2oai4EYYDpQGWvEL62BBGqN9T",
        ),
        ("(P2SH)", "3EktnHQD7RiAE6uzMj2ZifT9YgRrkSgzQX"),
        ("(P2WPKH)", "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4"),
        (
            "(P2WSH)",
            "bc1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3qccfmv3",
        ),
        (
            "(P2TR)",
            "bc1pptdvg0d2nj99568qn6ssdy4cygnwuxgw2ukmnwgwz7jpqjz2kszse2s3lm",
        ),
    ] {
        let i = invoice(what);
        assert_eq!(i.fallbacks[0].address(i.network), address, "{what}");
    }
    let r = invoice("fallback address 1Rusty");
    assert_eq!(r.route_hints[0].len(), 2);
    assert_eq!(
        r.route_hints[0][1].short_channel_id,
        (197_637 << 40) | (395_016 << 16) | 2314
    );
}

#[test]
fn a_fallback_of_an_unknown_version_is_skipped_and_the_next_one_read() {
    // The first f field is of version 19, the second a version 0 program of 20 bytes.
    let i = decode(
        "lnbc3621900210p1qqq64fxfpwnh6yh6ccwwmf986xwzhs0evxzl6cd09d0rqnjgakuz3smspp5w76z7n66zm68829y0epah7f7y4rj9kljg2x75f5qjm4pw8kv8leqcqpphp583xaalxyrs5tdva20cyvvs8spd2zacuehj592mvxc0s4sfv4kjcqfppq5kmktheu9hk748uwnr5qfja39qgl5d86sp5hhfqgdz64phkudw2n0t9mqnnxh7769uaw2hj36rjap8ahktfx3pquyrmr3k2cfh9teqxrvuf2xuxeg4utqaqn3j0nat86gzweu466lex8hp78ec3l7semd65s2dvh7nme6752nkcrzu4njdq65fnxt7737cqcyvaw6",
    )
    .unwrap();
    assert_eq!(i.fallbacks.len(), 1);
    assert_eq!(i.unknown.len(), 1);
    assert_eq!(
        i.fallbacks[0],
        Fallback::Witness {
            version: 0,
            program: vec![
                0xa5, 0xb7, 0x65, 0xdf, 0x3c, 0x2d, 0xed, 0xea, 0x9f, 0x8e, 0x98, 0xe8, 0x04, 0xcb,
                0xb1, 0x28, 0x11, 0xfa, 0x34, 0xfa
            ],
        }
    );
}

#[test]
fn features_metadata_case_and_ignored_fields() {
    let i = invoice("features 8, 14 and 99");
    assert_eq!(i.features, vec![8, 14, 99]);
    assert_eq!(i.amount_msat, Some(2_500_000_000));
    assert_eq!(invoice("all upper case"), invoice("features 8, 14 and 99"));
    let i = invoice("fields which must be ignored");
    assert!(!i.unknown.is_empty());
    assert_eq!(
        invoice("payment metadata").metadata,
        Some(vec![0x01, 0xfa, 0xfa, 0xf0])
    );
}

#[test]
fn recovered_key_with_high_s_and_the_n_field() {
    // The recovered key accepts a high-S signature.
    let i = invoice("high-S");
    assert!(!i.payee_from_field);
    // The key the signature recovers to, as the `bolt11` package also reads it
    // (signed with `NODE`'s key, but its recovery id is the low-S signature's,
    // so recovery gives another key).
    assert_eq!(
        hex(&i.payee),
        "02d0139ce7427d6dfffd26a326c18be754ef1e64672b42694ba5b23ef6e6e7803d"
    );
    // A signature checked against an `n` field must be low-S.
    let (_, _, text) = vectors()
        .into_iter()
        .find(|(d, _, _)| d.contains("Non canonical"))
        .unwrap();
    assert!(decode(&text).is_err());
}

#[test]
fn refusals_say_why() {
    let reason = |what: &str| {
        let (_, _, text) = vectors()
            .into_iter()
            .find(|(d, _, _)| d.contains(what))
            .unwrap();
        decode(&text).unwrap_err()
    };
    assert_eq!(reason("Bech32 checksum is invalid"), Error::Checksum);
    assert_eq!(reason("no 1"), Error::Bech32);
    assert_eq!(reason("mixed case"), Error::MixedCase);
    assert_eq!(reason("String is too short"), Error::TooShort);
    assert_eq!(reason("Invalid multiplier"), Error::Amount);
    assert_eq!(reason("sub-millisatoshi"), Error::Amount);
    assert_eq!(reason("Missing required `p`"), Error::NoPaymentHash);
    assert_eq!(reason("Missing required `s`"), Error::NoPaymentSecret);
    assert_eq!(reason("Two distinct s"), Error::Duplicate('s'));
    assert_eq!(
        reason("unknown feature 100"),
        Error::UnknownRequiredFeature(100)
    );
    assert_eq!(reason("`n` field of invalid length"), Error::Field('n'));
    assert_eq!(reason("not recoverable"), Error::Signature);
}

#[test]
fn prefix_and_length() {
    let (_, _, text) = vectors().into_iter().next().unwrap();
    let i = decode(&format!("lightning:{text}")).unwrap();
    assert_eq!(i, decode(&text).unwrap());
    assert_eq!(
        decode(&format!("  LIGHTNING:{}\n", text.to_uppercase())).unwrap(),
        i
    );
    assert_eq!(decode(&"a".repeat(1024)), Err(Error::TooLong));
    assert!(decode("").is_err());
    assert!(decode("lnbc1").is_err());
}

/// An invoice's text from its fields, each a tag and its values, with a
/// signature that does not verify: the reader reads the fields first.
fn unsigned(fields: &[(u8, Vec<u8>)]) -> String {
    use bitcoin::bech32::primitives::iter::Fe32IterExt;
    use bitcoin::bech32::{Bech32, Fe32, Hrp};
    let mut values = vec![0u8; 7];
    for (tag, data) in fields {
        values.extend([*tag, (data.len() >> 5) as u8, (data.len() & 31) as u8]);
        values.extend(data);
    }
    values.extend([0u8; 104]);
    values
        .into_iter()
        .map(|v| Fe32::try_from(v).unwrap())
        .with_checksum::<Bech32>(&Hrp::parse("lnbc").unwrap())
        .chars()
        .collect()
}

#[test]
fn expiry_cltv_and_features_with_a_leading_zero_are_refused() {
    // BOLT #11 says an invoice with such a field should be treated as invalid.
    // p, s and d are tags 1, 16 and 13; x, c and 9 are 6, 24 and 5.
    let base = || vec![(1, vec![0; 52]), (16, vec![0; 52]), (13, vec![0; 8])];
    for (tag, letter) in [(6u8, 'x'), (24, 'c'), (5, '9')] {
        let with = |data: Vec<u8>| {
            let mut f = base();
            f.push((tag, data));
            decode(&unsigned(&f))
        };
        assert_eq!(with(vec![0, 10]), Err(Error::Field(letter)));
        // A lone zero value is not minimal either: zero is an empty field.
        assert_eq!(with(vec![0]), Err(Error::Field(letter)));
        assert_eq!(with(vec![]), Err(Error::Signature));
        // The same value without the zero is read, and only the signature fails.
        assert_eq!(with(vec![10]), Err(Error::Signature));
    }
}
