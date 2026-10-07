//! BIP 93's own test vectors, extracted from the BIP text into
//! `tools/vectors/bip93/vectors.json` by the script beside it: the
//! unshared secrets, the two share sets with the shares they derive and
//! the secret they recover, the alternative paddings of one seed, and
//! every string the BIP refuses.
//!
//! Where the BIP states a master node xprv, the recovered seed is run
//! through `bitcoin::bip32` and compared against it.

mod common;

use common::{hex, unhex};
use osk_bip::bitcoin::{Network, bip32::Xpriv};
use osk_bip::codex32::{Codex32, Error};
use serde_json::Value;

const VECTORS: &str = include_str!("../../../tools/vectors/bip93/vectors.json");

fn vectors() -> Value {
    serde_json::from_str(VECTORS).unwrap()
}

/// The refusal a call returns. A `Codex32` has no `Debug`, so
/// `unwrap_err` cannot be used on one.
fn err<T>(r: Result<T, Error>) -> Error {
    match r {
        Ok(_) => panic!("the call was accepted"),
        Err(e) => e,
    }
}

fn seed_of(s: &str) -> Vec<u8> {
    Codex32::parse(s)
        .unwrap()
        .to_seed()
        .unwrap()
        .expose()
        .bytes()
        .to_vec()
}

fn xprv_of(seed: &[u8]) -> String {
    Xpriv::new_master(Network::Bitcoin, seed)
        .unwrap()
        .to_string()
}

#[test]
fn unshared_secrets_carry_their_master_seed() {
    let v = vectors();
    for case in v["secrets"].as_array().unwrap() {
        let string = case["string"].as_str().unwrap();
        let name = case["vector"].as_str().unwrap();
        let s = Codex32::parse(string).unwrap();

        assert!(s.is_secret(), "vector {name}: index s");
        assert_eq!(s.share_index(), b's', "vector {name}");
        let seed = s.to_seed().unwrap();
        assert_eq!(
            hex(seed.expose().bytes()),
            case["seed"].as_str().unwrap(),
            "vector {name}: master seed"
        );
        assert_eq!(
            s.encode().as_str(),
            string.to_lowercase(),
            "vector {name}: re-encoded"
        );
        if let Some(xprv) = case["xprv"].as_str() {
            assert_eq!(xprv_of(seed.expose().bytes()), xprv, "vector {name}: xprv");
        }
    }
}

#[test]
fn a_secret_states_its_header() {
    let s = Codex32::parse("ms13cashsllhdmn9m42vcsamx24zrxgs3qqjzqud4m0d6nln").unwrap();
    assert_eq!(s.threshold(), 3);
    assert_eq!(&s.identifier(), b"cash");
    assert_eq!(s.share_index(), b's');

    // Test vector 5 names the parts of a long string; the identifier and
    // index read the same from the uppercase form.
    let long = Codex32::parse(
        "MS100C8VSM32ZXFGUHPCHTLUPZRY9X8GF2TVDW0S3JN54KHCE6MUA7LQPZYGSFJD6AN074RXVC\
         EMLH8WU3TK925ACDEFGHJKLMNPQRSTUVWXY06FHPV80UNDVARHRAK",
    )
    .unwrap();
    assert_eq!(long.threshold(), 0);
    assert_eq!(&long.identifier(), b"0c8v");
    assert_eq!(long.payload().len(), 103);
    assert_eq!(long.to_seed().unwrap().expose().bytes().len(), 64);
}

#[test]
fn shares_recover_the_secret_and_derive_more() {
    let v = vectors();
    for case in v["sets"].as_array().unwrap() {
        let name = case["vector"].as_str().unwrap();
        let given: Vec<&str> = case["shares"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap())
            .collect();
        let secret = case["secret"].as_str().unwrap();
        let seed = case["seed"].as_str().unwrap();

        // Every share the BIP derives from the given ones, in turn.
        let mut all: Vec<String> = given.iter().map(|s| s.to_lowercase()).collect();
        for (index, expected) in case["derived"].as_object().unwrap() {
            let mut set: Vec<Codex32> = given.iter().map(|s| Codex32::parse(s).unwrap()).collect();
            if set.len() < Codex32::parse(secret).unwrap().threshold() as usize {
                set.push(Codex32::parse(secret).unwrap());
            }
            let share =
                Codex32::derive_share(&set, index.as_bytes()[0].to_ascii_lowercase()).unwrap();
            assert_eq!(
                share.encode().as_str(),
                expected.as_str().unwrap().to_lowercase(),
                "vector {name}: share {index}"
            );
            all.push(share.encode().as_str().into());
        }

        // Any k of the shares recover the secret and its master seed.
        let k = Codex32::parse(secret).unwrap().threshold() as usize;
        for combo in combinations(all.len(), k) {
            let set: Vec<Codex32> = combo
                .iter()
                .map(|&i| Codex32::parse(&all[i]).unwrap())
                .collect();
            let got = Codex32::recover(&set).unwrap();
            assert_eq!(
                got.encode().as_str(),
                secret.to_lowercase(),
                "vector {name}: recovered from {combo:?}"
            );
            assert_eq!(hex(got.to_seed().unwrap().expose().bytes()), seed);
        }

        if let Some(xprv) = case["xprv"].as_str() {
            assert_eq!(xprv_of(&unhex(seed)), xprv, "vector {name}: xprv");
        }
    }
}

