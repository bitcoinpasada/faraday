//! The steps of a spend with nothing to decide are passed after the
//! review's Continue; the review itself never is.

use crate::common::add_key;
use faraday_core::testkit::{self, Kit};
use faraday_core::wallet::step;
use faraday_core::{Action, Faraday, StorageEvent};

fn kit(id: &str) -> Kit {
    testkit::kits().into_iter().find(|k| k.id == id).unwrap()
}

/// The multisig "savings" spend open, with the keys `seeds` loaded.
fn opened(seeds: &[&str]) -> Faraday {
    let k = kit("savings");
    let (unsigned, _) = testkit::spend_of(&k).unwrap();
    opened_with(&k, unsigned.to_bytes(), seeds, true)
}

/// The spend `psbt` of wallet `k` open, with the keys `seeds` loaded; the
/// wallet loaded first or, with `load` false, left in Files.
fn opened_with(k: &Kit, psbt: Vec<u8>, seeds: &[&str], load: bool) -> Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            ("spend.psbt".to_string(), psbt),
            (
                "savings-wallet.txt".to_string(),
                k.descriptor.clone().into_bytes(),
            ),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    if load {
        load_from_files(&mut app);
    } else {
        // Loading the wallet would move the session to its network.
        testkit::move_to(&mut app.session, testkit::NET).unwrap();
    }
    for s in seeds {
        add_key(&mut app, s);
    }
    let at = app
        .inbox()
        .iter()
        .position(|i| i.name.ends_with(".psbt"))
        .unwrap();
    app.press(Action::StartSpend(at));
    app
}

fn load_from_files(app: &mut Faraday) {
    let wallet_at = app
        .inbox()
        .iter()
        .position(|i| i.name.ends_with("-wallet.txt"))
        .unwrap();
    testkit::load_wallet(app, wallet_at);
}

fn through_review(app: &mut Faraday) {
    app.press(Action::StepNext(step::WALLET));
    app.press(Action::StepNext(step::TRANSACTION));
}

#[test]
fn with_every_signature_held_here_signers_is_passed_after_check() {
    let mut app = opened(&["zebra", "bacon"]);
    through_review(&mut app);
    // Check compares addresses with the online wallet: it still asks.
    assert_eq!(app.spend().unwrap().open, Some(step::CHECK));
    app.press(Action::StepNext(step::CHECK));
    let s = app.spend().unwrap();
    assert_eq!(s.open, Some(step::SIGN));
    assert!(s.done[step::SIGNERS as usize]);
}

#[test]
fn with_a_signature_still_to_bring_signers_stops() {
    let mut app = opened(&["zebra"]);
    through_review(&mut app);
    app.press(Action::StepNext(step::CHECK));
    assert_eq!(app.spend().unwrap().open, Some(step::SIGNERS));
}

#[test]
fn nothing_is_signed_before_the_review_is_continued() {
    let mut app = opened(&["zebra", "bacon"]);
    let before = app.spend().unwrap().spend.psbt.to_bytes();
    app.press(Action::SignHere);
    let s = app.spend().unwrap();
    assert_eq!(s.spend.psbt.to_bytes(), before);
    assert_ne!(s.open, Some(step::SIGN));
}

#[test]
fn wallets_continue_stops_at_the_review() {
    let mut app = opened(&["zebra", "bacon"]);
    app.press(Action::StepNext(step::WALLET));
    let s = app.spend().unwrap();
    assert_eq!(s.open, Some(step::TRANSACTION));
    assert!(!s.done[step::TRANSACTION as usize]);
    assert!(!s.done[step::SIGNERS as usize]);
}

/// A spend with no wallet known has no addresses to compare: Check is
/// passed after the review. Loading the wallet then makes Check owed again,
/// and nothing is signed until it is done.
#[test]
fn check_passed_with_no_wallet_is_owed_again_once_the_wallet_loads() {
    // A single key's spend: the review does not wait for a registered
    // wallet, as a multisig's would.
    let k = kit("spending");
    let psbt = testkit::unsigned(&k).unwrap().to_bytes();
    let mut app = opened_with(&k, psbt, &["zebra"], false);
    through_review(&mut app);
    let s = app.spend().unwrap();
    assert!(s.done[step::CHECK as usize], "{:?} {:?}", s.open, s.error);
    assert_ne!(s.open, Some(step::CHECK));
    load_from_files(&mut app);
    let before = app.spend().unwrap().spend.psbt.to_bytes();
    app.press(Action::SignHere);
    let s = app.spend().unwrap();
    assert_eq!(s.spend.psbt.to_bytes(), before, "signed before Check");
    assert!(!s.done[step::CHECK as usize]);
}

