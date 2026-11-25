//! A wallet one of whose keys carries a loaded key's fingerprint but
//! another seed's extended public key is refused when it is added: its
//! addresses would be shown as the person's and could not be signed for.

use faraday_core::{testkit, wallet::Origin};

/// The key of test seed 0 at `path` with the extended public key of test
/// a seed at the same path: this device's fingerprint and origin, another
/// seed's key.
fn forged(path: &str, xpub_path: &str) -> String {
    let own = testkit::key(0, path);
    let other = testkit::key(1, xpub_path);
    let origin = own.split_once(']').expect("origin").0;
    let xpub = other.split_once(']').expect("xpub").1;
    format!("{origin}]{xpub}")
}

fn session_with_key() -> faraday_core::wallet::Session {
    let c = testkit::compute();
    let mut session = testkit::session();
    session
        .keys
        .add_words(
            &c,
            &testkit::test_words(testkit::TEST_SEEDS[0].0),
            "Bacon",
            None,
        )
        .expect("the key");
    session
}

fn add(
    session: &mut faraday_core::wallet::Session,
    descriptor: &str,
) -> Result<usize, faraday_core::wallet::Refusal> {
    session.add_wallet_confirmed(
        &testkit::compute(),
        "Other",
        descriptor,
        "test",
        Origin::Named,
    )
}

#[test]
fn a_single_key_wallet_with_my_fingerprint_and_another_xpub_is_refused() {
    let mut session = session_with_key();
    let d = format!("wpkh({}/<0;1>/*)", forged("m/84'/1'/0'", "m/84'/1'/0'"));
    let e = add(&mut session, &d).expect_err("refused");
    let fp = d.trim_start_matches("wpkh([").split('/').next().unwrap();
    assert!(e.text().contains(fp), "{}", &*e.text());
    assert!(session.wallets.is_empty());
}

#[test]
fn a_multisig_with_my_fingerprint_and_another_xpub_is_refused() {
    let mut session = session_with_key();
    let p = "m/48'/1'/0'/2'";
    let d = format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
        forged(p, "m/48'/1'/1'/2'"),
        testkit::key(1, p),
        testkit::key(2, p)
    );
    let e = add(&mut session, &d).expect_err("refused");
    assert!(matches!(e, faraday_core::wallet::Refusal::NotYourKey(_)));
    assert!(session.wallets.is_empty());
}

#[test]
fn a_musig2_wallet_with_my_fingerprint_and_another_xpub_is_refused() {
    let mut session = session_with_key();
    let p = "m/86'/1'/0'";
    let d = format!(
        "tr(musig({},{})/<0;1>/*)",
        forged(p, "m/86'/1'/1'"),
        testkit::key(1, p)
    );
    let e = add(&mut session, &d).expect_err("refused");
    assert!(matches!(e, faraday_core::wallet::Refusal::NotYourKey(_)));
    assert!(session.wallets.is_empty());
}

#[test]
fn a_miniscript_wallet_with_my_fingerprint_and_another_xpub_is_refused() {
    let mut session = session_with_key();
    let p = "m/48'/1'/0'/2'";
    let d = format!(
        "wsh(and_v(v:pk({}/<0;1>/*),pk({}/<0;1>/*)))",
        forged(p, "m/48'/1'/1'/2'"),
        testkit::key(1, p)
    );
    let e = add(&mut session, &d).expect_err("refused");
    assert!(matches!(e, faraday_core::wallet::Refusal::NotYourKey(_)));
    assert!(session.wallets.is_empty());
}

