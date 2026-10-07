//! SLIP-39 against the vectors Trezor publishes with
//! `python-shamir-mnemonic`, and against itself.
//!
//! `tools/vectors/slip39/vectors.json` holds 45 cases: a description, the
//! mnemonics, the master secret in hex (empty when the case must be
//! refused) and the BIP-32 root key the master secret gives. The
//! passphrase throughout is `TREZOR`.

mod common;

use common::{hex, unhex};
use osk_bip::bitcoin::Network;
use osk_bip::bitcoin::bip32::Xpriv;
use osk_bip::slip39::{self, GroupSpec, Share, recover};
use osk_crypto::sha256;
use serde_json::Value;

const VECTORS: &str = include_str!("../../../tools/vectors/slip39/vectors.json");
const PASSPHRASE: &[u8] = b"TREZOR";

struct Vector {
    description: String,
    mnemonics: Vec<String>,
    master_secret: String,
    xprv: String,
}

fn vectors() -> Vec<Vector> {
    let parsed: Value = serde_json::from_str(VECTORS).unwrap();
    parsed
        .as_array()
        .unwrap()
        .iter()
        .map(|case| {
            let case = case.as_array().unwrap();
            Vector {
                description: case[0].as_str().unwrap().to_string(),
                mnemonics: case[1]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|m| m.as_str().unwrap().to_string())
                    .collect(),
                master_secret: case[2].as_str().unwrap().to_string(),
                xprv: case[3].as_str().unwrap().to_string(),
            }
        })
        .collect()
}

/// The master secret behind a case's mnemonics, or the refusal.
fn combine(mnemonics: &[String]) -> Result<Vec<u8>, slip39::Error> {
    let mut shares = Vec::new();
    for m in mnemonics {
        shares.push(Share::parse(m)?);
    }
    let secret = recover(&shares, PASSPHRASE)?;
    Ok(secret.expose().as_bytes().to_vec())
}

#[test]
fn every_published_vector_recovers_or_is_refused() {
    let mut recovered = 0;
    let mut refused = 0;
    for v in vectors() {
        match (combine(&v.mnemonics), v.master_secret.is_empty()) {
            (Ok(secret), false) => {
                assert_eq!(hex(&secret), v.master_secret, "{}", v.description);
                recovered += 1;
            }
            (Ok(_), true) => panic!("{}: accepted, should be refused", v.description),
            (Err(_), true) => refused += 1,
            (Err(e), false) => panic!("{}: refused with {e}", v.description),
        }
    }
    assert_eq!((recovered, refused), (15, 30));
}

#[test]
fn a_recovered_master_secret_is_the_bip32_seed_the_vector_states() {
    for v in vectors() {
        if v.master_secret.is_empty() {
            continue;
        }
        let secret = combine(&v.mnemonics).unwrap();
        let root = Xpriv::new_master(Network::Bitcoin, &secret).unwrap();
        assert_eq!(root.to_string(), v.xprv, "{}", v.description);
    }
}

/// A master secret read as BIP-39 entropy gives a different wallet, so
/// nothing but SLIP-39's own derivation opens what Trezor shows.
#[test]
fn the_master_secret_is_not_bip39_entropy() {
    let v = &vectors()[0];
    let secret = combine(&v.mnemonics).unwrap();
    let as_bip39 =
        osk_bip::bip39::Mnemonic::from_entropy(osk_bip::bip39::Language::English, &secret)
            .unwrap()
            .to_seed(PASSPHRASE)
            .unwrap();
    let other = Xpriv::new_master(Network::Bitcoin, as_bip39.expose()).unwrap();
    assert_ne!(other.to_string(), v.xprv);
}

#[test]
fn changing_one_word_of_a_share_is_refused() {
    let v = &vectors()[0];
    let share = Share::parse(&v.mnemonics[0]).unwrap();
    for position in 0..share.word_count() {
        let mut indices = share.indices().to_vec();
        indices[position] = (indices[position] + 1) % 1024;
        assert!(
            Share::from_indices(&indices).is_err(),
            "word {position} could be changed unnoticed"
        );
    }
}

#[test]
fn a_word_outside_the_list_is_named_by_position() {
    let v = &vectors()[0];
    let mut words: Vec<&str> = v.mnemonics[0].split(' ').collect();
    words[5] = "bitcoin";
    let mnemonic = words.join(" ");
    assert_eq!(
        Share::parse(&mnemonic).err(),
        Some(slip39::Error::UnknownWord { position: 5 })
    );
}

/// A fixed stream, so that a split is reproducible and every subset of
/// its shares can be tried.
struct Stream(u8);

impl Stream {
    fn fill(&mut self, out: &mut [u8]) {
        for b in out {
            *b = self.0;
            self.0 = self.0.wrapping_add(37).wrapping_mul(3);
        }
    }
}

fn split_to_shares(secret: &[u8], group_threshold: u8, groups: &[GroupSpec]) -> Vec<Share> {
    let mut stream = Stream(7);
    let mut out: Vec<Share> = (0..64).map(|_| Share::default()).collect();
    let n = slip39::split(
        secret,
        PASSPHRASE,
        0x1234,
        false,
        0,
        group_threshold,
        groups,
        &mut |buf| stream.fill(buf),
        &mut out,
    )
    .unwrap();
    out.truncate(n);
    out
}

