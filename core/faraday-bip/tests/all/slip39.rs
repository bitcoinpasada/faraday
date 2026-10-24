//! SLIP-39 against the vectors Trezor publishes with
//! `python-shamir-mnemonic`, and against itself.
//!
//! `tools/vectors/slip39/vectors.json` holds 45 cases: a description, the
//! mnemonics, the master secret in hex (empty when the case must be
//! refused) and the BIP-32 root key the master secret gives. The
//! passphrase throughout is `TREZOR`.

use crate::common::{hex, unhex};
use faraday_bip::bitcoin::Network;
use faraday_bip::bitcoin::bip32::Xpriv;
use faraday_bip::slip39::{self, GroupSpec, Share, recover};
use faraday_crypto::sha256;
use faraday_crypto::work::Compute;
use serde_json::Value;

const VECTORS: &str = include_str!("../../../../tools/vectors/slip39/vectors.json");
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
    let c = Compute::unchecked();
    let mut shares = Vec::new();
    for m in mnemonics {
        shares.push(Share::parse(m)?);
    }
    let secret = recover(&c, &shares, PASSPHRASE)?;
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
    let c = Compute::unchecked();
    let v = &vectors()[0];
    let secret = combine(&v.mnemonics).unwrap();
    let as_bip39 =
        faraday_bip::bip39::Mnemonic::from_entropy(faraday_bip::bip39::Language::English, &secret)
            .unwrap()
            .to_seed(&c, PASSPHRASE)
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
    let c = Compute::unchecked();
    let mut stream = Stream(7);
    let mut out: Vec<Share> = (0..64).map(|_| Share::default()).collect();
    let n = slip39::split(
        &c,
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
    let c = Compute::unchecked();
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
                for k in b + 1..5 {
                    let chosen = [shares[a].clone(), shares[b].clone(), shares[k].clone()];
                    let got = recover(&c, &chosen, PASSPHRASE).unwrap();
                    assert_eq!(got.expose().as_bytes(), &secret[..]);
                    subsets += 1;
                }
            }
        }
        assert_eq!(subsets, 10);

        // Two of the five are not enough: the set is short a share.
        let two = [shares[0].clone(), shares[1].clone()];
        assert_eq!(
            recover(&c, &two, PASSPHRASE).err(),
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
    let c = Compute::unchecked();
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
    let got = recover(&c, &chosen, PASSPHRASE).unwrap();
    assert_eq!(got.expose().as_bytes(), &secret[..]);

    let mut chosen = by_group(1).into_iter().take(2).collect::<Vec<_>>();
    chosen.extend(by_group(2).into_iter().take(3));
    let got = recover(&c, &chosen, PASSPHRASE).unwrap();
    assert_eq!(got.expose().as_bytes(), &secret[..]);

    // One group is not enough when two must be present.
    assert_eq!(
        recover(&c, &by_group(0), PASSPHRASE).err(),
        Some(slip39::Error::WrongGroupCount {
            needed: 2,
            given: 1,
        })
    );
}

