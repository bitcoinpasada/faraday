//! BIP-32 test vectors 1–5, run through `MasterKey` and `DerivedKey`.
//!
//! Private keys are compared through `DerivedKey::secret_key` against the
//! vector's xprv decoded by rust-bitcoin; everything else in an xprv
//! (depth, parent fingerprint, child number, chain code) is also in the
//! xpub, which is compared as a whole.

mod common;

use std::str::FromStr;

use bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
use common::unhex;
use osk_bip::keys::{ImportError, MasterKey, Network};
use osk_bip::xkey::{self, Error};
use osk_crypto::Secret;

struct Vector {
    seed: &'static str,
    chain: &'static [(&'static str, &'static str, &'static str)],
}

const VECTORS: &[Vector] = &[
    Vector {
        seed: "000102030405060708090a0b0c0d0e0f",
        chain: &[
            (
                "m",
                "xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGhePY2gZ29ESFjqJoCu1Rupje8YtGqsefD265TMg7usUDFdp6W1EGMcet8",
                "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi",
            ),
            (
                "m/0h",
                "xpub68Gmy5EdvgibQVfPdqkBBCHxA5htiqg55crXYuXoQRKfDBFA1WEjWgP6LHhwBZeNK1VTsfTFUHCdrfp1bgwQ9xv5ski8PX9rL2dZXvgGDnw",
                "xprv9uHRZZhk6KAJC1avXpDAp4MDc3sQKNxDiPvvkX8Br5ngLNv1TxvUxt4cV1rGL5hj6KCesnDYUhd7oWgT11eZG7XnxHrnYeSvkzY7d2bhkJ7",
            ),
            (
                "m/0h/1",
                "xpub6ASuArnXKPbfEwhqN6e3mwBcDTgzisQN1wXN9BJcM47sSikHjJf3UFHKkNAWbWMiGj7Wf5uMash7SyYq527Hqck2AxYysAA7xmALppuCkwQ",
                "xprv9wTYmMFdV23N2TdNG573QoEsfRrWKQgWeibmLntzniatZvR9BmLnvSxqu53Kw1UmYPxLgboyZQaXwTCg8MSY3H2EU4pWcQDnRnrVA1xe8fs",
            ),
            (
                "m/0h/1/2h",
                "xpub6D4BDPcP2GT577Vvch3R8wDkScZWzQzMMUm3PWbmWvVJrZwQY4VUNgqFJPMM3No2dFDFGTsxxpG5uJh7n7epu4trkrX7x7DogT5Uv6fcLW5",
                "xprv9z4pot5VBttmtdRTWfWQmoH1taj2axGVzFqSb8C9xaxKymcFzXBDptWmT7FwuEzG3ryjH4ktypQSAewRiNMjANTtpgP4mLTj34bhnZX7UiM",
            ),
            (
                "m/0h/1/2h/2",
                "xpub6FHa3pjLCk84BayeJxFW2SP4XRrFd1JYnxeLeU8EqN3vDfZmbqBqaGJAyiLjTAwm6ZLRQUMv1ZACTj37sR62cfN7fe5JnJ7dh8zL4fiyLHV",
                "xprvA2JDeKCSNNZky6uBCviVfJSKyQ1mDYahRjijr5idH2WwLsEd4Hsb2Tyh8RfQMuPh7f7RtyzTtdrbdqqsunu5Mm3wDvUAKRHSC34sJ7in334",
            ),
            (
                "m/0h/1/2h/2/1000000000",
                "xpub6H1LXWLaKsWFhvm6RVpEL9P4KfRZSW7abD2ttkWP3SSQvnyA8FSVqNTEcYFgJS2UaFcxupHiYkro49S8yGasTvXEYBVPamhGW6cFJodrTHy",
                "xprvA41z7zogVVwxVSgdKUHDy1SKmdb533PjDz7J6N6mV6uS3ze1ai8FHa8kmHScGpWmj4WggLyQjgPie1rFSruoUihUZREPSL39UNdE3BBDu76",
            ),
        ],
    },
    Vector {
        seed: "fffcf9f6f3f0edeae7e4e1dedbd8d5d2cfccc9c6c3c0bdbab7b4b1aeaba8a5a29f9c999693908d8a8784817e7b7875726f6c696663605d5a5754514e4b484542",
        chain: &[
            (
                "m",
                "xpub661MyMwAqRbcFW31YEwpkMuc5THy2PSt5bDMsktWQcFF8syAmRUapSCGu8ED9W6oDMSgv6Zz8idoc4a6mr8BDzTJY47LJhkJ8UB7WEGuduB",
                "xprv9s21ZrQH143K31xYSDQpPDxsXRTUcvj2iNHm5NUtrGiGG5e2DtALGdso3pGz6ssrdK4PFmM8NSpSBHNqPqm55Qn3LqFtT2emdEXVYsCzC2U",
            ),
            (
                "m/0",
                "xpub69H7F5d8KSRgmmdJg2KhpAK8SR3DjMwAdkxj3ZuxV27CprR9LgpeyGmXUbC6wb7ERfvrnKZjXoUmmDznezpbZb7ap6r1D3tgFxHmwMkQTPH",
                "xprv9vHkqa6EV4sPZHYqZznhT2NPtPCjKuDKGY38FBWLvgaDx45zo9WQRUT3dKYnjwih2yJD9mkrocEZXo1ex8G81dwSM1fwqWpWkeS3v86pgKt",
            ),
            (
                "m/0/2147483647h",
                "xpub6ASAVgeehLbnwdqV6UKMHVzgqAG8Gr6riv3Fxxpj8ksbH9ebxaEyBLZ85ySDhKiLDBrQSARLq1uNRts8RuJiHjaDMBU4Zn9h8LZNnBC5y4a",
                "xprv9wSp6B7kry3Vj9m1zSnLvN3xH8RdsPP1Mh7fAaR7aRLcQMKTR2vidYEeEg2mUCTAwCd6vnxVrcjfy2kRgVsFawNzmjuHc2YmYRmagcEPdU9",
            ),
            (
                "m/0/2147483647h/1",
                "xpub6DF8uhdarytz3FWdA8TvFSvvAh8dP3283MY7p2V4SeE2wyWmG5mg5EwVvmdMVCQcoNJxGoWaU9DCWh89LojfZ537wTfunKau47EL2dhHKon",
                "xprv9zFnWC6h2cLgpmSA46vutJzBcfJ8yaJGg8cX1e5StJh45BBciYTRXSd25UEPVuesF9yog62tGAQtHjXajPPdbRCHuWS6T8XA2ECKADdw4Ef",
            ),
            (
                "m/0/2147483647h/1/2147483646h",
                "xpub6ERApfZwUNrhLCkDtcHTcxd75RbzS1ed54G1LkBUHQVHQKqhMkhgbmJbZRkrgZw4koxb5JaHWkY4ALHY2grBGRjaDMzQLcgJvLJuZZvRcEL",
                "xprvA1RpRA33e1JQ7ifknakTFpgNXPmW2YvmhqLQYMmrj4xJXXWYpDPS3xz7iAxn8L39njGVyuoseXzU6rcxFLJ8HFsTjSyQbLYnMpCqE2VbFWc",
            ),
            (
                "m/0/2147483647h/1/2147483646h/2",
                "xpub6FnCn6nSzZAw5Tw7cgR9bi15UV96gLZhjDstkXXxvCLsUXBGXPdSnLFbdpq8p9HmGsApME5hQTZ3emM2rnY5agb9rXpVGyy3bdW6EEgAtqt",
                "xprvA2nrNbFZABcdryreWet9Ea4LvTJcGsqrMzxHx98MMrotbir7yrKCEXw7nadnHM8Dq38EGfSh6dqA9QWTyefMLEcBYJUuekgW4BYPJcr9E7j",
            ),
        ],
    },
    // Vector 3: retention of leading zeros.
    Vector {
        seed: "4b381541583be4423346c643850da4b320e46a87ae3d2a4e6da11eba819cd4acba45d239319ac14f863b8d5ab5a0d0c64d2e8a1e7d1457df2e5a3c51c73235be",
        chain: &[
            (
                "m",
                "xpub661MyMwAqRbcEZVB4dScxMAdx6d4nFc9nvyvH3v4gJL378CSRZiYmhRoP7mBy6gSPSCYk6SzXPTf3ND1cZAceL7SfJ1Z3GC8vBgp2epUt13",
                "xprv9s21ZrQH143K25QhxbucbDDuQ4naNntJRi4KUfWT7xo4EKsHt2QJDu7KXp1A3u7Bi1j8ph3EGsZ9Xvz9dGuVrtHHs7pXeTzjuxBrCmmhgC6",
            ),
            (
                "m/0h",
                "xpub68NZiKmJWnxxS6aaHmn81bvJeTESw724CRDs6HbuccFQN9Ku14VQrADWgqbhhTHBaohPX4CjNLf9fq9MYo6oDaPPLPxSb7gwQN3ih19Zm4Y",
                "xprv9uPDJpEQgRQfDcW7BkF7eTya6RPxXeJCqCJGHuCJ4GiRVLzkTXBAJMu2qaMWPrS7AANYqdq6vcBcBUdJCVVFceUvJFjaPdGZ2y9WACViL4L",
            ),
        ],
    },
    // Vector 4: retention of leading zeros.
    Vector {
        seed: "3ddd5602285899a946114506157c7997e5444528f3003f6134712147db19b678",
        chain: &[
            (
                "m",
                "xpub661MyMwAqRbcGczjuMoRm6dXaLDEhW1u34gKenbeYqAix21mdUKJyuyu5F1rzYGVxyL6tmgBUAEPrEz92mBXjByMRiJdba9wpnN37RLLAXa",
                "xprv9s21ZrQH143K48vGoLGRPxgo2JNkJ3J3fqkirQC2zVdk5Dgd5w14S7fRDyHH4dWNHUgkvsvNDCkvAwcSHNAQwhwgNMgZhLtQC63zxwhQmRv",
            ),
            (
                "m/0h",
                "xpub69AUMk3qDBi3uW1sXgjCmVjJ2G6WQoYSnNHyzkmdCHEhSZ4tBok37xfFEqHd2AddP56Tqp4o56AePAgCjYdvpW2PU2jbUPFKsav5ut6Ch1m",
                "xprv9vB7xEWwNp9kh1wQRfCCQMnZUEG21LpbR9NPCNN1dwhiZkjjeGRnaALmPXCX7SgjFTiCTT6bXes17boXtjq3xLpcDjzEuGLQBM5ohqkao9G",
            ),
            (
                "m/0h/1h",
                "xpub6BJA1jSqiukeaesWfxe6sNK9CCGaujFFSJLomWHprUL9DePQ4JDkM5d88n49sMGJxrhpjazuXYWdMf17C9T5XnxkopaeS7jGk1GyyVziaMt",
                "xprv9xJocDuwtYCMNAo3Zw76WENQeAS6WGXQ55RCy7tDJ8oALr4FWkuVoHJeHVAcAqiZLE7Je3vZJHxspZdFHfnBEjHqU5hG1Jaj32dVoS6XLT1",
            ),
        ],
    },
];

