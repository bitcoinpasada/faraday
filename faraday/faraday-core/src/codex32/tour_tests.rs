//! The tour of the shares made and S (`docs/CODEX32.md` §5.7): the
//! translation worksheets filling live, and past the last checksum box
//! Next going through share D and S in the booklet's order, Previous
//! back, the choice off, and the Practice walk reaching the same steps.

use super::tour::TourStep;
use super::tour_screen::tour_text;
use super::walk::{Step, WalkAct};
use super::{C32, Card, Cell, Codex32Make, WAYS, Way};
use crate::testkit;
use faraday_keys::Reveal;
use faraday_ui::keyboard::OskPress;

/// Vector 2's initial shares A and C (`tools/vectors/bip93/vectors.json`).
const SHARES: [&str; 2] = [
    "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm",
    "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn",
];
const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// The entry open at 2 of 3, identifier `name`, by `way`.
fn entry(way: Way) -> Codex32Make {
    let c = testkit::compute();
    let mut m = Codex32Make::with_scheme(2, 3, Vec::new());
    for ch in "name".chars() {
        m.act(&c, C32::Key(OskPress::Char(ch)), None);
    }
    m.act(&c, C32::Next, None);
    let at = WAYS.iter().position(|w| *w == way).unwrap();
    m.act(&c, C32::Way(at as u8), None);
    m.act(&c, C32::Next, None);
    assert_eq!(m.open(), Some(Card::Enter));
    m
}

/// The data characters of vector 2's A then C, from `from` on.
fn data() -> Vec<char> {
    SHARES
        .iter()
        .flat_map(|s| s[9..35].chars().collect::<Vec<_>>())
        .collect()
}

fn type_char(m: &mut Codex32Make, ch: char) {
    let c = testkit::compute();
    m.act(&c, C32::Key(OskPress::Char(ch)), None);
}

/// The ten six-faced faces that roll `ch`: a die's bit 1 as 1 then 2, 0
/// as 2 then 1.
fn faces_of(ch: char) -> Vec<u8> {
    let v = CHARSET.iter().position(|&x| char::from(x) == ch).unwrap() as u8;
    let bits: Vec<u8> = (0..5).map(|d| v >> (4 - d) & 1).collect();
    let mut out: Vec<u8> = bits.iter().map(|&b| if b == 1 { 1 } else { 2 }).collect();
    out.extend(bits.iter().map(|&b| if b == 1 { 2 } else { 1 }));
    out
}

fn step(m: &mut Codex32Make, d: i8) {
    let c = testkit::compute();
    m.act(&c, C32::Step(d), None);
}

/// The tracker stepped to its last step: the last checksum box.
fn to_last_checksum(m: &mut Codex32Make) {
    for _ in 0..200 {
        if m.track_at_end() {
            break;
        }
        step(m, 1);
    }
    assert!(m.track_at_end());
    assert!(matches!(m.track_shown(), Some(Step::Below { b: 48 })));
}

/// What the tour's step shown reads.
fn shown(m: &Codex32Make) -> String {
    let (_, s) = m.tour_shown().expect("a tour step is shown");
    tour_text(m, m.work().unwrap(), s)
}

#[test]
fn a_derived_shares_box_fills_on_its_tab_as_soon_as_its_inputs_exist() {
    let mut m = entry(Way::Typed);
    let d = |m: &Codex32Make| {
        let ts = m
            .work()
            .unwrap()
            .live_translation(0, Reveal::PaperForm)
            .unwrap();
        (
            ts.translated_cell(0, 10),
            ts.translated_cell(1, 10),
            ts.sum_cell(10),
        )
    };
    assert_eq!(d(&m), (None, None, None));
    type_char(&mut m, '3');
    let (a, c, sum) = d(&m);
    assert!(a.is_some() && c.is_none() && sum.is_none());
    for ch in &data()[1..27] {
        type_char(&mut m, *ch);
    }
    // C's box 10 is in: its translation and the sum, share D's box 10.
    let (_, c, sum) = d(&m);
    assert!(c.is_some() && sum.is_some());
}

