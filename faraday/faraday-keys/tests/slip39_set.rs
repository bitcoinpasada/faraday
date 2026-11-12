//! A SLIP-39 set typed into Convert a seed: a share made at a member index,
//! a new set of the same master secret, and what is held while it is typed.
//! Fixed sets are SLIP-39's published vectors; the new shares are read back
//! through the decoder that opens the published ones.
#![allow(clippy::disallowed_methods)]

use faraday_bip::keys::Network;
use faraday_bip::slip39::{Error, GroupSpec, Share};
use faraday_crypto::work::Compute;
use faraday_keys::slip39_set::{RotateGroup, SetRefused, rotate_count};
use faraday_keys::{Keys, Reveal};
use serde_json::Value;

const SLIP39: &str = include_str!("../../../tools/vectors/slip39/vectors.json");
const PASS: &[u8] = b"TREZOR";

/// A token for the tests' key work (`docs/COMPUTE.md` §3), counting
/// nothing: faraday-core's `testkit::compute` is out of this crate's
/// reach.
#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-keys' tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_| {})
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// A published vector (0-based): its shares and its master secret.
fn vector(n: usize) -> (Vec<String>, Vec<u8>) {
    let v: Value = serde_json::from_str(SLIP39).unwrap();
    let shares = v[n][1]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect();
    (shares, unhex(v[n][2].as_str().unwrap()))
}

fn holding(shares: &[String]) -> Keys {
    let mut keys = Keys::on(Network::Mainnet);
    for s in shares {
        keys.slip39_set_hold(s).unwrap();
    }
    keys
}

fn parse(s: &str) -> Share {
    Share::parse(s).unwrap()
}

fn seed_of(c: &Compute, shares: &[String]) -> Vec<u8> {
    let parts: Vec<Share> = shares.iter().map(|s| parse(s)).collect();
    faraday_keys::slip39_seed(c, &parts, PASS).unwrap().to_vec()
}

fn groups(shape: &[(u8, u8)]) -> Vec<GroupSpec> {
    shape
        .iter()
        .map(|&(threshold, count)| GroupSpec { threshold, count })
        .collect()
}

/// Randomness for a new set, its first two bytes the identifier.
fn random(secret_len: usize, gt: u8, g: &[GroupSpec], identifier: u16) -> Vec<u8> {
    let mut r: Vec<u8> = (0..faraday_keys::slip39_random_len(secret_len, gt, g))
        .map(|i| (i * 29 + 11) as u8)
        .collect();
    r[..2].copy_from_slice(&identifier.to_le_bytes());
    r
}

fn rotated(keys: &Keys, gt: u8, shape: &[(u8, u8)], identifier: u16) -> Vec<String> {
    let g = groups(shape);
    let r = random(16, gt, &g, identifier);
    keys.slip39_set_rotated(gt, &g, &r, Reveal::PaperForm)
        .unwrap()
        .release()
        .iter()
        .map(|l| l.as_str().to_string())
        .collect()
}

fn made(keys: &Keys, group: u8, at: u8) -> Result<String, SetRefused> {
    keys.slip39_set_share(group, at, Reveal::PaperForm)
        .map(|h| h.release().as_str().to_string())
}

/// A 2 of 3 set of a 16-byte seed made by Convert, with its shares.
fn converted(shape: &[(u8, u8)], gt: u8) -> (Keys, Vec<String>) {
    let c = compute();
    let mut source = Keys::on(Network::Mainnet);
    source.add_seed(&c, &[7u8; 16], "").unwrap();
    let g = groups(shape);
    let r = random(16, gt, &g, 0x1234);
    let lines = source
        .slip39_converted(&c, 0, gt, &g, &r, Reveal::PaperForm)
        .unwrap()
        .release();
    let shares: Vec<String> = lines.iter().map(|l| l.as_str().to_string()).collect();
    (source, shares)
}

