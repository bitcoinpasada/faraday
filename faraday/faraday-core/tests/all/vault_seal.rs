//! A vault is sealed only once the system has given the session its
//! randomness: the seal's nonce is drawn from it. Before that, a lock with
//! a changed vault open seals nothing, writes nothing, keeps the vault open
//! with its changes and says why; the same lock seals once the randomness
//! arrives. Two sessions given different randomness seal the same vault
//! under different nonces.

use crate::common::type_text;
use faraday_core::strings::EN;
use faraday_core::vaults::VaultAction as V;
use faraday_core::wallet::FileKind;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent, testkit};
use faraday_shell_api::{App, EntropyBytes, Event};
use faraday_vault::records::kind;

/// A device whose shell has not answered a request for randomness, the
/// test vault in its Inbox.
fn device(vault: Vec<u8>) -> Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.vaults.ms_per_unit = Some(100);
    app.storage(StorageEvent::Restored {
        inbox: vec![("vault.ofv".into(), vault)],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

fn randomness(app: &mut Faraday, byte: u8) {
    app.event(Event::Entropy(EntropyBytes::new([byte; 32])));
}

/// Opens the vault with its first passphrase: the keys it holds.
fn open(app: &mut Faraday) -> usize {
    app.press(Action::Vault(V::Open(0)));
    type_text(app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults.open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults.open.len(), 1, "the vault did not open");
    app.vaults.open[0].contents.of(kind::KEY).count()
}

/// Opens the vault and saves test key 2 into it: the keys it held.
fn open_and_change(app: &mut Faraday) -> usize {
    let held = open(app);
    app.press(Action::Entry(None));
    type_text(app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    let k = app.session.keys.len() - 1;
    app.press(Action::Vault(V::SaveKey(k)));
    assert!(app.vaults.open[0].changes > 0, "the vault was not changed");
    held
}

fn sealed(app: &Faraday) -> Option<Vec<u8>> {
    app.outbox
        .iter()
        .find(|i| i.kind == FileKind::Vault)
        .map(|i| i.bytes.clone())
}

/// Each slot's nonce.
fn nonces(file: &[u8]) -> Vec<Vec<u8>> {
    use faraday_vault::{HEADER_LEN, NONCE_LEN, SLOTS, TAG_LEN};
    let slot = faraday_vault::read_header(file).unwrap().slot_len as usize;
    (0..SLOTS)
        .map(|i| {
            let at = HEADER_LEN + i * (slot + NONCE_LEN + TAG_LEN);
            file[at..at + NONCE_LEN].to_vec()
        })
        .collect()
}

#[test]
fn with_no_randomness_a_lock_seals_nothing_until_it_arrives() {
    let mut app = device(testkit::test_vault().unwrap());
    let held = open_and_change(&mut app);
    app.event(Event::Tick { now_ms: 120_000 });

    app.press(Action::Lock);
    // Nothing sealed, nothing written, nothing wiped: the vault is open
    // with its change, and the sheet says why it cannot be sealed.
    assert_eq!(sealed(&app), None);
    assert!(!app.restart_requested());
    assert_eq!(app.vaults.open.len(), 1);
    assert!(app.vaults.open[0].changes > 0);
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == EN.seed_missing), "{texts:?}");
    assert!(texts.iter().any(|t| t == EN.vault_not_sealed), "{texts:?}");
    assert!(
        texts.iter().any(|t| t == EN.vault_lock_without_sealing),
        "{texts:?}"
    );
    // Home says it too.
    app.press(Action::Cancel);
    app.press(Action::Nav(Screen::Home));
    assert!(app.drawn_texts().iter().any(|t| t == EN.seed_missing));

    // The randomness arrives: the same lock seals, and the file opens with
    // the passphrase, holding the key saved into it.
    randomness(&mut app, 0x21);
    assert!(!app.drawn_texts().iter().any(|t| t == EN.seed_missing));
    app.press(Action::Lock);
    assert!(app.restart_requested());
    let file = sealed(&app).expect("the vault was not sealed");
    let mut next = device(file);
    assert_eq!(open(&mut next), held + 1);
}

#[test]
fn a_new_keyboard_and_a_lock_wait_for_the_same_randomness() {
    // Before the system's randomness: no code for a new keyboard, and no
    // seal.
    let mut app = device(testkit::test_vault().unwrap());
    app.storage(StorageEvent::NewInput {
        id: 4,
        name: "USB Keyboard".into(),
        keyboard: true,
        pointer: false,
    });
    assert_eq!(app.inputs[0].code, None);
    app.press(Action::InputIgnore(4));
    // After it: both.
    randomness(&mut app, 0x22);
    app.storage(StorageEvent::NewInput {
        id: 5,
        name: "USB Keyboard".into(),
        keyboard: true,
        pointer: false,
    });
    assert!(app.inputs[0].code.is_some());
    app.press(Action::InputIgnore(5));
    open_and_change(&mut app);
    app.press(Action::Lock);
    assert!(sealed(&app).is_some());
}

#[test]
fn locking_without_sealing_writes_nothing() {
    let mut app = device(testkit::test_vault().unwrap());
    open_and_change(&mut app);
    app.press(Action::Lock);
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    app.press(Action::LockUnsealed);
    assert!(app.restart_requested());
    assert!(app.vaults.open.is_empty());
    assert_eq!(sealed(&app), None);
}

#[test]
fn different_randomness_seals_the_same_vault_under_different_nonces() {
    let original = testkit::test_vault().unwrap();
    let lock_with = |byte: u8| {
        let mut app = device(original.clone());
        randomness(&mut app, byte);
        open_and_change(&mut app);
        app.press(Action::Lock);
        sealed(&app).expect("the vault was not sealed")
    };
    let (a, b) = (lock_with(0x31), lock_with(0x32));
    let (before, a, b) = (nonces(&original), nonces(&a), nonces(&b));
    let resealed: Vec<usize> = (0..before.len()).filter(|&i| a[i] != before[i]).collect();
    assert_eq!(resealed.len(), 1, "one slot is sealed again");
    let i = resealed[0];
    assert_ne!(a[i], b[i]);
    assert_ne!(b[i], before[i]);
}

#[test]
fn two_sessions_with_no_randomness_write_nothing() {
    for _ in 0..2 {
        let mut app = device(testkit::test_vault().unwrap());
        open_and_change(&mut app);
        app.press(Action::Lock);
        assert_eq!(sealed(&app), None);
        assert!(!app.restart_requested());
    }
}