/// The whole of A and C typed, the tracker at the last checksum box.
fn made() -> Codex32Make {
    let mut m = entry(Way::Typed);
    for ch in data() {
        type_char(&mut m, ch);
    }
    assert!(m.strings().is_some());
    to_last_checksum(&mut m);
    m
}

#[test]
fn after_the_last_checksum_box_next_goes_through_d_then_s_in_order() {
    let mut m = made();
    assert_eq!(m.view(), 0);
    step(&mut m, 1);
    // Share D's tab, already filled; its first step, A's symbol.
    assert_eq!(m.view(), 1);
    let ts = m
        .work()
        .unwrap()
        .live_translation(0, Reveal::PaperForm)
        .unwrap();
    assert!((4..=48).all(|b| ts.sum_cell(b).is_some()));
    assert_eq!(m.tour_shown(), Some((1, TourStep::Symbol { t: 0, i: 0 })));
    assert!(
        shown(&m).starts_with("Share A's translation symbol for D: "),
        "{}",
        shown(&m)
    );
    assert_eq!(
        m.tour_cells(0),
        vec![(Cell::Symbol { sheet: 0, i: 0 }, true)]
    );
    let mut seen = vec![m.tour_shown().unwrap().1];
    let mut texts = vec![shown(&m)];
    while m.tour_shown().is_some() {
        let (n, s) = m.tour_shown().unwrap();
        if n == m.tour().steps().len() {
            break;
        }
        step(&mut m, 1);
        let (_, s2) = m.tour_shown().unwrap();
        assert_eq!(m.view() as usize, s2.sheet() + 1);
        // Each step marks its cells, the one worked out among them.
        let marks = if s2.on_check() {
            m.tour_check_cells(s2.sheet())
        } else {
            m.tour_cells(s2.sheet())
        };
        assert!(marks.iter().any(|c| c.1), "{s2:?} {marks:?}");
        assert_ne!(s, s2);
        seen.push(s2);
        texts.push(shown(&m));
    }
    // D: two symbols, A's row and C's row box by box, the sum box by
    // box, the check, its end; then S the same, without a check.
    let at = |want: TourStep| seen.iter().position(|&s| s == want).unwrap();
    assert!(at(TourStep::Symbol { t: 0, i: 1 }) < at(TourStep::Translate { t: 0, i: 0, b: 4 }));
    assert!(
        at(TourStep::Translate { t: 0, i: 0, b: 48 })
            < at(TourStep::Translate { t: 0, i: 1, b: 4 })
    );
    assert!(at(TourStep::Translate { t: 0, i: 1, b: 48 }) < at(TourStep::Sum { t: 0, b: 4 }));
    assert!(at(TourStep::Sum { t: 0, b: 48 }) < at(TourStep::Holds { t: 0 }));
    assert!(at(TourStep::Holds { t: 0 }) < at(TourStep::Symbol { t: 1, i: 0 }));
    assert!(!seen.contains(&TourStep::Holds { t: 1 }));
    assert_eq!(seen.last(), Some(&TourStep::Sum { t: 1, b: 48 }));
    // A translated cell, a sum, D's check holding, S's recovery symbol.
    let text = |s: TourStep| texts[at(s)].clone();
    let tr = text(TourStep::Translate { t: 0, i: 0, b: 10 });
    assert!(tr.starts_with("Share A, box 10: 3 × "), "{tr}");
    assert!(tr.contains("Translation wheel set to"), "{tr}");
    // D's box 10 is the string made's box 10.
    let d = m.strings().unwrap()[2].clone();
    let d10 = d.as_bytes()[9].to_ascii_uppercase() as char;
    let sum = text(TourStep::Sum { t: 0, b: 10 });
    assert!(
        sum.starts_with("Box 10, Share D: ") && sum.ends_with(&format!(" = {d10}")),
        "{sum}"
    );
    assert_eq!(
        text(TourStep::Holds { t: 0 }),
        "Share D's final row reads SECRETSHARE32: its checksum holds."
    );
    let s = text(TourStep::Symbol { t: 1, i: 0 });
    assert!(s.starts_with("Share A's recovery symbol: "), "{s}");
    // At the last step, Next goes on to the worksheets.
    step(&mut m, 1);
    assert_eq!(m.open(), Some(Card::Sheets));
}