#[test]
fn slip39_typing_a_published_set_says_what_is_held_and_adds_no_key() {
    let (shares, _) = vector(3);
    let mut keys = Keys::on(Network::Mainnet);
    let first = keys.slip39_set_hold(&shares[0]).unwrap().clone();
    assert!(!first.recoverable);
    assert_eq!(first.groups.len(), 1);
    assert!(!first.groups[0].quorum);
    let both = keys.slip39_set_hold(&shares[1]).unwrap().clone();
    assert!(both.recoverable);
    assert!(both.groups[0].quorum);
    assert_eq!(both.groups[0].members.len(), 2);
    assert!(!both.extendable);
    assert_eq!(keys.len(), 0);
    keys.slip39_set_forget();
    assert!(keys.slip39_set_facts().is_none());
}

#[test]
fn slip39_a_share_of_another_set_or_typed_twice_is_refused_and_what_was_held_stays() {
    let (a, _) = vector(3);
    let (b, _) = vector(42);
    let mut keys = holding(&a[..1]);
    assert!(matches!(
        keys.slip39_set_hold(&b[0]),
        Err(SetRefused::Share(_))
    ));
    assert!(matches!(
        keys.slip39_set_hold(&a[0]),
        Err(SetRefused::Share(_))
    ));
    assert!(matches!(
        keys.slip39_set_hold("not words at all"),
        Err(SetRefused::Share(_))
    ));
    let facts = keys.slip39_set_facts().unwrap();
    assert_eq!(facts.groups[0].members.len(), 1);
}

#[test]
fn slip39_a_quorum_of_one_group_of_several_adds_a_share_to_that_group() {
    let (shares, _) = vector(16);
    let all = holding(&shares);
    let facts = all.slip39_set_facts().unwrap().clone();
    assert!(facts.group_count > 1);
    let group = facts
        .groups
        .iter()
        .find(|g| g.quorum && g.threshold > 1)
        .expect("a group with a quorum of two or more");
    let own: Vec<String> = shares
        .iter()
        .filter(|s| parse(s).group_index() == group.index)
        .cloned()
        .collect();
    let keys = holding(&own);
    assert!(!keys.slip39_set_facts().unwrap().recoverable);
    let free = (0..16u8).find(|i| !group.members.contains(i)).unwrap();
    let line = made(&keys, group.index, free).unwrap();
    let share = parse(&line);
    assert_eq!(share.group_index(), group.index);
    assert_eq!(share.member_index(), free);
    assert_eq!(share.identifier(), facts.identifier);

    // The share made takes a place in the group's quorum: the whole set
    // with the new share for one of the typed ones opens the same wallet.
    let c = compute();
    let mut swapped = shares.clone();
    let at = swapped
        .iter()
        .position(|s| parse(s).group_index() == group.index)
        .unwrap();
    swapped[at] = line;
    let (_, secret) = vector(16);
    assert_eq!(seed_of(&c, &swapped), secret);
}

#[test]
fn slip39_replace_gives_the_very_share_that_was_lost() {
    let (_, shares) = converted(&[(2, 3)], 1);
    for lost in 0..3 {
        let kept: Vec<String> = shares
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != lost)
            .map(|(_, s)| s.clone())
            .collect();
        let keys = holding(&kept);
        let at = parse(&shares[lost]).member_index();
        assert_eq!(made(&keys, 0, at).unwrap(), shares[lost]);
    }
}

#[test]
fn slip39_a_set_of_groups_replaces_a_lost_share_from_its_own_group() {
    let (_, shares) = converted(&[(2, 3), (2, 3), (1, 1)], 2);
    // Group 0's shares are the first three; lose its second.
    let kept: Vec<String> = [0usize, 2, 3, 4, 6]
        .iter()
        .map(|&i| shares[i].clone())
        .collect();
    let keys = holding(&kept);
    assert!(keys.slip39_set_facts().unwrap().recoverable);
    let at = parse(&shares[1]).member_index();
    assert_eq!(made(&keys, 0, at).unwrap(), shares[1]);
}

