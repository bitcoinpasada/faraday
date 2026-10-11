//! Create › All keys · MuSig2: every key signs together, one Taproot key
//! on chain. The wallet made here is BIP-390's `tr(musig(…)/<0;1>/*)` over
//! the keys' Taproot accounts, and its count of signatures is always all
//! of its keys.

use faraday_core::create::NewKind;
use faraday_core::testkit;
use faraday_core::wallet::Kind;
use faraday_core::{Action, Faraday, cstep};
use osk_bip::policy::WalletPolicy;
use osk_shell_api::{App, Event, Key};

fn add_key(app: &mut Faraday, seed: &str) {
    app.press(Action::Entry(None));
    for c in testkit::test_words(seed).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
}

fn musig_index() -> u8 {
    NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::MuSig)
        .unwrap() as u8
}

#[test]
fn a_musig2_wallet_made_here_is_bip390s_over_the_taproot_accounts() {
    let mut app = faraday_core::testkit::started();
    add_key(&mut app, "bacon");
    add_key(&mut app, "zebra");
    app.press(Action::CreateWallet);
    app.press(Action::CKind(musig_index()));
    app.press(Action::CNext(cstep::KIND));
    // Two keys: one fewer than Create starts with.
    app.press(Action::CN(-1));
    let c = app.create.as_ref().unwrap();
    assert_eq!(c.n, 2);
    assert_eq!(c.m, c.n, "every key signs");
    app.press(Action::CNext(cstep::QUORUM));
    for (slot, k) in [(0u8, 0usize), (1, 1)] {
        let fp = app.session.keys[k].master.fingerprint().0;
        app.press(Action::CSlotHere(slot, fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    let built = app
        .create
        .as_ref()
        .unwrap()
        .built
        .expect("the wallet did not build");
    let made = &app.session.wallets[built].policy;
    assert_eq!(Kind::of(made), Kind::MuSig);

    // Each key's Taproot account on the session's network.
    let account = |k: usize| {
        NewKind::Taproot
            .key_text(&app.session.keys[k].master)
            .unwrap()
    };
    let (a, b) = (account(0), account(1));
    let want = WalletPolicy::from_parts("tr(musig(@0,@1)/**)", &[a.as_str(), b.as_str()]).unwrap();
    assert_eq!(made, &want);
    assert!(made.address_at(app.session.network(), false, 0).is_ok());
}

#[test]
fn the_signatures_needed_follow_the_keys() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::CreateWallet);
    app.press(Action::CKind(musig_index()));
    let n = app.create.as_ref().unwrap().n;
    app.press(Action::CN(1));
    app.press(Action::CM(-1));
    let c = app.create.as_ref().unwrap();
    assert_eq!(c.n, n + 1);
    assert_eq!(c.m, n + 1, "MuSig2 needs every key; fewer cannot be chosen");
}
