//! A vault is read, unlocked and sealed only where this computer has the
//! memory free for it, found out before any is taken: a file too large for
//! what is free says so and is not read, and a lock that cannot seal for
//! memory keeps the vault open with its changes and says why, an idle lock
//! included.

use crate::vault_seal::{device, open_and_change, sealed};
use faraday_core::vault::VaultAction as V;
use faraday_core::{
    Action, Faraday, Screen, Sheet, StickInfo, StorageCommand, StorageEvent, testkit,
};
use faraday_shell_api::{App, Event};
use faraday_vault::records::{Record, field, kind};

const MIB: u64 = 1024 * 1024;
const MIN: u64 = 60_000;

fn memory(app: &mut Faraday, available_mib: u32) {
    app.storage(StorageEvent::Memory { available_mib });
}

fn stick_with(app: &mut Faraday, name: &str, size: u64) {
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: "S".into(),
        label: "TESTSTICK".into(),
        boot: false,
        files: vec![(name.into(), size)],
        vaults: vec![name.into()],
    }]));
}

fn reads_asked(app: &mut Faraday) -> usize {
    let mut n = 0;
    while let Some(c) = app.poll_storage() {
        if matches!(c, StorageCommand::Read { .. }) {
            n += 1;
        }
    }
    n
}

#[test]
fn a_vault_file_too_large_for_the_free_memory_says_so_and_is_not_read() {
    let mut app = testkit::started();
    memory(&mut app, 20);
    stick_with(&mut app, "big.kdbx", 17 * MIB);
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        texts
            .iter()
            .any(|t| t == "Needs 34 MiB. This computer has 20 MiB free."),
        "{texts:?}"
    );
    assert!(!app.offers(Action::VisitIn(0)), "the file can be ticked");
    app.press(Action::VisitIn(0));
    app.press(Action::VisitCopy);
    assert_eq!(reads_asked(&mut app), 0);
    assert!(app.inbox().is_empty());

    // With the memory free for it, the same file is offered.
    memory(&mut app, 200);
    let _ = app.frame();
    assert!(app.offers(Action::VisitIn(0)), "the file cannot be ticked");
}

/// A device with a vault open and changed, and the randomness a seal needs,
/// then short of memory.
fn changed_and_short() -> Faraday {
    let mut app = device(crate::common::vault("Test", testkit::VAULT_PASSPHRASE, 7));
    crate::common::top_up(&mut app);
    open_and_change(&mut app);
    memory(&mut app, 50);
    app
}

fn says_needs(app: &Faraday) -> bool {
    app.toast_text()
        .is_some_and(|t| t.starts_with("Needs ") && t.ends_with("This computer has 50 MiB free."))
}

#[test]
fn a_lock_that_cannot_seal_for_memory_keeps_the_vault_open_with_its_changes() {
    let mut app = changed_and_short();
    app.press(Action::Lock);
    testkit::settle(&mut app);
    assert!(!app.restart_requested());
    assert_eq!(sealed(&app), None);
    assert_eq!(app.vaults().open.len(), 1);
    assert!(app.vaults().open[0].changes > 0);
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    assert!(says_needs(&app));

    // Once the memory is free the same lock seals.
    memory(&mut app, 15_000);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    assert!(app.restart_requested());
    assert!(sealed(&app).is_some());
}

