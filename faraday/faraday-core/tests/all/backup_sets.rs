//! Seed XOR parts and SLIP-39 shares counted on a wallet's plan
//! (`docs/decisions/F276.md`), as a person reading the plan's check and
//! saving it would notice: a place that loses its part of an all-needed
//! set loses the key, a quorum survives a lost place, and the saved plan
//! reads back, never holding a part where it may not.

use faraday_core::plan::slip39::{Group, Slip39};
use faraday_core::plan::xor::Xor;
use faraday_core::plan::{self, Answers, At, Lost, Seed, Shape, Spot, Tag, What};
use faraday_core::testkit;
use faraday_text::Text;

fn shape() -> Shape {
    Shape {
        m: 1,
        paths: Vec::new(),
        keys: 1,
        seeds: vec![Seed {
            name: Text::of("Savings"),
            here: true,
            shares: false,
            passphrase: false,
        }],
        splits: false,
        vaults: 0,
    }
}

/// A plan whose three places keep exactly `held`, each its own list.
fn plan_of(set: impl FnOnce(&mut Answers, &Shape), held: [Vec<What>; 3]) -> Answers {
    let shape = shape();
    let mut a = Answers::defaults(&shape);
    a.places = 3;
    set(&mut a, &shape);
    a.set_map(
        held.into_iter()
            .enumerate()
            .map(|(p, ws)| Spot {
                at: At::Place(p),
                holds: ws.into_iter().map(|w| (w, Tag::Secret)).collect(),
            })
            .collect(),
    );
    a
}

fn xor3(a: &mut Answers, shape: &Shape) {
    a.set_xor(
        shape,
        0,
        Some(Xor {
            seed: 0,
            parts: vec![[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]],
        }),
    );
}

fn slip_2_of_3(a: &mut Answers, shape: &Shape) {
    a.set_slip39(
        shape,
        0,
        Some(Slip39 {
            seed: 0,
            identifier: 12345,
            exponent: 1,
            group_threshold: 1,
            groups: vec![Group {
                threshold: 2,
                count: 3,
                made: vec![0, 1, 2],
            }],
        }),
    );
}

fn lost_of(a: &Answers) -> Lost {
    plan::check(&testkit::compute(), &shape(), a).lost
}

#[test]
fn losing_one_place_of_three_seed_xor_parts_loses_the_key() {
    let a = plan_of(
        xor3,
        [
            vec![What::XorPart(0, 0)],
            vec![What::XorPart(0, 1)],
            vec![What::XorPart(0, 2)],
        ],
    );
    assert_eq!(lost_of(&a), Lost::No);
}

#[test]
fn all_the_seed_xor_parts_in_one_place_are_the_key_found() {
    let a = plan_of(
        xor3,
        [
            vec![
                What::XorPart(0, 0),
                What::XorPart(0, 1),
                What::XorPart(0, 2),
            ],
            vec![],
            vec![],
        ],
    );
    let check = plan::check(&testkit::compute(), &shape(), &a);
    assert_eq!(check.one_place, Some(0));
}

#[test]
fn a_two_of_three_slip39_group_survives_one_lost_place() {
    let a = plan_of(
        slip_2_of_3,
        [
            vec![What::Slip39Share(0, 0, 0)],
            vec![What::Slip39Share(0, 0, 1)],
            vec![What::Slip39Share(0, 0, 2)],
        ],
    );
    assert_eq!(lost_of(&a), Lost::Yes);
}

#[test]
fn two_slip39_shares_at_one_place_are_lost_with_it() {
    let a = plan_of(
        slip_2_of_3,
        [
            vec![What::Slip39Share(0, 0, 0), What::Slip39Share(0, 0, 1)],
            vec![What::Slip39Share(0, 0, 2)],
            vec![],
        ],
    );
    assert_eq!(lost_of(&a), Lost::No);
}

#[test]
fn a_saved_plan_with_both_sets_reads_back() {
    let a = plan_of(
        |a, s| {
            xor3(a, s);
            slip_2_of_3(a, s);
        },
        [
            vec![What::XorPart(0, 0), What::Slip39Share(0, 0, 0)],
            vec![What::XorPart(0, 1), What::Slip39Share(0, 0, 1)],
            vec![What::XorPart(0, 2)],
        ],
    );
    let text = a.to_text();
    let back = Answers::from_text(&shape(), &text).expect("the plan reads back");
    assert_eq!(back.to_text().as_str(), text.as_str());
    assert_eq!(back.xor(0), a.xor(0));
    assert_eq!(back.slip39(0), a.slip39(0));
    assert_eq!(lost_of(&back), Lost::No);
}

#[test]
fn a_part_of_a_set_the_plan_does_not_hold_is_refused() {
    let a = plan_of(
        xor3,
        [vec![What::XorPart(0, 0)], vec![What::XorPart(0, 1)], vec![]],
    );
    let text = a.to_text();
    let without: String = text
        .lines()
        .filter(|l| !l.starts_with("xor "))
        .map(|l| format!("{l}\n"))
        .collect();
    assert!(Answers::from_text(&shape(), &without).is_none());
}

#[test]
fn a_part_in_a_vault_is_refused() {
    let mut a = plan_of(
        xor3,
        [vec![What::XorPart(0, 0)], vec![What::XorPart(0, 1)], vec![]],
    );
    let mut m = plan::stored_map(&shape(), &a);
    m.push(Spot {
        at: At::Vault(0),
        holds: vec![(What::XorPart(0, 2), Tag::Sealed)],
    });
    a.set_map(m);
    assert!(Answers::from_text(&shape(), &a.to_text()).is_none());
}
