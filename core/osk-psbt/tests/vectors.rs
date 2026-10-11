//! BIP-174 test vectors: every valid PSBT round-trips byte for byte in
//! both encodings, every invalid one is rejected, and the "fails signer
//! checks" PSBTs block signing in the inspection.

mod common;

use common::{records, unhex};
use osk_bip::keys::Network;
use osk_psbt::{Context, Error, Level, Psbt, WarningKind, inspect};

#[test]
fn bip174_round_trips_and_rejections() {
    let recs = records("tools/vectors/psbt/bip174.txt");
    assert_eq!(recs.len(), 34);
    let (mut valid, mut invalid, mut signer_fails) = (0, 0, 0);
    for rec in &recs {
        let case = rec.get("case");
        let bytes = unhex(rec.get("hex"));
        let b64 = rec.get("base64");
        match rec.get("expect") {
            "invalid" => {
                assert!(
                    Psbt::parse_bytes(&bytes).is_err(),
                    "accepted invalid: {case}"
                );
                assert!(Psbt::parse_base64(b64).is_err(), "accepted invalid: {case}");
                invalid += 1;
            }
            expect => {
                let psbt = Psbt::parse_bytes(&bytes).unwrap_or_else(|e| panic!("{case}: {e}"));
                assert_eq!(psbt.to_bytes(), bytes, "binary round trip: {case}");
                assert_eq!(psbt.to_base64(), b64, "base64 round trip: {case}");
                let from_text = Psbt::parse_base64(b64).unwrap();
                assert_eq!(from_text, psbt);
                assert_eq!(Psbt::parse_base64(&psbt.to_base64()).unwrap(), psbt);
                if expect == "signer-fails" {
                    let ctx = Context {
                        network: Network::Testnet,
                        keys: &[],
                        wallets: &[],
                        musig_session: None,
                        shares: &[],
                        carry: None,
                    };
                    let insp = inspect(&psbt, &ctx);
                    assert!(
                        insp.warnings
                            .iter()
                            .any(|w| w.level == Level::Blocked
                                && w.kind == WarningKind::UtxoMismatch),
                        "{case}: expected a UtxoMismatch block, got {:?}",
                        insp.warnings
                    );
                    signer_fails += 1;
                } else {
                    valid += 1;
                }
            }
        }
    }
    assert_eq!((valid, invalid, signer_fails), (10, 20, 4));
}

/// Every role vector parses and re-serialises identically, except the
/// combiner output: the BIP lists its two partial signatures in the order
/// Core inserted them (`03…` before `02…`), while `bitcoin` serialises
/// every map sorted by key. BIP-174 does not mandate an order, so that
/// vector is compared after parsing.
#[test]
fn bip174_role_vectors_round_trip() {
    let recs = records("tools/vectors/psbt/bip174-roles.txt");
    let mut n = 0;
    for rec in recs.iter().filter(|r| r.opt("hex").is_some()) {
        let case = rec.get("case");
        let bytes = unhex(rec.get("hex"));
        let psbt = Psbt::parse_bytes(&bytes).unwrap_or_else(|e| panic!("{case}: {e}"));
        assert_eq!(
            Psbt::parse_base64(rec.get("base64")).unwrap(),
            psbt,
            "{case}"
        );
        let again = psbt.to_bytes();
        assert_eq!(Psbt::parse_bytes(&again).unwrap(), psbt, "{case}");
        if case == "role combiner" {
            assert_ne!(again, bytes, "unsorted vector, see doc comment");
        } else {
            assert_eq!(again, bytes, "{case}");
            assert_eq!(psbt.to_base64(), rec.get("base64"), "{case}");
        }
        n += 1;
    }
    assert_eq!(n, 10);
}

#[test]
fn parse_errors() {
    assert_eq!(Psbt::parse_bytes(b"psbt"), Err(Error::NotPsbt));
    assert_eq!(Psbt::parse_bytes(b"\x02\x00\x00\x00"), Err(Error::NotPsbt));
    assert_eq!(Psbt::parse_base64("cHNidP8*"), Err(Error::Base64));
    assert!(matches!(
        Psbt::parse_base64("cHNidP8="),
        Err(Error::Parse(_))
    ));
    // Trailing newline and surrounding whitespace are fine.
    let ok = "cHNidP8BAAoAAAAAAAAAAAAAAA==\n";
    assert!(Psbt::parse_base64(ok).is_ok());
    assert!(Psbt::parse_base64(&format!("  {ok}\n")).is_ok());
}