#[test]
fn slip39_the_same_index_twice_gives_the_same_share_and_a_typed_index_is_refused() {
    let (shares, _) = vector(3);
    let keys = holding(&shares);
    let typed: Vec<u8> = shares.iter().map(|s| parse(s).member_index()).collect();
    let free = (0..16u8).find(|i| !typed.contains(i)).unwrap();
    let one = made(&keys, 0, free).unwrap();
    assert_eq!(one, made(&keys, 0, free).unwrap());
    assert!(!shares.contains(&one));
    assert!(matches!(
        made(&keys, 0, typed[0]),
        Err(SetRefused::Share(_))
    ));
}

#[test]
fn slip39_a_group_of_threshold_one_or_a_group_never_typed_makes_no_share() {
    let (_, shares) = converted(&[(1, 1), (2, 3)], 2);
    let keys = holding(&shares[..1]);
    assert_eq!(parse(&shares[0]).member_threshold(), 1);
    assert!(matches!(made(&keys, 0, 5), Err(SetRefused::Share(_))));
    assert!(matches!(made(&keys, 1, 5), Err(SetRefused::Share(_))));
}

#[test]
fn slip39_one_share_of_two_needed_makes_no_share() {
    let (shares, _) = vector(3);
    let keys = holding(&shares[..1]);
    assert!(matches!(made(&keys, 0, 7), Err(SetRefused::Share(_))));
}

#[test]
fn slip39_nothing_is_given_without_a_held_set_or_for_another_reason() {
    let (shares, _) = vector(3);
    let keys = holding(&[]);
    assert_eq!(made(&keys, 0, 7), Err(SetRefused::NotGiven));
    let g = groups(&[(2, 3)]);
    let r = random(16, 1, &g, 5);
    assert_eq!(
        keys.slip39_set_rotated(1, &g, &r, Reveal::PaperForm).err(),
        Some(SetRefused::NotGiven)
    );
    let keys = holding(&shares);
    assert_eq!(
        keys.slip39_set_share(0, 7, Reveal::Words).err(),
        Some(SetRefused::NotGiven)
    );
    assert_eq!(
        keys.slip39_set_rotated(1, &g, &r, Reveal::Words).err(),
        Some(SetRefused::NotGiven)
    );
}

#[test]
fn slip39_rotating_a_flag_clear_set_keeps_the_wallet_and_the_old_shares_do_not_mix_in() {
    let c = compute();
    let (old, secret) = vector(3);
    let keys = holding(&old);
    let new = rotated(&keys, 1, &[(2, 3)], 0x0abc);
    assert_eq!(new.len(), 3);
    assert_eq!(seed_of(&c, &old), secret);
    assert_eq!(seed_of(&c, &new[..2]), secret);
    assert_eq!(seed_of(&c, &new[1..]), secret);
    // The identifier is in the encryption's salt: it stays.
    assert_eq!(parse(&new[0]).identifier(), parse(&old[0]).identifier());
    assert!(!parse(&new[0]).is_extendable());
    let mixed = [parse(&old[0]), parse(&new[1])];
    assert!(faraday_keys::slip39_seed(&c, &mixed, PASS).is_err());
}

#[test]
fn slip39_rotating_an_extendable_set_gives_a_new_identifier_and_the_same_wallet() {
    let c = compute();
    let (old, secret) = vector(42);
    let keys = holding(&old);
    let old_id = keys.slip39_set_facts().unwrap().identifier;
    let new = rotated(&keys, 1, &[(2, 3)], 0x0abc);
    assert_ne!(old_id, 0x0abc);
    assert_eq!(parse(&new[0]).identifier(), 0x0abc);
    assert!(parse(&new[0]).is_extendable());
    assert_eq!(seed_of(&c, &old), secret);
    assert_eq!(seed_of(&c, &new[..2]), secret);
    assert_eq!(seed_of(&c, &new[1..]), secret);
    let mixed = [parse(&old[0]), parse(&new[1])];
    assert!(faraday_keys::slip39_seed(&c, &mixed, PASS).is_err());
}

