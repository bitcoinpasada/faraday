//! Copying an item to another open vault, as a person does it from the
//! item's page: the sheet of the other open vaults, a row dimmed with its
//! reason, the two dialogs, and what the other vault keeps after a seal.

use crate::common::type_text;
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_core::in_progress::Going;
use faraday_core::strings::EN;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageCommand, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};
use faraday_vault::Record;
use faraday_vault::kdbx::xml::Entry;
use faraday_vault::records::{field, kind};

type Files = Vec<(String, Vec<u8>)>;

const PHRASE: &str = "copy phrase";
const KEYS: usize = 0;
const ENTRIES: usize = 2;

/// A device with a 480x640 panel holding `inbox`.
fn process(inbox: Files) -> Faraday {
    process_kept(inbox, Vec::new())
}

/// [`process`] with what the last power-on kept.
fn process_kept(inbox: Files, kept: Files) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 480,
        height: 640,
        dpi: 286,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    testkit::vault_speed(&mut app, Some(100));
    app.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept,
    });
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x40 + i; 32])));
    }
    app.press(Action::ReduceMotion(true));
    app
}

/// Unlocks place `at` of the Vaults list.
fn unlock(app: &mut Faraday, at: usize) {
    let before = app.vaults().open.len();
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(at)));
    let _ = app.frame();
    app.press(Action::Vault(V::FocusPassphrase));
    type_text(app, PHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if app.vaults().open.len() > before || app.sheet.is_some() {
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

/// A process with every file unlocked, in the order given.
fn opened(files: Vec<(&str, Vec<u8>)>) -> Faraday {
    let n = files.len();
    let mut app = process(files.into_iter().map(|(n, f)| (n.into(), f)).collect());
    for at in 0..n {
        unlock(&mut app, at);
    }
    app
}

fn vault(label: &str, id: u8, records: Vec<Record>) -> Vec<u8> {
    testkit::kdbx_vault(label, PHRASE, id, records).unwrap()
}

fn page(app: &mut Faraday) -> String {
    let _ = app.frame();
    app.drawn_texts().join(" ")
}

fn entry(title: &str, password: &str) -> Record {
    Record::new(kind::ENTRY)
        .with(field::TITLE, title.as_bytes())
        .with(field::PASSWORD, password.as_bytes())
}

fn seed(word: &str) -> Vec<u8> {
    let c = testkit::compute();
    let words = Mnemonic::parse(&c, Language::English, &testkit::test_words(word)).unwrap();
    faraday_vault::records::words_payload(&words).to_vec()
}

fn key(word: &str) -> Record {
    Record::new(kind::KEY).with(field::KEY, &seed(word))
}

/// A record's type and its fields.
type Held = (u8, Vec<(u8, Vec<u8>)>);

/// What open vault `o` holds, as fields, its own label apart.
fn held(app: &Faraday, o: usize) -> Vec<Held> {
    let id = app.vaults().open[o].id;
    app.vaults()
        .store()
        .contents_for_tests(id)
        .iter()
        .filter(|r| r.kind != kind::SLOT_LABEL)
        .map(|r| {
            (
                r.kind,
                r.fields
                    .iter()
                    .map(|f| (f.number, f.bytes.to_vec()))
                    .collect(),
            )
        })
        .collect()
}

/// The sheet of vaults for item `item` of `category` in open vault `from`.
fn sheet_for(app: &mut Faraday, from: usize, category: usize, item: usize) -> String {
    testkit::vault_current(app, from);
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(category)));
    app.press(Action::Vault(V::Item(item)));
    app.press(Action::Vault(V::CopyTo));
    page(app)
}

fn holds() -> &'static str {
    EN.vault_copy_holds
}

#[test]
fn a_seed_and_an_entry_copied_arrive_with_their_fields_and_the_original_stays() {
    let a = vault("Alpha", 1, vec![key("summer"), entry("Mail", "hunter2")]);
    let b = vault("Beta", 2, Vec::new());
    let mut app = opened(vec![("a.kdbx", a), ("b.kdbx", b)]);
    let before = held(&app, 0);
    assert_eq!(before.len(), 2);

    let texts = sheet_for(&mut app, 0, KEYS, 0);
    assert_eq!(app.sheet, Some(Sheet::CopyTo));
    assert!(texts.contains("Beta"), "{texts}");
    app.press(Action::Vault(V::CopyPick(0)));
    assert_eq!(app.sheet, None);
    let seeds: Vec<_> = before
        .iter()
        .filter(|r| r.0 == kind::KEY)
        .cloned()
        .collect();
    assert_eq!(held(&app, 1), seeds);

    sheet_for(&mut app, 0, ENTRIES, 0);
    app.press(Action::Vault(V::CopyPick(0)));
    let (mut copies, mut originals) = (held(&app, 1), held(&app, 0));
    copies.sort();
    originals.sort();
    assert_eq!(copies, originals);
    assert_eq!(held(&app, 0), before, "the original is unchanged");
}

#[test]
fn a_read_only_target_and_a_full_target_are_dimmed_and_a_press_does_nothing() {
    let newer = read_only_file();
    let a = vault("Alpha", 1, vec![entry("Mail", "hunter2")]);
    let full = vault("Full", 3, Vec::new());
    let mut app = opened(vec![("a.kdbx", a), ("b.kdbx", newer), ("c.kdbx", full)]);
    let id = app.vaults().open[2].id;
    let slot = app.vaults().open[2].facts().size.expect("a Faraday seal");
    let mut x = 0x9e37_79b9_7f4a_7c15u64;
    let notes: String = (0..2 * slot)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            char::from(b'A' + u8::try_from(x % 58).unwrap())
        })
        .collect();
    let big = Record::new(kind::ENTRY)
        .with(field::TITLE, b"big")
        .with(field::NOTES, notes.as_bytes());
    testkit::vault_edit(&mut app, id, |s| s.insert(id, big));
    testkit::vault_changed(&mut app, id);
    let (read_only, full_held) = (held(&app, 1), held(&app, 2));

    let texts = sheet_for(&mut app, 0, ENTRIES, 0);
    assert!(texts.contains(EN.vault_error_read_only), "{texts}");
    assert!(texts.contains(EN.vaults_cannot_write), "{texts}");
    for row in 0..2 {
        app.press(Action::Vault(V::CopyPick(row)));
        assert_eq!(app.sheet, Some(Sheet::CopyTo), "row {row} took a press");
    }
    assert_eq!(held(&app, 1), read_only);
    assert_eq!(held(&app, 2), full_held);
}