#[test]
fn a_split_recombines_from_every_threshold_subset() {
    for secret in [
        unhex("bb54aac4b89dc868ba37d9cc21b2cece"),
        unhex("989baf9dcaad5b10ca33dfd8cc75e42477025dce88ae83e75a230086a0e00e92"),
    ] {
        let groups = [GroupSpec {
            threshold: 3,
            count: 5,
        }];
        let shares = split_to_shares(&secret, 1, &groups);
        assert_eq!(shares.len(), 5);

        let mut subsets = 0;
        for a in 0..5 {
            for b in a + 1..5 {
                for c in b + 1..5 {
                    let chosen = [shares[a].clone(), shares[b].clone(), shares[c].clone()];
                    let got = recover(&chosen, PASSPHRASE).unwrap();
                    assert_eq!(got.expose().as_bytes(), &secret[..]);
                    subsets += 1;
                }
            }
        }
        assert_eq!(subsets, 10);

        // Two of the five are not enough: the set is short a share.
        let two = [shares[0].clone(), shares[1].clone()];
        assert_eq!(
            recover(&two, PASSPHRASE).err(),
            Some(slip39::Error::WrongMemberCount {
                group_index: 0,
                needed: 3,
                given: 2,
            })
        );
    }
}

#[test]
fn a_two_level_split_needs_a_threshold_of_groups_and_of_members() {
    let secret = unhex("bb54aac4b89dc868ba37d9cc21b2cece");
    let groups = [
        GroupSpec {
            threshold: 1,
            count: 1,
        },
        GroupSpec {
            threshold: 2,
            count: 3,
        },
        GroupSpec {
            threshold: 3,
            count: 5,
        },
    ];
    let shares = split_to_shares(&secret, 2, &groups);
    assert_eq!(shares.len(), 9);

    let by_group = |g: u8| -> Vec<Share> {
        shares
            .iter()
            .filter(|s| s.group_index() == g)
            .cloned()
            .collect()
    };

    let mut chosen = by_group(0);
    chosen.extend(by_group(1).into_iter().take(2));
    let got = recover(&chosen, PASSPHRASE).unwrap();
    assert_eq!(got.expose().as_bytes(), &secret[..]);

    let mut chosen = by_group(1).into_iter().take(2).collect::<Vec<_>>();
    chosen.extend(by_group(2).into_iter().take(3));
    let got = recover(&chosen, PASSPHRASE).unwrap();
    assert_eq!(got.expose().as_bytes(), &secret[..]);

    // One group is not enough when two must be present.
    assert_eq!(
        recover(&by_group(0), PASSPHRASE).err(),
        Some(slip39::Error::WrongGroupCount {
            needed: 2,
            given: 1,
        })
    );
}

#[test]
fn an_extendable_split_recovers_the_same_secret() {
    let secret = unhex("bb54aac4b89dc868ba37d9cc21b2cece");
    let mut stream = Stream(19);
    let mut out: Vec<Share> = (0..8).map(|_| Share::default()).collect();
    let n = slip39::split(
        &secret,
        PASSPHRASE,
        0x0abc,
        true,
        1,
        1,
        &[GroupSpec {
            threshold: 2,
            count: 3,
        }],
        &mut |buf| stream.fill(buf),
        &mut out,
    )
    .unwrap();
    out.truncate(n);
    assert!(out.iter().all(Share::is_extendable));
    let got = recover(&out[..2], PASSPHRASE).unwrap();
    assert_eq!(got.expose().as_bytes(), &secret[..]);
}

#[test]
fn a_different_passphrase_gives_a_different_master_secret() {
    let v = &vectors()[0];
    let share = Share::parse(&v.mnemonics[0]).unwrap();
    let other = recover(&[share], b"").unwrap();
    assert_ne!(hex(other.expose().as_bytes()), v.master_secret);
}

#[test]
fn shares_of_two_backups_do_not_combine() {
    let secret = unhex("bb54aac4b89dc868ba37d9cc21b2cece");
    let groups = [GroupSpec {
        threshold: 2,
        count: 3,
    }];
    let mine = split_to_shares(&secret, 1, &groups);

    let mut stream = Stream(200);
    let mut theirs: Vec<Share> = (0..8).map(|_| Share::default()).collect();
    let n = slip39::split(
        &secret,
        PASSPHRASE,
        0x1234,
        false,
        0,
        1,
        &groups,
        &mut |buf| stream.fill(buf),
        &mut theirs,
    )
    .unwrap();
    theirs.truncate(n);

    let mixed = [mine[0].clone(), theirs[1].clone()];
    assert_eq!(
        recover(&mixed, PASSPHRASE).err(),
        Some(slip39::Error::BadDigest)
    );
}

#[test]
fn the_embedded_wordlist_is_the_published_file() {
    let mut rebuilt = String::new();
    for i in 0..1024u16 {
        rebuilt.push_str(slip39::word(i));
        rebuilt.push('\n');
    }
    assert_eq!(hex(&sha256(rebuilt.as_bytes())), slip39::wordlist_sha256());
}

#[test]
fn a_word_is_found_by_its_first_four_letters() {
    for i in 0..1024u16 {
        let w = slip39::word(i);
        assert_eq!(slip39::index_of(w), Some(i));
        assert_eq!(slip39::index_of(&w[..4]), Some(i));
        assert_eq!(slip39::candidates(w).collect::<Vec<_>>(), vec![i]);
    }
}
