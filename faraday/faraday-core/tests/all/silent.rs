//! Silent payments from a loaded key: the address is BIP-352's for the
//! key, a label makes another, the record in the Outbox reads back as
//! the same wallet, and the scan key leaves only through the secret sheet.
//! The key's silent payments wallet is among the session's wallets, a
//! record from Files is one too, and Check a payment finds a payment
//! BIP-352's sending side made to it, to the address or to a label, and
//! not one made to someone else.

#![expect(
    clippy::disallowed_types,
    reason = "a test or example builds keys and seeds directly and is not shipped (docs/KEYS.md)"
)]

use crate::common::RecordsOf as _;
use faraday_bip::bitcoin::key::TapTweak;
use faraday_bip::bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
use faraday_bip::bitcoin::{
    Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness, absolute, consensus,
    transaction,
};
use faraday_bip::silent::{self, Receiver, SendKey};
use faraday_bip::silent_wallet::SilentWallet;
use faraday_core::silent::check::Checked;
use faraday_core::wallet::Kind;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent};
use faraday_shell_api::{App, Event, Key};

fn with_key() -> Faraday {
    let mut app = faraday_core::testkit::started();
    add_key(&mut app);
    app
}

/// The first test key typed into Add a key.
fn add_key(app: &mut Faraday) {
    app.press(Action::Entry(None));
    for c in faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    faraday_core::testkit::keep_published(app);
}

/// Everything the current frame draws, one line after another.
fn drawn(app: &mut Faraday) -> String {
    app.drawn_texts().join(" ")
}

/// The first test key's silent payments record at `account`, its points
/// derived at `at`, with the scan key replaced by `scan` when given.
fn record_of(account: u32, at: u32, scan: Option<PublicKey>) -> SilentWallet {
    let c = faraday_core::testkit::compute();
    let app = with_key();
    let secp = Secp256k1::new();
    let r = Receiver::derive(&c, app.session.keys.master_for_tests(0), at);
    SilentWallet {
        network: app.session.network(),
        fingerprint: app.session.keys[0].fingerprint,
        account,
        scan: scan.unwrap_or_else(|| r.scan_public_key(&secp)),
        spend: r.spend_public_key(),
        labels: 0,
    }
}

/// A scan key that is not the test key's.
fn another_scan_key() -> PublicKey {
    SecretKey::from_slice(&[0x33u8; 32])
        .unwrap()
        .public_key(&Secp256k1::new())
}

#[test]
fn the_address_is_bip352s_for_the_key_and_a_label_makes_another() {
    let c = faraday_core::testkit::compute();
    // Mainnet's `sp1` address, on Trezor's published "all all" firmware test seed, which is not a public sample.
    let mut app = faraday_core::testkit::started();
    faraday_core::testkit::move_to(&mut app.session, faraday_bip::keys::Network::Mainnet).unwrap();
    app.press(Action::Entry(None));
    for c in "all all all all all all all all all all all all".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    faraday_core::testkit::keep_published(&mut app);
    app.press(Action::Silent);
    assert_eq!(app.screen, Screen::Silent);
    let secp = faraday_bip::bitcoin::secp256k1::Secp256k1::new();
    let r = Receiver::derive(&c, app.session.keys.master_for_tests(0), 0);
    let plain = r.address(&secp).unwrap().as_str().to_string();
    assert!(plain.starts_with("sp1"), "{plain}");
    assert_eq!(app.silent_address(&c).as_deref(), Some(plain.as_str()));
    app.press(Action::SLabel(1));
    let one = r.labelled_address(&secp, 1).unwrap().as_str().to_string();
    assert_eq!(app.silent_address(&c).as_deref(), Some(one.as_str()));
    assert_ne!(one, plain);
    // The record lists the label handed out and reads back.
    app.press(Action::SRecord);
    let text = String::from_utf8(app.outbox[0].bytes.to_vec()).unwrap();
    let w = SilentWallet::parse(&text).expect("the record does not read back");
    assert_eq!(w.labels, 1);
    assert_eq!(w.address(), plain);
}

/// Types `text` into the DNS record field `here`.
fn type_into(app: &mut Faraday, here: faraday_core::edit::Here, text: &str) {
    app.press(Action::TypeHere(here));
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

#[test]
fn a_name_and_a_domain_show_the_addresss_dns_record_and_a_bad_domain_shows_none() {
    use faraday_core::edit::Here;
    let mut app = with_key();
    app.press(Action::Silent);
    let text = drawn(&mut app);
    assert!(text.contains("DNS record"), "{text}");
    assert!(!text.contains("@"), "{text}");
    type_into(&mut app, Here::SilentName, "matt");
    assert!(!drawn(&mut app).contains("DNS record as QR"));
    type_into(&mut app, Here::SilentDomain, "example.com");
    let text = drawn(&mut app);
    assert!(text.contains("\u{20bf}matt@example.com"), "{text}");
    assert!(text.contains("DNS record as QR"), "{text}");
    let c = faraday_core::testkit::compute();
    let address = app.silent_address(&c).unwrap();
    let view = app
        .code_view(&c, faraday_core::Code::SilentDns)
        .expect("the record's code");
    let _ = view;
    let want = silent::dns_record("matt", "example.com", &address).unwrap();
    assert!(
        want.as_str()
            .starts_with("matt.user._bitcoin-payment.example.com. IN TXT")
    );

    // A domain that is not DNS's: a line, no record.
    app.event(Event::Key(Key::Char(' ')));
    let text = drawn(&mut app);
    assert!(text.contains("Not a valid DNS name"), "{text}");
    assert!(!text.contains("DNS record as QR"), "{text}");
    assert!(!text.contains("_bitcoin-payment"), "{text}");
    assert!(app.code_view(&c, faraday_core::Code::SilentDns).is_err());
}

#[test]
fn the_dns_record_of_a_labelled_address_names_its_label() {
    use faraday_core::edit::Here;
    let mut app = with_key();
    app.press(Action::Silent);
    type_into(&mut app, Here::SilentName, "matt");
    type_into(&mut app, Here::SilentDomain, "example.com");
    let c = faraday_core::testkit::compute();
    let plain = app.code_view(&c, faraday_core::Code::SilentDns).unwrap();
    assert!(
        !plain.png_name.as_str().contains("label"),
        "{}",
        plain.png_name.as_str()
    );
    app.press(Action::SLabel(1));
    let view = app.code_view(&c, faraday_core::Code::SilentDns).unwrap();
    assert!(
        view.png_name.as_str().contains("label-1"),
        "{}",
        view.png_name.as_str()
    );
    assert!(
        view.label.iter().any(|l| l == "Label 1"),
        "{:?}",
        view.label
    );
}

#[test]
fn the_scan_key_leaves_only_through_the_secret_sheet() {
    let mut app = with_key();
    app.press(Action::Silent);
    app.press(Action::SStep(faraday_core::silent::sstep::SCAN));
    app.press(Action::SScanVault);
    assert!(app.outbox.is_empty(), "no vault is open");
    app.press(Action::SScanOut);
    assert_eq!(app.sheet, Some(Sheet::SecretOut));
    assert!(app.outbox.is_empty());
}

#[test]
fn another_starts_over_and_done_goes_back_to_where_it_was_opened() {
    let mut app = with_key();
    let from = app.screen;
    app.press(Action::Silent);
    app.press(Action::SLabel(1));
    assert_eq!(app.silent().unwrap().label, 1);
    app.press(Action::SAnother);
    assert_eq!(app.screen, Screen::Silent);
    assert_eq!(app.silent().unwrap().label, 0);
    app.press(Action::Nav(app.silent().unwrap().back));
    assert_eq!(app.screen, from);
}

/// The key a payer spends with, and the P2WPKH output it pays from.
fn payer() -> (SecretKey, ScriptBuf) {
    let secp = Secp256k1::new();
    let key = SecretKey::from_slice(&[0x11u8; 32]).unwrap();
    let public = faraday_bip::bitcoin::PublicKey::new(key.public_key(&secp));
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
                faraday_bip::bitcoin::XOnlyPublicKey::from_slice(k)
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
        inbox: faraday_core::files_from(
            files
                .iter()
                .map(|(n, b)| (n.to_string(), b.clone()))
                .collect(),
        ),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
}

/// What the check of Files' first file came to.
fn checked(app: &Faraday) -> Option<Checked> {
    let (_, check) = app.silent()?.check.as_ref()?;
    check.result().cloned()
}

#[test]
fn the_key_s_silent_payments_wallet_is_among_the_wallets_and_keeps_its_labels() {
    let c = faraday_core::testkit::compute();
    let mut app = with_key();
    app.press(Action::Silent);
    app.press(Action::SAddWallet);
    assert_eq!(app.session.wallets.len(), 1);
    let w = &app.session.wallets[0];
    assert_eq!(Kind::of(w.policy()), Kind::Silent);
    let secp = Secp256k1::new();
    let r = Receiver::derive(&c, app.session.keys.master_for_tests(0), 0);
    let plain = r.address(&secp).unwrap().as_str().to_string();
    assert_eq!(w.policy().silent().unwrap().address(), plain);
    // Its one key is here.
    let slots = app.session.slots(w);
    assert_eq!(slots.len(), 1);
    assert!(slots[0].held_by.is_some());
    // A label shown is a label handed out, on the wallet's record.
    app.press(Action::SLabel(1));
    app.press(Action::SLabel(1));
    assert_eq!(app.session.wallets[0].policy().silent().unwrap().labels, 2);
    // The wallet card opens its page.
    app.press(Action::Nav(Screen::Wallets));
    let _ = app.frame();
    assert!(app.offers(Action::SWallet(0)));
    app.press(Action::SWallet(0));
    assert_eq!(app.screen, Screen::Silent);
    assert_eq!(app.silent_address(&c).as_deref(), Some(plain.as_str()));
}

#[test]
fn a_record_read_from_files_is_a_wallet_whose_address_needs_no_key() {
    let c = faraday_core::testkit::compute();
    let mut keyed = with_key();
    keyed.press(Action::Silent);
    let record = keyed.silent_record(&c).unwrap();
    let address = keyed.silent_address(&c).unwrap();
    let mut app = faraday_core::testkit::started();
    files(&mut app, &[("silent-1.txt", record.as_bytes().to_vec())]);
    faraday_core::testkit::load_wallet(&mut app, 0);
    assert_eq!(app.session.wallets.len(), 1, "{:?}", app.toast_text());
    assert_eq!(Kind::of(app.session.wallets[0].policy()), Kind::Silent);
    app.press(Action::SWallet(0));
    assert_eq!(app.silent_address(&c).as_deref(), Some(address.as_str()));
    assert!(!app.silent_key_here(&c));
    // A label needs the scan private key, which is the key's.
    app.press(Action::SLabel(1));
    assert_eq!(app.silent_address(&c), None);
}

#[test]
fn a_payment_to_this_wallet_is_found_and_one_to_another_is_not() {
    let c = faraday_core::testkit::compute();
    let mut app = with_key();
    let secp = Secp256k1::new();
    let r = Receiver::derive(&c, app.session.keys.master_for_tests(0), 0);
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
    let c = faraday_core::testkit::compute();
    let mut app = with_key();
    let secp = Secp256k1::new();
    let r = Receiver::derive(&c, app.session.keys.master_for_tests(0), 0);
    let scan = r.scan_public_key(&secp);
    let (paid, funding_tx) = payment(&scan, &r.labelled_spend_key(&secp, 2).unwrap());
    files(&mut app, &[("payment.txn", consensus::serialize(&paid))]);
    app.press(Action::Silent);
    app.press(Action::SAddWallet);
    app.press(Action::SLabel(1));
    app.press(Action::SLabel(1));
    // Without the transaction it spends from there is no answer yet.
    app.press(Action::SCheck(0));
    let (_, check) = app.silent().unwrap().check.as_ref().unwrap();
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
    let c = faraday_core::testkit::compute();
    use faraday_core::testkit;
    use faraday_core::vault::VaultAction as V;
    use faraday_vault::records::{field, kind};
    let mut keyed = with_key();
    keyed.press(Action::Silent);
    keyed.press(Action::SLabel(1));
    let record = keyed.silent_record(&c).unwrap();
    let mut app = testkit::started();
    files(
        &mut app,
        &[
            ("vault.kdbx", testkit::test_vault().unwrap()),
            ("silent-1.txt", record.as_bytes().to_vec()),
        ],
    );
    // Opened from a flow (the vault way), not the Vaults list: it shows
    // what it holds without loading it, as today,
    // so the wallet picked below is the silent one just loaded.
    app.press(Action::Vault(V::OpenFrom(0, Screen::Files)));
    for c in testkit::VAULT_PASSPHRASE.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    for t in 1..10u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1);
    let file = app
        .inbox()
        .iter()
        .position(|i| i.name == "silent-1.txt")
        .unwrap();
    faraday_core::testkit::load_wallet(&mut app, file);
    let w = app.wallet_index();
    assert_eq!(Kind::of(app.session.wallets[w].policy()), Kind::Silent);
    app.press(Action::Vault(V::SaveWallet(w)));
    let open = &app.vaults().open[0];
    let (_, saved) = app
        .vaults()
        .store()
        .contents_for_tests(open.id)
        .of(kind::WALLET)
        .last()
        .unwrap();
    let text = saved.text(field::WALLET).unwrap();
    let back = faraday_core::wallet::read_wallet(&c, text).expect("the saved wallet reads back");
    assert_eq!(back.silent(), app.session.wallets[w].policy().silent());
    assert_eq!(back.silent().unwrap().labels, 1);
}

#[test]
fn a_silent_payments_wallet_saved_into_a_vault_is_there_after_the_file_is_written_and_opened() {
    use faraday_core::StorageCommand;
    use faraday_core::testkit;
    use faraday_core::vault::VaultAction as V;
    use faraday_vault::records::{field, kind};
    let c = testkit::compute();
    let mut keyed = with_key();
    keyed.press(Action::Silent);
    keyed.press(Action::SLabel(1));
    let record = keyed.silent_record(&c).unwrap();
    let want = SilentWallet::parse(&record).unwrap();
    let mut app = testkit::started();
    files(
        &mut app,
        &[
            ("vault.kdbx", testkit::test_vault().unwrap()),
            ("silent-1.txt", record.as_bytes().to_vec()),
        ],
    );
    app.press(Action::Vault(V::OpenFrom(0, Screen::Files)));
    crate::common::type_text(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..10u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    let file = app
        .inbox()
        .iter()
        .position(|i| i.name == "silent-1.txt")
        .unwrap();
    testkit::load_wallet(&mut app, file);
    let w = app.wallet_index();
    app.press(Action::Vault(V::SaveWallet(w)));
    crate::common::top_up(&mut app);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    let mut written = Vec::new();
    while let Some(cmd) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { outbox, .. } = cmd {
            written = testkit::plain_files(outbox);
        }
    }
    assert_eq!(written.len(), 1, "the vault file was sealed");

    // Power off: a new process with only the written file.
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(written),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    let _ = app.frame();
    app.press(Action::Vault(V::FocusPassphrase));
    crate::common::type_text(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1);
    let open = &app.vaults().open[0];
    let held = app.vaults().store().contents_for_tests(open.id);
    let (_, saved) = held
        .of(kind::WALLET)
        .last()
        .expect("the wallet item is there");
    let back = faraday_core::wallet::read_wallet(&c, saved.text(field::WALLET).unwrap()).unwrap();
    assert_eq!(back.silent(), Some(&want));
    assert_eq!(back.silent().unwrap().address(), want.address());
    // Load takes the item as a wallet of the session, with its record.
    app.press(Action::Vault(V::LoadAll(0)));
    let loaded = app
        .session
        .wallets
        .iter()
        .find_map(|w| w.policy().silent())
        .expect("the saved wallet loads");
    assert_eq!(loaded, &want);
    assert_eq!(loaded.labels, 1);
}

