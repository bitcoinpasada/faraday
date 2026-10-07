//! The signed-amount memory (`docs/WALLETS.md` §3.3): a transaction signed
//! here and offered again stating other amounts for its inputs is refused,
//! and the memory outlives a lock.

use faraday_core::testkit::{self, Kit};
use faraday_core::{Action, Faraday, StorageCommand, StorageEvent};
use osk_shell_api::{App, Event, Key};

fn kit() -> Kit {
    testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap()
}

fn spending() -> osk_psbt::Psbt {
    testkit::unsigned(&kit()).unwrap()
}

fn offer(app: &mut Faraday, psbt: &osk_psbt::Psbt, kept: Vec<(String, Vec<u8>)>) {
    app.storage(StorageEvent::Restored {
        inbox: vec![
            ("spend.psbt".to_string(), psbt.to_bytes()),
            (
                "spending-wallet.txt".to_string(),
                kit().descriptor.into_bytes(),
            ),
        ],
        outbox: Vec::new(),
        kept,
    });
    app.press(Action::LoadWallet(1));
    app.press(Action::Entry(None));
    for c in testkit::test_words("bacon").chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    app.press(Action::StartSpend(0));
    app.press(Action::SignHere);
}

fn kept(app: &mut Faraday) -> Vec<(String, Vec<u8>)> {
    let mut last = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { kept, .. } = c {
            last = kept;
        }
    }
    last
}

#[test]
fn the_same_transaction_with_other_amounts_is_refused_across_a_lock() {
    let psbt = spending();
    let mut app = Faraday::new();
    offer(&mut app, &psbt, Vec::new());
    assert!(!app.spend.as_ref().unwrap().spend.signed_here.is_empty());
    let kept = kept(&mut app);

    // A fresh process, as after a lock, offered the same transaction with
    // its input stating one satoshi more.
    let mut changed = psbt.inner().clone();
    changed.inputs[0].witness_utxo.as_mut().unwrap().value += osk_bip::bitcoin::Amount::from_sat(1);
    let changed = osk_psbt::Psbt::from(changed);
    let mut app = Faraday::new();
    offer(&mut app, &changed, kept.clone());
    let s = app.spend.as_ref().unwrap();
    assert!(s.spend.signed_here.is_empty());
    assert!(s.error.as_deref().is_some_and(|e| e.contains("refused")));

    // The same amounts again sign.
    let mut app = Faraday::new();
    offer(&mut app, &psbt, kept);
    assert!(!app.spend.as_ref().unwrap().spend.signed_here.is_empty());
}
