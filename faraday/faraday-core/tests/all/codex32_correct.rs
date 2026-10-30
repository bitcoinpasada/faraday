//! Codex32 error correction as a person meets it (`docs/CODEX32.md` §8.4):
//! the tick off by default, a damaged string's one proposal shown box by
//! box and four to a page, nothing taken until the changed boxes are
//! typed again from the paper and match, a refusal as its own line, the
//! marks a corrected string and its key carry, and no costly work on a
//! press. Driven through the interface a codex32 form holds
//! (`faraday_core::codex32_correct`), on BIP 93's published strings.

use faraday_bip::codex32::{Codex32, correct};
use faraday_core::codex32_correct::{
    BoxShown, Correction, KeyMarks, Mark, Offered, Outcome, PAIRS_PER_PAGE, Press, Step, typable,
};
use faraday_core::strings::EN;
use faraday_core::testkit;
use faraday_core::work::{KINDS, Work, since, snapshot};
use faraday_shell_api::Key;

/// BIP 93's vector 1, the secret alone.
const SECRET: &str = "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw";
/// BIP 93's vector 2, two shares of one set, lowercase.
const SHARE_A: &str = "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm";
const SHARE_C: &str = "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn";

const BECH32: &[u8] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// `s` with box `n` (from 1) changed to another bech32 character.
fn changed(s: impl AsRef<str>, n: usize) -> String {
    let mut b = s.as_ref().as_bytes().to_vec();
    let at = BECH32.iter().position(|&c| c == b[n - 1]).unwrap_or(0);
    b[n - 1] = BECH32[(at + 7) % 32];
    String::from_utf8(b).unwrap()
}

/// `s` with box `n` marked unreadable.
fn unreadable(s: impl AsRef<str>, n: usize) -> String {
    let mut b = s.as_ref().as_bytes().to_vec();
    b[n - 1] = b'?';
    String::from_utf8(b).unwrap()
}

fn ticked() -> Correction {
    let mut c = Correction::new();
    c.press(Press::Tick);
    c
}

fn offer(c: &mut Correction, text: &str, held: Option<&Codex32>) -> Offered {
    c.offer(&testkit::compute(), text, held)
}

fn typed(c: &mut Correction, chars: &[u8]) -> Outcome {
    let mut last = Outcome::Stay;
    for &ch in chars {
        last = c.key(Key::Char(char::from(ch)));
    }
    last
}

fn taken(o: Outcome) -> (String, Mark) {
    match o {
        Outcome::Take(t) => (t.text().to_string(), t.mark()),
        Outcome::Stay => panic!("nothing taken"),
        Outcome::Edit => panic!("the entry went back for editing"),
    }
}

/// The form opens with the tick off: nothing is decoded, a failing string
/// is the form's to refuse as it always has, and `?` cannot be typed.
#[test]
fn the_tick_is_off_until_pressed() {
    let mut c = Correction::new();
    assert!(!c.on());
    assert!(!typable(&c, '?'));
    let before = snapshot();
    assert_eq!(offer(&mut c, &changed(SECRET, 20), None), Offered::Holds);
    assert_eq!(since(&before).of(Work::Codex32), 0, "nothing decoded");
    assert!(!c.is_open());
    c.press(Press::Tick);
    assert!(c.on());
    assert!(typable(&c, '?'));
    assert!(!typable(&c, '!'));
}

/// A string that holds is taken as typed: nothing is shown.
#[test]
fn a_string_that_holds_shows_nothing() {
    let mut c = ticked();
    assert_eq!(offer(&mut c, SECRET, None), Offered::Holds);
    assert!(!c.is_open());
}

/// Every one-box mistake, at every box the decoder may change, is shown
/// as the decoder's own proposal, box by box, four to a page.
#[test]
fn one_wrong_box_anywhere_is_shown_as_the_decoder_proposes() {
    for n in 4..=SECRET.len() {
        let damaged = changed(SECRET, n);
        let mut c = ticked();
        assert_eq!(offer(&mut c, &damaged, None), Offered::Shown, "box {n}");
        let want = correct(&testkit::compute(), &damaged, None).expect("within the bound");
        let shown: Vec<_> = c
            .edits()
            .iter()
            .map(|e| (e.position(), e.was(), e.now()))
            .collect();
        let decoded: Vec<_> = want
            .edits()
            .iter()
            .map(|e| (e.position(), e.was(), e.now()))
            .collect();
        assert_eq!(shown, decoded, "box {n}");
        assert_eq!(
            shown,
            vec![(n, damaged.as_bytes()[n - 1], SECRET.as_bytes()[n - 1])]
        );
        assert_eq!(c.step(), Some(Step::Pairs));
        assert_eq!(c.page(), Some((0, 1)));
        assert!(c.all_seen());
    }
}

