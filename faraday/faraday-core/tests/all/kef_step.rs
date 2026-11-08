//! A seed's Krux code on the backup checklist (`docs/KEF.md` §15.1): the
//! item is listed for a seed whose code is on a stick, reads Not made, Not
//! checked and Checked as the code is made and opened back with its
//! passphrase, and a wrong passphrase leaves it Not checked.

use crate::common::{board, open_item, take_off, type_text};
use faraday_core::board::{BoardPress as BP, Chip, Thing, Verb, Zone};
use faraday_core::kef_form::KefAct;
use faraday_core::kef_tool::KefToolAction as K;
use faraday_core::plan::{Item, Preset};
use faraday_core::seeds::SeedsAction as S;
use faraday_core::{Action, Faraday, bstep, qstep, testkit};
use faraday_shell_api::{App, EntropyBytes, Event, Key};

/// A one-key wallet whose plan keeps seed A's Krux code on a new stick,
/// at the checklist.
fn at_the_checklist() -> Faraday {
    let mut app = testkit::started();
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x40 + i; 32])));
    }
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    let fp = app.session.keys.last().unwrap().fingerprint;
    app.press(Action::KeyWallet(fp.0, 1));
    app.press(Action::Seeds(S::Make));
    let w = app.session.wallets.len() - 1;
    app.press(Action::Backup(w));
    testkit::backup_as(&mut app, Preset::Paper);
    board(&mut app, BP::Menu(Chip::List(Thing::Seed(0))));
    board(&mut app, BP::MenuVerb(Verb::Kef));
    board(&mut app, BP::MenuTo(Zone::NewStick(0)));
    // The words are kept in the vault of another backup: only the code.
    take_off(&mut app, Zone::Paper(0), Thing::Seed(0));
    app.press(Action::BQ(qstep::PLAN));
    app.press(Action::BChecklist);
    if app
        .backup_items_now(&testkit::compute())
        .contains(&bstep::BLANK)
    {
        app.press(Action::BNext(bstep::BLANK));
    }
    app.settle();
    app
}

fn done(app: &Faraday) -> bool {
    app.backup_item_done_now(&testkit::compute(), Item::KefCode(0))
}

/// The status the item's row reads once the card is put away, the card
/// opened again after.
fn status(app: &mut Faraday) -> &'static str {
    if app.backup().and_then(|b| b.open) == Some(bstep::KEF) {
        app.press(Action::BStep(bstep::KEF));
    }
    let texts = crate::common::texts(app);
    let row = texts
        .iter()
        .position(|t| t.starts_with("Krux code of"))
        .expect("the item's row");
    let read = ["Not made", "Not checked", "Checked"]
        .into_iter()
        .find(|s| texts[row + 1..].iter().take(3).any(|t| t == s))
        .unwrap_or("none");
    open_item(app, bstep::KEF);
    read
}

fn type_in(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

fn make(app: &mut Faraday) {
    app.press(Action::KefTool(K::Backup(KefAct::Options)));
    app.press(Action::KefTool(K::Backup(KefAct::Rounds)));
    for _ in 0..8 {
        app.event(Event::Key(Key::Backspace));
    }
    type_in(app, "10000");
    app.press(Action::KefTool(K::Backup(KefAct::Back)));
    app.press(Action::KefTool(K::Backup(KefAct::Field)));
    type_in(app, "a passphrase");
    app.press(Action::KefTool(K::Backup(KefAct::Make)));
    testkit::settle(app);
}

fn check_with(app: &mut Faraday, passphrase: &str) {
    app.press(Action::KefTool(K::Backup(KefAct::Check)));
    type_in(app, passphrase);
    app.press(Action::KefTool(K::Backup(KefAct::Compare)));
    testkit::settle(app);
}

#[test]
fn the_item_is_listed_for_a_seed_with_a_code_on_a_stick() {
    let c = testkit::compute();
    let app = at_the_checklist();
    let items = app.backup_items_now(&c);
    assert!(items.contains(&bstep::KEF), "{items:?}");
    if let Some(sheets) = items.iter().position(|n| *n == bstep::SHEETS) {
        assert!(items.iter().position(|n| *n == bstep::KEF) < Some(sheets));
    }
    assert!(!done(&app));
}

#[test]
fn the_item_reads_not_checked_once_made_and_checked_once_opened_back() {
    let mut app = at_the_checklist();
    assert_eq!(status(&mut app), "Not made");
    open_item(&mut app, bstep::KEF);
    assert_eq!(status(&mut app), "Not made");
    make(&mut app);
    assert!(!done(&app), "made, not checked");
    assert_eq!(status(&mut app), "Not checked");
    check_with(&mut app, "a passphrase");
    assert!(done(&app));
    assert_eq!(status(&mut app), "Checked");
    // Put away and opened again, it is still checked.
    app.press(Action::BStep(bstep::KEF));
    open_item(&mut app, bstep::KEF);
    assert!(done(&app));
    assert_eq!(status(&mut app), "Checked");
}

#[test]
fn a_wrong_passphrase_leaves_it_not_checked() {
    let mut app = at_the_checklist();
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    check_with(&mut app, "another one");
    assert!(!done(&app));
    assert_eq!(status(&mut app), "Not checked");
}

#[test]
fn back_at_the_plan_typing_makes_no_code() {
    let mut app = at_the_checklist();
    open_item(&mut app, bstep::KEF);
    let sent = app.outbox.len();
    app.press(Action::BPlan);
    type_in(&mut app, "Home");
    app.event(Event::Key(Key::Enter));
    testkit::settle(&mut app);
    assert_eq!(app.outbox.len(), sent, "nothing new in the Outbox");
    app.press(Action::BChecklist);
    assert_eq!(status(&mut app), "Not made");
}

#[test]
fn leaving_the_item_or_the_checklist_wipes_the_passphrase() {
    for leave in [Action::BStep(bstep::KEF), Action::BPlan] {
        let mut app = at_the_checklist();
        open_item(&mut app, bstep::KEF);
        app.press(Action::KefTool(K::Backup(KefAct::Field)));
        type_in(&mut app, "typed and left");
        app.press(leave);
        app.press(Action::BChecklist);
        open_item(&mut app, bstep::KEF);
        make(&mut app);
        check_with(&mut app, "a passphrase");
        assert!(done(&app), "{leave:?}: the old text is gone");
    }
}
