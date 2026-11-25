//! The end of signing: the nonce check recomputes every signature a key
//! held here made and says which came from elsewhere, and calls none
//! valid that it could not check; the finished
//! transaction decodes to what it spends and pays, with its change marked
//! as the wallet's own; and a raw transaction in a text file decodes too.
//! Before signing, the card after Wallet is Transaction, with the txid
//! at its foot, and a wallet checked once this power-on has Check closed
//! on its next spend, across a lock, until the machine starts afresh.

#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "a test or example builds keys and seeds directly and is not shipped (docs/KEYS.md)"
)]

use crate::common::add_key;
use faraday_core::testkit::Plain;
use faraday_core::testkit::{self, Kit};
use faraday_core::wallet::{FileKind, step};
use faraday_core::{Action, Faraday, Screen, StorageCommand, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

fn kit(id: &str) -> Kit {
    testkit::kits().into_iter().find(|k| k.id == id).unwrap()
}

fn with_inbox(files: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = faraday_core::testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(files),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

fn waiting(app: &Faraday, stem: &str) -> Vec<String> {
    app.outbox
        .iter()
        .filter(|i| i.name.starts_with(stem))
        .map(|i| String::from(&*i.name))
        .collect()
}

/// A signed PSBT put For the stick again makes no second file, and one
/// that differs from the file of its name goes beside it.
#[test]
fn a_signed_psbt_goes_beside_a_different_file_of_its_name() {
    let mut app = spending_finished();
    assert_eq!(waiting(&app, "spend-signed"), ["spend-signed.psbt"]);
    app.press(Action::SignedToOutbox);
    assert_eq!(waiting(&app, "spend-signed"), ["spend-signed.psbt"]);
    app.outbox
        .iter_mut()
        .find(|i| i.name == "spend-signed.psbt")
        .unwrap()
        .bytes = (b"another".to_vec()).into();
    app.press(Action::SignedToOutbox);
    assert_eq!(
        waiting(&app, "spend-signed"),
        ["spend-signed.psbt", "spend-signed-2.psbt"]
    );
}

/// The part-signed PSBT of a transaction is replaced by the next pass of
/// the same transaction; a different transaction's file of the name stays.
#[test]
fn a_part_signed_psbt_replaces_its_own_transaction_and_keeps_another() {
    let k = kit("savings");
    let (unsigned, _) = testkit::spend_of(&k).unwrap();
    let sign_with = |waiting_part: Vec<u8>| {
        let mut app = faraday_core::testkit::started();
        app.storage(StorageEvent::Restored {
            inbox: faraday_core::files_from(vec![
                (faraday_text::Text::of("spend.psbt"), unsigned.to_bytes()),
                (
                    faraday_text::Text::of("savings-wallet.txt"),
                    k.descriptor.clone().into_bytes(),
                ),
            ]),
            outbox: faraday_core::files_from(vec![(
                faraday_text::Text::of("spend-part.psbt"),
                waiting_part,
            )]),
            kept: Vec::new(),
        });
        let wallet_at = at(&app, "savings-wallet.txt");
        testkit::load_wallet(&mut app, wallet_at);
        add_key(&mut app, "zebra");
        app.press(Action::StartSpend(at(&app, "spend.psbt")));
        testkit::steps_before_sign_done(&mut app);
        app.press(Action::SignHere);
        app
    };
    let same = sign_with(unsigned.to_bytes());
    assert_eq!(waiting(&same, "spend-part"), ["spend-part.psbt"]);
    let other = sign_with(b"psbt of another transaction".to_vec());
    assert_eq!(
        waiting(&other, "spend-part"),
        ["spend-part.psbt", "spend-part-2.psbt"]
    );
}

fn at(app: &Faraday, name: &str) -> usize {
    app.inbox().iter().position(|i| i.name == name).unwrap()
}

/// A spend whose files went out beside another spend's of the same names
/// offers Remove on its own files and not on the other's.
#[test]
fn a_second_spends_remove_takes_off_its_own_file() {
    let k = kit("spending");
    let psbt = testkit::unsigned(&k).unwrap();
    let mut app = faraday_core::testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (faraday_text::Text::of("spend.psbt"), psbt.to_bytes()),
            (
                faraday_text::Text::of("spending-wallet.txt"),
                k.descriptor.into_bytes(),
            ),
        ]),
        outbox: faraday_core::files_from(vec![
            (
                faraday_text::Text::of("spend-signed.psbt"),
                b"first spend".to_vec(),
            ),
            (
                faraday_text::Text::of("spend-final.txn"),
                b"first spend".to_vec(),
            ),
        ]),
        kept: Vec::new(),
    });
    let wallet_at = at(&app, "spending-wallet.txt");
    testkit::load_wallet(&mut app, wallet_at);
    add_key(&mut app, "bacon");
    app.press(Action::StartSpend(at(&app, "spend.psbt")));
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::SignHere);
    let index = |app: &Faraday, name: &str| app.outbox.iter().position(|i| i.name == name).unwrap();
    let (first, own) = (
        index(&app, "spend-signed.psbt"),
        index(&app, "spend-signed-2.psbt"),
    );
    let (first_tx, own_tx) = (
        index(&app, "spend-final.txn"),
        index(&app, "spend-final-2.txn"),
    );
    let _ = app.frame();
    assert!(app.offers(Action::OutboxRemove(own)));
    assert!(app.offers(Action::OutboxRemove(own_tx)));
    assert!(!app.offers(Action::OutboxRemove(first)));
    assert!(!app.offers(Action::OutboxRemove(first_tx)));
}

/// Spending, signed and finished here with test key 1.
fn spending_finished() -> Faraday {
    spending_finished_with(Vec::new())
}

/// As [`spending_finished`], with other files already waiting For the stick.
fn spending_finished_with(waiting: Vec<(&str, Vec<u8>)>) -> Faraday {
    let k = kit("spending");
    let psbt = testkit::unsigned(&k).unwrap();
    let mut app = faraday_core::testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            ("spend.psbt".to_string(), psbt.to_bytes()),
            ("spending-wallet.txt".to_string(), k.descriptor.into_bytes()),
        ]),
        outbox: faraday_core::files_from(
            waiting
                .into_iter()
                .map(|(n, b)| (n.to_string(), b))
                .collect::<Vec<_>>(),
        ),
        kept: Vec::new(),
    });
    let wallet_at = at(&app, "spending-wallet.txt");
    testkit::load_wallet(&mut app, wallet_at);
    add_key(&mut app, "bacon");
    app.press(Action::StartSpend(at(&app, "spend.psbt")));
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::SignHere);
    assert!(app.spend().unwrap().spend.finished.is_some());
    app
}