#[test]
fn an_idle_lock_that_cannot_seal_for_memory_does_not_lose_the_changes() {
    let mut app = changed_and_short();
    app.event(Event::Tick { now_ms: 0 });
    app.event(Event::Tick { now_ms: 30 * MIN });
    // The warning runs out, the lock starts, and the reason shows.
    let mut said = false;
    for t in 1..120u64 {
        app.event(Event::Tick {
            now_ms: 30 * MIN + t * 1000,
        });
        said |= says_needs(&app);
    }
    assert!(said, "no reason shown");
    testkit::settle(&mut app);
    assert!(!app.restart_requested(), "the idle lock went ahead");
    assert_eq!(sealed(&app), None);
    assert_eq!(app.vaults().open.len(), 1);
    assert!(app.vaults().open[0].changes > 0);
    // The person is told what memory it needs, on the sheet too.
    let _ = app.frame();
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t.contains("Needs ") && t.contains("This computer has 50 MiB free.")),
        "{:?}",
        app.drawn_texts()
    );
    // A later idle period warns and tries again, still short.
    app.press(Action::Cancel);
    let later = 31 * MIN + 120_000;
    app.event(Event::Tick { now_ms: later });
    let (mut warned, mut said) = (false, false);
    for t in 1..=31 * 60u64 {
        app.event(Event::Tick {
            now_ms: later + t * 1000,
        });
        warned |= app.sheet == Some(Sheet::IdleWarn);
        said |= says_needs(&app);
    }
    assert!(warned, "no second warning");
    assert!(said, "the idle lock did not try again");
    assert_eq!(app.vaults().open.len(), 1);
    assert!(app.vaults().open[0].changes > 0);
}

#[test]
fn the_lock_sheet_of_a_vault_short_of_memory_offers_lock_and_lock_without_saving() {
    let mut app = changed_and_short();
    app.press(Action::Lock);
    testkit::settle(&mut app);
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t.contains("memory is short") && t.contains("Needs ")),
        "{:?}",
        app.drawn_texts()
    );
    assert!(app.offers(Action::Lock));
    assert!(app.offers(Action::LockUnsealed));
    // Lock tries again: still short, still open with its changes.
    app.press(Action::Lock);
    testkit::settle(&mut app);
    assert!(!app.restart_requested());
    assert_eq!(app.vaults().open.len(), 1);
    assert!(app.vaults().open[0].changes > 0);
    // Lock without saving is the person's choice: the vault locks and its
    // changes are not in the file.
    app.press(Action::LockUnsealed);
    testkit::settle(&mut app);
    assert!(app.restart_requested());
    assert_eq!(sealed(&app), None);
}

#[test]
fn the_lock_sheet_moves_on_from_a_memory_short_line_once_memory_is_free() {
    // Short of memory to count a vault past a lowered ceiling; then memory
    // frees, the count is made, and the sheet says the changes are past the
    // ceiling instead of asking again for memory.
    let mut app = grown(45);
    let id = app.vaults().open[0].id;
    testkit::vault_edit(&mut app, id, |s| s.lower_limit(100_000));
    testkit::vault_changed(&mut app, id);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    let _ = app.frame();
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t.contains("memory is short")),
        "{:?}",
        app.drawn_texts()
    );
    memory(&mut app, 15_000);
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        !texts.iter().any(|t| t.contains("memory is short")),
        "{texts:?}"
    );
    assert!(
        texts.iter().any(|t| t.contains("cannot be sealed")),
        "{texts:?}"
    );
}

/// A vault of entries whose document is about 2 MiB.
fn vault_of_entries() -> Vec<u8> {
    let entries = (0..200)
        .map(|_| {
            Record::new(kind::ENTRY)
                .with(field::TITLE, b"filler")
                .with(field::NOTES, "x".repeat(10_000).as_bytes())
        })
        .collect();
    crate::common::vault_with("Test", testkit::VAULT_PASSPHRASE, 7, entries)
}

fn unlock_with_free(free: u32) -> Faraday {
    unlock_file_with_free(vault_of_entries(), free)
}

fn unlock_file_with_free(file: Vec<u8>, free: u32) -> Faraday {
    let mut app = device(file);
    memory(&mut app, free);
    app.press(Action::Vault(faraday_core::vault::VaultAction::Open(0)));
    crate::common::type_text(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Vault(faraday_core::vault::VaultAction::Unlock));
    for t in 1..60u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    testkit::settle(&mut app);
    app
}

#[test]
fn a_vault_of_entries_that_the_free_memory_cannot_parse_says_so_and_stays_locked() {
    // Room for the file and its body, not for the document read from it.
    let mut app = unlock_with_free(125);
    assert!(app.vaults().open.is_empty(), "the vault opened");
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        texts
            .iter()
            .any(|t| t.starts_with("Needs ") && t.ends_with("This computer has 125 MiB free.")),
        "{texts:?}"
    );

    let app = unlock_with_free(15_000);
    assert_eq!(app.vaults().open.len(), 1, "the vault did not open");
}

