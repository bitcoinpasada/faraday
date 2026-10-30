//! Sweep a private key (`docs/INTEROP.md` §5 I11, F171): a WIF typed,
//! scanned or pasted opens the Sweep sheet on its addresses and is never
//! loaded as a key; the sweep's PSBT is reviewed against the loaded
//! wallets and signed with a hold; the key is erased at Done and on
//! leaving the sheet. Every other scanner refuses a private key and
//! files nothing.

use faraday_bip::bitcoin::absolute::LockTime;
use faraday_bip::bitcoin::address::NetworkUnchecked;
use faraday_bip::bitcoin::consensus::deserialize;
use faraday_bip::bitcoin::transaction::Version;
use faraday_bip::bitcoin::{
    Address, Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness,
};
use faraday_bip::keys::Network;
use faraday_core::catalog::{Go, TILES};
use faraday_core::seeds::sweep::SweepAction as S;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent, testkit};
use faraday_shell_api::{App, Command, Event, FileKind, Key};
use faraday_strings::EN;

/// The key 1's WIF for the test networks, compressed: it pays
/// `bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080` on regtest.
const WIF: &str = "cMahea7zqjxrtgAbB7LSGbcQUr1uX1ojuat9jZodMN87JcbXMTcA";
/// The same key's mainnet WIF.
const MAINNET_WIF: &str = "KwDiBf89QgGbjEhKnhXJuH7LrciVrZi3qYjgd9M7rFU73sVHnoWn";

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn spending() -> testkit::Kit {
    testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap()
}