#[test]
fn previous_goes_back_through_the_tour_and_out_of_it() {
    let mut m = made();
    step(&mut m, 1);
    step(&mut m, 1);
    step(&mut m, 1);
    assert_eq!(m.tour_shown().map(|s| s.0), Some(3));
    step(&mut m, -1);
    step(&mut m, -1);
    assert_eq!(m.tour_shown().map(|s| s.0), Some(1));
    // From the first, back to the last checksum box, on the Checksum tab.
    step(&mut m, -1);
    assert_eq!(m.tour_shown(), None);
    assert_eq!(m.view(), 0);
    assert!(matches!(m.track_shown(), Some(Step::Below { b: 48 })));
    // A character taken back unmakes the set: no tour.
    let c = testkit::compute();
    m.act(&c, C32::Key(OskPress::Back), None);
    assert!(m.tour().steps().is_empty());
}

#[test]
fn with_the_choice_off_there_is_no_tour() {
    let mut m = made();
    assert!(m.tour().on());
    let c = testkit::compute();
    m.act(&c, C32::Tour, None);
    assert!(!m.tour().on());
    step(&mut m, 1);
    assert_eq!(m.tour_shown(), None);
    assert_eq!(m.view(), 0);
    // The tabs are still filled.
    let ts = m
        .work()
        .unwrap()
        .live_translation(1, Reveal::PaperForm)
        .unwrap();
    assert!((4..=48).all(|b| ts.sum_cell(b).is_some()));
    // Turned off during the tour, it closes back to the checksum.
    m.act(&c, C32::Tour, None);
    step(&mut m, 1);
    assert!(m.tour_shown().is_some());
    m.act(&c, C32::Tour, None);
    assert_eq!(m.tour_shown(), None);
    assert_eq!(m.view(), 0);
}

#[test]
fn the_demo_walk_reaches_the_same_steps() {
    let c = testkit::compute();
    let mut m = entry(Way::Dice);
    let all = data();
    let (last, rest) = all.split_last().unwrap();
    for ch in rest {
        for f in faces_of(*ch) {
            type_char(&mut m, char::from(b'0' + f));
        }
    }
    // The computer rolls the last character.
    let mut faces = faces_of(*last);
    let mut draw = move || {
        let mut out = [0u8; 32];
        for (i, slot) in out.iter_mut().rev().enumerate() {
            *slot = if faces.is_empty() {
                (i % 2) as u8
            } else {
                faces.remove(0) - 1
            };
        }
        Some(out)
    };
    for _ in 0..400 {
        if m.tour_shown().is_some() {
            break;
        }
        m.walk_act(&c, WalkAct::Next, 0, &mut draw);
    }
    assert!(m.strings().is_some());
    assert_eq!(m.tour_shown(), Some((1, TourStep::Symbol { t: 0, i: 0 })));
    assert_eq!(m.view(), 1);
    // The same steps as the person's own entry reaches.
    assert_eq!(m.tour().steps(), made().tour().steps());
    m.walk_act(&c, WalkAct::Next, 0, &mut draw);
    assert_eq!(m.tour_shown(), Some((2, TourStep::Symbol { t: 0, i: 1 })));
    // Previous back out of it, onto the walk's last checksum box.
    m.walk_act(&c, WalkAct::Previous, 0, &mut draw);
    m.walk_act(&c, WalkAct::Previous, 0, &mut draw);
    assert_eq!(m.tour_shown(), None);
    assert!(matches!(
        m.walk().shown(),
        Some((_, _, Step::Below { b: 48 }))
    ));
    // Play to the end runs through the tour and stops at its last step.
    m.walk_act(
        &c,
        WalkAct::Reach(crate::fresh::roller::five::Reach::End),
        0,
        &mut draw,
    );
    m.walk_act(&c, WalkAct::Speed(3), 0, &mut draw);
    m.walk_act(&c, WalkAct::Play, 0, &mut draw);
    for now in 1..2000 {
        if !m.walk().playing() {
            break;
        }
        m.walk_tick(&c, now, &mut draw);
    }
    let len = m.tour().steps().len();
    assert_eq!(m.tour_shown().map(|s| s.0), Some(len));
    assert_eq!(m.open(), Some(Card::Enter));
}