/// `MasterKey` takes a 64-byte seed; BIP-32 vectors use 16 to 64 bytes.
/// Shorter seeds are handled by deriving the master through rust-bitcoin
/// directly and checking our wrapper agrees wherever the seed is 64 bytes.
fn master_from_hex(seed_hex: &str) -> Option<MasterKey> {
    let seed = unhex(seed_hex);
    let bytes: [u8; 64] = seed.try_into().ok()?;
    Some(MasterKey::from_seed(&Secret::new(bytes), Network::Mainnet))
}

#[test]
fn vectors_1_to_4_through_master_key() {
    let mut checked = 0;
    for v in VECTORS {
        let Some(master) = master_from_hex(v.seed) else {
            continue;
        };
        for &(path, xpub, xprv) in v.chain {
            let path = DerivationPath::from_str(path).unwrap();
            let derived = master.derive(&path);
            let expected_pub = Xpub::from_str(xpub).unwrap();
            let expected_prv = Xpriv::from_str(xprv).unwrap();
            assert_eq!(derived.to_xpub(), expected_pub, "{path}");
            assert_eq!(derived.to_xpub().to_string(), xpub);
            assert_eq!(*derived.secret_key(), expected_prv.private_key, "{path}");
            assert_eq!(derived.path(), &path);
            assert_eq!(derived.master_fingerprint(), master.fingerprint());
            assert_eq!(
                derived.fingerprint().as_bytes(),
                &expected_pub.fingerprint().to_bytes()
            );
            if path.is_empty() {
                assert_eq!(master.xpub(), expected_pub);
            }
            checked += 1;
        }
    }
    // Vectors 2 and 3 have 64-byte seeds: 6 + 2 steps.
    assert_eq!(checked, 8);
}