#[test]
fn a_target_that_writes_but_cannot_take_the_item_says_no_room() {
    let a = vault("Alpha", 1, vec![entry("Mail", "hunter2")]);
    let nearly = vault("Nearly", 3, Vec::new());
    let mut app = opened(vec![("a.kdbx", a), ("c.kdbx", nearly)]);
    let id = app.vaults().open[1].id;
    let c = testkit::compute();
    let mail = Record::new(kind::ENTRY)
        .with(field::TITLE, b"Mail")
        .with(field::PASSWORD, b"hunter2");
    // The most a vault may hold, lowered to just under what the copy needs.
    // Past a limit of nothing the size is counted, not bounded.
    testkit::vault_edit(&mut app, id, |s| s.lower_limit(0));
    let used = app.vaults().records(id).used(&c);
    testkit::vault_edit(&mut app, id, |s| s.lower_limit(used + 20));
    assert!(
        !app.vaults().store().fits(&c, id, &[], &[&mail]),
        "the item fits"
    );
    assert!(
        !app.vaults().records(id).unwritable(&c),
        "the vault still writes"
    );
    let texts = sheet_for(&mut app, 0, ENTRIES, 0);
    assert!(texts.contains(EN.vaults_slot_too_small), "{texts}");
    assert!(!texts.contains(EN.vaults_cannot_write), "{texts}");
}

/// A file of a newer format, which opens to read only.
fn read_only_file() -> Vec<u8> {
    testkit::kdbx_edited(
        "Newer",
        PHRASE,
        9,
        |_| {},
        |xml| {
            let at = xml.find("faraday.format").unwrap();
            let v = at + xml[at..].find("<Value>1</Value>").unwrap();
            format!(
                "{}<Value>99</Value>{}",
                &xml[..v],
                &xml[v + "<Value>1</Value>".len()..]
            )
        },
    )
    .unwrap()
}

#[test]
fn a_target_with_the_same_item_says_so_and_a_different_one_does_not() {
    let a = vault("Alpha", 1, vec![entry("Mail", "hunter2")]);
    let same = vault("Same", 2, vec![entry("Mail", "hunter2")]);
    let other = vault("Other", 3, vec![entry("Mail", "different")]);
    let mut app = opened(vec![("a.kdbx", a), ("b.kdbx", same), ("c.kdbx", other)]);
    let texts = sheet_for(&mut app, 0, ENTRIES, 0);
    assert_eq!(texts.matches(holds()).count(), 1, "{texts}");
    let before = held(&app, 1);
    app.press(Action::Vault(V::CopyPick(0)));
    assert_eq!(held(&app, 1), before, "a dimmed row took a press");
    app.press(Action::Vault(V::CopyPick(1)));
    assert_eq!(held(&app, 2).len(), 2);
}

#[test]
fn a_target_with_the_same_key_under_other_fields_says_so() {
    let a = vault("Alpha", 1, vec![key("summer")]);
    let relabelled = Record::new(kind::KEY)
        .with(field::KEY, &seed("summer"))
        .with(field::KEY_LABEL, b"Cold storage");
    let b = vault("Beta", 2, vec![relabelled]);
    let c = vault("Gamma", 3, vec![key("zebra")]);
    let mut app = opened(vec![("a.kdbx", a), ("b.kdbx", b), ("c.kdbx", c)]);
    let texts = sheet_for(&mut app, 0, KEYS, 0);
    assert_eq!(texts.matches(holds()).count(), 1, "{texts}");
    app.press(Action::Vault(V::CopyPick(0)));
    assert_eq!(held(&app, 1).len(), 1);
    app.press(Action::Vault(V::CopyPick(1)));
    assert_eq!(held(&app, 2).len(), 2);
}

fn passphrase(text: &str) -> Record {
    Record::new(kind::PASSPHRASE)
        .with(field::PASSPHRASE_KEY, &[1, 2, 3, 4])
        .with(field::PASSPHRASE, text.as_bytes())
}

#[test]
fn a_second_passphrase_for_a_key_goes_through_continue_and_cancel() {
    let build = || {
        let a = vault("Alpha", 1, vec![passphrase("two")]);
        let b = vault("Beta", 2, vec![passphrase("one")]);
        opened(vec![("a.kdbx", a), ("b.kdbx", b)])
    };
    let mut app = build();
    sheet_for(&mut app, 0, KEYS, 0);
    app.press(Action::Vault(V::CopyPick(0)));
    assert_eq!(app.sheet, Some(Sheet::SaveAlso));
    assert!(page(&mut app).contains("Beta"));
    app.press(Action::Vault(V::SaveAlsoCancel));
    assert_eq!(held(&app, 1).len(), 1, "Cancel copied");

    let mut app = build();
    sheet_for(&mut app, 0, KEYS, 0);
    app.press(Action::Vault(V::CopyPick(0)));
    app.press(Action::Vault(V::SaveAlsoContinue));
    assert_eq!(app.sheet, None);
    assert_eq!(held(&app, 1).len(), 2);
}

#[test]
fn an_unreadable_item_offers_no_copy() {
    let damaged = testkit::kdbx_edited(
        "Damaged",
        PHRASE,
        11,
        |doc| {
            let mut e = Entry::new([60; 16]);
            e.set_field("Title", "Broken item", false).unwrap();
            e.custom_mut().set("faraday.kind", "rocket").unwrap();
            doc.root_mut().push_entry(e).unwrap();
        },
        |xml| xml,
    )
    .unwrap();
    let b = vault("Beta", 2, Vec::new());
    let mut app = opened(vec![("a.kdbx", damaged), ("b.kdbx", b)]);
    testkit::vault_current(&mut app, 0);
    app.press(Action::Nav(Screen::VaultContents));
    let texts = page(&mut app);
    assert!(
        texts.contains(&EN.vault_not_read.replace("{}", "1")),
        "{texts}"
    );
    assert!(!texts.contains(EN.vault_copy_to), "{texts}");
    app.press(Action::Vault(V::CopyTo));
    assert_eq!(app.sheet, None);
}

#[test]
fn a_copy_survives_a_seal_and_a_reopen() {
    let a = vault("Alpha", 1, vec![entry("Mail", "hunter2")]);
    let b = vault("Beta", 2, Vec::new());
    let mut app = opened(vec![("a.kdbx", a), ("b.kdbx", b)]);
    let want = held(&app, 0);
    sheet_for(&mut app, 0, ENTRIES, 0);
    app.press(Action::Vault(V::CopyPick(0)));
    crate::common::top_up(&mut app);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    let mut written = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { outbox, .. } = c {
            written = outbox;
        }
    }
    assert_eq!(written.len(), 1, "the target's file was sealed");

    let mut app = process(written);
    unlock(&mut app, 0);
    assert_eq!(held(&app, 0), want);
}

/// A KeePass app's file: one entry that carries a tag.
fn tagged() -> Vec<u8> {
    testkit::kdbx_edited(
        "Saved",
        PHRASE,
        7,
        |doc| {
            let mut e = Entry::new([3; 16]);
            e.set_field("Title", "Kept", false).unwrap();
            doc.root_mut().push_entry(e).unwrap();
        },
        |xml| {
            let at = xml.find("</Entry>").unwrap();
            format!("{}<Tags>x</Tags>{}", &xml[..at], &xml[at..])
        },
    )
    .unwrap()
}

