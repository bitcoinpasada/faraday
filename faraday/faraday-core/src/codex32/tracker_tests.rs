//! The step tracker of the person's own entry (`docs/CODEX32.md` §5.7):
//! what each value entered on the pad reads, a character completed, the
//! worksheet's steps with Previous and Next, and a new value moving the
//! tracker, on the walk's own model.

use super::walk::Step;
use super::walk_screen::step_text;
use super::{C32, Cell, Codex32Make};
use crate::testkit;
use faraday_keys::Reveal;
use faraday_ui::keyboard::OskPress;

/// The entry open on six-faced dice at 2 of 3, identifier `name`: BIP
/// 93's vector 2's scheme.
fn entry() -> Codex32Make {
    let c = testkit::compute();
    let mut m = Codex32Make::with_scheme(2, 3, Vec::new());
    for ch in "name".chars() {
        m.act(&c, C32::Key(OskPress::Char(ch)), None);
    }
    m.act(&c, C32::Next, None);
    m.act(&c, C32::Next, None);
    m
}

/// A value of the person's own dice, typed on the pad.
fn face(m: &mut Codex32Make, f: u8) {
    let c = testkit::compute();
    m.act(&c, C32::Key(OskPress::Char(char::from(b'0' + f))), None);
}

fn step(m: &mut Codex32Make, d: i8) {
    let c = testkit::compute();
    m.act(&c, C32::Step(d), None);
}

/// What the step shown reads.
fn shown(m: &Codex32Make) -> String {
    let (ch, _, step) = m.shown_step().expect("a step is shown");
    step_text(m.work().unwrap(), ch, step)
}

/// Vector 2's share A begins `3`, value 17, bits 1 0 0 0 1.
const THREE: [u8; 10] = [1, 2, 2, 2, 1, 2, 1, 1, 1, 2];

#[test]
fn each_value_entered_reads_its_step() {
    let mut m = entry();
    let mut texts = Vec::new();
    for (k, &f) in THREE[..9].iter().enumerate() {
        face(&mut m, f);
        texts.push(shown(&m));
        // The die's pad is marked on its track.
        let (die, pad) = m.walk_die_mark().unwrap();
        assert_eq!((die, pad), (k % 5, f));
        // Nothing to step on to: the next value is the person's.
        assert!(m.track_at_end());
    }
    assert_eq!(
        texts,
        [
            "Die 1, first roll: 1. Marker on pad 1.",
            "Die 2, first roll: 2. Marker on pad 2.",
            "Die 3, first roll: 2. Marker on pad 2.",
            "Die 4, first roll: 2. Marker on pad 2.",
            "Die 5, first roll: 1. Marker on pad 1.",
            "Die 1, second roll: 2. 2 is higher than 1, so bit 1.",
            "Die 2, second roll: 1. 1 is lower than 2, so bit 0.",
            "Die 3, second roll: 1. 1 is lower than 2, so bit 0.",
            "Die 4, second roll: 1. 1 is lower than 2, so bit 0.",
        ]
    );
    assert_eq!(m.next_roll(), Some((4, true)));
}

#[test]
fn an_equal_pair_reads_roll_both_again() {
    let mut m = entry();
    for f in [3, 4, 5, 6, 1, 3] {
        face(&mut m, f);
    }
    assert_eq!(shown(&m), "Die 1: 3 then 3. Equal, so roll both again.");
    // Die 1 is rolled again: its first roll is next.
    assert_eq!(m.next_roll(), Some((0, false)));
    face(&mut m, 6);
    assert_eq!(shown(&m), "Die 1, first roll: 6. Marker on pad 6.");
}

#[test]
fn a_completed_character_shows_its_leaf_and_its_box_marked() {
    let mut m = entry();
    for f in THREE {
        face(&mut m, f);
    }
    assert_eq!(
        shown(&m),
        "Bits 1 0 0 0 1 = 16 + 1 = 17. Leaf 17 of the tree is 3. 3 goes in box 10."
    );
    assert_eq!(
        m.walk_cells(0),
        vec![(Cell::Given { sheet: 0, b: 10 }, true)]
    );
    // The dice sheet shows the character whole, its pairs with it.
    assert!(m.shown_dice().character().is_ok());
    assert_eq!(m.shown_rows().len(), 5);
    // It went in its box as entered.
    let sh = m.work().unwrap().worksheet(0, Reveal::PaperForm).unwrap();
    assert_eq!(sh.symbol(10), super::symbol_of('3'));
}

