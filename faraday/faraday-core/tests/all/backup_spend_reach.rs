//! How near a place brings a spend (`docs/BACKUP-FLOW.md` §17.22, F56):
//! seeds alone spend, the description being public; every place that
//! keeps a seed says how many more places it needs; a drop that brings a
//! place keeping two seeds or more nearer a spend asks on a sheet, with
//! Keep it and Take it off. Driven through Back up's board.

use crate::common::*;
use faraday_core::board::{BoardPress as BP, Chip, Thing, Zone};
use faraday_core::plan::{At, What};
use faraday_core::{Action, Faraday, Sheet, cstep, testkit};
use faraday_shell_api::{App, Event, Key};

/// Two published BIP-39 test mnemonics, for a wallet of five keys.
const MORE_WORDS: [&str; 2] = [
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon \
     abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon \
     abandon art",
    "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo \
     zoo vote",
];

fn words_in(app: &mut Faraday, words: &str) {
    app.press(Action::Entry(None));
    for c in words.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
}

fn multi_index() -> u8 {
    faraday_core::create::NewKind::ALL
        .iter()
        .position(|k| *k == faraday_core::create::NewKind::Multi)
        .expect("Multisig is one of the kinds") as u8
}

/// An m-of-n multisig over keys loaded here, its backup on the board
/// with The standard way placed, opened for changes.
fn board_of(m: usize, n: usize) -> Faraday {
    let mut app = testkit::started();
    for (w, _) in testkit::TEST_SEEDS.iter().take(n) {
        words_in(&mut app, &testkit::test_words(w));
    }
    for w in MORE_WORDS.iter().take(n.saturating_sub(3)) {
        words_in(&mut app, w);
    }
    app.press(Action::CreateWallet);
    app.press(Action::CKind(multi_index()));
    for _ in 3..n {
        app.press(Action::CN(1));
    }
    for _ in 2..m {
        app.press(Action::CM(1));
    }
    for slot in 0..n {
        let fp = app.session.keys[slot].fingerprint.0;
        app.press(Action::CSlotHere(slot as u8, fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    let w = app.session.wallets.len() - 1;
    app.press(Action::Backup(w));
    app.press(Action::Board(BP::Customize));
    let _ = app.frame();
    app
}

/// A placed chip moved, pressed as a person taps it and then where it
/// goes, with any sheet the drop opens left open.
fn moved(app: &mut Faraday, from: Zone, t: Thing, to: Zone) {
    app.press(Action::Board(BP::Chip(Chip::Placed(from, t))));
    app.press(Action::Board(BP::Zone(to)));
}

/// Every text on screen, a sheet's wrapped lines joined back into their
/// sentences.
fn drawn(app: &mut Faraday) -> String {
    app.drawn_texts().join(" ")
}

fn place_holds(app: &Faraday, p: usize) -> Vec<What> {
    backup_map(app)
        .iter()
        .filter(|s| s.at == At::Place(p))
        .flat_map(|s| s.holds.iter().map(|(w, _)| *w))
        .collect()
}

#[test]
fn the_standard_two_of_three_says_each_place_spends_with_one_more() {
    let mut app = board_of(2, 3);
    let t = drawn(&mut app);
    assert_eq!(
        t.matches("Spends with 1 more place").count(),
        3,
        "every place: {t}"
    );
    assert_eq!(app.sheet, None);
}

#[test]
fn a_second_seed_at_a_place_without_the_description_can_spend_and_asks() {
    let mut app = board_of(2, 3);
    take_off(&mut app, Zone::Paper(0), Thing::Whole);
    moved(&mut app, Zone::Paper(1), Thing::Seed(1), Zone::Paper(0));
    assert_eq!(app.sheet, Some(Sheet::BoardSees), "the drop asks");
    let t = drawn(&mut app);
    assert!(t.contains("Anyone with access to Place 1 can spend"), "{t}");
    assert!(
        t.contains("Place 1 keeps 2 of the 2 seeds a spend needs."),
        "{t}"
    );
    app.press(Action::Board(BP::SeesKeep));
    assert_eq!(app.sheet, None);
    let t = drawn(&mut app);
    assert!(
        t.contains("Cannot see balance"),
        "no description there: {t}"
    );
    assert!(t.contains("Can spend alone"), "{t}");
    assert!(t.contains("One place found: can spend Yes"), "{t}");
}

#[test]
fn take_it_off_puts_the_second_seed_back() {
    let mut app = board_of(2, 3);
    moved(&mut app, Zone::Paper(1), Thing::Seed(1), Zone::Paper(0));
    assert_eq!(app.sheet, Some(Sheet::BoardSees));
    app.press(Action::Board(BP::SeesUndo));
    assert_eq!(app.sheet, None);
    assert!(!place_holds(&app, 0).contains(&What::Words(1)), "taken off");
    assert!(place_holds(&app, 1).contains(&What::Words(1)), "back");
}

#[test]
fn a_first_seed_at_a_place_asks_nothing() {
    let mut app = board_of(2, 3);
    take_off(&mut app, Zone::Paper(0), Thing::Seed(0));
    app.press(Action::Board(BP::Chip(Chip::List(Thing::Seed(0)))));
    app.press(Action::Board(BP::Zone(Zone::Paper(0))));
    assert!(place_holds(&app, 0).contains(&What::Words(0)));
    assert_eq!(app.sheet, None);
}

#[test]
fn a_second_seed_in_the_places_vault_spends_with_the_vaults_passphrase() {
    let mut app = board_of(2, 3);
    board(&mut app, BP::AddVault);
    put(&mut app, Thing::Vault(0), Zone::Card(0));
    moved(
        &mut app,
        Zone::Paper(1),
        Thing::Seed(1),
        Zone::Vault(0, 0, 0),
    );
    assert_eq!(app.sheet, Some(Sheet::BoardSees), "the drop asks");
    let t = drawn(&mut app);
    assert!(
        t.contains("Anyone with access to Place 1 and the vault's passphrase can spend"),
        "{t}"
    );
    assert!(
        t.contains("Place 1 keeps 2 of the 2 seeds a spend needs, counting its vaults."),
        "{t}"
    );
    app.press(Action::Board(BP::SeesKeep));
    let t = drawn(&mut app);
    assert!(
        t.contains("Can spend alone with the vault's passphrase"),
        "{t}"
    );
}

#[test]
fn a_three_of_five_with_two_seeds_at_a_place_needs_only_one_more() {
    let mut app = board_of(3, 5);
    let t = drawn(&mut app);
    assert!(
        t.contains("Spends with 2 more places"),
        "one seed each: {t}"
    );
    moved(&mut app, Zone::Paper(1), Thing::Seed(1), Zone::Paper(0));
    assert_eq!(app.sheet, Some(Sheet::BoardSees), "the drop asks");
    let t = drawn(&mut app);
    assert!(
        t.contains("Anyone with access to Place 1 needs only 1 more place to spend"),
        "{t}"
    );
    assert!(
        t.contains("Place 1 keeps 2 of the 3 seeds a spend needs."),
        "{t}"
    );
    app.press(Action::Board(BP::SeesKeep));
    let t = drawn(&mut app);
    assert!(t.contains("Spends with 1 more place"), "{t}");
}

#[test]
fn a_single_keys_seed_says_it_spends_alone_and_never_asks() {
    let mut app = testkit::started();
    words_in(&mut app, &testkit::test_words("bacon"));
    app.press(Action::CreateWallet);
    let fp = app.session.keys[0].fingerprint.0;
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    let w = app.session.wallets.len() - 1;
    app.press(Action::Backup(w));
    app.press(Action::Board(BP::Customize));
    take_off(&mut app, Zone::Paper(0), Thing::Seed(0));
    app.press(Action::Board(BP::Chip(Chip::List(Thing::Seed(0)))));
    app.press(Action::Board(BP::Zone(Zone::Paper(0))));
    assert!(place_holds(&app, 0).contains(&What::Words(0)));
    assert_eq!(app.sheet, None, "a single key never asks");
    let t = drawn(&mut app);
    assert!(t.contains("Can spend alone"), "{t}");
}
