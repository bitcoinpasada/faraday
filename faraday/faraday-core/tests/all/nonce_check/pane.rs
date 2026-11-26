//! The nonce check on the flow's one card: the file and one row per input
//! at the left, the card showing every input's signatures or the chosen
//! one's, every warning above them whichever row is chosen.

use super::*;
use crate::common::add_key;
use crate::common::{has, has_part, lock as lock_app, texts};
use faraday_core::decode::CheckPress;
use faraday_shell_api::{BootState, DisplayInfo, SecureHardware};

fn wide(app: &mut Faraday) {
    sized(app, 1366, 768);
}

fn sized(app: &mut Faraday, width: u16, height: u16) {
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
}

fn open_wide(files: Vec<(&str, Vec<u8>)>) -> Faraday {
    let mut app = with_inbox(files);
    wide(&mut app);
    app.press(Action::NonceCheck(CheckPress::Inbox(0)));
    assert_eq!(app.screen, Screen::NonceCheck);
    app
}

fn count(app: &mut Faraday, want: &str) -> usize {
    texts(app).iter().filter(|t| *t == want).count()
}

fn choose(app: &mut Faraday, input: Option<usize>) {
    app.press(Action::NonceCheck(CheckPress::Input(input)));
}

/// Where a text holding `part` is drawn, the first of it: (x, y, w, h).
fn drawn_at(app: &mut Faraday, part: &str) -> Option<(i32, i32, i32, i32)> {
    let _ = app.frame();
    app.drawn_text_boxes()
        .plain()
        .into_iter()
        .find(|(t, _)| t.contains(part))
        .map(|b| b.1)
}

/// The nonce check of a finished transaction of 14 inputs on the window
/// the card is drawn on (1366x768), with its hex shown and the card
/// scrolled down.
fn card_scrolled_down() -> Faraday {
    let mut app = open_wide(vec![(
        "big.psbt",
        taproot_psbt(&vec![Tap::Key; 14]).to_bytes(),
    )]);
    assert!(app.outside_check().unwrap().hex.is_some(), "a finished one");
    app.press(Action::NonceCheck(CheckPress::Hex));
    let _ = app.frame();
    app.event(Event::Hover { x: 900, y: 300 });
    for _ in 0..4 {
        app.event(Event::Scroll {
            x: 900,
            y: 300,
            dy: 600,
        });
        app.event(Event::ScrollEnd { x: 900, y: 300 });
        app.settle();
    }
    assert!(
        app.nonce_card_offset().unwrap() > 0.0,
        "the card is scrolled"
    );
    app
}

/// Choosing a row puts the card at its top, so the headline and the row
/// chosen are in view, though the hex keeps the card taller than the window.
#[test]
fn choosing_a_row_brings_the_card_to_its_top() {
    let mut app = card_scrolled_down();
    choose(&mut app, Some(3));
    let _ = app.frame();
    assert_eq!(app.nonce_card_offset(), Some(0.0));
}

/// The same on opening another file.
#[test]
fn a_second_file_opens_with_the_card_at_its_top() {
    let one = taproot_psbt(&vec![Tap::Key; 14]).to_bytes();
    let mut app = open_wide(vec![("a.psbt", one.clone()), ("b.psbt", one)]);
    app.press(Action::NonceCheck(CheckPress::Hex));
    let _ = app.frame();
    app.event(Event::Hover { x: 900, y: 300 });
    app.event(Event::Scroll {
        x: 900,
        y: 300,
        dy: 1500,
    });
    app.event(Event::ScrollEnd { x: 900, y: 300 });
    app.settle();
    assert!(app.nonce_card_offset().unwrap() > 0.0);
    app.press(Action::NonceCheck(CheckPress::Inbox(1)));
    let _ = app.frame();
    assert_eq!(app.nonce_card_offset(), Some(0.0));
}

/// Back from Decode puts the card at its top.
#[test]
fn back_from_decode_brings_the_card_to_its_top() {
    let mut app = card_scrolled_down();
    app.press(Action::NonceCheck(CheckPress::Decode));
    app.press(Action::NonceCheck(CheckPress::Back));
    let _ = app.frame();
    assert_eq!(app.nonce_card_offset(), Some(0.0));
}