/// I mistyped: the changed box is cleared and every other box locked; the
/// box typed again from the paper is taken only where it matches, and the
/// string then carries "Share entry corrected".
#[test]
fn i_mistyped_takes_the_boxes_typed_again_from_the_paper() {
    let damaged = changed(SECRET, 30);
    let mut c = ticked();
    offer(&mut c, &damaged, None);
    assert!(!c.offers_unreadable(), "a substituted box was readable");
    assert!(matches!(c.press(Press::Mistyped), Outcome::Stay));
    assert_eq!(c.step(), Some(Step::Retype));
    assert!(
        !typable(&c, '?'),
        "no box is unreadable while typing from the paper"
    );
    for n in 1..=SECRET.len() {
        let b = c.box_at(n).unwrap();
        if n == 30 {
            assert_eq!(b, BoxShown::Open { next: true });
        } else {
            assert_eq!(b, BoxShown::Locked(SECRET.as_bytes()[n - 1]), "box {n}");
        }
    }
    assert_eq!(c.box_at(SECRET.len() + 1), None);
    // The wrong character again: nothing taken, the box cleared.
    let wrong = damaged.as_bytes()[29];
    assert!(matches!(typed(&mut c, &[wrong]), Outcome::Stay));
    assert!(c.differs());
    assert_eq!(c.retyped(), (0, 1));
    assert_eq!(c.box_at(30), Some(BoxShown::Open { next: true }));
    // The paper's character.
    let (text, mark) = taken(typed(&mut c, &[SECRET.as_bytes()[29]]));
    assert_eq!(text, SECRET);
    assert_eq!(mark, Mark::Corrected);
    assert!(!c.is_open());
    let string = Codex32::parse(&text).unwrap();
    c.note(&string, mark);
    assert_eq!(c.mark_of(&string), Some(Mark::Corrected));
    assert_eq!(c.key_mark(), Some(Mark::Corrected));
    assert_eq!(
        Mark::Corrected.fingerprint_line(),
        EN.codex32_correct_entry_corrected
    );
    assert_eq!(Mark::Corrected.string_line(), None);
}

/// Typing again takes bech32 characters only, upper case read as lower,
/// and Backspace takes the last one back.
#[test]
fn the_re_entry_takes_bech32_characters_and_backspace() {
    let damaged = changed(changed(SECRET, 12), 40);
    let mut c = ticked();
    offer(&mut c, &damaged, None);
    c.press(Press::Mistyped);
    typed(&mut c, b"b1o?");
    assert_eq!(c.retyped(), (0, 2), "b, 1, o and ? are not bech32");
    let first = SECRET.as_bytes()[11];
    c.key(Key::Char(char::from(first).to_ascii_uppercase()));
    assert_eq!(
        c.box_at(12),
        Some(BoxShown::Typed {
            ch: first,
            next: false
        })
    );
    assert_eq!(c.box_at(40), Some(BoxShown::Open { next: true }));
    c.key(Key::Backspace);
    assert_eq!(c.box_at(12), Some(BoxShown::Open { next: true }));
    let (text, _) = taken(typed(&mut c, &[first, SECRET.as_bytes()[39]]));
    assert_eq!(text, SECRET);
}

/// The re-entry's caret moves between its boxes: a character typed goes
/// in the box at the caret, in place of what was there, and the caret
/// moves on; Left takes it back a box (`docs/TEXT-CURSOR.md`).
#[test]
fn the_re_entry_caret_moves_between_the_boxes() {
    let damaged = changed(changed(SECRET, 12), 40);
    let mut c = ticked();
    offer(&mut c, &damaged, None);
    c.press(Press::Mistyped);
    let (first, last) = (SECRET.as_bytes()[11], SECRET.as_bytes()[39]);
    let wrong = if first == b'q' { b'p' } else { b'q' };
    c.key(Key::Char(char::from(wrong)));
    assert_eq!(c.box_at(40), Some(BoxShown::Open { next: true }));
    c.key(Key::Left);
    c.key(Key::Char(char::from(first)));
    assert_eq!(c.retyped(), (1, 2), "the box at the caret, typed over");
    let (text, _) = taken(typed(&mut c, &[last]));
    assert_eq!(text, SECRET);
}