#[test]
fn a_vault_of_entries_opens_with_exactly_the_memory_its_parse_needs_free() {
    use faraday_core::vault::{Need, need_mib, vault_need_mib};
    let file = vault_of_entries();
    let opened = faraday_vault::kdbx::open(&testkit::compute(), &file, testkit::VAULT_PASSPHRASE)
        .expect("opens");
    let attachments = opened.binaries.iter().map(|b| b.data.len()).sum();
    let parse = need_mib(Need::Parse {
        document: opened.document.len(),
        attachments,
    });
    // The unlock's own test is met as well: the figure is the larger.
    let free = parse.max(vault_need_mib(64, file.len()));
    let app = unlock_with_free(free);
    assert_eq!(
        app.vaults().open.len(),
        1,
        "the vault did not open at {free} MiB"
    );
}

/// A vault open and changed with 40 long entries put in it since it
/// opened (about 2.3 MiB of notes), on a device that has `free` MiB.
fn grown(free: u32) -> Faraday {
    let mut app = device(crate::common::vault("Test", testkit::VAULT_PASSPHRASE, 7));
    crate::common::top_up(&mut app);
    open_and_change(&mut app);
    memory(&mut app, free);
    let id = app.vaults().open[0].id;
    testkit::vault_edit(&mut app, id, |s| {
        for _ in 0..40 {
            s.insert(
                id,
                Record::new(kind::ENTRY)
                    .with(field::TITLE, b"long")
                    .with(field::NOTES, "x".repeat(60_000).as_bytes()),
            );
        }
    });
    testkit::vault_changed(&mut app, id);
    app
}

#[test]
fn a_lock_counts_what_was_added_since_the_vault_opened() {
    // Room for the vault as it opened, not for the long entries put in it.
    let mut app = grown(120);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    assert!(!app.restart_requested());
    assert_eq!(sealed(&app), None);
    assert_eq!(app.vaults().open.len(), 1);
    assert!(app.vaults().open[0].changes > 0);
    let toast = app.toast_text().unwrap_or_default().to_string();
    assert!(
        toast.starts_with("Needs ") && toast.ends_with("This computer has 120 MiB free."),
        "{toast}"
    );

    // With the memory free for it the same lock seals.
    memory(&mut app, 15_000);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    assert!(app.restart_requested());
    assert!(sealed(&app).is_some());
}

#[test]
fn a_lock_is_not_refused_for_more_than_what_was_added_takes() {
    // Enough for what the entries put in take, though not for what a
    // bound that counts each of them several times over would say, and for
    // the copy that counts them, sized by that bound.
    let mut app = grown(300);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    assert!(app.restart_requested(), "{:?}", app.toast_text());
    assert!(sealed(&app).is_some());
}

#[test]
fn an_unlock_is_checked_against_the_memory_free_now() {
    let mut app = device(crate::common::vault("Test", testkit::VAULT_PASSPHRASE, 7));
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (
                "vault.kdbx".into(),
                crate::common::vault("Test", testkit::VAULT_PASSPHRASE, 7),
            ),
            ("second.kdbx".into(), vault_of_entries()),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    open_and_change(&mut app);
    // The memory falls while the first vault is open.
    memory(&mut app, 125);
    let before = app.vaults().open.len();
    app.press(Action::Vault(faraday_core::vault::VaultAction::Open(1)));
    crate::common::type_text(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Vault(faraday_core::vault::VaultAction::Unlock));
    for t in 1..60u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    testkit::settle(&mut app);
    assert_eq!(app.vaults().open.len(), before, "the second vault opened");
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        texts
            .iter()
            .any(|t| t.starts_with("Needs ") && t.ends_with("This computer has 125 MiB free.")),
        "{texts:?}"
    );
}

