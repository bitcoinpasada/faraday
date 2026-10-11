//! The first run (UX.md §7.1, job A4) and the row each Learn page ends
//! in: what a person with only this app sees from nothing to a
//! backed-up key and a checked address.

mod common;

use common::{ABANDON, Harness, PANEL};
use opensigner_core::create::Step as CreateStep;
use opensigner_core::quiz::QuizState;
use opensigner_core::{AssuranceTier, QuizView, ScreenKind, ids};
use osk_shell_api::{Command, Event, Key};

/// The settings the app last asked the shell to keep.
fn stored(h: &Harness) -> Option<Vec<u8>> {
    h.seen.iter().rev().find_map(|c| match c {
        Command::StoreSettings { bytes } => Some(bytes.clone()),
        _ => None,
    })
}

/// Home → Learn → page `i`.
fn open_page(h: &mut Harness, i: usize) {
    h.go_home();
    h.tap(ids::at(ids::HOME_TILE_BASE, 4));
    assert_eq!(h.app.screen(), ScreenKind::Learn);
    h.tap(ids::at(ids::LEARN_ROW_BASE, i));
    assert_eq!(h.app.screen(), ScreenKind::LearnPage);
}

/// The "Try it" row of page `i`, and where it goes.
fn try_row(h: &mut Harness, i: usize) {
    open_page(h, i);
    h.tap(ids::LEARN_TRY);
}

/// Create a 12-word key from 32 zero hex digits, which is "abandon …
/// about", and pass the quiz.
fn create_abandon(h: &mut Harness) {
    h.choose(
        ids::at(ids::CREATE_SOURCE_BASE, 2),
        ids::CREATE_SOURCE_CONTINUE,
    );
    h.choose(
        ids::at(ids::CREATE_COUNT_BASE, 0),
        ids::CREATE_COUNT_CONTINUE,
    );
    h.choose(ids::at(ids::CREATE_LANG_BASE, 0), ids::CREATE_LANG_CONTINUE);
    h.type_text("00000000000000000000000000000000");
    h.key(Key::Enter);
    assert_eq!(h.app.create_step(), Some(CreateStep::Sanity));
    h.tap(ids::CREATE_CONTINUE);
    assert_eq!(h.app.create_step(), Some(CreateStep::Words));
    h.tap(ids::CREATE_CONTINUE);
    assert_eq!(h.app.create_step(), Some(CreateStep::QuizStart));
    // §7.1: the first run has no way past the quiz.
    assert!(
        h.app.rect_of(ids::QUIZ_SKIP).is_none(),
        "the first run offers no Skip"
    );
    h.tap(ids::QUIZ_START);
    for _ in 0..ABANDON.len() {
        let v: QuizView = h.app.quiz_view().expect("a quiz on screen");
        assert_eq!(v.state, QuizState::Asking);
        let want = ABANDON[v.word_number - 1];
        let slot = v
            .choices
            .iter()
            .position(|c| c == want)
            .unwrap_or_else(|| panic!("word {} not offered", v.word_number));
        h.choose(ids::at(ids::QUIZ_CHOICE_BASE, slot), ids::QUIZ_CONTINUE);
    }
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    h.add_key();
}

#[test]
fn a_device_with_no_settings_opens_on_start_here() {
    let mut h = Harness::first_run(PANEL);
    assert_eq!(h.app.screen(), ScreenKind::StartHere);
    assert!(
        h.app.rect_of(ids::START_HERE_CONTINUE).is_some(),
        "the one action"
    );
    assert!(
        h.app.rect_of(ids::KEYS_LOAD).is_none(),
        "the list is not drawn behind it"
    );
    // The chevron ends the first run: Home is its list, and what the
    // shell was asked to keep says so.
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    h.open_keys();
    assert!(h.app.rect_of(ids::KEYS_LOAD).is_some());
    h.go_home();
    let kept = stored(&h).expect("the settings were kept");
    assert!(String::from_utf8_lossy(&kept).contains("first_run_done=on"));

    // The next start reads them back and opens on Home.
    let mut next = Harness::first_run(PANEL);
    next.send(Event::Settings { bytes: kept });
    assert_eq!(next.app.screen(), ScreenKind::Home);
}

#[test]
fn tier_d_keeps_nothing_so_start_here_comes_back_every_start() {
    let mut h = Harness::fresh(PANEL, AssuranceTier::D, true);
    assert_eq!(h.app.screen(), ScreenKind::StartHere);
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert!(stored(&h).is_none(), "Tier D persists nothing");
    assert_eq!(
        Harness::fresh(PANEL, AssuranceTier::D, true).app.screen(),
        ScreenKind::StartHere
    );
}