#[test]
fn an_entry_saved_by_keepass_with_unread_data_asks_and_cancel_copies_nothing() {
    let b = vault("Beta", 2, Vec::new());
    let mut app = opened(vec![("a.kdbx", tagged()), ("b.kdbx", b)]);
    sheet_for(&mut app, 0, ENTRIES, 0);
    app.press(Action::Vault(V::CopyPick(0)));
    let texts = page(&mut app);
    assert!(texts.contains(EN.vault_copy_unread_head), "{texts}");
    app.press(Action::Vault(V::CopyCancel));
    assert_eq!(app.sheet, None);
    assert!(held(&app, 1).is_empty());

    sheet_for(&mut app, 0, ENTRIES, 0);
    app.press(Action::Vault(V::CopyPick(0)));
    app.press(Action::Vault(V::CopyGo));
    assert_eq!(held(&app, 1).len(), 1);
}

// ---- Move ----

const MIN: u64 = 60_000;

/// The item page of item `item` of `category` in open vault `from`, and
/// its Move sheet.
fn move_sheet(app: &mut Faraday, from: usize, category: usize, item: usize) -> String {
    testkit::vault_current(app, from);
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(category)));
    app.press(Action::Vault(V::Item(item)));
    app.press(Action::Vault(V::MoveTo));
    page(app)
}

/// The files the app wrote to the outbox, in the order written, after a
/// lock of every vault.
fn lock_all(app: &mut Faraday) -> Files {
    crate::common::top_up(app);
    app.press(Action::Lock);
    testkit::settle(app);
    let mut written = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { outbox, .. } = c {
            written = outbox;
        }
    }
    written
}

/// Alpha holding "Mail" moved into the empty Beta, the dialog answered.
fn moved() -> (Faraday, Vec<u8>, Vec<u8>) {
    let a = vault("Alpha", 1, vec![entry("Mail", "hunter2")]);
    let b = vault("Beta", 2, Vec::new());
    let mut app = opened(vec![("a.kdbx", a.clone()), ("b.kdbx", b.clone())]);
    move_sheet(&mut app, 0, ENTRIES, 0);
    app.press(Action::Vault(V::CopyPick(0)));
    app.press(Action::Vault(V::CopyGo));
    (app, a, b)
}

#[test]
fn the_move_dialog_names_both_vaults_and_cancel_changes_nothing() {
    let a = vault("Alpha", 1, vec![entry("Mail", "hunter2")]);
    let b = vault("Beta", 2, Vec::new());
    let mut app = opened(vec![("a.kdbx", a), ("b.kdbx", b)]);
    let (before_a, before_b) = (held(&app, 0), held(&app, 1));
    let sheet = move_sheet(&mut app, 0, ENTRIES, 0);
    assert!(sheet.contains(EN.vault_move_title), "{sheet}");
    app.press(Action::Vault(V::CopyPick(0)));
    let texts = page(&mut app);
    assert!(texts.contains(EN.vault_move_head), "{texts}");
    assert!(texts.contains("Beta") && texts.contains("Alpha"), "{texts}");
    assert!(texts.contains(EN.vault_move_go), "{texts}");
    app.press(Action::Vault(V::CopyCancel));
    assert_eq!(app.sheet, None);
    assert_eq!((held(&app, 0), held(&app, 1)), (before_a, before_b));
}

#[test]
fn the_move_dialog_says_what_a_keepass_apps_data_does() {
    let b = vault("Beta", 2, Vec::new());
    let mut app = opened(vec![("a.kdbx", tagged()), ("b.kdbx", b)]);
    move_sheet(&mut app, 0, ENTRIES, 0);
    app.press(Action::Vault(V::CopyPick(0)));
    let texts = page(&mut app);
    assert!(
        texts.contains("do not go with it and are gone from"),
        "{texts}"
    );
}

#[test]
fn a_moved_item_is_in_the_target_and_not_the_original_after_a_seal_and_reopen() {
    let (mut app, _, _) = moved();
    let want = held(&app, 1);
    assert_eq!(want.len(), 1);
    assert!(held(&app, 0).is_empty(), "the original still holds it");
    let written = lock_all(&mut app);
    assert_eq!(written.len(), 2, "both vaults were sealed");

    let mut app = process(written);
    unlock(&mut app, 0);
    unlock(&mut app, 1);
    let (x, y) = (held(&app, 0), held(&app, 1));
    assert!(
        (x.is_empty() && y == want) || (y.is_empty() && x == want),
        "one vault holds the item, the other nothing"
    );
}

#[test]
fn a_move_is_not_offered_from_a_read_only_vault() {
    let b = vault("Beta", 2, Vec::new());
    let mut app = opened(vec![("a.kdbx", read_only_file()), ("b.kdbx", b)]);
    testkit::vault_current(&mut app, 0);
    app.press(Action::Nav(Screen::VaultContents));
    let texts = page(&mut app);
    assert!(!texts.contains(EN.vault_move_to), "{texts}");
    app.press(Action::Vault(V::MoveTo));
    assert_eq!(app.sheet, None);
}

#[test]
fn a_moved_second_passphrase_leaves_the_original_only_on_continue() {
    let build = || {
        let a = vault("Alpha", 1, vec![passphrase("two")]);
        let b = vault("Beta", 2, vec![passphrase("one")]);
        opened(vec![("a.kdbx", a), ("b.kdbx", b)])
    };
    let mut app = build();
    move_sheet(&mut app, 0, KEYS, 0);
    app.press(Action::Vault(V::CopyPick(0)));
    app.press(Action::Vault(V::CopyGo));
    assert_eq!(app.sheet, Some(Sheet::SaveAlso));
    assert_eq!(held(&app, 0).len(), 1, "it left before the answer");
    app.press(Action::Vault(V::SaveAlsoCancel));
    assert_eq!((held(&app, 0).len(), held(&app, 1).len()), (1, 1));

    let mut app = build();
    move_sheet(&mut app, 0, KEYS, 0);
    app.press(Action::Vault(V::CopyPick(0)));
    app.press(Action::Vault(V::CopyGo));
    app.press(Action::Vault(V::SaveAlsoContinue));
    assert_eq!((held(&app, 0).len(), held(&app, 1).len()), (0, 2));
}

#[test]
fn the_original_cannot_be_locked_alone_until_the_target_is() {
    let (mut app, _, _) = moved();
    crate::common::top_up(&mut app);
    assert!(!testkit::vault_lock_one(&mut app, 0));
    assert_eq!(app.vaults().open.len(), 2, "the original closed");
    let texts = page(&mut app);
    assert!(texts.contains("Lock Beta first"), "{texts}");

    testkit::vault_lock_one(&mut app, 1);
    testkit::settle(&mut app);
    assert_eq!(app.vaults().open.len(), 1, "the target did not lock");
    testkit::vault_lock_one(&mut app, 0);
    testkit::settle(&mut app);
    assert!(app.vaults().open.is_empty(), "the original did not lock");
}

#[test]
fn a_lock_of_all_seals_the_target_before_the_original() {
    let (mut app, _, _) = moved();
    // The target is the second vault open: the order of a plain lock puts
    // the original first, so the files tell which came first.
    let written = lock_all(&mut app);
    assert_eq!(written.len(), 2);
    let mut check = process(vec![written[0].clone()]);
    unlock(&mut check, 0);
    assert_eq!(
        held(&check, 0).len(),
        1,
        "the first file written is the target's"
    );
}