#[test]
fn slip39_an_extendable_set_refuses_the_identifier_it_already_has() {
    let (old, _) = vector(42);
    let keys = holding(&old);
    let id = keys.slip39_set_facts().unwrap().identifier;
    let g = groups(&[(2, 3)]);
    let r = random(16, 1, &g, id);
    assert_eq!(
        keys.slip39_set_rotated(1, &g, &r, Reveal::PaperForm).err(),
        Some(SetRefused::SameIdentifier)
    );
}

#[test]
fn slip39_a_set_of_groups_rotates_into_another_shape_and_opens_the_same_wallet() {
    let c = compute();
    for n in [16, 17, 18] {
        let (old, secret) = vector(n);
        let keys = holding(&old);
        assert!(keys.slip39_set_facts().unwrap().recoverable, "vector {n}");
        let new = rotated(&keys, 2, &[(2, 3), (1, 1), (3, 5)], 0x2222);
        assert_eq!(new.len(), 9);
        // Groups 0 and 1: two of three, and the one.
        let pick = [&new[0], &new[1], &new[3]].map(|s| s.clone());
        assert_eq!(seed_of(&c, &pick), secret, "vector {n}");
        // Groups 1 and 2: the one, and three of five.
        let pick = [&new[3], &new[4], &new[5], &new[6]].map(|s| s.clone());
        assert_eq!(seed_of(&c, &pick), secret, "vector {n}");
    }
}

#[test]
fn slip39_a_rotation_needs_the_whole_set_quorum_not_one_groups() {
    let (shares, _) = vector(16);
    let facts = holding(&shares).slip39_set_facts().unwrap().clone();
    assert!(facts.group_threshold > 1);
    let first_group = facts.groups.iter().find(|g| g.quorum).unwrap().index;
    let own: Vec<String> = shares
        .iter()
        .filter(|s| parse(s).group_index() == first_group)
        .cloned()
        .collect();
    let keys = holding(&own);
    let g = groups(&[(2, 3)]);
    let r = random(16, 1, &g, 5);
    assert!(matches!(
        keys.slip39_set_rotated(1, &g, &r, Reveal::PaperForm).err(),
        Some(SetRefused::Share(Error::WrongGroupCount { .. }))
    ));
}

#[test]
fn slip39_a_rotation_short_of_randomness_or_of_a_valid_shape_gives_nothing() {
    let (old, _) = vector(3);
    let keys = holding(&old);
    let g = groups(&[(2, 3)]);
    let r = random(16, 1, &g, 5);
    assert_eq!(
        keys.slip39_set_rotated(1, &g, &r[..r.len() - 1], Reveal::PaperForm)
            .err(),
        Some(SetRefused::NoRandomness)
    );
    let bad = groups(&[(1, 3)]);
    let r = random(16, 1, &bad, 5);
    assert!(
        keys.slip39_set_rotated(1, &bad, &r, Reveal::PaperForm)
            .is_err()
    );
}

#[test]
fn slip39_a_forget_leaves_nothing_held_and_the_key_list_is_unchanged() {
    let c = compute();
    let mut keys = Keys::on(Network::Mainnet);
    keys.add_seed(&c, &[3u8; 16], "").unwrap();
    let (shares, _) = vector(3);
    for s in &shares {
        keys.slip39_set_hold(s).unwrap();
    }
    assert_eq!(keys.len(), 1);
    assert!(made(&keys, 0, 7).is_ok());
    keys.slip39_set_forget();
    assert!(keys.slip39_set_facts().is_none());
    assert_eq!(made(&keys, 0, 7), Err(SetRefused::NotGiven));
    assert_eq!(keys.len(), 1);
}

#[test]
fn slip39_practice_takes_no_share() {
    let mut keys = Keys::on(Network::Mainnet);
    keys.set_demo(&[], &[], &[]);
    let (shares, _) = vector(3);
    assert_eq!(
        keys.slip39_set_hold(&shares[0]).err(),
        Some(SetRefused::Practice)
    );
    assert!(keys.slip39_set_facts().is_none());
}