/// Inputs past the 12th are one row: a warning on one of them is said
/// there, in the error role, as on the card.
#[test]
fn the_last_row_names_a_warning_on_an_input_past_the_twelfth() {
    let mut app = open_wide(vec![(
        "big.psbt",
        signed_elsewhere_of(How::SameNonce, 14).to_bytes(),
    )]);
    assert_eq!(app.outside_check().unwrap().rows.len(), 14);
    // The pane's end, where the row is drawn.
    let _ = app.frame();
    app.event(Event::Hover { x: 330, y: 300 });
    app.event(Event::Scroll {
        x: 330,
        y: 300,
        dy: 3000,
    });
    app.event(Event::ScrollEnd { x: 330, y: 300 });
    app.settle();
    assert!(has(&mut app, "2 more"), "the row");
    let wrong = EN.nonce_check_one_wrong.trim_start_matches("· ");
    assert!(has(&mut app, wrong), "named wrong");
    assert!(!has(
        &mut app,
        EN.nonce_check_none_wrong.trim_start_matches("· ")
    ));
}

#[test]
fn a_signature_made_by_no_nonce_rule_reads_wrong_in_its_row() {
    let mut app = with_inbox(vec![(
        "own.psbt",
        signed_elsewhere(How::OwnNonce).to_bytes(),
    )]);
    add_key(&mut app, "zebra");
    wide(&mut app);
    app.press(Action::NonceCheck(CheckPress::Inbox(0)));
    let o = app.outside_check().unwrap();
    assert!(o.rows[0].wrong);
    let said = o.rows[0].verdict.clone().unwrap();
    assert_eq!(said, EN.nonce_check_wrong_nonce);
}

#[test]
fn the_file_row_chooses_all_inputs() {
    let mut app = open_wide(vec![(
        "signed.psbt",
        signed_elsewhere(How::SameNonce).to_bytes(),
    )]);
    choose(&mut app, Some(1));
    assert_eq!(app.outside_check().unwrap().input, Some(1));
    let at = drawn_at(&mut app, "signed.psbt").expect("the file row");
    let (x, y) = (
        u16::try_from(at.0 + 4).unwrap(),
        u16::try_from(at.1 + 4).unwrap(),
    );
    app.event(Event::Touch {
        x,
        y,
        phase: faraday_shell_api::TouchPhase::Down,
    });
    app.event(Event::Touch {
        x,
        y,
        phase: faraday_shell_api::TouchPhase::Up,
    });
    app.settle();
    assert_eq!(app.outside_check().unwrap().input, None);
}

#[test]
fn the_headline_stays_with_every_row_chosen() {
    let mut app = open_wide(vec![(
        "signed.psbt",
        signed_elsewhere(How::SameNonce).to_bytes(),
    )]);
    let head = app.outside_check().unwrap().check.headline();
    for row in [None, Some(0), Some(1)] {
        choose(&mut app, row);
        assert!(has(&mut app, &head), "{row:?}");
    }
}

#[test]
fn an_input_not_known_is_warned_of_with_a_row_chosen() {
    let mut app = open_wide(vec![("t.psbt", unread_final("none").to_bytes())]);
    for row in [None, Some(0), Some(1)] {
        choose(&mut app, row);
        assert!(has(&mut app, EN.nonce_screen_refused_unread), "{row:?}");
    }
}

#[test]
fn the_buttons_act_on_the_whole_transaction_with_a_row_chosen() {
    let mut app = open_wide(vec![(
        "signed.psbt",
        signed_elsewhere(How::SameNonce).to_bytes(),
    )]);
    let all = {
        let o = app.outside_check().unwrap();
        (o.out.0.clone(), o.hex.clone())
    };
    choose(&mut app, Some(0));
    let o = app.outside_check().unwrap();
    assert_eq!((o.out.0.clone(), o.hex.clone()), all);
    app.press(Action::NonceCheck(CheckPress::Hex));
    assert!(app.outside_check().unwrap().show_hex);
}

