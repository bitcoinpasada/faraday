//! Codex32 in a backup (`docs/CODEX32.md` §9), as a person backing up a
//! wallet would notice it, against BIP 93's vector 2
//! (`tools/vectors/bip93/vectors.json`): a key with a set opens on one
//! string per place; the checklist checks each string typed back, its
//! header against the set and, once two are in, that they rebuild the
//! key; a set the plan makes is made, placed and typed back; a lost share
//! is made again exactly, a new one added, and a set rotated only once the
//! held strings are destroyed; the checks count strings, and the
//! templates name their sheet.

#![expect(
    clippy::disallowed_methods,
    reason = "a test or example builds keys and seeds directly and is not shipped (docs/KEYS.md)"
)]

use faraday_bip::keys::ScriptType;
use faraday_core::board::{BoardPress, Thing, Zone};
use faraday_core::codex32_backup::{Found, Press, SetPress, SetStep};
use faraday_core::plan::{self, At, What};
use faraday_core::{Action, Faraday, StorageEvent, testkit};
use faraday_keys::texts::Spec;
use faraday_shell_api::{
    App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware, TouchPhase,
};
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

/// Vector 3's shares, threshold 3, and its secret.
const A3: &str = "ms13casha320zyxwvutsrqpnmlkjhgfedca2a8d0zehn8a0t";
const C3: &str = "ms13cashcacdefghjklmnpqrstuvwxyz023949xq35my48dr";
const D3: &str = "ms13cashd0wsedstcdcts64cd7wvy4m90lm28w4ffupqs7rm";
const E3: &str = "ms13casheekgpemxzshcrmqhaydlp6yhms3ws7320xyxsar9";
const F3: &str = "ms13cashf8jh6sdrkpyrsp5ut94pj8ktehhw2hfvyrj48704";

const BIP93: &str = include_str!("../../../../tools/vectors/bip93/vectors.json");

/// A share index's bech32 value.
fn ix(c: char) -> usize {
    plan::codex32::index_of(c as u8).unwrap()
}

/// A desktop with its display told and no seed from the system yet.
pub(crate) fn on_unseeded() -> Faraday {
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
    app
}