fn lost(threshold: Option<u8>, n: u8) -> RotateGroup {
    RotateGroup {
        threshold,
        lost_or_exposed: n,
    }
}

#[test]
fn slip39_the_rotate_count_counts_a_group_out_of_reach_at_its_threshold() {
    // One group of two needed: one lost is under, two reach it.
    let under = rotate_count(1, &[lost(Some(2), 1)]);
    assert_eq!(under.groups_out_of_reach, 0);
    assert!(!under.may_be_in_other_hands);
    let at = rotate_count(1, &[lost(Some(2), 2)]);
    assert_eq!(at.groups_out_of_reach, 1);
    assert!(at.may_be_in_other_hands);
    // Two groups needed: one group out is under the limit, two reach it.
    let groups = [lost(Some(2), 2), lost(Some(3), 2), lost(Some(1), 1)];
    let one = rotate_count(2, &groups[..2]);
    assert_eq!(one.groups_out_of_reach, 1);
    assert!(!one.may_be_in_other_hands);
    let two = rotate_count(2, &[groups[0], groups[2]]);
    assert_eq!(two.groups_out_of_reach, 2);
    assert!(two.may_be_in_other_hands);
    // Nothing lost anywhere.
    let none = rotate_count(1, &[lost(Some(1), 0), lost(None, 0)]);
    assert_eq!(none.groups_out_of_reach, 0);
}

#[test]
fn slip39_a_group_never_typed_counts_one_lost_share_as_reaching_its_threshold() {
    let c = rotate_count(2, &[lost(None, 1), lost(None, 1), lost(Some(2), 0)]);
    assert_eq!(c.groups_out_of_reach, 2);
    assert!(c.may_be_in_other_hands);
    let c = rotate_count(2, &[lost(None, 1), lost(Some(3), 1)]);
    assert_eq!(c.groups_out_of_reach, 1);
    assert!(!c.may_be_in_other_hands);
}

#[test]
fn slip39_a_third_member_of_a_group_of_two_or_a_member_threshold_that_differs_is_refused() {
    let (_, shares) = converted(&[(2, 3)], 1);
    let mut keys = holding(&shares[..2]);
    assert!(matches!(
        keys.slip39_set_hold(&shares[2]),
        Err(SetRefused::Share(Error::WrongMemberCount { .. }))
    ));
    assert_eq!(keys.slip39_set_facts().unwrap().groups[0].members.len(), 2);

    // The same identifier and group, but three of three needed.
    let (_, other) = converted(&[(3, 3)], 1);
    let mut keys = holding(&shares[..1]);
    assert!(matches!(
        keys.slip39_set_hold(&other[0]),
        Err(SetRefused::Share(Error::MismatchedMemberThresholds))
    ));
    assert_eq!(keys.slip39_set_facts().unwrap().groups[0].members.len(), 1);
}

#[test]
fn slip39_a_share_is_replaced_in_a_later_group_when_earlier_groups_have_met_their_quorums() {
    let (_, shares) = converted(&[(2, 3), (2, 3), (2, 3)], 2);
    // Every group's first two shares: all three quorums are met.
    let typed: Vec<String> = [0usize, 1, 3, 4, 6, 7]
        .iter()
        .map(|&i| shares[i].clone())
        .collect();
    let keys = holding(&typed);
    assert!(keys.slip39_set_facts().unwrap().recoverable);
    let at = parse(&shares[8]).member_index();
    assert_eq!(made(&keys, 2, at).unwrap(), shares[8]);
}

