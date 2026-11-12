//! A vault is sealed only once the system has given the session its
//! randomness: the seal's nonce is drawn from it. Before that, a lock with
//! a changed vault open seals nothing, writes nothing, keeps the vault open
//! with its changes and says why; the same lock seals once the randomness
//! arrives. Two sessions given different randomness seal the same vault
//! into different bytes.

use crate::common::type_text;
use faraday_core::strings::EN;
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet::FileKind;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent, testkit};
use faraday_shell_api::{App, EntropyBytes, Event};
use faraday_vault::records::kind;

/// A vault the first test passphrase opens, at the smallest cost: nothing
/// here is about the cost, and the nonces do not depend on it.
fn cheap_vault() -> Vec<u8> {
    crate::common::vault("Test", testkit::VAULT_PASSPHRASE, 7)
}

/// A device whose shell has not answered a request for randomness, the
/// test vault in its Inbox.
pub(crate) fn device(vault: Vec<u8>) -> Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    faraday_core::testkit::vault_speed(&mut app, Some(100));
    app.storage(StorageEvent::Restored {
        inbox: vec![("vault.kdbx".into(), vault)],
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
    type_text(app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
        if !app.vaults().open.is_empty() {
            break;
        }
    }
    assert_eq!(app.vaults().open.len(), 1, "the vault did not open");
    app.vaults()
        .records(app.vaults().open[0].id)
        .of(kind::KEY)
        .count()
}

/// Opens the vault and saves test key 2 into it: the keys it held.
pub(crate) fn open_and_change(app: &mut Faraday) -> usize {
    let held = open(app);
    app.press(Action::Entry(None));
    type_text(app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    let k = app.session.keys.len() - 1;
    app.press(Action::Vault(V::SaveKey(k)));
    assert!(
        app.vaults().open[0].changes > 0,
        "the vault was not changed"
    );
    held
}

pub(crate) fn sealed(app: &Faraday) -> Option<Vec<u8>> {
    app.outbox
        .iter()
        .find(|i| i.kind == FileKind::Vault)
        .map(|i| i.bytes.clone())
}

/// A lock, the seals run on the worker and taken.
fn lock(app: &mut Faraday) {
    app.press(Action::Lock);
    testkit::settle(app);
}

#[test]
fn a_lock_pressed_while_one_vault_is_sealed_locks_when_the_seal_ends() {
    let mut app = device(cheap_vault());
    open_and_change(&mut app);
    randomness(&mut app, 0x21);
    crate::common::top_up(&mut app);
    testkit::hold_worker();
    assert!(
        !testkit::vault_lock_one(&mut app, 0),
        "the seal is under way"
    );
    testkit::press_on_worker(&mut app, Action::Lock);
    testkit::release_worker();
    testkit::settle(&mut app);
    assert!(sealed(&app).is_some(), "the vault was not sealed");
    assert_eq!(app.vaults().open.len(), 0);
    assert!(app.restart_requested(), "the device did not lock");
}

#[test]
fn a_lock_pressed_while_one_vault_is_sealed_closes_the_code_on_show_at_once() {
    let mut app = device(cheap_vault());
    open_and_change(&mut app);
    let c = testkit::compute();
    testkit::move_to(&mut app.session, testkit::net()).expect("the test network");
    let w = app
        .session
        .add_wallet(
            &c,
            "Savings",
            &testkit::savings(),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .expect("a wallet");
    app.press(Action::ShowCode(faraday_core::Code::Descriptor(w)));
    assert!(app.qr().is_some(), "no code on show");
    randomness(&mut app, 0x21);
    crate::common::top_up(&mut app);
    testkit::hold_worker();
    assert!(
        !testkit::vault_lock_one(&mut app, 0),
        "the seal is under way"
    );
    testkit::press_on_worker(&mut app, Action::Lock);
    assert!(app.qr().is_none(), "the code stays on show during the seal");
    assert!(!app.restart_requested(), "the lock did not wait");
    testkit::release_worker();
    testkit::settle(&mut app);
    assert!(sealed(&app).is_some(), "the vault was not sealed");
    assert!(app.restart_requested(), "the device did not lock");
}

#[test]
fn a_lock_pressed_while_one_vault_fails_to_seal_still_locks() {
    let mut app = device(cheap_vault());
    open_and_change(&mut app);
    randomness(&mut app, 0x21);
    crate::common::top_up(&mut app);
    let have = app.vaults().open[0].facts().header.cost.memory_kib;
    let pick = if have == faraday_vault::file::STEPS_KIB[1] {
        2
    } else {
        1
    };
    app.press(Action::Vault(V::MemoryAsk));
    let _ = app.frame();
    app.press(Action::Vault(V::MemoryStep(pick)));
    testkit::fail_next_step_seal();
    testkit::hold_worker();
    assert!(
        !testkit::vault_lock_one(&mut app, 0),
        "the seal is under way"
    );
    testkit::press_on_worker(&mut app, Action::Lock);
    testkit::release_worker();
    testkit::settle(&mut app);
    assert!(
        sealed(&app).is_some(),
        "the Lock did not seal the vault: {:?}",
        app.toast_text()
    );
    assert!(app.restart_requested(), "the device did not lock");
}

#[test]
fn with_no_randomness_a_lock_seals_nothing_until_it_arrives() {
    let mut app = device(cheap_vault());
    let held = open_and_change(&mut app);
    app.event(Event::Tick { now_ms: 1_000 });
    app.event(Event::Tick { now_ms: 120_000 });

    app.press(Action::Lock);
    // Nothing sealed, nothing written, nothing wiped: the vault is open
    // with its change, and the sheet says why it cannot be sealed.
    assert_eq!(sealed(&app), None);
    assert!(!app.restart_requested());
    assert_eq!(app.vaults().open.len(), 1);
    assert!(app.vaults().open[0].changes > 0);
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == EN.no_randomness), "{texts:?}");
    // The vault's changes are listed as wiped from memory.
    let wiped = texts.iter().position(|t| t == "Wiped from memory");
    let lost = texts
        .iter()
        .position(|t| t.ends_with(EN.vaults_changes_not_sealed));
    assert!(wiped.is_some() && lost > wiped, "{texts:?}");
    assert!(
        texts.iter().any(|t| t == EN.vault_lock_without_sealing),
        "{texts:?}"
    );
    // Home says nothing of it.
    app.press(Action::Cancel);
    app.press(Action::Nav(Screen::Home));
    assert!(!app.drawn_texts().iter().any(|t| t == EN.no_randomness));

    // The randomness arrives: the same lock seals, and the file opens with
    // the passphrase, holding the key saved into it.
    randomness(&mut app, 0x21);
    // A seal waits for five answers: the seed and four more are short.
    for b in 0..4 {
        randomness(&mut app, 0x30 + b);
    }
    lock(&mut app);
    assert_eq!(sealed(&app), None);
    assert!(!app.restart_requested());
    assert_eq!(app.vaults().open.len(), 1);
    crate::common::top_up(&mut app);
    lock(&mut app);
    assert!(app.restart_requested());
    let file = sealed(&app).expect("the vault was not sealed");
    let mut next = device(file);
    assert_eq!(open(&mut next), held + 1);
}

#[test]
fn a_new_keyboard_and_a_lock_wait_for_the_same_randomness() {
    // Before the system's randomness: no code for a new keyboard, and no
    // seal.
    let mut app = device(cheap_vault());
    app.storage(StorageEvent::NewInput {
        id: 4,
        name: "USB Keyboard".into(),
        keyboard: true,
        pointer: false,
        touch: false,
    });
    assert_eq!(app.new_inputs()[0].code, None);
    app.press(Action::InputIgnore(4));
    // After it: both.
    randomness(&mut app, 0x22);
    app.storage(StorageEvent::NewInput {
        id: 5,
        name: "USB Keyboard".into(),
        keyboard: true,
        pointer: false,
        touch: false,
    });
    assert!(app.new_inputs()[0].code.is_some());
    app.press(Action::InputIgnore(5));
    open_and_change(&mut app);
    crate::common::top_up(&mut app);
    lock(&mut app);
    assert!(sealed(&app).is_some());
}

#[test]
fn locking_without_sealing_writes_nothing() {
    let mut app = device(cheap_vault());
    open_and_change(&mut app);
    app.press(Action::Lock);
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    app.press(Action::LockUnsealed);
    assert!(app.restart_requested());
    assert!(app.vaults().open.is_empty());
    assert_eq!(sealed(&app), None);
}

/// A seal draws its randomness from the answers the system gave, so a
/// session brought back with the same answers (a virtual machine restored
/// from a snapshot) seals the same bytes, and one given other answers
/// seals other bytes. With none waiting, a lock seals nothing.
#[test]
fn a_seal_takes_the_answers_the_system_gave() {
    let original = cheap_vault();
    let lock_with = |first: u8, answers: u8| {
        let mut app = device(original.clone());
        randomness(&mut app, first);
        for i in 0..10 {
            randomness(&mut app, answers.wrapping_add(i));
        }
        open_and_change(&mut app);
        lock(&mut app);
        sealed(&app).expect("the vault was not sealed")
    };
    let one = lock_with(0x31, 0x41);
    assert_eq!(
        one,
        lock_with(0x31, 0x41),
        "the same answers seal the same file"
    );
    assert_ne!(
        one,
        lock_with(0x31, 0x51),
        "other answers seal another file"
    );
    assert_ne!(one, original);
}

#[test]
fn two_sessions_with_no_randomness_write_nothing() {
    for _ in 0..2 {
        let mut app = device(cheap_vault());
        open_and_change(&mut app);
        lock(&mut app);
        assert_eq!(sealed(&app), None);
        assert!(!app.restart_requested());
    }
}

/// With a vault open and a key in no vault, Lock asks first with the
/// sheet every lock and power-off has (decision 28): Kept, Wiped from
/// memory with the key, Then, and Back to the vault beside Lock.
#[test]
fn a_lock_with_a_vault_open_lists_kept_wiped_and_then() {
    let mut app = device(cheap_vault());
    randomness(&mut app, 0x21);
    open(&mut app);
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    app.press(Action::LockAsk);
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    let texts = app.drawn_texts();
    let at = |s: &str| texts.iter().position(|t| t == s);
    let (kept, wiped, then) = (at("Kept"), at("Wiped from memory"), at("Then"));
    assert!(kept.is_some() && kept < wiped && wiped < then, "{texts:?}");
    assert!(then < at("Faraday restarts on Unlock"), "{texts:?}");
    let nowhere = texts.iter().position(|t| t.contains("nowhere else"));
    assert!(wiped < nowhere && nowhere < then, "the key: {texts:?}");
    assert!(app.offers(Action::Cancel));
    assert!(texts.iter().any(|t| t == "Back to the vault"), "{texts:?}");
    assert!(app.offers(Action::Lock));
    app.press(Action::Lock);
    assert!(app.restart_requested());
}

/// With no vault open, a wallet being made, the sheet's way out is
/// Cancel: there is no vault to go back to.
#[test]
fn a_lock_with_no_vault_open_is_cancelled_not_back_to_a_vault() {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, 0);
    app.press(Action::LockAsk);
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Cancel"), "{texts:?}");
    assert!(!texts.iter().any(|t| t == "Back to the vault"), "{texts:?}");
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    assert!(app.holds_secret(), "nothing wiped");
}

/// On that sheet, a key in no vault has its Save: pressed, the key is in
/// the open vault, the sheet stays and lists it there, with no Save.
#[test]
fn the_lock_sheet_saves_a_key_in_no_vault_into_the_open_vault() {
    let mut app = device(cheap_vault());
    randomness(&mut app, 0x21);
    open(&mut app);
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    app.press(Action::LockAsk);
    let k = app.session.keys.all().count() - 1;
    let save = Action::Vault(V::SaveKey(k));
    let _ = app.drawn_texts();
    assert!(app.offers(save), "Save beside the key");
    app.press(save);
    assert_eq!(app.sheet, Some(Sheet::LockAsk), "the sheet stays");
    let texts = app.drawn_texts();
    assert!(
        !texts.iter().any(|t| t.contains("nowhere else")),
        "{texts:?}"
    );
    assert!(!app.offers(save), "{texts:?}");
}