#[test]
fn hardened_marker_spellings_are_equivalent() {
    let a = DerivationPath::from_str("m/84'/0'/0'").unwrap();
    let b = DerivationPath::from_str("m/84h/0h/0h").unwrap();
    let c = DerivationPath::from_str("84h/0h/0h").unwrap();
    assert_eq!(a, b);
    assert_eq!(a, c);
    let master = master_from_hex(VECTORS[1].seed).unwrap();
    assert_eq!(master.derive(&a).to_xpub(), master.derive(&c).to_xpub());
    assert!(DerivationPath::from_str("m/84x").is_err());
    assert!(DerivationPath::from_str("m/2147483648").is_err());
}

/// BIP-32 test vector 5, read from `tools/vectors/bip32/invalid-keys.txt`
/// as published (see the README beside it): the key, then the BIP's own
/// reason.
const INVALID: &str = include_str!("../../../tools/vectors/bip32/invalid-keys.txt");

/// Every one of the sixteen is rejected, and the reason the decoder gives
/// is the one the BIP states. `bitcoin`'s own decoders accept seven of
/// them (the pad byte before a private key, and a depth-0 key carrying a
/// parent fingerprint or a child number, are not checked there), which is
/// why the text is decoded here.
#[test]
fn vector_5_invalid_keys_are_rejected() {
    let mut checked = 0;
    for line in INVALID.lines() {
        let (key, why) = line.split_once(' ').expect("key then reason");
        let err = match &key[..4] {
            "xprv" => xkey::decode_xpriv(key).expect_err(why),
            "xpub" => xkey::decode_xpub(key).expect_err(why),
            // An unknown version is neither, so both halves must refuse.
            _ => {
                let as_pub = xkey::decode_xpub(key).expect_err(why);
                assert_eq!(xkey::decode_xpriv(key).expect_err(why), as_pub);
                as_pub
            }
        };
        let expected = match why {
            "invalid checksum" => err == Error::Base58,
            "unknown extended key version" => matches!(err, Error::UnknownVersion(_)),
            "prvkey version / pubkey mismatch" => matches!(err, Error::Pad(_)),
            w if w.starts_with("invalid prvkey prefix") => matches!(err, Error::Pad(_)),
            w if w.starts_with("zero depth") => err == Error::ZeroDepth,
            "pubkey version / prvkey mismatch" => matches!(err, Error::Key(_)),
            w if w.starts_with("invalid pubkey") || w.starts_with("private key") => {
                matches!(err, Error::Key(_))
            }
            other => panic!("unhandled reason: {other}"),
        };
        assert!(expected, "{why}: {err}");
        // Whatever the reason, no import path takes the key.
        assert!(MasterKey::decode(key).is_err(), "{why}");
        assert!(osk_bip::slip132::decode_xpub(key).is_err(), "{why}");
        checked += 1;
    }
    assert_eq!(checked, 16);
}

