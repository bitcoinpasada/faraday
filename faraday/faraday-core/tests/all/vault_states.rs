//! The vault's remaining states as a person meets them: a KeePass file
//! with history, a wrong passphrase, a vault open to read only, and items
//! this build could not read.

use crate::common::type_text;
use faraday_core::strings::EN;
use faraday_core::vault::VaultAction as V;
use faraday_core::{
    Action, Faraday, Screen, Sheet, StickInfo, StorageCommand, StorageEvent, testkit,
};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};
use faraday_vault::kdbx::xml::{Entry, Group};

type Files = Vec<(String, Vec<u8>)>;

const PHRASE: &str = "states phrase";
const BIN: &str = "<RecycleBinUUID>BAQEBAQEBAQEBAQEBAQEBA==</RecycleBinUUID><RecycleBinEnabled>True</RecycleBinEnabled>";
const NO_BIN: &str = "<RecycleBinUUID>AAAAAAAAAAAAAAAAAAAAAA==</RecycleBinUUID>";

/// A device with a 480x640 panel holding `inbox`.
fn process(inbox: Files) -> Faraday {
    process_at(480, 640, inbox)
}

/// A device with a `width` x `height` panel holding `inbox`.
fn process_at(width: u16, height: u16, inbox: Files) -> Faraday {
    process_with(width, height, Some(15_000), inbox)
}

/// [`process_at`] with `free` MiB of memory free, or not told.
fn process_with(width: u16, height: u16, free: Option<u32>, inbox: Files) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi: 286,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    if let Some(available_mib) = free {
        app.storage(StorageEvent::Memory { available_mib });
    }
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

/// An open writable vault under `PHRASE`.
fn open_vault() -> Faraday {
    let file = testkit::kdbx_vault("Main", PHRASE, 0x31, Vec::new()).unwrap();
    let mut app = process(vec![("main.kdbx".into(), file)]);
    try_unlock(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 1);
    app.press(Action::Nav(Screen::VaultContents));
    app
}

/// Locks the open vault and returns the file it sealed.
fn lock_and_seal(app: &mut Faraday) -> Files {
    crate::common::top_up(app);
    app.press(Action::Lock);
    testkit::settle(app);
    let out = saved_outbox(app);
    assert_eq!(out.len(), 1, "a lock sealed no file");
    out
}

/// Types `first`, then `again`, into Change passphrase and presses Save.
fn change_passphrase(app: &mut Faraday, first: &str, again: &str) {
    app.press(Action::Vault(V::ChangePassphrase));
    assert_eq!(app.sheet, Some(Sheet::VaultPassphrase));
    type_text(app, first);
    app.press(Action::Vault(V::FocusField(1)));
    type_text(app, again);
    app.press(Action::Vault(V::FormSave));
}

#[test]
fn a_changed_passphrase_is_the_one_the_next_seal_opens_under() {
    let mut app = open_vault();
    change_passphrase(&mut app, "a new phrase", "a new phrase");
    assert_eq!(app.sheet, None);
    assert!(
        app.toast_text()
            .is_some_and(|t| t == EN.vault_passphrase_next_seal)
    );
    // The lock sheet counts it as an unsaved change.
    assert!(app.vaults().open[0].changes > 0);
    let sealed = lock_and_seal(&mut app);

    let mut app = process(sealed.clone());
    try_unlock(&mut app, "a new phrase");
    assert_eq!(
        app.vaults().open.len(),
        1,
        "the new passphrase did not open it"
    );
    let mut app = process(sealed);
    try_unlock(&mut app, PHRASE);
    assert!(app.vaults().open.is_empty(), "the old passphrase opened it");
}

#[test]
fn a_passphrase_typed_twice_differently_or_empty_is_refused_and_changes_nothing() {
    let mut app = open_vault();
    change_passphrase(&mut app, "one phrase", "another phrase");
    assert_eq!(app.sheet, Some(Sheet::VaultPassphrase));
    assert_eq!(app.toast_text(), Some(EN.vaults_each_passphrase));
    app.press(Action::Vault(V::FormCancel));
    assert_eq!(app.sheet, None);
    app.press(Action::Vault(V::ChangePassphrase));
    app.press(Action::Vault(V::FormSave));
    assert_eq!(app.toast_text(), Some(EN.vaults_every_passphrase));
    app.press(Action::Vault(V::FormCancel));
    assert_eq!(app.vaults().open[0].changes, 0);
}