#[test]
fn an_idle_lock_with_the_target_unable_to_seal_lets_the_original_go_with_its_item() {
    let (mut app, a, _) = moved();
    // The target takes more than its file can hold.
    let id = app.vaults().open[1].id;
    let slot = app.vaults().open[1].facts().size.expect("a Faraday seal");
    let mut x = 0x9e37_79b9_7f4a_7c15u64;
    let notes: String = (0..2 * slot)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            char::from(b'A' + u8::try_from(x % 58).unwrap())
        })
        .collect();
    let big = Record::new(kind::ENTRY)
        .with(field::TITLE, b"big")
        .with(field::NOTES, notes.as_bytes());
    testkit::vault_edit(&mut app, id, |s| s.insert(id, big));
    testkit::vault_changed(&mut app, id);
    crate::common::top_up(&mut app);
    app.event(Event::Tick { now_ms: 0 });
    app.event(Event::Tick { now_ms: 30 * MIN });
    testkit::settle(&mut app);
    assert!(app.restart_requested(), "the idle lock did not go ahead");
    let mut written = None;
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { outbox, .. } = c {
            written = Some(outbox);
        }
    }
    let written = written.expect("the lock saved");
    assert!(written.is_empty(), "a file was sealed: {}", written.len());

    // The original's last sealed file is the one it was opened from.
    let mut app = process(vec![("a.kdbx".into(), a)]);
    unlock(&mut app, 0);
    assert_eq!(held(&app, 0).len(), 1, "the item is gone from the original");
}

// ---------------------------------------------------------------------
// A vault locked, or not yet made (`docs/VAULT.md` §7.2).
// ---------------------------------------------------------------------

/// Alpha (a seed, an entry) open, and Beta known and locked.
fn with_locked_beta(beta: Vec<Record>) -> Faraday {
    let a = vault("Alpha", 1, vec![key("summer"), entry("Mail", "hunter2")]);
    let b = vault("Beta", 2, beta);
    let mut app = opened(vec![("a.kdbx", a), ("b.kdbx", b)]);
    lock_one(&mut app, 1);
    app
}

fn lock_one(app: &mut Faraday, at: usize) {
    crate::common::top_up(app);
    testkit::vault_lock_one(app, at);
    testkit::settle(app);
}