#[test]
fn a_forged_wallet_added_before_its_key_is_taken_out_when_the_key_loads() {
    use faraday_core::Action;
    let mut app = testkit::started();
    let c = testkit::compute();
    let forged_d = format!("wpkh({}/<0;1>/*)", forged("m/84'/1'/0'", "m/84'/1'/0'"));
    let real = format!("wpkh({}/<0;1>/*)", testkit::key(0, "m/84'/1'/1'"));
    for (name, d) in [("Forged", &forged_d), ("Real", &real)] {
        app.session
            .add_wallet_confirmed(&c, name, d, "test", Origin::Named)
            .expect("no key is loaded: not checked");
    }
    assert_eq!(app.session.wallets.len(), 2);
    app.session
        .keys
        .add_words(
            &c,
            &testkit::test_words(testkit::TEST_SEEDS[0].0),
            "Bacon",
            None,
        )
        .expect("the key");
    app.press(Action::Back);
    let names: Vec<_> = app.session.wallets.iter().map(|w| w.name.clone()).collect();
    assert_eq!(names, ["Real"]);
    assert_eq!(app.session.message_wallets(), [0]);
    let toast = app.toast_text().expect("a toast");
    assert!(toast.contains("Forged"), "{toast}");
}
#[test]
fn my_own_wallet_still_loads() {
    let mut session = session_with_key();
    let d = format!("wpkh({}/<0;1>/*)", testkit::key(0, "m/84'/1'/0'"));
    add(&mut session, &d).expect("loads");
    let d = format!("wpkh({}/<0;1>/*)", testkit::key(1, "m/84'/1'/0'"));
    add(&mut session, &d).expect("a key that is not loaded is not checked");
}

/// A key made here and not backed up, in a started app. Returns it.
fn made_here(app: &mut faraday_core::Faraday) -> faraday_bip::keys::Fingerprint {
    use faraday_core::{Action, keygen::Way};
    app.press(Action::KeyGen(None));
    app.press(Action::KStart(
        faraday_core::keygen::Start::Make(faraday_core::forms::Form::Words).index(),
    ));
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Coins.index()));
    app.press(Action::KNext);
    for i in 0..128 {
        app.press(Action::KFlip((i * 7 + i / 5) % 2 == 0));
    }
    app.press(Action::KNext);
    app.press(Action::KLock);
    app.session.keys.last().expect("the key").fingerprint
}

/// The key made here as a wallet key at `path`: its extended public key
/// under origin fingerprint `00000001`, which no key here has.
fn under_another_fingerprint(app: &faraday_core::Faraday, path: &str) -> String {
    let c = testkit::compute();
    let p: faraday_bip::keys::DerivationPath = path.parse().expect("a path");
    let i = app.session.keys.len() - 1;
    let x = app.session.keys.xpub_at(&c, i, &p).expect("its xpub");
    format!("[00000001/{}]{x}", path.trim_start_matches("m/"))
}

fn held_or_refused(
    app: &mut faraday_core::Faraday,
    descriptor: &str,
) -> Result<bool, faraday_core::wallet::Refusal> {
    let c = testkit::compute();
    app.session
        .add_wallet_confirmed(&c, "Copy", descriptor, "test", Origin::Named)?;
    let wl = app.session.wallets.last().expect("added");
    let (_, held) = app.receive_line(&c, wl, false, 0);
    assert_eq!(held, app.receive_held(&c, wl));
    Ok(held)
}

fn single(app: &faraday_core::Faraday) -> String {
    format!(
        "wpkh({}/<0;1>/*)",
        under_another_fingerprint(app, "m/84'/1'/0'")
    )
}

fn not_backed_up(r: Result<bool, faraday_core::wallet::Refusal>) {
    let e = r.expect_err("refused");
    assert!(
        matches!(e, faraday_core::wallet::Refusal::NotBackedUp(_)),
        "{}",
        &*e.text()
    );
    assert!(e.text().contains("00000001"), "{}", &*e.text());
    assert!(e.text().contains("not backed up"), "{}", &*e.text());
}

#[test]
fn a_key_made_here_under_another_fingerprint_is_refused_so_its_addresses_are_never_shown_unheld() {
    let mut app = testkit::started();
    let fp = made_here(&mut app);
    assert!(app.key_held(&testkit::compute(), fp));
    let d = single(&app);
    not_backed_up(held_or_refused(&mut app, &d));
}