#[test]
fn continue_opens_add_and_the_created_key_lands_on_check_an_address() {
    let mut h = Harness::first_run(PANEL);
    h.tap(ids::START_HERE_CONTINUE);
    assert_eq!(h.app.screen(), ScreenKind::Add);
    h.tap(ids::KEYS_CREATE);
    assert_eq!(h.app.screen(), ScreenKind::Create);
    create_abandon(&mut h);
    assert_eq!(h.app.screen(), ScreenKind::Created);
    assert!(h.app.rect_of(ids::CREATED_CHECK).is_some());
    assert!(!h.app.first_run_done(), "nothing has been checked yet");
    h.tap(ids::CREATED_CHECK);
    assert_eq!(h.app.screen(), ScreenKind::Addresses);
    assert!(!h.app.first_run_done(), "the list has not been left");
    // Leaving the list is what ends the first run.
    h.tap(ids::BACK);
    assert!(h.app.first_run_done());
    assert!(
        String::from_utf8_lossy(&stored(&h).expect("kept")).contains("first_run_done=on"),
        "and it is kept"
    );
}

#[test]
fn a_loaded_key_ends_the_first_run_on_its_own() {
    let mut h = Harness::first_run(PANEL);
    h.tap(ids::START_HERE_CONTINUE);
    h.tap(ids::KEYS_LOAD);
    h.load_choices(0);
    for w in ABANDON {
        h.type_text(w);
        h.key(Key::Enter);
    }
    h.tap(ids::LOAD_CONTINUE);
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    h.add_key();
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert!(h.app.first_run_done(), "a loaded key has a backup already");
}

#[test]
fn settings_reopens_start_here_without_reopening_the_first_run() {
    let mut h = Harness::new(PANEL);
    assert!(h.app.first_run_done());
    h.open_settings();
    h.tap(ids::SETTINGS_START_HERE_ROW);
    assert_eq!(h.app.screen(), ScreenKind::StartHere);
    assert!(
        h.app.first_run_done(),
        "reading it again is not a first run"
    );
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Settings);
    assert!(h.app.first_run_done());
}

#[test]
fn start_here_is_the_first_page_of_learn() {
    let mut h = Harness::new(PANEL);
    open_page(&mut h, 0);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == "Start here"),
        "the first page is Start here: {texts:?}"
    );
}

#[test]
fn a_learn_page_with_a_flow_ends_in_the_row_that_opens_it() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    for (page, want) in [
        (1, ScreenKind::Create),
        (6, ScreenKind::BackupMenu),
        // The encrypted backup's own flow now begins with the form it
        // will leave the device in (§16.112).
        (7, ScreenKind::ExportForm),
        (9, ScreenKind::OpenPassphrase),
        (10, ScreenKind::Scan),
        (13, ScreenKind::Build),
        (14, ScreenKind::Tool),
        (15, ScreenKind::AddWallet),
        (19, ScreenKind::Scan),
    ] {
        try_row(&mut h, page);
        assert_eq!(h.app.screen(), want, "the row of Learn page {page}");
        h.go_home();
    }
    // Dice opens the same wizard past the step that chooses a source.
    try_row(&mut h, 4);
    assert_eq!(h.app.screen(), ScreenKind::Create);
    assert_eq!(h.app.create_step(), Some(CreateStep::Count));
}

#[test]
fn a_row_whose_flow_needs_a_key_opens_add_when_none_is_loaded() {
    let mut h = Harness::new(PANEL);
    for (page, want) in [
        (1, ScreenKind::Create),
        (4, ScreenKind::Create),
        (6, ScreenKind::Add),
        (7, ScreenKind::Add),
        (9, ScreenKind::Add),
        (10, ScreenKind::Scan),
        (13, ScreenKind::Build),
        (14, ScreenKind::Tool),
        (15, ScreenKind::AddWallet),
        (19, ScreenKind::Add),
    ] {
        try_row(&mut h, page);
        assert_eq!(h.app.screen(), want, "the row of Learn page {page}");
        h.go_home();
    }
}

#[test]
fn a_learn_page_with_no_flow_has_no_row() {
    let mut h = Harness::new(PANEL);
    for page in [0, 2, 3, 5, 8, 11, 12, 16, 17, 18, 20, 21] {
        open_page(&mut h, page);
        assert!(
            h.app.rect_of(ids::LEARN_TRY).is_none(),
            "Learn page {page} has no flow of its own"
        );
        h.go_home();
    }
}