fn said_in_pane(app: &mut Faraday, part: &str) -> usize {
    texts(app).iter().filter(|t| t.contains(part)).count()
}

#[test]
fn a_reused_nonce_is_said_in_the_pane_row_of_each_input_it_names() {
    let mut app = open_wide(vec![(
        "signed.psbt",
        signed_elsewhere(How::SameNonce).to_bytes(),
    )]);
    assert_eq!(said_in_pane(&mut app, EN.nonce_pane_reused), 2);
}

#[test]
fn a_mismatched_input_is_said_in_its_pane_row_only() {
    let mut app = open_wide(vec![(
        "t.psbt",
        taproot_psbt(&[Tap::Key, lock(true)]).to_bytes(),
    )]);
    assert_eq!(said_in_pane(&mut app, EN.nonce_pane_mismatch), 1);
}

#[test]
fn an_input_no_warning_names_keeps_its_verdict() {
    let mut app = open_wide(vec![("low.psbt", signed_elsewhere(How::LowR).to_bytes())]);
    for part in [
        EN.nonce_pane_reused,
        EN.nonce_pane_mismatch,
        EN.nonce_pane_used_before,
    ] {
        assert_eq!(said_in_pane(&mut app, part), 0, "{part}");
    }
    let o = app.outside_check().unwrap();
    let said = o.rows[0].verdict.clone().unwrap();
    assert!(said_in_pane(&mut app, &said) >= 1);
}

fn warned(app: &mut Faraday, part: &str) -> bool {
    texts(app).iter().any(|t| t.contains(part))
}

#[test]
fn the_pane_has_the_file_all_inputs_and_a_row_per_input() {
    let mut app = open_wide(vec![(
        "signed.psbt",
        signed_elsewhere(How::SameNonce).to_bytes(),
    )]);
    let o = app.outside_check().unwrap();
    assert_eq!(o.rows.len(), 2);
    assert!(
        o.rows
            .iter()
            .all(|r| r.signatures == 1 && r.verdict.is_some())
    );
    assert_eq!(o.input, None, "it opens on All inputs");
    assert!(texts(&mut app).contains(&"signed.psbt".to_string()));
    assert_eq!(count(&mut app, EN.nonce_pane_all), 1);
    assert_eq!(count(&mut app, EN.nonce_pane_question), 1);
    // The input's number as the card's rows have it: in the pane and in
    // the card.
    assert_eq!(count(&mut app, "Input 0"), 2);
    assert_eq!(count(&mut app, "Input 1"), 2);
}

#[test]
fn a_row_shows_only_its_inputs_signatures() {
    let mut app = open_wide(vec![(
        "signed.psbt",
        signed_elsewhere(How::SameNonce).to_bytes(),
    )]);
    choose(&mut app, Some(1));
    assert_eq!(app.outside_check().unwrap().input, Some(1));
    assert_eq!(count(&mut app, "Input 0"), 1, "the pane's row only");
    assert_eq!(count(&mut app, "Input 1"), 2);
    choose(&mut app, Some(0));
    assert_eq!(count(&mut app, "Input 0"), 2);
    assert_eq!(count(&mut app, "Input 1"), 1);
    choose(&mut app, None);
    assert_eq!(count(&mut app, "Input 0"), 2);
    assert_eq!(count(&mut app, "Input 1"), 2);
}

#[test]
fn a_reused_nonce_is_warned_of_with_every_row_chosen() {
    let mut app = open_wide(vec![(
        "signed.psbt",
        signed_elsewhere(How::SameNonce).to_bytes(),
    )]);
    let part = "one nonce on inputs";
    for row in [None, Some(0), Some(1)] {
        choose(&mut app, row);
        assert!(warned(&mut app, part), "{row:?}");
    }
}