/// A vault of about 12 MiB of small entries: a document whose parse takes
/// well over the 100 MiB the app itself is allowed for.
fn vault_of_many_entries() -> Vec<u8> {
    let entries = (0..30_000)
        .map(|n| {
            Record::new(kind::ENTRY)
                .with(field::TITLE, format!("site {n}").as_bytes())
                .with(field::USERNAME, b"someone")
                .with(field::PASSWORD, b"correct horse")
                .with(field::URL, b"https://example.org/login")
        })
        .collect();
    crate::common::vault_with("Test", testkit::VAULT_PASSPHRASE, 7, entries)
}

fn parse_figure(file: &[u8]) -> u32 {
    use faraday_core::vault::{Need, need_mib, vault_need_mib};
    let opened = faraday_vault::kdbx::open(&testkit::compute(), file, testkit::VAULT_PASSPHRASE)
        .expect("opens");
    let attachments = opened.binaries.iter().map(|b| b.data.len()).sum();
    need_mib(Need::Parse {
        document: opened.document.len(),
        attachments,
    })
    .max(vault_need_mib(64, file.len()))
}

#[test]
fn a_large_vault_of_entries_opens_with_exactly_the_memory_its_parse_needs_free() {
    let file = vault_of_many_entries();
    let free = parse_figure(&file);
    assert!(free > 150, "the document is too small to show it: {free}");
    let app = unlock_file_with_free(file, free);
    assert_eq!(
        app.vaults().open.len(),
        1,
        "the vault did not open at {free} MiB"
    );
}

/// A vault file whose document is half bare `<Entry/>` and half entries of
/// long values, about 12 MiB.
fn crafted_vault() -> Vec<u8> {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"yes\"?>\n<KeePassFile><Meta></Meta><Root><Group><UUID>",
    );
    xml.push_str("CQkJCQkJCQkJCQkJCQ==");
    xml.push_str("</UUID><Name>V</Name>");
    xml.push_str(&"<Entry/>".repeat(750_000));
    let long = "v".repeat(20_000);
    for _ in 0..300 {
        xml.push_str(&format!(
            "<Entry><String><Key>Notes</Key><Value>{long}</Value></String></Entry>"
        ));
    }
    xml.push_str("</Group></Root></KeePassFile>\n\n");
    let random = [3u8; faraday_vault::kdbx::RANDOM_LEN];
    let contents = faraday_vault::kdbx::Contents {
        document: xml.as_bytes(),
        binaries: faraday_vault::kdbx::Binaries::all(&[]),
        public_custom_data: None,
    };
    let cost = faraday_vault::Cost {
        memory_kib: 8 * 1024,
        passes: 1,
        lanes: 1,
    };
    faraday_vault::kdbx::seal(
        &testkit::compute(),
        testkit::VAULT_PASSPHRASE,
        cost,
        &random,
        &contents,
        xml.len() + 1024 * 1024,
    )
    .expect("sealed")
}

#[test]
fn a_crafted_document_takes_no_more_than_its_parse_check_counted_however_much_is_free() {
    // Bare entries cost a few bytes of document each and far more to hold:
    // with all the memory in the world free, the open is still held to
    // what the parse check counted for the document's length.
    let file = crafted_vault();
    let figure = parse_figure(&file);
    let app = unlock_file_with_free(file, figure + 6000);
    assert!(app.vaults().open.is_empty(), "the crafted vault opened");
    let mut app = app;
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        texts
            .iter()
            .any(|t| t == "Needs 113 MiB. This computer has 6.2\u{a0}GiB free."),
        "{texts:?}"
    );
}

#[test]
fn a_vault_in_keepassxc_s_shape_opens_with_exactly_the_memory_its_parse_needs_free() {
    // KeePassXC's own save (gzip), and a KeePass-shaped vault holding a
    // 1.2 MiB attachment (plain).
    for name in [
        "keepassxc-2.7-aes-argon2d-gzip-edited.kdbx",
        "pykeepass-chacha20-argon2id-plain-large.kdbx",
    ] {
        opens_at_its_parse_figure(name);
    }
}

