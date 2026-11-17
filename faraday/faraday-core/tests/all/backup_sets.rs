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

use faraday_bip::keys::ScriptType;
use faraday_core::board::BoardPress;
use faraday_core::{Action, Faraday, StorageEvent, qstep};
use faraday_keys::texts::Spec;
use faraday_shell_api::{App, Event, Key};
use faraday_vault::{slip39 as v39, xor as vxor};

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
        seed_alone_fails: false,
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

/// One group of 2 of 3 of which only share 1 was made: a plan that places
/// the other two as well does not make the key rebuildable, and reading it
/// back refuses the places of shares that do not exist.
#[test]
fn a_slip39_share_that_was_never_made_is_not_counted_or_read_back() {
    let only_first = |a: &mut Answers, shape: &Shape| {
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
                    made: vec![0],
                }],
            }),
        );
    };
    let a = plan_of(
        only_first,
        [
            vec![What::Slip39Share(0, 0, 0)],
            vec![What::Slip39Share(0, 0, 1)],
            vec![What::Slip39Share(0, 0, 2)],
        ],
    );
    assert_eq!(lost_of(&a), Lost::No);
    assert!(Answers::from_text(&shape(), &a.to_text()).is_none());
}

/// A set whose shares were all placed is replaced by one that made only
/// the first: the places of the shares never made leave the plan.
#[test]
fn replacing_a_slip39_set_drops_the_places_of_shares_never_made() {
    let mut a = plan_of(
        slip_2_of_3,
        [
            vec![What::Slip39Share(0, 0, 0)],
            vec![What::Slip39Share(0, 0, 1)],
            vec![What::Slip39Share(0, 0, 2)],
        ],
    );
    a.set_slip39(
        &shape(),
        0,
        Some(Slip39 {
            seed: 0,
            identifier: 12345,
            exponent: 1,
            group_threshold: 1,
            groups: vec![Group {
                threshold: 2,
                count: 3,
                made: vec![0],
            }],
        }),
    );
    let held: Vec<What> = plan::stored_map(&shape(), &a)
        .into_iter()
        .flat_map(|s| s.holds.into_iter().map(|(w, _)| w))
        .collect();
    assert_eq!(held, vec![What::Slip39Share(0, 0, 0)]);
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

// ---- a key's recorded sets on a wallet's plan (F276) ----

/// A key of words typed in, a single-key wallet over it loaded, and Back
/// up opened on that wallet after the key was given the sets.
fn backup_of_split_key(xor: Option<vxor::XorFacts>, slip39: Option<v39::Slip39Facts>) -> Faraday {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    testkit::keep_published(&mut app);
    assert!(xor.is_none() || app.session.keys.set_xor(0, xor));
    assert!(slip39.is_none() || app.session.keys.set_slip39(0, slip39));
    let c = testkit::compute();
    let text = app
        .session
        .keys
        .key_text(&c, 0, &Spec::Single(ScriptType::NativeSegwit, 0))
        .unwrap()
        .unwrap();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            Text::of("split-wallet.txt"),
            format!("wpkh({}/<0;1>/*)", &*text).into_bytes(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    testkit::load_wallet(&mut app, 0);
    app.press(Action::Backup(0));
    app
}

fn three_parts() -> vxor::XorFacts {
    vxor::XorFacts {
        parts: vec![[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]],
    }
}

/// The things the open backup's plan keeps at each place.
fn kept_at_places(app: &Faraday) -> Vec<Vec<What>> {
    let a = app.backup().unwrap().answers.clone();
    let shape = app.plan_shape_now(0);
    plan::stored_map(&shape, &a)
        .into_iter()
        .filter(|s| matches!(s.at, At::Place(_)))
        .map(|s| s.holds.into_iter().map(|(w, _)| w).collect())
        .collect()
}

#[test]
fn a_seed_xor_set_saved_for_a_key_is_on_its_wallets_plan_one_part_to_a_place() {
    let app = backup_of_split_key(Some(three_parts()), None);
    let places = kept_at_places(&app);
    for j in 0..3 {
        let at: Vec<usize> = places
            .iter()
            .enumerate()
            .filter(|(_, ws)| ws.contains(&What::XorPart(0, j)))
            .map(|(p, _)| p)
            .collect();
        assert_eq!(at, vec![j], "part {} sits at its own place", j + 1);
    }
}

#[test]
fn a_slip39_set_saved_for_a_key_is_on_its_wallets_plan_with_only_the_shares_made() {
    let set = v39::Slip39Facts {
        identifier: 77,
        exponent: 1,
        threshold: 1,
        groups: vec![v39::Group {
            threshold: 2,
            count: 3,
            made: 0b011,
        }],
    };
    let app = backup_of_split_key(None, Some(set));
    let all: Vec<What> = kept_at_places(&app).into_iter().flatten().collect();
    assert!(all.contains(&What::Slip39Share(0, 0, 0)));
    assert!(all.contains(&What::Slip39Share(0, 0, 1)));
    assert!(!all.contains(&What::Slip39Share(0, 0, 2)));
}

#[test]
fn a_set_the_plan_has_stays_when_the_key_keeps_another_of_that_kind() {
    let shape = shape();
    let mut a = Answers::defaults(&shape);
    xor3(&mut a, &shape);
    plan::sets::sync(
        &mut a,
        &shape,
        &[Some(vxor::XorFacts {
            parts: vec![[9, 9, 9, 9], [8, 8, 8, 8]],
        })],
        &[],
    );
    assert_eq!(a.xor(0).map(|x| x.n()), Some(3));
}

#[test]
fn the_board_lists_each_part_of_a_keys_seed_xor_set_as_not_checked() {
    let mut app = backup_of_split_key(Some(three_parts()), None);
    app.press(Action::BQ(qstep::BOARD));
    app.press(Action::Board(BoardPress::Customize));
    app.press(Action::Board(BoardPress::OpenList));
    app.settle();
    let mut seen = false;
    let mut all: Vec<String> = Vec::new();
    for _ in 0..30 {
        let _ = app.frame();
        let t = crate::common::texts(&mut app);
        all.extend(t.iter().cloned());
        if t.iter().any(|s| s == "Part 3") {
            seen = t.iter().any(|s| s.contains("Not checked"));
            break;
        }
        // The page and the list column beside the cards scroll on their own.
        for x in [600, 1326] {
            app.event(Event::Scroll { x, y: 400, dy: 150 });
        }
    }
    all.dedup();
    assert!(seen, "Part 3 and Not checked are on the board: {all:?}");
}

#[test]
fn back_ups_own_seed_xor_split_is_on_the_plan_one_part_to_a_place() {
    let mut app = crate::paper::backing_up();
    app.press(Action::BXor(3));
    assert!(app.backup().unwrap().paper.is_some());
    let places = kept_at_places(&app);
    for j in 0..3 {
        let at: Vec<usize> = places
            .iter()
            .enumerate()
            .filter(|(_, ws)| ws.contains(&What::XorPart(0, j)))
            .map(|(p, _)| p)
            .collect();
        assert_eq!(at, vec![j], "part {} is a chip at its own place", j + 1);
    }
}

#[test]
fn a_second_split_of_a_key_with_a_set_is_refused() {
    let mut app = crate::paper::backing_up();
    app.press(Action::BXor(2));
    assert!(app.backup().unwrap().paper.is_some());
    app.press(Action::BPaperHide);
    app.press(Action::BPaperHide);
    app.press(Action::BXor(3));
    assert!(app.backup().unwrap().paper.is_none());
    assert_eq!(
        app.toast_text(),
        Some(faraday_strings::EN.paper_section_has_set)
    );
    let xors = app.backup().unwrap().answers.xor.len();
    assert_eq!(xors, 1);
}
