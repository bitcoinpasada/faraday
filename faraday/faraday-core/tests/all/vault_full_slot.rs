//! A vault's slot is a fixed size. Entries the camera reads that do not fit
//! in the room left add nothing and say how much room is left; a vault
//! changed this session whose contents cannot be sealed stays open at the
//! lock with its changes, and the lock says which vault and why.

use crate::common::type_text;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Sheet, StorageEvent, testkit};
use faraday_shell_api::{App, EntropyBytes, Event};
use faraday_vault::records::{Record, field, kind};

fn open_vault() -> Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut app, Some(100));
    app.storage(StorageEvent::Restored {
        inbox: vec![("vault.ofv".into(), testkit::test_vault().unwrap())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.event(Event::Entropy(EntropyBytes::new([7; 32])));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
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

/// An entry whose notes take `n` bytes.
fn entry_of(n: usize) -> Record {
    Record::new(kind::ENTRY)
        .with(field::TITLE, b"filler")
        .with(field::NOTES, "x".repeat(n).as_bytes())
}

#[test]
fn entries_scanned_that_do_not_fit_add_nothing_and_say_how_much_room_is_left() {
    let mut app = open_vault();
    let id = app.vaults().open[0].id;
    let used = app.vaults().records(id).used();
    let slot = app.vaults().open[0].header().slot_len as usize;
    testkit::vault_edit(&mut app, id, |s| {
        s.insert(id, entry_of(slot - used - 2_000))
    });
    let before = app.vaults().records(id).used();
    let changes = app.vaults().open[0].changes;

    app.press(Action::Vault(V::ScanEntry));
    let text = format!("title: Bank\nnotes: {}", "y".repeat(4_000));
    app.event(Event::Scanned {
        bytes: text.into_bytes(),
    });
    let _ = app.frame();

    assert_eq!(
        app.vaults().records(id).used(),
        before,
        "records were added"
    );
    assert_eq!(app.vaults().open[0].changes, changes);
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t.contains("bytes; this vault has")),
        "{texts:?}"
    );
}

#[test]
fn a_vault_changed_this_session_that_cannot_be_sealed_stays_open_at_the_lock() {
    let mut app = open_vault();
    let id = app.vaults().open[0].id;
    let slot = app.vaults().open[0].header().slot_len as usize;
    testkit::vault_edit(&mut app, id, |s| s.insert(id, entry_of(slot)));
    testkit::vault_changed(&mut app, id);
    let used = app.vaults().records(id).used();

    app.press(Action::Lock);

    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    assert!(!app.restart_requested());
    assert_eq!(app.vaults().open.len(), 1, "the vault was closed");
    assert!(app.vaults().open[0].changes > 0);
    assert_eq!(app.vaults().records(id).used(), used, "its changes went");
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t.contains("cannot be sealed")),
        "{texts:?}"
    );
}