#[test]
fn a_changed_memory_step_is_the_cost_the_sealed_file_states() {
    let mut app = open_vault();
    let steps = faraday_vault::file::STEPS_KIB;
    let have = app.vaults().open[0].facts().header.cost.memory_kib;
    let pick = if have == steps[1] { 2 } else { 1 };
    app.press(Action::Vault(V::MemoryAsk));
    assert_eq!(app.sheet, Some(Sheet::VaultMemory));
    let _ = app.frame();
    app.press(Action::Vault(V::MemoryStep(pick)));
    assert_eq!(app.sheet, None);
    assert!(app.vaults().open[0].changes > 0);
    let sealed = lock_and_seal(&mut app);

    let mut app = process(sealed);
    try_unlock(&mut app, PHRASE);
    assert_eq!(
        app.vaults().open[0].facts().header.cost.memory_kib,
        steps[pick]
    );
}

#[test]
fn a_read_only_vault_offers_neither_and_opens_neither() {
    let (name, file, _) = read_only_files().into_iter().next().unwrap();
    let mut app = process(vec![("ro.kdbx".into(), file)]);
    try_unlock(&mut app, PHRASE);
    let texts = head(&mut app);
    assert!(
        !texts.contains(EN.vault_change_passphrase),
        "{name}: {texts}"
    );
    app.press(Action::Vault(V::ChangePassphrase));
    assert_eq!(app.sheet, None);
    app.press(Action::Vault(V::MemoryAsk));
    assert_eq!(app.sheet, None);
}

#[test]
fn an_open_vault_offers_change_passphrase_and_memory() {
    let mut app = open_vault();
    let texts = page(&mut app);
    assert!(texts.contains(EN.vault_change_passphrase), "{texts}");
    assert!(texts.contains(EN.vault_memory), "{texts}");
}

const TYPED: &str = "typed-not-saved-9";

/// The dialog `ask` opens, typed into where it takes text, then `lock`;
/// what is left of it afterwards.
fn lock_with_dialog_open(ask: V, lock: impl Fn(&mut Faraday)) {
    let mut app = open_vault();
    app.press(Action::Vault(ask));
    assert!(matches!(
        app.sheet,
        Some(Sheet::VaultPassphrase | Sheet::VaultMemory)
    ));
    if ask == V::ChangePassphrase {
        type_text(&mut app, TYPED);
        app.press(Action::Vault(V::FocusField(1)));
        type_text(&mut app, TYPED);
    }
    lock(&mut app);
    testkit::settle(&mut app);
    assert!(
        !matches!(app.sheet, Some(Sheet::VaultPassphrase | Sheet::VaultMemory)),
        "the dialog is still open: {:?}",
        app.sheet
    );
    assert!(app.vaults().open.is_empty(), "the vault is still open");
    assert!(!page(&mut app).contains(TYPED), "the typed text is shown");
    let out = saved_outbox(&mut app);
    for (_, bytes) in &out {
        // Whatever was written opens under the old passphrase.
        let mut next = process(vec![("main.kdbx".into(), bytes.clone())]);
        try_unlock(&mut next, PHRASE);
        assert_eq!(next.vaults().open.len(), 1, "the passphrase changed");
        if ask == V::ChangePassphrase {
            next.press(Action::Nav(Screen::VaultContents));
            next.press(Action::Vault(V::ChangePassphrase));
            assert!(!page(&mut next).contains(TYPED));
        }
    }
}

fn plain_lock(app: &mut Faraday) {
    crate::common::top_up(app);
    app.press(Action::Lock);
}

fn idle_lock(app: &mut Faraday) {
    crate::common::top_up(app);
    app.event(Event::Tick { now_ms: 100_000 });
    app.event(Event::Tick {
        now_ms: 100_000 + 30 * MIN,
    });
}

#[test]
fn a_lock_closes_change_passphrase_and_drops_what_was_typed() {
    lock_with_dialog_open(V::ChangePassphrase, plain_lock);
    lock_with_dialog_open(V::ChangePassphrase, idle_lock);
}

#[test]
fn a_lock_closes_memory_and_changes_nothing() {
    lock_with_dialog_open(V::MemoryAsk, plain_lock);
    lock_with_dialog_open(V::MemoryAsk, idle_lock);
}

/// The last of the five memory steps, below the dialog's cut at first.
const LAST_STEP: usize = 4;

