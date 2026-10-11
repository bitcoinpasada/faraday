//! Silent payments from a loaded key: the address is BIP-352's for the
//! key, a label makes another, the record in the Outbox reads back as
//! the same wallet, and the scan key leaves only through the secret sheet.
//! The key's silent payments wallet is among the session's wallets, a
//! record from Files is one too, and Check a payment finds a payment
//! BIP-352's sending side made to it, to the address or to a label, and
//! not one made to someone else.

use faraday_core::wallet::Kind;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent};
use opensigner_core::silent::Checked;
use osk_bip::bitcoin::key::TapTweak;
use osk_bip::bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
use osk_bip::bitcoin::{
    Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness, absolute, consensus,
    transaction,
};
use osk_bip::silent::{self, Receiver, SendKey};
use osk_bip::silent_wallet::SilentWallet;
use osk_shell_api::{App, Event, Key};

fn with_key() -> Faraday {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Entry(None));
    for c in faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    app
}

#[test]
fn the_address_is_bip352s_for_the_key_and_a_label_makes_another() {
    let mut app = with_key();
    app.press(Action::Silent);
    assert_eq!(app.screen, Screen::Silent);
    let secp = osk_bip::bitcoin::secp256k1::Secp256k1::new();
    let r = Receiver::derive(&app.session.keys[0].master, 0);
    let plain = r.address(&secp).unwrap().as_str().to_string();
    assert!(plain.starts_with("sp1"), "{plain}");
    assert_eq!(app.silent_address().as_deref(), Some(plain.as_str()));
    app.press(Action::SLabel(1));
    let one = r.labelled_address(&secp, 1).unwrap().as_str().to_string();
    assert_eq!(app.silent_address().as_deref(), Some(one.as_str()));
    assert_ne!(one, plain);
    // The record lists the label handed out and reads back.
    app.press(Action::SRecord);
    let text = String::from_utf8(app.outbox[0].bytes.clone()).unwrap();
    let w = SilentWallet::parse(&text).expect("the record does not read back");
    assert_eq!(w.labels, 1);
    assert_eq!(w.address(), plain);
}

#[test]
fn the_scan_key_leaves_only_through_the_secret_sheet() {
    let mut app = with_key();
    app.press(Action::Silent);
    app.press(Action::SNext);
    app.press(Action::SScanVault);
    assert!(app.outbox.is_empty(), "no vault is open");
    app.press(Action::SScanOut);
    assert_eq!(app.sheet, Some(Sheet::SecretOut));
    assert!(app.outbox.is_empty());
}

/// The key a payer spends with, and the P2WPKH output it pays from.
fn payer() -> (SecretKey, ScriptBuf) {
    let secp = Secp256k1::new();
    let key = SecretKey::from_slice(&[0x11u8; 32]).unwrap();
    let public = osk_bip::bitcoin::PublicKey::new(key.public_key(&secp));
    let script = ScriptBuf::new_p2wpkh(&public.wpubkey_hash().unwrap());
    (key, script)
}

/// The transaction that funded the payer: Check a payment needs it for
/// the output its input spends.
fn funding(script: &ScriptBuf) -> Transaction {
    Transaction {
        version: transaction::Version::TWO,
        lock_time: absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(200_000),
            script_pubkey: script.clone(),
        }],
    }
}