/// The app on regtest with the Spending wallet loaded and no key.
fn app() -> Faraday {
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, Network::Regtest).unwrap();
    app.storage(StorageEvent::Restored {
        inbox: vec![(
            "spending-wallet.txt".to_string(),
            spending().descriptor.into_bytes(),
        )],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
    app.press(Action::Nav(Screen::Home));
    app
}

fn tile() -> u8 {
    TILES.iter().position(|t| t.go == Go::Sweep).unwrap() as u8
}

/// The camera reads `bytes`.
fn read(app: &mut Faraday, bytes: &[u8]) {
    app.event(Event::Scanned {
        bytes: bytes.to_vec(),
    });
    let _ = app.frame();
}

fn type_key(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// Tools' tile, and `text` typed and used.
fn swept(text: &str) -> Faraday {
    let mut app = app();
    app.press(Action::Catalog(tile()));
    assert_eq!(app.sheet, Some(Sheet::Sweep));
    type_key(&mut app, text);
    app.press(Action::Sweep(S::Use));
    app
}

fn error(app: &Faraday) -> String {
    app.sweep()
        .and_then(|s| s.error.clone())
        .unwrap_or_default()
}

fn note(app: &Faraday) -> String {
    app.scan().and_then(|s| s.note.clone()).unwrap_or_default()
}

#[test]
fn a_typed_key_shows_its_three_addresses_and_is_never_loaded() {
    let app = swept(WIF);
    let k = app.session.keys.sweep().expect("the key is read");
    let addresses: Vec<&str> = k.addresses().iter().map(|(_, a)| a.as_str()).collect();
    assert_eq!(
        addresses,
        [
            "mrCDrCybB6J1vRfbwM5hemdJz73FwDBC8r",
            "bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080",
            "2NAUYAHhujozruyzpsFRP63mbrdaU5wnEpN",
        ]
    );
    assert_eq!(k.name(), "mrCDrCybB6J1vRfbwM5hemdJz73FwDBC8r");
    assert_eq!(app.sheet, Some(Sheet::Sweep));
    // No key joins the session, and no wallet: Wallets, Backup and
    // Explore see nothing of it.
    assert!(app.session.keys.is_empty());
    assert_eq!(app.session.wallets.len(), 1);
    assert!(app.session.keys.holds_secret());
}

/// Opened over Tools, the sheet's field takes the keys, not Find a tool.
#[test]
fn a_key_typed_over_tools_goes_to_the_sheet() {
    let mut app = app();
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::Catalog(tile()));
    type_key(&mut app, WIF);
    app.press(Action::Sweep(S::Use));
    assert!(app.session.keys.sweep().is_some(), "{}", error(&app));
}

#[test]
fn an_uncompressed_key_pays_a_legacy_address_only() {
    let app = swept("92ZdE5HoLafywnTBbzPxbvRmp75pSfzvdU3XaZGh1cToipgdHVh");
    let k = app.session.keys.sweep().expect("the key is read");
    assert!(!k.compressed());
    assert_eq!(k.addresses().len(), 1);
}

#[test]
fn a_key_of_the_other_network_and_text_that_is_no_key_are_refused() {
    let app = swept(MAINNET_WIF);
    assert!(app.session.keys.sweep().is_none());
    assert_eq!(
        error(&app),
        "This is a mainnet key, and this session is on Regtest"
    );
    let app = swept("not a key at all");
    assert!(app.session.keys.sweep().is_none());
    // A space is no character of a WIF: typed text holds none.
    assert_eq!(error(&app), EN.wif_not_a_key);
}

#[test]
fn homes_scan_opens_the_sweep_sheet_on_a_key() {
    let mut app = app();
    app.press(Action::Scan);
    read(&mut app, WIF.as_bytes());
    assert_eq!(app.sheet, Some(Sheet::Sweep));
    assert!(app.session.keys.sweep().is_some());
    assert!(app.inbox().len() == 1, "the key was filed");
}

#[test]
fn files_scanner_refuses_a_key_and_files_nothing() {
    let mut app = app();
    app.press(Action::Nav(Screen::Files));
    app.press(Action::Scan);
    read(&mut app, WIF.as_bytes());
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert_eq!(note(&app), EN.sweep_elsewhere);
    assert!(app.session.keys.sweep().is_none());
    assert_eq!(app.inbox().len(), 1, "the key was filed");
}

#[test]
fn done_and_leaving_the_sheet_erase_the_key() {
    let mut app = swept(WIF);
    app.press(Action::Sweep(S::Done));
    assert_eq!(app.sheet, None);
    assert!(app.session.keys.sweep().is_none());
    assert!(app.sweep().is_none());

    let mut app = swept(WIF);
    app.press(Action::Cancel);
    assert!(app.session.keys.sweep().is_none());
    assert!(!app.session.keys.holds_secret());

    // The sheet's own scanner keeps it, and closing that scanner comes
    // back to the sheet.
    let mut app = swept(WIF);
    app.press(Action::Sweep(S::Scan));
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(app.session.keys.sweep().is_some());
    app.press(Action::Cancel);
    assert_eq!(app.sheet, Some(Sheet::Sweep));
    assert!(app.session.keys.sweep().is_some());
}

/// A version 2 PSBT comes back as version 2: the same transaction, the
/// signature added and nothing else; one the signatures do not go back
/// into is refused, and nothing is written.
#[test]
fn a_version_2_sweep_comes_back_as_version_2() {
    use faraday_psbt::v2::PsbtV2;
    let c = testkit::compute();
    let mut app = swept(WIF);
    let to = [(receive(&app, 2), 99_000)];
    let v0 = sweep_psbt(&app, &to);
    let v2 = PsbtV2::from_v0(&c, &v0);
    app.press(Action::Sweep(S::Scan));
    read(&mut app, v2.to_base64().as_bytes());
    app.press(Action::Sweep(S::Hold));
    let s = app.sweep().unwrap();
    let done = s.signed.as_ref().unwrap_or_else(|| panic!("{:?}", s.error));
    let back = PsbtV2::parse_bytes(&c, &done.psbt).expect("the signed PSBT is version 2");
    let back_v0 = back.to_v0(&c).unwrap();
    assert_eq!(
        back_v0.unsigned_tx().compute_txid(),
        v0.unsigned_tx().compute_txid()
    );
    assert_eq!(back_v0.inner().inputs[0].partial_sigs.len(), 1);
    let file = app
        .outbox
        .iter()
        .find(|i| i.name == "sweep-signed.psbt")
        .unwrap();
    assert!(PsbtV2::parse_bytes(&c, &file.bytes).is_ok());

    // The signatures of another transaction do not go back in.
    let other = sweep_psbt(&app, &[(receive(&app, 3), 98_000)]);
    assert!(faraday_core::seeds::sweep::out_bytes(Some(&v2), &other).is_err());
}

/// A PSBT spending 100,000 sat on the key's native SegWit output, paying
/// `to`, as a watch-only wallet builds it: the previous transaction
/// beside the output, no key origin.
fn sweep_psbt(app: &Faraday, to: &[(ScriptBuf, u64)]) -> faraday_psbt::Psbt {
    let k = app.session.keys.sweep().unwrap();
    let address: Address<NetworkUnchecked> = k.addresses()[1].1.parse().unwrap();
    let spk = address.assume_checked().script_pubkey();
    let funding = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: ScriptBuf::from_bytes(vec![1, 101]),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(100_000),
            script_pubkey: spk,
        }],
    };
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::new(funding.compute_txid(), 0),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::new(),
        }],
        output: to
            .iter()
            .map(|(s, v)| TxOut {
                value: Amount::from_sat(*v),
                script_pubkey: s.clone(),
            })
            .collect(),
    };
    let mut inner = faraday_bip::bitcoin::Psbt::from_unsigned_tx(tx).unwrap();
    inner.inputs[0].witness_utxo = Some(funding.output[0].clone());
    inner.inputs[0].non_witness_utxo = Some(funding);
    faraday_psbt::Psbt::from(inner)
}