#[test]
fn after_a_label_is_handed_out_the_wallets_checksum_is_its_exported_files() {
    let c = faraday_core::testkit::compute();
    let mut app = with_key();
    app.press(Action::Silent);
    app.press(Action::SAddWallet);
    let before = app.session.wallets[0].checksum().to_string();
    app.press(Action::SLabel(1));
    app.press(Action::SLabel(1));
    let after = app.session.wallets[0].checksum().to_string();
    // A silent payments wallet is known by its address, which a label
    // leaves as it is.
    assert_eq!(before, after);
    app.press(Action::SRecord);
    let text = String::from_utf8(app.outbox[0].bytes.to_vec()).unwrap();
    let exported =
        faraday_core::wallet::read_wallet(&c, &text).expect("the file reads as a wallet");
    assert_eq!(exported.silent().unwrap().labels, 2);
    assert_eq!(exported.checksum(), after);
}

#[test]
fn a_record_with_the_keys_spend_key_and_another_scan_key_is_refused_while_the_key_is_loaded() {
    let bad = record_of(0, 0, Some(another_scan_key()));
    let fp = faraday_core::wallet::fp_text(bad.fingerprint).to_string();
    let mut app = with_key();
    files(&mut app, &[("silent-1.txt", bad.to_text().into_bytes())]);
    faraday_core::testkit::load_wallet(&mut app, 0);
    assert!(app.session.wallets.is_empty());
    let toast = app.toast_text().unwrap_or_default();
    assert!(
        toast.contains(&format!("does not match key {fp}")),
        "{toast}"
    );
    // It stays in Files.
    assert_eq!(app.inbox().len(), 1);
}

#[test]
fn a_record_loaded_before_its_key_shows_no_address_once_the_key_says_it_is_not_its() {
    let c = faraday_core::testkit::compute();
    let bad = record_of(0, 0, Some(another_scan_key()));
    let fp = faraday_core::wallet::fp_text(bad.fingerprint).to_string();
    let mut app = faraday_core::testkit::started();
    files(&mut app, &[("silent-1.txt", bad.to_text().into_bytes())]);
    faraday_core::testkit::load_wallet(&mut app, 0);
    assert_eq!(app.session.wallets.len(), 1, "{:?}", app.toast_text());
    // Without its key the card shows the record's address, not checked.
    app.press(Action::Nav(Screen::Wallets));
    let text = drawn(&mut app);
    assert!(text.contains("Not checked"), "{text}");
    assert!(!text.contains("does not match"), "{text}");
    add_key(&mut app);
    // The card: no address, and why.
    app.press(Action::Nav(Screen::Wallets));
    let text = drawn(&mut app);
    assert!(text.contains(&format!("does not match key {fp}")), "{text}");
    // Its key chip says so, where it said the key is here and the wallet
    // can spend.
    assert!(text.contains("Does not match"), "{text}");
    assert!(!text.contains("Can spend"), "{text}");
    assert!(!app.drawn_texts().iter().any(|t| t == "Here"), "{text}");
    assert!(!text.contains("Not checked"), "{text}");
    let address = bad.address();
    assert!(!text.contains(&address[..16]), "{text}");
    // Its row in the tile of the wallets (the tile where the list stood)
    // says so too, and shows no address, "Can spend" or key here.
    app.press(Action::Nav(Screen::WalletsStart));
    let text = drawn(&mut app);
    assert!(text.contains("Does not match its key"), "{text}");
    assert!(!text.contains("Can spend"), "{text}");
    assert!(!text.contains(&address[..16]), "{text}");
    // Its page: no address, no labels, no scan key.
    app.press(Action::SWallet(0));
    assert_eq!(app.screen, Screen::Silent);
    assert_eq!(app.silent_address(&c), None);
    assert!(!app.silent_key_here(&c));
    let text = drawn(&mut app);
    assert!(text.contains(&format!("does not match key {fp}")), "{text}");
    app.press(Action::SLabel(1));
    assert_eq!(app.silent_address(&c), None);
    assert_eq!(app.silent_record(&c), None);
    // Its code and its public files are not made.
    assert!(
        app.code_view(&c, faraday_core::Code::Descriptor(0))
            .is_err()
    );
    app.press(Action::SRecord);
    app.press(Action::SQr(false));
    assert!(app.outbox.is_empty());
}

#[test]
fn a_record_at_another_account_than_its_points_is_refused_and_one_at_its_own_account_is_the_keys() {
    let c = faraday_core::testkit::compute();
    // Account 1 stated, account 0's points.
    let mut app = with_key();
    files(
        &mut app,
        &[("silent-1.txt", record_of(1, 0, None).to_text().into_bytes())],
    );
    faraday_core::testkit::load_wallet(&mut app, 0);
    assert!(app.session.wallets.is_empty());
    assert!(
        app.toast_text()
            .unwrap_or_default()
            .contains("does not match key")
    );
    // Account 1 stated and its own points: the key's wallet at account 1.
    let mut app = with_key();
    files(
        &mut app,
        &[("silent-1.txt", record_of(1, 1, None).to_text().into_bytes())],
    );
    faraday_core::testkit::load_wallet(&mut app, 0);
    assert_eq!(app.session.wallets.len(), 1, "{:?}", app.toast_text());
    app.press(Action::SWallet(0));
    let secp = Secp256k1::new();
    let r = Receiver::derive(&c, app.session.keys.master_for_tests(0), 1);
    let derived = r.address(&secp).unwrap().as_str().to_string();
    assert_eq!(app.silent_address(&c).as_deref(), Some(derived.as_str()));
    assert!(app.silent_key_here(&c));
    // A label is the key's at that account too.
    app.press(Action::SLabel(1));
    let one = r.labelled_address(&secp, 1).unwrap().as_str().to_string();
    assert_eq!(app.silent_address(&c).as_deref(), Some(one.as_str()));
}

#[test]
fn the_keys_own_record_with_the_key_loaded_shows_the_address_the_key_gives_unmarked() {
    let c = faraday_core::testkit::compute();
    let mut app = with_key();
    files(
        &mut app,
        &[("silent-1.txt", record_of(0, 0, None).to_text().into_bytes())],
    );
    faraday_core::testkit::load_wallet(&mut app, 0);
    assert_eq!(app.session.wallets.len(), 1, "{:?}", app.toast_text());
    let secp = Secp256k1::new();
    let r = Receiver::derive(&c, app.session.keys.master_for_tests(0), 0);
    let derived = r.address(&secp).unwrap().as_str().to_string();
    app.press(Action::Nav(Screen::Wallets));
    let text = drawn(&mut app);
    assert!(!text.contains("Not checked"), "{text}");
    assert!(!text.contains("does not match"), "{text}");
    app.press(Action::SWallet(0));
    assert_eq!(app.silent_address(&c).as_deref(), Some(derived.as_str()));
    let text = drawn(&mut app);
    assert!(!text.contains("Not checked"), "{text}");
    assert!(app.code_view(&c, faraday_core::Code::Descriptor(0)).is_ok());
}

#[test]
fn the_keys_own_record_without_the_key_shows_its_address_marked_not_checked() {
    let c = faraday_core::testkit::compute();
    let honest = record_of(0, 0, None);
    let fp = faraday_core::wallet::fp_text(honest.fingerprint).to_string();
    let mut app = faraday_core::testkit::started();
    files(&mut app, &[("silent-1.txt", honest.to_text().into_bytes())]);
    faraday_core::testkit::load_wallet(&mut app, 0);
    app.press(Action::SWallet(0));
    assert_eq!(
        app.silent_address(&c).as_deref(),
        Some(honest.address().as_str())
    );
    let line = format!("Not checked: load key {fp} to check this address");
    let text = drawn(&mut app);
    assert!(text.contains("Not checked"), "{text}");
    app.press(Action::Nav(Screen::Wallets));
    let text = drawn(&mut app);
    assert!(text.contains("Not checked"), "{text}");
    // Its code says so too.
    let view = app
        .code_view(&c, faraday_core::Code::Descriptor(0))
        .unwrap();
    assert!(
        view.label.iter().any(|l| l.as_str() == line),
        "{:?}",
        view.label
    );
}

// BIP-392's `sp()` descriptor, written and read (`docs/INTEROP.md` §5, BIP-392 sp()).

/// The test key's own silent payments wallet, its `sp()` descriptor
/// taken through the secret sheet: the app, and the descriptor's text.
fn descriptor_out() -> (Faraday, String) {
    let mut app = with_key();
    app.press(Action::Silent);
    app.press(Action::SAddWallet);
    app.press(Action::SDescriptorOut(0));
    assert_eq!(app.sheet, Some(Sheet::SecretOut));
    assert!(app.outbox.is_empty(), "nothing out before the sheet");
    app.press(Action::SecretUnprotected);
    app.press(Action::SecretHold);
    let name = format!(
        "{}-sp-descriptor.txt",
        faraday_core::file_stem(&app.session.wallets[0].name).as_str()
    );
    let item = app
        .outbox
        .iter()
        .find(|i| i.name.as_str() == name.as_str())
        .unwrap_or_else(|| panic!("no {name}"));
    let text = String::from_utf8(item.bytes.to_vec()).unwrap();
    (app, text)
}

#[test]
fn a_silent_payments_wallets_sp_descriptor_leaves_through_the_secret_sheet_with_its_origin() {
    let c = faraday_core::testkit::compute();
    let (app, text) = descriptor_out();
    let fp = faraday_core::wallet::fp_text(app.session.keys[0].fingerprint).to_string();
    let coin = app.session.network().coin_type();
    assert!(
        text.starts_with(&format!("sp([{fp}/352h/{coin}h/0h]")),
        "{text}"
    );
    // It is the key's own scan key.
    let r = Receiver::derive(&c, app.session.keys.master_for_tests(0), 0);
    assert!(text.contains(r.scan_key_text().unwrap().as_str()));
    // Read back, it is the wallet it came from.
    let back = faraday_core::wallet::read_wallet(&c, &text).unwrap();
    let mine = app.session.wallets[0].policy().silent().unwrap();
    let read = back.silent().unwrap();
    assert_eq!(read.address(), mine.address());
    assert_eq!(
        (read.fingerprint, read.account),
        (mine.fingerprint, mine.account)
    );
}

#[test]
fn an_sp_descriptor_with_no_key_loaded_offers_nothing_out() {
    let c = faraday_core::testkit::compute();
    let mut keyed = with_key();
    keyed.press(Action::Silent);
    let record = keyed.silent_record(&c).unwrap();
    let mut app = faraday_core::testkit::started();
    files(&mut app, &[("silent-1.txt", record.as_bytes().to_vec())]);
    faraday_core::testkit::load_wallet(&mut app, 0);
    app.press(Action::SDescriptorOut(0));
    assert_ne!(app.sheet, Some(Sheet::SecretOut));
    assert!(app.outbox.is_empty());
    assert!(app.toast_text().unwrap_or_default().contains("sp()"));
}

#[test]
fn an_sp_descriptor_from_files_loads_as_its_wallet_and_one_whose_keys_are_not_the_keys_is_refused()
{
    let (_, text) = descriptor_out();
    // Into a session without the key: the wallet, named by its file, a
    // wallet file in Files (the scan key in it spends nothing).
    let mut app = faraday_core::testkit::started();
    files(&mut app, &[("sp.txt", text.clone().into_bytes())]);
    assert_eq!(app.inbox()[0].kind, faraday_core::wallet::FileKind::Wallet);
    faraday_core::testkit::load_wallet(&mut app, 0);
    assert_eq!(app.session.wallets.len(), 1, "{:?}", app.toast_text());
    assert_eq!(Kind::of(app.session.wallets[0].policy()), Kind::Silent);
    // With the key loaded, a descriptor at its origin over another scan
    // key is refused.
    let other = Receiver::new(
        SecretKey::from_slice(&[0x33u8; 32]).unwrap(),
        another_scan_key(),
        app.session.network(),
    );
    let origin = &text[3..text.find(']').unwrap() + 1];
    let bad = format!("sp({origin}{})\n", other.scan_key_text().unwrap().as_str());
    let mut app = with_key();
    files(&mut app, &[("sp.txt", bad.into_bytes())]);
    faraday_core::testkit::load_wallet(&mut app, 0);
    assert!(app.session.wallets.is_empty());
    let toast = app.toast_text().unwrap_or_default();
    assert!(toast.contains("does not match key"), "{toast}");
    // And one with no origin is refused, named.
    let mut app = faraday_core::testkit::started();
    let bare = format!("sp({})\n", other.scan_key_text().unwrap().as_str());
    files(&mut app, &[("sp.txt", bare.into_bytes())]);
    faraday_core::testkit::load_wallet(&mut app, 0);
    assert!(app.session.wallets.is_empty());
    let toast = app.toast_text().unwrap_or_default();
    assert!(toast.contains("names no key origin"), "{toast}");
}

/// Coldcard's export of one key's single-sig accounts and its silent
/// payments account (Sparrow's test files).
const CC_SP: &str = include_str!("../vectors/interop/sparrow/cc-singlesig-sp-keystore-1.json");

/// Files' Load of that export, its dialog open.
fn coldcard_asked() -> Faraday {
    let mut app = faraday_core::testkit::started();
    files(&mut app, &[("cc.json", CC_SP.as_bytes().to_vec())]);
    faraday_core::testkit::load_wallet(&mut app, 0);
    assert_eq!(app.sheet, Some(Sheet::Account), "{:?}", app.toast_text());
    assert!(app.session.wallets.is_empty(), "loaded before a choice");
    let shown = drawn(&mut app);
    assert!(shown.contains("Single-sig · BIP-84"), "{shown}");
    assert!(shown.contains("Silent payments · BIP-352"), "{shown}");
    app
}