#[test]
fn next_walks_the_worksheet_steps_in_order_and_previous_goes_back() {
    let mut m = entry();
    for f in THREE {
        face(&mut m, f);
    }
    let before = m.entry_pos();
    let mut seen = vec![(shown(&m), m.walk_cells(0))];
    while !m.track_at_end() {
        step(&mut m, 1);
        let cells = m.walk_cells(0);
        // Each worksheet step marks its inputs and its result, the one
        // worked out last.
        assert!(cells.len() >= 2, "{cells:?}");
        assert!(cells.iter().any(|c| c.1), "{cells:?}");
        seen.push((shown(&m), cells));
    }
    // Box 10's four sum rows, worked top down; the entry never moved.
    let texts: Vec<&str> = seen.iter().map(|s| s.0.as_str()).collect();
    assert_eq!(texts.len(), 5, "{texts:#?}");
    assert_eq!(texts[1], "Box 10, row a: 3 + R = J");
    for (k, t) in texts[2..].iter().enumerate() {
        assert!(
            t.starts_with(&format!("Box 10, row {}: ", ["c", "e", "g"][k])),
            "{t}"
        );
    }
    assert!(matches!(
        seen[1].1.last(),
        Some((Cell::Sum { j: 0, .. }, true))
    ));
    assert_eq!(m.entry_pos(), before);
    // Previous, back over each step to the character's own.
    for want in seen.iter().rev().skip(1) {
        step(&mut m, -1);
        assert_eq!((shown(&m), m.walk_cells(0)), want.clone());
    }
    // And back into its values, shown on its own whole sheet.
    step(&mut m, -1);
    assert_eq!(
        shown(&m),
        "Die 5, second roll: 2. 2 is higher than 1, so bit 1."
    );
    assert!(m.shown_dice().character().is_ok());
    assert_eq!(m.walk_die_mark(), Some((4, 2)));
    assert_eq!(m.entry_pos(), before);
}

#[test]
fn a_new_value_moves_the_tracker_to_that_roll() {
    let mut m = entry();
    for f in THREE {
        face(&mut m, f);
    }
    step(&mut m, 1);
    assert!(matches!(m.track_shown(), Some(Step::Sum { .. })));
    face(&mut m, 4);
    assert_eq!(shown(&m), "Die 1, first roll: 4. Marker on pad 4.");
    // Previous reaches the character before, to its last worksheet step.
    step(&mut m, -1);
    assert!(shown(&m).starts_with("Box 10, row g: "), "{}", shown(&m));
    assert_eq!(m.entry_pos(), (1, 1));
    // Backspace takes the value back: the tracker shows the character
    // before, at its own step.
    let c = testkit::compute();
    m.act(&c, C32::Key(OskPress::Back), None);
    assert_eq!(m.entry_pos(), (1, 0));
    assert!(matches!(m.track_shown(), Some(Step::Char)));
    // Backspace again takes the character out of its box: its values
    // back but the last, the tracker on the last of them.
    m.act(&c, C32::Key(OskPress::Back), None);
    assert_eq!(m.entry_pos(), (0, 9));
    assert_eq!(
        shown(&m),
        "Die 4, second roll: 1. 1 is lower than 2, so bit 0."
    );
}

#[test]
fn a_typed_character_steps_through_its_cells() {
    let c = testkit::compute();
    let mut m = Codex32Make::with_scheme(2, 3, Vec::new());
    for ch in "name".chars() {
        m.act(&c, C32::Key(OskPress::Char(ch)), None);
    }
    m.act(&c, C32::Next, None);
    let typed = super::WAYS
        .iter()
        .position(|w| *w == super::Way::Typed)
        .unwrap();
    m.act(&c, C32::Way(typed as u8), None);
    m.act(&c, C32::Next, None);
    m.act(&c, C32::Key(OskPress::Char('3')), None);
    assert_eq!(shown(&m), "3 goes in box 10.");
    assert_eq!(
        m.walk_cells(0),
        vec![(Cell::Given { sheet: 0, b: 10 }, true)]
    );
    step(&mut m, 1);
    assert_eq!(shown(&m), "Box 10, row a: 3 + R = J");
    step(&mut m, 1);
    assert!(shown(&m).starts_with("Box 10, row c: "));
}
