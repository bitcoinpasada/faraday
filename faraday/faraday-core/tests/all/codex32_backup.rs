//! Codex32 in a backup (`docs/CODEX32.md` §9), as a person backing up a
//! wallet would notice it, against BIP 93's vector 2
//! (`tools/vectors/bip93/vectors.json`): a key with a set opens on one
//! string per place; the checklist checks each string typed back, its
//! header against the set and, once two are in, that they rebuild the
//! key; a set the plan makes is made, placed and typed back; a lost share
//! is made again exactly, a new one added, and a set rotated only once the
//! held strings are destroyed; the checks count strings, and the
//! templates name their sheet.

use faraday_bip::keys::ScriptType;
use faraday_core::board::{BoardPress, Thing, Zone};
use faraday_core::bstep;
use faraday_core::codex32_backup::{Found, Press, SetPress, SetStep};
use faraday_core::plan::{self, At, What};
use faraday_core::{Action, Faraday, StorageEvent, testkit};
use faraday_keys::texts::Spec;
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};
use faraday_ui::keyboard::OskPress;
use faraday_vault::codex32::{SetFacts, State};

use crate::common::board;

/// Vector 2: shares A and C, D derived, the secret; the fingerprint of its
/// seed's key.
const A: &str = "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm";
const C: &str = "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn";
const D: &str = "ms12namedll4f8jlh4e5vdvuldlfxu2jhdnlsm97xvenrxeg";
const FP: [u8; 4] = [0xfa, 0xb6, 0x86, 0x8a];
/// Vector 3's A: a string of another set, threshold 3.
const OTHER: &str = "ms13casha320zyxwvutsrqpnmlkjhgfedca2a8d0zehn8a0t";

const BIP93: &str = include_str!("../../../../tools/vectors/bip93/vectors.json");

/// A share index's bech32 value.
fn ix(c: char) -> usize {
    plan::codex32::index_of(c as u8).unwrap()
}

fn on() -> Faraday {
    let mut app = testkit::started();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x60 + i; 32])));
    }
    app
}