/// A Coldcard export holding a single-sig account and a silent payments
/// account asks which to load (owner, 2026-10-29): the one chosen loads,
/// and Cancel loads neither.
#[test]
fn a_coldcard_export_of_both_accounts_loads_the_one_chosen() {
    use faraday_core::wallet::Account;
    let mut app = coldcard_asked();
    app.press(Action::LoadAccount(Some(Account::Silent)));
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.wallets.len(), 1, "{:?}", app.toast_text());
    let w = app.session.wallets[0].policy().silent().expect("silent");
    assert_eq!(w.fingerprint.to_string(), "0f056943");
    let mut app = coldcard_asked();
    app.press(Action::LoadAccount(Some(Account::SingleSig)));
    assert_eq!(app.session.wallets.len(), 1, "{:?}", app.toast_text());
    let d = app.session.wallets[0]
        .policy()
        .to_descriptor()
        .as_str()
        .to_owned();
    assert!(d.starts_with("wpkh([0f056943/84h/1h/0h]"), "{d}");
    let mut app = coldcard_asked();
    app.press(Action::LoadAccount(None));
    assert_eq!(app.sheet, None);
    assert!(app.session.wallets.is_empty());
}

#[test]
fn a_silent_payments_wallets_backup_sheet_names_its_key_and_address_and_lists_no_receive_addresses()
{
    let c = faraday_core::testkit::compute();
    let mut app = with_key();
    app.press(Action::Silent);
    let record = app.silent_record(&c).unwrap();
    let mut app = with_key();
    files(&mut app, &[("silent-1.txt", record.as_bytes().to_vec())]);
    faraday_core::testkit::load_wallet(&mut app, 0);
    let w = &app.session.wallets[0];
    let sheet = String::from(&*faraday_core::wallet_backup::sheet_wallet(
        &c,
        &app.session,
        w,
    ));
    let fp = faraday_core::wallet::fp_text(app.session.keys[0].fingerprint).to_string();
    let silent = w.policy().silent().unwrap();
    assert!(
        sheet.starts_with("faraday-sheet 1\nkind: wallet\n"),
        "{sheet}"
    );
    assert!(sheet.contains(&format!("key: 1 {fp} ")), "{sheet}");
    assert!(sheet.contains(&silent.address()), "{sheet}");
    assert!(sheet.contains("account 0"), "{sheet}");
    assert!(!sheet.contains("address: "), "{sheet}");
}

#[test]
fn one_step_is_drawn_at_a_time() {
    let mut app = with_key();
    app.press(Action::Silent);
    let _ = app.frame();
    let shown = drawn(&mut app);
    assert!(shown.contains("Step 2 of 4") && !shown.contains("Step 1 of 4"));
    app.press(Action::SStep(0));
    let _ = app.frame();
    let shown = drawn(&mut app);
    assert!(shown.contains("Step 1 of 4") && !shown.contains("Step 2 of 4"));
}

/// `record` stated under a fingerprint no loaded key has.
fn under_another_fingerprint(mut record: SilentWallet) -> SilentWallet {
    record.fingerprint = faraday_bip::keys::Fingerprint([0; 4]);
    record
}

#[test]
fn a_record_naming_the_loaded_keys_points_under_another_fingerprint_is_refused() {
    // At account 0, at account 1, and with account 0's points under
    // account 1.
    for (account, at) in [(0, 0), (1, 1), (1, 0)] {
        let crafted = under_another_fingerprint(record_of(account, at, None));
        let mut app = with_key();
        files(
            &mut app,
            &[("silent-1.txt", crafted.to_text().into_bytes())],
        );
        faraday_core::testkit::load_wallet(&mut app, 0);
        assert!(app.session.wallets.is_empty(), "account {account} at {at}");
        let toast = app.toast_text().unwrap_or_default();
        assert!(toast.contains("its record names key 00000000"), "{toast}");
        assert!(
            toast.contains(&format!(
                "Key {} gives",
                &*faraday_core::wallet::fp_text(app.session.keys[0].fingerprint)
            )),
            "{toast}"
        );
    }
}

#[test]
fn a_record_of_a_key_that_is_not_loaded_under_another_fingerprint_stays_not_checked() {
    let crafted = under_another_fingerprint(record_of(0, 0, None));
    let mut app = faraday_core::testkit::started();
    files(
        &mut app,
        &[("silent-1.txt", crafted.to_text().into_bytes())],
    );
    faraday_core::testkit::load_wallet(&mut app, 0);
    assert_eq!(app.session.wallets.len(), 1, "{:?}", app.toast_text());
    app.press(Action::Nav(Screen::Wallets));
    let text = drawn(&mut app);
    assert!(text.contains("Not checked"), "{text}");
}

#[test]
fn a_record_under_another_fingerprint_shows_no_address_while_the_key_it_names_is_loaded() {
    let c = faraday_core::testkit::compute();
    let crafted = under_another_fingerprint(record_of(0, 0, None));
    let mut app = faraday_core::testkit::started();
    files(
        &mut app,
        &[("silent-1.txt", crafted.to_text().into_bytes())],
    );
    faraday_core::testkit::load_wallet(&mut app, 0);
    assert_eq!(app.session.wallets.len(), 1, "{:?}", app.toast_text());
    app.press(Action::SWallet(0));
    assert_eq!(
        app.silent_address(&c).as_deref(),
        Some(crafted.address().as_str())
    );
    let text = drawn(&mut app);
    assert!(text.contains("Not checked"), "{text}");
    // The key is loaded: the record is taken out of the session, with
    // the refusal's words.
    add_key(&mut app);
    assert!(app.session.wallets.is_empty());
    let toast = app.toast_text().unwrap_or_default();
    assert!(toast.contains("its record names key 00000000"), "{toast}");
}

/// A file whose record states `account` hardened, over the test key.
fn record_at_account(account: u32) -> Vec<u8> {
    let text = record_of(1, 1, None).to_text();
    text.replace("/1h]", &format!("/{account}h]")).into_bytes()
}

#[test]
fn a_record_of_an_account_of_two_to_the_31_or_more_is_refused_and_nothing_panics() {
    for account in [1u32 << 31, u32::MAX] {
        // With the key loaded, and with none (then a key loads).
        let mut app = with_key();
        files(&mut app, &[("silent-1.txt", record_at_account(account))]);
        faraday_core::testkit::load_wallet(&mut app, 0);
        assert!(app.session.wallets.is_empty(), "{account}");
        let mut app = faraday_core::testkit::started();
        files(&mut app, &[("silent-1.txt", record_at_account(account))]);
        faraday_core::testkit::load_wallet(&mut app, 0);
        add_key(&mut app);
        let _ = drawn(&mut app);
        assert!(
            app.session
                .wallets
                .iter()
                .all(|w| w.policy().silent().is_none_or(|s| s.account < 1 << 31))
        );
    }
}

#[test]
fn a_record_that_differs_from_its_key_stays_without_an_address_on_the_pressed_keys_page() {
    let c = faraday_core::testkit::compute();
    let bad = record_of(0, 0, Some(another_scan_key()));
    let mut app = faraday_core::testkit::started();
    files(&mut app, &[("silent-1.txt", bad.to_text().into_bytes())]);
    faraday_core::testkit::load_wallet(&mut app, 0);
    add_key(&mut app);
    // Another key too.
    app.press(Action::Entry(None));
    faraday_core::testkit::type_words(
        &mut app,
        &faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[1].0),
    );
    app.press(Action::EntryAdd);
    faraday_core::testkit::keep_published(&mut app);
    let other = app.session.keys[1].fingerprint.0;
    let at = app
        .session
        .wallets
        .iter()
        .position(|w| w.policy().silent().is_some_and(|s| s.scan == bad.scan))
        .unwrap();
    app.press(Action::SWallet(at));
    app.press(Action::SStep(faraday_core::silent::sstep::KEY));
    app.press(Action::SKey(other));
    assert_eq!(app.silent_address(&c), None);
    assert_eq!(app.silent_record(&c), None);
    let text = drawn(&mut app);
    assert!(!text.contains("Show as QR"), "{text}");
    assert!(!text.contains(&bad.address()[..16]), "{text}");
}

#[test]
fn a_crafted_record_is_taken_out_when_its_key_loads_and_the_honest_one_is_then_added() {
    let c = faraday_core::testkit::compute();
    let crafted = under_another_fingerprint(record_of(0, 0, None));
    let honest = record_of(0, 0, None);
    let mut app = faraday_core::testkit::started();
    files(
        &mut app,
        &[("silent-1.txt", crafted.to_text().into_bytes())],
    );
    faraday_core::testkit::load_wallet(&mut app, 0);
    assert_eq!(app.session.wallets.len(), 1);
    add_key(&mut app);
    assert!(app.session.wallets.is_empty());
    files(&mut app, &[("silent-2.txt", honest.to_text().into_bytes())]);
    faraday_core::testkit::load_wallet(&mut app, 0);
    assert_eq!(app.session.wallets.len(), 1, "{:?}", app.toast_text());
    app.press(Action::SWallet(0));
    assert!(app.silent_key_here(&c));
    assert_eq!(
        app.silent_address(&c).as_deref(),
        Some(honest.address().as_str())
    );
    // The same file loaded again is still one wallet.
    files(&mut app, &[("silent-2.txt", honest.to_text().into_bytes())]);
    faraday_core::testkit::load_wallet(&mut app, 0);
    assert_eq!(app.session.wallets.len(), 1);
}

#[test]
fn a_record_of_the_same_address_under_another_fingerprint_is_not_the_same_wallet() {
    let c = faraday_core::testkit::compute();
    let honest = record_of(0, 0, None);
    let crafted = under_another_fingerprint(record_of(0, 0, None));
    let at_account_1 = record_of(1, 0, None);
    let same = |w: &SilentWallet| {
        faraday_core::wallet::same_wallet(
            &faraday_core::wallet::read_wallet(&c, &w.to_text()).unwrap(),
        )
    };
    assert_eq!(same(&honest), same(&record_of(0, 0, None)));
    assert_ne!(same(&honest), same(&crafted));
    assert_ne!(same(&honest), same(&at_account_1));
}

/// Adds the first test key from a wallet's page (Add a key, then the
/// words): the page is still there until the key is added.
fn add_key_from_page(app: &mut Faraday) {
    app.press(Action::Entry(None));
    for c in faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    assert!(
        app.silent().is_some(),
        "the page is kept while a key is added"
    );
    app.press(Action::EntryAdd);
    faraday_core::testkit::keep_published(app);
}

/// Loads the second test seed as a key, returning its fingerprint.
fn add_other_key(app: &mut Faraday) -> [u8; 4] {
    app.press(Action::Entry(None));
    faraday_core::testkit::type_words(
        app,
        &faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[1].0),
    );
    app.press(Action::EntryAdd);
    faraday_core::testkit::keep_published(app);
    app.session.keys[app.session.keys.len() - 1].fingerprint.0
}

fn qr_lines(app: &Faraday) -> String {
    let c = faraday_core::testkit::compute();
    let view = app
        .code_view(&c, faraday_core::Code::Silent(false))
        .expect("the page offers its code");
    let mut all = vec![view.title.to_string()];
    all.push(view.png_name.to_string());
    all.extend(view.label.iter().map(|l| l.to_string()));
    all.join(" | ")
}

#[test]
fn a_wallets_page_names_its_records_fingerprint_whichever_key_is_pressed() {
    let crafted = under_another_fingerprint(record_of(0, 0, None));
    let mut app = faraday_core::testkit::started();
    files(
        &mut app,
        &[("silent-1.txt", crafted.to_text().into_bytes())],
    );
    faraday_core::testkit::load_wallet(&mut app, 0);
    let x = add_other_key(&mut app);
    let x_text = faraday_core::wallet::fp_text(faraday_bip::keys::Fingerprint(x)).to_string();
    assert_eq!(app.session.wallets.len(), 1, "{:?}", app.toast_text());
    app.press(Action::SWallet(0));
    app.press(Action::SStep(faraday_core::silent::sstep::KEY));
    app.press(Action::SKey(x));
    let text = drawn(&mut app);
    assert!(text.contains("00000000"), "{text}");
    assert!(!text.contains(&format!("load key {x_text}")), "{text}");
    let qr = qr_lines(&app);
    assert!(qr.contains("silent-00000000"), "{qr}");
    assert!(qr.contains("00000000") && !qr.contains(&x_text), "{qr}");
    app.press(Action::SRecord);
    assert_eq!(app.outbox[0].name.to_string(), "silent-00000000.txt");
}

#[test]
fn a_record_that_differs_names_its_own_key_whichever_key_is_pressed() {
    let bad = record_of(0, 0, Some(another_scan_key()));
    let mut app = faraday_core::testkit::started();
    files(&mut app, &[("silent-1.txt", bad.to_text().into_bytes())]);
    faraday_core::testkit::load_wallet(&mut app, 0);
    add_key(&mut app);
    let own = faraday_core::wallet::fp_text(app.session.keys[0].fingerprint).to_string();
    let y = add_other_key(&mut app);
    let y_text = faraday_core::wallet::fp_text(faraday_bip::keys::Fingerprint(y)).to_string();
    let at = app
        .session
        .wallets
        .iter()
        .position(|w| w.policy().silent().is_some())
        .unwrap();
    app.press(Action::SWallet(at));
    app.press(Action::SStep(faraday_core::silent::sstep::KEY));
    app.press(Action::SKey(y));
    let text = drawn(&mut app);
    assert!(
        text.contains(&format!("does not match key {own}")),
        "{text}"
    );
    assert!(
        !text.contains(&format!("does not match key {y_text}")),
        "{text}"
    );
}

#[test]
fn a_record_dropped_while_its_page_adds_a_key_ends_the_page_and_spares_the_other_record() {
    let crafted = under_another_fingerprint(record_of(0, 0, None));
    // The key that is added makes the crafted record forged; the other
    // record beside it (another key's points) stays with its labels.
    let mut other = record_of(0, 0, Some(another_scan_key()));
    other.fingerprint = faraday_bip::keys::Fingerprint([0x77; 4]);
    other.labels = 3;
    let mut app = faraday_core::testkit::started();
    files(
        &mut app,
        &[
            ("silent-1.txt", crafted.to_text().into_bytes()),
            ("silent-2.txt", other.to_text().into_bytes()),
        ],
    );
    faraday_core::testkit::load_wallet(&mut app, 0);
    faraday_core::testkit::load_wallet(&mut app, 1);
    assert_eq!(app.session.wallets.len(), 2, "{:?}", app.toast_text());
    let at = app
        .session
        .wallets
        .iter()
        .position(|w| w.policy().silent() == Some(&crafted))
        .unwrap();
    app.press(Action::SWallet(at));
    app.press(Action::SStep(faraday_core::silent::sstep::KEY));
    add_key_from_page(&mut app);
    assert_eq!(app.session.wallets.len(), 1);
    let text = drawn(&mut app);
    assert!(!text.contains(&crafted.address()[..16]), "{text}");
    assert_ne!(app.screen, Screen::Silent);
    assert!(app.silent().is_none());
    // The other record keeps its labels.
    assert_eq!(app.session.wallets[0].policy().silent().unwrap().labels, 3);
}