#[test]
fn the_last_memory_step_is_reached_by_touch() {
    let mut app = open_vault();
    app.press(Action::Vault(V::MemoryAsk));
    let step = Action::Vault(V::MemoryStep(LAST_STEP));
    let mut got = crate::common::offered(&mut app, step);
    for _ in 0..10 {
        if got {
            break;
        }
        app.event(Event::Scroll {
            x: 240,
            y: 320,
            dy: 120,
        });
        got = crate::common::offered(&mut app, step);
    }
    assert!(got, "1024 MiB cannot be reached by touch");
}

#[test]
fn the_last_memory_step_is_reached_by_the_mouse_wheel() {
    let mut app = open_vault();
    app.press(Action::Vault(V::MemoryAsk));
    let step = Action::Vault(V::MemoryStep(LAST_STEP));
    let mut got = crate::common::offered(&mut app, step);
    for _ in 0..10 {
        if got {
            break;
        }
        app.event(Event::Wheel {
            x: 240,
            y: 320,
            dy: 48,
        });
        got = crate::common::offered(&mut app, step);
    }
    assert!(got, "1024 MiB cannot be reached by the wheel");
}

/// An open vault with `free` MiB of memory free.
fn open_vault_with_free(free: u32) -> Faraday {
    let mut app = open_vault();
    app.storage(StorageEvent::Memory {
        available_mib: free,
    });
    app
}

#[test]
fn a_memory_step_this_computer_could_not_unlock_is_neither_offered_nor_taken() {
    let mut app = open_vault_with_free(1100);
    app.press(Action::Vault(V::MemoryAsk));
    let last = Action::Vault(V::MemoryStep(LAST_STEP));
    let small = Action::Vault(V::MemoryStep(2));
    assert!(crate::common::offered(&mut app, small));
    assert!(!crate::common::offered(&mut app, last), "1 GiB is offered");
    app.press(last);
    assert_eq!(app.sheet, Some(Sheet::VaultMemory), "1 GiB was taken");
    assert_eq!(app.vaults().open[0].changes, 0);
    assert!(app.toast_text().is_some());
}

#[test]
fn with_free_memory_unknown_a_step_above_64_mib_is_neither_offered_nor_taken() {
    let file = testkit::kdbx_vault("Main", PHRASE, 0x31, Vec::new()).unwrap();
    let mut app = process_with(480, 640, None, vec![("main.kdbx".into(), file)]);
    try_unlock(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 1, "a 64 MiB vault did not open");
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::MemoryAsk));
    for k in 1..=LAST_STEP {
        let step = Action::Vault(V::MemoryStep(k));
        assert!(!crate::common::offered(&mut app, step), "step {k} offered");
        app.press(step);
        assert_eq!(app.sheet, Some(Sheet::VaultMemory), "step {k} was taken");
        assert_eq!(
            app.vaults().open[0].changes,
            0,
            "step {k} changed the vault"
        );
    }
}

/// An entry being written, with `TYPED` in its title.
fn writing_an_entry(width: u16, height: u16) -> Faraday {
    let file = testkit::kdbx_vault("Main", PHRASE, 0x31, Vec::new()).unwrap();
    let mut app = process_at(width, height, vec![("main.kdbx".into(), file)]);
    try_unlock(&mut app, PHRASE);
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(2)));
    app.press(Action::Vault(V::Add));
    app.press(Action::Vault(V::FocusField(0)));
    type_text(&mut app, TYPED);
    app
}

#[test]
fn an_entry_being_written_survives_change_passphrase_and_memory() {
    for (w, h) in [(1280, 800), (480, 640)] {
        let mut app = writing_an_entry(w, h);
        assert!(page(&mut app).contains(TYPED), "{w}x{h}");
        for ask in [V::MemoryAsk, V::ChangePassphrase] {
            app.press(Action::Vault(ask));
            assert_eq!(app.sheet, None, "{w}x{h}: {ask:?} opened");
            assert!(!crate::common::offered(&mut app, Action::Vault(ask)));
            assert!(page(&mut app).contains(TYPED), "{w}x{h}: the draft is gone");
        }
        // Typing still goes to the entry.
        type_text(&mut app, "!");
        assert!(page(&mut app).contains(&format!("{TYPED}!")), "{w}x{h}");
    }
}

/// The ways out of a dialog other than its Cancel.
fn leave(app: &mut Faraday, how: &str) {
    match how {
        "escape" => app.event(Event::Key(faraday_shell_api::Key::Escape)),
        _ => app.event(Event::Back),
    }
}

