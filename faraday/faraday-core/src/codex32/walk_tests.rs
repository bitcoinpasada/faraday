//! The step-through of the "Demo only" card (`docs/CODEX32.md` §5.5)
//! against the flow's own entry, with this computer's randomness given
//! as known faces: what each step reads, Previous, and Play.

use super::walk::{Step, WalkAct};
use super::walk_screen::step_text;
use super::{C32, Codex32Make};
use crate::fresh::roller::five::Reach;
use crate::testkit;
use faraday_keys::Reveal;
use faraday_ui::keyboard::OskPress;

/// The entry open on six-faced dice at 2 of 3, identifier `name`: BIP 93's
/// vector 2's scheme.
fn entry() -> Codex32Make {
    let c = testkit::compute();
    let mut m = Codex32Make::with_scheme(2, 3, Vec::new());
    for ch in "name".chars() {
        m.act(&c, C32::Key(OskPress::Char(ch)), None);
    }
    m.act(&c, C32::Next, None);
    m.act(&c, C32::Next, None);
    assert!(m.walk_open(), "the entry is open on dice");
    m
}

/// Answers of this computer's randomness that give `faces` (six-faced
/// dice), then ones and twos, and count how many were asked for.
fn answers(
    faces: &[u8],
) -> (
    impl FnMut() -> Option<[u8; 32]>,
    std::rc::Rc<std::cell::Cell<usize>>,
) {
    let mut next = faces.to_vec();
    let asked = std::rc::Rc::new(std::cell::Cell::new(0));
    let count = asked.clone();
    let draw = move || {
        count.set(count.get() + 1);
        let mut out = [0u8; 32];
        // The roller takes an answer's bytes from its end.
        for (i, slot) in out.iter_mut().rev().enumerate() {
            *slot = if next.is_empty() {
                (i % 2) as u8
            } else {
                next.remove(0) - 1
            };
        }
        Some(out)
    };
    (draw, asked)
}

/// Vector 2's share A begins `3`, value 17, bits 1 0 0 0 1: die 1 and
/// die 5 higher, the others lower.
const THREE: [u8; 10] = [1, 2, 2, 2, 1, 2, 1, 1, 1, 2];

fn press(m: &mut Codex32Make, act: WalkAct, draw: &mut impl FnMut() -> Option<[u8; 32]>) {
    let c = testkit::compute();
    m.walk_act(&c, act, 0, draw);
}

/// What the step shown reads.
fn shown(m: &Codex32Make) -> String {
    let (ch, _, step) = m.walk().shown().expect("a step is shown");
    step_text(m.work().unwrap(), ch, step)
}

#[test]
fn each_step_of_vector_2s_first_character_reads_its_working() {
    let mut m = entry();
    let (mut draw, _) = answers(&THREE);
    let mut texts = Vec::new();
    loop {
        press(&mut m, WalkAct::Next, &mut draw);
        texts.push(shown(&m));
        let (ch, s, _) = m.walk().shown().unwrap();
        if s == ch.steps().len() {
            break;
        }
    }
    assert_eq!(
        &texts[..11],
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
            "Die 5, second roll: 2. 2 is higher than 1, so bit 1.",
            "Bits 1 0 0 0 1 = 16 + 1 = 17. Leaf 17 of the tree is 3. 3 goes in box 10.",
        ]
    );
    // Box 10 is in the first four sum rows; the first adds the residue of
    // `ms` (R at box 10) to the string's 3: 17 + 3 is 18 in GF(32), J.
    assert_eq!(texts.len(), 15, "{texts:#?}");
    assert_eq!(texts[11], "Box 10, row a: 3 + R = J");
    for (k, t) in texts[12..].iter().enumerate() {
        assert!(
            t.starts_with(&format!("Box 10, row {}: ", ["c", "e", "g"][k])),
            "{t}"
        );
        // Each sum is its two cells added: in GF(32), their values' xor.
        let v = |c: &str| super::symbol_of(c.chars().next().unwrap()).unwrap();
        let rest = t.split(": ").nth(1).unwrap();
        let parts: Vec<&str> = rest.split(' ').collect();
        assert_eq!(v(parts[0]) ^ v(parts[2]), v(parts[4]), "{t}");
    }
    // The character went in its box: vector 2's share A starts `3`.
    let sh = m.work().unwrap().worksheet(0, Reveal::PaperForm).unwrap();
    assert_eq!(sh.symbol(10), super::symbol_of('3'));
}