/// A transaction paying `spend` (an address's spend key, or a label's)
/// beside `scan`, made by BIP-352's sending side from the funding output,
/// with someone else's output after it; and the funding transaction.
fn payment(scan: &PublicKey, spend: &PublicKey) -> (Transaction, Transaction) {
    let secp = Secp256k1::new();
    let (key, script) = payer();
    let previous = funding(&script);
    let outpoint = OutPoint {
        txid: previous.compute_txid(),
        vout: 0,
    };
    let mut serialized = [0u8; 36];
    serialized[..32].copy_from_slice(&consensus::serialize(&outpoint.txid));
    serialized[32..].copy_from_slice(&0u32.to_le_bytes());
    let outputs = silent::sender_outputs(
        &secp,
        &[SendKey {
            key,
            taproot: false,
        }],
        &[serialized],
        scan,
        &[*spend],
    )
    .unwrap();
    // BIP-352 reads the public key from the witness's last item.
    let mut witness = Witness::new();
    witness.push([0x30u8; 71]);
    witness.push(key.public_key(&secp).serialize());
    let mut out: Vec<TxOut> = outputs
        .iter()
        .map(|k| TxOut {
            value: Amount::from_sat(150_000),
            script_pubkey: ScriptBuf::new_p2tr_tweaked(
                osk_bip::bitcoin::XOnlyPublicKey::from_slice(k)
                    .unwrap()
                    .dangerous_assume_tweaked(),
            ),
        })
        .collect();
    out.push(TxOut {
        value: Amount::from_sat(40_000),
        script_pubkey: script,
    });
    let tx = Transaction {
        version: transaction::Version::TWO,
        lock_time: absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: outpoint,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness,
        }],
        output: out,
    };
    (tx, previous)
}