#[test]
fn a_multisig_with_a_key_made_here_under_another_fingerprint_is_refused() {
    let mut app = testkit::started();
    let fp = made_here(&mut app);
    assert!(app.key_held(&testkit::compute(), fp));
    let p = "m/48'/1'/0'/2'";
    let d = format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
        under_another_fingerprint(&app, p),
        testkit::key(1, p),
        testkit::key(2, p)
    );
    not_backed_up(held_or_refused(&mut app, &d));
}

#[test]
fn the_same_wallet_adds_once_the_key_is_backed_up() {
    let mut app = testkit::started();
    let fp = made_here(&mut app);
    let d = single(&app);
    not_backed_up(held_or_refused(&mut app, &d));
    testkit::release_made(&mut app, fp);
    app.press(faraday_core::Action::Back);
    held_or_refused(&mut app, &d).expect("added once backed up");
}

#[test]
fn a_key_that_begins_to_wait_after_the_wallet_was_added_takes_the_wallet_out() {
    let mut app = testkit::started();
    let fp = made_here(&mut app);
    testkit::release_made(&mut app, fp);
    app.press(faraday_core::Action::Back);
    let d = single(&app);
    held_or_refused(&mut app, &d).expect("added: the key does not wait");
    testkit::mark_made_here(&mut app, fp);
    app.press(faraday_core::Action::Back);
    assert!(app.session.wallets.is_empty(), "taken out");
    let toast = app.toast_text().expect("a toast");
    assert!(toast.contains("Copy"), "{toast}");
}

#[test]
fn a_multisig_held_for_its_sheet_is_refused_when_the_key_waits_by_the_hold() {
    let mut app = testkit::started();
    let fp = made_here(&mut app);
    testkit::release_made(&mut app, fp);
    app.press(faraday_core::Action::Back);
    let p = "m/48'/1'/0'/2'";
    let d = format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
        under_another_fingerprint(&app, p),
        testkit::key(1, p),
        testkit::key(2, p)
    );
    let c = testkit::compute();
    let e = app
        .session
        .add_wallet(&c, "Multi", &d, "test", Origin::Named)
        .expect_err("held for its sheet");
    assert!(matches!(e, faraday_core::wallet::Refusal::Unconfirmed));
    testkit::mark_made_here(&mut app, fp);
    testkit::confirm_wallets(&mut app);
    assert!(app.session.wallets.is_empty(), "the hold adds nothing");
}

/// Three wallets in a started app, none checked yet: a forged one (this
/// device's fingerprint, another seed's key), the person's own and one
/// more of the person's own. Returns the app.
fn forged_then_two_real() -> faraday_core::Faraday {
    let mut app = testkit::started();
    let c = testkit::compute();
    let forged_d = format!("wpkh({}/<0;1>/*)", forged("m/84'/1'/0'", "m/84'/1'/0'"));
    let real = format!("wpkh({}/<0;1>/*)", testkit::key(0, "m/84'/1'/1'"));
    let other = format!("wpkh({}/<0;1>/*)", testkit::key(0, "m/84'/1'/2'"));
    for (name, d) in [("Forged", &forged_d), ("Real", &real), ("Other", &other)] {
        app.session
            .add_wallet_confirmed(&c, name, d, "test", Origin::Named)
            .expect("no key is loaded: not checked");
    }
    app
}

/// Loads the key of test seed 0 and lets a press go by, as the person's
/// typing does: the forged wallet goes.
fn key_loads(app: &mut faraday_core::Faraday) {
    app.session
        .keys
        .add_words(
            &testkit::compute(),
            &testkit::test_words(testkit::TEST_SEEDS[0].0),
            "Bacon",
            None,
        )
        .expect("the key");
    app.press(faraday_core::Action::RemoveWallet(99));
}

fn backing_up(app: &faraday_core::Faraday) -> Option<String> {
    let b = app.backup()?;
    Some(app.session.wallets.get(b.wallet)?.name.to_string())
}

#[test]
fn a_backup_stays_on_its_wallet_when_an_earlier_one_is_taken_out() {
    let mut app = forged_then_two_real();
    app.press(faraday_core::Action::Backup(1));
    assert_eq!(backing_up(&app).as_deref(), Some("Real"));
    key_loads(&mut app);
    let names: Vec<_> = app
        .session
        .wallets
        .iter()
        .map(|w| w.name.to_string())
        .collect();
    assert_eq!(names, ["Real", "Other"]);
    assert_eq!(backing_up(&app).as_deref(), Some("Real"));
}

