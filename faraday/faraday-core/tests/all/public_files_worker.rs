//! A wallet's public files and the backup's checklist are made on the
//! worker (`docs/WORKERS.md` §2.1): the files are the ones made on the
//! thread that asked, a Back or a lock while they are made keeps nothing
//! of the answer, and a second press while they are made starts no second
//! job.

use crate::common::type_text;
use faraday_core::seeds::SeedsAction as S;
use faraday_core::testkit;
use faraday_core::{Action, BStage, Faraday, plan};

/// The Savings multisig, loaded on the Wallets tab.
fn holding() -> (Faraday, usize) {
    let c = testkit::compute();
    let mut app = testkit::started();
    app.session = testkit::session();
    let w = app
        .session
        .add_wallet(
            &c,
            "Savings",
            &testkit::savings(),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .expect("a wallet");
    (app, w)
}

/// What is For the stick, by name.
fn outbox(app: &Faraday) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<_> = app
        .outbox
        .iter()
        .map(|i| (String::from(&*i.name), i.bytes.to_vec()))
        .collect();
    files.sort();
    files
}

/// Test key 2 typed in and a one-key wallet made from it.
fn one_key(app: &mut Faraday) -> usize {
    app.press(Action::Entry(None));
    type_text(app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    let fp = app.session.keys.last().unwrap().fingerprint;
    app.press(Action::KeyWallet(fp.0, 1));
    app.press(Action::Seeds(S::Make));
    app.session.wallets.len() - 1
}

/// A backup of a one-key wallet whose plan calls for public files
/// (Sparrow), on the plan card, ready for Make the checklist.
fn plan_card() -> (Faraday, usize) {
    let mut app = testkit::started();
    let w = one_key(&mut app);
    app.press(Action::Backup(w));
    testkit::backup_as(&mut app, plan::Preset::PaperVaultSoftware);
    app.press(Action::BAnswer(
        faraday_core::qrow::SOFTWARE,
        plan::software::SPARROW as u8,
    ));
    (app, w)
}

/// The kinds of a wallet's files made on the worker: the multisig config,
/// the split shares and Specter DIY's pictures.
const KINDS: [u8; 3] = [2, 4, 14];

#[test]
fn each_kind_of_public_files_on_the_worker_is_the_files_made_on_the_thread_that_asked() {
    for what in KINDS {
        let (mut inline, w) = holding();
        inline.press(Action::PublicOut(w, what));
        let want = outbox(&inline);
        assert!(!want.is_empty(), "kind {what} made nothing");

        let (mut app, w) = holding();
        testkit::press_on_worker(&mut app, Action::PublicOut(w, what));
        assert!(app.busy().is_some(), "kind {what}: the screen is busy");
        assert!(app.outbox.is_empty(), "kind {what}: nothing yet");
        testkit::settle(&mut app);
        assert!(app.busy().is_none());
        assert_eq!(outbox(&app), want, "kind {what}");
        assert_eq!(app.toast_text(), inline.toast_text(), "kind {what}");
    }
}

#[test]
fn a_second_press_while_public_files_are_made_starts_no_second_job() {
    let (mut app, w) = holding();
    testkit::hold_worker();
    let before = testkit::jobs_asked(&app);
    testkit::press_on_worker(&mut app, Action::PublicOut(w, 4));
    app.press(Action::PublicOut(w, 4));
    app.press(Action::PublicOut(w, 14));
    assert_eq!(testkit::jobs_asked(&app), before + 1);
    testkit::release_worker();
    testkit::settle(&mut app);
    assert!(
        app.outbox.iter().all(|i| !i.name.contains("specter")),
        "the second press made nothing"
    );
}

#[test]
fn back_while_public_files_are_made_keeps_nothing_of_the_answer() {
    let (mut app, w) = holding();
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::PublicOut(w, 4));
    app.press(Action::Back);
    assert!(app.busy().is_none());
    testkit::release_worker();
    testkit::settle_stale(&mut app);
    assert!(app.outbox.is_empty(), "the answer was dropped");
}

#[test]
fn a_lock_while_public_files_are_made_keeps_nothing_of_the_answer() {
    let (mut app, w) = holding();
    let (screen, sheet) = (app.screen, app.sheet);
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::PublicOut(w, 4));
    app.press(Action::LockUnsealed);
    assert!(app.busy().is_none());
    // The person is back where the press was when the answer comes.
    app.screen = screen;
    app.sheet = sheet;
    testkit::release_worker();
    testkit::settle_stale(&mut app);
    assert!(app.outbox.is_empty(), "no file is kept after the lock");
}