#[test]
fn a_wallets_page_follows_its_record_when_a_record_before_it_is_dropped() {
    let crafted = under_another_fingerprint(record_of(0, 0, None));
    let mut other = record_of(0, 0, Some(another_scan_key()));
    other.fingerprint = faraday_bip::keys::Fingerprint([0x77; 4]);
    let mut app = faraday_core::testkit::started();
    files(
        &mut app,
        &[
            ("silent-1.txt", crafted.to_text().into_bytes()),
            ("silent-2.txt", other.to_text().into_bytes()),
        ],
    );
    faraday_core::testkit::load_wallet(&mut app, 0);
    faraday_core::testkit::load_wallet(&mut app, 1);
    let at = app
        .session
        .wallets
        .iter()
        .position(|w| w.policy().silent() == Some(&other))
        .unwrap();
    app.press(Action::SWallet(at));
    app.press(Action::SStep(faraday_core::silent::sstep::KEY));
    add_key_from_page(&mut app);
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(app.silent_wallet(), Some(&other));
    let text = drawn(&mut app);
    assert!(!text.contains(&crafted.address()[..16]), "{text}");
}

#[test]
fn the_page_of_the_only_wallet_ends_when_its_record_is_dropped() {
    let crafted = under_another_fingerprint(record_of(0, 0, None));
    let mut app = faraday_core::testkit::started();
    files(
        &mut app,
        &[("silent-1.txt", crafted.to_text().into_bytes())],
    );
    faraday_core::testkit::load_wallet(&mut app, 0);
    app.press(Action::SWallet(0));
    app.press(Action::SStep(faraday_core::silent::sstep::KEY));
    add_key_from_page(&mut app);
    assert!(app.session.wallets.is_empty());
    let text = drawn(&mut app);
    assert!(!text.contains("Not checked"), "{text}");
    assert!(!text.contains("Show as QR"), "{text}");
}

/// An open, empty vault (a file `vault.kdbx` unlocked from the Vaults tab).
/// A silent record is not kept in a vault file that is read again (the file
/// reader takes a wallet as a policy or a descriptor), so a vault holds one
/// here as the session saved it.
fn open_empty_vault(app: &mut Faraday) {
    use faraday_core::vault::VaultAction as V;
    let file = faraday_core::testkit::kdbx_vault(
        "Test vault",
        faraday_core::testkit::VAULT_PASSPHRASE,
        0x7a,
        Vec::new(),
    )
    .unwrap();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            ("vault.kdbx".to_string(), file),
            ("silent-1.txt".to_string(), Vec::new()),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Vaults));
    let n = app
        .vault_files()
        .iter()
        .position(|f| f.name == "vault.kdbx")
        .expect("the vault is listed");
    app.press(Action::Vault(V::Open(n)));
    crate::common::type_text(app, faraday_core::testkit::VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1);
}

/// Loads `record`, as a file `silent-1.txt` in Files, into the session.
fn load_record(app: &mut Faraday, record: &SilentWallet) {
    let mut inbox: Vec<(String, Vec<u8>)> = app
        .inbox()
        .iter()
        .filter(|i| i.name == "vault.kdbx")
        .map(|i| (i.name.to_string(), i.bytes.to_vec()))
        .collect();
    inbox.push(("silent-1.txt".to_string(), record.to_text().into_bytes()));
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(inbox),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let at = app
        .inbox()
        .iter()
        .position(|i| i.name == "silent-1.txt")
        .unwrap();
    faraday_core::testkit::load_wallet(app, at);
}

fn silent_records(app: &Faraday) -> Vec<SilentWallet> {
    app.session
        .wallets
        .iter()
        .filter_map(|w| w.policy().silent().cloned())
        .collect()
}

fn wallets_in_vault(app: &Faraday) -> usize {
    crate::common::contents(app, 0)
        .of(faraday_vault::records::kind::WALLET)
        .count()
}

/// Saves the session's only wallet into the open vault and takes it out of
/// the session.
fn save_and_forget(app: &mut Faraday) {
    app.press(Action::Vault(faraday_core::vault::VaultAction::SaveWallet(
        0,
    )));
    app.press(Action::RemoveWallet(0));
    assert!(app.session.wallets.is_empty());
}

#[test]
fn a_vaults_honest_record_loads_beside_a_crafted_one_of_its_address() {
    use faraday_core::vault::VaultAction as V;
    let honest = record_of(0, 0, None);
    let crafted = under_another_fingerprint(record_of(0, 0, None));
    let mut app = faraday_core::testkit::started();
    open_empty_vault(&mut app);
    load_record(&mut app, &honest);
    save_and_forget(&mut app);
    assert_eq!(wallets_in_vault(&app), 1);
    load_record(&mut app, &crafted);
    assert_eq!(silent_records(&app), vec![crafted.clone()]);
    app.press(Action::Vault(V::LoadAll(0)));
    faraday_core::testkit::settle(&mut app);
    faraday_core::testkit::confirm_wallets(&mut app);
    let held = silent_records(&app);
    assert!(held.contains(&honest), "{held:?}");
    assert!(held.contains(&crafted), "{held:?}");
}

#[test]
fn a_crafted_record_in_the_vault_does_not_make_the_honest_one_already_there() {
    let honest = record_of(0, 0, None);
    let crafted = under_another_fingerprint(record_of(0, 0, None));
    let mut app = faraday_core::testkit::started();
    open_empty_vault(&mut app);
    load_record(&mut app, &crafted);
    save_and_forget(&mut app);
    assert_eq!(wallets_in_vault(&app), 1);
    load_record(&mut app, &honest);
    app.press(Action::Vault(faraday_core::vault::VaultAction::SaveWallet(
        0,
    )));
    assert_eq!(wallets_in_vault(&app), 2, "{}", drawn(&mut app));
}

fn add_key_waiting_for_backup(app: &mut Faraday, seed: usize) {
    app.press(Action::Entry(None));
    faraday_core::testkit::type_words(
        app,
        &faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[seed].0),
    );
    app.press(Action::EntryAdd);
}

#[test]
fn a_keys_address_under_another_fingerprint_stays_hidden_while_its_backup_waits_and_another_key_is_pressed()
 {
    let c = faraday_core::testkit::compute();
    let crafted = under_another_fingerprint(record_of(0, 0, None));
    let mut app = faraday_core::testkit::started();
    files(
        &mut app,
        &[("silent-1.txt", crafted.to_text().into_bytes())],
    );
    faraday_core::testkit::load_wallet(&mut app, 0);
    add_key_waiting_for_backup(&mut app, 0);
    add_key_waiting_for_backup(&mut app, 1);
    let other = app.session.keys[1].fingerprint.0;
    app.press(Action::SWallet(0));
    app.press(Action::SStep(faraday_core::silent::sstep::KEY));
    app.press(Action::SKey(other));
    assert_eq!(app.silent_address(&c), None);
    assert_eq!(app.silent_record(&c), None);
    let text = drawn(&mut app);
    assert!(!text.contains(&crafted.address()[..16]), "{text}");
    assert!(!text.contains("Show as QR"), "{text}");
}

#[test]
fn a_wallets_page_with_another_loaded_key_pressed_still_checks_its_own_record() {
    let honest = record_of(0, 0, None);
    let mut app = with_key();
    files(&mut app, &[("silent-1.txt", honest.to_text().into_bytes())]);
    faraday_core::testkit::load_wallet(&mut app, 0);
    let y = add_other_key(&mut app);
    let at = app
        .session
        .wallets
        .iter()
        .position(|w| w.policy().silent() == Some(&honest))
        .expect("the record is a session wallet");
    app.press(Action::SWallet(at));
    app.press(Action::SStep(faraday_core::silent::sstep::KEY));
    app.press(Action::SKey(y));
    let text = drawn(&mut app);
    assert!(!text.contains("Not checked"), "{text}");
    assert!(!text.contains("Its key is not loaded"), "{text}");
    // The key the record names is the one on show, not the one pressed.
    let c = faraday_core::testkit::compute();
    let honest = app.silent_wallet().unwrap().address();
    assert_eq!(app.silent_address(&c).as_deref(), Some(honest.as_str()));
}

#[test]
fn a_record_that_differs_under_a_key_no_longer_loaded_says_the_key_is_not_loaded() {
    let y = {
        let mut scratch = faraday_core::testkit::started();
        add_other_key(&mut scratch)
    };
    let y_text = faraday_core::wallet::fp_text(faraday_bip::keys::Fingerprint(y)).to_string();
    // The first test key's points under the second key's fingerprint.
    let mut crafted = record_of(0, 0, None);
    crafted.fingerprint = faraday_bip::keys::Fingerprint(y);
    let mut app = faraday_core::testkit::started();
    files(
        &mut app,
        &[("silent-1.txt", crafted.to_text().into_bytes())],
    );
    faraday_core::testkit::load_wallet(&mut app, 0);
    add_other_key(&mut app);
    add_key(&mut app);
    // The key the record names goes; the key whose points it states stays.
    app.session.keys.remove(0);
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(
        app.session
            .wallets
            .iter()
            .filter(|w| w.policy().silent().is_some())
            .count(),
        1
    );
    let at = app
        .session
        .wallets
        .iter()
        .position(|w| w.policy().silent().is_some())
        .unwrap();
    app.press(Action::SWallet(at));
    let text = drawn(&mut app);
    assert!(
        text.contains(&format!("Key {y_text} is not loaded")),
        "{text}"
    );
    assert!(!text.contains("does not match key"), "{text}");
}

#[test]
fn two_records_over_one_wallets_points_are_two_rows_and_each_opens_its_own() {
    use faraday_shell_api::{BootState, DisplayInfo, SecureHardware};
    let crafted = under_another_fingerprint(record_of(0, 0, None));
    let honest = record_of(0, 0, None);
    let mut app = faraday_core::testkit::started();
    app.event(Event::Display(DisplayInfo {
        width: 1920,
        height: 1200,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    files(
        &mut app,
        &[
            ("silent-1.txt", crafted.to_text().into_bytes()),
            ("silent-2.txt", honest.to_text().into_bytes()),
        ],
    );
    faraday_core::testkit::load_wallet(&mut app, 0);
    faraday_core::testkit::load_wallet(&mut app, 1);
    assert_eq!(app.session.wallets.len(), 2, "{:?}", app.toast_text());
    assert_eq!(
        app.session.wallets[0].checksum(),
        app.session.wallets[1].checksum()
    );
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    let own = |fp: faraday_bip::keys::Fingerprint| faraday_core::wallet::fp_text(fp).to_string();
    for (w, record) in [(1, &honest), (0, &crafted)] {
        let row = Action::RowOpen(faraday_core::Row::Wallet(w));
        assert!(app.offers(row), "wallet {w}");
        app.press(row);
        let text = drawn(&mut app);
        assert!(
            text.contains(&own(record.fingerprint)),
            "wallet {w}: {text}"
        );
        let other = if w == 1 { &crafted } else { &honest };
        assert!(
            !text.contains(&own(other.fingerprint)),
            "wallet {w}: {text}"
        );
    }
}

/// A record whose scan key is another's, loaded with no key, then the key.
fn differing_wallet_then_key() -> (Faraday, SilentWallet) {
    let bad = record_of(0, 0, Some(another_scan_key()));
    let mut app = faraday_core::testkit::started();
    files(&mut app, &[("silent-1.txt", bad.to_text().into_bytes())]);
    faraday_core::testkit::load_wallet(&mut app, 0);
    assert_eq!(app.session.wallets.len(), 1, "{:?}", app.toast_text());
    add_key(&mut app);
    (app, bad)
}

#[test]
fn the_krux_code_tool_refuses_a_record_that_differs_and_takes_one_that_matches() {
    use faraday_core::kef_tool::{self, KefToolAction as K};
    let (mut app, bad) = differing_wallet_then_key();
    let fp = faraday_core::wallet::fp_text(bad.fingerprint).to_string();
    crate::common::open_kef_tool(&mut app);
    app.press(Action::KefTool(K::Wallet(0)));
    let toast = app.toast_text().unwrap_or_default();
    assert!(
        toast.contains(&format!("does not match key {fp}")),
        "{toast}"
    );
    assert!(kef_tool::state(&app).unwrap().form().is_none());
    // A record the key gives is taken.
    let good = record_of(0, 0, None);
    let mut app = with_key();
    files(&mut app, &[("silent-1.txt", good.to_text().into_bytes())]);
    faraday_core::testkit::load_wallet(&mut app, 0);
    crate::common::open_kef_tool(&mut app);
    app.press(Action::KefTool(K::Wallet(0)));
    assert!(kef_tool::state(&app).unwrap().form().is_some());
}

#[test]
fn saving_a_record_that_differs_to_a_vault_is_refused_and_one_that_matches_is_saved() {
    let (mut app, bad) = differing_wallet_then_key();
    let fp = faraday_core::wallet::fp_text(bad.fingerprint).to_string();
    crate::common::open_plain_vault(&mut app);
    app.press(Action::Vault(faraday_core::vault::VaultAction::SaveWallet(
        0,
    )));
    let toast = app.toast_text().unwrap_or_default();
    assert!(
        toast.contains(&format!("does not match key {fp}")),
        "{toast}"
    );
    let shown = crate::common::vault_wallet_item_text(&mut app);
    assert!(shown.is_empty(), "{shown}");
    // A record the key gives is saved and its item shows the address.
    let good = record_of(0, 0, None);
    let mut app = with_key();
    files(&mut app, &[("silent-1.txt", good.to_text().into_bytes())]);
    faraday_core::testkit::load_wallet(&mut app, 0);
    crate::common::open_plain_vault(&mut app);
    app.press(Action::Vault(faraday_core::vault::VaultAction::SaveWallet(
        0,
    )));
    let shown = crate::common::vault_wallet_item_text(&mut app);
    assert!(shown.contains(&good.address()[..24]), "{shown}");
    assert!(!shown.contains("does not match"), "{shown}");
}

#[test]
fn a_vault_item_whose_record_differs_from_the_loaded_key_says_so_and_shows_no_address() {
    let bad = record_of(0, 0, Some(another_scan_key()));
    let fp = faraday_core::wallet::fp_text(bad.fingerprint).to_string();
    let mut app = faraday_core::testkit::started();
    files(&mut app, &[("silent-1.txt", bad.to_text().into_bytes())]);
    faraday_core::testkit::load_wallet(&mut app, 0);
    crate::common::open_plain_vault(&mut app);
    // Without its key the record is saved, not checked.
    app.press(Action::Vault(faraday_core::vault::VaultAction::SaveWallet(
        0,
    )));
    let shown = crate::common::vault_wallet_item_text(&mut app);
    assert!(shown.contains(&bad.address()[..24]), "{shown}");
    add_key(&mut app);
    let shown = crate::common::vault_wallet_item_text(&mut app);
    assert!(
        shown.contains(&format!("does not match key {fp}")),
        "{shown}"
    );
    assert!(!shown.contains(&bad.address()[..24]), "{shown}");
    assert!(!shown.contains("Show as QR"), "{shown}");
}

/// A file in Files put into a vault with Add to vault, the vault sealed,
/// the device powered off and the vault opened again: what its wallet
/// item holds after, and the wallet Load makes of it.
fn added_from_a_file_and_reopened(
    name: &str,
    bytes: Vec<u8>,
) -> (Vec<String>, Option<SilentWallet>) {
    use faraday_core::StorageCommand;
    use faraday_core::testkit;
    use faraday_core::vault::VaultAction as V;
    use faraday_vault::records::{field, kind};
    let mut app = testkit::started();
    files(
        &mut app,
        &[
            ("vault.kdbx", testkit::test_vault().unwrap()),
            (name, bytes),
        ],
    );
    app.press(Action::Vault(V::OpenFrom(0, Screen::Files)));
    crate::common::type_text(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..10u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    let file = app.inbox().iter().position(|i| i.name == name).unwrap();
    app.press(Action::Vault(V::AddFile(file)));
    crate::common::top_up(&mut app);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    let mut written = Vec::new();
    while let Some(cmd) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { outbox, .. } = cmd {
            written = testkit::plain_files(outbox);
        }
    }
    assert_eq!(written.len(), 1, "the vault file was sealed");
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(written),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    let _ = app.frame();
    app.press(Action::Vault(V::FocusPassphrase));
    crate::common::type_text(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1);
    let open = &app.vaults().open[0];
    let held = app.vaults().store().contents_for_tests(open.id);
    let mut kept = Vec::new();
    for (_, r) in held.of(kind::WALLET) {
        kept.push(r.text(field::WALLET).unwrap_or_default().to_string());
        kept.push(r.text(field::WALLET_NAME).unwrap_or_default().to_string());
    }
    app.press(Action::Vault(V::LoadAll(0)));
    let loaded = app
        .session
        .wallets
        .iter()
        .find_map(|w| w.policy().silent().cloned());
    (kept, loaded)
}

fn keeps_only_the_record(kept: &[String], want: &SilentWallet) {
    assert!(kept.iter().all(|t| !t.contains("spscan")), "{kept:?}");
    assert!(kept.iter().any(|t| *t == want.to_text()), "{kept:?}");
}

#[test]
fn a_record_file_with_crlf_lines_added_to_a_vault_is_there_after_power_off() {
    let c = faraday_core::testkit::compute();
    let mut keyed = with_key();
    keyed.press(Action::Silent);
    keyed.press(Action::SLabel(1));
    let want = SilentWallet::parse(&keyed.silent_record(&c).unwrap()).unwrap();
    let crlf = want.to_text().replace('\n', "\r\n");
    let (kept, loaded) = added_from_a_file_and_reopened("silent-1.txt", crlf.into_bytes());
    keeps_only_the_record(&kept, &want);
    let loaded = loaded.expect("the item loads");
    assert_eq!(loaded.address(), want.address());
    assert_eq!(loaded.labels, 1);
}

#[test]
fn a_record_file_with_a_comment_line_added_to_a_vault_is_there_after_power_off() {
    let c = faraday_core::testkit::compute();
    let mut keyed = with_key();
    keyed.press(Action::Silent);
    keyed.press(Action::SLabel(1));
    let want = SilentWallet::parse(&keyed.silent_record(&c).unwrap()).unwrap();
    let text = format!("# kept for the shop\n{}", want.to_text());
    let (kept, loaded) = added_from_a_file_and_reopened("silent-1.txt", text.into_bytes());
    keeps_only_the_record(&kept, &want);
    let loaded = loaded.expect("the item loads");
    assert_eq!(loaded.address(), want.address());
    assert_eq!(loaded.labels, 1);
}

#[test]
fn an_sp_descriptor_file_added_to_a_vault_keeps_its_record_and_no_scan_key() {
    let c = faraday_core::testkit::compute();
    let sp = include_str!("../vectors/interop/sparrow/keystone-singlesig-sp-keystore-1.txt");
    let want = faraday_core::wallet::read_wallet(&c, sp)
        .unwrap()
        .silent()
        .cloned()
        .unwrap();
    let (kept, loaded) = added_from_a_file_and_reopened("keystone-sp.txt", sp.as_bytes().to_vec());
    keeps_only_the_record(&kept, &want);
    let loaded = loaded.expect("the item loads");
    assert_eq!(loaded.address(), want.address());
    assert_eq!(loaded.labels, want.labels);
}

/// Opens the vault at place `at` of the vault files with `phrase`, from
/// the Vaults page, which loads what it says to load at unlock.
fn open_vault(app: &mut Faraday, at: usize, phrase: &str) {
    use faraday_core::vault::VaultAction as V;
    let open = app.vaults().open.len();
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(at)));
    let _ = app.frame();
    app.press(Action::Vault(V::FocusPassphrase));
    crate::common::type_text(app, phrase);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if app.vaults().open.len() > open {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), open + 1);
    if app.sheet == Some(Sheet::UnlockLoad) {
        app.press(Action::Vault(V::LoadNotNow));
    }
}

