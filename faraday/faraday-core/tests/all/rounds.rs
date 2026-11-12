//! A MuSig2 round's secret nonce: every pass that shares one draws it
//! afresh, so a transaction started again never shows the nonce it
//! showed before; a round waits for the system's randomness; and a
//! session whose nonce has signed is over.

use faraday_core::testkit;
use faraday_core::{Action, Faraday, StorageEvent};
use faraday_shell_api::{App, EntropyBytes, Event, Key};

fn kit_files(prefix: &str) -> Vec<(String, Vec<u8>)> {
    testkit::files()
        .unwrap()
        .into_iter()
        .filter(|(n, _)| n.starts_with(prefix))
        .collect()
}

pub(crate) fn at(app: &Faraday, name: &str) -> usize {
    app.inbox().iter().position(|i| i.name == name).unwrap()
}

/// Test key 1 and the MuSig2 wallet loaded, the unsigned spend and the
/// other signers' nonce copies in the Inbox; seeded when `seeded`.
pub(crate) fn device(seeded: bool) -> Faraday {
    device_with(seeded.then_some(5), Vec::new())
}

/// [`device`], seeded with `entropy` and with `extra` files in the Inbox
/// beside the kit's.
fn device_with(entropy: Option<u8>, extra: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = testkit::started();
    if let Some(e) = entropy {
        app.event(Event::Entropy(EntropyBytes::new([e; 32])));
    }
    let mut inbox = kit_files("musig-");
    inbox.extend(extra);
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(inbox),
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
pub(crate) fn our_nonce(app: &Faraday) -> Option<Vec<u8>> {
    let c = testkit::compute();
    let s = app.spend()?;
    let ours = app
        .session
        .keys
        .master_for_tests(0)
        .derive(&c, &"m/86'/1'/0'".parse().unwrap())
        .to_xpub(&c)
        .public_key;
    faraday_psbt::musig::pub_nonces(&s.spend.psbt.inner().inputs[0])
        .ok()?
        .into_iter()
        .find(|n| n.participant == ours)
        .map(|n| n.value.serialize().to_vec())
}

#[test]
fn a_transaction_started_again_shares_another_nonce() {
    let mut app = device(true);
    app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::SignHere);
    let first = our_nonce(&app).expect("round 1 shares this device's nonce");
    assert!(app.spend().unwrap().musig.is_some(), "the session is open");

    // The same unsigned transaction, started over: a new round.
    app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
    testkit::steps_before_sign_done(&mut app);
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
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::SignHere);
    let s = app.spend().unwrap();
    assert_eq!(s.error, None, "the press waits and says nothing");
    assert!(s.musig.is_none());
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "Waiting for randomness"),
        "the button says what it waits for"
    );
    assert!(
        our_nonce(&app).is_none(),
        "no nonce on bytes the system did not give"
    );

    // The shell answers, and the same press goes through by itself.
    app.event(Event::Entropy(EntropyBytes::new([5u8; 32])));
    testkit::settle(&mut app);
    assert!(our_nonce(&app).is_some());
    assert_eq!(app.spend().unwrap().error, None);
}

#[test]
fn a_session_whose_nonce_has_signed_is_over() {
    let mut app = device(true);
    app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::SignHere);
    assert!(app.spend().unwrap().musig.is_some());
    // The other signers' nonces arrive, and round 2 signs.
    let copies: Vec<usize> = app
        .inbox()
        .iter()
        .enumerate()
        .filter(|(_, i)| i.name.starts_with("musig-nonce-"))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(copies.len(), 2);
    for i in copies {
        app.press(Action::Collect(i));
    }
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::SignHere);
    let s = app.spend().unwrap();
    assert_eq!(s.error, None);
    assert!(!s.spend.signed_here.is_empty(), "round 2 signed");
    assert!(s.musig.is_none(), "nothing left to sign with");
}

/// Walks the steps before Nonces with Continue, as a person does.
fn continue_before_nonces(app: &mut Faraday) {
    let before: Vec<u8> = app
        .spend()
        .unwrap()
        .steps
        .iter()
        .copied()
        .take_while(|&n| n != faraday_core::wallet::step::NONCES)
        .collect();
    for n in before {
        app.press(Action::StepNext(n));
    }
}

#[test]
fn go_first_shares_this_devices_nonce_once_the_steps_before_nonces_are_done() {
    let mut app = device(true);
    app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
    continue_before_nonces(&mut app);
    assert!(
        !app.spend().unwrap().done[faraday_core::wallet::step::NONCES as usize],
        "Nonces is still to do"
    );
    app.press(Action::SignHere);
    assert!(our_nonce(&app).is_some(), "this device's nonce is shared");
    assert!(app.spend().unwrap().musig.is_some(), "round 1 is open");
}

#[test]
fn go_first_before_the_earlier_steps_shares_nothing() {
    let mut app = device(true);
    app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
    app.press(Action::SignHere);
    let s = app.spend().unwrap();
    assert!(s.musig.is_none());
    let first = s.steps.iter().copied().find(|&n| !s.done[n as usize]);
    assert_eq!(s.open, first);
    assert!(our_nonce(&app).is_none(), "no nonce shared");
}

/// A version 2 file that carries an older nonce of this device's, offered
/// for a new round 1: the file written back is version 2 and holds the
/// nonce this device's pass just wrote.
#[test]
fn a_version_2_file_with_an_older_nonce_of_this_device_is_written_back_with_the_new_one() {
    let c = testkit::compute();
    let mut first = device_with(Some(5), Vec::new());
    first.press(Action::StartSpend(at(&first, "musig-unsigned.psbt")));
    testkit::steps_before_sign_done(&mut first);
    first.press(Action::SignHere);
    let older = our_nonce(&first).expect("round 1 shares this device's nonce");
    let file = faraday_psbt::v2::PsbtV2::from_v0(&c, &first.spend().unwrap().spend.psbt).to_bytes();

    let mut app = device_with(Some(6), vec![("older.psbt".to_string(), file)]);
    app.press(Action::StartSpend(at(&app, "older.psbt")));
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::SignHere);
    let new = our_nonce(&app).expect("round 1 shares a nonce again");
    assert_ne!(older, new);
    let out = app
        .spend()
        .unwrap()
        .spend
        .out_bytes()
        .expect("the version 2 file takes the new nonce");
    let back = faraday_psbt::v2::PsbtV2::parse_bytes(&c, &out)
        .expect("written as version 2")
        .to_v0(&c)
        .unwrap();
    let ours = app
        .session
        .keys
        .master_for_tests(0)
        .derive(&c, &"m/86'/1'/0'".parse().unwrap())
        .to_xpub(&c)
        .public_key;
    let got = faraday_psbt::musig::pub_nonces(&back.inner().inputs[0])
        .unwrap()
        .into_iter()
        .find(|n| n.participant == ours)
        .map(|n| n.value.serialize().to_vec());
    assert_eq!(got, Some(new));
}