#[test]
fn padding_bits_do_not_change_the_seed() {
    let v = vectors();
    for case in v["alternates"].as_array().unwrap() {
        let seed = case["seed"].as_str().unwrap();
        for string in case["strings"].as_array().unwrap() {
            assert_eq!(
                hex(&seed_of(string.as_str().unwrap())),
                seed,
                "vector {}: {}",
                case["vector"].as_str().unwrap(),
                string.as_str().unwrap()
            );
        }
    }
}

#[test]
fn every_invalid_string_is_refused() {
    let v = vectors();
    let mut seen = 0;
    for group in v["invalid"].as_array().unwrap() {
        let reason = group["reason"].as_str().unwrap();
        for string in group["strings"].as_array().unwrap() {
            let string = string.as_str().unwrap();
            assert!(
                Codex32::parse(string).is_err(),
                "accepted a string the BIP refuses ({reason}): {string}"
            );
            seen += 1;
        }
    }
    assert_eq!(seen, 55, "the BIP lists 55 invalid strings");
}

#[test]
fn each_refusal_is_named() {
    let cases = [
        (
            "ms10fauxsxxxxxxxxxxxxxxxxxxxxxxxxxxve740yyge2ghq",
            Error::BadChecksum,
        ),
        (
            "ms10fauxxxxxxxxxxxxxxxxxxxxxxxxxxxx0z26tfn0ulw3p",
            Error::SecretIndexRequired,
        ),
        (
            "ms1fauxxxxxxxxxxxxxxxxxxxxxxxxxxxxxda3kr3s0s2swg",
            Error::BadThreshold,
        ),
        (
            "m10fauxsxxxxxxxxxxxxxxxxxxxxxxxxxx8t28z74x8hs4l",
            Error::BadPrefix,
        ),
        (
            "Ms10fauxsxxxxxxxxxxxxxxxxxxxxxxxxxxuqxkk05lyf3x2",
            Error::MixedCase,
        ),
        (
            "ms10fauxsxxxxxxxxxxxxxxxxxxxxxxxxw0a4c70rfefn4",
            Error::BadLength,
        ),
    ];
    for (string, expected) in cases {
        assert_eq!(err(Codex32::parse(string)), expected, "{string}");
    }

    // `b` is not in the bech32 alphabet; the refusal names where it is.
    let mut bytes = b"ms13cashsllhdmn9m42vcsamx24zrxgs3qqjzqud4m0d6nln".to_vec();
    bytes[9] = b'b';
    assert_eq!(
        err(Codex32::parse(core::str::from_utf8(&bytes).unwrap())),
        Error::BadCharacter { position: 9 }
    );
}

#[test]
fn one_changed_character_is_caught() {
    // The checksum could correct up to four substitutions; this module
    // only detects them, and a string that fails is refused outright.
    let good = "ms13cashsllhdmn9m42vcsamx24zrxgs3qqjzqud4m0d6nln";
    for position in 4..good.len() {
        let mut bytes = good.as_bytes().to_vec();
        bytes[position] = if bytes[position] == b'q' { b'p' } else { b'q' };
        let broken = String::from_utf8(bytes).unwrap();
        assert!(
            matches!(Codex32::parse(&broken), Err(Error::BadChecksum)),
            "accepted {broken}"
        );
    }
}

#[test]
fn a_seed_encodes_to_the_string_the_bip_gives() {
    // Test vectors 3 and 4 are the zero-padded choice among the encodings
    // the BIP lists, which is what this module writes.
    let v3 = "ms13cashsllhdmn9m42vcsamx24zrxgs3qqjzqud4m0d6nln";
    let s3 = Codex32::from_seed(3, *b"cash", &unhex("ffeeddccbbaa99887766554433221100")).unwrap();
    assert_eq!(s3.encode().as_str(), v3);

    let v4 = "ms10leetsllhdmn9m42vcsamx24zrxgs3qrl7ahwvhw4fnzrhve25gvezzyqqtum9pgv99ycma";
    let seed4 = unhex("ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100");
    assert_eq!(
        Codex32::from_seed(0, *b"leet", &seed4)
            .unwrap()
            .encode()
            .as_str(),
        v4
    );

    let v6 = "ms10seedsqqqsyqcyq5rqwzqfpg9scrgwpugpzysn9vaqzzvs20xnl";
    let seed6 = unhex("000102030405060708090a0b0c0d0e0f10111213");
    assert_eq!(
        Codex32::from_seed(0, *b"seed", &seed6)
            .unwrap()
            .encode()
            .as_str(),
        v6
    );

    // Every seed size round-trips, long checksum included.
    for len in [16, 20, 24, 28, 32, 64] {
        let seed: Vec<u8> = (0..len).map(|i| i as u8).collect();
        let s = Codex32::from_seed(2, *b"cafe", &seed).unwrap();
        assert_eq!(seed_of(s.encode().as_str()), seed, "{len}-byte seed");
    }

    assert_eq!(
        err(Codex32::from_seed(0, *b"seed", &[0u8; 17])),
        Error::BadSeedLength
    );
    assert_eq!(
        err(Codex32::from_seed(0, *b"seeb", &[0u8; 16])),
        Error::BadIdentifier
    );
}

