//! The vault's remaining states as a person meets them: a KeePass file
//! with history, a wrong passphrase, a vault open to read only, and items
//! this build could not read.

use crate::common::type_text;
use faraday_core::strings::EN;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageCommand, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};
use faraday_vault::kdbx::xml::{Entry, Group};

type Files = Vec<(String, Vec<u8>)>;

const PHRASE: &str = "states phrase";
const BIN: &str = "<RecycleBinUUID>BAQEBAQEBAQEBAQEBAQEBA==</RecycleBinUUID><RecycleBinEnabled>True</RecycleBinEnabled>";
const NO_BIN: &str = "<RecycleBinUUID>AAAAAAAAAAAAAAAAAAAAAA==</RecycleBinUUID>";

/// A device with a 480x640 panel holding `inbox`.
fn process(inbox: Files) -> Faraday {
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
        kept: Vec::new(),
    });
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x40 + i; 32])));
    }
    app.press(Action::ReduceMotion(true));
    app
}

/// Opens place 0 of the Vaults list and presses Unlock with `phrase`.
fn try_unlock(app: &mut Faraday, phrase: &str) {
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    let _ = app.frame();
    app.press(Action::Vault(V::FocusPassphrase));
    type_text(app, phrase);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() || app.sheet.is_some() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

/// Everything drawn, the wrapped lines joined as one text.
fn page(app: &mut Faraday) -> String {
    let _ = app.frame();
    app.drawn_texts().join(" ")
}

/// The vault page of the open vault.
fn head(app: &mut Faraday) -> String {
    if app.sheet.is_some() {
        app.press(Action::Cancel);
    }
    app.press(Action::Nav(Screen::VaultContents));
    page(app)
}

fn saved_outbox(app: &mut Faraday) -> Files {
    let mut out = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { outbox, .. } = c {
            out = outbox;
        }
    }
    out
}

fn plain(n: u8, title: &str) -> Entry {
    let mut e = Entry::new([n; 16]);
    e.set_field("Title", title, false).unwrap();
    e
}

/// A KeePass app's file: one entry with a history entry, one in the
/// recycle bin.
fn with_history() -> Vec<u8> {
    history_file(|xml| xml)
}

fn history_file(more: impl Fn(String) -> String) -> Vec<u8> {
    testkit::kdbx_edited(
        "Saved",
        PHRASE,
        7,
        |doc| {
            doc.root_mut().push_entry(plain(3, "Kept")).unwrap();
            let mut bin = Group::new([4; 16], "Recycle Bin");
            bin.push_entry(plain(5, "Thrown away")).unwrap();
            doc.root_mut().push_group(bin).unwrap();
        },
        |xml| {
            let xml = xml.replace(NO_BIN, BIN);
            let at = xml.find("</Entry>").unwrap();
            let old = "<History><Entry><UUID>AgICAgICAgICAgICAgICAg==</UUID></Entry></History>";
            more(format!("{}{old}{}", &xml[..at], &xml[at..]))
        },
    )
    .unwrap()
}

#[test]
fn a_file_with_history_stops_at_the_dialog_and_says_what_the_seal_leaves_out() {
    let mut app = process(vec![("saved.kdbx".into(), with_history())]);
    try_unlock(&mut app, PHRASE);
    assert_eq!(app.sheet, Some(Sheet::OlderCopy), "no dialog");
    let texts = page(&mut app);
    let line = EN
        .vault_history_line
        .replacen("{}", "1", 1)
        .replacen("{}", "1", 1);
    assert!(texts.contains(&line), "{texts}");
    assert!(app.vaults().open.len() == 1);

    // Keep locked leaves it locked, on Unlock.
    app.press(Action::Vault(V::OlderBack));
    assert!(app.vaults().open.is_empty());
    assert_eq!(app.screen, Screen::Unlock);

    // Open opens it.
    try_unlock(&mut app, PHRASE);
    assert_eq!(app.sheet, Some(Sheet::OlderCopy));
    app.press(Action::Vault(V::OlderOpen));
    assert_eq!(app.sheet, None);
    assert_eq!(app.vaults().open.len(), 1);
}