fn opens_at_its_parse_figure(name: &str) {
    use faraday_core::vault::{Need, need_mib, vault_need_mib};
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/vectors/kdbx")
        .join(name);
    let file = std::fs::read(&path).expect("the vector");
    let pass = "kdbx vector passphrase";
    let opened = faraday_vault::kdbx::open(&testkit::compute(), &file, pass).expect("opens");
    let attachments = opened.binaries.iter().map(|b| b.data.len()).sum();
    let memory_mib = faraday_vault::file::inspect(&file)
        .expect("a vault file")
        .header
        .cost
        .memory_kib
        / 1024;
    let free = need_mib(Need::Parse {
        document: opened.document.len(),
        attachments,
    })
    .max(vault_need_mib(memory_mib, file.len()));
    let mut app = device(file);
    memory(&mut app, free);
    app.press(Action::Vault(faraday_core::vault::VaultAction::Open(0)));
    crate::common::type_text(&mut app, pass);
    app.press(Action::Vault(faraday_core::vault::VaultAction::Unlock));
    for t in 1..60u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    testkit::settle(&mut app);
    assert_eq!(
        app.vaults().open.len(),
        1,
        "{name} did not open at {free} MiB"
    );
}

#[test]
fn copy_of_a_vault_file_the_memory_no_longer_holds_is_not_read_and_says_what_it_needs() {
    let mut app = testkit::started();
    memory(&mut app, 200);
    stick_with(&mut app, "big.kdbx", 17 * MIB);
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    assert!(app.offers(Action::VisitIn(0)));
    // The memory falls after the file was ticked, before Copy.
    memory(&mut app, 20);
    app.press(Action::VisitCopy);
    assert_eq!(reads_asked(&mut app), 0);
    assert!(app.inbox().is_empty());
    let _ = app.frame();
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "Needs 34 MiB. This computer has 20 MiB free."),
        "{:?}",
        app.drawn_texts()
    );
}

#[test]
fn the_boot_import_does_not_read_a_vault_file_the_memory_does_not_hold() {
    use faraday_core::boot_import::ImportAction as I;
    let mut app = testkit::started();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    memory(&mut app, 15_000);
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: "BOOT".into(),
        label: "FARADAY".into(),
        boot: true,
        files: vec![("big.kdbx".into(), 17 * MIB)],
        vaults: vec!["big.kdbx".into()],
    }]));
    let _ = app.frame();
    memory(&mut app, 20);
    app.press(Action::Import(I::Go));
    assert_eq!(reads_asked(&mut app), 0);
    // Every ticked file was refused, so nothing is left to read: the import
    // ends, says one file was not read and what memory it needs.
    let _ = app.frame();
    let toast = app.toast_text().map(str::to_string);
    assert!(
        toast.as_deref().is_some_and(|t| t.contains("1 not read")
            && t.contains("Needs 34 MiB. This computer has 20 MiB free.")),
        "{toast:?} {:?}",
        app.drawn_texts()
    );
}

#[test]
fn a_lock_past_the_ceiling_for_want_of_memory_to_count_says_memory_is_short() {
    // The grown vault's changes are over a lowered ceiling by the bound; the
    // memory free holds the vault but not a copy of its document, so it is
    // not counted exactly and the lock says what it needs.
    let mut app = grown(45);
    let id = app.vaults().open[0].id;
    testkit::vault_edit(&mut app, id, |s| s.lower_limit(100_000));
    testkit::vault_changed(&mut app, id);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    assert!(!app.restart_requested());
    assert_eq!(app.vaults().open.len(), 1);
    assert!(app.vaults().open[0].changes > 0);
    let _ = app.frame();
    let mut texts = app.drawn_texts();
    texts.extend(app.toast_text().map(str::to_string));
    assert!(
        texts
            .iter()
            .any(|t| t.contains("This computer has 45 MiB free.")),
        "{texts:?}"
    );
}

