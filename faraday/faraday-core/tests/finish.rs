//! The end of signing: the nonce check recomputes every signature a key
//! held here made and says which came from elsewhere; the finished
//! transaction decodes to what it spends and pays, with its change marked
//! as the wallet's own; and a raw transaction in a text file decodes too.

use faraday_core::testkit::{self, Kit};
use faraday_core::wallet::FileKind;
use faraday_core::{Action, Faraday, Screen, StorageEvent};
use osk_shell_api::{App, Event, Key};

fn kit(id: &str) -> Kit {
    testkit::kits().into_iter().find(|k| k.id == id).unwrap()
}

fn add_key(app: &mut Faraday, seed: &str) {
    app.press(Action::Entry(None));
    for c in testkit::test_words(seed).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
}

fn with_inbox(files: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = Faraday::new();
    app.storage(StorageEvent::Restored {
        inbox: files,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

fn at(app: &Faraday, name: &str) -> usize {
    app.inbox.iter().position(|i| i.name == name).unwrap()
}

/// Spending, signed and finished here with test key 1.
fn spending_finished() -> Faraday {
    let k = kit("spending");
    let psbt = testkit::unsigned(&k).unwrap();
    let mut app = with_inbox(vec![
        ("spend.psbt".to_string(), psbt.to_bytes()),
        ("spending-wallet.txt".to_string(), k.descriptor.into_bytes()),
    ]);
    app.press(Action::LoadWallet(at(&app, "spending-wallet.txt")));
    add_key(&mut app, "bacon");
    app.press(Action::StartSpend(at(&app, "spend.psbt")));
    app.press(Action::SignHere);
    assert!(app.spend.as_ref().unwrap().spend.finished.is_some());
    app
}

#[test]
fn every_signature_made_here_is_recomputed() {
    let app = spending_finished();
    let s = app.spend.as_ref().unwrap();
    let check = app.session.nonce_check(&s.spend.psbt);
    assert!(!check.rows.is_empty());
    assert!(check.sound());
    assert!(
        check
            .rows
            .iter()
            .all(|r| matches!(r.recomputed, Some(Some(_)))),
        "a signature made here was not recomputed"
    );
}

#[test]
fn a_signature_from_another_device_is_checked_but_not_recomputed() {
    let k = kit("savings");
    let (unsigned, _) = testkit::spend_of(&k).unwrap();
    // Device A: test key 2 signs.
    let mut a = with_inbox(vec![
        ("spend.psbt".to_string(), unsigned.to_bytes()),
        (
            "savings-wallet.txt".to_string(),
            k.descriptor.clone().into_bytes(),
        ),
    ]);
    a.press(Action::LoadWallet(at(&a, "savings-wallet.txt")));
    add_key(&mut a, "zebra");
    a.press(Action::StartSpend(at(&a, "spend.psbt")));
    a.press(Action::SignHere);
    let from_a = a.spend.as_ref().unwrap().spend.psbt.to_bytes();

    // Device B: test key 1 signs and adds A's signature.
    let mut b = with_inbox(vec![
        ("spend.psbt".to_string(), unsigned.to_bytes()),
        ("savings-wallet.txt".to_string(), k.descriptor.into_bytes()),
        ("from-a.psbt".to_string(), from_a),
    ]);
    b.press(Action::LoadWallet(at(&b, "savings-wallet.txt")));
    add_key(&mut b, "bacon");
    b.press(Action::StartSpend(at(&b, "spend.psbt")));
    b.press(Action::SignHere);
    b.press(Action::Collect(at(&b, "from-a.psbt")));
    let s = b.spend.as_ref().unwrap();
    assert!(s.spend.finished.is_some(), "two of three did not finish it");
    let check = b.session.nonce_check(&s.spend.psbt);
    assert!(check.sound());
    let here = check
        .rows
        .iter()
        .filter(|r| matches!(r.recomputed, Some(Some(_))))
        .count();
    let elsewhere = check.rows.iter().filter(|r| r.recomputed.is_none()).count();
    assert!(here > 0 && elsewhere > 0);
    assert_eq!(here, elsewhere, "one of each per input");
}

#[test]
fn the_finished_transaction_decodes_to_what_it_pays() {
    let mut app = spending_finished();
    let tx = app.spend.as_ref().unwrap().spend.finished.clone().unwrap();
    app.press(Action::DecodeFinished);
    assert_eq!(app.screen, Screen::Decode);
    let d = &app.decode.as_ref().unwrap().decoded;
    assert_eq!(d.txid, tx.compute_txid().to_string());
    assert_eq!(d.outputs.len(), tx.output.len());
    assert!(
        d.fee.is_some_and(|f| f > 0),
        "the PSBT states the amounts going in"
    );
    assert!(
        d.outputs
            .iter()
            .any(|o| o.ours.as_ref().is_some_and(|w| w.0 == "Spending" && w.1)),
        "the change is not marked as Spending's"
    );
    assert!(
        d.outputs.iter().any(|o| o.ours.is_none()),
        "the payment goes out"
    );
}

#[test]
fn a_raw_transaction_in_a_text_file_decodes() {
    let app = spending_finished();
    let hex = app.spend.as_ref().unwrap().spend.finished_hex().unwrap();
    let txid = app
        .spend
        .as_ref()
        .unwrap()
        .spend
        .finished
        .as_ref()
        .unwrap()
        .compute_txid()
        .to_string();

    // Alone: decoded, the fee not stated.
    let mut alone = with_inbox(vec![(
        "broadcast-me.txt".to_string(),
        hex.clone().into_bytes(),
    )]);
    assert_eq!(alone.inbox[0].kind, FileKind::Transaction);
    alone.press(Action::DecodeInbox(0));
    let d = &alone.decode.as_ref().unwrap().decoded;
    assert_eq!(d.txid, txid);
    assert_eq!(d.fee, None);

    // Beside its PSBT: the fee too.
    let psbt = testkit::unsigned(&kit("spending")).unwrap();
    let mut beside = with_inbox(vec![
        ("broadcast-me.txt".to_string(), hex.into_bytes()),
        ("spend.psbt".to_string(), psbt.to_bytes()),
    ]);
    beside.press(Action::DecodeInbox(0));
    assert!(beside.decode.as_ref().unwrap().decoded.fee.is_some());
}
