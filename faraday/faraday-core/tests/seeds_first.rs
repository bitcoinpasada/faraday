//! A wallet made from seed words alone (`docs/WALLETS.md` §2,
//! `docs/FAMILY.md` §4): someone whose backup is lists of words and a
//! line like "2 of 3" types each seed, chooses M of N, the kind and the
//! path, and gets the wallet the seeds were part of, on Restore and on
//! the Spend tab, with the same descriptor and first address as the test
//! kit's wallets.

use faraday_core::family::{FamilyAction as F, Open, Route, page};
use faraday_core::rstep::{CHECK, KIND, QUORUM};
use faraday_core::seeds::{Focus, SLIDE_M, SLIDE_N, SeedsAction as S};
use faraday_core::testkit;
use faraday_core::{Action, Faraday, Screen};
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

const MULTI_PATH: &str = "m/48'/1'/0'/2'";

fn shown() -> Faraday {
    let mut app = Faraday::new();
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
    app.press(Action::Network(testkit::NET));
    app
}

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// Add a key from where the app is, the words of test seed `word`
/// typed; back where it came from.
fn add_seed(app: &mut Faraday, word: &str) {
    let from = app.screen;
    app.press(Action::Entry(None));
    assert_eq!(app.screen, Screen::Entry);
    type_text(app, &testkit::test_words(word));
    app.press(Action::EntryAdd);
    assert_eq!(app.screen, from, "Add a key did not return");
}

fn desc(text: &str) -> String {
    faraday_core::wallet::read_wallet(text)
        .unwrap()
        .to_descriptor()
        .replace('\'', "h")
}

fn kit(id: &str) -> String {
    testkit::kits()
        .into_iter()
        .find(|k| k.id == id)
        .unwrap()
        .descriptor
}

/// Restore, from the seeds alone: the kind (`multi`: Multisig · native
/// SegWit at its 2 of 3) and I do not have it.
fn restore_from_seeds(multi: bool) -> Faraday {
    let mut app = shown();
    app.press(Action::RestoreWallet);
    if multi {
        app.press(Action::RKind(4));
        app.press(Action::RNext(KIND));
        app.press(Action::RNext(QUORUM));
    } else {
        app.press(Action::RNext(KIND));
    }
    app.press(Action::RSeeds);
    assert_eq!(app.screen, Screen::Restore);
    app
}

fn restored(app: &Faraday) -> String {
    let r = app.restore.as_ref().unwrap();
    let w = r.wallet.expect("no wallet made");
    assert_eq!(r.open, Some(CHECK), "the Check card is not next");
    desc(&app.session.wallets[w].policy.to_descriptor())
}

#[test]
fn three_seeds_on_restore_make_the_two_of_three() {
    let mut app = restore_from_seeds(true);
    for w in ["bacon", "zebra", "summer"] {
        add_seed(&mut app, w);
    }
    // Three keys, two to sign, P2WSH at BIP-48 by default.
    app.press(Action::Slide(SLIDE_M, 2));
    app.press(Action::Seeds(S::Make));
    assert_eq!(restored(&app), desc(&testkit::savings()));
}

#[test]
fn two_seeds_and_the_third_cosigners_xpub_make_the_same_wallet() {
    let mut app = restore_from_seeds(true);
    add_seed(&mut app, "bacon");
    add_seed(&mut app, "zebra");
    // The arrow keys move the slider last used: one more key.
    app.press(Action::Slide(SLIDE_N, 2));
    app.event(Event::Key(Key::Right));
    app.press(Action::Seeds(S::Focus(Focus::Cosigner(0))));
    type_text(&mut app, &testkit::key(2, MULTI_PATH));
    app.event(Event::Key(Key::Enter));
    app.press(Action::Seeds(S::Make));
    assert_eq!(restored(&app), desc(&testkit::savings()));
}

#[test]
fn one_seed_on_restore_makes_the_words_routes_wallet() {
    let mut app = restore_from_seeds(false);
    add_seed(&mut app, "bacon");
    app.press(Action::Seeds(S::Make));
    let restored = restored(&app);
    assert_eq!(restored, desc(&kit("spending")));

    // The Spend tab's words route opens the same wallet from the same
    // words.
    let mut tab = shown();
    tab.press(Action::Nav(Screen::Family));
    tab.press(Action::Family(F::Holding(Route::Words)));
    add_seed(&mut tab, "bacon");
    let w = tab.family_wallet().expect("the words opened no wallet");
    let words = &tab.session.wallets[w];
    let r = app.restore.as_ref().unwrap().wallet.unwrap();
    assert_eq!(
        tab.session.address(words, false, 0),
        app.session.address(&app.session.wallets[r], false, 0)
    );
}

#[test]
fn the_spend_tab_takes_two_seeds_and_an_xpub_to_page_five() {
    let mut app = shown();
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(F::Holding(Route::Words)));
    add_seed(&mut app, "bacon");
    // One seed opens native SegWit, and the page stays for another.
    assert!(app.family_wallet().is_some());
    assert_eq!(app.family.open, Some(Open::Page(page::OPEN)));
    add_seed(&mut app, "zebra");
    // Two: the single-key wallet goes; how many sign comes up.
    assert!(app.family_wallet().is_none());
    assert!(app.family.seeds.shaping);
    app.press(Action::Slide(SLIDE_N, 3));
    app.press(Action::Slide(SLIDE_M, 2));
    app.press(Action::Seeds(S::Focus(Focus::Cosigner(0))));
    type_text(&mut app, &testkit::key(2, MULTI_PATH));
    app.press(Action::Seeds(S::Make));
    let w = app.family_wallet().expect("no wallet made");
    assert_eq!(
        desc(&app.session.wallets[w].policy.to_descriptor()),
        desc(&testkit::savings())
    );
    assert_eq!(app.family.open, Some(Open::Page(page::CHECK)));
}