#[test]
fn a_file_with_neither_history_nor_a_recycle_bin_opens_with_no_dialog() {
    let file = testkit::kdbx_edited(
        "Plain",
        PHRASE,
        8,
        |doc| doc.root_mut().push_entry(plain(3, "Kept")).unwrap(),
        |xml| xml,
    )
    .unwrap();
    let mut app = process(vec![("plain.kdbx".into(), file)]);
    try_unlock(&mut app, PHRASE);
    assert_eq!(app.sheet, None);
    assert_eq!(app.vaults().open.len(), 1);
}

#[test]
fn a_wrong_passphrase_shows_its_message_on_the_unlock_screen() {
    let file = testkit::kdbx_vault("Main", PHRASE, 1, Vec::new()).unwrap();
    let mut app = process(vec![("main.kdbx".into(), file)]);
    try_unlock(&mut app, "not the phrase");
    assert!(app.vaults().open.is_empty());
    assert_eq!(app.screen, Screen::Unlock);
    let texts = page(&mut app);
    // At 480x640 the pinned Unlock bar covers the message's second line:
    // its first line is what shows.
    let first = EN.vault_error_passphrase.split(". ").next().unwrap();
    assert!(texts.contains(first), "{texts}");
}

/// A vault open to read only for each reason.
fn read_only_files() -> [(&'static str, Vec<u8>, &'static str); 2] {
    let newer = testkit::kdbx_edited(
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
    .unwrap();
    let settings = testkit::kdbx_edited(
        "Odd",
        PHRASE,
        10,
        |_| {},
        |xml| xml.replace(NO_BIN, &format!("{NO_BIN}{NO_BIN}")),
    )
    .unwrap();
    [
        ("newer", newer, EN.vault_read_only_newer),
        ("settings", settings, EN.vault_read_only_settings),
    ]
}

#[test]
fn a_read_only_vault_says_why_and_an_edit_and_a_seal_write_nothing() {
    for (name, file, reason) in read_only_files() {
        let mut app = process(vec![("ro.kdbx".into(), file.clone())]);
        try_unlock(&mut app, PHRASE);
        assert_eq!(app.vaults().open.len(), 1, "{name}");
        let texts = head(&mut app);
        assert!(texts.contains(reason), "{name}: {texts}");

        app.press(Action::Entry(None));
        type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
        app.press(Action::EntryAdd);
        app.press(Action::Vault(V::SaveKey(0)));
        crate::common::top_up(&mut app);
        app.press(Action::Lock);
        testkit::settle(&mut app);
        assert!(saved_outbox(&mut app).is_empty(), "{name}: a seal wrote");
        let mut app = process(vec![("ro.kdbx".into(), file)]);
        try_unlock(&mut app, PHRASE);
        let texts = head(&mut app);
        assert!(texts.contains(reason), "{name}: {texts}");
    }
}

#[test]
fn a_read_only_file_with_history_is_not_told_a_seal_leaves_anything_out() {
    let file = history_file(|xml| {
        let at = xml.find("faraday.format").unwrap();
        let v = at + xml[at..].find("<Value>1</Value>").unwrap();
        format!(
            "{}<Value>99</Value>{}",
            &xml[..v],
            &xml[v + "<Value>1</Value>".len()..]
        )
    });
    let mut app = process(vec![("ro.kdbx".into(), file)]);
    try_unlock(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 1);
    assert_ne!(app.sheet, Some(Sheet::OlderCopy), "a dialog for nothing");
    let texts = head(&mut app);
    assert!(texts.contains(EN.vault_read_only_newer), "{texts}");
}

/// A vault with a note and a damaged item (an unknown kind).
fn with_damage() -> Vec<u8> {
    testkit::kdbx_edited(
        "Damaged",
        PHRASE,
        11,
        |doc| {
            let mut e = plain(60, "Broken item");
            e.custom_mut().set("faraday.kind", "rocket").unwrap();
            doc.root_mut().push_entry(e).unwrap();
        },
        |xml| xml,
    )
    .unwrap()
}

#[test]
fn an_unreadable_item_is_listed_and_a_seal_writes_it_back_as_read() {
    let not_read = EN.vault_not_read.replace("{}", "1");
    let mut app = process(vec![("damaged.kdbx".into(), with_damage())]);
    try_unlock(&mut app, PHRASE);
    let texts = head(&mut app);
    assert!(texts.contains(&not_read), "{texts}");
    // The section at the page's end lists the item.
    app.event(Event::Scroll {
        x: 240,
        y: 320,
        dy: 800,
    });
    app.settle();
    let texts = page(&mut app);
    assert!(texts.contains("Broken item"), "{texts}");

    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    app.press(Action::Vault(V::SaveKey(0)));
    crate::common::top_up(&mut app);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    let outbox = saved_outbox(&mut app);
    assert_eq!(outbox.len(), 1, "the seal wrote one file");

    let mut app = process(outbox);
    try_unlock(&mut app, PHRASE);
    let texts = head(&mut app);
    assert!(texts.contains(&not_read), "{texts}");
}

const MIN: u64 = 60_000;

/// A Faraday vault open in a process, `size` bytes of text that does not
/// compress added when `fill` is set, else one small entry; then thirty
/// minutes without input. Returns the process that locked.
fn idle_locked_with(fill: bool) -> Faraday {
    use faraday_vault::Record;
    use faraday_vault::records::{field, kind};
    let file = testkit::kdbx_vault("Idle", PHRASE, 0x21, Vec::new()).unwrap();
    let mut app = process(vec![("idle.kdbx".into(), file)]);
    try_unlock(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 1);
    let id = app.vaults().open[0].id;
    let notes: String = if fill {
        let slot = app.vaults().open[0].facts().size.expect("a Faraday seal");
        let mut x = 0x9e37_79b9_7f4a_7c15u64;
        (0..2 * slot)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                char::from(b'A' + u8::try_from(x % 58).unwrap())
            })
            .collect()
    } else {
        "short".into()
    };
    let rec = Record::new(kind::ENTRY)
        .with(field::TITLE, b"edit")
        .with(field::NOTES, notes.as_bytes());
    testkit::vault_edit(&mut app, id, |s| s.insert(id, rec));
    testkit::vault_changed(&mut app, id);
    crate::common::top_up(&mut app);
    let t = 100_000;
    app.event(Event::Tick { now_ms: t });
    app.event(Event::Tick {
        now_ms: t + 30 * MIN,
    });
    testkit::settle(&mut app);
    app
}