/// Types `phrase` at the Unlock screen and waits for the answer.
fn answer(app: &mut Faraday, phrase: &str) {
    let before = app.vaults().open.len();
    let _ = app.frame();
    app.press(Action::Vault(V::FocusPassphrase));
    type_text(app, phrase);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if app.vaults().open.len() > before
            || app.sheet.is_some()
            || app.vaults().unlock_error.is_some()
        {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

/// The sheet of item `item` of `category` in the vault `from`, with the
/// locked Beta's row pressed.
fn press_unlock_row(app: &mut Faraday, category: usize, item: usize, mv: bool) {
    let texts = if mv {
        move_sheet(app, 0, category, item)
    } else {
        sheet_for(app, 0, category, item)
    };
    assert!(texts.contains("Unlock "), "{texts}");
    assert!(texts.contains(EN.vault_copy_new), "{texts}");
    app.press(Action::Vault(V::CopyPick(0)));
    assert_eq!(app.screen, Screen::Unlock);
}

fn seeds_of(app: &Faraday, o: usize) -> Vec<Held> {
    held(app, o)
        .into_iter()
        .filter(|r| r.0 == kind::KEY)
        .collect()
}

#[test]
fn an_item_copied_into_a_locked_vault_arrives_after_its_unlock() {
    let mut app = with_locked_beta(Vec::new());
    let want = seeds_of(&app, 0);
    press_unlock_row(&mut app, KEYS, 0, false);
    answer(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 2);
    assert_eq!(held(&app, 1), want);
    assert_eq!(app.sheet, None);
    let texts = page(&mut app);
    assert!(texts.contains("Copied to Beta"), "{texts}");
    assert_eq!(held(&app, 0).len(), 2, "the original is unchanged");
}

#[test]
fn backing_out_of_the_unlock_makes_nothing_and_leaves_nothing_pending() {
    let mut app = with_locked_beta(Vec::new());
    press_unlock_row(&mut app, KEYS, 0, false);
    app.press(Action::Vault(V::Back));
    testkit::settle(&mut app);
    // Beta unlocked later from the list takes nothing.
    unlock(&mut app, 1);
    assert!(held(&app, 1).is_empty(), "an item came in later");
    assert_eq!(app.sheet, None);
}

#[test]
fn a_wrong_passphrase_can_be_retried_and_the_item_still_comes() {
    let mut app = with_locked_beta(Vec::new());
    let want = seeds_of(&app, 0);
    press_unlock_row(&mut app, KEYS, 0, false);
    answer(&mut app, "not it");
    assert_eq!(app.vaults().open.len(), 1);
    assert!(app.vaults().unlock_error.is_some());
    answer(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 2);
    assert_eq!(held(&app, 1), want);
}

#[test]
fn a_locked_vault_that_holds_the_item_says_so_and_nothing_is_made() {
    let mut app = with_locked_beta(vec![entry("Mail", "hunter2")]);
    press_unlock_row(&mut app, ENTRIES, 0, false);
    answer(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 2);
    let texts = page(&mut app);
    assert!(texts.contains(EN.vault_copy_holds), "{texts}");
    assert!(texts.contains("Beta"), "{texts}");
    assert_eq!(held(&app, 1).len(), 1, "a second copy was made");
    assert_eq!(app.sheet, None);
}

#[test]
fn a_read_only_locked_vault_says_so_and_nothing_is_made() {
    let a = vault("Alpha", 1, vec![entry("Mail", "hunter2")]);
    let mut app = opened(vec![("a.kdbx", a), ("b.kdbx", read_only_file())]);
    lock_one(&mut app, 1);
    press_unlock_row(&mut app, ENTRIES, 0, false);
    answer(&mut app, PHRASE);
    let texts = page(&mut app);
    assert!(texts.contains(EN.vault_error_read_only), "{texts}");
    assert!(held(&app, 1).is_empty());
}

#[test]
fn the_original_locked_while_the_item_waits_makes_nothing() {
    let mut app = with_locked_beta(Vec::new());
    press_unlock_row(&mut app, KEYS, 0, false);
    // The original goes (the only other vault open), then Beta unlocks.
    lock_one(&mut app, 0);
    assert!(app.vaults().open.is_empty());
    unlock(&mut app, 1);
    assert!(held(&app, 0).is_empty(), "an item came from a closed vault");
    assert_eq!(app.sheet, None);
}

#[test]
fn a_moved_item_in_a_locked_vault_leaves_the_original_only_after_the_dialog() {
    let mut app = with_locked_beta(Vec::new());
    let want = vec![entry("Mail", "hunter2")];
    let _ = want;
    press_unlock_row(&mut app, ENTRIES, 0, true);
    answer(&mut app, PHRASE);
    assert_eq!(app.sheet, Some(Sheet::CopyTo));
    let texts = page(&mut app);
    assert!(texts.contains(EN.vault_move_head), "{texts}");
    assert!(texts.contains("Beta") && texts.contains("Alpha"), "{texts}");
    assert!(held(&app, 1).is_empty(), "moved before the dialog");
    assert_eq!(held(&app, 0).len(), 2);
    app.press(Action::Vault(V::CopyGo));
    assert_eq!(held(&app, 1).len(), 1);
    assert_eq!(held(&app, 0).len(), 1, "the original still holds it");

    let (alpha, beta) = (held(&app, 0), held(&app, 1));
    let written = lock_all(&mut app);
    let mut app = process(written);
    unlock(&mut app, 0);
    unlock(&mut app, 1);
    let (x, y) = (held(&app, 0), held(&app, 1));
    assert!(
        (x == alpha && y == beta) || (x == beta && y == alpha),
        "the sealed vaults differ from what they held"
    );
}

#[test]
fn a_move_cancelled_after_the_unlock_changes_nothing() {
    let mut app = with_locked_beta(Vec::new());
    press_unlock_row(&mut app, ENTRIES, 0, true);
    answer(&mut app, PHRASE);
    app.press(Action::Vault(V::CopyCancel));
    assert_eq!(app.sheet, None);
    assert!(held(&app, 1).is_empty());
    assert_eq!(held(&app, 0).len(), 2);
}

/// Alpha unlocked in a power-on after Beta was sealed twice, with Beta's
/// first copy in Files: unlocking Beta stops at the older-copy dialog.
fn with_older_beta() -> Faraday {
    let a = vault("Alpha", 1, vec![key("summer")]);
    let b = vault("Beta", 2, Vec::new());
    let mut app = opened(vec![("a.kdbx", a.clone()), ("b.kdbx", b.clone())]);
    sheet_for(&mut app, 0, KEYS, 0);
    app.press(Action::Vault(V::CopyPick(0)));
    crate::common::top_up(&mut app);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    let mut kept = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { kept: k, .. } = c {
            kept = k;
        }
    }
    let mut app = process_kept(vec![("a.kdbx".into(), a), ("b.kdbx".into(), b)], kept);
    unlock(&mut app, 0);
    app
}

#[test]
fn keep_locked_at_the_older_copy_dialog_drops_the_waiting_item_and_open_goes_on() {
    let mut app = with_older_beta();
    press_unlock_row(&mut app, KEYS, 0, false);
    answer(&mut app, PHRASE);
    assert_eq!(app.sheet, Some(Sheet::OlderCopy));
    app.press(Action::Vault(V::OlderBack));
    assert_eq!(app.vaults().open.len(), 1, "the older copy stayed open");
    testkit::settle(&mut app);
    // Opened later, nothing comes in.
    unlock(&mut app, 1);
    app.press(Action::Vault(V::OlderOpen));
    assert!(held(&app, 1).is_empty(), "the dropped item came later");

    let mut app = with_older_beta();
    press_unlock_row(&mut app, KEYS, 0, false);
    answer(&mut app, PHRASE);
    app.press(Action::Vault(V::OlderOpen));
    assert_eq!(seeds_of(&app, 1), seeds_of(&app, 0));
    assert_eq!(held(&app, 1).len(), 1);
}

/// Pressing **New vault with this item** and making the vault.
fn make_new(app: &mut Faraday, first: usize) {
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(app, PHRASE);
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(app, PHRASE);
    let before = app.vaults().open.len();
    app.press(Action::Vault(V::CGo));
    for t in 1..120u64 {
        if app.vaults().open.len() > before {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), before + 1, "no vault was made");
    let _ = first;
}

/// The ordinary Create finished with a passphrase typed twice: whether
/// the vault it made was opened at once, as one made for an item is.
fn made_plain_opens(app: &mut Faraday) -> bool {
    crate::common::top_up(app);
    let before = app.vaults().open.len();
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(app, PHRASE);
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(app, PHRASE);
    app.press(Action::Vault(V::CGo));
    for t in 1..120u64 {
        if app.vaults().create.is_none() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert!(app.vaults().create.is_none(), "no vault was made");
    app.vaults().open.len() > before || app.sheet.is_some()
}

/// The open vault's label and what each file in `written` holds.
fn read_back(written: Files) -> Vec<(String, Vec<Held>)> {
    written
        .into_iter()
        .map(|f| {
            let mut app = process(vec![f]);
            unlock(&mut app, 0);
            (app.vaults().open[0].name.clone(), held(&app, 0))
        })
        .collect()
}

#[test]
fn a_new_vault_with_a_seed_or_an_entry_holds_it_after_a_seal_and_reopen() {
    for (category, kind_of) in [(KEYS, kind::KEY), (ENTRIES, kind::ENTRY)] {
        let a = vault("Alpha", 1, vec![key("summer"), entry("Mail", "hunter2")]);
        let mut app = opened(vec![("a.kdbx", a)]);
        let want: Vec<Held> = held(&app, 0)
            .into_iter()
            .filter(|r| r.0 == kind_of)
            .collect();
        let texts = sheet_for(&mut app, 0, category, 0);
        assert!(texts.contains(EN.vault_copy_new), "{texts}");
        app.press(Action::Vault(V::CopyPick(0)));
        assert_eq!(app.screen, Screen::CreateVault);
        make_new(&mut app, 1);
        assert_eq!(held(&app, 1), want, "the new vault lacks the item");
        assert_eq!(held(&app, 0).len(), 2, "a copy changed the original");
        assert_eq!(app.sheet, None);

        let written = lock_all(&mut app);
        let read = read_back(written);
        assert!(
            read.iter().any(|(_, h)| *h == want),
            "no sealed file holds the item"
        );
    }
}

#[test]
fn backing_out_of_create_makes_no_vault_and_nothing_waits() {
    let a = vault("Alpha", 1, vec![entry("Mail", "hunter2")]);
    let mut app = opened(vec![("a.kdbx", a)]);
    sheet_for(&mut app, 0, ENTRIES, 0);
    app.press(Action::Vault(V::CopyPick(0)));
    app.press(Action::Vault(V::Back));
    testkit::settle(&mut app);
    assert_eq!(app.vaults().open.len(), 1);
    // A vault made by the ordinary Create takes no item.
    app.press(Action::Vault(V::Create));
    assert!(
        !made_plain_opens(&mut app),
        "an item came into an ordinary vault"
    );
}

#[test]
fn a_moved_item_in_a_new_vault_is_sealed_there_before_the_original() {
    let a = vault("Alpha", 1, vec![entry("Mail", "hunter2")]);
    let mut app = opened(vec![("a.kdbx", a)]);
    let want = held(&app, 0);
    move_sheet(&mut app, 0, ENTRIES, 0);
    app.press(Action::Vault(V::CopyPick(0)));
    make_new(&mut app, 1);
    assert_eq!(app.sheet, Some(Sheet::CopyTo));
    let texts = page(&mut app);
    assert!(texts.contains(EN.vault_move_head), "{texts}");
    assert!(held(&app, 0) == want, "moved before the dialog");
    app.press(Action::Vault(V::CopyGo));
    assert_eq!(held(&app, 1), want);
    assert!(held(&app, 0).is_empty());

    crate::common::top_up(&mut app);
    assert!(
        !testkit::vault_lock_one(&mut app, 0),
        "the original locked alone"
    );
    let written = lock_all(&mut app);
    let read = read_back(written);
    let with = read
        .iter()
        .position(|(_, h)| *h == want)
        .expect("no file holds it");
    let original = read
        .iter()
        .position(|(_, h)| h.is_empty() && *h != want)
        .expect("no original");
    assert!(with < original, "the original was sealed first");
}

// ---------------------------------------------------------------------
// What keeps a wait alive, a changed original, and the sizes offered.
// ---------------------------------------------------------------------

#[test]
fn leaving_the_unlock_or_create_by_the_bar_drops_the_item_for_good() {
    for away in [Screen::Vaults, Screen::VaultContents, Screen::Tools] {
        let mut app = with_locked_beta(Vec::new());
        press_unlock_row(&mut app, KEYS, 0, false);
        app.press(Action::Nav(away));
        testkit::settle(&mut app);
        unlock(&mut app, 1);
        assert!(
            held(&app, 1).is_empty(),
            "an item came after leaving for {away:?}"
        );
        assert_eq!(app.sheet, None);
    }
    for away in [Screen::Vaults, Screen::VaultContents, Screen::Tools] {
        let a = vault("Alpha", 1, vec![entry("Mail", "hunter2")]);
        let mut app = opened(vec![("a.kdbx", a)]);
        sheet_for(&mut app, 0, ENTRIES, 0);
        app.press(Action::Vault(V::CopyPick(0)));
        app.press(Action::Nav(away));
        testkit::settle(&mut app);
        // Back through the in-progress row: an ordinary Create, no item.
        app.press(Action::Resume(Going::Vault));
        assert_eq!(app.screen, Screen::CreateVault);
        assert!(
            !made_plain_opens(&mut app),
            "an item came after leaving for {away:?}"
        );
    }
}

#[test]
fn an_item_waiting_for_a_new_vault_does_not_go_into_a_backup_flows_vault() {
    let a = vault("Alpha", 1, vec![entry("Mail", "hunter2")]);
    let mut app = process(vec![("a.kdbx".into(), a)]);
    crate::common::keep_time(&mut app);
    crate::common::start(&mut app, faraday_core::backup_flow::Kind::Gpg);
    assert_eq!(app.screen, Screen::BackupFlow);
    unlock(&mut app, 0);
    sheet_for(&mut app, 0, ENTRIES, 0);
    app.press(Action::Vault(V::CopyPick(0)));
    assert_eq!(app.screen, Screen::CreateVault);
    // Away by the bar, into the flow, and its own Create vault.
    app.press(Action::Resume(Going::Flow(0)));
    assert_eq!(app.screen, Screen::BackupFlow);
    app.press(Action::Flow(
        faraday_core::backup_flow::steps::FlowAction::CreateVault(0),
    ));
    assert!(
        made_plain_opens(&mut app),
        "the flow's vault was not made and opened"
    );
    assert!(
        held(&app, 1).is_empty(),
        "the item went into the flow's vault"
    );
    assert_eq!(app.flow().unwrap().live.creating, None);
}

#[test]
fn a_moved_item_stays_when_the_original_changed_under_the_dialog() {
    let a = vault("Alpha", 1, vec![passphrase("two")]);
    let b = vault("Beta", 2, vec![passphrase("one")]);
    let mut app = opened(vec![("a.kdbx", a), ("b.kdbx", b)]);
    move_sheet(&mut app, 0, KEYS, 0);
    app.press(Action::Vault(V::CopyPick(0)));
    app.press(Action::Vault(V::CopyGo));
    assert_eq!(app.sheet, Some(Sheet::SaveAlso));
    let id = app.vaults().open[0].id;
    testkit::vault_edit(&mut app, id, |s| s.insert(id, entry("Other", "x")));
    testkit::vault_changed(&mut app, id);
    app.press(Action::Vault(V::SaveAlsoContinue));
    assert_eq!(held(&app, 1).len(), 2, "the copy was not made");
    assert_eq!(held(&app, 0).len(), 2, "an item left the changed original");
    let said = app.toast_text().unwrap_or_default();
    assert!(said.contains("Copied to Beta"), "{said}");
    assert!(!said.contains("Moved"), "{said}");
}

/// An entry with a note and a URL of `len` bytes each, which do not
/// compress.
fn noted(len: usize) -> Record {
    let mut x = 0x9e37_79b9_7f4a_7c15u64;
    let mut text = || -> String {
        (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                char::from(b'A' + u8::try_from(x % 58).unwrap())
            })
            .collect()
    };
    let (notes, url) = (text(), text());
    Record::new(kind::ENTRY)
        .with(field::TITLE, b"big")
        .with(field::NOTES, notes.as_bytes())
        .with(field::URL, url.as_bytes())
}

#[test]
fn a_new_vault_for_a_large_item_holds_it_whatever_its_size() {
    let record = noted(60_000);
    let a = vault("Alpha", 1, vec![entry("Mail", "hunter2")]);
    let mut app = opened(vec![("a.kdbx", a)]);
    let id = app.vaults().open[0].id;
    testkit::vault_edit(&mut app, id, |s| s.insert(id, record.clone()));
    testkit::vault_changed(&mut app, id);

    let want: Vec<Held> = held(&app, 0).into_iter().skip(1).collect();
    move_sheet(&mut app, 0, ENTRIES, 1);
    app.press(Action::Vault(V::CopyPick(0)));
    assert_eq!(app.screen, Screen::CreateVault);
    make_new(&mut app, 1);
    // A move: the original could not be sealed with the item.
    app.press(Action::Vault(V::CopyGo));
    assert_eq!(held(&app, 1), want);

    // The next seal writes it, into a file that grew to hold it.
    let written = lock_all(&mut app);
    let read = read_back(written);
    let bytes = |h: &[Held]| -> usize {
        h.iter()
            .map(|r| r.1.iter().map(|f| f.1.len()).sum::<usize>())
            .sum()
    };
    assert!(
        read.iter()
            .any(|(_, h)| h.len() == 1 && bytes(h) == bytes(&want)),
        "the new vault's file does not hold the item"
    );
}

#[test]
fn an_item_larger_than_a_new_vault_arrives_in_a_vault_named_with_the_longest_escape() {
    let a = vault("Alpha", 1, vec![entry("Mail", "hunter2")]);
    let mut app = opened(vec![("a.kdbx", a)]);
    // The document writes each `&` of a name as five bytes.
    let escaped = "&".repeat(64);
    let id = app.vaults().open[0].id;
    testkit::vault_edit(&mut app, id, |s| s.insert(id, noted(20_000)));
    testkit::vault_changed(&mut app, id);
    let want: Vec<Held> = held(&app, 0).into_iter().skip(1).collect();
    move_sheet(&mut app, 0, ENTRIES, 1);
    app.press(Action::Vault(V::CopyPick(0)));
    assert_eq!(app.screen, Screen::CreateVault);
    app.press(Action::Vault(V::CName));
    type_text(&mut app, &escaped);
    make_new(&mut app, 1);
    app.press(Action::Vault(V::CopyGo));
    assert_eq!(held(&app, 1), want, "the item did not arrive");
}

// ---------------------------------------------------------------------
// What a seed keeps (`docs/VAULT.md` §7): its codex32 set and BIP-85 origin.
// ---------------------------------------------------------------------

const SET: [u8; 9] = [0, b'c', b'a', b's', b'h', 48, 0, b's', 0];
const ORIGIN: [u8; 14] = [1, 2, 3, 4, 39, 0, 0, 0, 0, 24, 0, 0, 0, 0];

/// A seed with a codex32 set and a BIP-85 origin.
fn rich() -> Record {
    key("summer")
        .with(field::KEY_CODEX32, &SET)
        .with(field::KEY_CHILD, &ORIGIN)
}

/// What a seed of `o` holds, without field `n`.
fn seeds_without(app: &Faraday, o: usize, n: &[u8]) -> Vec<Held> {
    seeds_of(app, o)
        .into_iter()
        .map(|(k, f)| (k, f.into_iter().filter(|x| !n.contains(&x.0)).collect()))
        .collect()
}

fn two_vaults() -> Faraday {
    let a = vault("Alpha", 1, vec![rich()]);
    let b = vault("Beta", 2, Vec::new());
    opened(vec![("a.kdbx", a), ("b.kdbx", b)])
}

/// The target's seeds after it is sealed and opened again.
fn reopened_target(app: &mut Faraday) -> Vec<Held> {
    // Alpha's seed has three fields; the copy has fewer.
    read_back(lock_all(app))
        .into_iter()
        .flat_map(|(_, h)| h)
        .filter(|r| r.0 == kind::KEY && r.1.len() < 3)
        .collect()
}

#[test]
fn an_unticked_codex32_set_is_left_out_of_the_copy_and_the_words_stay() {
    let mut app = two_vaults();
    let original = held(&app, 0);
    let texts = sheet_for(&mut app, 0, KEYS, 0);
    assert!(texts.contains(EN.vault_copy_keep_codex32), "{texts}");
    assert!(texts.contains(EN.vault_copy_keep_child), "{texts}");
    app.press(Action::Vault(V::CopyKeep(0)));
    app.press(Action::Vault(V::CopyPick(0)));
    let want = seeds_without(&app, 0, &[field::KEY_CODEX32]);
    assert_eq!(seeds_of(&app, 1), want);
    assert!(seeds_of(&app, 1)[0].1.iter().any(|f| f.0 == field::KEY));
    assert!(
        seeds_of(&app, 1)[0]
            .1
            .iter()
            .any(|f| f.0 == field::KEY_CHILD)
    );
    assert_eq!(held(&app, 0), original, "the original is unchanged");
    assert_eq!(reopened_target(&mut app), want);
}

#[test]
fn an_unticked_bip85_origin_is_left_out_of_the_copy() {
    let mut app = two_vaults();
    sheet_for(&mut app, 0, KEYS, 0);
    app.press(Action::Vault(V::CopyKeep(1)));
    app.press(Action::Vault(V::CopyPick(0)));
    let want = seeds_without(&app, 0, &[field::KEY_CHILD]);
    assert_eq!(seeds_of(&app, 1), want);
    assert!(
        seeds_of(&app, 1)[0]
            .1
            .iter()
            .any(|f| f.0 == field::KEY_CODEX32)
    );
    assert_eq!(reopened_target(&mut app), want);
}

#[test]
fn ticked_both_keeps_both_and_a_tick_can_be_put_back() {
    let mut app = two_vaults();
    let original = seeds_of(&app, 0);
    sheet_for(&mut app, 0, KEYS, 0);
    app.press(Action::Vault(V::CopyKeep(0)));
    app.press(Action::Vault(V::CopyKeep(0)));
    app.press(Action::Vault(V::CopyPick(0)));
    assert_eq!(seeds_of(&app, 1), original);
}

#[test]
fn a_seed_with_neither_and_an_entry_show_no_tick() {
    let a = vault("Alpha", 1, vec![key("summer"), entry("Mail", "x")]);
    let b = vault("Beta", 2, Vec::new());
    let mut app = opened(vec![("a.kdbx", a), ("b.kdbx", b)]);
    for category in [KEYS, ENTRIES] {
        let texts = sheet_for(&mut app, 0, category, 0);
        assert!(!texts.contains(EN.vault_copy_keep_codex32), "{texts}");
        assert!(!texts.contains(EN.vault_copy_keep_child), "{texts}");
        app.press(Action::Vault(V::CopyCancel));
    }
}

#[test]
fn a_seed_with_only_a_set_shows_only_that_tick() {
    let a = vault(
        "Alpha",
        1,
        vec![key("summer").with(field::KEY_CODEX32, &SET)],
    );
    let b = vault("Beta", 2, Vec::new());
    let mut app = opened(vec![("a.kdbx", a), ("b.kdbx", b)]);
    let texts = sheet_for(&mut app, 0, KEYS, 0);
    assert!(texts.contains(EN.vault_copy_keep_codex32), "{texts}");
    assert!(!texts.contains(EN.vault_copy_keep_child), "{texts}");
}

#[test]
fn the_choice_survives_an_unlock_of_the_target() {
    let a = vault("Alpha", 1, vec![rich()]);
    let b = vault("Beta", 2, Vec::new());
    let mut app = opened(vec![("a.kdbx", a), ("b.kdbx", b)]);
    lock_one(&mut app, 1);
    sheet_for(&mut app, 0, KEYS, 0);
    app.press(Action::Vault(V::CopyKeep(0)));
    app.press(Action::Vault(V::CopyPick(0)));
    assert_eq!(app.screen, Screen::Unlock);
    answer(&mut app, PHRASE);
    assert_eq!(
        seeds_of(&app, 1),
        seeds_without(&app, 0, &[field::KEY_CODEX32])
    );
}

#[test]
fn the_choice_survives_a_new_vault_for_the_item() {
    let a = vault("Alpha", 1, vec![rich()]);
    let mut app = opened(vec![("a.kdbx", a)]);
    sheet_for(&mut app, 0, KEYS, 0);
    app.press(Action::Vault(V::CopyKeep(1)));
    app.press(Action::Vault(V::CopyPick(0)));
    assert_eq!(app.screen, Screen::CreateVault);
    make_new(&mut app, 1);
    assert_eq!(
        seeds_of(&app, 1),
        seeds_without(&app, 0, &[field::KEY_CHILD])
    );
}

#[test]
fn a_move_with_a_tick_off_says_the_data_is_gone_from_both() {
    let mut app = two_vaults();
    move_sheet(&mut app, 0, KEYS, 0);
    app.press(Action::Vault(V::CopyKeep(0)));
    app.press(Action::Vault(V::CopyPick(0)));
    let texts = page(&mut app);
    assert!(texts.contains("codex32 set does not go with it"), "{texts}");
    assert!(!texts.contains("BIP-85 origin does not go"), "{texts}");
}

// ---------------------------------------------------------------------
// Several moves from one vault: the order of the seals
// (`docs/VAULT.md` §7.2).
// ---------------------------------------------------------------------

/// Alpha holding "Mail" and "Post", with empty Beta and Gamma open.
fn three() -> (Faraday, Vec<u8>) {
    let a = vault(
        "Alpha",
        1,
        vec![entry("Mail", "hunter2"), entry("Post", "swordfish")],
    );
    let b = vault("Beta", 2, Vec::new());
    let c = vault("Gamma", 3, Vec::new());
    let app = opened(vec![("a.kdbx", a.clone()), ("b.kdbx", b), ("c.kdbx", c)]);
    (app, a)
}

/// The first entry of open vault `from` moved to open vault `to`.
fn move_first(app: &mut Faraday, from: usize, to: usize) {
    move_sheet(app, from, ENTRIES, 0);
    let row = (0..3).filter(|&o| o != from).position(|o| o == to).unwrap();
    app.press(Action::Vault(V::CopyPick(row)));
    app.press(Action::Vault(V::CopyGo));
}

#[test]
fn an_item_cannot_be_moved_back_round_a_ring_of_two_moves() {
    let (mut app, _) = three();
    move_first(&mut app, 0, 1);
    move_first(&mut app, 0, 2);
    // Gamma waits for nothing; Alpha waits for Beta and for Gamma: an item
    // moved from Gamma to Alpha would leave Alpha and Gamma each waiting
    // for the other.
    let texts = move_sheet(&mut app, 2, ENTRIES, 0);
    assert!(texts.contains("Lock Gamma first"), "{texts}");
    let before = held(&app, 0);
    app.press(Action::Vault(V::CopyPick(0)));
    assert_eq!(held(&app, 0), before, "a dimmed row took a press");
}

#[test]
fn a_chain_of_moves_seals_the_last_target_first() {
    let a = vault("Alpha", 1, vec![entry("Mail", "hunter2")]);
    let b = vault("Beta", 2, Vec::new());
    let c = vault("Gamma", 3, Vec::new());
    let mut app = opened(vec![("a.kdbx", a), ("b.kdbx", b), ("c.kdbx", c)]);
    move_first(&mut app, 0, 1);
    move_first(&mut app, 1, 2);
    let written = lock_all(&mut app);
    assert_eq!(written.len(), 3);
    let mut check = process(vec![written[0].clone()]);
    unlock(&mut check, 0);
    assert_eq!(
        held(&check, 0).len(),
        1,
        "the first file written is the last target's"
    );
}

#[test]
fn a_lock_of_all_with_an_original_waiting_for_an_open_target_asks_first() {
    // Alpha moved an item into Beta and another into Gamma; Gamma cannot
    // seal what it holds. A person's lock stops at the Lock sheet.
    let (mut app, _) = three();
    move_first(&mut app, 0, 1);
    move_first(&mut app, 0, 2);
    let id = app.vaults().open[2].id;
    let slot = app.vaults().open[2].facts().size.expect("a Faraday seal");
    let mut x = 0x9e37_79b9_7f4a_7c15u64;
    let notes: String = (0..2 * slot)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            char::from(b'A' + u8::try_from(x % 58).unwrap())
        })
        .collect();
    let big = Record::new(kind::ENTRY)
        .with(field::TITLE, b"big")
        .with(field::NOTES, notes.as_bytes());
    testkit::vault_edit(&mut app, id, |s| s.insert(id, big));
    testkit::vault_changed(&mut app, id);
    crate::common::top_up(&mut app);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    assert_eq!(app.sheet, Some(Sheet::LockAsk), "the lock did not ask");
    let names: Vec<_> = app.vaults().open.iter().map(|o| o.name.clone()).collect();
    assert!(
        names.iter().any(|n| n == "a.kdbx") && names.iter().any(|n| n == "c.kdbx"),
        "Alpha and Gamma stay open: {names:?}"
    );
}