/// More boxes than fit a page: the ways on are dimmed and do nothing
/// until every page has been seen.
#[test]
fn every_page_is_seen_before_going_on() {
    // Two substitutions and four unreadable boxes: 2 × 2 + 4 = 8.
    let mut damaged = changed(changed(SECRET, 15), 33);
    for n in [20, 21, 25, 44] {
        damaged = unreadable(&damaged, n);
    }
    let mut c = ticked();
    offer(&mut c, &damaged, None);
    assert_eq!(c.edits().len(), 6);
    assert_eq!(c.page(), Some((0, 2)));
    assert_eq!(c.page_edits().len(), PAIRS_PER_PAGE);
    assert!(!c.all_seen());
    c.press(Press::Mistyped);
    c.press(Press::Damaged);
    assert_eq!(
        c.step(),
        Some(Step::Pairs),
        "nothing goes on before page 2 is seen"
    );
    c.press(Press::Next);
    assert_eq!(c.page(), Some((1, 2)));
    assert_eq!(c.page_edits().len(), 2);
    c.press(Press::Previous);
    assert!(c.all_seen(), "seen stays seen");
    c.press(Press::Mistyped);
    assert_eq!(c.step(), Some(Step::Retype));
}

/// The paper is damaged: the pairs as repairs to write on it, then the
/// same re-entry; the string is "Repaired, not checked".
#[test]
fn a_damaged_paper_is_repaired_then_typed_again() {
    let damaged = changed(SHARE_A, 25);
    let mut c = ticked();
    offer(&mut c, &damaged, None);
    c.press(Press::Damaged);
    assert_eq!(c.step(), Some(Step::Repairs));
    assert_eq!(c.page_edits().len(), 1);
    c.press(Press::Continue);
    assert_eq!(c.step(), Some(Step::Retype));
    let (text, mark) = taken(typed(&mut c, &[SHARE_A.as_bytes()[24]]));
    assert_eq!(text, SHARE_A);
    assert_eq!(mark, Mark::Repaired);
    assert_eq!(
        mark.string_line(),
        Some(EN.codex32_correct_repaired_not_checked)
    );
    assert_eq!(
        mark.fingerprint_line(),
        EN.codex32_correct_repairs_unchecked
    );
}

/// Unreadable on the paper is offered only where every changed box was
/// marked `?`; it is taken as it is after its caution, marked repaired.
#[test]
fn unreadable_boxes_are_taken_after_a_caution() {
    let damaged = unreadable(unreadable(unreadable(SECRET, 11), 27), 46);
    let mut c = ticked();
    assert!(typable(&c, '?'));
    offer(&mut c, &damaged, None);
    assert!(c.offers_unreadable());
    assert!(c.edits().iter().all(|e| e.was_unreadable()));
    c.press(Press::Unreadable);
    assert_eq!(c.step(), Some(Step::FromChecksum));
    let (text, mark) = taken(c.press(Press::Take));
    assert_eq!(text, SECRET);
    assert_eq!(mark, Mark::Repaired);

    // With a readable box changed too, it is not offered.
    let mixed = changed(unreadable(SECRET, 11), 30);
    let mut c = ticked();
    offer(&mut c, &mixed, None);
    assert!(!c.offers_unreadable());
    c.press(Press::Unreadable);
    assert_eq!(c.step(), Some(Step::Pairs));
}

/// A run of more than eight unreadable boxes shows its caution first.
#[test]
fn a_long_run_shows_its_caution_first() {
    let mut damaged = SECRET.to_string();
    for n in 20..30 {
        damaged = unreadable(&damaged, n);
    }
    let mut c = ticked();
    offer(&mut c, &damaged, None);
    assert_eq!(c.step(), Some(Step::Caution));
    assert!(c.assumes_the_rest());
    c.key(Key::Enter);
    assert_eq!(c.step(), Some(Step::Pairs));
    assert_eq!(c.edits().len(), 10);
    assert_eq!(c.page(), Some((0, 3)));
}