#[test]
fn an_idle_lock_past_a_lowered_ceiling_for_want_of_memory_to_count_keeps_the_vault_open() {
    // As the lock above, started by idleness: memory is short, so the
    // changes are neither sealed nor given up.
    let mut app = grown(45);
    let id = app.vaults().open[0].id;
    testkit::vault_edit(&mut app, id, |s| s.lower_limit(100_000));
    testkit::vault_changed(&mut app, id);
    app.event(Event::Tick { now_ms: 0 });
    app.event(Event::Tick { now_ms: 30 * MIN });
    for t in 1..120u64 {
        app.event(Event::Tick {
            now_ms: 30 * MIN + t * 1000,
        });
    }
    testkit::settle(&mut app);
    assert!(!app.restart_requested(), "the idle lock went ahead");
    assert_eq!(sealed(&app), None);
    assert_eq!(app.vaults().open.len(), 1);
    assert!(app.vaults().open[0].changes > 0, "the changes were dropped");
}

#[test]
fn visit_shows_what_this_computer_has_free_as_it_is_read() {
    let mut app = testkit::started();
    testkit::read_memory_as(&mut app, &[200]);
    stick_with(&mut app, "big.kdbx", 17 * MIB);
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    assert!(app.offers(Action::VisitIn(0)));
    // The memory falls; the list follows at its next reading.
    testkit::read_memory_as(&mut app, &[20]);
    app.event(Event::Tick { now_ms: 1000 });
    let _ = app.frame();
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "Needs 34 MiB. This computer has 20 MiB free."),
        "{:?}",
        app.drawn_texts()
    );
}

#[test]
fn unlock_shows_what_this_computer_has_free_as_it_is_read() {
    let mut app = device(crate::common::vault("Test", testkit::VAULT_PASSPHRASE, 7));
    testkit::read_memory_as(&mut app, &[2048]);
    app.press(Action::Vault(faraday_core::vault::VaultAction::Open(0)));
    app.event(Event::Tick { now_ms: 1000 });
    let _ = app.frame();
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "This computer: 2\u{a0}GiB free"),
        "{:?}",
        app.drawn_texts()
    );
    // The memory falls; the screen follows at its next reading.
    testkit::read_memory_as(&mut app, &[500]);
    app.event(Event::Tick { now_ms: 2000 });
    let _ = app.frame();
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "This computer: 500 MiB free"),
        "{:?}",
        app.drawn_texts()
    );
}

thread_local! {
    /// Reads of the free memory made so far, and which of them says 20 MiB
    /// (the rest say 270).
    static READS: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) };
}

fn read_with_one_low() -> Option<u32> {
    READS.with(|r| {
        let (n, low) = r.get();
        r.set((n + 1, low));
        Some(if n == low { 20 } else { 270 })
    })
}

#[test]
fn a_seal_is_not_started_when_the_memory_falls_between_its_two_readings() {
    // The grown vault's bound says its seal needs 360 MiB and its exact
    // count 164: with 270 free, a copy of the document to count it fits, so
    // it is counted, and the seal goes ahead. That count reads the free
    // memory again; if that reading is lower the count cannot be made and the
    // seal waits on the bound's figure, it does not go ahead as if the vault
    // fitted. Each reading in turn is the one that says 20: where it is the
    // count's, the lock is refused with the bound's figure against the 270
    // the seal was checked at.
    let mut refused = 0;
    for low in 0..30 {
        let mut app = grown(270);
        READS.with(|r| r.set((0, low)));
        app.set_memory_reader(read_with_one_low);
        app.press(Action::Lock);
        testkit::settle(&mut app);
        if app.restart_requested() {
            continue;
        }
        assert_eq!(sealed(&app), None, "{low}");
        assert_eq!(app.vaults().open.len(), 1, "{low}");
        assert!(app.vaults().open[0].changes > 0, "{low}");
        if app
            .toast_text()
            .is_some_and(|t| t.starts_with("Needs 360 MiB") && t.ends_with("has 270 MiB free."))
        {
            refused += 1;
        }
    }
    assert!(refused > 0, "no reading fell between a seal's two");
}

// ---- The Lock sheet names what Lock without saving lets go ----

/// A device with two vaults open, `alpha.kdbx` and `beta.kdbx`, nothing
/// changed, and the randomness a seal needs.
fn two_open() -> Faraday {
    n_open(2)
}

