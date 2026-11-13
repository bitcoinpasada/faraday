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
    session.add_wallet(
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
            .add_wallet(&c, name, d, "test", Origin::Named)
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
