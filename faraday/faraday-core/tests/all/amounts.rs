//! The signed-amount memory (`docs/WALLETS.md` §3.3): a transaction signed
//! here and offered again stating other amounts for its inputs is refused,
//! and the memory outlives a lock. What an open vault remembers counts,
//! whether or not the vault holds the wallet.

use faraday_core::testkit::{self, Kit};
use faraday_core::{Action, Faraday, StorageCommand, StorageEvent};
use faraday_shell_api::{App, Event, Key};

fn kit() -> Kit {
    testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap()
}

fn spending() -> faraday_psbt::Psbt {
    testkit::unsigned(&kit()).unwrap()
}

fn offer(app: &mut Faraday, psbt: &faraday_psbt::Psbt, kept: Vec<(String, Vec<u8>)>) {
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
    let mut app = faraday_core::testkit::started();
    offer(&mut app, &psbt, Vec::new());
    assert!(!app.spend().unwrap().spend.signed_here.is_empty());
    let kept = kept(&mut app);

    // A fresh process, as after a lock, offered the same transaction with
    // its input stating one satoshi more.
    let mut changed = psbt.inner().clone();
    changed.inputs[0].witness_utxo.as_mut().unwrap().value +=
        faraday_bip::bitcoin::Amount::from_sat(1);
    let changed = faraday_psbt::Psbt::from(changed);
    let mut app = faraday_core::testkit::started();
    offer(&mut app, &changed, kept.clone());
    let s = app.spend().unwrap();
    assert!(s.spend.signed_here.is_empty());
    assert!(s.error.as_deref().is_some_and(|e| e.contains("refused")));

    // The same amounts again sign.
    let mut app = faraday_core::testkit::started();
    offer(&mut app, &psbt, kept);
    assert!(!app.spend().unwrap().spend.signed_here.is_empty());
}

const PHRASE: &str = "amounts phrase";

/// A vault remembering `psbt` signed with its input stating one satoshi
/// more than it does, holding the spending wallet when `with_wallet`.
fn remembering_vault(psbt: &faraday_psbt::Psbt, with_wallet: bool) -> Vec<u8> {
    use faraday_vault::records::{field, kind};
    use faraday_vault::{Contents, Cost, Record};
    let c = testkit::compute();
    let (hash, mut amounts) = faraday_core::memory::stated(psbt).unwrap();
    amounts[0] += 1;
    let mut records = vec![
        Record::new(kind::SLOT_LABEL).with(field::LABEL, b"Ledger"),
        faraday_core::memory::record_of(&(hash, amounts)),
    ];
    if with_wallet {
        records.push(
            Record::new(kind::WALLET)
                .with(field::WALLET, kit().descriptor.as_bytes())
                .with(field::WALLET_NAME, b"Spending"),
        );
    }
    faraday_vault::create(
        &c,
        faraday_vault::SLOT_SIZES[0],
        Cost {
            memory_kib: 64 * 1024,
            passes: 1,
            lanes: 1,
        },
        &[PHRASE.as_bytes()],
        &[Contents { records }],
        &[0x4d; 32],
    )
    .unwrap()
}

/// The vault unlocked and nothing loaded from it; then the spending
/// wallet from its file, test key 1, and Sign here on `psbt`.
fn sign_with_vault_open(vault: Vec<u8>, psbt: &faraday_psbt::Psbt) -> Faraday {
    use faraday_core::Screen;
    use faraday_core::vault::VaultAction as V;
    let mut app = faraday_core::testkit::started();
    for i in 0..6u8 {
        app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
            [0x70 + i; 32],
        )));
    }
    app.storage(StorageEvent::Restored {
        inbox: vec![
            ("ledger.ofv".to_string(), vault),
            ("spend.psbt".to_string(), psbt.to_bytes()),
            (
                "spending-wallet.txt".to_string(),
                kit().descriptor.into_bytes(),
            ),
        ],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut app, Some(180));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    for c in PHRASE.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(
        app.vaults().open.len(),
        1,
        "{:?}",
        app.vaults().unlock_error
    );
    if app.sheet == Some(faraday_core::Sheet::UnlockLoad) {
        app.press(Action::Vault(V::LoadNotNow));
    }
    let w = app
        .inbox()
        .iter()
        .position(|i| i.name == "spending-wallet.txt")
        .unwrap();
    app.press(Action::LoadWallet(w));
    assert_eq!(app.session.wallets.len(), 1);
    app.press(Action::Entry(None));
    for c in testkit::test_words("bacon").chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    let p = app
        .inbox()
        .iter()
        .position(|i| i.name == "spend.psbt")
        .unwrap();
    app.press(Action::StartSpend(p));
    app.press(Action::SignHere);
    app
}

#[test]
fn a_vault_without_the_wallet_still_makes_other_amounts_refused() {
    let psbt = spending();
    let app = sign_with_vault_open(remembering_vault(&psbt, false), &psbt);
    let s = app.spend().unwrap();
    assert!(s.spend.signed_here.is_empty());
    assert!(s.error.as_deref().is_some_and(|e| e.contains("refused")));
}

#[test]
fn the_vault_of_the_wallet_makes_other_amounts_refused() {
    let psbt = spending();
    let app = sign_with_vault_open(remembering_vault(&psbt, true), &psbt);
    let s = app.spend().unwrap();
    assert!(s.spend.signed_here.is_empty());
    assert!(s.error.as_deref().is_some_and(|e| e.contains("refused")));
}