/// The labels each silent payments wallet record of open vault `o` states,
/// in the records' order.
fn vault_labels(app: &Faraday, o: usize) -> Vec<u32> {
    use faraday_vault::records::{field, kind};
    let c = faraday_core::testkit::compute();
    app.vaults()
        .store()
        .contents_for_tests(app.vaults().open[o].id)
        .of(kind::WALLET)
        .filter_map(|(_, r)| {
            let w = faraday_core::wallet::read_wallet(&c, r.text(field::WALLET)?).ok()?;
            w.silent().map(|s| s.labels)
        })
        .collect()
}

/// A vault file named `name` holding a wallet record of each of `wallets`.
fn vault_of(name: &str, seed: u8, wallets: &[SilentWallet]) -> Vec<u8> {
    use faraday_vault::Record;
    use faraday_vault::records::{field, kind};
    let records = wallets
        .iter()
        .map(|w| {
            Record::new(kind::WALLET)
                .with(field::WALLET, w.to_text().as_bytes())
                .with(field::WALLET_NAME, b"Silent")
        })
        .collect();
    faraday_core::testkit::kdbx_vault(name, name, seed, records).unwrap()
}

fn with_labels(mut record: SilentWallet, labels: u32) -> SilentWallet {
    record.labels = labels;
    record
}

#[test]
fn labels_handed_out_after_a_save_are_in_the_vault_and_a_payment_to_one_is_found_after_power_off() {
    use faraday_core::StorageCommand;
    use faraday_core::testkit;
    use faraday_core::vault::VaultAction as V;
    let c = testkit::compute();
    let secp = Secp256k1::new();
    let mut app = testkit::started();
    files(&mut app, &[("vault.kdbx", testkit::test_vault().unwrap())]);
    add_key(&mut app);
    let r = Receiver::derive(&c, app.session.keys.master_for_tests(0), 0);
    let (paid, funding_tx) = payment(
        &r.scan_public_key(&secp),
        &r.labelled_spend_key(&secp, 3).unwrap(),
    );
    open_vault(&mut app, 0, testkit::VAULT_PASSPHRASE);
    app.press(Action::Silent);
    app.press(Action::SLabel(1));
    app.press(Action::SAddWallet);
    let w = app
        .session
        .wallets
        .iter()
        .position(|w| w.policy().silent().is_some())
        .unwrap();
    app.press(Action::Vault(V::SaveWallet(w)));
    assert_eq!(vault_labels(&app, 0), [1]);
    app.press(Action::Silent);
    app.press(Action::SLabel(1));
    app.press(Action::SLabel(1));
    assert_eq!(app.session.wallets[w].policy().silent().unwrap().labels, 3);
    crate::common::top_up(&mut app);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    let mut written = Vec::new();
    while let Some(cmd) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { outbox, .. } = cmd {
            written = testkit::plain_files(outbox);
        }
    }
    assert_eq!(written.len(), 1, "the vault file was sealed");

    // Power off: a new process with only the written file.
    written.push(("payment.txn".into(), consensus::serialize(&paid)));
    written.push(("funding.txn".into(), consensus::serialize(&funding_tx)));
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(written),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    open_vault(&mut app, 0, testkit::VAULT_PASSPHRASE);
    assert_eq!(vault_labels(&app, 0), [3]);
    app.press(Action::Vault(V::LoadAll(0)));
    testkit::settle(&mut app);
    let w = app
        .session
        .wallets
        .iter()
        .position(|w| w.policy().silent().is_some())
        .expect("the saved wallet loads");
    assert_eq!(app.session.wallets[w].policy().silent().unwrap().labels, 3);
    app.press(Action::SWallet(w));
    let payment = app
        .inbox()
        .iter()
        .position(|i| i.name == "payment.txn")
        .unwrap();
    app.press(Action::SCheck(payment));
    match checked(&app) {
        Some(Checked::Paid(p)) => assert_eq!(p[0].label, Some(3)),
        other => panic!("{other:?}"),
    }
}

#[test]
fn every_open_vault_holding_the_wallet_follows_its_label_count_and_no_other_changes() {
    let mut app = faraday_core::testkit::started();
    let wallet = record_of(0, 0, None);
    files(
        &mut app,
        &[
            // Its own wallet, and beside it one of another origin over
            // the same address.
            (
                "a.kdbx",
                vault_of(
                    "Alpha",
                    1,
                    &[
                        with_labels(wallet.clone(), 1),
                        under_another_fingerprint(with_labels(wallet.clone(), 1)),
                    ],
                ),
            ),
            ("b.kdbx", vault_of("Beta", 2, std::slice::from_ref(&wallet))),
            (
                "c.kdbx",
                vault_of("Gamma", 3, &[under_another_fingerprint(wallet.clone())]),
            ),
        ],
    );
    add_key(&mut app);
    // Opened from a flow, which loads nothing from them.
    for (at, name) in ["Alpha", "Beta", "Gamma"].iter().enumerate() {
        use faraday_core::vault::VaultAction as V;
        app.press(Action::Vault(V::OpenFrom(at, Screen::Files)));
        crate::common::type_text(&mut app, name);
        app.press(Action::Vault(V::Unlock));
        faraday_core::testkit::settle(&mut app);
        assert_eq!(app.vaults().open.len(), at + 1);
    }
    let at = |app: &Faraday, name: &str| {
        app.vaults()
            .open
            .iter()
            .position(|o| o.name == name)
            .unwrap()
    };
    let (a, b, g) = (at(&app, "a.kdbx"), at(&app, "b.kdbx"), at(&app, "c.kdbx"));
    let changes = |app: &Faraday, o: usize| app.vaults().open[o].changes;
    assert_eq!([a, b, g].map(|o| changes(&app, o)), [0; 3]);
    // The key's page, its wallet not among the session's.
    app.press(Action::Silent);
    assert!(app.session.wallets.is_empty());
    app.press(Action::SLabel(1));
    // Alpha states label 1 already, so the first new one is 2.
    assert_eq!(vault_labels(&app, a), [2, 1]);
    assert_eq!(vault_labels(&app, b), [2]);
    assert_eq!([a, b, g].map(|o| changes(&app, o)), [1, 1, 0]);
    // Stepping back down writes nothing.
    app.press(Action::SLabel(-1));
    assert_eq!([a, b, g].map(|o| changes(&app, o)), [1, 1, 0]);
    // With the wallet among the session's.
    app.press(Action::SAddWallet);
    app.press(Action::SLabel(1));
    app.press(Action::SLabel(1));
    assert_eq!(vault_labels(&app, a), [3, 1]);
    assert_eq!(vault_labels(&app, b), [3]);
    assert_eq!(vault_labels(&app, g), [0]);
    assert_eq!([a, b, g].map(|o| changes(&app, o)), [2, 2, 0]);
}

#[test]
fn loading_a_vaults_wallet_gives_the_loaded_one_its_higher_label_count_and_never_a_lower() {
    use faraday_core::vault::VaultAction as V;
    let mut app = faraday_core::testkit::started();
    let wallet = record_of(0, 0, None);
    files(
        &mut app,
        &[
            (
                "a.kdbx",
                vault_of("Alpha", 1, &[with_labels(wallet.clone(), 3)]),
            ),
            (
                "b.kdbx",
                vault_of("Beta", 2, &[with_labels(wallet.clone(), 2)]),
            ),
        ],
    );
    add_key(&mut app);
    app.press(Action::Silent);
    app.press(Action::SLabel(1));
    app.press(Action::SAddWallet);
    let labels = |app: &Faraday| app.session.wallets[0].policy().silent().unwrap().labels;
    assert_eq!(labels(&app), 1);
    for (o, name) in ["Alpha", "Beta"].iter().enumerate() {
        open_vault(&mut app, o, name);
        app.press(Action::Vault(V::LoadAll(o)));
        faraday_core::testkit::settle(&mut app);
        assert_eq!(app.session.wallets.len(), 1);
        assert_eq!(labels(&app), 3);
    }
}

/// The labels stated by the record the page puts in the Outbox.
fn record_labels(app: &mut Faraday) -> u32 {
    app.outbox.clear();
    app.press(Action::SRecord);
    let text = String::from_utf8(app.outbox[0].bytes.to_vec()).unwrap();
    SilentWallet::parse(&text).unwrap().labels
}

#[test]
fn choosing_another_key_starts_from_that_keys_own_label_count() {
    let c = faraday_core::testkit::compute();
    let mut app = with_key();
    let x = app.session.keys[0].fingerprint.0;
    let y = add_other_key(&mut app);
    // The second key's wallet is added with one label handed out.
    app.press(Action::Silent);
    app.press(Action::SKey(y));
    app.press(Action::SLabel(1));
    app.press(Action::SAddWallet);
    app.press(Action::SAnother);
    app.press(Action::SKey(x));
    for _ in 0..3 {
        app.press(Action::SLabel(1));
    }
    let third = app.silent_address(&c);
    // The same key again changes nothing.
    app.press(Action::SKey(x));
    assert_eq!(app.silent_address(&c), third);
    assert_eq!(record_labels(&mut app), 3);
    app.press(Action::SKey(y));
    assert_eq!(record_labels(&mut app), 1);
    // No wallet is held for the first key: the page still counts what it
    // handed out for it, with no label on show.
    app.press(Action::SKey(x));
    assert_eq!(record_labels(&mut app), 3);
    assert_ne!(app.silent_address(&c), third);
    // So after starting over, and the wallet added states it.
    app.press(Action::SAnother);
    app.press(Action::SKey(x));
    assert_eq!(record_labels(&mut app), 3);
    app.press(Action::SAddWallet);
    assert_eq!(count_of(&app, x), 3);
}

/// Silent payments labels up to 3 handed out with no vault open, the
/// wallet added to the session's.
fn three_labels_and_no_vault(app: &mut Faraday) {
    add_key(app);
    app.press(Action::Silent);
    app.press(Action::SLabel(1));
    app.press(Action::SAddWallet);
    app.press(Action::SLabel(1));
    app.press(Action::SLabel(1));
    assert_eq!(app.session.wallets[0].policy().silent().unwrap().labels, 3);
}