#[test]
fn every_signature_made_here_is_recomputed() {
    let c = testkit::compute();
    let app = spending_finished();
    let s = app.spend().unwrap();
    let check = app.session.nonce_check(&c, &s.spend.psbt);
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
    let c = testkit::compute();
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
    let wallet_at = at(&a, "savings-wallet.txt");
    testkit::load_wallet(&mut a, wallet_at);
    add_key(&mut a, "zebra");
    a.press(Action::StartSpend(at(&a, "spend.psbt")));
    testkit::steps_before_sign_done(&mut a);
    a.press(Action::SignHere);
    let from_a = a.spend().unwrap().spend.psbt.to_bytes();

    // Device B: test key 1 signs and adds A's signature.
    let mut b = with_inbox(vec![
        ("spend.psbt".to_string(), unsigned.to_bytes()),
        ("savings-wallet.txt".to_string(), k.descriptor.into_bytes()),
        ("from-a.psbt".to_string(), from_a),
    ]);
    let wallet_at = at(&b, "savings-wallet.txt");
    testkit::load_wallet(&mut b, wallet_at);
    add_key(&mut b, "bacon");
    b.press(Action::StartSpend(at(&b, "spend.psbt")));
    testkit::steps_before_sign_done(&mut b);
    b.press(Action::SignHere);
    b.press(Action::Collect(at(&b, "from-a.psbt")));
    let s = b.spend().unwrap();
    assert!(s.spend.finished.is_some(), "two of three did not finish it");
    let check = b.session.nonce_check(&c, &s.spend.psbt);
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

/// A signature on an input that carries no previous output cannot be
/// checked: the nonce check's headline says how many were not checked,
/// in place of calling them valid.
#[test]
fn a_signature_that_could_not_be_checked_is_not_called_valid() {
    let c = testkit::compute();
    let app = spending_finished();
    let signed = &app.spend().unwrap().spend.psbt;
    let valid = app.session.nonce_check(&c, signed);
    assert!(valid.sound());

    let mut stripped = signed.clone();
    let input = &mut stripped.inner_mut().inputs[0];
    input.witness_utxo = None;
    input.non_witness_utxo = None;
    let check = app.session.nonce_check(&c, &stripped);
    assert!(
        check.unchecked() > 0,
        "the signature was checked without its output"
    );
    assert!(!check.sound());
    assert!(!check.wrong(), "nothing was found wrong, only not checked");
    assert_ne!(check.headline(), valid.headline());
    assert!(
        check.headline().contains(&check.unchecked().to_string()),
        "{}",
        check.headline()
    );
}

/// Opened from Sign a transaction on a wide window, the Result card is
/// the step opened on, and its way back goes to the signing.
#[test]
fn the_finished_transaction_opens_on_the_result_card_and_goes_back_to_signing() {
    use faraday_core::strings::EN;
    let mut app = spending_finished();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.press(Action::DecodeFinished);
    assert_eq!(app.screen, Screen::Decode);
    assert_eq!(app.decode().unwrap().back, Screen::Spend);
    assert!(crate::common::has(&mut app, EN.decode_transaction));
    assert!(crate::common::offered(&mut app, Action::Nav(Screen::Spend)));
    assert!(crate::common::offered(
        &mut app,
        Action::ScanFor(faraday_core::Ask::Decode)
    ));
}

#[test]
fn the_finished_transaction_decodes_to_what_it_pays() {
    let mut app = spending_finished();
    let tx = app.spend().unwrap().spend.finished.clone().unwrap();
    app.press(Action::DecodeFinished);
    assert_eq!(app.screen, Screen::Decode);
    let d = &app.decode().unwrap().decoded;
    assert_eq!(d.txid.as_str(), tx.compute_txid().to_string());
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
    let hex = app.spend().unwrap().spend.finished_hex().unwrap();
    let txid = app
        .spend()
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
        hex.as_bytes().to_vec(),
    )]);
    assert_eq!(alone.inbox()[0].kind, FileKind::Transaction);
    alone.press(Action::DecodeInbox(0));
    let d = &alone.decode().unwrap().decoded;
    assert_eq!(d.txid.as_str(), txid);
    assert_eq!(d.fee, None);

    // Beside its PSBT: the fee too.
    let psbt = testkit::unsigned(&kit("spending")).unwrap();
    let mut beside = with_inbox(vec![
        ("broadcast-me.txt".to_string(), hex.as_bytes().to_vec()),
        ("spend.psbt".to_string(), psbt.to_bytes()),
    ]);
    beside.press(Action::DecodeInbox(0));
    assert!(beside.decode().unwrap().decoded.fee.is_some());
}

/// Spending offered to a process that starts from `kept`: the wallet
/// loaded, test key 1 added and the spend opened, nothing signed.
fn spending_offered(kept: Vec<(String, Vec<u8>)>) -> Faraday {
    let k = kit("spending");
    let psbt = testkit::unsigned(&k).unwrap();
    let mut app = faraday_core::testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (faraday_text::Text::of("spend.psbt"), psbt.to_bytes()),
            (
                faraday_text::Text::of("spending-wallet.txt"),
                k.descriptor.into_bytes(),
            ),
        ]),
        outbox: Vec::new(),
        kept: faraday_core::kept_from(kept),
    });
    let wallet_at = at(&app, "spending-wallet.txt");
    testkit::load_wallet(&mut app, wallet_at);
    add_key(&mut app, "bacon");
    app.press(Action::StartSpend(at(&app, "spend.psbt")));
    assert_eq!(app.screen, Screen::Spend);
    app
}

/// Whether the spend's Check card is closed, as for a wallet checked
/// already.
fn check_closed(app: &Faraday) -> bool {
    app.spend().unwrap().done[step::CHECK as usize]
}

/// What a lock keeps, as the shell is told to save it.
fn kept_after_lock(app: &mut Faraday) -> Vec<(String, Vec<u8>)> {
    app.press(Action::Lock);
    let mut last = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { kept, .. } = c {
            last = faraday_core::testkit::plain_kept(kept);
        }
    }
    last
}

#[test]
fn the_card_after_wallet_is_transaction_and_it_carries_the_txid() {
    let mut app = spending_offered(Vec::new());
    let s = app.spend().unwrap();
    assert_eq!(&s.steps[..2], &[step::WALLET, step::TRANSACTION]);
    let txid = s.spend.txid.to_string();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Txid"), "{texts:?}");
    let head = format!("{} {}", &txid[..4], &txid[4..8]);
    assert!(
        texts.iter().any(|t| t.contains(&head)),
        "the Transaction card shows the txid {txid}: {texts:?}"
    );
}

#[test]
fn a_wallet_never_checked_has_check_open() {
    let app = spending_offered(Vec::new());
    assert!(!check_closed(&app));
}

#[test]
fn a_checked_wallet_has_check_closed_on_its_next_spend_and_after_a_lock() {
    let mut app = spending_offered(Vec::new());
    assert!(!check_closed(&app));
    app.press(Action::StepNext(step::CHECK));
    // The next spend from it, in this process.
    app.press(Action::StartSpend(at(&app, "spend.psbt")));
    assert!(check_closed(&app), "checked this power-on");
    // After a lock, the next process starts from what the lock kept.
    let kept = kept_after_lock(&mut app);
    let app = spending_offered(kept);
    assert!(check_closed(&app), "still checked after a lock");
    // A fresh process, with nothing kept, has checked nothing.
    let app = spending_offered(Vec::new());
    assert!(!check_closed(&app), "open again in a fresh process");
}