#[test]
fn a_mismatched_input_is_warned_of_with_every_row_chosen() {
    let mut app = open_wide(vec![(
        "t.psbt",
        taproot_psbt(&[Tap::Key, lock(true)]).to_bytes(),
    )]);
    for row in [None, Some(0), Some(1)] {
        choose(&mut app, row);
        assert!(has(&mut app, &mismatch_text(1)), "{row:?}");
        assert!(has(&mut app, EN.nonce_screen_refused_witness), "{row:?}");
    }
}

#[test]
fn an_input_with_no_signature_says_so() {
    let mut psbt = signed_elsewhere(How::SameNonce);
    psbt.inner_mut().inputs[1].partial_sigs.clear();
    let mut app = open_wide(vec![("one.psbt", psbt.to_bytes())]);
    assert_eq!(app.outside_check().unwrap().rows[1].signatures, 0);
    assert_eq!(count(&mut app, EN.nonce_pane_no_signature), 1);
    choose(&mut app, Some(1));
    assert!(has(&mut app, EN.nonce_card_input_none));
}

#[test]
fn the_row_resets_with_a_new_file_on_leaving_and_at_a_lock() {
    let one = signed_elsewhere(How::SameNonce).to_bytes();
    let mut app = open_wide(vec![("a.psbt", one.clone()), ("b.psbt", one)]);
    choose(&mut app, Some(1));
    app.press(Action::NonceCheck(CheckPress::Inbox(1)));
    assert_eq!(app.outside_check().unwrap().input, None);
    choose(&mut app, Some(1));
    app.press(Action::Nav(Screen::Home));
    assert_eq!(app.outside_check().unwrap().input, None, "leaving");
    app.press(Action::NonceCheck(CheckPress::Inbox(0)));
    choose(&mut app, Some(1));
    lock_app(&mut app);
    assert!(
        app.outside_check()
            .expect("the check stays")
            .input
            .is_none(),
        "a lock"
    );
}

/// The pane keeps no scroll from the file before it.
#[test]
fn a_second_file_opens_with_the_pane_at_its_top() {
    let one = signed_elsewhere_of(How::SameNonce, 14).to_bytes();
    let mut app = open_wide(vec![("a.psbt", one.clone()), ("b.psbt", one)]);
    let _ = app.frame();
    app.event(Event::Hover { x: 330, y: 300 });
    app.event(Event::Scroll {
        x: 330,
        y: 300,
        dy: 1500,
    });
    app.event(Event::ScrollEnd { x: 330, y: 300 });
    app.settle();
    assert!(app.choices_offset() > 0.0, "the pane is scrolled");
    app.press(Action::NonceCheck(CheckPress::Inbox(1)));
    assert_eq!(app.choices_offset(), 0.0);
}

/// What the Out button sends of a two-input taproot file, with `row`
/// chosen first, and its hex.
fn sent(row: Option<usize>) -> (Vec<u8>, Option<faraday_text::Text>) {
    let mut app = open_wide(vec![(
        "t.psbt",
        taproot_psbt(&[Tap::Key, Tap::Key]).to_bytes(),
    )]);
    if row.is_some() {
        choose(&mut app, row);
    }
    let hex = app.outside_check().unwrap().hex.clone();
    app.press(Action::NonceCheck(CheckPress::Out));
    let bytes = app.outbox.last().expect("sent").bytes.to_vec();
    (bytes, hex)
}

/// The buttons carry the whole transaction to the stick with a row chosen.
#[test]
fn the_file_for_the_stick_is_the_whole_transaction_with_a_row_chosen() {
    let (all, all_hex) = sent(None);
    let (one, one_hex) = sent(Some(0));
    assert!(all_hex.is_some(), "a finished one");
    assert_eq!(one_hex, all_hex);
    assert_eq!(one, all);
}