fn on() -> Faraday {
    let mut app = on_unseeded();
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
        inbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("shares-wallet.txt"),
            format!("wpkh({}/<0;1>/*)", &*text).into_bytes(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    testkit::load_wallet(app, 0);
    app.press(Action::Backup(0));
}

/// Vector 2's key loaded from strings A and C, its set known as a person
/// who typed them in would tell it (2 of 3, `name`, A, C and D), and Back
/// up opened on a single-key wallet over it.
fn with_set() -> Faraday {
    with_set_on(on())
}

/// [`with_set`] on `app`, which may not have its seed yet.
pub(crate) fn with_set_on(app: Faraday) -> Faraday {
    assert!(BIP93.contains("MS12NAMEDLL4F8JLH4E5VDVULDLFXU2JHDNLSM97XVENRXEG"));
    let facts = SetFacts {
        threshold: 2,
        identifier: *b"name",
        length: 48,
        unknown: 0,
        strings: vec![
            (b'a', State::Made),
            (b'c', State::Made),
            (b'd', State::Made),
        ],
    };
    let app = with_strings_on(app, &[A, C], facts);
    assert_eq!(app.session.keys[0].fingerprint.0, FP, "vector 2's key");
    app
}

/// A key loaded from the codex32 `strings` typed in, its set known as
/// `facts`, and Back up opened on a single-key wallet over it.
fn with_strings_on(mut app: Faraday, strings: &[&str], facts: SetFacts) -> Faraday {
    app.press(Action::Entry(None));
    app.press(Action::EntryForm(2));
    for s in strings {
        typed(&mut app, s);
        app.event(Event::Key(Key::Enter));
    }
    app.event(Event::Key(Key::Enter));
    faraday_core::testkit::keep_published(&mut app);
    assert_eq!(
        app.session.keys.len(),
        1,
        "{:?}",
        testkit::entry(&app).error
    );
    app.session.keys.set_codex32(0, Some(facts));
    wallet_and_backup(&mut app);
    app
}

/// Vector 3's set: 3 of 5, `cash`, strings A, C, D, E and F.
fn with_three_of_five() -> Faraday {
    let facts = SetFacts {
        threshold: 3,
        identifier: *b"cash",
        length: 48,
        unknown: 0,
        strings: b"acdef".iter().copied().map(|j| (j, State::Made)).collect(),
    };
    with_strings_on(on(), &[A3, C3, D3], facts)
}

pub(crate) fn c32(app: &mut Faraday, p: Press) {
    app.press(Action::BC32(p));
}

fn type_in(app: &mut Faraday, s: &str) {
    for ch in s[3..].chars() {
        c32(app, Press::Key(OskPress::Char(ch)));
    }
}

/// The checklist made and its codex32 item open.
pub(crate) fn checklist_at_codex32(app: &mut Faraday) {
    app.press(Action::BChecklist);
    let n = plan::Item::Codex32(0);
    let c = testkit::compute();
    assert!(app.backup_items_now(&c).contains(&n), "the codex32 item");
    // The blank templates, printed from Faraday Starter, come first.
    if app.backup().unwrap().open == Some(faraday_core::plan::Item::Templates) {
        app.press(Action::BNext(faraday_core::plan::Item::Templates));
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
            held.contains(&What::codex32_share(0, j)),
            "place {p}: {held:?}"
        );
        assert!(held.contains(&What::Sheet), "the description at place {p}");
        assert!(!held.iter().any(|w| w.is_words()));
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
    assert_eq!(
        found(&app),
        Some(Found::Differs(faraday_text::Text::of("Checksum fails")))
    );
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
    let n = plan::Item::Codex32(0);
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
        inbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("spending-wallet.txt"),
            kit.descriptor.into_bytes(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    testkit::load_wallet(&mut app, 0);
    app.press(Action::Entry(None));
    typed(&mut app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
    app.press(Action::EntryAdd);
    faraday_core::testkit::keep_published(&mut app);
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
        vec![(48, plan::codex32::PaperCopy::Written, 3)]
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
        "{}",
        &*note
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
    let why = plan::lost_why(
        &c,
        &shape,
        &a,
        &|p| faraday_text::text!("Place {}", p + 1),
        "stick",
    );
    assert!(why.is_some());
}

#[test]
fn a_plan_with_strings_reads_back_from_its_text() {
    let app = with_set();
    let (shape, a) = app.board_plan().unwrap();
    let text = String::from(&*a.to_text());
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

/// Strings of another 2-of-3 set with vector 2's header, their checksums
/// holding: made on another device.
const FOREIGN_C: &str = "ms12namec9ptn2r8ckcgunuyzs6w7xdq0q0lzskhv4xh9xtz";
const FOREIGN_D: &str = "ms12named9ptn2r8ckcgunuyzs6w7xdq0q0fm5juw03ls5xu";

fn typed_next(app: &mut Faraday, j: char, s: &str) {
    c32(app, Press::Open(ix(j) as u8));
    type_in(app, s);
    c32(app, Press::Next);
}

#[test]
fn a_string_of_another_set_with_the_same_header_is_not_counted_checked() {
    let mut app = with_set();
    checklist_at_codex32(&mut app);
    typed_next(&mut app, 'a', A);
    typed_next(&mut app, 'c', C);
    assert!(app.backup().unwrap().c32_checked.contains(&(0, ix('c'))));
    // The header and checksum hold, but it is not of this key's set.
    typed_next(&mut app, 'd', FOREIGN_D);
    let b = app.backup().unwrap();
    assert!(!b.c32_checked.contains(&(0, ix('d'))));
    assert_eq!(b.c32.as_ref().unwrap().rebuilds, Some(false));
    let c = testkit::compute();
    assert!(!app.backup_item_done_now(&c, plan::Item::Codex32(0)));
}

#[test]
fn strings_that_do_not_rebuild_the_key_are_none_of_them_counted() {
    let mut app = with_set();
    checklist_at_codex32(&mut app);
    typed_next(&mut app, 'a', A);
    typed_next(&mut app, 'c', FOREIGN_C);
    let b = app.backup().unwrap();
    assert!(b.c32_checked.is_empty(), "{:?}", b.c32_checked);
    assert_eq!(b.c32.as_ref().unwrap().rebuilds, Some(false));
    // Typed again right, the two are counted.
    typed_next(&mut app, 'a', A);
    typed_next(&mut app, 'c', C);
    let b = app.backup().unwrap();
    assert!(b.c32_checked.contains(&(0, ix('a'))));
    assert!(b.c32_checked.contains(&(0, ix('c'))));
    assert_eq!(b.c32.as_ref().unwrap().rebuilds, Some(true));
}

fn skipped(app: &Faraday) -> Vec<(usize, usize)> {
    app.backup().unwrap().c32_skipped.clone()
}

/// The string the foot's Check my copy opens, once pressed where it is
/// drawn.
fn check_my_copy_opens(app: &mut Faraday) -> Option<usize> {
    c32(app, Press::List);
    // The foot may be below the fold: scrolled down to.
    let mut found = None;
    for _ in 0..30 {
        let _ = app.frame();
        found = app
            .drawn_text_boxes()
            .into_iter()
            .find(|(t, _)| t == "Check my copy");
        if found.is_some() {
            break;
        }
        app.event(Event::Scroll {
            x: 600,
            y: 400,
            dy: 120,
        });
    }
    let (_, r) = found.expect("Check my copy is drawn");
    let (x, y) = (
        u16::try_from(r.0 + r.2 / 2).ok()?,
        u16::try_from(r.1 + r.3 / 2).ok()?,
    );
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    app.settle();
    let at = app.backup()?.c32.as_ref()?.at;
    let c = app.hit_box(Action::BC32(Press::Open(ix('c') as u8)));
    assert!(at.is_some(), "tapped {x},{y} on {r:?}; Open(c) is at {c:?}");
    at
}

#[test]
fn a_string_skipped_twice_reads_skipped_and_one_typed_after_reads_checked() {
    let mut app = with_set();
    checklist_at_codex32(&mut app);
    c32(&mut app, Press::Open(ix('a') as u8));
    c32(&mut app, Press::Skip);
    c32(&mut app, Press::Skip);
    assert_eq!(skipped(&app), vec![(0, ix('a'))]);
    c32(&mut app, Press::List);
    let _ = app.frame();
    assert!(app.drawn_texts().iter().any(|t| t == "Skipped"));
    assert!(!app.backup().unwrap().c32_skipped.is_empty());
    typed_next(&mut app, 'c', C);
    typed_next(&mut app, 'd', D);
    typed_next(&mut app, 'a', A);
    assert!(skipped(&app).is_empty());
    assert!(app.backup().unwrap().c32_checked.contains(&(0, ix('a'))));
    c32(&mut app, Press::List);
    let _ = app.frame();
    assert!(!app.drawn_texts().iter().any(|t| t == "Skipped"));
}

#[test]
fn a_string_typed_twice_still_counts_with_the_ones_after_it() {
    let mut app = with_three_of_five();
    checklist_at_codex32(&mut app);
    typed_next(&mut app, 'a', A3);
    typed_next(&mut app, 'a', A3);
    for (j, s) in [('c', C3), ('d', D3), ('e', E3)] {
        typed_next(&mut app, j, s);
    }
    let b = app.backup().unwrap();
    for j in ['a', 'c', 'd', 'e'] {
        assert!(b.c32_checked.contains(&(0, ix(j))), "{j} not checked");
    }
    assert_eq!(b.c32.as_ref().unwrap().rebuilds, Some(true));
}

#[test]
fn check_my_copy_does_not_open_a_string_waiting_for_others() {
    let mut app = with_three_of_five();
    checklist_at_codex32(&mut app);
    typed_next(&mut app, 'a', A3);
    let opens = check_my_copy_opens(&mut app);
    assert_eq!(
        opens,
        Some(ix('c')),
        "the string typed waits; the next opens"
    );
}

#[test]
fn leaving_a_set_action_keeps_what_was_proved_and_a_foreign_string_does_not_spoil_it() {
    let mut app = with_set();
    checklist_at_codex32(&mut app);
    typed_next(&mut app, 'a', A);
    typed_next(&mut app, 'c', C);
    c32(&mut app, Press::Set(SetPress::Add));
    c32(&mut app, Press::Set(SetPress::Leave));
    // A string of another set does not rebuild the key and is not kept.
    typed_next(&mut app, 'c', FOREIGN_C);
    assert_eq!(
        app.backup().unwrap().c32.as_ref().unwrap().rebuilds,
        Some(false)
    );
    // The strings proved before the action still prove the next.
    typed_next(&mut app, 'd', D);
    let b = app.backup().unwrap();
    assert!(b.c32_checked.contains(&(0, ix('d'))));
    assert_eq!(b.c32.as_ref().unwrap().rebuilds, Some(true));
}

/// What the list reads for the strings of the open seed.
fn list_reads(app: &mut Faraday) -> String {
    c32(app, Press::List);
    let _ = app.frame();
    app.drawn_texts().join(" ")
}

#[test]
fn a_share_added_after_three_strings_were_checked_leaves_the_fourth_to_read_checked() {
    let mut app = with_three_of_five();
    checklist_at_codex32(&mut app);
    for (j, s) in [('a', A3), ('c', C3), ('d', D3)] {
        typed_next(&mut app, j, s);
    }
    assert!(app.backup().unwrap().c32_checked.contains(&(0, ix('d'))));
    // Add a share, made from three strings and typed back.
    c32(&mut app, Press::List);
    c32(&mut app, Press::Set(SetPress::Add));
    for s in [A3, C3, D3] {
        type_in(&mut app, s);
    }
    c32(&mut app, Press::Set(SetPress::MakeNew));
    let at = app
        .backup()
        .unwrap()
        .c32
        .as_ref()
        .unwrap()
        .at
        .expect("opened");
    let made = app
        .backup()
        .unwrap()
        .c32
        .as_ref()
        .unwrap()
        .made(at)
        .unwrap()
        .to_string();
    c32(&mut app, Press::Open(at as u8));
    type_in(&mut app, &made);
    c32(&mut app, Press::Next);
    // The string typed next reads Checked at once.
    typed_next(&mut app, 'e', E3);
    assert!(app.backup().unwrap().c32_checked.contains(&(0, ix('e'))));
}

#[test]
fn leaving_a_set_action_with_nothing_done_keeps_what_was_checked() {
    let mut app = with_three_of_five();
    checklist_at_codex32(&mut app);
    for (j, s) in [('a', A3), ('c', C3), ('d', D3)] {
        typed_next(&mut app, j, s);
    }
    c32(&mut app, Press::List);
    c32(&mut app, Press::Set(SetPress::Add));
    c32(&mut app, Press::Set(SetPress::Leave));
    typed_next(&mut app, 'e', E3);
    assert!(app.backup().unwrap().c32_checked.contains(&(0, ix('e'))));
}

/// A 16-byte master seed of its own, not a public vector's.
const OWN: [u8; 16] = [
    0x5a, 0x17, 0x93, 0x0c, 0xe2, 0x41, 0xb8, 0x6d, 0x24, 0xf1, 0x0e, 0x9b, 0x73, 0xc6, 0x38, 0xad,
];

/// The strings of the 2-of-3 set of [`OWN`], by index.
fn own_set() -> Vec<(char, String)> {
    let own = faraday_bip::codex32::Codex32::from_seed(2, *b"tree", &OWN).unwrap();
    own.split(2, 3, &[0x37; 64])
        .unwrap()
        .iter()
        .map(|x| x.encode().as_str().to_string())
        .map(|s| (s.as_bytes()[8] as char, s))
        .collect()
}

/// A wallet of two codex32 seeds: a 2-of-3 set of a seed of its own,
/// then vector 3's 3-of-5 set, both typed in, over a 2-of-2, Back up
/// open.
fn with_two_seeds() -> Faraday {
    let mut app = on();
    let own = own_set();
    for strings in [vec![own[0].1.as_str(), own[1].1.as_str()], vec![A3, C3, D3]] {
        app.press(Action::Entry(None));
        app.press(Action::EntryForm(2));
        for s in strings {
            typed(&mut app, s);
            app.event(Event::Key(Key::Enter));
        }
        app.event(Event::Key(Key::Enter));
    }
    faraday_core::testkit::keep_published(&mut app);
    assert_eq!(
        app.session.keys.len(),
        2,
        "{:?}",
        testkit::entry(&app).error
    );
    app.session.keys.set_codex32(
        1,
        Some(SetFacts {
            threshold: 3,
            identifier: *b"cash",
            length: 48,
            unknown: 0,
            strings: b"acdef".iter().copied().map(|j| (j, State::Made)).collect(),
        }),
    );
    app.session.keys.set_codex32(
        0,
        Some(SetFacts {
            threshold: 2,
            identifier: *b"tree",
            length: 48,
            unknown: 0,
            strings: b"acd".iter().copied().map(|j| (j, State::Made)).collect(),
        }),
    );
    let c = testkit::compute();
    let key = |i| {
        app.session
            .keys
            .key_text(
                &c,
                i,
                &Spec::Multisig(faraday_bip::keys::MultisigScriptType::NativeSegwit, 0),
            )
            .unwrap()
            .unwrap()
    };
    let descriptor = format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*))",
        &*key(0),
        &*key(1)
    );
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("two-seeds.txt"),
            descriptor.into_bytes(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    testkit::load_wallet(&mut app, 0);
    app.press(Action::Backup(0));
    app
}

/// Seed `i`'s codex32 item open, the checklist made.
fn open_seed(app: &mut Faraday, i: usize) {
    let n = plan::Item::Codex32(i);
    let c = testkit::compute();
    assert!(
        app.backup_items_now(&c).contains(&n),
        "the codex32 item {i}"
    );
    if app.backup().unwrap().open == Some(faraday_core::plan::Item::Templates) {
        app.press(Action::BNext(faraday_core::plan::Item::Templates));
    }
    if app.backup().unwrap().open != Some(n) {
        app.press(Action::BStep(n));
    }
    assert_eq!(app.backup().unwrap().open, Some(n));
}

/// Seed 0, a 2-of-3 set, done: its three strings typed back.
fn first_seed_done(app: &mut Faraday) {
    app.press(Action::BChecklist);
    open_seed(app, 0);
    for (j, s) in own_set() {
        typed_next(app, j, &s);
    }
}

#[test]
fn strings_checked_on_one_seed_still_count_after_another_seed_is_opened() {
    let mut app = with_two_seeds();
    first_seed_done(&mut app);
    open_seed(&mut app, 1);
    for (j, s) in [('a', A3), ('c', C3), ('d', D3)] {
        typed_next(&mut app, j, s);
    }
    open_seed(&mut app, 0);
    open_seed(&mut app, 1);
    typed_next(&mut app, 'e', E3);
    typed_next(&mut app, 'f', F3);
    let b = app.backup().unwrap();
    for j in ['a', 'c', 'd', 'e', 'f'] {
        assert!(b.c32_checked.contains(&(1, ix(j))), "{j} not checked");
    }
    // The last string checked moves the item on; the item opened again
    // reads the strings Checked on its list.
    open_seed(&mut app, 1);
    assert!(list_reads(&mut app).contains("Checked"));
    let c = testkit::compute();
    assert!(app.backup_item_done_now(&c, plan::Item::Codex32(1)));
}

#[test]
fn a_string_skipped_on_one_seed_still_reads_skipped_after_another_seed_is_opened() {
    let mut app = with_two_seeds();
    first_seed_done(&mut app);
    open_seed(&mut app, 1);
    c32(&mut app, Press::Open(ix('a') as u8));
    c32(&mut app, Press::Skip);
    c32(&mut app, Press::Skip);
    open_seed(&mut app, 0);
    open_seed(&mut app, 1);
    assert!(list_reads(&mut app).contains("Skipped"));
}

#[test]
fn a_replaced_share_reads_not_checked_until_typed_back_and_the_others_stay_checked() {
    let mut app = with_set();
    checklist_at_codex32(&mut app);
    typed_next(&mut app, 'a', A);
    typed_next(&mut app, 'c', C);
    c32(&mut app, Press::Set(SetPress::Replace(ix('d') as u8)));
    type_in(&mut app, A);
    type_in(&mut app, C);
    c32(&mut app, Press::Set(SetPress::MakeNew));
    let b = app.backup().unwrap();
    assert!(b.c32_checked.contains(&(0, ix('a'))));
    assert!(b.c32_checked.contains(&(0, ix('c'))));
    assert!(
        !b.c32_checked.contains(&(0, ix('d'))),
        "D reads Not checked"
    );
    typed_next(&mut app, 'd', D);
    assert!(app.backup().unwrap().c32_checked.contains(&(0, ix('d'))));
}

#[test]
fn a_set_action_rotated_then_left_keeps_what_was_proved_for_the_next_string() {
    let mut app = with_three_of_five();
    checklist_at_codex32(&mut app);
    for (j, s) in [('a', A3), ('c', C3), ('d', D3)] {
        typed_next(&mut app, j, s);
    }
    c32(&mut app, Press::List);
    c32(&mut app, Press::Set(SetPress::Rotate));
    c32(&mut app, Press::Set(SetPress::Leave));
    typed_next(&mut app, 'e', E3);
    assert!(app.backup().unwrap().c32_checked.contains(&(0, ix('e'))));
}
