//! A lock the person asks for never loses a vault's changes without the
//! Lock sheet: when a later vault cannot be sealed, the vaults sealed
//! before it are closed with their files in the Outbox, it stays open with
//! its changes, nothing is wiped, and the sheet shows. An idle lock has no
//! one to ask: it lets such a vault go and says so on the Locked sheet, once.

use crate::common::type_text;
use crate::vault_seal::{device, open_and_change};
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet::FileKind;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageCommand, StorageEvent, testkit};
use faraday_shell_api::{App, Event};
use faraday_vault::Record;
use faraday_vault::records::{field, kind};

const MIN: u64 = 60_000;
const A: &str = "a phrase";
const B: &str = "b phrase";

/// A vault holding a backup plan for the Savings wallet.
fn with_plan(label: &str, phrase: &str, id: u8) -> Vec<u8> {
    let plan = Record::new(kind::PLAN)
        .with(field::PLAN_WALLET, testkit::savings().as_bytes())
        .with(field::PLAN_ANSWERS, b"seeds 1\n".as_slice());
    crate::common::vault_of(label, phrase, id, vec![plan])
}

fn unlock(app: &mut Faraday, at: usize, phrase: &str) {
    let before = app.vaults().open.len();
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(at)));
    type_text(app, phrase);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if app.vaults().open.len() > before {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(
        app.vaults().open.len(),
        before + 1,
        "the vault did not open"
    );
}

fn entry(title: &[u8]) -> Record {
    Record::new(kind::ENTRY).with(field::TITLE, title)
}

fn vault_files(app: &Faraday) -> usize {
    app.outbox
        .iter()
        .filter(|i| i.kind == FileKind::Vault)
        .count()
}

/// Two vaults, both changed, that hold one plan each; the second is
/// within a seals line of its size, so the seal of the first, which adds
/// its line to the second's plan, leaves the second too big.
fn two_changed_second_nearly_full() -> Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    testkit::vault_speed(&mut app, Some(100));
    app.storage(StorageEvent::Restored {
        inbox: vec![
            ("a.kdbx".into(), with_plan("Plans A", A, 5)),
            ("b.kdbx".into(), with_plan("Plans B", B, 6)),
        ],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    crate::common::top_up(&mut app);
    unlock(&mut app, 0, A);
    unlock(&mut app, 1, B);
    let (ida, idb) = (app.vaults().open[0].id, app.vaults().open[1].id);
    testkit::vault_edit(&mut app, ida, |s| s.insert(ida, entry(b"in a")));
    testkit::vault_changed(&mut app, ida);

    let c = testkit::compute();
    let slot = app.vaults().open[1].slot_limit();
    let used = |app: &Faraday| app.vaults().records(idb).used(&c);
    let add = |app: &mut Faraday, n: usize| {
        testkit::vault_edit(app, idb, |s| s.insert(idb, entry(&vec![b'x'; n])));
        testkit::vault_changed(app, idb);
    };
    // The first entry pays for its group; the next shows what one costs.
    add(&mut app, 10);
    let before = used(&app);
    add(&mut app, 10);
    let overhead = used(&app) - before - 10;
    while slot - used(&app) >= 100 + overhead {
        let room = slot - used(&app);
        add(&mut app, (room - overhead - 60).min(3000));
    }
    assert!(slot - used(&app) < 100, "room left: {}", slot - used(&app));
    assert!(!app.lock_would_lose(&c));
    app
}

#[test]
fn a_lock_that_cannot_seal_a_later_vault_keeps_it_open_and_asks() {
    let mut app = two_changed_second_nearly_full();
    app.press(Action::Lock);
    testkit::settle(&mut app);

    // Nothing is wiped; the Lock sheet shows, naming the vault.
    assert!(!app.restart_requested());
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        texts
            .iter()
            .any(|t| t.starts_with("b.kdbx cannot be sealed")),
        "{texts:?}"
    );
    // The first was sealed and is closed, its file in the Outbox; the
    // second is open with its changes.
    assert_eq!(vault_files(&app), 1);
    assert_eq!(app.vaults().open.len(), 1);
    assert_eq!(app.vaults().open[0].name, "b.kdbx");
    assert!(app.vaults().open[0].changes > 0);

    // Locking again asks again; Lock without sealing lets it go.
    app.press(Action::Lock);
    testkit::settle(&mut app);
    assert!(!app.restart_requested());
    assert_eq!(app.vaults().open.len(), 1);
    app.press(Action::LockUnsealed);
    testkit::settle(&mut app);
    assert!(app.restart_requested());
    assert_eq!(vault_files(&app), 1, "the second was not sealed");
}

#[test]
fn an_idle_lock_lets_the_vault_go_and_says_so() {
    let mut app = two_changed_second_nearly_full();
    app.event(Event::Tick { now_ms: 0 });
    app.event(Event::Tick { now_ms: 30 * MIN });
    testkit::settle(&mut app);
    assert!(app.restart_requested(), "the idle lock did not go ahead");
    assert_eq!(vault_files(&app), 1);
    let kept = kept(&mut app);
    assert!(kept.iter().any(|(k, _)| k == "idle-unsaved"), "{kept:?}");
}

/// What the app asked the shell to keep, last.
fn kept(app: &mut Faraday) -> Vec<(String, Vec<u8>)> {
    let mut last = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { kept, .. } = c {
            last = kept;
        }
    }
    last
}

/// A process that follows an idle lock which kept `kept`, holding a wallet
/// so that it can lock for idleness too.
fn after(kept: Vec<(String, Vec<u8>)>) -> Faraday {
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: vec![("spending-wallet.txt".into(), kit.descriptor.into_bytes())],
        outbox: Vec::new(),
        kept,
    });
    app
}

fn warned(app: &mut Faraday) -> bool {
    assert_eq!(app.sheet, Some(Sheet::Locked));
    let _ = app.frame();
    app.drawn_texts()
        .iter()
        .any(|t| t.starts_with("Changes to a vault"))
}

#[test]
fn the_lost_changes_warning_shows_once_and_not_after_a_lock_that_lost_none() {
    // No randomness has arrived: the idle lock cannot seal the change.
    let mut first = device(crate::common::vault("Test", testkit::VAULT_PASSPHRASE, 7));
    open_and_change(&mut first);
    first.event(Event::Tick { now_ms: 100_000 });
    first.event(Event::Tick {
        now_ms: 100_000 + 30 * MIN,
    });
    testkit::settle(&mut first);
    assert!(first.restart_requested());
    let mut second = after(kept(&mut first));
    let w = warned(&mut second);
    assert!(w, "the warning did not show");

    // That process locks for idleness again with nothing to lose.
    second.press(Action::LoadWallet(0));
    second.event(Event::Tick { now_ms: 0 });
    second.event(Event::Tick { now_ms: MIN });
    testkit::settle(&mut second);
    assert!(
        second.restart_requested(),
        "the second idle lock did not go"
    );
    let mut third = after(kept(&mut second));
    assert!(!warned(&mut third), "the warning showed again");
}