/// At Finish the primary does not sign (it locks and writes): its box is
/// pinned at the pane's foot, in view unscrolled even on a low window.
#[test]
fn at_finish_the_box_is_pinned_in_view_on_a_low_window() {
    for size in [(1280, 800), (1366, 600)] {
        let mut app = spending_finished();
        app.event(Event::Display(DisplayInfo {
            width: size.0,
            height: size.1,
            dpi: 160,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: false,
            secure: SecureHardware::None,
            boot: BootState::Unknown,
            memory_mib: None,
        }));
        app.settle();
        assert!(
            crate::common::offered(&mut app, Action::Primary),
            "the primary is in view unscrolled at {size:?}"
        );
    }
}

/// Finishing a spend puts both results For the stick with no further
/// press: the signed PSBT and the finished
/// transaction. Each card offers Remove in their place, and the primary
/// is "Lock and write to the stick", which locks, and writes them to the
/// stick that comes back.
#[test]
fn finishing_a_spend_puts_both_files_for_the_stick() {
    let mut app = spending_finished();
    let names: Vec<&str> = app.outbox.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains(&"spend-signed.psbt"), "{names:?}");
    assert!(names.contains(&"spend-final.txn"), "{names:?}");
    let signed = app
        .outbox
        .iter()
        .position(|i| i.name == "spend-signed.psbt")
        .unwrap();
    let _ = app.frame();
    assert!(app.offers(Action::OutboxRemove(signed)));
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Lock and write to the stick"),
        "the primary locks and writes: {texts:?}"
    );
    app.press(Action::Primary);
    assert_ne!(app.screen, Screen::Files, "no trip through Files");
}

/// The amounts going in come from a PSBT of exactly this transaction,
/// and Decode names it; two PSBTs of it that state other amounts give
/// none.
#[test]
fn decode_names_the_psbt_the_amounts_going_in_came_from() {
    let app = spending_finished();
    let hex = app.spend().unwrap().spend.finished_hex().unwrap();
    let psbt = testkit::unsigned(&kit("spending")).unwrap();
    let mut beside = with_inbox(vec![
        ("broadcast-me.txt".to_string(), hex.as_bytes().to_vec()),
        ("spend.psbt".to_string(), psbt.to_bytes()),
    ]);
    beside.press(Action::DecodeInbox(0));
    let d = &beside.decode().unwrap().decoded;
    assert!(d.fee.is_some());
    let _ = beside.frame();
    let text = beside.drawn_texts().join("\n");
    assert!(text.contains("From spend.psbt"), "{text}");

    // Another PSBT of the same transaction that states another amount.
    let mut other = psbt.clone();
    // Its amount stands in a `witness_utxo` alone: beside a previous
    // transaction of the right txid a `witness_utxo` is not what is read.
    other.inner_mut().inputs[0].non_witness_utxo = None;
    let utxo = other.inner_mut().inputs[0].witness_utxo.as_mut().unwrap();
    utxo.value += faraday_bip::bitcoin::Amount::from_sat(1000);
    let mut both = with_inbox(vec![
        ("broadcast-me.txt".to_string(), hex.as_bytes().to_vec()),
        ("spend.psbt".to_string(), psbt.to_bytes()),
        ("other.psbt".to_string(), other.to_bytes()),
    ]);
    both.press(Action::DecodeInbox(0));
    let d = &both.decode().unwrap().decoded;
    assert_eq!(d.fee, None, "amounts that disagree give no fee");
    let _ = both.frame();
    let text = both.drawn_texts().join("\n");
    assert!(!text.contains("From spend.psbt"), "{text}");
}

/// Savings' unsigned spend with test key 2's good signature on every
/// input, made under `ty` where the PSBT asks for nothing: a cosigner
/// that answered another question than the one asked.
pub(crate) fn signed_by_zebra_under(
    ty: faraday_bip::bitcoin::sighash::EcdsaSighashType,
) -> Vec<u8> {
    use faraday_bip::bitcoin::secp256k1::{Message, Secp256k1};
    use faraday_bip::bitcoin::sighash::SighashCache;
    use faraday_bip::bitcoin::{PublicKey, ecdsa, hashes::Hash};
    let c = testkit::compute();
    let (mut psbt, _) = testkit::spend_of(&kit("savings")).unwrap();
    let words = testkit::test_words("zebra");
    let m = faraday_bip::bip39::Mnemonic::parse(&c, faraday_bip::bip39::Language::English, &words)
        .unwrap();
    let zebra =
        faraday_bip::keys::MasterKey::from_seed(&c, &m.to_seed(&c, b"").unwrap(), testkit::net());
    let tx = psbt.unsigned_tx().clone();
    let secp = Secp256k1::new();
    for i in 0..tx.input.len() {
        let input = psbt.inner().inputs[i].clone();
        let (pk, (_, path)) = input
            .bip32_derivation
            .iter()
            .find(|(_, (fp, _))| fp.to_bytes() == zebra.fingerprint().0)
            .expect("test key 2 is a key of Savings");
        let derived = zebra.derive(&c, path);
        let ws = input.witness_script.clone().expect("a P2WSH input");
        let value = input.witness_utxo.as_ref().expect("its output").value;
        let hash = SighashCache::new(&tx)
            .p2wsh_signature_hash(i, &ws, value, ty)
            .unwrap();
        let msg = Message::from_digest(hash.to_byte_array());
        psbt.inner_mut().inputs[i].partial_sigs.insert(
            PublicKey::new(*pk),
            ecdsa::Signature {
                signature: secp.sign_ecdsa(&msg, derived.secret_key()),
                sighash_type: ty,
            },
        );
    }
    psbt.to_bytes()
}

/// A cosigner's signature made under a sighash other than the one the
/// transaction asks for is refused when it is collected, though it is a
/// good signature: SIGHASH_NONE signs no output, and anyone could send
/// the money elsewhere.
#[test]
fn a_cosigners_signature_under_another_sighash_is_refused() {
    use faraday_bip::bitcoin::sighash::EcdsaSighashType;
    let k = kit("savings");
    let (unsigned, _) = testkit::spend_of(&k).unwrap();
    let mut b = with_inbox(vec![
        ("spend.psbt".to_string(), unsigned.to_bytes()),
        (
            "savings-wallet.txt".to_string(),
            k.descriptor.clone().into_bytes(),
        ),
        (
            "from-a.psbt".to_string(),
            signed_by_zebra_under(EcdsaSighashType::None),
        ),
        (
            "from-a-all.psbt".to_string(),
            signed_by_zebra_under(EcdsaSighashType::All),
        ),
    ]);
    let wallet_at = at(&b, "savings-wallet.txt");
    testkit::load_wallet(&mut b, wallet_at);
    add_key(&mut b, "bacon");
    b.press(Action::StartSpend(at(&b, "spend.psbt")));
    testkit::steps_before_sign_done(&mut b);
    b.press(Action::SignHere);
    let before = b.spend().unwrap().spend.psbt.to_bytes();
    b.press(Action::Collect(at(&b, "from-a.psbt")));
    let s = b.spend().unwrap();
    assert_eq!(s.spend.psbt.to_bytes(), before, "the signature was taken");
    assert!(s.spend.finished.is_none());
    assert!(
        s.error
            .as_deref()
            .is_some_and(|e| e.contains("from-a.psbt")),
        "{:?}",
        s.error
    );
    // The same cosigner under SIGHASH_ALL is taken, and finishes it.
    b.press(Action::Collect(at(&b, "from-a-all.psbt")));
    assert!(b.spend().unwrap().spend.finished.is_some());
}