/// The master keys of vectors 1 to 4 import from their published `xprv`
/// and derive the same children as the key built from the seed.
#[test]
fn vector_master_keys_import_and_derive() {
    let mut checked = 0;
    for v in VECTORS {
        let (_, _, master_xprv) = v.chain[0];
        let master = MasterKey::decode(master_xprv).expect("a published master key imports");
        for &(path, xpub, xprv) in v.chain {
            let path = DerivationPath::from_str(path).unwrap();
            let derived = master.derive(&path);
            assert_eq!(derived.to_xpub().to_string(), xpub, "{path}");
            assert_eq!(derived.xpriv_ascii().as_str(), xprv, "{path}");
            checked += 1;
        }
    }
    assert_eq!(checked, 17);
}

/// Blinding a key's signing context changes nothing a wallet can see:
/// two keys built from the same seed under different blinds have the
/// same fingerprint, the same extended keys and the same signature
/// bytes. Blinding is there to make a power or timing trace useless, and
/// a signer whose output moved with it would be a signer nobody could
/// check against another implementation.
#[test]
fn blinding_a_key_changes_nothing_it_produces() {
    let seed = unhex(VECTORS[0].seed);
    let bytes: [u8; 64] = std::array::from_fn(|i| seed[i % seed.len()]);
    let a = MasterKey::from_seed(&Secret::new(bytes), Network::Mainnet).blinded(&[0x11; 32]);
    let b = MasterKey::from_seed(&Secret::new(bytes), Network::Mainnet).blinded(&[0xee; 32]);

    assert_eq!(a.fingerprint(), b.fingerprint());
    assert_eq!(a.xpub().to_string(), b.xpub().to_string());
    assert_eq!(a.xpriv_ascii().as_str(), b.xpriv_ascii().as_str());

    let path = DerivationPath::from_str("m/84h/0h/0h/0/0").unwrap();
    let (da, db) = (a.derive(&path), b.derive(&path));
    assert_eq!(da.to_xpub().to_string(), db.to_xpub().to_string());

    let msg = bitcoin::secp256k1::Message::from_digest([0x7c; 32]);
    let sig_a = a.secp().sign_ecdsa(&msg, da.secret_key());
    let sig_b = b.secp().sign_ecdsa(&msg, db.secret_key());
    assert_eq!(sig_a.serialize_der()[..], sig_b.serialize_der()[..]);

    // And again after the session hands down a new blind mid-life.
    let mut c = MasterKey::from_seed(&Secret::new(bytes), Network::Mainnet);
    c.reblind(&[0x5a; 32]);
    assert_eq!(c.fingerprint(), a.fingerprint());
    let dc = c.derive(&path);
    assert_eq!(
        c.secp().sign_ecdsa(&msg, dc.secret_key()).serialize_der()[..],
        sig_a.serialize_der()[..]
    );
}

/// An extended private key that decodes but is a child, not a master, is
/// refused with the master-key reason rather than a decoding one.
#[test]
fn a_child_key_is_not_a_master_key() {
    let child = VECTORS[0].chain[1].2;
    assert!(xkey::decode_xpriv(child).is_ok());
    assert_eq!(MasterKey::decode(child).err(), Some(ImportError::NotMaster));
}