#[test]
fn slip39_a_stick_attached_holds_nothing_and_makes_nothing_until_it_is_gone() {
    let (shares, _) = vector(3);
    let mut keys = Keys::on(Network::Mainnet);
    keys.set_stick_attached(Some("stick"));
    assert_eq!(
        keys.slip39_set_hold(&shares[0]).err(),
        Some(SetRefused::Stick("stick"))
    );
    assert!(keys.slip39_set_facts().is_none());
    keys.set_stick_attached(None);
    for s in &shares {
        keys.slip39_set_hold(s).unwrap();
    }
    let typed: Vec<u8> = shares.iter().map(|s| parse(s).member_index()).collect();
    let free = (0..16u8).find(|i| !typed.contains(i)).unwrap();
    let g = groups(&[(2, 3)]);
    let r = random(16, 1, &g, 5);

    keys.set_stick_attached(Some("stick"));
    assert_eq!(made(&keys, 0, free), Err(SetRefused::Stick("stick")));
    assert_eq!(
        keys.slip39_set_rotated(1, &g, &r, Reveal::PaperForm).err(),
        Some(SetRefused::Stick("stick"))
    );
    assert!(keys.slip39_set_facts().is_some());

    keys.set_stick_attached(None);
    assert!(made(&keys, 0, free).is_ok());
    assert!(
        keys.slip39_set_rotated(1, &g, &r, Reveal::PaperForm)
            .is_ok()
    );
}

#[test]
fn slip39_a_rotation_gives_shares_that_every_member_recovers_with() {
    let c = compute();
    let (old, secret) = vector(3);
    let keys = holding(&old);
    // Two of five: every pair, the middle members among them.
    let new = rotated(&keys, 1, &[(2, 5)], 0x3333);
    for a in 0..5 {
        for b in a + 1..5 {
            let pick = [new[a].clone(), new[b].clone()];
            assert_eq!(seed_of(&c, &pick), secret, "members {a} and {b}");
        }
    }
    // Two of five groups, each two of three: a middle group with each end.
    let new = rotated(&keys, 2, &[(2, 3); 5], 0x4444);
    for other in [0usize, 4] {
        let mut pick: Vec<String> = new[6..8].to_vec();
        pick.extend_from_slice(&new[other * 3..other * 3 + 2]);
        assert_eq!(seed_of(&c, &pick), secret, "group 2 with group {other}");
    }
}

mod an_aezeed_and_a_set_held {
    use super::*;
    use faraday_bip::bip39::Language;
    use faraday_intake::secret_buf::SecretText;
    use faraday_keys::lightning::AEZEED_WORDS;

    const WORDS: &str = "above judge emerge veteran reform crunch system all snap please \
        shoulder vault hurt city quarter cover enlist swear success suggest drink wagon enrich body";

    fn hold_aezeed(keys: &mut Keys, words: &str) -> Result<(), String> {
        let c = compute();
        let mut indices = [0u16; AEZEED_WORDS];
        for (slot, w) in indices.iter_mut().zip(words.split_whitespace()) {
            *slot = Language::English.index_of(w).unwrap_or(0);
        }
        let read = keys.aezeed_seed_job(&indices, SecretText::of("")).run(&c);
        keys.aezeed_hold(read).map(|_| ())
    }

    #[test]
    fn slip39_holding_an_aezeed_drops_a_set_and_holding_a_set_drops_the_aezeed() {
        let (shares, _) = vector(3);
        let mut keys = holding(&shares[..1]);
        hold_aezeed(&mut keys, WORDS).unwrap();
        assert!(keys.slip39_set_facts().is_none());
        assert!(keys.aezeed_birthday().is_some());
        keys.slip39_set_hold(&shares[0]).unwrap();
        assert!(keys.aezeed_birthday().is_none());
        assert!(keys.slip39_set_facts().is_some());
    }

    #[test]
    fn slip39_only_a_success_drops_the_other_source() {
        let (shares, _) = vector(3);
        let mut keys = holding(&shares[..1]);
        // Words that are not an aezeed: nothing is held, the set stays.
        let bad = WORDS.replacen("above", "abandon", 1);
        assert!(hold_aezeed(&mut keys, &bad).is_err());
        assert!(keys.slip39_set_facts().is_some());

        let mut keys = Keys::on(Network::Mainnet);
        hold_aezeed(&mut keys, WORDS).unwrap();
        // A share that is refused leaves the aezeed.
        assert!(keys.slip39_set_hold("not words at all").is_err());
        assert!(keys.aezeed_birthday().is_some());
    }
}
