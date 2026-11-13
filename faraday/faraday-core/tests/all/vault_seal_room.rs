//! A seal does not run out of randomness however many entries are new, and
//! a vault the store refuses to write is told apart from one that is
//! merely full: its amounts records stay, and the person is told the vault
//! cannot be written.

use crate::common::type_text;
use faraday_core::strings::EN;
use faraday_core::testkit::KindText;
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet::FileKind;
use faraday_core::{Action, Faraday, StorageEvent, testkit};
use faraday_shell_api::{App, EntropyBytes, Event};
use faraday_vault::records::{Record, field, kind};

fn open_file(file: Vec<u8>) -> Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut app, Some(100));
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(faraday_text::Text::of("vault.kdbx"), file)]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.event(Event::Entropy(EntropyBytes::new([7; 32])));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1, "the vault did not open");
    app
}

fn count(app: &Faraday, k: u8) -> usize {
    let id = app.vaults().open[0].id;
    app.vaults().records(id).of(k).count()
}

#[test]
fn sixty_new_entries_seal_and_all_of_them_come_back() {
    let file = crate::common::vault("Test", testkit::VAULT_PASSPHRASE, 7);
    let mut app = open_file(file);
    let id = app.vaults().open[0].id;
    testkit::vault_edit(&mut app, id, |s| {
        for n in 0..60 {
            let title = format!("entry {n}");
            s.insert(
                id,
                Record::new(kind::ENTRY).with(field::TITLE, title.as_bytes()),
            );
        }
    });
    testkit::vault_changed(&mut app, id);

    // The system has answered every request the session made.
    for n in 0..26u8 {
        app.event(Event::Entropy(EntropyBytes::new([n + 20; 32])));
    }
    app.press(Action::Lock);
    assert!(app.restart_requested(), "the lock did not seal");
    let sealed = app
        .outbox
        .iter()
        .find(|i| i.kind == FileKind::Vault)
        .map(|i| i.bytes.to_vec())
        .unwrap_or_else(|| panic!("the vault was not sealed: {:?}", app.drawn_texts()));

    let again = open_file(sealed);
    assert_eq!(count(&again, kind::ENTRY), 60);
}

#[test]
fn a_vault_that_cannot_be_written_keeps_its_amounts_and_says_so() {
    let amounts: Vec<Record> = (0..3u8)
        .map(|n| faraday_core::memory::record_of(&([n; 32], vec![1u64])))
        .collect();
    let file = testkit::kdbx_vault("Test", testkit::VAULT_PASSPHRASE, 9, amounts).unwrap();
    let mut app = open_file(file);
    let id = app.vaults().open[0].id;
    assert_eq!(count(&app, kind::AMOUNTS), 3);
    // A record the seal refuses to write.
    testkit::vault_edit(&mut app, id, |s| s.insert(id, Record::new(kind::ROUND)));
    testkit::vault_changed(&mut app, id);

    testkit::vault_add_amounts(
        &mut app,
        0,
        vec![faraday_core::memory::record_of(&([9; 32], vec![2u64]))],
    );
    assert_eq!(
        count(&app, kind::AMOUNTS),
        4,
        "amounts records were removed"
    );

    crate::common::top_up(&mut app);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    assert_eq!(app.vaults().open.len(), 1, "the vault was closed");
    assert!(app.vaults().open[0].changes > 0, "the changes were dropped");
    assert!(!app.restart_requested(), "the lock went on");
    // The lock sheet says so, and offers no lock that seals.
    assert_eq!(app.sheet, Some(faraday_core::Sheet::LockAsk));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        texts
            .iter()
            .any(|t| t.contains("cannot be written as it is")),
        "the lock did not say the vault cannot be written: {texts:?}"
    );
    assert!(!app.offers(Action::Lock), "a lock that seals is offered");
    assert_eq!(count(&app, kind::AMOUNTS), 4, "the amounts were lost");
}

#[test]
fn saving_a_wallet_into_a_vault_that_cannot_be_written_says_so() {
    let file = crate::common::vault("Test", testkit::VAULT_PASSPHRASE, 7);
    let mut app = open_file(file);
    let id = app.vaults().open[0].id;
    // A record the seal refuses to write.
    testkit::vault_edit(&mut app, id, |s| s.insert(id, Record::new(kind::ROUND)));
    testkit::vault_changed(&mut app, id);

    let c = testkit::compute();
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
    app.press(Action::EntryAdd);
    let text = faraday_core::create::NewKind::NativeSegwit
        .key_text(&c, app.session.keys.master_for_tests(0))
        .unwrap();
    let w = app
        .session
        .add_wallet(
            &c,
            "Spending",
            &format!("wpkh({}/<0;1>/*)", &*text),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    let before = app.vaults().records(id).len();
    app.press(Action::Vault(V::SaveWallet(w)));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == EN.vaults_cannot_write),
        "the vault was said to be full: {texts:?}"
    );
    assert!(!texts.iter().any(|t| t == EN.vaults_slot_too_small));
    assert_eq!(app.vaults().records(id).len(), before);
}

#[test]
fn signed_amounts_are_not_trimmed_when_memory_is_short_to_count_the_vault() {
    let amounts: Vec<Record> = (0..3u8)
        .map(|n| faraday_core::memory::record_of(&([n; 32], vec![1u64])))
        .collect();
    let file = testkit::kdbx_vault("Test", testkit::VAULT_PASSPHRASE, 9, amounts).unwrap();
    let mut app = open_file(file);
    let id = app.vaults().open[0].id;
    // A ceiling the bound is over, and no memory free for the exact count.
    testkit::vault_edit(&mut app, id, |s| s.lower_limit(1_000));
    app.storage(StorageEvent::Memory { available_mib: 0 });
    testkit::vault_add_amounts(
        &mut app,
        0,
        vec![faraday_core::memory::record_of(&([9; 32], vec![2u64]))],
    );
    assert_eq!(
        count(&app, kind::AMOUNTS),
        4,
        "amounts records were trimmed for want of memory"
    );
}