/// Opens vault file `at` as a flow does, which loads nothing from it.
fn open_loading_nothing(app: &mut Faraday, at: usize, phrase: &str) {
    use faraday_core::vault::VaultAction as V;
    let open = app.vaults().open.len();
    app.press(Action::Vault(V::OpenFrom(at, Screen::Files)));
    crate::common::type_text(app, phrase);
    app.press(Action::Vault(V::Unlock));
    faraday_core::testkit::settle(app);
    assert_eq!(app.vaults().open.len(), open + 1);
}

/// Locks, and the files the lock writes.
fn written_at_lock(app: &mut Faraday) -> Vec<(String, Vec<u8>)> {
    crate::common::top_up(app);
    app.press(Action::Lock);
    faraday_core::testkit::settle(app);
    let mut written = Vec::new();
    while let Some(cmd) = app.poll_storage() {
        if let faraday_core::StorageCommand::SaveBoxes { outbox, .. } = cmd {
            written = faraday_core::testkit::plain_files(outbox);
        }
    }
    written
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Way {
    /// The Vaults list's Unlock: a vault of one wallet loads itself.
    Unlock,
    /// Opened, and nothing loaded.
    Opened,
    LoadAll,
    /// Load all of a vault that keeps a seed too, which a job loads.
    Job,
}

#[test]
fn a_vault_opened_after_labels_were_handed_out_follows_the_count() {
    use faraday_core::testkit;
    use faraday_core::vault::VaultAction as V;
    use faraday_vault::Record;
    use faraday_vault::records::{field, kind};
    let c = testkit::compute();
    let secp = Secp256k1::new();
    let wallet = record_of(0, 0, None);
    let r = Receiver::derive(&c, with_key().session.keys.master_for_tests(0), 0);
    let (paid, funding_tx) = payment(
        &r.scan_public_key(&secp),
        &r.labelled_spend_key(&secp, 3).unwrap(),
    );
    for way in [Way::Unlock, Way::Opened, Way::LoadAll, Way::Job] {
        let mut records = vec![
            Record::new(kind::WALLET)
                .with(
                    field::WALLET,
                    with_labels(wallet.clone(), 1).to_text().as_bytes(),
                )
                .with(field::WALLET_NAME, b"Silent"),
        ];
        if way == Way::Job {
            // A published test seed, made into a record.
            #[allow(clippy::disallowed_methods)]
            let words = faraday_bip::bip39::Mnemonic::parse(
                &c,
                faraday_bip::bip39::Language::English,
                &testkit::test_words(testkit::TEST_SEEDS[1].0),
            )
            .unwrap();
            records.push(
                Record::new(kind::KEY)
                    .with(field::KEY, &faraday_vault::records::words_payload(&words)),
            );
        }
        let file = testkit::kdbx_vault("Alpha", "Alpha", 1, records).unwrap();
        let mut app = testkit::started();
        files(&mut app, &[("a.kdbx", file)]);
        three_labels_and_no_vault(&mut app);
        match way {
            Way::Unlock => {
                open_vault(&mut app, 0, "Alpha");
            }
            Way::Opened => open_loading_nothing(&mut app, 0, "Alpha"),
            Way::LoadAll | Way::Job => {
                open_loading_nothing(&mut app, 0, "Alpha");
                app.press(Action::Vault(V::LoadAll(0)));
                testkit::settle(&mut app);
            }
        }
        // The job brought the vault's own seed with it.
        let keys = 1 + usize::from(way == Way::Job);
        assert_eq!(app.session.keys.len(), keys, "{way:?}");
        assert_eq!(vault_labels(&app, 0), [3], "{way:?}");
        assert_eq!(app.vaults().open[0].changes, 1, "{way:?}");
        let mut written = written_at_lock(&mut app);
        assert_eq!(written.len(), 1, "the vault file was sealed: {way:?}");

        // Power off: a new process with only the written file.
        written.push(("payment.txn".into(), consensus::serialize(&paid)));
        written.push(("funding.txn".into(), consensus::serialize(&funding_tx)));
        let mut app = testkit::started();
        app.storage(StorageEvent::Restored {
            inbox: faraday_core::files_from(written),
            outbox: Vec::new(),
            kept: Vec::new(),
        });
        add_key(&mut app);
        open_loading_nothing(&mut app, 0, "Alpha");
        assert_eq!(vault_labels(&app, 0), [3], "{way:?}");
        app.press(Action::Vault(V::LoadAll(0)));
        testkit::settle(&mut app);
        let w = app
            .session
            .wallets
            .iter()
            .position(|w| w.policy().silent().is_some())
            .expect("the saved wallet loads");
        assert_eq!(app.session.wallets[w].policy().silent().unwrap().labels, 3);
        app.press(Action::SWallet(w));
        let payment = app
            .inbox()
            .iter()
            .position(|i| i.name == "payment.txn")
            .unwrap();
        app.press(Action::SCheck(payment));
        match checked(&app) {
            Some(Checked::Paid(p)) => assert_eq!(p[0].label, Some(3), "{way:?}"),
            other => panic!("{way:?}: {other:?}"),
        }
    }
}

/// As the device runs it, an unlock's answer comes from the worker on a
/// later tick: the vault follows then, with no press after it.
#[test]
fn a_vault_unlocked_on_the_worker_follows_the_count_with_no_press_after_it() {
    use faraday_core::testkit;
    use faraday_core::vault::VaultAction as V;
    let file = vault_of("Alpha", 1, &[with_labels(record_of(0, 0, None), 1)]);
    let mut app = testkit::started();
    files(&mut app, &[("a.kdbx", file)]);
    three_labels_and_no_vault(&mut app);
    app.press(Action::Vault(V::OpenFrom(0, Screen::Files)));
    crate::common::type_text(&mut app, "Alpha");
    testkit::press_on_worker(&mut app, Action::Vault(V::Unlock));
    assert!(app.vaults().open.is_empty(), "the unlock waits");
    for t in 1..6000u64 {
        if !app.vaults().open.is_empty() && app.busy().is_none() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(vault_labels(&app, 0), [3]);
    assert_eq!(app.vaults().open[0].changes, 1);
}

/// The count comes to the session from a second vault's load, a press with
/// no job: the vault loaded before it follows.
#[test]
fn a_higher_count_loaded_from_a_later_vault_reaches_the_vault_open_before() {
    use faraday_core::vault::VaultAction as V;
    let mut app = faraday_core::testkit::started();
    let wallet = record_of(0, 0, None);
    files(
        &mut app,
        &[
            (
                "a.kdbx",
                vault_of("Alpha", 1, &[with_labels(wallet.clone(), 1)]),
            ),
            (
                "b.kdbx",
                vault_of("Beta", 2, &[with_labels(wallet.clone(), 3)]),
            ),
        ],
    );
    add_key(&mut app);
    // Both open, the session holding neither's wallet: nothing differs from
    // it and neither is written.
    open_loading_nothing(&mut app, 0, "Alpha");
    open_loading_nothing(&mut app, 1, "Beta");
    let at = |app: &Faraday, name: &str| {
        app.vaults()
            .open
            .iter()
            .position(|o| o.name == name)
            .unwrap()
    };
    let (a, b) = (at(&app, "a.kdbx"), at(&app, "b.kdbx"));
    assert_eq!([a, b].map(|o| app.vaults().open[o].changes), [0, 0]);
    app.press(Action::Vault(V::LoadAll(a)));
    assert_eq!([a, b].map(|o| app.vaults().open[o].changes), [0, 0]);
    app.press(Action::Vault(V::LoadAll(b)));
    assert_eq!(app.session.wallets[0].policy().silent().unwrap().labels, 3);
    assert_eq!(vault_labels(&app, a), [3]);
    assert_eq!([a, b].map(|o| app.vaults().open[o].changes), [1, 0]);
}

#[test]
fn a_vault_whose_counts_agree_or_are_higher_is_not_written_when_opened_and_loaded() {
    use faraday_core::vault::VaultAction as V;
    let mut app = faraday_core::testkit::started();
    let wallet = record_of(0, 0, None);
    files(
        &mut app,
        &[
            (
                "a.kdbx",
                vault_of(
                    "Alpha",
                    1,
                    &[
                        with_labels(wallet.clone(), 3),
                        under_another_fingerprint(with_labels(wallet.clone(), 1)),
                    ],
                ),
            ),
            (
                "b.kdbx",
                vault_of("Beta", 2, &[with_labels(wallet.clone(), 4)]),
            ),
        ],
    );
    three_labels_and_no_vault(&mut app);
    open_vault(&mut app, 0, "Alpha");
    app.press(Action::Vault(V::LoadAll(0)));
    assert_eq!(app.vaults().open[0].changes, 0);
    assert_eq!(vault_labels(&app, 0), [3, 1]);
    open_loading_nothing(&mut app, 1, "Beta");
    assert!(app.vaults().open.iter().all(|o| o.changes == 0));
    assert_eq!(written_at_lock(&mut app), []);
}

#[test]
fn a_read_only_vault_reads_no_wallet_record_and_is_not_written_after_labels_were_handed_out() {
    use faraday_core::testkit;
    use faraday_core::vault::VaultAction as V;
    use faraday_vault::kdbx::items::{Body, Place, put};
    use faraday_vault::records::{field, kind};
    const NO_BIN: &str = "<RecycleBinUUID>AAAAAAAAAAAAAAAAAAAAAA==</RecycleBinUUID>";
    let c = testkit::compute();
    let wallet = with_labels(record_of(0, 0, None), 1);
    // Two recycle bins: a file that is opened to read only.
    let file = testkit::kdbx_edited(
        "Odd",
        "Odd",
        10,
        |doc| {
            let record = faraday_vault::Record::new(kind::WALLET)
                .with(field::WALLET, wallet.to_text().as_bytes())
                .with(field::WALLET_NAME, b"Silent");
            put(
                &c,
                doc,
                &Place {
                    uuid: [7; 16],
                    group: "Wallets",
                    group_uuid: [8; 16],
                    title: "Silent",
                    now: None,
                },
                &Body::Record(record),
            )
            .unwrap();
        },
        |xml| xml.replace(NO_BIN, &format!("{NO_BIN}{NO_BIN}")),
    )
    .unwrap();
    let mut app = testkit::started();
    files(&mut app, &[("ro.kdbx", file)]);
    three_labels_and_no_vault(&mut app);
    open_loading_nothing(&mut app, 0, "Odd");
    app.press(Action::Vault(V::LoadAll(0)));
    testkit::settle(&mut app);
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(vault_labels(&app, 0), [0u32; 0]);
    assert_eq!(app.vaults().open[0].changes, 0);
}

/// The labels of the session's account 0 silent payments wallet of key
/// `fp`, the one the key gives.
fn count_of(app: &Faraday, fp: [u8; 4]) -> u32 {
    silent_records(app)
        .iter()
        .filter(|w| w.fingerprint.0 == fp && w.account == 0)
        .filter(|w| w.scan != another_scan_key())
        .map(|w| w.labels)
        .next()
        .expect("the key's wallet is held")
}

/// Home, which forgets the page, and the Silent payments page again.
fn home_and_back(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Home));
    assert!(app.silent().is_none());
    app.press(Action::Silent);
}

#[test]
fn labels_pressed_after_choosing_a_key_whose_wallet_is_held_count_for_that_wallet() {
    let c = faraday_core::testkit::compute();
    let secp = Secp256k1::new();
    let mut app = with_key();
    let x = app.session.keys[0].fingerprint.0;
    add_other_key(&mut app);
    let r = Receiver::derive(&c, app.session.keys.master_for_tests(0), 0);
    let (paid, funding_tx) = payment(
        &r.scan_public_key(&secp),
        &r.labelled_spend_key(&secp, 2).unwrap(),
    );
    files(
        &mut app,
        &[
            ("payment.txn", consensus::serialize(&paid)),
            ("funding.txn", consensus::serialize(&funding_tx)),
        ],
    );
    app.press(Action::Silent);
    app.press(Action::SKey(x));
    app.press(Action::SLabel(1));
    app.press(Action::SAddWallet);
    home_and_back(&mut app);
    app.press(Action::SKey(x));
    app.press(Action::SLabel(1));
    assert_eq!(count_of(&app, x), 2);
    app.press(Action::Nav(Screen::Home));
    app.press(Action::SWallet(0));
    app.press(Action::SCheck(0));
    match checked(&app) {
        Some(Checked::Paid(p)) => assert_eq!(p[0].label, Some(2)),
        other => panic!("{other:?}"),
    }
}

#[test]
fn another_accounts_count_never_reaches_the_keys_page() {
    // Account 1 alone: the key's wallet, added from its page, has none.
    let mut app = faraday_core::testkit::started();
    load_record(&mut app, &with_labels(record_of(1, 1, None), 5));
    add_key(&mut app);
    let x = app.session.keys[0].fingerprint.0;
    // One key: the page chooses it as it opens.
    app.press(Action::Silent);
    assert_eq!(record_labels(&mut app), 0);
    app.press(Action::SAddWallet);
    assert_eq!(count_of(&app, x), 0);
    add_other_key(&mut app);
    home_and_back(&mut app);
    app.press(Action::SKey(x));
    assert_eq!(record_labels(&mut app), 0);

    // Account 1 first: the account 0 count is still found.
    let mut app = faraday_core::testkit::started();
    load_record(&mut app, &record_of(1, 1, None));
    load_record(&mut app, &with_labels(record_of(0, 0, None), 3));
    add_key(&mut app);
    app.press(Action::Silent);
    assert_eq!(record_labels(&mut app), 3);
    add_other_key(&mut app);
    home_and_back(&mut app);
    app.press(Action::SKey(x));
    assert_eq!(record_labels(&mut app), 3);
}

#[test]
fn a_record_that_does_not_match_its_key_gives_the_keys_page_no_count_either_way() {
    // The record that does not match states 20 and is alone, or states
    // none and comes before the key's own at 3.
    for (forged, own) in [(20, None), (0, Some(3))] {
        for second in [false, true] {
            let bad = with_labels(record_of(0, 0, Some(another_scan_key())), forged);
            let mut app = faraday_core::testkit::started();
            load_record(&mut app, &bad);
            add_key(&mut app);
            if let Some(n) = own {
                load_record(&mut app, &with_labels(record_of(0, 0, None), n));
            }
            let x = app.session.keys[0].fingerprint.0;
            let want = own.unwrap_or(0);
            if second {
                add_other_key(&mut app);
            }
            // With one key the page chooses it as it opens.
            app.press(Action::Silent);
            if second {
                app.press(Action::SKey(x));
            }
            assert_eq!(record_labels(&mut app), want, "{forged} {second}");
            app.press(Action::SLabel(1));
            assert_eq!(record_labels(&mut app), want + 1, "{forged} {second}");
        }
    }
}