/// A cosigner's PSBT that writes SIGHASH_ALL where this one writes no
/// sighash asks the same of every input: its good signature is taken.
#[test]
fn a_cosigner_writing_sighash_all_where_none_is_written_is_taken() {
    use faraday_bip::bitcoin::psbt::PsbtSighashType;
    use faraday_bip::bitcoin::sighash::EcdsaSighashType;
    let c = testkit::compute();
    let k = kit("savings");
    let (unsigned, _) = testkit::spend_of(&k).unwrap();
    let signed = signed_by_zebra_under(EcdsaSighashType::All);
    let mut written = faraday_psbt::Psbt::parse_bytes(&c, &signed).unwrap();
    for i in &mut written.inner_mut().inputs {
        i.sighash_type = Some(PsbtSighashType::from(EcdsaSighashType::All));
    }
    let mut b = with_inbox(vec![
        ("spend.psbt".to_string(), unsigned.to_bytes()),
        (
            "savings-wallet.txt".to_string(),
            k.descriptor.clone().into_bytes(),
        ),
        ("from-a.psbt".to_string(), written.to_bytes()),
    ]);
    let wallet_at = at(&b, "savings-wallet.txt");
    testkit::load_wallet(&mut b, wallet_at);
    add_key(&mut b, "bacon");
    b.press(Action::StartSpend(at(&b, "spend.psbt")));
    testkit::steps_before_sign_done(&mut b);
    b.press(Action::SignHere);
    b.press(Action::Collect(at(&b, "from-a.psbt")));
    let s = b.spend().unwrap();
    assert_eq!(s.error, None);
    assert!(s.spend.finished.is_some(), "the signature was not taken");
}

/// A PSBT of the transaction that leaves an input's amount out does not
/// disagree with one that states it: the fee is still worked out. Two
/// that state different amounts are said to.
#[test]
fn a_psbt_without_the_amounts_does_not_disagree_with_one_that_has_them() {
    let app = spending_finished();
    let hex = app.spend().unwrap().spend.finished_hex().unwrap();
    let psbt = testkit::unsigned(&kit("spending")).unwrap();
    let mut bare = psbt.clone();
    for i in &mut bare.inner_mut().inputs {
        i.witness_utxo = None;
        i.non_witness_utxo = None;
    }
    let mut both = with_inbox(vec![
        ("broadcast-me.txt".to_string(), hex.as_bytes().to_vec()),
        ("bare.psbt".to_string(), bare.to_bytes()),
        ("spend.psbt".to_string(), psbt.to_bytes()),
    ]);
    both.press(Action::DecodeInbox(0));
    assert!(both.decode().unwrap().decoded.fee.is_some());
    let _ = both.frame();
    let text = both.drawn_texts().join("\n");
    assert!(text.contains("From spend.psbt"), "{text}");

    let mut other = psbt.clone();
    // Its amount stands in a `witness_utxo` alone: beside a previous
    // transaction of the right txid a `witness_utxo` is not what is read.
    other.inner_mut().inputs[0].non_witness_utxo = None;
    let utxo = other.inner_mut().inputs[0].witness_utxo.as_mut().unwrap();
    utxo.value += faraday_bip::bitcoin::Amount::from_sat(1000);
    let mut differ = with_inbox(vec![
        ("broadcast-me.txt".to_string(), hex.as_bytes().to_vec()),
        ("spend.psbt".to_string(), psbt.to_bytes()),
        ("other.psbt".to_string(), other.to_bytes()),
    ]);
    differ.press(Action::DecodeInbox(0));
    let _ = differ.frame();
    let text = differ.drawn_texts().join("\n");
    assert!(
        text.contains("PSBTs of it state different amounts"),
        "{text}"
    );
}

