//! The end of signing: the nonce check recomputes every signature a key
//! held here made and says which came from elsewhere, and calls none
//! valid that it could not check; the finished
//! transaction decodes to what it spends and pays, with its change marked
//! as the wallet's own; and a raw transaction in a text file decodes too.
//! Before signing, the card after Wallet is Transaction, with the txid
//! at its foot, and a wallet checked once this power-on has Check closed
//! on its next spend, across a lock, until the machine starts afresh.

use crate::common::add_key;
use faraday_core::testkit::{self, Kit};
use faraday_core::wallet::{FileKind, step};
use faraday_core::{Action, Faraday, Screen, StorageCommand, StorageEvent};
use faraday_shell_api::App;

fn kit(id: &str) -> Kit {
    testkit::kits().into_iter().find(|k| k.id == id).unwrap()
}

fn with_inbox(files: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = faraday_core::testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: files,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

fn at(app: &Faraday, name: &str) -> usize {
    app.inbox().iter().position(|i| i.name == name).unwrap()
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
    a.press(Action::LoadWallet(at(&a, "savings-wallet.txt")));
    add_key(&mut a, "zebra");
    a.press(Action::StartSpend(at(&a, "spend.psbt")));
    a.press(Action::SignHere);
    let from_a = a.spend().unwrap().spend.psbt.to_bytes();

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

#[test]
fn the_finished_transaction_decodes_to_what_it_pays() {
    let mut app = spending_finished();
    let tx = app.spend().unwrap().spend.finished.clone().unwrap();
    app.press(Action::DecodeFinished);
    assert_eq!(app.screen, Screen::Decode);
    let d = &app.decode().unwrap().decoded;
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
        hex.clone().into_bytes(),
    )]);
    assert_eq!(alone.inbox()[0].kind, FileKind::Transaction);
    alone.press(Action::DecodeInbox(0));
    let d = &alone.decode().unwrap().decoded;
    assert_eq!(d.txid, txid);
    assert_eq!(d.fee, None);

    // Beside its PSBT: the fee too.
    let psbt = testkit::unsigned(&kit("spending")).unwrap();
    let mut beside = with_inbox(vec![
        ("broadcast-me.txt".to_string(), hex.into_bytes()),
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
        inbox: vec![
            ("spend.psbt".to_string(), psbt.to_bytes()),
            ("spending-wallet.txt".to_string(), k.descriptor.into_bytes()),
        ],
        outbox: Vec::new(),
        kept,
    });
    app.press(Action::LoadWallet(at(&app, "spending-wallet.txt")));
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
            last = kept;
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

/// Finishing a spend puts both results For the stick with no further
/// press (`docs/history/SIMPLIFY.md` §4.2): the signed PSBT and the finished
/// transaction. Each card offers Remove in their place, and the primary
/// opens Files, where the visit writes them.
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
        texts.iter().any(|t| t == "Insert a stick to write it"),
        "with no stick the primary asks for one: {texts:?}"
    );
    app.press(Action::Primary);
    assert_eq!(app.screen, Screen::Files);
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
        ("broadcast-me.txt".to_string(), hex.clone().into_bytes()),
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
    let utxo = other.inner_mut().inputs[0].witness_utxo.as_mut().unwrap();
    utxo.value += faraday_bip::bitcoin::Amount::from_sat(1000);
    let mut both = with_inbox(vec![
        ("broadcast-me.txt".to_string(), hex.into_bytes()),
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
fn signed_by_zebra_under(ty: faraday_bip::bitcoin::sighash::EcdsaSighashType) -> Vec<u8> {
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
    b.press(Action::LoadWallet(at(&b, "savings-wallet.txt")));
    add_key(&mut b, "bacon");
    b.press(Action::StartSpend(at(&b, "spend.psbt")));
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
    b.press(Action::LoadWallet(at(&b, "savings-wallet.txt")));
    add_key(&mut b, "bacon");
    b.press(Action::StartSpend(at(&b, "spend.psbt")));
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
        ("broadcast-me.txt".to_string(), hex.clone().into_bytes()),
        ("bare.psbt".to_string(), bare.to_bytes()),
        ("spend.psbt".to_string(), psbt.to_bytes()),
    ]);
    both.press(Action::DecodeInbox(0));
    assert!(both.decode().unwrap().decoded.fee.is_some());
    let _ = both.frame();
    let text = both.drawn_texts().join("\n");
    assert!(text.contains("From spend.psbt"), "{text}");

    let mut other = psbt.clone();
    let utxo = other.inner_mut().inputs[0].witness_utxo.as_mut().unwrap();
    utxo.value += faraday_bip::bitcoin::Amount::from_sat(1000);
    let mut differ = with_inbox(vec![
        ("broadcast-me.txt".to_string(), hex.into_bytes()),
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