#[test]
fn escape_and_the_mouse_back_close_change_passphrase_and_memory_and_drop_what_was_typed() {
    for how in ["escape", "back"] {
        for ask in [V::ChangePassphrase, V::MemoryAsk] {
            let mut app = open_vault();
            app.press(Action::Vault(ask));
            if ask == V::ChangePassphrase {
                type_text(&mut app, TYPED);
                app.press(Action::Vault(V::FocusField(1)));
                type_text(&mut app, TYPED);
            }
            leave(&mut app, how);
            let _ = app.frame();
            assert_eq!(app.sheet, None, "{how} {ask:?}: the dialog is open");
            assert!(!page(&mut app).contains(TYPED), "{how} {ask:?}: shown");
            // Nothing typed is in the form that Add an entry opens.
            app.press(Action::Vault(V::Category(2)));
            app.press(Action::Vault(V::Add));
            let texts = page(&mut app);
            assert!(texts.contains(EN.vault_save), "{how} {ask:?}: no form");
            assert!(!texts.contains(TYPED), "{how} {ask:?}: typed text kept");
            assert_eq!(app.vaults().open[0].changes, 0);
        }
    }
}

/// The size of the Memory dialog's row marked as the next seal's, looked
/// for at its end and then at its top.
fn next_seal_size(app: &mut Faraday) -> String {
    for key in [faraday_shell_api::Key::Down, faraday_shell_api::Key::Up] {
        for _ in 0..30 {
            app.event(Event::Key(key));
            let _ = app.frame();
        }
        let texts = app.drawn_texts();
        if let Some(at) = texts.iter().position(|t| t == EN.vault_next_seal) {
            return texts[at - 1].to_string();
        }
    }
    panic!("no row is marked as the next seal");
}

#[test]
fn the_last_memory_step_is_reached_by_keys_alone_at_the_smallest_size() {
    // Each try on a fresh device: Down scrolls the sheet to its end, Tab
    // is pressed n times, Enter presses what Tab reached. The 1 GiB row is
    // among them.
    let last = EN.vaults_gib.replace("{}", "1");
    let mut taken = Vec::new();
    for n in 1..=8 {
        let file = testkit::kdbx_vault("Main", PHRASE, 0x31, Vec::new()).unwrap();
        let mut app = process_at(320, 480, vec![("main.kdbx".into(), file)]);
        try_unlock(&mut app, PHRASE);
        app.press(Action::Nav(Screen::VaultContents));
        app.press(Action::Vault(V::MemoryAsk));
        let _ = app.frame();
        for _ in 0..30 {
            app.event(Event::Key(faraday_shell_api::Key::Down));
            let _ = app.frame();
        }
        for _ in 0..n {
            app.event(Event::Key(faraday_shell_api::Key::Tab));
            let _ = app.frame();
        }
        app.event(Event::Key(faraday_shell_api::Key::Enter));
        let _ = app.frame();
        if app.sheet.is_none() {
            app.press(Action::Vault(V::MemoryAsk));
            taken.push(next_seal_size(&mut app));
        }
    }
    assert!(
        taken.contains(&last),
        "1 GiB was not taken by keys: {taken:?}"
    );
}

/// A stick plugged in.
fn attach_stick(app: &mut Faraday) {
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: "a".into(),
        label: "STICK".into(),
        boot: false,
        files: Vec::new(),
        vaults: Vec::new(),
    }]));
    if app.sheet == Some(Sheet::Lock) {
        app.press(Action::NotNow);
    }
}

#[test]
fn escape_closes_change_passphrase_when_a_stick_leaves_no_field_focused() {
    let mut app = open_vault();
    attach_stick(&mut app);
    app.press(Action::Vault(V::ChangePassphrase));
    type_text(&mut app, TYPED);
    assert_eq!(
        app.sheet,
        Some(Sheet::VaultPassphrase),
        "the dialog is not open"
    );
    leave(&mut app, "escape");
    let _ = app.frame();
    assert_eq!(app.sheet, None, "Escape left the dialog open");
    app.press(Action::Vault(V::Category(2)));
    app.press(Action::Vault(V::Add));
    let texts = page(&mut app);
    assert!(texts.contains(EN.vault_save), "no form");
    assert!(!texts.contains(TYPED), "typed text kept");
}
