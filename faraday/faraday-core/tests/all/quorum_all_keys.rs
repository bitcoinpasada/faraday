//! A wallet whose keys must all sign (MuSig2, or a script needing both
//! keys) is read as needing every seed wherever the app counts the seeds
//! a spend needs, never as one that any one key spends; ordinary
//! multisig and single-key wallets read as they did.

use faraday_core::wallet::{Origin, Session};
use faraday_core::{Action, testkit};

fn both() -> String {
    let path = "m/48'/1'/0'/2'";
    format!(
        "wsh(and_v(v:pk({}/<0;1>/*),pk({}/<0;1>/*)))",
        testkit::key(0, path),
        testkit::key(1, path)
    )
}

fn musig() -> String {
    testkit::musig_policy()
        .expect("the MuSig2 kit")
        .to_descriptor_checksummed()
        .as_str()
        .to_owned()
}

/// What the restore flow takes the wallet's quorum to be after its
/// description is used.
fn restore_quorum(descriptor: &str) -> (usize, usize) {
    let mut app =
        crate::common::device(vec![("w.txt".to_string(), descriptor.as_bytes().to_vec())]);
    app.press(Action::RestoreWallet);
    app.press(Action::RUse(0));
    let r = app.restore().expect("restoring");
    assert!(r.wallet.is_some(), "{descriptor}: {:?}", r.error);
    let s = r.seeds.as_ref().expect("the seeds step");
    (s.m, s.n)
}

#[test]
fn restore_asks_for_every_seed_of_a_wallet_whose_keys_must_all_sign() {
    for (d, all) in [(both(), 2), (musig(), 3)] {
        assert_eq!(restore_quorum(&d), (all, all), "{d}");
    }
}

#[test]
fn the_seeds_a_spend_needs_are_all_of_them_for_a_wallet_whose_keys_must_all_sign() {
    let c = testkit::compute();
    for (d, all) in [(both(), 2), (musig(), 3)] {
        let mut app = crate::common::device(Vec::new());
        let i = app
            .session
            .add_wallet(&c, "Test", &d, "Test wallet", Origin::Named)
            .expect("reads");
        assert_eq!(Session::quorum(&app.session.wallets[i]), (all, all), "{d}");
    }
}

#[test]
fn ordinary_multisig_and_single_key_wallets_read_as_they_did() {
    let c = testkit::compute();
    let mut app = crate::common::device(Vec::new());
    for (id, want) in [("savings", (2, 3)), ("spending", (1, 1))] {
        let d = testkit::kits()
            .into_iter()
            .find(|k| k.id == id)
            .expect("kit")
            .descriptor;
        let i = app
            .session
            .add_wallet(&c, id, &d, "Test wallet", Origin::Named)
            .expect("reads");
        assert_eq!(Session::quorum(&app.session.wallets[i]), want, "{id}");
    }
}

/// A script stating no quorum, over keys 0 and 1 of seed A and key B or
/// C, as the policy with the given pieces.
fn a_policy(body: impl Fn(&str, &str, &str, &str) -> String) -> String {
    let path = "m/48'/1'/0'/2'";
    let a0 = format!("{}/<0;1>/*", testkit::key(0, path));
    let a1 = format!("{}/<0;1>/*", testkit::key(0, "m/48'/1'/1'/2'"));
    let b = format!("{}/<0;1>/*", testkit::key(1, path));
    let c = format!("{}/<0;1>/*", testkit::key(2, path));
    body(&a0, &a1, &b, &c)
}

fn added(app: &mut faraday_core::Faraday, d: &str) -> usize {
    let c = testkit::compute();
    app.session
        .add_wallet(&c, "Test", d, "Test wallet", Origin::Named)
        .expect("reads")
}

#[test]
fn two_keys_of_one_seed_and_another_read_as_two_of_two_seeds() {
    let d = a_policy(|a0, a1, b, _| format!("wsh(and_v(v:pk({a0}),and_v(v:pk({a1}),pk({b}))))"));
    assert_eq!(restore_quorum(&d), (2, 2), "{d}");
    let mut app = crate::common::device(Vec::new());
    let i = added(&mut app, &d);
    assert_eq!(Session::quorum(&app.session.wallets[i]), (2, 2));
    // Holding only seed A is one signature of two, not two of three.
    crate::common::add_key(&mut app, testkit::TEST_SEEDS[0].0);
    assert_eq!(app.session.held_toward_quorum(&app.session.wallets[i]), 1);
}

#[test]
fn a_mix_of_one_key_or_two_keys_reads_as_one_of_three_seeds() {
    let d = a_policy(|a0, _, b, c| format!("wsh(or_d(pk({a0}),and_v(v:pk({b}),pk({c}))))"));
    assert_eq!(restore_quorum(&d), (1, 3), "{d}");
}

#[test]
fn a_script_whose_ways_cannot_be_worked_out_asks_for_every_seed() {
    let d = a_policy(|a0, _, b, _| {
        format!(
            "wsh(and_v(v:pk({a0}),and_v(v:pk({b}),sha256(1111111111111111111111111111111111111111111111111111111111111111))))"
        )
    });
    let mut app = crate::common::device(Vec::new());
    let i = added(&mut app, &d);
    let (m, n) = Session::quorum(&app.session.wallets[i]);
    assert_eq!((m, n), (2, 2), "{d}");
}