/// [`two_open`] with a third vault, `gamma.kdbx`, when `n` is 3.
fn n_open(n: usize) -> Faraday {
    let mut app = testkit::started();
    memory(&mut app, 15_000);
    testkit::vault_speed(&mut app, Some(100));
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(
            vec![
                (
                    "alpha.kdbx".into(),
                    crate::common::vault("Alpha", testkit::VAULT_PASSPHRASE, 7),
                ),
                (
                    "beta.kdbx".into(),
                    crate::common::vault("Beta", testkit::VAULT_PASSPHRASE, 8),
                ),
                (
                    "gamma.kdbx".into(),
                    crate::common::vault("Gamma", testkit::VAULT_PASSPHRASE, 9),
                ),
            ]
            .into_iter()
            .take(n)
            .collect(),
        ),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    crate::common::top_up(&mut app);
    for at in 0..n {
        app.press(Action::Nav(Screen::Vaults));
        app.press(Action::Vault(V::Open(at)));
        let _ = app.frame();
        app.press(Action::Vault(V::FocusPassphrase));
        crate::common::type_text(&mut app, testkit::VAULT_PASSPHRASE);
        app.press(Action::Vault(V::Unlock));
        for t in 1..60u64 {
            if app.vaults().open.len() > at {
                break;
            }
            let _ = app.frame();
            app.event(Event::Tick { now_ms: t * 1000 });
        }
        assert_eq!(app.vaults().open.len(), at + 1, "a vault did not open");
    }
    app
}

/// `n` long entries (60 KB of notes each) put in open vault `o`, a change.
fn add_notes(app: &mut Faraday, o: usize, n: usize) {
    let id = app.vaults().open[o].id;
    testkit::vault_edit(app, id, |s| {
        for _ in 0..n {
            s.insert(
                id,
                Record::new(kind::ENTRY)
                    .with(field::TITLE, b"long")
                    .with(field::NOTES, "x".repeat(60_000).as_bytes()),
            );
        }
    });
    testkit::vault_changed(app, id);
}

/// The Lock sheet's text after a Lock that stopped.
fn sheet_text(app: &mut Faraday) -> String {
    let _ = app.frame();
    app.drawn_texts().join(" ")
}

/// Free memory, MiB, at which a vault of a few entries seals and the one
/// grown by forty long entries does not.
const BETWEEN: u32 = 250;

#[test]
fn the_lock_sheet_names_a_vault_past_the_ceiling_and_one_short_of_memory_and_lets_both_go() {
    // Alpha's 240 KB are counted exactly and are over the lowered ceiling;
    // Beta's 2.4 MiB cannot be counted in the memory free, so it is short.
    let mut app = two_open();
    add_notes(&mut app, 0, 4);
    add_notes(&mut app, 1, 40);
    let id = app.vaults().open[0].id;
    testkit::vault_edit(&mut app, id, |s| s.lower_limit(100_000));
    memory(&mut app, 45);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    let text = sheet_text(&mut app);
    assert!(
        text.contains("alpha") && text.contains("cannot be sealed"),
        "{text}"
    );
    assert!(
        text.contains("beta") && text.contains("memory is short"),
        "{text}"
    );
    // One press lets all the named go.
    assert!(app.offers(Action::LockUnsealed));
    app.press(Action::LockUnsealed);
    testkit::settle(&mut app);
    assert!(app.restart_requested(), "{:?}", app.toast_text());
    assert_eq!(sealed(&app), None);
}

#[test]
fn the_lock_sheet_names_a_vault_with_an_item_moved_to_a_short_one() {
    // Beta fits; Alpha, which an item of Beta was moved into, is short. A
    // lock without saving would lose Beta's last sealed file's item, so the
    // sheet names Beta too.
    let mut app = two_open();
    add_notes(&mut app, 0, 40);
    add_notes(&mut app, 1, 1);
    testkit::vault_moved(&mut app, 1, 0);
    memory(&mut app, BETWEEN);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    let text = sheet_text(&mut app);
    assert!(
        text.contains("alpha") && text.contains("memory is short"),
        "{text}"
    );
    assert!(text.contains("beta") && text.contains("moved"), "{text}");
    app.press(Action::LockUnsealed);
    testkit::settle(&mut app);
    assert!(app.restart_requested(), "{:?}", app.toast_text());
    assert_eq!(sealed(&app), None);
}

