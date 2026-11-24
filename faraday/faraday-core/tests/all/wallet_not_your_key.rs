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