/// Where no correction is found, the refusal is its own line, and Check
/// the boxes hands the entry back as typed.
#[test]
fn each_refusal_is_its_own_line() {
    // Five wrong boxes: past the bound.
    let mut far = SECRET.to_string();
    for n in [12, 18, 24, 30, 36] {
        far = changed(&far, n);
    }
    let mut c = ticked();
    assert_eq!(offer(&mut c, &far, None), Offered::Shown);
    if c.refusal().is_some() {
        assert_eq!(
            c.refusal_line().as_deref(),
            Some(EN.codex32_correct_not_found)
        );
    }
    assert!(matches!(c.press(Press::CheckBoxes), Outcome::Edit));
    assert!(!c.is_open());

    // Unreadable boxes past eight and not in one run.
    let mut holes = SECRET.to_string();
    for n in [10, 12, 14, 16, 18, 20, 22, 24, 26] {
        holes = unreadable(&holes, n);
    }
    let mut c = ticked();
    offer(&mut c, &holes, None);
    let line = c.refusal_line().expect("refused");
    assert!(line.contains('8') && line.contains("13"), "{line}");

    // Not a codex32 string at all.
    let mut c = ticked();
    offer(&mut c, "ms1short", None);
    let line = c.refusal_line().expect("refused");
    assert!(
        line.starts_with(&EN.codex32_correct_malformed[..20]),
        "{line}"
    );

    // A second string of a set whose identifier box was changed: the
    // strings already in fix it.
    let held = Codex32::parse(SHARE_A).unwrap();
    let mut c = ticked();
    offer(&mut c, &changed(SHARE_C, 6), Some(&held));
    assert_eq!(
        c.refusal_line().as_deref(),
        Some(EN.codex32_correct_header_fixed)
    );
    assert!(matches!(c.key(Key::Escape), Outcome::Edit));
}

/// The same five changes, decoded: the decoder's own answer is what is
/// shown, never a guess.
#[test]
fn past_the_bound_nothing_is_proposed_the_decoder_does_not() {
    let mut far = SECRET.to_string();
    for n in [12, 18, 24, 30, 36] {
        far = changed(&far, n);
    }
    let decoded = correct(&testkit::compute(), &far, None);
    let mut c = ticked();
    offer(&mut c, &far, None);
    match decoded {
        Ok(p) => {
            assert_ne!(p.string().encode().as_str(), SECRET);
            assert_eq!(c.edits().len(), p.edits().len());
        }
        Err(r) => assert_eq!(c.refusal(), Some(r)),
    }
}

/// The tick turned off closes what is shown; the entry stays as typed.
#[test]
fn the_tick_off_closes_the_proposal() {
    let mut c = ticked();
    offer(&mut c, &changed(SECRET, 20), None);
    assert!(c.is_open());
    c.press(Press::Tick);
    assert!(!c.on() && !c.is_open());
}

/// The key's line stays: a weaker mark never replaces a stronger, and
/// nothing removes one.
#[test]
fn a_keys_mark_is_never_removed() {
    let fp = [0x73, 0xc5, 0xda, 0x0a];
    let mut marks = KeyMarks::default();
    assert_eq!(marks.of(fp), None);
    marks.note(fp, Mark::Repaired);
    marks.note(fp, Mark::Corrected);
    assert_eq!(marks.of(fp), Some(Mark::Repaired));
    let mut marks = KeyMarks::default();
    marks.note(fp, Mark::Corrected);
    marks.note(fp, Mark::Repaired);
    assert_eq!(marks.of(fp), Some(Mark::Repaired));

    let mut c = ticked();
    let a = Codex32::parse(SHARE_A).unwrap();
    let cc = Codex32::parse(SHARE_C).unwrap();
    c.note(&a, Mark::Corrected);
    assert_eq!(c.mark_of(&cc), None);
    c.note(&cc, Mark::Repaired);
    assert_eq!(c.key_mark(), Some(Mark::Repaired));
}

/// A press's budget: the offer decodes the string once and does nothing
/// else costly; every press, key and box read after it, and drawing the
/// screens, does no costly work at all.
#[test]
fn only_the_offer_decodes() {
    let mut damaged = changed(SECRET, 15);
    for n in [20, 21, 25, 44] {
        damaged = unreadable(&damaged, n);
    }
    let mut c = ticked();
    let before = snapshot();
    offer(&mut c, &damaged, None);
    let work = since(&before);
    for k in KINDS {
        let want = u64::from(k == Work::Codex32);
        assert_eq!(work.of(k), want, "{k:?} on the offer");
    }
    let before = snapshot();
    let _ = faraday_core::codex32_correct::screen::preview(&c, None, 480, 640, 160);
    c.press(Press::Next);
    c.press(Press::Mistyped);
    for n in 1..=c.len() {
        let _ = c.box_at(n);
    }
    let _ = faraday_core::codex32_correct::screen::preview(&c, Some(Mark::Repaired), 480, 640, 160);
    let edits: Vec<u8> = c.edits().iter().map(|e| e.now()).collect();
    let _ = taken(typed(&mut c, &edits));
    let work = since(&before);
    for k in KINDS {
        assert_eq!(work.of(k), 0, "{k:?} on a press, a key or a frame");
    }
}