/// A key record of the same words as `word` under another label.
fn relabelled(word: &str) -> Record {
    Record::new(kind::KEY)
        .with(field::KEY, &seed(word))
        .with(field::KEY_LABEL, b"Cold storage")
}

/// Replaces the first key record of open vault `o` with `record`.
fn swap_key(app: &mut Faraday, o: usize, record: Record) {
    let id = app.vaults().open[o].id;
    let at = app
        .vaults()
        .store()
        .records(id)
        .of(kind::KEY)
        .next()
        .unwrap()
        .0;
    testkit::vault_edit(app, id, |s| {
        assert!(s.replace(id, at, record));
    });
    testkit::vault_changed(app, id);
}

#[test]
fn the_same_key_is_judged_alike_every_time_the_sheet_opens() {
    let a = vault("Alpha", 1, vec![key("summer")]);
    let b = vault("Beta", 2, vec![relabelled("summer")]);
    let c = vault("Gamma", 3, vec![key("zebra")]);
    let mut app = opened(vec![("a.kdbx", a), ("b.kdbx", b), ("c.kdbx", c)]);
    let first = sheet_for(&mut app, 0, KEYS, 0);
    assert_eq!(first.matches(holds()).count(), 1, "{first}");
    for _ in 0..2 {
        app.press(Action::Vault(V::CopyCancel));
        let again = sheet_for(&mut app, 0, KEYS, 0);
        assert_eq!(again, first, "the sheet judged the rows another way");
    }
}