/// The Spending wallet's receive address `i`, as the app derives it.
fn receive(app: &Faraday, i: u32) -> ScriptBuf {
    let c = testkit::compute();
    app.session.wallets[0]
        .policy()
        .address_at(&c, Network::Regtest, false, i)
        .unwrap()
        .script_pubkey()
}

#[test]
fn the_sweeps_psbt_is_reviewed_against_the_wallets_and_signed_with_a_hold() {
    let mut app = swept(WIF);
    let foreign = ScriptBuf::from_bytes([vec![0x00, 0x14], vec![7; 20]].concat());
    let to = [(receive(&app, 3), 90_000), (foreign, 9_000)];
    let psbt = sweep_psbt(&app, &to);
    app.press(Action::Sweep(S::Scan));
    read(&mut app, psbt.to_base64().as_bytes());
    assert_eq!(app.sheet, Some(Sheet::Sweep), "{}", note(&app));
    let r = app
        .sweep()
        .unwrap()
        .review
        .as_ref()
        .expect("the PSBT is reviewed");
    assert_eq!(r.inputs.len(), 1);
    assert_eq!(r.others, 0);
    assert_eq!(
        r.outputs[0].wallet.as_deref(),
        Some("Spending, receive address 3")
    );
    // The output to no loaded wallet is flagged, and the hold is still
    // offered past it.
    assert_eq!(r.outputs[1].wallet, None);
    assert!(r.stop.is_none(), "{:?}", r.stop);

    app.press(Action::Sweep(S::Hold));
    let s = app.sweep().unwrap();
    let done = s.signed.as_ref().unwrap_or_else(|| panic!("{:?}", s.error));
    assert_eq!(
        done.line,
        "Signed 1 inputs. The transaction is in the Outbox"
    );
    let names: Vec<&str> = app.outbox.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains(&"sweep-signed.psbt") && names.contains(&"sweep-final.txn"));
    let tx: Transaction = deserialize(&unhex(done.hex.as_deref().unwrap())).unwrap();
    assert_eq!(tx.input[0].witness.len(), 2);
    assert_eq!(
        hex(&tx.input[0].witness[1]),
        "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"
    );
    // The key stays until Done.
    assert!(app.session.keys.sweep().is_some());
    app.press(Action::Sweep(S::ShowQr));
    assert_eq!(app.sheet, Some(Sheet::Qr));
    assert!(
        app.session.keys.sweep().is_none(),
        "leaving for the QR erases the key"
    );
}

#[test]
fn a_psbt_of_another_key_signs_nothing() {
    let mut app = swept(WIF);
    let to = [(receive(&app, 0), 99_000)];
    let psbt = sweep_psbt(&app, &to);
    let mut other = psbt.into_inner();
    other.inputs[0].witness_utxo.as_mut().unwrap().script_pubkey = receive(&app, 1);
    other.inputs[0].non_witness_utxo = None;
    let other = faraday_psbt::Psbt::from(other);
    app.press(Action::Sweep(S::Scan));
    read(&mut app, other.to_base64().as_bytes());
    let r = app.sweep().unwrap().review.as_ref().unwrap();
    assert_eq!(r.stop.as_deref(), Some(EN.sweep_no_inputs));
    app.press(Action::Sweep(S::Hold));
    assert!(app.sweep().unwrap().signed.is_none());
    assert!(app.outbox.is_empty());
}

/// Faraday Demo: the Sweep scanner takes a key pasted; no other scanner
/// does.
#[test]
fn the_sweep_scanner_takes_a_pasted_key_on_the_desktop() {
    let mut app = app();
    app.clipboard = true;
    app.press(Action::Catalog(tile()));
    app.press(Action::Sweep(S::Scan));
    assert!(app.scan_pastes());
    app.press(Action::ScanPaste);
    let asked = std::iter::from_fn(|| app.poll_command())
        .any(|c| matches!(c, Command::RequestClipboard { .. }));
    assert!(asked);
    app.event(Event::Clipboard {
        kind: FileKind::Text,
        text: WIF.to_string(),
    });
    assert_eq!(app.sheet, Some(Sheet::Sweep), "{}", note(&app));
    assert!(app.session.keys.sweep().is_some());
}