/// An input not known, past the twelfth, is not "none wrong".
#[test]
fn the_last_row_does_not_say_none_wrong_over_an_input_not_known() {
    let mut psbt = taproot_psbt(&vec![Tap::Key; 14]);
    psbt.inner_mut().inputs[13].witness_utxo = None;
    let mut app = open_wide(vec![("big.psbt", psbt.to_bytes())]);
    assert_eq!(app.outside_check().unwrap().check.unread, 1);
    let _ = app.frame();
    app.event(Event::Hover { x: 330, y: 300 });
    app.event(Event::Scroll {
        x: 330,
        y: 300,
        dy: 3000,
    });
    app.event(Event::ScrollEnd { x: 330, y: 300 });
    app.settle();
    assert!(has(&mut app, "2 more"), "the row");
    for words in [EN.nonce_check_none_wrong, EN.nonce_check_one_wrong] {
        assert!(!has(&mut app, words.trim_start_matches("· ")), "{words}");
    }
}

/// The card's own line over the inputs past the twelfth says what the
/// pane's does.
#[test]
fn the_cards_more_line_names_a_warning_on_a_late_input() {
    let mut app = open_wide(vec![(
        "big.psbt",
        signed_elsewhere_of(How::SameNonce, 14).to_bytes(),
    )]);
    let _ = app.frame();
    app.event(Event::Hover { x: 900, y: 300 });
    app.event(Event::Scroll {
        x: 900,
        y: 300,
        dy: 6000,
    });
    app.event(Event::ScrollEnd { x: 900, y: 300 });
    app.settle();
    assert!(has_part(&mut app, "2 more"), "the card's line");
    assert!(has_part(&mut app, EN.nonce_check_one_wrong));
    assert!(!has_part(&mut app, EN.nonce_check_none_wrong));
}

/// Decode and back leave the pane at its top.
#[test]
fn the_pane_is_at_its_top_in_decode_and_back() {
    let mut app = open_wide(vec![(
        "big.psbt",
        taproot_psbt(&vec![Tap::Key; 14]).to_bytes(),
    )]);
    let _ = app.frame();
    app.event(Event::Hover { x: 330, y: 300 });
    app.event(Event::Scroll {
        x: 330,
        y: 300,
        dy: 1500,
    });
    app.event(Event::ScrollEnd { x: 330, y: 300 });
    app.settle();
    assert!(app.choices_offset() > 0.0, "the pane is scrolled");
    app.press(Action::NonceCheck(CheckPress::Decode));
    assert_eq!(app.choices_offset(), 0.0, "Decode");
    app.event(Event::Scroll {
        x: 330,
        y: 300,
        dy: 1500,
    });
    app.event(Event::ScrollEnd { x: 330, y: 300 });
    app.settle();
    app.press(Action::NonceCheck(CheckPress::Back));
    assert_eq!(app.choices_offset(), 0.0, "back");
}

fn nonce_tile_from_check(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Check));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Nonce check")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
}

#[test]
fn opened_from_check_with_nothing_waiting_the_card_holds_the_ways_in() {
    let mut app = with_inbox(Vec::new());
    wide(&mut app);
    nonce_tile_from_check(&mut app);
    assert_eq!(app.screen, Screen::NonceCheck);
    assert_eq!(app.sheet, None, "no camera");
    let ways = drawn_at(&mut app, EN.parts_open_files).expect("a way in");
    let card = drawn_at(&mut app, EN.nonce_screen_none).expect("the card waits");
    assert!(
        ways.0 + ways.2 <= card.0,
        "ways in at the left: {ways:?} {card:?}"
    );
    assert!(has(&mut app, EN.compact_scan));
    // A scan from there opens the check, whose way back is to Check.
    app.press(Action::Scan);
    app.event(Event::Scanned {
        bytes: signed_elsewhere(How::LowR).to_base64().into_bytes(),
    });
    assert_eq!(app.screen, Screen::NonceCheck);
    assert_eq!(app.outside_check().expect("checked").back, Screen::Check);
    let _ = app.frame();
    assert!(!has(&mut app, EN.nonce_screen_none));
}

#[test]
fn opened_from_check_on_a_small_panel_with_nothing_waiting_is_the_camera() {
    let mut app = with_inbox(Vec::new());
    sized(&mut app, 480, 640);
    nonce_tile_from_check(&mut app);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(app.outside_check().is_none());
}