/// Files holding these, as a stick visit leaves them.
fn files(app: &mut Faraday, files: &[(&str, Vec<u8>)]) {
    app.storage(StorageEvent::Restored {
        inbox: files
            .iter()
            .map(|(n, b)| (n.to_string(), b.clone()))
            .collect(),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
}

/// What the check of Files' first file came to.
fn checked(app: &Faraday) -> Option<Checked> {
    let (_, check) = app.silent.as_ref()?.check.as_ref()?;
    check.result().cloned()
}

#[test]
fn the_key_s_silent_payments_wallet_is_among_the_wallets_and_keeps_its_labels() {
    let mut app = with_key();
    app.press(Action::Silent);
    app.press(Action::SAddWallet);
    assert_eq!(app.session.wallets.len(), 1);
    let w = &app.session.wallets[0];
    assert_eq!(Kind::of(&w.policy), Kind::Silent);
    let secp = Secp256k1::new();
    let r = Receiver::derive(&app.session.keys[0].master, 0);
    let plain = r.address(&secp).unwrap().as_str().to_string();
    assert_eq!(w.policy.silent().unwrap().address(), plain);
    // Its one key is here.
    let slots = app.session.slots(w);
    assert_eq!(slots.len(), 1);
    assert!(slots[0].held_by.is_some());
    // A label shown is a label handed out, on the wallet's record.
    app.press(Action::SLabel(1));
    app.press(Action::SLabel(1));
    assert_eq!(app.session.wallets[0].policy.silent().unwrap().labels, 2);
    // The wallet card opens its page.
    app.press(Action::Nav(Screen::Wallets));
    let _ = app.frame();
    assert!(app.offers(Action::SWallet(0)));
    app.press(Action::SWallet(0));
    assert_eq!(app.screen, Screen::Silent);
    assert_eq!(app.silent_address().as_deref(), Some(plain.as_str()));
}

#[test]
fn a_record_read_from_files_is_a_wallet_whose_address_needs_no_key() {
    let mut keyed = with_key();
    keyed.press(Action::Silent);
    let record = keyed.silent_record().unwrap();
    let address = keyed.silent_address().unwrap();
    let mut app = faraday_core::testkit::started();
    files(&mut app, &[("silent-1.txt", record.into_bytes())]);
    app.press(Action::LoadWallet(0));
    assert_eq!(app.session.wallets.len(), 1, "{:?}", app.toast_text());
    assert_eq!(Kind::of(&app.session.wallets[0].policy), Kind::Silent);
    app.press(Action::SWallet(0));
    assert_eq!(app.silent_address().as_deref(), Some(address.as_str()));
    assert!(!app.silent_key_here());
    // A label needs the scan private key, which is the key's.
    app.press(Action::SLabel(1));
    assert_eq!(app.silent_address(), None);
}

#[test]
fn a_payment_to_this_wallet_is_found_and_one_to_another_is_not() {
    let mut app = with_key();
    let secp = Secp256k1::new();
    let r = Receiver::derive(&app.session.keys[0].master, 0);
    let scan = r.scan_public_key(&secp);
    let (paid, funding_tx) = payment(&scan, &r.spend_public_key());
    // Someone else's address: the same scan key, another spend key.
    let other = SecretKey::from_slice(&[0x22u8; 32])
        .unwrap()
        .public_key(&secp);
    let (unpaid, _) = payment(&scan, &other);
    files(
        &mut app,
        &[
            ("payment.txn", consensus::serialize(&paid)),
            ("other.txn", consensus::serialize(&unpaid)),
            ("funding.txn", consensus::serialize(&funding_tx)),
        ],
    );
    app.press(Action::Silent);
    app.press(Action::SAddWallet);
    app.press(Action::SCheck(0));
    match checked(&app) {
        Some(Checked::Paid(p)) => {
            assert_eq!(p.len(), 1);
            assert_eq!(p[0].vout, 0);
            assert_eq!(p[0].amount, Amount::from_sat(150_000));
            assert_eq!(p[0].label, None);
        }
        other => panic!("{other:?}"),
    }
    app.press(Action::SCheck(1));
    assert_eq!(checked(&app), Some(Checked::NotPaid));
}

#[test]
fn a_payment_to_a_label_names_the_label_and_one_missing_its_funding_waits() {
    let mut app = with_key();
    let secp = Secp256k1::new();
    let r = Receiver::derive(&app.session.keys[0].master, 0);
    let scan = r.scan_public_key(&secp);
    let (paid, funding_tx) = payment(&scan, &r.labelled_spend_key(&secp, 2).unwrap());
    files(&mut app, &[("payment.txn", consensus::serialize(&paid))]);
    app.press(Action::Silent);
    app.press(Action::SAddWallet);
    app.press(Action::SLabel(1));
    app.press(Action::SLabel(1));
    // Without the transaction it spends from there is no answer yet.
    app.press(Action::SCheck(0));
    let (_, check) = app.silent.as_ref().unwrap().check.as_ref().unwrap();
    assert_eq!(check.waiting_for(), Some(funding_tx.compute_txid()));
    assert_eq!(checked(&app), None);
    files(
        &mut app,
        &[
            ("payment.txn", consensus::serialize(&paid)),
            ("funding.txn", consensus::serialize(&funding_tx)),
        ],
    );
    app.press(Action::SCheck(0));
    match checked(&app) {
        Some(Checked::Paid(p)) => assert_eq!(p[0].label, Some(2)),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_silent_payments_wallet_saved_into_a_vault_reads_back_as_itself() {
    use faraday_core::testkit;
    use faraday_core::vaults::VaultAction as V;
    use faraday_vault::records::{field, kind};
    let mut keyed = with_key();
    keyed.press(Action::Silent);
    keyed.press(Action::SLabel(1));
    let record = keyed.silent_record().unwrap();
    let mut app = testkit::started();
    files(
        &mut app,
        &[
            ("vault.ofv", testkit::test_vault().unwrap()),
            ("silent-1.txt", record.clone().into_bytes()),
        ],
    );
    // Opened from a flow (the vault way), not the Vaults list: it shows
    // what it holds without loading it, as today (`docs/NEW-WALLET.md`
    // §11.1a), so the wallet picked below is the silent one just loaded.
    app.press(Action::Vault(V::OpenFrom(0, Screen::Files)));
    for c in testkit::VAULT_PASSPHRASES[0].chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    for t in 1..10u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults.open.len(), 1);
    let file = app
        .inbox
        .iter()
        .position(|i| i.name == "silent-1.txt")
        .unwrap();
    app.press(Action::LoadWallet(file));
    let w = app.wallet;
    assert_eq!(Kind::of(&app.session.wallets[w].policy), Kind::Silent);
    app.press(Action::Vault(V::SaveWallet(w)));
    let open = &app.vaults.open[0];
    let (_, saved) = open.contents.of(kind::WALLET).last().unwrap();
    let text = saved.text(field::WALLET).unwrap();
    let back = faraday_core::wallet::read_wallet(text).expect("the saved wallet reads back");
    assert_eq!(back.silent(), app.session.wallets[w].policy.silent());
    assert_eq!(back.silent().unwrap().labels, 1);
}