/// Decode a transaction on a 1920 × 1080 window lays its outputs out as
/// a table as wide as its columns (`docs/LAYOUT.md` §17): each output's
/// amount stands just past the longest address or outpoint, not at the
/// far end of the page's column.
#[test]
fn decodes_outputs_stand_at_their_columns_width_on_a_wide_window() {
    use faraday_shell_api::{BootState, DisplayInfo, Event, SecureHardware};
    let app = spending_finished();
    let hex = app.spend().unwrap().spend.finished_hex().unwrap();
    let mut app = with_inbox(vec![(
        "broadcast-me.txt".to_string(),
        hex.as_bytes().to_vec(),
    )]);
    app.event(Event::Display(DisplayInfo {
        width: 1920,
        height: 1080,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.press(Action::DecodeInbox(0));
    let _ = app.frame();
    let amounts: Vec<String> = app
        .decode()
        .unwrap()
        .decoded
        .outputs
        .iter()
        .map(|o| String::from(&*faraday_core::decode::btc(o.amount)))
        .collect();
    let texts = app.drawn_text_boxes().plain();
    // Each amount's end and the end of the address on its row.
    let rows: Vec<(i32, i32)> = amounts
        .iter()
        .map(|a| {
            let (_, (ax, ay, aw, _)) = texts
                .iter()
                .find(|(t, _)| t == a)
                .unwrap_or_else(|| panic!("the amount {a} drawn"));
            let row_end = texts
                .iter()
                .filter(|(t, b)| t != a && (b.1 - ay).abs() < 6 && b.0 < *ax)
                .map(|(_, b)| b.0 + b.2)
                .max()
                .expect("the address beside the amount");
            (ax + aw, row_end)
        })
        .collect();
    // The longest value of the table: an address, or an input's outpoint.
    let outpoints: Vec<String> = app
        .decode()
        .unwrap()
        .decoded
        .inputs
        .iter()
        .map(|i| String::from(&*i.outpoint))
        .collect();
    let longest = texts
        .iter()
        .filter(|(t, _)| outpoints.contains(t))
        .map(|(_, b)| b.0 + b.2)
        .chain(rows.iter().map(|r| r.1))
        .max()
        .expect("an output");
    for (end, _) in rows {
        let gap = end - longest;
        assert!(
            gap <= 220,
            "an amount stands {gap} px past the longest address, not at the column's width"
        );
    }
}

/// With the three keys of Savings here, the main button reads whole: the
/// keys counted where their names would not fit it, never cut short.
#[test]
fn the_sign_button_with_three_keys_here_reads_whole() {
    let k = kit("savings");
    let psbt = testkit::unsigned(&k).unwrap();
    let mut app = with_inbox(vec![
        ("spend.psbt".to_string(), psbt.to_bytes()),
        ("savings-wallet.txt".to_string(), k.descriptor.into_bytes()),
    ]);
    let wallet_at = at(&app, "savings-wallet.txt");
    testkit::load_wallet(&mut app, wallet_at);
    for seed in ["bacon", "zebra", "summer"] {
        add_key(&mut app, seed);
    }
    // A window tall enough for the whole of Your choices pane, where the
    // button stands under the review's facts.
    app.event(faraday_shell_api::Event::Display(
        faraday_shell_api::DisplayInfo {
            width: 1280,
            height: 1600,
            dpi: 160,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: false,
            secure: faraday_shell_api::SecureHardware::None,
            boot: faraday_shell_api::BootState::Unknown,
            memory_mib: None,
        },
    ));
    app.press(Action::StartSpend(at(&app, "spend.psbt")));
    assert_eq!(app.screen, Screen::Spend);
    let t = app.drawn_texts();
    assert!(
        !t.iter()
            .any(|s| s.starts_with("Sign with") && s.ends_with('…')),
        "{t:?}"
    );
    assert!(t.iter().any(|s| s == "Sign with 3 keys"), "{t:?}");
}

/// A raw transaction of one input paying `amounts`, in hex, as a text
/// file in the Inbox would hold it.
fn raw_paying(amounts: &[u64]) -> Vec<u8> {
    use faraday_bip::bitcoin::consensus::encode::serialize_hex;
    use faraday_bip::bitcoin::transaction::Version;
    use faraday_bip::bitcoin::{Amount, ScriptBuf, Transaction, TxIn, TxOut, absolute::LockTime};
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn::default()],
        output: amounts
            .iter()
            .map(|&a| TxOut {
                value: Amount::from_sat(a),
                script_pubkey: ScriptBuf::new(),
            })
            .collect(),
    };
    serialize_hex(&tx).into_bytes()
}

/// What Decode does with `bytes` in the Inbox: the amounts of the
/// outputs it shows, or the line it refuses the transaction with.
fn decode_file(bytes: Vec<u8>) -> Result<Vec<u64>, String> {
    let mut app = with_inbox(vec![("tx.txt".to_string(), bytes)]);
    assert_eq!(app.inbox()[0].kind, FileKind::Transaction);
    app.press(Action::DecodeInbox(0));
    match app.decode() {
        Some(d) => Ok(d.decoded.outputs.iter().map(|o| o.amount).collect()),
        None => Err(app.toast_text().unwrap_or_default().to_string()),
    }
}

// Bitcoin Core's CheckTransaction refuses an output over MAX_MONEY
// (21,000,000 BTC, "bad-txns-vout-toolarge") and outputs whose total is
// over it ("bad-txns-txouttotal-toolarge"); an output of MAX_MONEY itself
// is valid. Decode refuses what Core refuses, so no total or fee it shows
// wraps.
#[test]
fn decode_refuses_amounts_over_21_million_bitcoin_as_core_does() {
    use faraday_core::strings::EN;
    let max = faraday_bip::bitcoin::Amount::MAX_MONEY.to_sat();
    let refused = Err(EN.decode_outputs_over_max_money.to_string());

    assert_eq!(decode_file(raw_paying(&[max])), Ok(vec![max]));
    assert_eq!(decode_file(raw_paying(&[max + 1])), refused);
    assert_eq!(decode_file(raw_paying(&[max, 1])), refused);
    // Two outputs whose amounts add past u64, as a fuzz target found.
    assert_eq!(decode_file(raw_paying(&[u64::MAX, u64::MAX])), refused);
}

// Bitcoin Core's CheckTxInputs refuses an amount spent, or their total,
// over MAX_MONEY ("bad-txns-inputvalues-outofrange"): a PSBT beside the
// transaction that states one is not taken for the fee.
#[test]
fn decode_refuses_an_amount_going_in_over_21_million_bitcoin() {
    use faraday_core::strings::EN;
    let app = spending_finished();
    let hex = app.spend().unwrap().spend.finished_hex().unwrap();
    let mut psbt = testkit::unsigned(&kit("spending")).unwrap();
    let max = faraday_bip::bitcoin::Amount::MAX_MONEY;
    // The amount stands in a `witness_utxo` alone: beside a previous
    // transaction of the right txid it is not what is read.
    psbt.inner_mut().inputs[0].non_witness_utxo = None;
    let utxo = psbt.inner_mut().inputs[0].witness_utxo.as_mut().unwrap();
    utxo.value = max + faraday_bip::bitcoin::Amount::from_sat(1);
    let mut beside = with_inbox(vec![
        ("broadcast-me.txt".to_string(), hex.as_bytes().to_vec()),
        ("spend.psbt".to_string(), psbt.to_bytes()),
    ]);
    beside.press(Action::DecodeInbox(0));
    assert!(beside.decode().is_none());
    assert_eq!(beside.toast_text(), Some(EN.decode_inputs_over_max_money));
}

// The same amount stated inside a previous transaction of the right txid,
// the output a node reads, is refused as before.
#[test]
fn decode_refuses_an_amount_over_21_million_inside_the_previous_transaction() {
    use faraday_core::strings::EN;
    let mut psbt = testkit::unsigned(&kit("spending")).unwrap();
    let over = faraday_bip::bitcoin::Amount::MAX_MONEY + faraday_bip::bitcoin::Amount::from_sat(1);
    let vout = psbt.inner().unsigned_tx.input[0].previous_output.vout as usize;
    let mut prev = psbt.inner().inputs[0]
        .non_witness_utxo
        .clone()
        .expect("a previous transaction");
    prev.output[vout].value = over;
    psbt.inner_mut().unsigned_tx.input[0].previous_output.txid = prev.compute_txid();
    psbt.inner_mut().inputs[0].non_witness_utxo = Some(prev);
    psbt.inner_mut().inputs[0].witness_utxo = None;
    let hex = faraday_bip::bitcoin::consensus::encode::serialize_hex(&psbt.inner().unsigned_tx);
    let mut beside = with_inbox(vec![
        ("broadcast-me.txt".to_string(), hex.as_bytes().to_vec()),
        ("spend.psbt".to_string(), psbt.to_bytes()),
    ]);
    beside.press(Action::DecodeInbox(0));
    assert!(beside.decode().is_none());
    assert_eq!(beside.toast_text(), Some(EN.decode_inputs_over_max_money));
}

/// The savings wallet's spend signed by two of its three keys on two
/// devices, and the first device's partly signed file.
fn two_devices_signed() -> (Vec<u8>, Vec<u8>, Vec<u8>, Faraday) {
    let k = kit("savings");
    let (unsigned, _) = testkit::spend_of(&k).unwrap();
    let files = |extra: Vec<(String, Vec<u8>)>| {
        let mut v = vec![
            ("spend.psbt".to_string(), unsigned.to_bytes()),
            (
                "savings-wallet.txt".to_string(),
                k.descriptor.clone().into_bytes(),
            ),
        ];
        v.extend(extra);
        v
    };
    let mut a = with_inbox(files(Vec::new()));
    let wallet_at = at(&a, "savings-wallet.txt");
    testkit::load_wallet(&mut a, wallet_at);
    add_key(&mut a, "zebra");
    a.press(Action::StartSpend(at(&a, "spend.psbt")));
    testkit::steps_before_sign_done(&mut a);
    a.press(Action::SignHere);
    let from_a = a.spend().unwrap().spend.psbt.to_bytes();
    let mut b = with_inbox(files(vec![("from-a.psbt".to_string(), from_a.clone())]));
    let wallet_at = at(&b, "savings-wallet.txt");
    testkit::load_wallet(&mut b, wallet_at);
    add_key(&mut b, "bacon");
    b.press(Action::StartSpend(at(&b, "spend.psbt")));
    testkit::steps_before_sign_done(&mut b);
    b.press(Action::SignHere);
    b.press(Action::Collect(at(&b, "from-a.psbt")));
    let full = b.spend().unwrap().spend.psbt.to_bytes();
    (unsigned.to_bytes(), from_a, full, b)
}

fn results_waiting(app: &Faraday) -> Vec<String> {
    app.outbox
        .iter()
        .filter(|i| i.name.ends_with("-signed.psbt") || i.name.ends_with("-final.txn"))
        .map(|i| String::from(&*i.name))
        .collect()
}

/// A PSBT that arrives signed by enough keys elsewhere puts nothing in the
/// Outbox until this device's Review (and Check) is gone through; then
/// both results wait there.
#[test]
fn a_psbt_signed_elsewhere_waits_for_review_before_the_outbox() {
    let k = kit("savings");
    let (_, _, full, _) = two_devices_signed();
    let mut app = with_inbox(vec![
        ("full.psbt".to_string(), full),
        ("savings-wallet.txt".to_string(), k.descriptor.into_bytes()),
    ]);
    let wallet_at = at(&app, "savings-wallet.txt");
    testkit::load_wallet(&mut app, wallet_at);
    app.press(Action::StartSpend(at(&app, "full.psbt")));
    assert!(app.spend().unwrap().spend.finished.is_some());
    assert!(results_waiting(&app).is_empty());
    // The main button does not skip Review either.
    app.press(Action::Primary);
    assert!(results_waiting(&app).is_empty());
    assert_eq!(app.spend().unwrap().open, Some(step::TRANSACTION));
    app.press(Action::StepNext(step::TRANSACTION));
    assert!(
        results_waiting(&app).is_empty(),
        "Check is still to be gone through"
    );
    app.press(Action::SignedToOutbox);
    app.press(Action::TxToOutbox);
    assert!(results_waiting(&app).is_empty());
    app.press(Action::StepNext(step::CHECK));
    assert_eq!(
        results_waiting(&app),
        ["full-signed.psbt", "full-final.txn"]
    );
}

/// A spend made complete by Collect waits for Review again, and a
/// Collect answer after Review needs Review again.
#[test]
fn a_collect_answer_needs_review_again_before_the_outbox() {
    let k = kit("savings");
    let (unsigned, from_a, _, _) = two_devices_signed();
    let mut b = with_inbox(vec![
        ("spend.psbt".to_string(), unsigned),
        ("savings-wallet.txt".to_string(), k.descriptor.into_bytes()),
        ("from-a.psbt".to_string(), from_a),
    ]);
    let wallet_at = at(&b, "savings-wallet.txt");
    testkit::load_wallet(&mut b, wallet_at);
    add_key(&mut b, "bacon");
    b.press(Action::StartSpend(at(&b, "spend.psbt")));
    testkit::steps_before_sign_done(&mut b);
    b.press(Action::SignHere);
    assert!(results_waiting(&b).is_empty(), "not complete yet");
    b.press(Action::Collect(at(&b, "from-a.psbt")));
    assert!(b.spend().unwrap().spend.finished.is_some());
    assert!(results_waiting(&b).is_empty());
    assert_eq!(b.spend().unwrap().open, Some(step::TRANSACTION));
    b.press(Action::StepNext(step::TRANSACTION));
    assert_eq!(
        results_waiting(&b),
        ["spend-signed.psbt", "spend-final.txn"]
    );
}

fn a_stick(files: Vec<(&str, Vec<u8>)>) -> faraday_core::StickInfo {
    faraday_core::StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("T"),
        label: faraday_text::Text::of("SIGNSTICK"),
        boot: false,
        files: files
            .iter()
            .map(|(n, b)| (faraday_text::Text::of(n), b.len() as u64))
            .collect(),
        vaults: Vec::new(),
    }
}