#[test]
fn a_keys_page_starts_from_the_highest_count_of_the_session_and_every_open_vault() {
    let wallet = record_of(0, 0, None);
    let two_vaults = |app: &mut Faraday| {
        files(
            app,
            &[
                (
                    "a.kdbx",
                    vault_of("Alpha", 1, &[with_labels(wallet.clone(), 3)]),
                ),
                (
                    "b.kdbx",
                    vault_of(
                        "Beta",
                        2,
                        &[
                            with_labels(wallet.clone(), 1),
                            under_another_fingerprint(with_labels(wallet.clone(), 9)),
                        ],
                    ),
                ),
            ],
        );
        open_loading_nothing(app, 0, "Alpha");
        open_loading_nothing(app, 1, "Beta");
    };
    // As the page is entered, one key chosen.
    let mut app = faraday_core::testkit::started();
    two_vaults(&mut app);
    add_key(&mut app);
    let x = app.session.keys[0].fingerprint.0;
    app.press(Action::Silent);
    assert_eq!(record_labels(&mut app), 3);
    // Start over, before the page has handed any label out.
    app.press(Action::SAnother);
    assert_eq!(record_labels(&mut app), 3);
    app.press(Action::SLabel(1));
    assert_eq!(app.silent().unwrap().label, 4);
    app.press(Action::SAddWallet);
    assert_eq!(count_of(&app, x), 4);

    // As the key is pressed.
    let mut app = faraday_core::testkit::started();
    two_vaults(&mut app);
    add_key(&mut app);
    add_other_key(&mut app);
    app.press(Action::Silent);
    app.press(Action::SKey(x));
    assert_eq!(record_labels(&mut app), 3);

    // On the page of a session wallet that states fewer.
    let mut app = faraday_core::testkit::started();
    files(
        &mut app,
        &[(
            "a.kdbx",
            vault_of("Alpha", 1, &[with_labels(wallet.clone(), 3)]),
        )],
    );
    add_key(&mut app);
    app.press(Action::Silent);
    app.press(Action::SLabel(1));
    app.press(Action::SAddWallet);
    app.press(Action::Nav(Screen::Home));
    open_loading_nothing(&mut app, 0, "Alpha");
    assert!(app.vaults().open.iter().all(|o| o.changes == 0));
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Silent);
    assert_eq!(record_labels(&mut app), 3);
    assert_eq!(count_of(&app, x), 1, "entering raises no wallet");
    app.press(Action::SAddWallet);
    assert_eq!(count_of(&app, x), 3);
}

/// From its card, with no key's page entered: the key's own wallet, and
/// one of account 1 with no key loaded.
#[test]
fn a_wallets_page_opened_from_its_card_starts_from_an_open_vaults_higher_count() {
    for (account, keyed) in [(0, true), (1, false)] {
        let wallet = record_of(account, account, None);
        let mut app = faraday_core::testkit::started();
        if keyed {
            add_key(&mut app);
        }
        load_record(&mut app, &with_labels(wallet.clone(), 1));
        files(
            &mut app,
            &[("a.kdbx", vault_of("Alpha", 1, &[with_labels(wallet, 3)]))],
        );
        open_loading_nothing(&mut app, 0, "Alpha");
        app.press(Action::Nav(Screen::Home));
        app.press(Action::SWallet(0));
        assert_eq!(record_labels(&mut app), 3, "{account} {keyed}");
        assert_eq!(
            silent_records(&app)[0].labels,
            1,
            "opening raises no wallet"
        );
    }
}

#[test]
fn a_code_found_by_the_search_opens_with_an_open_vaults_count() {
    use faraday_core::grind::Dial;
    use faraday_core::vanity::VanityAction as V;
    let found = |app: &mut Faraday| {
        crate::vanity::silent_search(app, Dial::Passphrase);
        app.press(Action::Vanity(V::Use));
        assert_eq!(app.screen, Screen::Silent);
    };
    // The search finds the same passphrase each time: its record first.
    let mut app = crate::vanity::with_key();
    found(&mut app);
    app.outbox.clear();
    app.press(Action::SRecord);
    let text = String::from_utf8(app.outbox[0].bytes.to_vec()).unwrap();
    let record = SilentWallet::parse(&text).unwrap();

    let mut app = crate::vanity::with_key();
    files(
        &mut app,
        &[("a.kdbx", vault_of("Alpha", 1, &[with_labels(record, 3)]))],
    );
    open_loading_nothing(&mut app, 0, "Alpha");
    app.press(Action::Vanity(V::Open));
    found(&mut app);
    assert_eq!(record_labels(&mut app), 3);
}

#[test]
fn an_older_record_added_to_an_open_vault_follows_the_count() {
    use faraday_core::vault::VaultAction as V;
    let mut app = faraday_core::testkit::started();
    let older = with_labels(record_of(0, 0, None), 1);
    files(
        &mut app,
        &[
            ("a.kdbx", vault_of("Alpha", 1, &[])),
            ("silent-1.txt", older.to_text().into_bytes()),
        ],
    );
    three_labels_and_no_vault(&mut app);
    open_loading_nothing(&mut app, 0, "Alpha");
    let at = app
        .inbox()
        .iter()
        .position(|i| i.name == "silent-1.txt")
        .unwrap();
    app.press(Action::Vault(V::AddFile(at)));
    assert_eq!(vault_labels(&app, 0), [3]);
}

/// The record file `record` loaded from Files over the wallet already
/// held: the held record takes its higher count and nothing else.
#[test]
fn a_record_file_of_another_test_network_raises_the_count_and_keeps_the_held_network() {
    use faraday_bip::keys::Network;
    let held = with_labels(record_of(0, 0, None), 1);
    let mut other = with_labels(held.clone(), 5);
    other.network = if held.network == Network::Regtest {
        Network::Signet
    } else {
        Network::Regtest
    };
    let mut app = faraday_core::testkit::started();
    load_record(&mut app, &held);
    load_record(&mut app, &other);
    let want = with_labels(held, 5);
    assert_eq!(silent_records(&app), [want]);
}

fn no_memory() -> Option<u32> {
    Some(0)
}

fn enough_memory() -> Option<u32> {
    Some(4096)
}

#[test]
fn a_vault_that_refuses_a_count_is_told_of_once_and_written_once_there_is_room() {
    let mut app = faraday_core::testkit::started();
    files(
        &mut app,
        &[("a.kdbx", vault_of("Alpha", 1, &[record_of(0, 0, None)]))],
    );
    add_key(&mut app);
    open_loading_nothing(&mut app, 0, "Alpha");
    // A limit a little over what the vault takes: each write has its size
    // counted, which takes memory, and with memory for that it fits.
    let c = faraday_core::testkit::compute();
    let id = app.vaults().open[0].id;
    faraday_core::testkit::vault_edit(&mut app, id, |s| s.lower_limit(1));
    let used = app.vaults().records(id).counted(&c).unwrap();
    faraday_core::testkit::vault_edit(&mut app, id, |s| s.lower_limit(used + 2048));
    app.set_memory_reader(no_memory);
    app.press(Action::Silent);
    app.press(Action::SLabel(1));
    let told = |app: &Faraday| {
        app.toast_text()
            .is_some_and(|t| t.starts_with("Label count not saved in a.kdbx: Needs "))
    };
    assert!(told(&app), "{:?}", app.toast_text());
    assert_eq!(vault_labels(&app, 0), [0]);
    // The same count refused again is not told of again.
    app.press(Action::SRecord);
    assert!(!told(&app), "{:?}", app.toast_text());
    app.press(Action::SLabel(-1));
    app.press(Action::SLabel(-1));
    assert!(!told(&app), "{:?}", app.toast_text());
    // Another count is.
    app.press(Action::SLabel(1));
    assert!(told(&app), "{:?}", app.toast_text());
    assert_eq!(vault_labels(&app, 0), [0]);
    // Memory back, with nothing else changed: the next press writes it.
    app.set_memory_reader(enough_memory);
    app.press(Action::SRecord);
    assert_eq!(vault_labels(&app, 0), [2]);
    let written = written_at_lock(&mut app);
    let written: Vec<_> = written
        .iter()
        .map(|(n, b)| (n.as_str(), b.clone()))
        .collect();
    let mut app = faraday_core::testkit::started();
    files(&mut app, &written);
    open_loading_nothing(&mut app, 0, "Alpha");
    assert_eq!(vault_labels(&app, 0), [2]);
}

#[test]
fn two_vaults_that_refuse_a_count_at_once_are_both_named_with_why() {
    let mut app = faraday_core::testkit::started();
    let wallet = record_of(0, 0, None);
    files(
        &mut app,
        &[
            (
                "a.kdbx",
                vault_of("Alpha", 1, std::slice::from_ref(&wallet)),
            ),
            ("b.kdbx", vault_of("Beta", 2, &[wallet])),
        ],
    );
    add_key(&mut app);
    open_loading_nothing(&mut app, 0, "Alpha");
    open_loading_nothing(&mut app, 1, "Beta");
    // No vault holds more than it does now.
    let id = app.vaults().open[0].id;
    faraday_core::testkit::vault_edit(&mut app, id, |s| s.lower_limit(1));
    app.press(Action::Silent);
    app.press(Action::SLabel(1));
    assert_eq!(
        app.toast_text(),
        Some(
            "Label count not saved in a.kdbx: More than a vault can hold \
             · Label count not saved in b.kdbx: More than a vault can hold"
        )
    );
    assert_eq!(vault_labels(&app, 0), [0]);
    assert_eq!(vault_labels(&app, 1), [0]);
}

/// No open vault holds more than it does now.
fn fill_every_vault(app: &mut Faraday) {
    let id = app.vaults().open[0].id;
    faraday_core::testkit::vault_edit(app, id, |s| s.lower_limit(1));
}

/// A record of open vault `o` is renamed: its records change, and the
/// count it states does not.
fn rename_a_wallet_record(app: &mut Faraday, o: usize) {
    use faraday_vault::records::field;
    let c = faraday_core::testkit::compute();
    let id = app.vaults().open[o].id;
    faraday_core::testkit::vault_edit(app, id, |s| {
        s.set_publics_in(&c, id, 0, &[(field::WALLET_NAME, b"Renamed")])
    })
    .unwrap();
}

const A_IS_FULL: &str = "Label count not saved in a.kdbx: More than a vault can hold";