/// The Locked sheet of the process that follows `app`'s idle lock.
fn locked_page(app: &mut Faraday) -> (Files, String) {
    let mut kept = Vec::new();
    let mut outbox = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes {
            kept: k, outbox: o, ..
        } = c
        {
            kept = k;
            outbox = o;
        }
    }
    let mut next = process(Vec::new());
    next.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: outbox.clone(),
        kept,
    });
    assert_eq!(next.sheet, Some(Sheet::Locked));
    (outbox, page(&mut next))
}

#[test]
fn an_idle_lock_wipes_a_vault_whose_changes_do_not_fit_and_the_locked_sheet_says_so() {
    let mut app = idle_locked_with(true);
    assert!(app.restart_requested(), "the idle lock restarts");
    assert!(app.vaults().open.is_empty(), "the vault is still open");
    let (outbox, texts) = locked_page(&mut app);
    assert!(outbox.is_empty(), "something was sealed");
    assert!(texts.contains(EN.locked_changes_lost), "{texts}");
}

#[test]
fn an_idle_lock_of_a_vault_that_seals_says_nothing_of_lost_changes() {
    let mut app = idle_locked_with(false);
    assert!(app.restart_requested());
    assert!(app.vaults().open.is_empty());
    let (outbox, texts) = locked_page(&mut app);
    assert!(
        !outbox.is_empty(),
        "the sealed file did not reach the Outbox"
    );
    assert!(!texts.contains(EN.locked_changes_lost), "{texts}");
}
