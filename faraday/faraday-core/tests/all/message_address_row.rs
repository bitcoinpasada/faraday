//! Sign a message's address row says the address the signature is for: the
//! wallet's receive address at the chosen number, and it follows the number.

use faraday_core::{Action, Faraday, testkit};

fn app() -> Faraday {
    let c = testkit::compute();
    let mut app = testkit::started();
    app.session = testkit::session();
    app.session
        .keys
        .add_words(
            &c,
            &testkit::test_words(testkit::TEST_SEEDS[0].0),
            "Bacon",
            None,
        )
        .expect("the key");
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    app.session
        .add_wallet_confirmed(
            &c,
            kit.name,
            &kit.descriptor,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .expect("the wallet");
    app
}

/// The address the row holds, without its groups' spaces.
fn row(app: &Faraday) -> String {
    let s = testkit::message(app).expect("open").shown.clone();
    s.expect("an address is kept").text.replace(' ', "")
}

#[test]
fn the_row_says_the_address_the_signature_is_for() {
    let mut app = app();
    app.press(Action::SignMessage);
    let first = row(&app);
    app.press(Action::MIndex(1));
    let second = row(&app);
    assert_ne!(first, second);
    app.press(Action::MSign);
    let m = testkit::message(&app).unwrap();
    let signed = m.signed.as_ref().expect("signed");
    assert_eq!(signed.address.as_str(), second.as_str());
    app.press(Action::MIndex(-1));
    assert_eq!(row(&app), first);
}

#[test]
fn the_address_is_in_groups_of_four() {
    let mut app = app();
    app.press(Action::SignMessage);
    let s = testkit::message(&app).unwrap().shown.clone().unwrap();
    assert!(!s.held);
    assert!(s.text.split(' ').all(|g| g.len() <= 4));
}

/// A second single-key wallet of the same key, after the first.
fn add_second(app: &mut Faraday) {
    let c = testkit::compute();
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "taproot")
        .unwrap();
    app.session
        .add_wallet_confirmed(
            &c,
            kit.name,
            &kit.descriptor,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .expect("the second wallet");
}

/// The text the row would say now: nothing while the kept address does not
/// match the session.
fn drawn(app: &Faraday) -> Option<String> {
    testkit::message_shown(app).map(|s| s.text.replace(' ', ""))
}

/// What the chosen wallet's receive address is now, worked out afresh.
fn fresh(app: &Faraday) -> String {
    let m = testkit::message(app).unwrap();
    let c = testkit::compute();
    let wl = &app.session.wallets[m.wallet.unwrap()];
    app.receive_line(&c, wl, false, m.index).0.replace(' ', "")
}

#[test]
fn a_wallet_removed_under_the_flow_never_leaves_the_row_on_another_wallets_address() {
    let mut app = app();
    add_second(&mut app);
    app.press(Action::SignMessage);
    app.press(Action::MWallet(1));
    let before = drawn(&app).expect("kept");
    // A wallet before the chosen one goes: the chosen wallet moves up one
    // and the row says its address still, from the place it has now.
    app.press(Action::RemoveWallet(0));
    assert_eq!(testkit::message(&app).unwrap().wallet, Some(0));
    assert_eq!(drawn(&app), Some(before));
    assert_eq!(drawn(&app), Some(fresh(&app)));
    // The chosen wallet goes: nothing is chosen, so no address.
    app.press(Action::RemoveWallet(0));
    assert_eq!(testkit::message(&app).unwrap().wallet, None);
    assert_eq!(drawn(&app), None);
}

#[test]
fn a_forged_wallet_taken_out_before_the_chosen_one_leaves_the_row_drawn() {
    let c = testkit::compute();
    let mut app = testkit::started();
    app.session = testkit::session();
    // Wallet 0 states test seed 0's fingerprint over seed 1's key, wallet 1
    // is seed 0's own: with no key loaded neither is checked.
    let path = "m/84'/1'/0'";
    let own = testkit::key(0, path);
    let other = testkit::key(1, path);
    let forged = format!(
        "wpkh({}]{}/<0;1>/*)",
        own.split_once(']').expect("origin").0,
        other.split_once(']').expect("xpub").1
    );
    let real = format!("wpkh({}/<0;1>/*)", testkit::key(0, "m/84'/1'/1'"));
    for (name, d) in [("Forged", &forged), ("Real", &real)] {
        app.session
            .add_wallet_confirmed(&c, name, d, "test", faraday_core::wallet::Origin::Named)
            .expect("the wallet");
    }
    app.press(Action::SignMessage);
    app.press(Action::MWallet(1));
    app.session
        .keys
        .add_words(
            &c,
            &testkit::test_words(testkit::TEST_SEEDS[0].0),
            "Bacon",
            None,
        )
        .expect("the key");
    // The next press shows wallet 0 forged: it goes, and the chosen wallet
    // moves up one.
    app.press(Action::MType);
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(testkit::message(&app).unwrap().wallet, Some(0));
    assert_eq!(drawn(&app), Some(fresh(&app)));
}

#[test]
fn a_wallet_renamed_under_the_flow_is_not_drawn_until_worked_out_again() {
    let mut app = app();
    app.press(Action::SignMessage);
    assert!(drawn(&app).is_some());
    app.session.wallets[0].name = faraday_text::Text::of("Renamed");
    assert_eq!(drawn(&app), None, "a renamed wallet");
    app.press(Action::MType);
    assert_eq!(drawn(&app), Some(fresh(&app)));
}

#[test]
fn a_wallet_added_under_the_flow_leaves_the_row_as_it_was() {
    let mut app = app();
    app.press(Action::SignMessage);
    let before = drawn(&app).unwrap();
    add_second(&mut app);
    assert_eq!(drawn(&app), Some(before));
}

#[test]
fn the_backup_done_or_undone_changes_the_row_at_the_press_that_does_it() {
    let mut app = app();
    app.press(Action::SignMessage);
    let fp = app.session.keys.last().unwrap().fingerprint;
    let address = drawn(&app).unwrap();
    testkit::mark_made_here(&mut app, fp);
    let held = testkit::message_shown(&app).expect("worked out at the change");
    assert!(held.held, "the line, not the address");
    assert_ne!(held.text.replace(' ', ""), address);
    testkit::release_made(&mut app, fp);
    app.press(Action::MType);
    let s = testkit::message_shown(&app).unwrap();
    assert!(!s.held);
    assert_eq!(s.text.replace(' ', ""), address);
}

#[test]
fn a_lock_leaves_nothing_of_the_row() {
    let mut app = app();
    app.press(Action::SignMessage);
    app.press(Action::Lock);
    assert!(drawn(&app).is_none());
    assert!(testkit::message(&app).is_none(), "the flow keeps nothing");
}