#[test]
fn a_sheet_naming_a_vault_only_for_an_item_moved_to_a_short_one_offers_lock_to_try_again() {
    let mut app = two_open();
    add_notes(&mut app, 0, 40);
    add_notes(&mut app, 1, 1);
    testkit::vault_moved(&mut app, 1, 0);
    memory(&mut app, BETWEEN);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    let text = sheet_text(&mut app);
    assert!(text.contains("beta") && text.contains("moved"), "{text}");
    assert!(app.offers(Action::Lock), "{text}");
    assert!(app.offers(Action::LockUnsealed), "{text}");
}

#[test]
fn a_vault_that_fit_when_the_sheet_was_drawn_and_is_short_at_the_press_is_kept() {
    // Alpha is short and named; Beta fitted when the sheet was drawn. The
    // memory falls before Lock without saving is pressed: Beta was not
    // named, so it stays open with its changes and the sheet comes back
    // naming it.
    let mut app = two_open();
    add_notes(&mut app, 0, 40);
    add_notes(&mut app, 1, 1);
    memory(&mut app, BETWEEN);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    let text = sheet_text(&mut app);
    assert!(text.contains("alpha") && !text.contains("beta"), "{text}");
    memory(&mut app, 150);
    app.press(Action::LockUnsealed);
    testkit::settle(&mut app);
    assert!(!app.restart_requested(), "Beta was lost unnamed");
    let beta = app
        .vaults()
        .open
        .iter()
        .find(|o| o.name.contains("beta"))
        .expect("Beta was closed");
    assert!(beta.changes > 0);
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    let text = sheet_text(&mut app);
    assert!(
        text.contains("beta") && text.contains("memory is short"),
        "{text}"
    );
}

#[test]
fn an_unwritable_vault_the_sheet_did_not_name_is_named_by_the_sheet_that_comes_back() {
    // Alpha is short and named. Beta cannot be written (a seal of it is
    // already under way) and fits, so the first sheet does not name it. The
    // press lets Alpha go, stops at Beta, and the sheet comes back naming
    // Beta, whose press then lets it go: never the same sheet again.
    let mut app = two_open();
    add_notes(&mut app, 0, 40);
    add_notes(&mut app, 1, 1);
    let beta = app.vaults().open[1].id;
    testkit::vault_edit(&mut app, beta, |s| {
        let n = s.uuid_bytes(beta).unwrap();
        let _ = s.seal_job(
            beta,
            &[0; faraday_vault::kdbx::RANDOM_LEN],
            &vec![0; n],
            None,
        );
    });
    crate::common::top_up(&mut app);
    memory(&mut app, BETWEEN);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    let text = sheet_text(&mut app);
    assert!(
        text.contains("alpha.kdbx") && !text.contains("beta.kdbx"),
        "{text}"
    );
    assert!(!text.contains("cannot be written"), "{text}");
    app.press(Action::LockUnsealed);
    testkit::settle(&mut app);
    assert!(!app.restart_requested(), "Beta was lost unnamed");
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    let text = sheet_text(&mut app);
    assert!(text.contains("beta.kdbx: cannot be written"), "{text}");
    app.press(Action::LockUnsealed);
    testkit::settle(&mut app);
    assert!(app.restart_requested(), "{:?}", app.toast_text());
}

#[test]
fn the_lock_sheet_names_the_short_vault_an_item_was_moved_to_not_a_fitting_one() {
    // Beta's items wait for a vault that fits (Gamma, open and not named)
    // and for Alpha, which is short and named: the line names Alpha.
    let mut app = n_open(3);
    add_notes(&mut app, 0, 40);
    add_notes(&mut app, 1, 1);
    testkit::vault_moved(&mut app, 1, 2);
    testkit::vault_moved(&mut app, 1, 0);
    memory(&mut app, BETWEEN);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    let text = sheet_text(&mut app);
    assert!(
        text.contains("beta.kdbx: its item moved to alpha.kdbx"),
        "{text}"
    );
}