#[test]
fn a_vault_told_of_before_is_left_out_when_another_refuses() {
    let c = faraday_core::testkit::compute();
    let mut app = faraday_core::testkit::started();
    let other = record_of(1, 1, None);
    files(
        &mut app,
        &[
            ("a.kdbx", vault_of("Alpha", 1, &[record_of(0, 0, None)])),
            ("c.kdbx", vault_of("Gamma", 3, std::slice::from_ref(&other))),
        ],
    );
    add_key(&mut app);
    open_loading_nothing(&mut app, 0, "Alpha");
    open_loading_nothing(&mut app, 1, "Gamma");
    fill_every_vault(&mut app);
    app.press(Action::Silent);
    app.press(Action::SLabel(1));
    assert_eq!(app.toast_text(), Some(A_IS_FULL));
    // The first vault's records change, so it is tried again, in the same
    // run as the second is first tried.
    let id = app.vaults().open[0].id;
    faraday_core::testkit::vault_edit(&mut app, id, |s| s.lower_limit(usize::MAX));
    rename_a_wallet_record(&mut app, 0);
    fill_every_vault(&mut app);
    app.session
        .add_wallet_confirmed(
            &c,
            "Silent",
            &with_labels(other, 3).to_text(),
            "a test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    files(&mut app, &[]);
    assert_eq!(
        app.toast_text(),
        Some("Label count not saved in c.kdbx: More than a vault can hold")
    );
}

#[test]
fn a_vault_that_refuses_two_wallets_is_one_line() {
    let c = faraday_core::testkit::compute();
    let mut app = faraday_core::testkit::started();
    let wallets = [record_of(0, 0, None), record_of(1, 1, None)];
    files(&mut app, &[("a.kdbx", vault_of("Alpha", 1, &wallets))]);
    add_key(&mut app);
    open_loading_nothing(&mut app, 0, "Alpha");
    fill_every_vault(&mut app);
    for wallet in wallets {
        app.session
            .add_wallet_confirmed(
                &c,
                "Silent",
                &with_labels(wallet, 3).to_text(),
                "a test",
                faraday_core::wallet::Origin::Named,
            )
            .unwrap();
    }
    files(&mut app, &[]);
    assert_eq!(app.toast_text(), Some(A_IS_FULL));
    assert_eq!(vault_labels(&app, 0), [0, 0]);
}

#[test]
fn two_vaults_of_one_name_that_refuse_a_count_are_two_lines() {
    let mut app = faraday_core::testkit::started();
    let wallet = record_of(0, 0, None);
    let holding = |name, seed| vault_of(name, seed, std::slice::from_ref(&wallet));
    files(&mut app, &[("a.kdbx", holding("Alpha", 1))]);
    add_key(&mut app);
    open_loading_nothing(&mut app, 0, "Alpha");
    files(&mut app, &[("a.kdbx", holding("Beta", 2))]);
    open_loading_nothing(&mut app, 0, "Beta");
    assert_eq!(app.vaults().open.len(), 2);
    fill_every_vault(&mut app);
    app.press(Action::Silent);
    app.press(Action::SLabel(1));
    let both = format!("{A_IS_FULL} · {A_IS_FULL}");
    assert_eq!(app.toast_text(), Some(&*both));
}

#[test]
fn a_full_vault_is_not_tried_again_until_its_records_change() {
    let mut app = faraday_core::testkit::started();
    files(
        &mut app,
        &[("a.kdbx", vault_of("Alpha", 1, &[record_of(0, 0, None)]))],
    );
    add_key(&mut app);
    open_loading_nothing(&mut app, 0, "Alpha");
    fill_every_vault(&mut app);
    app.press(Action::Silent);
    app.press(Action::SLabel(1));
    assert_eq!(app.toast_text(), Some(A_IS_FULL));
    // Room with no change of its records: a try would write the count.
    let id = app.vaults().open[0].id;
    faraday_core::testkit::vault_edit(&mut app, id, |s| s.lower_limit(usize::MAX));
    app.press(Action::SRecord);
    assert_eq!(vault_labels(&app, 0), [0]);
    // Nor for a higher count.
    app.press(Action::SLabel(1));
    assert_eq!(vault_labels(&app, 0), [0]);
    rename_a_wallet_record(&mut app, 0);
    app.press(Action::SRecord);
    assert_eq!(vault_labels(&app, 0), [2]);
}

/// A seal of open vault `id` is under way: the vault takes no edit until
/// it ends.
fn begin_seal(app: &mut Faraday, id: faraday_keys::vault::VaultId) {
    let job = faraday_core::testkit::vault_edit(app, id, |s| {
        let uuids = vec![7; s.uuid_bytes(id).unwrap()];
        s.seal_job(id, &[7; faraday_keys::vault::SEAL_RANDOM_LEN], &uuids, None)
    });
    assert!(job.is_some());
}

#[test]
fn a_vault_being_sealed_is_not_said_to_be_full_and_gets_the_count_after() {
    let mut app = faraday_core::testkit::started();
    files(
        &mut app,
        &[("a.kdbx", vault_of("Alpha", 1, &[record_of(0, 0, None)]))],
    );
    add_key(&mut app);
    open_loading_nothing(&mut app, 0, "Alpha");
    let id = app.vaults().open[0].id;
    begin_seal(&mut app, id);
    app.press(Action::Silent);
    app.press(Action::SLabel(1));
    let toast = app.toast_text().unwrap_or("");
    assert!(!toast.starts_with("Label count not saved"), "{toast}");
    assert_eq!(vault_labels(&app, 0), [0]);
    // The seal fails, with nothing else changed: the next run writes it.
    faraday_core::testkit::vault_edit_unseen(&mut app, id, |s| s.seal_ended(id, false));
    app.press(Action::SRecord);
    assert_eq!(vault_labels(&app, 0), [1]);
}

#[test]
fn a_count_that_waits_is_tried_again_when_a_job_answers() {
    use faraday_core::testkit;
    use faraday_core::vault::VaultAction as V;
    let mut app = testkit::started();
    files(
        &mut app,
        &[
            ("a.kdbx", vault_of("Alpha", 1, &[record_of(0, 0, None)])),
            ("b.kdbx", vault_of("Beta", 2, &[])),
        ],
    );
    add_key(&mut app);
    open_loading_nothing(&mut app, 0, "Alpha");
    let id = app.vaults().open[0].id;
    begin_seal(&mut app, id);
    app.press(Action::Silent);
    app.press(Action::SLabel(1));
    assert_eq!(vault_labels(&app, 0), [0]);
    app.press(Action::Vault(V::OpenFrom(1, Screen::Files)));
    // A wrong passphrase: the job answers and no vault opens, so nothing
    // but the wait brings the write.
    crate::common::type_text(&mut app, "not Beta's");
    testkit::press_on_worker(&mut app, Action::Vault(V::Unlock));
    assert!(app.busy().is_some(), "the unlock is on the worker");
    assert_eq!(vault_labels(&app, 0), [0]);
    // The seal fails, with no press after it.
    testkit::vault_edit_unseen(&mut app, id, |s| s.seal_ended(id, false));
    assert_eq!(vault_labels(&app, 0), [0]);
    for t in 1..6000u64 {
        if app.busy().is_none() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(app.busy().is_none());
    assert_eq!(app.vaults().open.len(), 1);
    assert_eq!(vault_labels(&app, 0), [1]);
}

#[test]
fn plus_after_a_vault_is_unlocked_from_the_page_goes_past_its_labels() {
    let c = faraday_core::testkit::compute();
    let mut app = faraday_core::testkit::started();
    let held = with_labels(record_of(0, 0, None), 5);
    files(&mut app, &[("a.kdbx", vault_of("Alpha", 1, &[held]))]);
    add_key(&mut app);
    app.press(Action::Silent);
    let fp = app.session.keys.all().next().unwrap().fingerprint.0;
    app.press(Action::SKey(fp));
    unlocked_from_the_page(&mut app);
    app.press(Action::SLabel(1));
    let page = SilentWallet::parse(&app.silent_record(&c).unwrap()).unwrap();
    assert_eq!(page.labels, 6);
    assert_eq!(vault_labels(&app, 0), [6]);
}

/// The vault named Alpha unlocked from the Silent payments page, the way
/// the Scan key card offers with no vault open.
fn unlocked_from_the_page(app: &mut Faraday) {
    use faraday_core::vault::VaultAction as V;
    app.press(Action::Vault(V::OpenFrom(0, Screen::Silent)));
    crate::common::type_text(app, "Alpha");
    app.press(Action::Vault(V::Unlock));
    faraday_core::testkit::settle(app);
    assert_eq!(app.screen, Screen::Silent);
}

/// The first test key, and a closed vault holding its wallet at five
/// labels.
fn five_labels_in_a_closed_vault(app: &mut Faraday) {
    let held = with_labels(record_of(0, 0, None), 5);
    files(app, &[("a.kdbx", vault_of("Alpha", 1, &[held]))]);
    add_key(app);
}

#[test]
fn a_wallet_added_after_an_unlock_from_the_page_has_the_vaults_count() {
    let mut app = faraday_core::testkit::started();
    five_labels_in_a_closed_vault(&mut app);
    app.press(Action::Silent);
    let fp = app.session.keys.all().next().unwrap().fingerprint.0;
    app.press(Action::SKey(fp));
    unlocked_from_the_page(&mut app);
    app.press(Action::SAddWallet);
    let labels: Vec<u32> = app
        .session
        .wallets
        .iter()
        .filter_map(|w| w.policy().silent().map(|r| r.labels))
        .collect();
    assert_eq!(labels, [5]);
}

#[test]
fn plus_on_a_wallets_page_after_an_unlock_from_it_goes_past_the_vaults_labels() {
    let c = faraday_core::testkit::compute();
    let mut app = faraday_core::testkit::started();
    five_labels_in_a_closed_vault(&mut app);
    app.session
        .add_wallet_confirmed(
            &c,
            "Silent",
            &record_of(0, 0, None).to_text(),
            "a test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    app.press(Action::SWallet(app.session.wallets.len() - 1));
    unlocked_from_the_page(&mut app);
    let record = SilentWallet::parse(&app.silent_record(&c).unwrap()).unwrap();
    assert_eq!(record.labels, 5);
    app.press(Action::SLabel(1));
    let page = SilentWallet::parse(&app.silent_record(&c).unwrap()).unwrap();
    assert_eq!(page.labels, 6);
    assert_eq!(vault_labels(&app, 0), [6]);
}

#[test]
fn a_storage_event_has_an_open_vault_follow_the_count() {
    let c = faraday_core::testkit::compute();
    let mut app = faraday_core::testkit::started();
    let wallet = record_of(0, 0, None);
    files(
        &mut app,
        &[(
            "a.kdbx",
            vault_of("Alpha", 1, &[with_labels(wallet.clone(), 1)]),
        )],
    );
    open_loading_nothing(&mut app, 0, "Alpha");
    // The wallet comes in with no press and no job.
    app.session
        .add_wallet_confirmed(
            &c,
            "Silent",
            &with_labels(wallet, 3).to_text(),
            "a test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    assert_eq!(vault_labels(&app, 0), [1]);
    files(&mut app, &[]);
    assert_eq!(vault_labels(&app, 0), [3]);
}

/// `n` presses of the label's "+".
fn labels_up(app: &mut Faraday, n: u32) {
    for _ in 0..n {
        app.press(Action::SLabel(1));
    }
}

/// Three labels on the page of the first test key, never added as a
/// wallet, then Home.
fn three_labels_never_added(app: &mut Faraday) {
    add_key(app);
    app.press(Action::Silent);
    labels_up(app, 3);
    app.press(Action::Nav(Screen::Home));
}

fn remove_every_key(app: &mut Faraday) {
    app.press(Action::RemoveKeys);
    app.press(Action::RemoveKeysHold);
    assert!(app.session.keys.is_empty());
}

#[test]
fn the_labels_of_a_page_never_added_are_kept_when_it_is_left() {
    let c = faraday_core::testkit::compute();
    let mut app = with_key();
    app.press(Action::Silent);
    labels_up(&mut app, 3);
    let third = app.silent_address(&c);
    app.press(Action::SQr(false));
    assert!(app.qr().is_some());
    app.press(Action::Cancel);
    home_and_back(&mut app);
    assert_eq!(record_labels(&mut app), 3);
    // No label is on show; "+" is the first not yet handed out, and "-"
    // steps back down to the third, the same address.
    assert_eq!(app.silent().unwrap().label, 0);
    labels_up(&mut app, 1);
    assert_eq!(app.silent().unwrap().label, 4);
    assert_eq!(record_labels(&mut app), 4);
    app.press(Action::SLabel(-1));
    assert_eq!(app.silent_address(&c), third);
    for want in [2, 1, 0, 0] {
        app.press(Action::SLabel(-1));
        assert_eq!(app.silent().unwrap().label, want);
    }
    assert_eq!(record_labels(&mut app), 4);
    app.press(Action::SAnother);
    assert_eq!(record_labels(&mut app), 4);
    assert!(app.session.wallets.is_empty());
}

/// At the most labels there are, "+" from none shows the last.
#[test]
fn plus_from_no_label_with_every_label_handed_out_shows_the_last() {
    use faraday_bip::silent_wallet::MAX_LABELS;
    let mut app = faraday_core::testkit::started();
    load_record(&mut app, &with_labels(record_of(0, 0, None), MAX_LABELS));
    add_key(&mut app);
    app.press(Action::SWallet(0));
    labels_up(&mut app, 1);
    assert_eq!(app.silent().unwrap().label, MAX_LABELS);
}

#[test]
fn the_labels_handed_out_outlast_their_key() {
    let mut app = faraday_core::testkit::started();
    add_key(&mut app);
    remove_every_key(&mut app);
    assert!(faraday_core::testkit::holds_nothing(&app));
    three_labels_never_added(&mut app);
    remove_every_key(&mut app);
    // The count is something a lock wipes, with nothing else left.
    assert!(!faraday_core::testkit::holds_nothing(&app));
    add_key(&mut app);
    app.press(Action::Silent);
    assert_eq!(record_labels(&mut app), 3);
}

#[test]
fn two_keys_and_two_accounts_keep_their_own_counts() {
    let mut app = faraday_core::testkit::started();
    add_key(&mut app);
    let x = app.session.keys[0].fingerprint.0;
    let y = add_other_key(&mut app);
    app.press(Action::Silent);
    app.press(Action::SKey(x));
    labels_up(&mut app, 3);
    app.press(Action::SKey(y));
    labels_up(&mut app, 1);
    // Account 1 of the first key, on the page of its record's card.
    app.press(Action::Nav(Screen::Home));
    load_record(&mut app, &record_of(1, 1, None));
    app.press(Action::SWallet(0));
    labels_up(&mut app, 5);
    app.press(Action::Nav(Screen::Home));
    app.press(Action::RemoveWallet(0));
    assert!(app.session.wallets.is_empty());
    app.press(Action::Silent);
    app.press(Action::SKey(y));
    assert_eq!(record_labels(&mut app), 1);
    app.press(Action::SKey(x));
    assert_eq!(record_labels(&mut app), 3);
    // Each account's record takes its own count as it is loaded.
    app.press(Action::Nav(Screen::Home));
    load_record(&mut app, &record_of(1, 1, None));
    load_record(&mut app, &record_of(0, 0, None));
    let counts: Vec<_> = silent_records(&app)
        .iter()
        .map(|w| (w.account, w.labels))
        .collect();
    assert_eq!(counts, [(1, 5), (0, 3)]);
    // A record that only names the fingerprint, with its key gone, takes
    // none.
    let mut app = faraday_core::testkit::started();
    three_labels_never_added(&mut app);
    remove_every_key(&mut app);
    load_record(&mut app, &under_another_fingerprint(record_of(0, 0, None)));
    assert_eq!(silent_records(&app)[0].labels, 0);
}

#[test]
fn a_record_file_loaded_after_labels_were_handed_out_ends_at_the_higher_count() {
    for (file, want) in [(1, 3), (7, 7)] {
        let mut app = faraday_core::testkit::started();
        three_labels_never_added(&mut app);
        load_record(&mut app, &with_labels(record_of(0, 0, None), file));
        assert_eq!(silent_records(&app)[0].labels, want, "{file}");
        home_and_back(&mut app);
        assert_eq!(record_labels(&mut app), want, "{file}");
    }
}

/// The count kept from a key's page raises a record of another test
/// network, which keeps its network.
#[test]
fn a_record_raised_by_the_kept_count_keeps_its_own_network() {
    use faraday_bip::keys::Network;
    let mut other = record_of(0, 0, None);
    other.network = if other.network == Network::Regtest {
        Network::Signet
    } else {
        Network::Regtest
    };
    let mut app = faraday_core::testkit::started();
    three_labels_never_added(&mut app);
    load_record(&mut app, &other);
    assert_eq!(silent_records(&app), [with_labels(other, 3)]);
}

#[test]
fn a_vault_opened_after_labels_were_handed_out_is_raised_only_where_it_has_the_wallet() {
    let mut app = faraday_core::testkit::started();
    let wallet = with_labels(record_of(0, 0, None), 1);
    files(
        &mut app,
        &[
            ("a.kdbx", vault_of("Alpha", 1, &[wallet])),
            ("b.kdbx", vault_of("Beta", 2, &[])),
        ],
    );
    three_labels_never_added(&mut app);
    open_loading_nothing(&mut app, 0, "Alpha");
    open_loading_nothing(&mut app, 1, "Beta");
    assert_eq!(vault_labels(&app, 0), [3]);
    assert_eq!(vault_labels(&app, 1), [0u32; 0]);
    assert_eq!(app.vaults().open[1].changes, 0);
    assert!(app.session.wallets.is_empty());
    home_and_back(&mut app);
    assert_eq!(record_labels(&mut app), 3);
}

#[test]
fn lock_and_power_off_leave_no_count_and_a_vaults_record_keeps_its_own() {
    // Lock with only the count left to wipe.
    let mut app = faraday_core::testkit::started();
    three_labels_never_added(&mut app);
    remove_every_key(&mut app);
    app.press(Action::Lock);
    faraday_core::testkit::settle(&mut app);
    assert!(faraday_core::testkit::holds_nothing(&app));
    add_key(&mut app);
    app.press(Action::Silent);
    assert_eq!(record_labels(&mut app), 0);

    // Lock that writes the vault.
    let mut app = faraday_core::testkit::started();
    let wallet = with_labels(record_of(0, 0, None), 1);
    files(&mut app, &[("a.kdbx", vault_of("Alpha", 1, &[wallet]))]);
    three_labels_never_added(&mut app);
    open_loading_nothing(&mut app, 0, "Alpha");
    let written = written_at_lock(&mut app);
    add_key(&mut app);
    app.press(Action::Silent);
    assert_eq!(record_labels(&mut app), 0);
    let mut app = faraday_core::testkit::started();
    let written: Vec<_> = written
        .iter()
        .map(|(n, b)| (n.as_str(), b.clone()))
        .collect();
    files(&mut app, &written);
    open_loading_nothing(&mut app, 0, "Alpha");
    assert_eq!(vault_labels(&app, 0), [3]);

    // Power off.
    let mut app = faraday_core::testkit::started();
    three_labels_never_added(&mut app);
    app.press(Action::PowerOff);
    assert!(faraday_core::testkit::holds_nothing(&app));
    add_key(&mut app);
    app.press(Action::Silent);
    assert_eq!(record_labels(&mut app), 0);
}
