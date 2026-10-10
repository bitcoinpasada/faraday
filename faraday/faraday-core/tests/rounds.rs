//! A MuSig2 round's secret nonce: every pass that shares one draws it
//! afresh, so a transaction started again never shows the nonce it
//! showed before; a round waits for the system's randomness; and a
//! session whose nonce has signed is over.

use faraday_core::testkit;
use faraday_core::{Action, Faraday, StorageEvent};
use osk_shell_api::{App, EntropyBytes, Event, Key};

fn kit_files(prefix: &str) -> Vec<(String, Vec<u8>)> {
    testkit::files()
        .unwrap()
        .into_iter()
        .filter(|(n, _)| n.starts_with(prefix))
        .collect()
}

fn at(app: &Faraday, name: &str) -> usize {
    app.inbox.iter().position(|i| i.name == name).unwrap()
}

/// Test key 1 and the MuSig2 wallet loaded, the unsigned spend and the
/// other signers' nonce copies in the Inbox; seeded when `seeded`.
fn device(seeded: bool) -> Faraday {
    let mut app = testkit::started();
    if seeded {
        app.event(Event::Entropy(EntropyBytes::new([5u8; 32])));
    }
    app.storage(StorageEvent::Restored {
        inbox: kit_files("musig-"),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(at(&app, "musig-wallet.txt")));
    app.press(Action::Entry(None));
    for c in testkit::test_words("bacon").chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    app
}

/// This device's public nonce on the spend, if one is there.
fn our_nonce(app: &Faraday) -> Option<Vec<u8>> {
    let s = app.spend.as_ref()?;
    let ours = app.session.keys[0]
        .master
        .derive(&"m/86'/1'/0'".parse().unwrap())
        .to_xpub()
        .public_key;
    osk_psbt::musig::pub_nonces(&s.spend.psbt.inner().inputs[0])
        .ok()?
        .into_iter()
        .find(|n| n.participant == ours)
        .map(|n| n.value.serialize().to_vec())
}

#[test]
fn a_transaction_started_again_shares_another_nonce() {
    let mut app = device(true);
    app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
    app.press(Action::SignHere);
    let first = our_nonce(&app).expect("round 1 shares this device's nonce");
    assert!(
        app.spend.as_ref().unwrap().musig.is_some(),
        "the session is open"
    );

    // The same unsigned transaction, started over: a new round.
    app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
    app.press(Action::SignHere);
    let second = our_nonce(&app).expect("round 1 again shares a nonce");
    assert_ne!(
        first, second,
        "a nonce shared again is never the one shared before"
    );
}

#[test]
fn a_round_waits_for_the_systems_randomness() {
    let mut app = device(false);
    app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
    app.press(Action::SignHere);
    let s = app.spend.as_ref().unwrap();
    assert_eq!(
        s.error.as_deref(),
        Some("No randomness from the system yet. Try again")
    );
    assert!(s.musig.is_none());
    assert!(
        our_nonce(&app).is_none(),
        "no nonce on bytes the system did not give"
    );

    // The shell answers, and the pass goes through.
    app.event(Event::Entropy(EntropyBytes::new([5u8; 32])));
    app.press(Action::SignHere);
    assert!(our_nonce(&app).is_some());
    assert_eq!(app.spend.as_ref().unwrap().error, None);
}

#[test]
fn a_session_whose_nonce_has_signed_is_over() {
    let mut app = device(true);
    app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
    app.press(Action::SignHere);
    assert!(app.spend.as_ref().unwrap().musig.is_some());
    // The other signers' nonces arrive, and round 2 signs.
    let copies: Vec<usize> = app
        .inbox
        .iter()
        .enumerate()
        .filter(|(_, i)| i.name.starts_with("musig-nonce-"))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(copies.len(), 2);
    for i in copies {
        app.press(Action::Collect(i));
    }
    app.press(Action::SignHere);
    let s = app.spend.as_ref().unwrap();
    assert_eq!(s.error, None);
    assert!(!s.spend.signed_here.is_empty(), "round 2 signed");
    assert!(s.musig.is_none(), "nothing left to sign with");
}