#[test]
fn an_equal_pair_reads_roll_both_again() {
    let mut m = entry();
    // Die 1 rolls 1 then 1, then 6 then 2.
    let (mut draw, _) = answers(&[1, 2, 3, 4, 5, 1, 6, 2, 3, 2, 5, 4]);
    for _ in 0..6 {
        press(&mut m, WalkAct::Next, &mut draw);
    }
    assert_eq!(shown(&m), "Die 1: 1 then 1. Equal, so roll both again.");
    press(&mut m, WalkAct::Next, &mut draw);
    assert_eq!(shown(&m), "Die 1, first roll: 6. Marker on pad 6.");
    press(&mut m, WalkAct::Next, &mut draw);
    assert_eq!(
        shown(&m),
        "Die 1, second roll: 2. 2 is lower than 6, so bit 0."
    );
}

#[test]
fn previous_shows_the_state_before_and_rolls_nothing() {
    let mut m = entry();
    let (mut draw, asked) = answers(&THREE);
    let state = |m: &Codex32Make| {
        (
            m.entry_pos(),
            m.dice().len(),
            m.walk().shown().map(|(_, s, _)| s),
            m.work().unwrap().entered(0),
        )
    };
    let mut seen = Vec::new();
    for _ in 0..13 {
        seen.push((state(&m), m.walk().shown().map(|_| shown(&m))));
        press(&mut m, WalkAct::Next, &mut draw);
    }
    let asked_once = asked.get();
    // Back over the character's own step, into its last face held whole
    // out of its box, and to its first face.
    for want in seen.iter().rev().take(12) {
        press(&mut m, WalkAct::Previous, &mut draw);
        assert_eq!((state(&m), Some(shown(&m))), (want.0, want.1.clone()));
    }
    // Forward again: the same faces, nothing new asked of the randomness.
    for _ in 0..12 {
        press(&mut m, WalkAct::Next, &mut draw);
    }
    assert_eq!(asked.get(), asked_once);
    assert_eq!(m.work().unwrap().entered(0), 1);
    assert_eq!(m.made_of(0, 0).unwrap().value(), 17);
    assert!(matches!(m.walk().shown(), Some((_, 13, Step::Sum { .. }))));
}

#[test]
fn play_reaches_the_same_end_as_stepping() {
    let c = testkit::compute();
    for reach in [Reach::Pass, Reach::Character, Reach::Share] {
        let mut stepped = entry();
        let mut played = entry();
        let (mut d1, _) = answers(&THREE);
        let (mut d2, _) = answers(&THREE);
        press(&mut played, WalkAct::Reach(reach), &mut d2);
        press(&mut played, WalkAct::Speed(3), &mut d2);
        press(&mut played, WalkAct::Play, &mut d2);
        let mut now = 0;
        while played.walk().playing() && now < 1_000_000 {
            now += 16;
            played.walk_tick(&c, now, &mut d2);
        }
        let end = played.walk().shown().map(|(_, s, _)| s);
        let chars = played.work().unwrap().entered(0);
        // Stepping as far, the same steps shown.
        loop {
            press(&mut stepped, WalkAct::Next, &mut d1);
            let at = stepped.walk().shown().map(|(_, s, _)| s);
            if stepped.work().unwrap().entered(0) == chars && at == end {
                break;
            }
            assert!(stepped.work().unwrap().entered(0) <= chars);
        }
        assert_eq!(stepped.entry_pos(), played.entry_pos(), "{reach:?}");
        for n in 0..chars {
            assert_eq!(
                stepped.made_of(0, n).unwrap().value(),
                played.made_of(0, n).unwrap().value()
            );
        }
        match reach {
            Reach::Pass => assert_eq!(played.entry_pos(), (0, 5)),
            Reach::Character => assert_eq!(played.entry_pos(), (1, 0)),
            _ => assert_eq!(chars, 26),
        }
    }
}

#[test]
fn a_face_of_the_persons_own_dice_forgets_the_walk() {
    let c = testkit::compute();
    let mut m = entry();
    let (mut draw, _) = answers(&THREE);
    press(&mut m, WalkAct::Next, &mut draw);
    assert!(m.walk().shown().is_some());
    m.act(&c, C32::Key(OskPress::Char('4')), None);
    assert!(m.walk().shown().is_none());
    assert_eq!(m.entry_pos(), (0, 2));
}