#[test]
fn the_checklist_made_on_the_worker_is_the_one_made_on_the_thread_that_asked() {
    let (mut inline, _) = plan_card();
    inline.press(Action::BChecklist);
    let want = outbox(&inline);
    assert!(!want.is_empty(), "the checklist made no file");
    assert_eq!(inline.backup().unwrap().stage, BStage::Checklist);

    let (mut app, _) = plan_card();
    testkit::press_on_worker(&mut app, Action::BChecklist);
    assert!(app.busy().is_some(), "the screen is busy");
    assert_ne!(
        app.backup().unwrap().stage,
        BStage::Checklist,
        "the checklist opens with the answer"
    );
    assert!(app.outbox.is_empty(), "nothing yet");
    testkit::settle(&mut app);
    assert!(app.busy().is_none());
    assert_eq!(app.backup().unwrap().stage, BStage::Checklist);
    assert_eq!(outbox(&app), want);
    assert_eq!(app.toast_text(), inline.toast_text());
}

#[test]
fn a_second_press_while_the_checklist_is_made_starts_no_second_job() {
    let (mut app, _) = plan_card();
    testkit::hold_worker();
    let before = testkit::jobs_asked(&app);
    testkit::press_on_worker(&mut app, Action::BChecklist);
    app.press(Action::BChecklist);
    assert_eq!(testkit::jobs_asked(&app), before + 1);
    testkit::release_worker();
    testkit::settle(&mut app);
    assert_eq!(app.backup().unwrap().stage, BStage::Checklist);
}

#[test]
fn back_while_the_checklist_is_made_keeps_nothing_of_the_answer() {
    let (mut app, _) = plan_card();
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::BChecklist);
    app.press(Action::Back);
    assert!(app.busy().is_none());
    testkit::release_worker();
    testkit::settle_stale(&mut app);
    assert!(app.outbox.is_empty(), "the answer was dropped");
    assert_ne!(app.backup().map(|b| b.stage), Some(BStage::Checklist));
}

#[test]
fn a_lock_while_the_checklist_is_made_keeps_nothing_of_the_answer() {
    let (mut app, _) = plan_card();
    let (screen, sheet) = (app.screen, app.sheet);
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::BChecklist);
    app.press(Action::LockUnsealed);
    assert!(app.busy().is_none());
    app.screen = screen;
    app.sheet = sheet;
    testkit::release_worker();
    testkit::settle_stale(&mut app);
    assert!(app.outbox.is_empty(), "no file is kept after the lock");
}

/// The plan card's checklist open, with nothing yet For the stick: Make the
/// files has its files still to make.
fn checklist_open() -> (Faraday, usize) {
    let (mut app, w) = plan_card();
    app.press(Action::BChecklist);
    assert_eq!(app.backup().unwrap().stage, BStage::Checklist);
    app.outbox.clear();
    (app, w)
}

#[test]
fn make_the_files_on_the_worker_makes_the_files_made_on_the_thread_that_asked() {
    let (mut inline, _) = checklist_open();
    inline.press(Action::BFilesMake);
    let want = outbox(&inline);
    assert!(!want.is_empty(), "Make the files made no file");

    let (mut app, _) = checklist_open();
    testkit::press_on_worker(&mut app, Action::BFilesMake);
    assert!(app.busy().is_some(), "the screen is busy");
    assert!(app.outbox.is_empty(), "nothing yet");
    testkit::settle(&mut app);
    assert!(app.busy().is_none());
    assert_eq!(outbox(&app), want);
    assert_eq!(app.toast_text(), inline.toast_text());
}

#[test]
fn a_second_press_while_the_files_are_made_starts_no_second_job() {
    let (mut app, _) = checklist_open();
    testkit::hold_worker();
    let before = testkit::jobs_asked(&app);
    testkit::press_on_worker(&mut app, Action::BFilesMake);
    app.press(Action::BFilesMake);
    assert_eq!(testkit::jobs_asked(&app), before + 1);
    testkit::release_worker();
    testkit::settle(&mut app);
    assert!(!app.outbox.is_empty());
}

#[test]
fn back_while_the_files_are_made_keeps_nothing_of_the_answer() {
    let (mut app, _) = checklist_open();
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::BFilesMake);
    app.press(Action::Back);
    assert!(app.busy().is_none());
    testkit::release_worker();
    testkit::settle_stale(&mut app);
    assert!(app.outbox.is_empty(), "the answer was dropped");
}

#[test]
fn a_lock_while_the_files_are_made_keeps_nothing_of_the_answer() {
    let (mut app, _) = checklist_open();
    let (screen, sheet) = (app.screen, app.sheet);
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::BFilesMake);
    app.press(Action::LockUnsealed);
    assert!(app.busy().is_none());
    app.screen = screen;
    app.sheet = sheet;
    testkit::release_worker();
    testkit::settle_stale(&mut app);
    assert!(app.outbox.is_empty(), "no file is kept after the lock");
}