#[test]
fn a_backup_keeps_its_place_when_a_later_wallet_is_taken_out() {
    let mut app = forged_then_two_real();
    app.press(faraday_core::Action::Backup(0));
    app.press(faraday_core::Action::RemoveWallet(2));
    assert_eq!(backing_up(&app).as_deref(), Some("Forged"));
}

/// A backup flow of wallet `w`, not made, as the plan the backup open now
/// draws gives it.
fn push_flow_of(app: &mut faraday_core::Faraday, w: usize) {
    let (shape, answers) = app.board_plan().expect("the plan");
    let mut f =
        faraday_core::backup_flow::BackupFlow::new_wallet(&testkit::compute(), shape, answers, "w");
    f.wallet.as_mut().unwrap().checksum =
        Some(faraday_text::Text::of(app.session.wallets[w].checksum()));
    testkit::flow_push(app, f);
}

/// The checksum each backup flow is of.
fn flows_of(app: &faraday_core::Faraday) -> Vec<String> {
    app.backup_flows()
        .iter()
        .filter_map(|f| f.wallet.as_ref()?.checksum.as_ref().map(|t| t.to_string()))
        .collect()
}

#[test]
fn a_backup_of_the_wallet_taken_out_closes_with_its_own_flow_and_leaves_the_next_ones() {
    let mut app = forged_then_two_real();
    app.press(faraday_core::Action::Backup(0));
    assert!(app.backup().is_some());
    push_flow_of(&mut app, 0);
    push_flow_of(&mut app, 1);
    key_loads(&mut app);
    assert!(app.backup().is_none());
    assert_eq!(app.screen, faraday_core::Screen::Wallets);
    assert_eq!(
        flows_of(&app),
        [app.session.wallets[0].checksum().to_string()],
        "the flow of the wallet gone ended, the next wallet's stands"
    );
}

fn restoring(app: &faraday_core::Faraday) -> Option<String> {
    let w = app.restore()?.wallet?;
    Some(app.session.wallets.get(w)?.name.to_string())
}

/// A restore open on the wallet read from a scanned description, after
/// a forged wallet.
fn restore_after_forged() -> faraday_core::Faraday {
    use faraday_core::Action;
    use faraday_shell_api::{App as _, Event};
    let mut app = testkit::started();
    let c = testkit::compute();
    let forged_d = format!("wpkh({}/<0;1>/*)", forged("m/84'/1'/0'", "m/84'/1'/0'"));
    app.session
        .add_wallet_confirmed(&c, "Forged", &forged_d, "test", Origin::Named)
        .expect("no key is loaded: not checked");
    app.press(Action::RestoreWallet);
    app.press(Action::Scan);
    app.event(Event::Scanned {
        bytes: testkit::savings().into_bytes(),
    });
    testkit::confirm_wallets(&mut app);
    app
}

#[test]
fn a_restore_stays_on_its_wallet_when_an_earlier_one_is_taken_out() {
    let mut app = restore_after_forged();
    assert_eq!(app.session.wallets.len(), 2);
    let was = restoring(&app).expect("the restored wallet");
    key_loads(&mut app);
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(restoring(&app), Some(was));
}

#[test]
fn a_restore_ends_when_its_own_wallet_is_taken_out() {
    let mut app = restore_after_forged();
    let w = app.restore().and_then(|r| r.wallet).expect("its wallet");
    app.press(faraday_core::Action::RemoveWallet(w));
    assert!(app.restore().is_none());
    assert_ne!(
        app.screen,
        faraday_core::Screen::Restore,
        "the restore's page stays on a wallet that is gone"
    );
}