#[test]
fn an_extendable_split_recovers_the_same_secret() {
    let c = Compute::unchecked();
    let secret = unhex("bb54aac4b89dc868ba37d9cc21b2cece");
    let mut stream = Stream(19);
    let mut out: Vec<Share> = (0..8).map(|_| Share::default()).collect();
    let n = slip39::split(
        &c,
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
    let got = recover(&c, &out[..2], PASSPHRASE).unwrap();
    assert_eq!(got.expose().as_bytes(), &secret[..]);
}

#[test]
fn a_different_passphrase_gives_a_different_master_secret() {
    let c = Compute::unchecked();
    let v = &vectors()[0];
    let share = Share::parse(&v.mnemonics[0]).unwrap();
    let other = recover(&c, &[share], b"").unwrap();
    assert_ne!(hex(other.expose().as_bytes()), v.master_secret);
}

#[test]
fn shares_of_two_backups_do_not_combine() {
    let c = Compute::unchecked();
    let secret = unhex("bb54aac4b89dc868ba37d9cc21b2cece");
    let groups = [GroupSpec {
        threshold: 2,
        count: 3,
    }];
    let mine = split_to_shares(&secret, 1, &groups);

    let mut stream = Stream(200);
    let mut theirs: Vec<Share> = (0..8).map(|_| Share::default()).collect();
    let n = slip39::split(
        &c,
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
        recover(&c, &mixed, PASSPHRASE).err(),
        Some(slip39::Error::BadDigest)
    );
}

/// The groups of the cross-check's seed 17af375c…: sixteen groups, all
/// sixteen needed, two of them of sixteen members.
const SIXTEEN_GROUPS: [(u8, u8); 16] = [
    (11, 12),
    (14, 16),
    (2, 2),
    (2, 3),
    (2, 2),
    (5, 5),
    (2, 2),
    (2, 3),
    (3, 3),
    (2, 3),
    (2, 3),
    (3, 3),
    (5, 5),
    (2, 2),
    (3, 5),
    (9, 16),
];

/// Splits SLIP-39's largest master secret under the cross-check's case
/// (identifier 27924, extendable, exponent 0, no passphrase) with
/// `Stream(7)`. The shares, one per line, their SHA-256 and the random
/// bytes drawn; or the refusal.
fn split_at_the_limits(
    group_threshold: u8,
    groups: &[(u8, u8)],
) -> Result<(Vec<Share>, String, usize), slip39::Error> {
    let c = Compute::unchecked();
    let secret = unhex("b75d4d788d991ba911003a0379cf58d753171db734d8bca9b99c8a3eb2386bcc");
    let groups: Vec<GroupSpec> = groups
        .iter()
        .map(|&(threshold, count)| GroupSpec { threshold, count })
        .collect();
    let mut stream = Stream(7);
    let mut used = 0;
    let mut out: Vec<Share> = (0..300).map(|_| Share::default()).collect();
    let n = slip39::split(
        &c,
        &secret,
        b"",
        27924,
        true,
        0,
        group_threshold,
        &groups,
        &mut |buf| {
            used += buf.len();
            stream.fill(buf)
        },
        &mut out,
    )?;
    out.truncate(n);
    let lines = out
        .iter()
        .map(|s| {
            s.indices()
                .iter()
                .map(|&i| slip39::word(i))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let digest = hex(&sha256(lines.as_bytes()));
    // Every group's threshold of members, from the first `group_threshold`
    // groups, gives the secret back.
    let mut chosen = Vec::new();
    let mut at = 0;
    for (gi, g) in groups.iter().enumerate() {
        if gi < group_threshold as usize {
            chosen.extend(out[at..at + g.threshold as usize].iter().cloned());
        }
        at += g.count as usize;
    }
    let got = recover(&c, &chosen, b"").unwrap();
    assert_eq!(got.expose().as_bytes(), &secret[..]);
    Ok((out, digest, used))
}

/// SLIP-39's limits are 16 groups, all 16 needed, and 16 members a group.
/// At and below them the shares are python-shamir-mnemonic's, word for
/// word, from the same random bytes (`tools/reference/slip39`, the lines'
/// SHA-256); one past any of them is refused, as the reference refuses it.
#[test]
fn a_split_at_slip39s_limits_matches_the_reference() {
    // The group threshold, the groups, how many shares, the random bytes
    // drawn and the SHA-256 of the reference's shares.
    type Case<'a> = (u8, &'a [(u8, u8)], usize, usize, &'a str);
    let cases: [Case; 4] = [
        (
            16,
            &SIXTEEN_GROUPS,
            85,
            2108,
            "f831ee4fbe5b7df0277ea98e0e8c783eeb5fd2964bf5e281467d2b2399430eb8",
        ),
        (
            15,
            &SIXTEEN_GROUPS[..15],
            69,
            1824,
            "7a8af73b4eb318db469bd85c1dca2d0aa9ce4307cc5d36786927ae1ff99521d1",
        ),
        (
            15,
            &SIXTEEN_GROUPS,
            85,
            2076,
            "720037fa15dc02610b69172f33a9b32ed1bcc146acc220a762d74f5628cdcfb4",
        ),
        (
            2,
            &[(16, 16), (15, 16)],
            32,
            948,
            "f90a6ad7753c90557ec31990081aa5ec2414493c5d745384a409731faec2efab",
        ),
    ];
    for (gt, groups, count, drawn, want) in cases {
        let (shares, digest, used) = split_at_the_limits(gt, groups).unwrap();
        assert_eq!(shares.len(), count, "{gt} of {} groups", groups.len());
        assert_eq!(used, drawn, "{gt} of {} groups", groups.len());
        assert_eq!(digest, want, "{gt} of {} groups", groups.len());
    }

    let mut seventeen = SIXTEEN_GROUPS.to_vec();
    seventeen.push((2, 2));
    for (gt, groups) in [
        (16, seventeen),
        (17, SIXTEEN_GROUPS.to_vec()),
        (2, vec![(2, 17), (2, 2)]),
        (2, vec![(17, 16), (2, 2)]),
    ] {
        assert_eq!(
            split_at_the_limits(gt, &groups).err(),
            Some(slip39::Error::InvalidParameters),
            "{gt} of {groups:?}"
        );
    }
}

/// RS1024 as SLIP-39 states it, written here from the specification so
/// that a share can carry any iteration exponent the field holds.
fn rs1024_checksum(extendable: bool, data: &[u16]) -> [u16; 3] {
    const GEN: [u32; 10] = [
        0x00E0_E040,
        0x01C1_C080,
        0x0383_8100,
        0x0707_0200,
        0x0E0E_0009,
        0x1C0C_2412,
        0x3808_6C24,
        0x3090_FC48,
        0x21B1_F890,
        0x03F3_F120,
    ];
    let customization: &[u8] = if extendable {
        b"shamir_extendable"
    } else {
        b"shamir"
    };
    let values = customization
        .iter()
        .map(|&b| u32::from(b))
        .chain(data.iter().map(|&w| u32::from(w)))
        .chain([0, 0, 0]);
    let mut chk: u32 = 1;
    for v in values {
        let b = chk >> 20;
        chk = ((chk & 0x000F_FFFF) << 10) ^ v;
        for (i, g) in GEN.iter().enumerate() {
            if (b >> i) & 1 != 0 {
                chk ^= g;
            }
        }
    }
    chk ^= 1;
    [
        ((chk >> 20) & 1023) as u16,
        ((chk >> 10) & 1023) as u16,
        (chk & 1023) as u16,
    ]
}

/// `share` with its iteration exponent set to `exponent` and its
/// checksum written again: a well-formed share asking for that many
/// rounds.
fn with_exponent(share: &Share, exponent: u8) -> Share {
    let mut words = share.indices().to_vec();
    // The exponent is the low four bits of the second word.
    words[1] = (words[1] & !0xF) | u16::from(exponent);
    let n = words.len();
    let sum = rs1024_checksum(share.is_extendable(), &words[..n - 3]);
    words[n - 3..].copy_from_slice(&sum);
    Share::from_indices(&words).unwrap()
}

/// A one-of-one set at `exponent`, written by `split`.
fn single_share(secret: &[u8], exponent: u8) -> Result<Share, slip39::Error> {
    let c = Compute::unchecked();
    let mut stream = Stream(11);
    let mut out = vec![Share::default()];
    slip39::split(
        &c,
        secret,
        PASSPHRASE,
        0x0321,
        true,
        exponent,
        1,
        &[GroupSpec {
            threshold: 1,
            count: 1,
        }],
        &mut |buf| stream.fill(buf),
        &mut out,
    )?;
    Ok(out.remove(0))
}

/// A set at the highest exponent read opens.
#[test]
fn a_set_at_the_highest_iteration_exponent_read_opens() {
    let c = Compute::unchecked();
    let secret = unhex("bb54aac4b89dc868ba37d9cc21b2cece");
    let e = slip39::MAX_READ_ITERATION_EXPONENT;
    let share = single_share(&secret, e).unwrap();
    assert_eq!(share.iteration_exponent(), e);
    let got = recover(&c, &[share], PASSPHRASE).unwrap();
    assert_eq!(got.expose().as_bytes(), &secret[..]);
}

/// A set asking for one exponent more, or for the 327 million rounds
/// the field allows, is refused before any round runs, naming the
/// exponent.
#[test]
fn a_set_above_the_highest_iteration_exponent_read_is_refused() {
    let secret = unhex("bb54aac4b89dc868ba37d9cc21b2cece");
    let base = single_share(&secret, 1).unwrap();
    for exponent in [slip39::MAX_READ_ITERATION_EXPONENT + 1, 15] {
        let share = with_exponent(&base, exponent);
        assert_eq!(share.iteration_exponent(), exponent);
        let c = Compute::unchecked();
        assert_eq!(
            recover(&c, &[share], PASSPHRASE).err(),
            Some(slip39::Error::IterationExponentTooHigh { exponent })
        );
        let text = slip39::Error::IterationExponentTooHigh { exponent }.to_string();
        assert!(text.contains(&format!("iteration exponent {exponent}")));
    }
    // The forged share is otherwise sound: set back to 1 it opens.
    let c = Compute::unchecked();
    let again = with_exponent(&with_exponent(&base, 15), 1);
    let got = recover(&c, &[again], PASSPHRASE).unwrap();
    assert_eq!(got.expose().as_bytes(), &secret[..]);
}

/// Neither a split nor a bare decryption runs above the exponent read.
#[test]
fn nothing_is_computed_above_the_highest_iteration_exponent_read() {
    let secret = unhex("bb54aac4b89dc868ba37d9cc21b2cece");
    let exponent = slip39::MAX_READ_ITERATION_EXPONENT + 1;
    assert_eq!(
        single_share(&secret, exponent).err(),
        Some(slip39::Error::IterationExponentTooHigh { exponent })
    );
    let c = Compute::unchecked();
    assert_eq!(
        slip39::decrypt(&c, &secret, PASSPHRASE, 1, true, exponent).err(),
        Some(slip39::Error::IterationExponentTooHigh { exponent })
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
