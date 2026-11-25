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