#[test]
fn a_split_seed_comes_back_from_any_threshold_of_its_shares() {
    let seed = unhex("ffeeddccbbaa99887766554433221100");
    let secret = Codex32::from_seed(0, *b"cafe", &seed).unwrap();
    let fresh: Vec<u8> = (0..=255u8).cycle().take(256).collect();

    let shares = secret.split(3, 5, &fresh).unwrap();
    assert_eq!(shares.len(), 5);
    let indices: Vec<u8> = shares.iter().map(|s| s.share_index()).collect();
    assert_eq!(indices, b"acdef".to_vec());
    for share in &shares {
        assert_eq!(share.threshold(), 3);
        assert_eq!(&share.identifier(), b"cafe");
        assert!(!share.is_secret());
        // Each share is a valid codex32 string in its own right.
        Codex32::parse(share.encode().as_str()).unwrap();
    }

    for combo in combinations(5, 3) {
        let set: Vec<Codex32> = combo
            .iter()
            .map(|&i| Codex32::parse(shares[i].encode().as_str()).unwrap())
            .collect();
        let got = Codex32::recover(&set).unwrap();
        assert_eq!(
            got.to_seed().unwrap().expose().bytes(),
            &seed[..],
            "{combo:?}"
        );
    }

    // Two of the three are not enough, and the secret is not a share.
    let two: Vec<Codex32> = (0..2)
        .map(|i| Codex32::parse(shares[i].encode().as_str()).unwrap())
        .collect();
    assert_eq!(err(Codex32::recover(&two)), Error::WrongShareCount);
}

#[test]
fn a_set_that_does_not_hold_together_is_refused() {
    let a = || Codex32::parse("ms13casha320zyxwvutsrqpnmlkjhgfedca2a8d0zehn8a0t").unwrap();
    let c = || Codex32::parse("ms13cashcacdefghjklmnpqrstuvwxyz023949xq35my48dr").unwrap();
    let d = || Codex32::parse("ms13cashd0wsedstcdcts64cd7wvy4m90lm28w4ffupqs7rm").unwrap();
    let secret = || Codex32::parse("ms13cashsllhdmn9m42vcsamx24zrxgs3qqjzqud4m0d6nln").unwrap();

    assert_eq!(
        err(Codex32::recover(&[a(), c(), a()])),
        Error::DuplicateIndex
    );
    assert_eq!(
        err(Codex32::recover(&[a(), c(), secret()])),
        Error::WrongKind
    );
    assert_eq!(
        err(Codex32::recover(&[a(), c(), d(), d()])),
        Error::DuplicateIndex
    );

    // A share of another set: same length, another threshold and name.
    let name_a = Codex32::parse("MS12NAMEA320ZYXWVUTSRQPNMLKJHGFEDCAXRPP870HKKQRM").unwrap();
    assert_eq!(
        err(Codex32::recover(&[a(), c(), name_a])),
        Error::ThresholdMismatch
    );

    // Deriving a share needs an index that is free and in the alphabet.
    assert_eq!(
        err(Codex32::derive_share(&[a(), c(), d()], b's')),
        Error::BadShareIndex
    );
    assert_eq!(
        err(Codex32::derive_share(&[a(), c(), d()], b'a')),
        Error::BadShareIndex
    );
    assert_eq!(
        err(Codex32::derive_share(&[a(), c(), d()], b'i')),
        Error::BadShareIndex
    );
    assert_eq!(
        err(Codex32::derive_share(&[a(), c()], b'g')),
        Error::WrongShareCount
    );

    // Two sets of the same shape and threshold, told apart by their name.
    let seed = unhex("ffeeddccbbaa99887766554433221100");
    let fresh: Vec<u8> = (0..=255u8).cycle().take(64).collect();
    let one = Codex32::from_seed(0, *b"cafe", &seed)
        .unwrap()
        .split(2, 2, &fresh)
        .unwrap();
    let two = Codex32::from_seed(0, *b"cafc", &seed)
        .unwrap()
        .split(2, 2, &fresh)
        .unwrap();
    let mixed = [
        Codex32::parse(one[0].encode().as_str()).unwrap(),
        Codex32::parse(two[1].encode().as_str()).unwrap(),
    ];
    assert_eq!(err(Codex32::recover(&mixed)), Error::IdentifierMismatch);

    // A share carries no seed.
    assert_eq!(err(a().to_seed()), Error::WrongKind);
}

/// Every way to choose `k` of `n` positions, in order.
fn combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    fn go(start: usize, n: usize, k: usize, pick: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if pick.len() == k {
            out.push(pick.clone());
            return;
        }
        for i in start..n {
            pick.push(i);
            go(i + 1, n, k, pick, out);
            pick.pop();
        }
    }
    let mut out = Vec::new();
    go(0, n, k, &mut Vec::new(), &mut out);
    out
}