#[test]
fn a_restore_keeps_its_wallet_when_a_later_one_is_taken_out() {
    let mut app = restore_after_forged();
    let was = restoring(&app).expect("the restored wallet");
    let c = testkit::compute();
    let late = format!("wpkh({}/<0;1>/*)", testkit::key(0, "m/84'/1'/2'"));
    app.session
        .add_wallet_confirmed(&c, "Late", &late, "test", Origin::Named)
        .expect("added");
    let last = app.session.wallets.len() - 1;
    app.press(faraday_core::Action::RemoveWallet(last));
    assert_eq!(restoring(&app), Some(was));
}

/// The names of the wallets, in order.
fn wallet_names(app: &faraday_core::Faraday) -> Vec<String> {
    app.session
        .wallets
        .iter()
        .map(|w| w.name.to_string())
        .collect()
}

/// Two forged wallets and then two of the person's own; the key loads and
/// both forged wallets go in one event.
#[test]
fn two_wallets_taken_out_at_once_leave_the_backup_and_the_flows_of_those_that_stay() {
    use faraday_core::Action;
    let mut app = testkit::started();
    let c = testkit::compute();
    let f1 = format!("wpkh({}/<0;1>/*)", forged("m/84'/1'/0'", "m/84'/1'/0'"));
    let f2 = format!("wpkh({}/<0;1>/*)", forged("m/84'/1'/3'", "m/84'/1'/3'"));
    let real = format!("wpkh({}/<0;1>/*)", testkit::key(0, "m/84'/1'/1'"));
    let other = format!("wpkh({}/<0;1>/*)", testkit::key(0, "m/84'/1'/2'"));
    for (name, d) in [
        ("Forged", &f1),
        ("Forged2", &f2),
        ("Real", &real),
        ("Other", &other),
    ] {
        app.session
            .add_wallet_confirmed(&c, name, d, "test", Origin::Named)
            .expect("no key is loaded: not checked");
    }
    app.press(Action::Backup(0));
    push_flow_of(&mut app, 0);
    app.press(Action::Backup(1));
    push_flow_of(&mut app, 1);
    app.press(Action::Backup(3));
    push_flow_of(&mut app, 3);
    let other_sum = app.session.wallets[3].checksum().to_string();
    key_loads(&mut app);
    assert_eq!(wallet_names(&app), ["Real", "Other"]);
    assert_eq!(backing_up(&app).as_deref(), Some("Other"));
    assert_eq!(flows_of(&app), [other_sum]);
}

/// Back up of the wallet after the forged one is open while the forged
/// one's own flow waits: the forged wallet's flow ends, the open Back up
/// stays on its wallet.
#[test]
fn the_flow_of_a_wallet_taken_out_ends_while_another_wallets_backup_is_open() {
    use faraday_core::Action;
    let mut app = forged_then_two_real();
    app.press(Action::Backup(0));
    push_flow_of(&mut app, 0);
    app.press(Action::Backup(1));
    assert_eq!(backing_up(&app).as_deref(), Some("Real"));
    key_loads(&mut app);
    assert_eq!(backing_up(&app).as_deref(), Some("Real"));
    assert!(
        flows_of(&app).is_empty(),
        "a flow of a wallet that is gone stays in progress"
    );
}

/// The Krux code tool on the wallet that is then taken out: it asks what
/// the code is made of again, and shows no other wallet in its place.
#[test]
fn the_krux_code_tool_of_a_wallet_taken_out_shows_no_other_wallet() {
    use faraday_core::Action;
    use faraday_core::kef_tool::{KefToolAction as K, Pick};
    let mut app = forged_then_two_real();
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Krux code")
        .unwrap();
    app.press(Action::Nav(faraday_core::Screen::Catalog));
    app.press(Action::Catalog(tile as u8));
    app.press(Action::KefTool(K::Pick(Pick::Wallet)));
    app.press(Action::KefTool(K::Wallet(0)));
    assert!(
        faraday_core::kef_tool::state(&app)
            .unwrap()
            .form()
            .is_some(),
        "the wallet's code form opened"
    );
    key_loads(&mut app);
    assert!(
        faraday_core::kef_tool::state(&app)
            .unwrap()
            .form()
            .is_none(),
        "the form of the wallet gone stays open on another wallet"
    );
}
