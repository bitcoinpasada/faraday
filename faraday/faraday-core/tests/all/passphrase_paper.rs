//! A passphrase the plan keeps on paper is its own checklist item, right
//! after its seed's copy (`docs/history/NEW-WALLET.md` §14.7 item 5): typed back
//! with the passphrase hidden and compared whole; the right passphrase
//! completes it, a wrong one does not. A plan with no passphrase on paper
//! has no such item.

use crate::common::type_text;
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_core::create::NewKind;
use faraday_core::plan::{At, Item, Preset, What};
use faraday_core::testkit::KindText;
use faraday_core::{Action, Faraday, bstep, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

const PASSPHRASE: &str = "lantern Orchid 41";

fn device() -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 2400,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
    app
}

/// A key loaded with `passphrase` (none when empty), a one-key wallet
/// over it, and its backup planned on paper, the checklist made.
fn backing_up(passphrase: &str) -> Faraday {
    let c = testkit::compute();
    let mut app = device();
    let m = Mnemonic::parse(
        &c,
        Language::English,
        &testkit::test_words(testkit::TEST_SEEDS[0].0),
    )
    .unwrap();
    app.session
        .keys
        .add_mnemonic(&c, &m, passphrase, "Key", None)
        .unwrap();
    let text = NewKind::NativeSegwit
        .key_text(&c, app.session.keys.master_for_tests(0))
        .unwrap();
    let w = app
        .session
        .add_wallet(
            &c,
            "Spending",
            &format!("wpkh({text}/<0;1>/*)"),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    app.press(Action::Backup(w));
    testkit::backup_as(&mut app, Preset::Paper);
    app.press(Action::BChecklist);
    let _ = app.frame();
    app
}

/// The templates printed and the seed's copy checked: the passphrase's
/// item is open.
fn at_the_passphrase(app: &mut Faraday) {
    if app
        .backup_items_now(&testkit::compute())
        .contains(&bstep::BLANK)
    {
        app.press(Action::BNext(bstep::BLANK));
    }
    assert_eq!(app.backup().unwrap().open, Some(bstep::COPY));
    testkit::type_copy(app);
    app.press(Action::BNext(bstep::COPY));
    assert_eq!(
        app.backup().unwrap().open,
        Some(bstep::PASSPHRASE),
        "the passphrase's item does not follow the copy"
    );
    let _ = app.frame();
}

fn done(app: &Faraday) -> bool {
    let c = testkit::compute();
    app.backup_item_done_now(&c, Item::Passphrase(0))
}

/// Whether the page, settled, says anything that contains `part`.
fn says(app: &mut Faraday, part: &str) -> bool {
    app.settle();
    let _ = app.frame();
    app.drawn_texts().iter().any(|t| t.contains(part))
}

#[test]
fn a_passphrase_on_paper_is_its_own_item_after_its_seeds_copy() {
    let mut app = backing_up(PASSPHRASE);
    let w = app.backup().unwrap().wallet;
    let shape = app.plan_shape_now(w);
    let a = app.backup().unwrap().answers.clone();
    let map = faraday_core::plan::map(&testkit::compute(), &shape, &a);
    assert!(
        map.iter().any(|s| matches!(s.at, At::Place(_))
            && s.holds.iter().any(|(h, _)| *h == What::Passphrase(0))),
        "the plan keeps no passphrase on paper: {map:?}"
    );
    let items = app.backup_items_now(&testkit::compute());
    let copy = items
        .iter()
        .position(|&i| i == bstep::COPY)
        .expect("a copy item");
    assert_eq!(items.get(copy + 1), Some(&bstep::PASSPHRASE), "{items:?}");
    assert!(says(&mut app, "Write the passphrase of seed"));
}

#[test]
fn typing_the_right_passphrase_back_completes_the_item() {
    let mut app = backing_up(PASSPHRASE);
    at_the_passphrase(&mut app);
    assert!(!done(&app));
    app.press(Action::BCheck);
    assert!(says(&mut app, "Your copy's passphrase"));
    type_text(&mut app, PASSPHRASE);
    assert!(!says(&mut app, PASSPHRASE), "the passphrase typed is drawn");
    app.event(Event::Key(Key::Enter));
    assert!(done(&app), "the right passphrase did not complete the item");
    assert!(
        app.backup().unwrap().check.is_none(),
        "the field stays open"
    );
    assert!(says(&mut app, "Your copy matches the passphrase"));
    let _ = app.frame();
    assert!(app.offers(Action::BNext(bstep::PASSPHRASE)), "no Continue");
}

#[test]
fn a_wrong_passphrase_leaves_the_item_undone_and_wipes_what_was_typed() {
    let mut app = backing_up(PASSPHRASE);
    at_the_passphrase(&mut app);
    app.press(Action::BCheck);
    // Another case is another passphrase.
    type_text(&mut app, &PASSPHRASE.to_lowercase());
    // Compare, the foot's button while the field is open.
    app.settle();
    let _ = app.frame();
    assert!(app.offers(Action::BCheck), "no Compare");
    app.press(Action::BCheck);
    assert!(!done(&app), "a wrong passphrase completed the item");
    assert!(says(&mut app, "Your copy differs from the passphrase"));
    let check = app
        .backup()
        .unwrap()
        .check
        .as_ref()
        .expect("the field closed");
    assert!(check.typed.is_empty(), "what was typed is kept");
    assert!(!app.offers(Action::BNext(bstep::PASSPHRASE)), "a Continue");
    // A space more is another passphrase.
    type_text(&mut app, &format!("{PASSPHRASE} "));
    app.event(Event::Key(Key::Enter));
    assert!(!done(&app), "a passphrase with a space more completed it");
    type_text(&mut app, PASSPHRASE);
    app.event(Event::Key(Key::Enter));
    assert!(done(&app));
}

#[test]
fn show_the_passphrase_shows_it_and_the_check_hides_it() {
    let mut app = backing_up(PASSPHRASE);
    at_the_passphrase(&mut app);
    assert!(!says(&mut app, PASSPHRASE), "shown before Show");
    app.press(Action::BReveal);
    assert!(
        says(&mut app, PASSPHRASE),
        "Show the passphrase shows nothing"
    );
    app.press(Action::BCheck);
    assert!(!says(&mut app, PASSPHRASE), "shown while the check is open");
    // Show ends the check and wipes what was typed.
    type_text(&mut app, "lan");
    app.press(Action::BReveal);
    assert!(app.backup().unwrap().check.is_none());
    assert!(says(&mut app, PASSPHRASE));
}

#[test]
fn a_plan_with_no_passphrase_on_paper_has_no_passphrase_item() {
    let app = backing_up("");
    let items = app.backup_items_now(&testkit::compute());
    assert!(items.contains(&bstep::COPY), "{items:?}");
    assert!(!items.iter().any(|&i| bstep::is_passphrase(i)), "{items:?}");
}