/// A wallet whose spend path is still to be chosen keeps Signers open
/// even with every key held here.
#[test]
fn a_miniscript_wallet_keeps_signers_open() {
    let p = "m/48'/1'/0'/2'";
    let descriptor = format!(
        "wsh(or_d(pk({}/<0;1>/*),and_v(v:pkh({}/<0;1>/*),older(52560))))",
        testkit::key(0, p),
        testkit::key(1, p)
    );
    let k = Kit {
        id: "years",
        name: "Years",
        descriptor,
        cosigner: None,
        fee: 1_000,
    };
    let psbt = testkit::unsigned(&k).unwrap().to_bytes();
    let mut app = opened_with(&k, psbt, &["zebra", "bacon"], true);
    through_review(&mut app);
    app.press(Action::StepNext(step::CHECK));
    // The spend path is chosen first, then Signers still asks.
    assert_eq!(app.spend().unwrap().open, Some(step::PATH));
    app.press(Action::StepNext(step::PATH));
    let s = app.spend().unwrap();
    assert_eq!(s.open, Some(step::SIGNERS));
    assert!(!s.done[step::SIGNERS as usize]);
}

/// The savings spend with a second input copied from the Spending
/// wallet's spend, the wallets `loaded` first, then the keys of
/// Savings: every key of the spend's own wallet is held here.
fn with_a_foreign_input(loaded: &[&str]) -> Faraday {
    let k = kit("savings");
    let (unsigned, _) = testkit::spend_of(&k).unwrap();
    let other = testkit::unsigned(&kit("spending")).unwrap();
    let mut inner = unsigned.inner().clone();
    let tx_in = other.inner().unsigned_tx.input[0].clone();
    let input = other.inner().inputs[0].clone();
    // The fee stays as it was: the new input's value goes to the output.
    if let Some(utxo) = &input.witness_utxo {
        inner.unsigned_tx.output[0].value += utxo.value;
    }
    inner.unsigned_tx.input.push(tx_in);
    inner.inputs.push(input);
    let psbt = faraday_psbt::Psbt::from(inner);
    let mut inbox = vec![
        ("spend.psbt".to_string(), psbt.to_bytes()),
        testkit::wallet_file(&k),
    ];
    for id in loaded {
        inbox.push(testkit::wallet_file(&kit(id)));
    }
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(inbox),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    for file in std::iter::once("savings").chain(loaded.iter().copied()) {
        let name = format!("{file}-wallet.txt");
        let at = app.inbox().iter().position(|i| *i.name == *name).unwrap();
        testkit::load_wallet(&mut app, at);
    }
    for s in ["zebra", "bacon"] {
        add_key(&mut app, s);
    }
    let at = app
        .inbox()
        .iter()
        .position(|i| i.name == "spend.psbt")
        .unwrap();
    app.press(Action::StartSpend(at));
    app
}

/// An input of another wallet, or of none loaded, has signatures this
/// device's keys for the spend's own wallet do not give.
#[test]
fn an_input_of_another_wallet_keeps_signers_open() {
    for loaded in [&["spending"][..], &[][..]] {
        let mut app = with_a_foreign_input(loaded);
        app.press(Action::StepNext(step::WALLET));
        app.press(Action::StepNext(step::TRANSACTION));
        app.press(Action::StepNext(step::CHECK));
        let s = app.spend().unwrap();
        assert_eq!(s.open, Some(step::SIGNERS), "{loaded:?}: {:?}", s.error);
        assert!(!s.done[step::SIGNERS as usize]);
    }
}

/// A wallet that takes the checked one's place, its script the same under
/// another account path on its keys, is checked in its own right.
#[test]
fn check_is_owed_again_when_another_wallet_takes_the_checked_ones_place() {
    let k = kit("savings");
    let (unsigned, _) = testkit::spend_of(&k).unwrap();
    let twin = k
        .descriptor
        .split('#')
        .next()
        .unwrap()
        .replace("/2']", "/1']")
        .replace("/2h]", "/1h]");
    assert_ne!(twin, k.descriptor.split('#').next().unwrap());
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            ("spend.psbt".to_string(), unsigned.to_bytes()),
            testkit::wallet_file(&k),
            ("twin-wallet.txt".to_string(), twin.into_bytes()),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    for name in ["savings-wallet.txt", "twin-wallet.txt"] {
        let at = app.inbox().iter().position(|i| i.name == name).unwrap();
        testkit::load_wallet(&mut app, at);
    }
    assert_eq!(app.session.wallets.len(), 2, "the twin did not load");
    let at = app
        .inbox()
        .iter()
        .position(|i| i.name == "spend.psbt")
        .unwrap();
    app.press(Action::StartSpend(at));
    through_review(&mut app);
    app.press(Action::StepNext(step::CHECK));
    assert!(app.spend().unwrap().done[step::CHECK as usize]);
    let c = testkit::compute();
    let w = &app.session.wallets;
    assert_eq!(
        w[0].script_at(&c, false, 0),
        w[1].script_at(&c, false, 0),
        "the twin has another script"
    );
    app.press(Action::RemoveWallet(0));
    let s = app.spend().expect("the spend stays open");
    assert!(!s.done[step::CHECK as usize], "the twin was never checked");
}