#[test]
fn a_key_changed_between_two_presses_is_judged_by_its_new_words() {
    let a = vault("Alpha", 1, vec![key("summer")]);
    let b = vault("Beta", 2, vec![relabelled("summer")]);
    let c = vault("Gamma", 3, vec![relabelled("zebra")]);
    let mut app = opened(vec![("a.kdbx", a), ("b.kdbx", b), ("c.kdbx", c)]);
    let texts = sheet_for(&mut app, 0, KEYS, 0);
    assert_eq!(texts.matches(holds()).count(), 1, "{texts}");
    app.press(Action::Vault(V::CopyCancel));

    // Beta's key becomes another one and Gamma's becomes the item's.
    swap_key(&mut app, 1, relabelled("zebra"));
    swap_key(&mut app, 2, relabelled("summer"));
    let texts = sheet_for(&mut app, 0, KEYS, 0);
    assert_eq!(texts.matches(holds()).count(), 1, "{texts}");
    app.press(Action::Vault(V::CopyPick(1)));
    assert_eq!(held(&app, 2).len(), 1, "Gamma holds the item, a press took");
    let before = held(&app, 1).len();
    app.press(Action::Vault(V::CopyPick(0)));
    assert_eq!(
        held(&app, 1).len(),
        before + 1,
        "Beta did not take the item"
    );
}