/// Answers every read the app asks of the stick from `files`.
fn answer_reads(app: &mut Faraday, files: &[(&str, Vec<u8>)]) {
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Read { stick, name } = c {
            let bytes = files.iter().find(|(n, _)| **n == *name).unwrap().1.clone();
            app.storage(StorageEvent::Read {
                stick,
                name,
                bytes: bytes.into(),
            });
        }
    }
}

/// One PSBT on the stick: "Import and sign {file}" brings it in and lands on
/// the review, with no trip through Files.
#[test]
fn one_psbt_on_a_stick_imports_and_signs_on_its_review() {
    let k = kit("spending");
    let files = vec![("spend.psbt", testkit::unsigned(&k).unwrap().to_bytes())];
    let mut app = with_inbox(vec![(
        "spending-wallet.txt".to_string(),
        k.descriptor.clone().into_bytes(),
    )]);
    testkit::load_wallet(&mut app, 0);
    app.storage(StorageEvent::Sticks(vec![a_stick(files.clone())]));
    assert_eq!(app.screen, Screen::Files);
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Import and sign spend.psbt"),
        "{texts:?}"
    );
    assert!(app.offers(Action::VisitCopyAndSign));
    app.press(Action::VisitCopyAndSign);
    answer_reads(&mut app, &files);
    testkit::settle(&mut app);
    assert_eq!(app.screen, Screen::Spend);
}

/// Finish offers Show as QR for the signed PSBT and for the finished
/// transaction, and its filled button says the keys go with the lock.
#[test]
fn finish_offers_a_qr_of_each_result_and_lock_and_write() {
    let mut app = spending_finished();
    let _ = app.frame();
    assert!(
        app.offers(Action::QrSigned),
        "Show as QR for the signed PSBT"
    );
    assert!(app.offers(Action::QrTx), "Show as QR for the transaction");
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Lock and write to the stick"),
        "{texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t == "Open Files"),
        "Finish does not open Files: {texts:?}"
    );
    // The end card.
    for want in ["Sign another", "Lock", "Power off"] {
        assert!(texts.iter().any(|t| t == want), "{want}: {texts:?}");
    }
}