fn typed(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// A single-key wallet over session key 0, loaded, and Back up opened.
fn wallet_and_backup(app: &mut Faraday) {
    let c = testkit::compute();
    let text = app
        .session
        .keys
        .key_text(&c, 0, &Spec::Single(ScriptType::NativeSegwit, 0))
        .unwrap()
        .unwrap();
    app.storage(StorageEvent::Restored {
        inbox: vec![(
            "shares-wallet.txt".into(),
            format!("wpkh({text}/<0;1>/*)").into_bytes(),
        )],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
    app.press(Action::Backup(0));
}

/// Vector 2's key loaded from strings A and C, its set known as a person
/// who typed them in would tell it (2 of 3, `name`, A, C and D), and Back
/// up opened on a single-key wallet over it.
fn with_set() -> Faraday {
    assert!(BIP93.contains("MS12NAMEDLL4F8JLH4E5VDVULDLFXU2JHDNLSM97XVENRXEG"));
    let mut app = on();
    app.press(Action::Entry(None));
    app.press(Action::EntryForm(2));
    for s in [A, C] {
        typed(&mut app, s);
        app.event(Event::Key(Key::Enter));
    }
    app.event(Event::Key(Key::Enter));
    assert_eq!(
        app.session.keys.len(),
        1,
        "{:?}",
        testkit::entry(&app).error
    );
    assert_eq!(app.session.keys[0].fingerprint.0, FP, "vector 2's key");
    app.session.keys.set_codex32(
        0,
        Some(SetFacts {
            threshold: 2,
            identifier: *b"name",
            length: 48,
            unknown: 0,
            strings: vec![
                (b'a', State::Made),
                (b'c', State::Made),
                (b'd', State::Made),
            ],
        }),
    );
    wallet_and_backup(&mut app);
    app
}

fn c32(app: &mut Faraday, p: Press) {
    app.press(Action::BC32(p));
}

fn type_in(app: &mut Faraday, s: &str) {
    for ch in s[3..].chars() {
        c32(app, Press::Key(OskPress::Char(ch)));
    }
}

/// The checklist made and its codex32 item open.
fn checklist_at_codex32(app: &mut Faraday) {
    app.press(Action::BChecklist);
    let n = bstep::of(plan::Item::Codex32(0));
    let c = testkit::compute();
    assert!(app.backup_items_now(&c).contains(&n), "the codex32 item");
    // The blank templates, printed from Faraday Starter, come first.
    if app.backup().unwrap().open == Some(bstep::BLANK) {
        app.press(Action::BNext(bstep::BLANK));
    }
    if app.backup().unwrap().open != Some(n) {
        app.press(Action::BStep(n));
    }
    assert_eq!(app.backup().unwrap().open, Some(n));
}

fn found(app: &Faraday) -> Option<Found> {
    app.backup().unwrap().c32.as_ref().unwrap().found.clone()
}

#[test]
fn a_key_with_a_set_opens_on_one_string_per_place_on_paper() {
    let app = with_set();
    let (shape, a) = app.board_plan().expect("the board");
    assert!(!a.custom, "read-only until Customize backup");
    let c = testkit::compute();
    let m = plan::map(&c, &shape, &a);
    for (p, j) in [ix('a'), ix('c'), ix('d')].into_iter().enumerate() {
        let held = plan::holds_at(&m, At::Place(p));
        assert!(
            held.contains(&What::Codex32Share(0, j)),
            "place {p}: {held:?}"
        );
        assert!(held.contains(&What::Sheet), "the description at place {p}");
        assert!(!held.iter().any(|w| matches!(w, What::Words(_))));
        assert_eq!(
            held.iter()
                .filter(|w| plan::codex32::string_of(**w).is_some())
                .count(),
            1,
            "one string per place"
        );
    }
    let check = plan::check(&c, &shape, &a);
    assert_eq!(
        check.lost,
        plan::Lost::Yes,
        "any place lost, the other two rebuild it"
    );
    assert_eq!(check.spend, plan::Found::No, "no place alone spends");
}

#[test]
fn each_string_typed_back_is_checked_against_the_set_and_two_rebuild_the_key() {
    let mut app = with_set();
    checklist_at_codex32(&mut app);
    c32(&mut app, Press::Open(ix('a') as u8));
    // A string of another set: its header names it.
    type_in(&mut app, OTHER);
    assert!(
        matches!(found(&app), Some(Found::Differs(l)) if l.contains("Box 4")),
        "{:?}",
        found(&app)
    );
    for _ in 0..45 {
        c32(&mut app, Press::Key(OskPress::Back));
    }
    // A box mistyped: the checksum fails.
    let mut wrong = A.to_string();
    wrong.replace_range(20..21, "q");
    type_in(&mut app, &wrong);
    assert_eq!(found(&app), Some(Found::Differs("Checksum fails".into())));
    for _ in 0..45 {
        c32(&mut app, Press::Key(OskPress::Back));
    }
    type_in(&mut app, A);
    assert!(
        matches!(found(&app), Some(Found::Holds(l)) if l == "A · checksum holds, a string of name"),
        "{:?}",
        found(&app)
    );
    c32(&mut app, Press::Next);
    // The next string on paper opens; C typed back, then the two rebuild
    // this key.
    assert_eq!(
        app.backup().unwrap().c32.as_ref().unwrap().at,
        Some(ix('c'))
    );
    type_in(&mut app, C);
    c32(&mut app, Press::Next);
    assert_eq!(
        app.backup().unwrap().c32.as_ref().unwrap().rebuilds,
        Some(true)
    );
    assert_eq!(
        app.backup().unwrap().c32.as_ref().unwrap().at,
        Some(ix('d'))
    );
    type_in(&mut app, D);
    let n = bstep::of(plan::Item::Codex32(0));
    // Typed back, the string waits for its Continue; then the item, done,
    // moves on.
    assert_eq!(app.backup().unwrap().open, Some(n));
    c32(&mut app, Press::Next);
    let c = testkit::compute();
    assert!(app.backup_item_done_now(&c, plan::Item::Codex32(0)));
    assert_ne!(app.backup().unwrap().open, Some(n), "a done item moves on");
}

#[test]
fn a_string_skipped_counts_only_after_the_caution() {
    let mut app = with_set();
    checklist_at_codex32(&mut app);
    c32(&mut app, Press::Open(ix('a') as u8));
    c32(&mut app, Press::Skip);
    assert!(app.backup().unwrap().c32.as_ref().unwrap().skip_ask);
    assert!(!app.backup().unwrap().c32_checked.contains(&(0, ix('a'))));
    c32(&mut app, Press::Skip);
    assert!(app.backup().unwrap().c32_checked.contains(&(0, ix('a'))));
}

/// Test key 1 and the Spending wallet over it, Back up open.
fn words_key() -> Faraday {
    let mut app = on();
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    app.storage(StorageEvent::Restored {
        inbox: vec![("spending-wallet.txt".into(), kit.descriptor.into_bytes())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
    app.press(Action::Entry(None));
    typed(&mut app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
    app.press(Action::EntryAdd);
    app.press(Action::Backup(0));
    app
}

/// A key of words is never offered codex32 strings: they would hold its
/// 64-byte seed with the passphrase merged in and give back neither the
/// words nor the passphrase. A key whose secret is a codex32 set keeps its
/// strings on the board.
#[test]
fn a_key_of_words_is_not_offered_codex32_strings() {
    let drawn = |app: &mut Faraday| {
        board(app, BoardPress::Customize);
        let _ = app.frame();
        app.drawn_texts()
    };
    let mut app = words_key();
    let texts = drawn(&mut app);
    assert!(
        !texts.iter().any(|t| t.to_lowercase().contains("codex32")),
        "{texts:?}"
    );
    let (_, a) = app.board_plan().unwrap();
    assert!(a.c32(0).is_none() && a.codex32.is_empty());
    let mut app = with_set();
    let texts = drawn(&mut app);
    assert!(
        texts.iter().any(|t| t.contains("codex32 name")),
        "the set's group: {texts:?}"
    );
}

/// A plan saved when a seed could be split into codex32 strings the plan
/// made reads back without that split: its form and strings are dropped
/// and the rest of the plan is as saved.
#[test]
fn a_saved_plan_with_a_split_the_plan_was_to_make_reads_without_it() {
    let app = words_key();
    let (shape, a) = app.board_plan().unwrap();
    let mut text = a.to_text();
    text.push_str("codex32 0 2 127 acd - 0\n");
    let back = plan::Answers::from_text(&shape, &text).expect("reads");
    assert!(back.c32(0).is_none());
    assert_eq!(
        back,
        plan::Answers::from_text(&shape, &a.to_text()).unwrap()
    );
}

#[test]
fn the_templates_item_names_the_codex32_sheet() {
    let app = with_set();
    let c = testkit::compute();
    let (shape, a) = app.board_plan().unwrap();
    let mapped = plan::Mapped::of(&c, &shape, &a);
    assert_eq!(
        plan::codex32::templates_of(&a, &mapped),
        vec![(48, plan::codex32::PaperCopy::String, 3)]
    );
    assert_eq!(plan::templates(&c, &shape, &a), 0, "no words on paper");
}

#[test]
fn a_lost_share_is_made_again_exactly_and_a_new_one_added() {
    let mut app = with_set();
    checklist_at_codex32(&mut app);
    c32(&mut app, Press::Set(SetPress::Replace(ix('d') as u8)));
    type_in(&mut app, A);
    type_in(&mut app, C);
    c32(&mut app, Press::Set(SetPress::MakeNew));
    let st = app.backup().unwrap().c32.as_ref().unwrap();
    assert_eq!(st.made(ix('d')), Some(D), "the very string that was lost");
    assert_eq!(st.at, Some(ix('d')), "opened to write and type back");
    // Add a share: the next unused index, E.
    c32(&mut app, Press::List);
    c32(&mut app, Press::Set(SetPress::Add));
    type_in(&mut app, C);
    type_in(&mut app, D);
    c32(&mut app, Press::Set(SetPress::MakeNew));
    let st = app.backup().unwrap().c32.as_ref().unwrap();
    let e = st.made(ix('e')).expect("share E").to_string();
    assert_eq!(&e[..9], "ms12namee");
    let (_, a) = app.board_plan().unwrap();
    assert!(
        a.c32(0).unwrap().indices.contains(&ix('e')),
        "the set gains E"
    );
    let c = testkit::compute();
    let parts = [
        faraday_bip::codex32::Codex32::parse(&e).unwrap(),
        faraday_bip::codex32::Codex32::parse(A).unwrap(),
    ];
    assert_eq!(app.session.keys.codex32_rebuilds(&c, 0, &parts), Some(true));
}

#[test]
fn rotating_says_what_to_destroy_and_refuses_when_too_many_are_out() {
    let mut app = with_set();
    checklist_at_codex32(&mut app);
    let step = |app: &Faraday| -> String {
        format!("{:?}", app.backup().unwrap().c32.as_ref().unwrap().set)
    };
    // Two of a 2 of 3 out of reach: the seed is to be treated as known.
    c32(&mut app, Press::Set(SetPress::Rotate));
    c32(&mut app, Press::Set(SetPress::Mark(ix('a') as u8, 1)));
    c32(&mut app, Press::Set(SetPress::Mark(ix('c') as u8, 2)));
    c32(&mut app, Press::Set(SetPress::Counted));
    let note = app
        .backup()
        .unwrap()
        .c32
        .as_ref()
        .unwrap()
        .note
        .clone()
        .unwrap();
    assert_eq!(
        note,
        "2 strings of name may be in other hands: rotating is not enough; move the coins to a new key"
    );
    assert!(step(&app).starts_with("Marking"), "rotation not offered");
    // One lost: A and D to destroy, then a new set.
    c32(&mut app, Press::Set(SetPress::Mark(ix('a') as u8, 0)));
    c32(&mut app, Press::Set(SetPress::Mark(ix('c') as u8, 1)));
    c32(&mut app, Press::Set(SetPress::Counted));
    let note = app
        .backup()
        .unwrap()
        .c32
        .as_ref()
        .unwrap()
        .note
        .clone()
        .unwrap();
    assert!(
        note.starts_with("Destroy A, D. Then at most C is left"),
        "{note}"
    );
    c32(&mut app, Press::Set(SetPress::Destroyed(ix('a') as u8)));
    assert!(
        step(&app).starts_with("Destroying"),
        "not before every one is"
    );
    c32(&mut app, Press::Set(SetPress::Destroyed(ix('d') as u8)));
    assert!(matches!(
        app.backup().unwrap().c32.as_ref().unwrap().set,
        SetStep::Scheme(2, 3)
    ));
    // The old identifier is refused; a new one makes the set.
    for ch in "name".chars() {
        c32(&mut app, Press::Key(OskPress::Char(ch)));
    }
    c32(&mut app, Press::Set(SetPress::MakeNew));
    assert!(app.backup().unwrap().c32.as_ref().unwrap().note.is_some());
    for _ in 0..4 {
        c32(&mut app, Press::Key(OskPress::Back));
    }
    for ch in "w0za".chars() {
        c32(&mut app, Press::Key(OskPress::Char(ch)));
    }
    c32(&mut app, Press::Set(SetPress::MakeNew));
    let facts = app.session.keys[0].codex32.clone().unwrap();
    assert_eq!(&facts.identifier, b"w0za");
    let st = app.backup().unwrap().c32.as_ref().unwrap();
    let new: Vec<String> = facts
        .strings
        .iter()
        .filter_map(|&(c, _)| {
            st.made(plan::codex32::index_of(c).unwrap())
                .map(str::to_string)
        })
        .collect();
    assert_eq!(new.len(), 3);
    assert!(new.iter().all(|s| &s[3..8] == "2w0za"));
    let c = testkit::compute();
    let parts: Vec<_> = new[1..]
        .iter()
        .map(|s| faraday_bip::codex32::Codex32::parse(s).unwrap())
        .collect();
    assert_eq!(
        app.session.keys.codex32_rebuilds(&c, 0, &parts),
        Some(true),
        "the same seed"
    );
    assert!(
        app.backup().unwrap().c32_checked.is_empty(),
        "the new strings are to check"
    );
}

#[test]
fn the_checks_count_strings_and_name_a_place_that_keeps_enough() {
    let mut app = with_set();
    board(&mut app, BoardPress::Customize);
    // C moved beside A: that place keeps the seed; losing it leaves too few.
    crate::common::move_to(
        &mut app,
        Zone::Paper(1),
        Thing::String(0, ix('c')),
        Zone::Paper(0),
    );
    let c = testkit::compute();
    let (shape, a) = app.board_plan().unwrap();
    let alone = plan::alone(&c, &shape, &a, At::Place(0));
    assert_eq!(
        alone.spend,
        plan::Found::Yes,
        "two strings of a 2 of 3 are the seed"
    );
    assert_eq!(plan::check(&c, &shape, &a).lost, plan::Lost::No);
    // A string kept nowhere is said.
    crate::common::take_off(&mut app, Zone::Paper(0), Thing::String(0, ix('c')));
    let (shape, a) = app.board_plan().unwrap();
    assert_eq!(plan::check(&c, &shape, &a).spend, plan::Found::No);
    let why = plan::lost_why(&c, &shape, &a, &|p| format!("Place {}", p + 1), "stick");
    assert!(why.is_some());
}

#[test]
fn a_plan_with_strings_reads_back_from_its_text() {
    let app = with_set();
    let (shape, a) = app.board_plan().unwrap();
    let text = a.to_text();
    assert!(text.contains("codex32 0 2 48 acd name 0"), "{text}");
    assert!(
        text.contains(&format!("spot place0 x0.{}", ix('a'))),
        "{text}"
    );
    let back = plan::Answers::from_text(&shape, &text).expect("reads");
    assert_eq!(back.codex32, a.codex32);
    let c = testkit::compute();
    assert_eq!(plan::map(&c, &shape, &back), plan::map(&c, &shape, &a));
}

/// The window at `width` × `height`, as a display reports it.
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

/// The checklist's "How many strings of name are there?" can be read
/// whole: neither its − nor its + button is pressed on any of it, on a
/// laptop's window and on a small one.
#[test]
fn how_many_strings_is_clear_of_its_minus_and_plus_buttons() {
    let label = "How many strings of name are there?";
    for (w, h) in [(1280, 800), (480, 640)] {
        let mut app = with_set();
        sized(&mut app, w, h);
        checklist_at_codex32(&mut app);
        let minus = Action::BC32(Press::Exist(-1));
        let plus = Action::BC32(Press::Exist(1));
        // Scrolled down until the row is on screen.
        for _ in 0..20 {
            let _ = app.frame();
            if app.hit_box(minus).is_some() && app.hit_box(plus).is_some() {
                break;
            }
            app.event(Event::Scroll {
                x: w / 2,
                y: h / 2,
                dy: 120,
            });
        }
        let _ = app.frame();
        let boxes = app.drawn_text_boxes();
        let text = boxes
            .iter()
            .find(|(t, _)| t == label)
            .map(|(_, b)| *b)
            .unwrap_or_else(|| panic!("{label} not drawn at {w}x{h}: {boxes:?}"));
        for a in [minus, plus] {
            let b = app
                .hit_box(a)
                .unwrap_or_else(|| panic!("{a:?} not drawn at {w}x{h}"));
            let b = (
                i32::from(b.0),
                i32::from(b.1),
                i32::from(b.2),
                i32::from(b.3),
            );
            let apart = text.0 + text.2 <= b.0
                || b.0 + b.2 <= text.0
                || text.1 + text.3 <= b.1
                || b.1 + b.3 <= text.1;
            assert!(
                apart,
                "at {w}x{h} the label {text:?} is under {a:?} at {b:?}"
            );
        }
    }
}

/// A string typed back is the string made here only when it is the
/// whole of it: one that stops short, or runs on past its end, differs.
#[test]
fn a_string_typed_back_short_or_long_is_not_the_one_made() {
    let form = plan::codex32::C32 {
        seed: 0,
        k: 2,
        length: 48,
        indices: vec![ix('a'), ix('c'), ix('d')],
        identifier: *b"name",
        unknown: 0,
    };
    let check = faraday_core::codex32_backup::check;
    assert!(matches!(check(&form, ix('a'), A, Some(A)), Found::Holds(_)));
    assert!(matches!(
        check(&form, ix('a'), &A[..40], Some(A)),
        Found::Differs(_)
    ));
    let longer = format!("{A}q");
    assert!(matches!(
        check(&form, ix('a'), &longer, Some(A)),
        Found::Differs(_)
    ));
}