/// The finished spend with an open vault holding a change: Lock asks first.
fn finished_with_vault_change() -> Faraday {
    let k = kit("spending");
    let psbt = testkit::unsigned(&k).unwrap();
    let mut app = faraday_core::testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut app, Some(100));
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            ("spend.psbt".to_string(), psbt.to_bytes()),
            ("spending-wallet.txt".to_string(), k.descriptor.into_bytes()),
            ("vault.kdbx".to_string(), testkit::test_vault().unwrap()),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Vault(faraday_core::vault::VaultAction::Open(0)));
    crate::common::type_text(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Vault(faraday_core::vault::VaultAction::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(faraday_shell_api::Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1, "the vault did not open");
    let wallet_at = at(&app, "spending-wallet.txt");
    testkit::load_wallet(&mut app, wallet_at);
    add_key(&mut app, "bacon");
    app.press(Action::StartSpend(at(&app, "spend.psbt")));
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::SignHere);
    assert!(app.spend().unwrap().spend.finished.is_some());
    let key = app.session.keys.len() - 1;
    app.press(Action::Vault(faraday_core::vault::VaultAction::SaveKey(
        key,
    )));
    assert!(
        app.vaults().open[0].changes > 0,
        "the vault was not changed"
    );
    app
}

/// What the last lock kept: the Outbox and the memory.
fn the_lock_kept(app: &mut Faraday) -> (faraday_core::Kept, faraday_core::Kept) {
    let mut saved = None;
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { outbox, kept, .. } = c {
            saved = Some((outbox, kept));
        }
    }
    saved.expect("the lock kept the boxes")
}

/// Whether the memory a lock kept holds a write waiting for the next stick.
fn keeps_a_write(kept: &faraday_core::Kept) -> bool {
    faraday_core::testkit::plain_kept(kept.clone())
        .iter()
        .any(|(n, _)| n == "write-waiting")
}

/// The next process: restored from what the lock kept.
fn restarted(outbox: faraday_core::Kept, kept: faraday_core::Kept) -> Faraday {
    let mut next = faraday_core::testkit::started();
    next.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox,
        kept,
    });
    while next.poll_storage().is_some() {}
    next
}

/// The names written by the commands the app has queued.
fn writes_queued(app: &mut Faraday) -> Vec<String> {
    let mut wrote = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Write { name, .. } = c {
            wrote.push(name.to_string());
        }
    }
    wrote
}

/// The finished spend with a vault open: the stick `T` is in, and Lock and
/// write is asked, the sheet up.
fn lock_asking() -> Faraday {
    let mut app = finished_with_vault_change();
    app.storage(StorageEvent::Sticks(vec![a_stick(Vec::new())]));
    app.press(Action::NotNow);
    app.press(Action::Primary);
    assert_eq!(app.sheet, Some(faraday_core::Sheet::LockAsk));
    app
}

/// The stick pulled and put back while the lock asks: the Lock sheet takes
/// its place.
fn lock_sheet_up() -> Faraday {
    let mut app = lock_asking();
    app.storage(StorageEvent::Sticks(Vec::new()));
    app.storage(StorageEvent::Sticks(vec![a_stick(Vec::new())]));
    assert_eq!(app.sheet, Some(faraday_core::Sheet::Lock));
    app
}

/// A later lock keeps no write for the next stick.
fn a_later_lock_keeps_no_write(app: &mut Faraday) {
    while app.poll_storage().is_some() {}
    app.press(Action::LockUnsealed);
    let (_, kept) = the_lock_kept(app);
    assert!(!keeps_a_write(&kept));
}

/// A lock that asks, answered with Cancel, writes nothing by itself when a
/// later lock comes.
#[test]
fn a_cancelled_lock_and_write_does_not_write_at_a_later_lock() {
    let mut app = lock_asking();
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    a_later_lock_keeps_no_write(&mut app);
}

/// The lock asks, the stick is put in, and Not now on the Lock sheet gives
/// the write up.
#[test]
fn not_now_on_the_lock_sheet_gives_the_write_up() {
    let mut app = lock_sheet_up();
    app.press(Action::NotNow);
    assert_eq!(app.sheet, None);
    a_later_lock_keeps_no_write(&mut app);
}

/// The Lock sheet's scan as a QR code gives the write up too.
#[test]
fn the_lock_sheets_scan_gives_the_write_up() {
    let mut app = lock_sheet_up();
    app.press(Action::ScanFor(faraday_core::Ask::SpendPsbt));
    a_later_lock_keeps_no_write(&mut app);
}

/// A stick pulled with the Lock sheet up gives the write up.
#[test]
fn a_stick_pulled_with_the_lock_sheet_up_gives_the_write_up() {
    let mut app = lock_sheet_up();
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert_eq!(app.sheet, None);
    a_later_lock_keeps_no_write(&mut app);
}

/// A write waiting in the next process is not carried over a plain lock.
#[test]
fn a_restored_write_waiting_is_not_kept_by_a_plain_lock() {
    let mut app = spending_finished();
    app.storage(StorageEvent::Sticks(vec![a_stick(Vec::new())]));
    app.press(Action::Primary);
    app.press(Action::Lock);
    let (outbox, kept) = the_lock_kept(&mut app);
    assert!(keeps_a_write(&kept));
    let mut next = restarted(outbox, kept);
    add_key(&mut next, "bacon");
    next.press(Action::Lock);
    let (_, kept) = the_lock_kept(&mut next);
    assert!(!keeps_a_write(&kept));
}

/// Import and sign, pull the stick to load the key, sign, Lock and write
/// with no stick in: the stick the PSBT came from is written, and another
/// stick that comes first is not.
#[test]
fn lock_and_write_goes_to_the_stick_the_psbt_came_from() {
    let k = kit("spending");
    let files = vec![("spend.psbt", testkit::unsigned(&k).unwrap().to_bytes())];
    let mut app = with_inbox(vec![(
        "spending-wallet.txt".to_string(),
        k.descriptor.clone().into_bytes(),
    )]);
    testkit::load_wallet(&mut app, 0);
    app.storage(StorageEvent::Sticks(vec![a_stick(files.clone())]));
    app.press(Action::VisitCopyAndSign);
    answer_reads(&mut app, &files);
    testkit::settle(&mut app);
    assert_eq!(app.screen, Screen::Spend);
    app.storage(StorageEvent::Sticks(Vec::new()));
    add_key(&mut app, "bacon");
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::SignHere);
    app.press(Action::Primary);
    app.press(Action::Lock);
    let (outbox, kept) = the_lock_kept(&mut app);
    assert!(keeps_a_write(&kept));

    let mut other = restarted(outbox.clone(), kept.clone());
    let mut elsewhere = a_stick(Vec::new());
    elsewhere.id = faraday_text::Text::of("U");
    other.storage(StorageEvent::Sticks(vec![elsewhere]));
    assert!(writes_queued(&mut other).is_empty());

    let mut next = restarted(outbox, kept);
    next.storage(StorageEvent::Sticks(vec![a_stick(Vec::new())]));
    let mut wrote = writes_queued(&mut next);
    wrote.sort();
    assert_eq!(wrote, ["spend-final.txn", "spend-signed.psbt"]);
}

/// A spend finished with no stick in and no Import and sign keeps no write,
/// and a stick put in next is not written with the spend's files.
#[test]
fn lock_and_write_with_no_stick_and_no_import_keeps_nothing() {
    let mut app = spending_finished();
    app.press(Action::Primary);
    app.press(Action::Lock);
    let (outbox, kept) = the_lock_kept(&mut app);
    assert!(!keeps_a_write(&kept));
    let mut next = restarted(outbox, kept);
    next.storage(StorageEvent::Sticks(vec![a_stick(Vec::new())]));
    assert!(writes_queued(&mut next).is_empty());
}

/// Import and sign from a stick, then a new spend from the Inbox finished
/// with no stick in: that first stick is not written with the new spend's
/// files.
#[test]
fn a_later_spend_is_not_written_to_the_stick_of_an_earlier_import_and_sign() {
    let k = kit("spending");
    let files = vec![("spend.psbt", testkit::unsigned(&k).unwrap().to_bytes())];
    let mut app = with_inbox(vec![(
        "spending-wallet.txt".to_string(),
        k.descriptor.clone().into_bytes(),
    )]);
    testkit::load_wallet(&mut app, 0);
    app.storage(StorageEvent::Sticks(vec![a_stick(files.clone())]));
    app.press(Action::VisitCopyAndSign);
    answer_reads(&mut app, &files);
    testkit::settle(&mut app);
    assert_eq!(app.screen, Screen::Spend);
    app.storage(StorageEvent::Sticks(Vec::new()));
    add_key(&mut app, "bacon");
    app.press(Action::StartSpend(at(&app, "spend.psbt")));
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::SignHere);
    app.press(Action::Primary);
    app.press(Action::Lock);
    let (outbox, kept) = the_lock_kept(&mut app);
    assert!(!keeps_a_write(&kept));
    let mut next = restarted(outbox, kept);
    next.storage(StorageEvent::Sticks(vec![a_stick(Vec::new())]));
    assert!(writes_queued(&mut next).is_empty());
}

/// Import and sign lands on Spend when the Inbox already holds a file of
/// the stick's file name with other bytes.
#[test]
fn import_and_sign_lands_on_spend_beside_an_inbox_file_of_its_name() {
    let k = kit("spending");
    let files = vec![("spend.psbt", testkit::unsigned(&k).unwrap().to_bytes())];
    let mut app = with_inbox(vec![
        (
            "spending-wallet.txt".to_string(),
            k.descriptor.clone().into_bytes(),
        ),
        ("spend.psbt".to_string(), b"another file".to_vec()),
    ]);
    let wallet_at = at(&app, "spending-wallet.txt");
    testkit::load_wallet(&mut app, wallet_at);
    app.storage(StorageEvent::Sticks(vec![a_stick(files.clone())]));
    app.press(Action::VisitCopyAndSign);
    answer_reads(&mut app, &files);
    testkit::settle(&mut app);
    assert_eq!(app.screen, Screen::Spend);
}

/// A stick is told by its FAT serial where it has one: the same serial
/// under another id is the same stick, another serial is not.
#[test]
fn lock_and_write_tells_a_stick_by_its_serial() {
    let mut app = spending_finished();
    let mut here = a_stick(Vec::new());
    here.serial = Some(0xABCD_0123);
    app.storage(StorageEvent::Sticks(vec![here]));
    app.press(Action::Primary);
    app.press(Action::Lock);
    let (outbox, kept) = the_lock_kept(&mut app);
    let mut wrong = restarted(outbox.clone(), kept.clone());
    let mut other = a_stick(Vec::new());
    other.serial = Some(0x1234_5678);
    wrong.storage(StorageEvent::Sticks(vec![other]));
    assert!(writes_queued(&mut wrong).is_empty());
    let mut right = restarted(outbox, kept);
    let mut same = a_stick(Vec::new());
    same.serial = Some(0xABCD_0123);
    same.id = faraday_text::Text::of("moved-to-another-port");
    right.storage(StorageEvent::Sticks(vec![same]));
    assert_eq!(writes_queued(&mut right).len(), 2);
}

/// "Lock and write": only the spend's own two files go, to the stick that
/// was in, and are read back; any other stick has the ordinary visit, and a
/// stick that comes into a process that is not clean gets the Lock sheet.
#[test]
fn lock_and_write_writes_exactly_the_spends_files_to_its_stick() {
    let mut app = spending_finished_with(vec![
        ("other.psbt", b"another transaction".to_vec()),
        ("export.txt", b"an export".to_vec()),
    ]);
    app.storage(StorageEvent::Sticks(vec![a_stick(Vec::new())]));
    app.press(Action::Primary);
    app.press(Action::Lock);
    let (outbox, kept) = the_lock_kept(&mut app);
    assert!(keeps_a_write(&kept));

    // Another stick is not written by itself.
    let mut other = restarted(outbox.clone(), kept.clone());
    let mut elsewhere = a_stick(Vec::new());
    elsewhere.id = faraday_text::Text::of("U");
    other.storage(StorageEvent::Sticks(vec![elsewhere]));
    assert!(writes_queued(&mut other).is_empty());

    // The stick that was in is, and only with the spend's files.
    let mut next = restarted(outbox.clone(), kept.clone());
    next.storage(StorageEvent::Sticks(vec![a_stick(Vec::new())]));
    let mut wrote = Vec::new();
    while let Some(c) = next.poll_storage() {
        if let StorageCommand::Write { stick, name, .. } = c {
            wrote.push(name.to_string());
            next.storage(StorageEvent::Written {
                stick,
                wrote_as: name.clone(),
                name,
            });
        }
    }
    wrote.sort();
    assert_eq!(wrote, ["spend-final.txn", "spend-signed.psbt"]);
    let _ = next.frame();
    let texts = next.drawn_texts();
    for want in ["Sign another", "Power off"] {
        assert!(texts.iter().any(|t| t == want), "{want}: {texts:?}");
    }
    // It was acted on: a second stick is not written.
    let mut second = a_stick(Vec::new());
    second.id = faraday_text::Text::of("V");
    next.storage(StorageEvent::Sticks(vec![a_stick(Vec::new()), second]));
    assert!(writes_queued(&mut next).is_empty());

    // A process that is not clean asks to lock, and writes nothing.
    let mut busy = restarted(outbox, kept);
    add_key(&mut busy, "bacon");
    busy.storage(StorageEvent::Sticks(vec![a_stick(Vec::new())]));
    assert_eq!(busy.sheet, Some(faraday_core::Sheet::Lock));
    assert!(writes_queued(&mut busy).is_empty());
}

/// After Remove of a result, the end card no longer says it waits.
#[test]
fn the_end_card_counts_only_what_is_still_waiting() {
    let mut app = spending_finished();
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "2 files wait for the stick"),
        "{texts:?}"
    );
    let first = app
        .outbox
        .iter()
        .position(|i| i.name == "spend-signed.psbt")
        .unwrap();
    app.press(Action::OutboxRemove(first));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "1 file waits for the stick"),
        "{texts:?}"
    );
    let last = app
        .outbox
        .iter()
        .position(|i| i.name == "spend-final.txn")
        .unwrap();
    app.press(Action::OutboxRemove(last));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        !texts.iter().any(|t| t == "Lock and write to the stick"),
        "{texts:?}"
    );
}
