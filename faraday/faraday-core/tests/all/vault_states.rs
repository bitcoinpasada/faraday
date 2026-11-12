//! The vault's remaining states as a person meets them: a KeePass file
//! with history, a wrong passphrase, a vault open to read only, and items
//! this build could not read.

use crate::common::type_text;
use faraday_core::strings::EN;
use faraday_core::vault::KeyChoice;
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
    process_boxes(width, height, free, inbox, Vec::new(), Vec::new())
}

/// [`process_with`] with files waiting in the Outbox and `kept` (what the
/// last process remembered).
fn process_boxes(
    width: u16,
    height: u16,
    free: Option<u32>,
    inbox: Files,
    outbox: Files,
    kept: Files,
) -> Faraday {
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
        outbox,
        kept,
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
fn a_file_with_history_and_a_recycle_bin_opens_with_no_dialog() {
    let mut app = process(vec![("saved.kdbx".into(), with_history())]);
    try_unlock(&mut app, PHRASE);
    assert_eq!(app.sheet, None, "a dialog");
    assert_eq!(app.vaults().open.len(), 1);
    let texts = head(&mut app);
    assert!(!texts.contains("Thrown away"), "{texts}");
}

#[test]
fn a_file_with_no_history_opens_with_no_dialog() {
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
    assert!(texts.contains(EN.vault_error_wrong), "{texts}");
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
        dy: 2000,
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

/// An idle lock of a vault with an entry added and a new memory step
/// chosen, the seal of the new step failing when `fail`. Returns the
/// process that locked and the memory step the vault was opened with.
fn idle_locked_with_step(fail: bool) -> (Faraday, u32) {
    idle_locked_step(fail, false)
}

/// [`idle_locked_with_step`], the seal retried at the old step failing too
/// when `retry_fails`.
fn idle_locked_step(fail: bool, retry_fails: bool) -> (Faraday, u32) {
    use faraday_vault::Record;
    use faraday_vault::records::{field, kind};
    let file = testkit::kdbx_vault("Step", PHRASE, 0x22, Vec::new()).unwrap();
    let mut app = process(vec![("step.kdbx".into(), file)]);
    try_unlock(&mut app, PHRASE);
    let id = app.vaults().open[0].id;
    let have = app.vaults().open[0].facts().header.cost.memory_kib;
    let steps = faraday_vault::file::STEPS_KIB;
    let pick = if have == steps[1] { 2 } else { 1 };
    app.press(Action::Vault(V::MemoryAsk));
    let _ = app.frame();
    app.press(Action::Vault(V::MemoryStep(pick)));
    let rec = Record::new(kind::ENTRY).with(field::TITLE, b"kept");
    testkit::vault_edit(&mut app, id, |s| s.insert(id, rec));
    testkit::vault_changed(&mut app, id);
    crate::common::top_up(&mut app);
    if retry_fails {
        testkit::fail_next_step_seal_and_retry();
    } else if fail {
        testkit::fail_next_step_seal();
    }
    app.event(Event::Tick { now_ms: 100_000 });
    app.event(Event::Tick {
        now_ms: 100_000 + 30 * MIN,
    });
    testkit::settle(&mut app);
    (app, have)
}

#[test]
fn an_idle_lock_that_cannot_allocate_a_new_memory_step_seals_at_the_old_one_and_says_so() {
    let (mut app, have) = idle_locked_with_step(true);
    assert!(app.restart_requested(), "the idle lock restarts");
    assert!(app.vaults().open.is_empty());
    let (outbox, texts) = locked_page(&mut app);
    assert_eq!(outbox.len(), 1, "the retried seal reached the Outbox");
    assert!(texts.contains(EN.locked_step_dropped), "{texts}");
    assert!(!texts.contains(EN.locked_changes_lost), "{texts}");

    let mut next = process(outbox);
    try_unlock(&mut next, PHRASE);
    assert_eq!(
        next.vaults().open.len(),
        1,
        "the passphrase did not open it"
    );
    let steps = faraday_vault::file::STEPS_KIB;
    let own = steps[steps.iter().position(|&s| s >= have).unwrap()];
    assert_eq!(next.vaults().open[0].facts().header.cost.memory_kib, own);
    let id = next.vaults().open[0].id;
    let held = next
        .vaults()
        .records(id)
        .of(faraday_vault::records::kind::ENTRY)
        .count();
    assert_eq!(held, 1, "the change was not kept");
}

#[test]
fn an_idle_lock_whose_retried_seal_runs_out_of_memory_too_keeps_the_vault_open_with_its_changes() {
    let (app, _) = idle_locked_step(true, true);
    assert!(!app.restart_requested(), "the idle lock went ahead");
    assert_eq!(app.vaults().open.len(), 1, "the vault was closed");
    assert!(app.vaults().open[0].changes > 0, "the changes were dropped");
    assert_eq!(
        app.toast_text(),
        Some("Needs 1 MiB. This computer has 14.6\u{a0}GiB free.")
    );
}

#[test]
fn a_lock_whose_seal_runs_out_of_memory_half_way_names_the_vault_and_lets_it_go_on_request() {
    // A person's Lock: the vault stays open with its changes, the sheet
    // comes back naming it with the memory line, and Lock without saving
    // (the seal running out of memory again) lets it go.
    half_way(false);
}

#[test]
fn a_seal_that_runs_out_of_memory_as_the_free_memory_falls_still_names_the_vault() {
    // The computer reads less free each time it is asked, as a seal that
    // took memory leaves it: the sheet still names the vault.
    half_way(true);
}

fn half_way(falling: bool) {
    let figures: Vec<u32> = (1..=200u32).map(|n| 20_000 - n * 50).collect();
    let mut app = short_vault_sheet(falling.then_some(figures.as_slice()));
    testkit::fail_next_step_seal();
    app.press(Action::LockUnsealed);
    testkit::settle(&mut app);
    assert!(app.restart_requested(), "{:?}", app.toast_text());
}

/// A lock whose seal of the vault with a new memory step runs out of memory
/// half way, the sheet that comes back drawn once; the computer reads
/// `figures` as it is asked, if any.
fn short_vault_sheet(figures: Option<&[u32]>) -> Faraday {
    use faraday_vault::Record;
    use faraday_vault::records::{field, kind};
    let file = testkit::kdbx_vault("Step", PHRASE, 0x22, Vec::new()).unwrap();
    let mut app = process(vec![("step.kdbx".into(), file)]);
    try_unlock(&mut app, PHRASE);
    let id = app.vaults().open[0].id;
    let have = app.vaults().open[0].facts().header.cost.memory_kib;
    let pick = if have == faraday_vault::file::STEPS_KIB[1] {
        2
    } else {
        1
    };
    app.press(Action::Vault(V::MemoryAsk));
    let _ = app.frame();
    app.press(Action::Vault(V::MemoryStep(pick)));
    let rec = Record::new(kind::ENTRY).with(field::TITLE, b"kept");
    testkit::vault_edit(&mut app, id, |s| s.insert(id, rec));
    testkit::vault_changed(&mut app, id);
    crate::common::top_up(&mut app);
    if let Some(figures) = figures {
        testkit::read_memory_as(&mut app, figures);
    }
    testkit::fail_next_step_seal();
    app.press(Action::Lock);
    testkit::settle(&mut app);
    assert!(!app.restart_requested());
    assert_eq!(app.vaults().open.len(), 1);
    assert!(app.vaults().open[0].changes > 0);
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    let _ = app.frame();
    let text = app.drawn_texts().join(" ");
    assert!(text.contains("memory is short"), "{text}");
    app
}

#[test]
fn the_sheet_keeps_naming_a_short_vault_as_the_free_memory_reads_higher() {
    let mut app = short_vault_sheet(None);
    let free = app.vaults().memory_free_mib.unwrap_or(20_000);
    for mib in [free + 1, free + 400, free + 3_000] {
        app.storage(StorageEvent::Memory { available_mib: mib });
        let _ = app.frame();
        let text = app.drawn_texts().join(" ");
        assert!(text.contains("memory is short"), "{mib}: {text}");
    }
}

#[test]
fn the_line_for_a_short_vault_says_the_memory_free_after_the_seal_failed() {
    let figures: Vec<u32> = (1..=200u32).map(|n| 60_000 - n * 250).collect();
    let mut app = short_vault_sheet(Some(&figures));
    let free = app.vaults().memory_free_mib.unwrap();
    let said = format!("has {} free", faraday_core::vault::mib_text(free));
    let text = app.drawn_texts().join(" ");
    assert!(text.contains(&said), "{said}: {text}");
    assert!(
        app.toast_text().is_some_and(|t| t.contains(&said)),
        "{said}: {:?}",
        app.toast_text()
    );
}

#[test]
fn a_short_vault_that_seals_on_the_next_lock_is_locked() {
    let mut app = short_vault_sheet(None);
    let text = app.drawn_texts().join(" ");
    assert!(text.contains("memory is short"), "{text}");
    assert!(app.offers(Action::Lock), "the sheet offers no Lock: {text}");
    app.press(Action::Lock);
    testkit::settle(&mut app);
    assert!(app.restart_requested(), "{:?}", app.toast_text());
    assert!(app.vaults().open.is_empty(), "the vault stayed open");
}

#[test]
fn a_vault_sealed_on_the_second_try_is_no_longer_named_short_beside_one_still_open() {
    // Two vaults; the first ran out of memory in a person's Lock and is
    // named. Sealed on its own on the next try, it is closed with its
    // file in the Outbox, and what is drawn names nothing short.
    use faraday_vault::Record;
    use faraday_vault::records::{field, kind};
    let a = testkit::kdbx_vault("Step", PHRASE, 0x22, Vec::new()).unwrap();
    let b = testkit::kdbx_vault("Other", PHRASE, 0x23, Vec::new()).unwrap();
    let mut app = process(vec![("a.kdbx".into(), a), ("b.kdbx".into(), b)]);
    for at in 0..2 {
        app.press(Action::Nav(Screen::Vaults));
        app.press(Action::Vault(V::Open(at)));
        let _ = app.frame();
        app.press(Action::Vault(V::FocusPassphrase));
        type_text(&mut app, PHRASE);
        app.press(Action::Vault(V::Unlock));
        for t in 1..60u64 {
            if app.vaults().open.len() > at {
                break;
            }
            let _ = app.frame();
            app.event(Event::Tick { now_ms: t * 1000 });
        }
    }
    assert_eq!(app.vaults().open.len(), 2);
    let id = app.vaults().open[0].id;
    let name = app.vaults().open[0].name.clone();
    let have = app.vaults().open[0].facts().header.cost.memory_kib;
    let pick = if have == faraday_vault::file::STEPS_KIB[1] {
        2
    } else {
        1
    };
    app.press(Action::Nav(Screen::VaultContents));
    testkit::vault_current(&mut app, 0);
    app.press(Action::Vault(V::MemoryAsk));
    let _ = app.frame();
    app.press(Action::Vault(V::MemoryStep(pick)));
    let rec = Record::new(kind::ENTRY).with(field::TITLE, b"kept");
    testkit::vault_edit(&mut app, id, |s| s.insert(id, rec));
    testkit::vault_changed(&mut app, id);
    crate::common::top_up(&mut app);
    testkit::fail_next_step_seal();
    app.press(Action::Lock);
    testkit::settle(&mut app);
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    let text = page(&mut app);
    assert!(text.contains("memory is short"), "{text}");
    // The second try seals it alone while the other stays open.
    testkit::vault_lock_one(&mut app, 0);
    testkit::settle(&mut app);
    assert_eq!(app.vaults().open.len(), 1, "the sealed vault stayed open");
    assert_ne!(app.vaults().open[0].name, name);
    let out = saved_outbox(&mut app);
    assert_eq!(out.len(), 1, "no file sealed");
    // It comes back from the file it sealed: nothing says it is short.
    let mut app = process_boxes(480, 640, Some(15_000), out, Vec::new(), Vec::new());
    try_unlock(&mut app, PHRASE);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    let text = page(&mut app);
    assert!(!text.contains("memory is short"), "{text}");
}

#[test]
fn an_idle_lock_that_runs_out_of_memory_again_corrects_the_line_a_person_s_lock_left() {
    // A person's Lock left the vault named short with one free reading; an
    // idle lock of the same unchanged vault fails with another, and the
    // sheet quotes that one, as the toast does.
    let mut app = short_vault_sheet(None);
    let before = app.vaults().memory_free_mib.unwrap();
    let now = before / 2;
    app.storage(StorageEvent::Memory { available_mib: now });
    crate::common::top_up(&mut app);
    testkit::fail_next_step_seal_and_retry();
    app.event(Event::Tick { now_ms: 100_000 });
    app.event(Event::Tick {
        now_ms: 100_000 + 30 * MIN,
    });
    testkit::settle(&mut app);
    let said = format!(
        "has {} free",
        faraday_core::vault::mib_text(app.vaults().memory_free_mib.unwrap())
    );
    let text = page(&mut app);
    assert!(text.contains(&said), "{said}: {text}");
}

#[test]
fn an_idle_lock_that_seals_the_new_memory_step_says_nothing_of_a_dropped_one() {
    let (mut app, have) = idle_locked_with_step(false);
    let (outbox, texts) = locked_page(&mut app);
    assert_eq!(outbox.len(), 1);
    assert!(!texts.contains(EN.locked_step_dropped), "{texts}");
    let mut next = process(outbox);
    try_unlock(&mut next, PHRASE);
    let steps = faraday_vault::file::STEPS_KIB;
    let own = steps[steps.iter().position(|&s| s >= have).unwrap()];
    assert_ne!(next.vaults().open[0].facts().header.cost.memory_kib, own);
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
fn pressing_the_memory_step_a_vault_has_changes_nothing_and_takes_a_pending_one_back() {
    let mut app = open_vault();
    let steps = faraday_vault::file::STEPS_KIB;
    let have = app.vaults().open[0].facts().header.cost.memory_kib;
    let own = steps.iter().position(|&s| s >= have).unwrap();
    let other = if own == 1 { 2 } else { 1 };
    app.press(Action::Vault(V::MemoryAsk));
    let _ = app.frame();
    app.press(Action::Vault(V::MemoryStep(own)));
    assert_eq!(app.sheet, None);
    assert_eq!(app.vaults().open[0].changes, 0, "nothing was changed");
    app.press(Action::Vault(V::MemoryAsk));
    let _ = app.frame();
    app.press(Action::Vault(V::MemoryStep(other)));
    assert!(app.vaults().open[0].changes > 0);
    app.press(Action::Vault(V::MemoryAsk));
    let _ = app.frame();
    app.press(Action::Vault(V::MemoryStep(own)));
    let sealed = lock_and_seal(&mut app);
    let mut app = process(sealed);
    try_unlock(&mut app, PHRASE);
    assert_eq!(
        app.vaults().open[0].facts().header.cost.memory_kib,
        steps[own],
        "the pending step was taken back"
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
/// for on the way to its end and back to its top.
fn next_seal_size(app: &mut Faraday) -> String {
    for key in [faraday_shell_api::Key::Down, faraday_shell_api::Key::Up] {
        for _ in 0..30 {
            app.event(Event::Key(key));
            let _ = app.frame();
            let texts = app.drawn_texts();
            if let Some(at) = texts.iter().position(|t| t == EN.vault_next_seal) {
                return texts[at - 1].to_string();
            }
        }
    }
    panic!("no row is marked as the next seal");
}

#[test]
fn the_last_memory_step_is_reached_by_keys_alone_at_the_smallest_size() {
    // Each try on a fresh device: Down scrolls the sheet by some steps (the
    // rows of machines lie between 1 GiB and the end), Tab is pressed n
    // times, Enter presses what Tab reached. The 1 GiB row is
    // among them.
    let last = EN.vaults_gib.replace("{}", "1");
    let mut taken = Vec::new();
    for (down, n) in (0..=30)
        .step_by(3)
        .flat_map(|d| (1..=12).map(move |n| (d, n)))
    {
        if taken.contains(&last) {
            break;
        }
        let file = testkit::kdbx_vault("Main", PHRASE, 0x31, Vec::new()).unwrap();
        let mut app = process_at(320, 480, vec![("main.kdbx".into(), file)]);
        try_unlock(&mut app, PHRASE);
        app.press(Action::Nav(Screen::VaultContents));
        app.press(Action::Vault(V::MemoryAsk));
        let _ = app.frame();
        for _ in 0..down {
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

/// An open vault `main.kdbx` with a change, and `outbox` waiting.
fn open_main_changed(inbox_main: Vec<u8>, outbox: Files, kept: Files) -> Faraday {
    open_changed(vec![("main.kdbx".into(), inbox_main)], outbox, kept)
}

/// The vault under PHRASE of `inbox` open with a change, and `outbox` waiting.
fn open_changed(inbox: Files, outbox: Files, kept: Files) -> Faraday {
    use faraday_vault::Record;
    use faraday_vault::records::{field, kind};
    let mut app = process_boxes(480, 640, Some(15_000), inbox, outbox, kept);
    // The list may hold the Outbox's file first: the place whose
    // passphrase is PHRASE opens.
    for place in 0..3 {
        app.press(Action::Nav(Screen::Vaults));
        app.press(Action::Vault(V::Open(place)));
        let _ = app.frame();
        app.press(Action::Vault(V::FocusPassphrase));
        type_text(&mut app, PHRASE);
        app.press(Action::Vault(V::Unlock));
        for t in 1..60u64 {
            if !app.vaults().open.is_empty() || app.sheet.is_some() {
                break;
            }
            let _ = app.frame();
            app.event(Event::Tick { now_ms: t * 1000 });
        }
        if !app.vaults().open.is_empty() {
            break;
        }
        if app.sheet.is_some() {
            app.press(Action::Cancel);
        }
    }
    assert_eq!(app.vaults().open.len(), 1);
    let id = app.vaults().open[0].id;
    let rec = Record::new(kind::ENTRY).with(field::TITLE, b"added");
    testkit::vault_edit(&mut app, id, |s| s.insert(id, rec));
    testkit::vault_changed(&mut app, id);
    app
}

/// Locks `app` and returns the Outbox and the memory it kept.
fn lock_boxes(app: &mut Faraday) -> (Files, Files) {
    crate::common::top_up(app);
    app.press(Action::Lock);
    testkit::settle(app);
    let (mut outbox, mut kept) = (Vec::new(), Vec::new());
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes {
            outbox: o, kept: k, ..
        } = c
        {
            (outbox, kept) = (o, k);
        }
    }
    (outbox, kept)
}

#[test]
fn a_sealed_vault_goes_beside_a_file_that_took_its_name_and_never_over_it() {
    // A KeePass export took main.kdbx while the vault, also main.kdbx, was open.
    let export = testkit::kdbx_vault("Export", "keepass passphrase", 0x52, Vec::new()).unwrap();
    let main = testkit::kdbx_vault("Main", PHRASE, 0x31, Vec::new()).unwrap();
    let mut app = open_main_changed(main, vec![("main.kdbx".into(), export.clone())], Vec::new());
    let (outbox, _) = lock_boxes(&mut app);
    let file = |name: &str| outbox.iter().find(|(n, _)| n == name).map(|(_, b)| b);
    assert_eq!(
        outbox.len(),
        2,
        "{:?}",
        outbox.iter().map(|f| &f.0).collect::<Vec<_>>()
    );
    assert_eq!(file("main.kdbx"), Some(&export), "the export was replaced");
    let mut next = process(vec![(
        "main-2.kdbx".into(),
        file("main-2.kdbx").cloned().unwrap(),
    )]);
    try_unlock(&mut next, PHRASE);
    assert_eq!(
        next.vaults().open.len(),
        1,
        "the seal is beside it, under its passphrase"
    );
}

#[test]
fn a_seal_put_beside_a_file_also_stays_clear_of_another_vault_in_files() {
    let export = testkit::kdbx_vault("Export", "keepass passphrase", 0x52, Vec::new()).unwrap();
    let main = testkit::kdbx_vault("Main", PHRASE, 0x31, Vec::new()).unwrap();
    let other = testkit::kdbx_vault("Other", "another passphrase", 0x53, Vec::new()).unwrap();
    let mut app = open_changed(
        vec![
            ("main.kdbx".into(), main),
            ("main-2.kdbx".into(), other.clone()),
        ],
        vec![("main.kdbx".into(), export.clone())],
        Vec::new(),
    );
    let (outbox, _) = lock_boxes(&mut app);
    let mut names: Vec<&str> = outbox.iter().map(|f| f.0.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, ["main-3.kdbx", "main.kdbx"]);
    assert!(app.inbox().iter().any(|i| i.name == "main-2.kdbx"));
}

#[test]
fn a_seal_replaces_the_earlier_copy_of_the_same_vault_waiting_across_a_lock() {
    let main = testkit::kdbx_vault("Main", PHRASE, 0x31, Vec::new()).unwrap();
    let mut first = open_main_changed(main, Vec::new(), Vec::new());
    let (outbox, kept) = lock_boxes(&mut first);
    assert_eq!(outbox.len(), 1);
    let earlier = outbox[0].1.clone();
    // The next power-on: the sealed copy waits in the Outbox and is opened from there.
    let mut next = process_boxes(480, 640, Some(15_000), Vec::new(), outbox, kept);
    try_unlock(&mut next, PHRASE);
    assert_eq!(next.vaults().open.len(), 1);
    use faraday_vault::Record;
    use faraday_vault::records::{field, kind};
    let id = next.vaults().open[0].id;
    let rec = Record::new(kind::ENTRY).with(field::TITLE, b"second");
    testkit::vault_edit(&mut next, id, |s| s.insert(id, rec));
    testkit::vault_changed(&mut next, id);
    let (outbox, _) = lock_boxes(&mut next);
    assert_eq!(
        outbox.len(),
        1,
        "{:?}",
        outbox.iter().map(|f| &f.0).collect::<Vec<_>>()
    );
    assert_eq!(outbox[0].0, "main.kdbx");
    assert_ne!(outbox[0].1, earlier, "the earlier copy is still there");
}

/// An open vault on a window tall enough to show the Memory dialog whole,
/// with 8000 MiB free.
fn tall_open_vault() -> Faraday {
    let file = testkit::kdbx_vault("Main", PHRASE, 0x31, Vec::new()).unwrap();
    let mut app = process_with(480, 2000, Some(8000), vec![("main.kdbx".into(), file)]);
    try_unlock(&mut app, PHRASE);
    app.press(Action::Nav(Screen::VaultContents));
    app
}

/// How many rows of the Memory dialog read as too large for `machine`.
fn too_large_rows(app: &mut Faraday, machine: &str) -> usize {
    let _ = app.frame();
    let note = EN.vault_too_large.replace("{}", machine);
    app.drawn_texts().iter().filter(|t| **t == note).count()
}

/// The Zero's place among the machines.
const ZERO: usize = 4;

#[test]
fn ticking_a_smaller_machine_in_memory_makes_a_higher_step_too_large_for_it() {
    let mut app = tall_open_vault();
    app.press(Action::Vault(V::MemoryAsk));
    let zero = EN.vaults_pi_zero_2_w;
    assert_eq!(too_large_rows(&mut app, zero), 0);
    app.press(Action::Vault(V::MemoryMachine(ZERO)));
    assert_eq!(too_large_rows(&mut app, zero), 2, "512 MiB and 1 GiB");
    app.press(Action::Vault(V::MemoryMachine(ZERO)));
    assert_eq!(too_large_rows(&mut app, zero), 0);
}

#[test]
fn a_step_over_the_ticked_limit_can_still_be_chosen_in_memory() {
    let mut app = open_vault_with_free(8000);
    app.press(Action::Vault(V::MemoryAsk));
    app.press(Action::Vault(V::MemoryMachine(ZERO)));
    app.press(Action::Vault(V::MemoryStep(3)));
    assert_eq!(app.sheet, None);
    assert_eq!(app.vaults().open[0].changes, 1);
}

#[test]
fn creates_ticks_and_the_memory_dialogs_do_not_reach_each_other() {
    let mut app = tall_open_vault();
    app.press(Action::Vault(V::Create));
    app.press(Action::Vault(V::CMachine(ZERO)));
    let before = app.vaults().create.as_ref().map(|c| (c.on, c.not_here));
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::MemoryAsk));
    assert_eq!(too_large_rows(&mut app, EN.vaults_pi_zero_2_w), 0);
    app.press(Action::Vault(V::MemoryMachine(3)));
    app.press(Action::Vault(V::MemoryHere));
    let after = app.vaults().create.as_ref().map(|c| (c.on, c.not_here));
    assert_eq!(before, after, "Memory changed Create's ticks");
}

#[test]
fn memory_opens_with_this_computer_alone() {
    let mut app = tall_open_vault();
    let zero = EN.vaults_pi_zero_2_w;
    app.press(Action::Vault(V::MemoryAsk));
    app.press(Action::Vault(V::MemoryMachine(ZERO)));
    assert_eq!(too_large_rows(&mut app, zero), 2);
    app.press(Action::Vault(V::FormCancel));
    app.press(Action::Vault(V::MemoryAsk));
    assert_eq!(too_large_rows(&mut app, zero), 0);
}

#[test]
fn a_step_this_computer_cannot_unlock_stays_refused_whatever_is_ticked() {
    let mut app = open_vault_with_free(1100);
    app.press(Action::Vault(V::MemoryAsk));
    let last = Action::Vault(V::MemoryStep(LAST_STEP));
    app.press(Action::Vault(V::MemoryHere));
    app.press(Action::Vault(V::MemoryMachine(0)));
    assert!(!crate::common::offered(&mut app, last));
    app.press(last);
    assert_eq!(app.sheet, Some(Sheet::VaultMemory));
    assert_eq!(app.vaults().open[0].changes, 0);
}

#[test]
fn a_step_refused_for_lack_of_memory_here_names_this_computer_not_the_machine_ticked() {
    let mut app = tall_open_vault();
    app.storage(StorageEvent::Memory {
        available_mib: 1100,
    });
    app.press(Action::Vault(V::MemoryAsk));
    app.press(Action::Vault(V::MemoryHere));
    app.press(Action::Vault(V::MemoryMachine(0)));
    assert_eq!(too_large_rows(&mut app, EN.vaults_other_pcs), 0);
    assert_eq!(too_large_rows(&mut app, EN.vaults_this_computer_name), 1);
}

#[test]
fn a_memory_tick_with_no_dialog_open_does_nothing() {
    let mut app = tall_open_vault();
    app.press(Action::Vault(V::MemoryMachine(ZERO)));
    app.press(Action::Vault(V::MemoryHere));
    app.press(Action::Vault(V::MemoryAsk));
    assert_eq!(too_large_rows(&mut app, EN.vaults_pi_zero_2_w), 0);
}

// ---------------------------------------------------------------------
// Unlock with a key file and a YubiKey's secret
// ---------------------------------------------------------------------

const KEY_BYTES: [u8; 32] = [7; 32];
const SECRET_HEX: &str = "abababababababababababababababababababab";

/// A vault that needs the key file `KEY_BYTES` and the secret `SECRET_HEX`
/// decodes to, with a key file of those bytes in Files.
fn keyed_app() -> Faraday {
    let file =
        testkit::kdbx_vault_keyed("Keyed", PHRASE, 3, (Some(KEY_BYTES), Some([0xab; 20]))).unwrap();
    process_at(
        800,
        1200,
        vec![
            ("keyed.kdbx".into(), file),
            ("other.keyx".into(), vec![9; 32]),
            ("vault.keyx".into(), KEY_BYTES.to_vec()),
        ],
    )
}

fn open_unlock(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    let _ = app.frame();
}

/// Chooses the second of the files that are not vaults: `vault.keyx`.
fn choose_key_file(app: &mut Faraday) {
    app.press(Action::Vault(V::KeyFileAsk));
    app.press(Action::Vault(V::KeyFilePick(1)));
    app.press(Action::Vault(V::KeyFileChoose));
}

fn type_secret(app: &mut Faraday, text: &str) {
    app.press(Action::Vault(V::YubiKeyAdd));
    type_text(app, text);
}

/// Types the passphrase and presses Unlock, waiting for the run.
fn attempt(app: &mut Faraday, phrase: &str) {
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

fn in_files(app: &Faraday, name: &str) -> bool {
    app.inbox().iter().any(|i| i.name == name)
}

#[test]
fn a_vault_made_with_a_key_file_and_a_secret_opens_from_unlock_and_the_key_file_leaves_files() {
    let mut app = keyed_app();
    open_unlock(&mut app);
    choose_key_file(&mut app);
    type_secret(&mut app, SECRET_HEX);
    attempt(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 1, "{}", page(&mut app));
    assert!(!in_files(&app, "vault.keyx"));
    assert!(in_files(&app, "keyed.kdbx"));
}

#[test]
fn a_secret_of_the_wrong_length_says_how_many_digits_it_has_and_nothing_runs() {
    let mut app = keyed_app();
    open_unlock(&mut app);
    type_secret(&mut app, "abcd");
    app.press(Action::Vault(V::FocusPassphrase));
    type_text(&mut app, PHRASE);
    app.press(Action::Vault(V::Unlock));
    assert!(app.vaults().working.is_none());
    let texts = page(&mut app);
    assert!(texts.contains("This has 4."), "{texts}");
}

#[test]
fn a_key_file_that_is_damaged_is_refused_naming_it() {
    let bad = br#"<?xml version="1.0"?><KeyFile><Meta><Version>1.0</Version></Meta><Key><Data>AA<x/>AA</Data></Key></KeyFile>"#;
    let file = testkit::kdbx_vault("Plain", PHRASE, 4, Vec::new()).unwrap();
    let mut app = process_at(
        800,
        1200,
        vec![
            ("plain.kdbx".into(), file),
            ("bad.keyx".into(), bad.to_vec()),
        ],
    );
    open_unlock(&mut app);
    app.press(Action::Vault(V::KeyFileAsk));
    app.press(Action::Vault(V::KeyFileChoose));
    app.press(Action::Vault(V::FocusPassphrase));
    type_text(&mut app, PHRASE);
    app.press(Action::Vault(V::Unlock));
    assert!(app.vaults().working.is_none());
    let texts = page(&mut app);
    assert!(
        texts.contains("bad.keyx is damaged as a key file."),
        "{texts}"
    );
    assert!(in_files(&app, "bad.keyx"));
}

#[test]
fn the_wrong_credentials_message_follows_what_was_tried_and_files_keep_the_key_file() {
    let mut app = keyed_app();
    open_unlock(&mut app);
    // A wrong key file, then a wrong secret, then both.
    let said = |app: &mut Faraday, line: &str| {
        let texts = page(app);
        assert!(texts.contains(line), "{line}: {texts}");
    };
    attempt(&mut app, "wrong");
    said(&mut app, EN.vault_error_wrong);
    app.press(Action::Vault(V::KeyFileAsk));
    app.press(Action::Vault(V::KeyFilePick(0)));
    app.press(Action::Vault(V::KeyFileChoose));
    attempt(&mut app, PHRASE);
    said(&mut app, EN.vault_error_wrong_key_file);
    app.press(Action::Vault(V::KeyFileDrop));
    type_secret(&mut app, SECRET_HEX.replace("ab", "cd").as_str());
    attempt(&mut app, PHRASE);
    said(&mut app, EN.vault_error_wrong_yubikey);
    choose_key_file(&mut app);
    attempt(&mut app, PHRASE);
    said(&mut app, EN.vault_error_wrong_both);
    assert!(in_files(&app, "other.keyx") && in_files(&app, "vault.keyx"));
}

// ---- Create a vault with a key file ----

/// Create vault opened and both passphrases typed, in a device whose Files
/// hold `inbox`.
fn create_form(inbox: Files) -> Faraday {
    let mut app = process_at(800, 1200, inbox);
    app.press(Action::Vault(V::Create));
    for second in [false, true] {
        app.press(Action::Vault(V::CFocus(0, second)));
        type_text(&mut app, PHRASE);
    }
    app
}

/// Presses Create vault and waits for the run.
fn press_create(app: &mut Faraday) {
    app.press(Action::Vault(V::CGo));
    for t in 1..60u64 {
        if app.screen == Screen::Unlock
            || app
                .vaults()
                .create
                .as_ref()
                .is_some_and(|c| c.error.is_some())
        {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

fn outbox_names(app: &Faraday) -> Vec<String> {
    app.outbox.iter().map(|i| i.name.clone()).collect()
}

#[test]
fn a_vault_made_with_a_key_file_made_at_create_opens_with_the_keyx_from_the_outbox() {
    let mut app = create_form(Vec::new());
    app.press(Action::Vault(V::CKeyAsk));
    app.press(Action::Vault(V::CKeyMake));
    assert!(
        app.vaults()
            .create
            .as_ref()
            .is_some_and(|c| c.key == KeyChoice::Make)
    );
    press_create(&mut app);
    let names = outbox_names(&app);
    assert!(names.contains(&"vault.kdbx".to_string()), "{names:?}");
    assert!(names.contains(&"vault.keyx".to_string()), "{names:?}");
    // The person carries the key file to Files; the vault is picked at Unlock.
    faraday_core::testkit::outbox_to_files(
        &mut app,
        &["vault.keyx".to_string(), "vault.kdbx".to_string()],
    );
    app.press(Action::Vault(V::KeyFileAsk));
    app.press(Action::Vault(V::KeyFileChoose));
    attempt(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 1, "{}", page(&mut app));
}

#[test]
fn a_vault_made_with_a_chosen_file_opens_with_it_and_the_file_leaves_files_at_create() {
    let mut app = create_form(vec![("mine.keyx".into(), KEY_BYTES.to_vec())]);
    app.press(Action::Vault(V::CKeyAsk));
    app.press(Action::Vault(V::CKeyChoose));
    app.press(Action::Vault(V::KeyFilePick(0)));
    app.press(Action::Vault(V::KeyFileChoose));
    assert!(
        app.vaults()
            .create
            .as_ref()
            .is_some_and(|c| c.key == KeyChoice::File("mine.keyx".into()))
    );
    press_create(&mut app);
    assert!(!in_files(&app, "mine.keyx"));
    assert!(!outbox_names(&app).iter().any(|n| n.ends_with(".keyx")));
    let vault = app
        .outbox
        .iter()
        .find(|i| i.name == "vault.kdbx")
        .expect("the vault was not made")
        .bytes
        .to_vec();
    // The same bytes brought back as a file open the vault.
    let mut again = process_at(
        800,
        1200,
        vec![
            ("vault.kdbx".into(), vault),
            ("mine.keyx".into(), KEY_BYTES.to_vec()),
        ],
    );
    open_unlock(&mut again);
    choose_key_file_at(&mut again, 0);
    attempt(&mut again, PHRASE);
    assert_eq!(again.vaults().open.len(), 1, "{}", page(&mut again));
}

fn choose_key_file_at(app: &mut Faraday, n: usize) {
    app.press(Action::Vault(V::KeyFileAsk));
    app.press(Action::Vault(V::KeyFilePick(n)));
    app.press(Action::Vault(V::KeyFileChoose));
}

#[test]
fn cancelling_the_key_file_dialog_makes_no_key_file() {
    let mut app = create_form(Vec::new());
    app.press(Action::Vault(V::CKeyAsk));
    app.press(Action::Vault(V::CKeyCancel));
    assert!(app.sheet.is_none());
    press_create(&mut app);
    assert!(!outbox_names(&app).iter().any(|n| n.ends_with(".keyx")));
    assert!(outbox_names(&app).contains(&"vault.kdbx".to_string()));
}

#[test]
fn taking_the_chip_back_before_create_makes_no_key_file() {
    let mut app = create_form(Vec::new());
    app.press(Action::Vault(V::CKeyAsk));
    app.press(Action::Vault(V::CKeyMake));
    app.press(Action::Vault(V::CKeyDrop));
    press_create(&mut app);
    assert!(!outbox_names(&app).iter().any(|n| n.ends_with(".keyx")));
}

#[test]
fn a_chosen_file_that_is_damaged_is_named_and_nothing_is_made() {
    let bad = br#"<?xml version="1.0"?><KeyFile><Meta><Version>1.0</Version></Meta><Key><Data>AA<x/>AA</Data></Key></KeyFile>"#;
    let mut app = create_form(vec![("bad.keyx".into(), bad.to_vec())]);
    app.press(Action::Vault(V::CKeyAsk));
    app.press(Action::Vault(V::CKeyChoose));
    app.press(Action::Vault(V::KeyFileChoose));
    press_create(&mut app);
    let said = app.vaults().create.as_ref().and_then(|c| c.error.clone());
    assert_eq!(said.as_deref(), Some("bad.keyx is damaged as a key file."));
    assert!(app.outbox.is_empty());
    assert!(in_files(&app, "bad.keyx"));
}

// ---------------------------------------------------------------------
// The open vault's ⋮: Key file and Remove YubiKey secret
// ---------------------------------------------------------------------

/// The keyed vault open, unlocked with its key file and secret; one more
/// key file, `other.keyx`, is left in Files.
fn open_keyed() -> Faraday {
    let mut app = keyed_app();
    open_unlock(&mut app);
    choose_key_file(&mut app);
    type_secret(&mut app, SECRET_HEX);
    attempt(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 1, "{}", page(&mut app));
    app.press(Action::Nav(Screen::VaultContents));
    let _ = app.frame();
    app
}

/// Locks, waits for the seal and returns every file in the Outbox.
fn lock_to_outbox(app: &mut Faraday) -> Files {
    crate::common::top_up(app);
    app.press(Action::Lock);
    testkit::settle(app);
    app.outbox
        .iter()
        .map(|i| (i.name.clone(), i.bytes.to_vec()))
        .collect()
}

fn file_named(files: &Files, suffix: &str) -> Vec<u8> {
    files
        .iter()
        .find(|f| f.0.ends_with(suffix))
        .unwrap_or_else(|| {
            panic!(
                "no {suffix} in {:?}",
                files.iter().map(|f| &f.0).collect::<Vec<_>>()
            )
        })
        .1
        .clone()
}

/// Opens the vault `sealed` from Unlock with the files in `keys` and the
/// secret if `secret`; whether it opened.
fn opens(sealed: &[u8], keys: &[(&str, Vec<u8>)], secret: bool) -> bool {
    let mut files: Files = vec![("keyed.kdbx".into(), sealed.to_vec())];
    files.extend(keys.iter().map(|(n, b)| ((*n).to_string(), b.clone())));
    let mut app = process_at(800, 1200, files);
    open_unlock(&mut app);
    if !keys.is_empty() {
        app.press(Action::Vault(V::KeyFileAsk));
        app.press(Action::Vault(V::KeyFilePick(0)));
        app.press(Action::Vault(V::KeyFileChoose));
    }
    if secret {
        type_secret(&mut app, SECRET_HEX);
    }
    attempt(&mut app, PHRASE);
    !app.vaults().open.is_empty()
}

/// Presses the ⋮ and then `item`.
fn more_then(app: &mut Faraday, item: V) {
    let _ = app.frame();
    app.press(Action::Vault(V::MoreAsk));
    app.press(Action::Vault(item));
}

#[test]
fn make_a_key_file_puts_the_keyx_in_the_outbox_at_once_and_the_sealed_vault_needs_it() {
    let mut app = open_keyed();
    more_then(&mut app, V::VKeyMake);
    assert!(outbox_names(&app).contains(&"keyed.keyx".to_string()));
    assert!(app.vaults().open[0].changes > 0);
    let out = lock_to_outbox(&mut app);
    let keyx = file_named(&out, ".keyx");
    let sealed = file_named(&out, ".kdbx");
    assert!(opens(&sealed, &[("keyed.keyx", keyx)], true));
    assert!(!opens(&sealed, &[], true));
    // The key file the vault had before is not the one it needs now.
    assert!(!opens(&sealed, &[("vault.keyx", KEY_BYTES.to_vec())], true));
}

#[test]
fn a_made_key_file_never_takes_the_name_of_a_file_already_there() {
    let file = testkit::kdbx_vault_keyed("Keyed", PHRASE, 3, (Some(KEY_BYTES), None)).unwrap();
    let mut app = process_at(
        800,
        1200,
        vec![
            ("keyed.kdbx".into(), file),
            ("keyed.keyx".into(), vec![1, 2, 3]),
            ("vault.keyx".into(), KEY_BYTES.to_vec()),
        ],
    );
    open_unlock(&mut app);
    app.press(Action::Vault(V::KeyFileAsk));
    app.press(Action::Vault(V::KeyFilePick(1)));
    app.press(Action::Vault(V::KeyFileChoose));
    attempt(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 1, "{}", page(&mut app));
    app.press(Action::Nav(Screen::VaultContents));
    more_then(&mut app, V::VKeyMake);
    let names = outbox_names(&app);
    assert_eq!(names, vec!["keyed-2.keyx".to_string()]);
    assert!(in_files(&app, "keyed.keyx"));
}

#[test]
fn a_made_key_file_taken_back_or_replaced_before_a_seal_leaves_no_keyx_in_the_outbox() {
    let mut app = open_keyed();
    more_then(&mut app, V::VKeyMake);
    assert_eq!(outbox_names(&app).len(), 1);
    // Another Make: the first one's file goes, the second stays.
    more_then(&mut app, V::VKeyMake);
    assert_eq!(outbox_names(&app).len(), 1);
    // A chosen file replaces it.
    more_then(&mut app, V::VKeyChoose);
    app.press(Action::Vault(V::KeyFilePick(0)));
    app.press(Action::Vault(V::KeyFileChoose));
    assert!(outbox_names(&app).is_empty(), "{:?}", outbox_names(&app));
    // And Remove key file takes a made one back.
    more_then(&mut app, V::VKeyMake);
    assert_eq!(outbox_names(&app).len(), 1);
    more_then(&mut app, V::VKeyRemove);
    assert!(outbox_names(&app).is_empty());
}

#[test]
fn a_copy_of_a_file_with_the_made_keyx_name_written_to_a_stick_keeps_the_key_to_take_back() {
    let mut app = open_keyed();
    more_then(&mut app, V::VKeyMake);
    assert_eq!(outbox_names(&app), vec!["keyed.keyx".to_string()]);
    // A file of the same name comes in from one stick and goes to another.
    app.storage(StorageEvent::Read {
        stick: "a".into(),
        name: "keyed.keyx".into(),
        bytes: vec![1, 2, 3],
    });
    // Its write to another stick was started and is answered now.
    testkit::inbox_write_started(&mut app, "keyed.keyx");
    app.storage(StorageEvent::Written {
        stick: "b".into(),
        wrote_as: "keyed.keyx".into(),
        name: "keyed.keyx".into(),
    });
    // The Outbox's own key file is still pending: taking it out takes the
    // key back.
    let at = app
        .outbox
        .iter()
        .position(|i| i.name == "keyed.keyx")
        .expect("still in the Outbox");
    app.press(Action::OutboxRemove(at));
    assert_eq!(
        app.vaults().open[0].changes,
        0,
        "the key was not taken back"
    );
    let out = lock_to_outbox(&mut app);
    assert!(!out.iter().any(|f| f.0.ends_with(".kdbx")), "sealed");
}

#[test]
fn a_made_key_file_stays_in_the_outbox_once_a_seal_wrote_it() {
    let mut app = open_keyed();
    more_then(&mut app, V::VKeyMake);
    let out = lock_to_outbox(&mut app);
    assert!(out.iter().any(|f| f.0.ends_with(".keyx")));
}

#[test]
fn a_made_key_file_stays_in_the_outbox_when_one_vault_is_locked_and_the_app_goes_on() {
    let mut app = open_keyed();
    more_then(&mut app, V::VKeyMake);
    crate::common::top_up(&mut app);
    testkit::vault_lock_one(&mut app, 0);
    testkit::settle(&mut app);
    for t in 1..5u64 {
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert!(app.vaults().open.is_empty(), "the vault is still open");
    let names = outbox_names(&app);
    assert!(names.iter().any(|n| n.ends_with(".kdbx")), "{names:?}");
    assert!(names.iter().any(|n| n.ends_with(".keyx")), "{names:?}");
}

#[test]
fn a_made_key_file_of_a_vault_let_go_unsaved_leaves_the_outbox() {
    let mut app = open_keyed();
    more_then(&mut app, V::VKeyMake);
    assert_eq!(outbox_names(&app).len(), 1);
    testkit::vault_let_go(&mut app, 0);
    for t in 1..5u64 {
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert!(outbox_names(&app).is_empty(), "{:?}", outbox_names(&app));
}

#[test]
fn a_made_key_file_of_a_vault_a_lock_without_saving_loses_does_not_reach_the_saved_outbox() {
    let mut app = open_keyed();
    more_then(&mut app, V::VKeyMake);
    assert_eq!(outbox_names(&app).len(), 1);
    // This computer is short of the memory the seal needs.
    crate::common::top_up(&mut app);
    app.storage(StorageEvent::Memory { available_mib: 20 });
    app.press(Action::Lock);
    testkit::settle(&mut app);
    assert!(!app.vaults().open.is_empty(), "the vault was let go");
    assert_eq!(outbox_names(&app).len(), 1);
    app.press(Action::LockUnsealed);
    testkit::settle(&mut app);
    assert!(app.restart_requested());
    assert!(app.vaults().open.is_empty());
    let saved = saved_outbox(&mut app);
    assert!(
        saved.is_empty(),
        "{:?}",
        saved.iter().map(|f| &f.0).collect::<Vec<_>>()
    );
    assert!(outbox_names(&app).is_empty(), "{:?}", outbox_names(&app));
}

#[test]
fn a_chosen_key_file_opens_the_sealed_vault_and_leaves_files_only_after_the_seal() {
    let mut app = open_keyed();
    more_then(&mut app, V::VKeyChoose);
    app.press(Action::Vault(V::KeyFilePick(0)));
    app.press(Action::Vault(V::KeyFileChoose));
    assert!(in_files(&app, "other.keyx"), "left Files before the seal");
    assert!(app.vaults().open[0].changes > 0);
    let out = lock_to_outbox(&mut app);
    assert!(!in_files(&app, "other.keyx"));
    let sealed = file_named(&out, ".kdbx");
    assert!(opens(&sealed, &[("other.keyx", vec![9; 32])], true));
    assert!(!opens(&sealed, &[("vault.keyx", KEY_BYTES.to_vec())], true));
    assert!(!opens(&sealed, &[], true));
}

#[test]
fn a_chosen_file_that_is_damaged_is_named_and_changes_nothing() {
    let bad = br#"<?xml version="1.0"?><KeyFile><Meta><Version>1.0</Version></Meta><Key><Data>AA<x/>AA</Data></Key></KeyFile>"#;
    let file = testkit::kdbx_vault_keyed("Keyed", PHRASE, 3, (Some(KEY_BYTES), None)).unwrap();
    let mut app = process_at(
        800,
        1200,
        vec![
            ("keyed.kdbx".into(), file),
            ("bad.keyx".into(), bad.to_vec()),
            ("vault.keyx".into(), KEY_BYTES.to_vec()),
        ],
    );
    open_unlock(&mut app);
    app.press(Action::Vault(V::KeyFileAsk));
    app.press(Action::Vault(V::KeyFilePick(1)));
    app.press(Action::Vault(V::KeyFileChoose));
    attempt(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 1, "{}", page(&mut app));
    app.press(Action::Nav(Screen::VaultContents));
    more_then(&mut app, V::VKeyChoose);
    app.press(Action::Vault(V::KeyFilePick(0)));
    app.press(Action::Vault(V::KeyFileChoose));
    assert!(
        app.toast_text()
            .is_some_and(|t| t.contains("bad.keyx is damaged")),
        "{:?}",
        app.toast_text()
    );
    assert_eq!(app.vaults().open[0].changes, 0);
    assert!(in_files(&app, "bad.keyx"));
}

#[test]
fn a_failed_seal_keeps_a_chosen_file_in_files() {
    let mut app = open_keyed();
    more_then(&mut app, V::VKeyChoose);
    app.press(Action::Vault(V::KeyFilePick(0)));
    app.press(Action::Vault(V::KeyFileChoose));
    // A new memory step whose seal fails, and so does the retry at the old one.
    let have = app.vaults().open[0].facts().header.cost.memory_kib;
    let pick = if have == faraday_vault::file::STEPS_KIB[1] {
        2
    } else {
        1
    };
    app.press(Action::Vault(V::MemoryAsk));
    let _ = app.frame();
    app.press(Action::Vault(V::MemoryStep(pick)));
    crate::common::top_up(&mut app);
    testkit::fail_next_step_seal_and_retry();
    app.press(Action::Lock);
    testkit::settle(&mut app);
    assert!(!app.vaults().open.is_empty(), "the vault was let go");
    assert!(in_files(&app, "other.keyx"));
}

#[test]
fn remove_key_file_seals_a_vault_that_opens_without_one() {
    let mut app = open_keyed();
    more_then(&mut app, V::VKeyRemove);
    assert!(app.vaults().open[0].changes > 0);
    let out = lock_to_outbox(&mut app);
    let sealed = file_named(&out, ".kdbx");
    assert!(opens(&sealed, &[], true));
}

#[test]
fn remove_yubikey_secret_seals_a_vault_that_opens_without_it() {
    let mut app = open_keyed();
    more_then(&mut app, V::YubiKeyRemoveAsk);
    assert_eq!(app.sheet, Some(Sheet::Trim));
    app.press(Action::Vault(V::TrimGo));
    assert!(app.vaults().open[0].changes > 0);
    let out = lock_to_outbox(&mut app);
    let sealed = file_named(&out, ".kdbx");
    let key = ("vault.keyx", KEY_BYTES.to_vec());
    assert!(opens(&sealed, std::slice::from_ref(&key), false));
    assert!(!opens(&sealed, &[], false));
}

#[test]
fn a_removal_taken_back_still_needs_the_secret() {
    let mut app = open_keyed();
    more_then(&mut app, V::YubiKeyRemoveAsk);
    app.press(Action::Vault(V::TrimGo));
    // A real change in between.
    let id = app.vaults().open[0].id;
    let rec = faraday_vault::Record::new(faraday_vault::records::kind::ENTRY)
        .with(faraday_vault::records::field::TITLE, b"edit");
    testkit::vault_edit(&mut app, id, |s| s.insert(id, rec));
    testkit::vault_changed(&mut app, id);
    // The menu now offers Keep.
    let _ = app.frame();
    app.press(Action::Vault(V::MoreAsk));
    let texts = page(&mut app);
    assert!(texts.contains(EN.vault_keep_yubikey), "{texts}");
    assert!(!texts.contains(EN.vault_remove_yubikey), "{texts}");
    app.press(Action::Vault(V::YubiKeyRemoveAsk));
    app.press(Action::Vault(V::TrimGo));
    let out = lock_to_outbox(&mut app);
    // The edit seals, and the vault still needs both.
    let sealed = file_named(&out, ".kdbx");
    let key = ("vault.keyx", KEY_BYTES.to_vec());
    assert!(!opens(&sealed, std::slice::from_ref(&key), false));
    assert!(opens(&sealed, &[key], true));
}

#[test]
fn the_menu_offers_key_file_always_and_remove_yubikey_only_with_a_secret() {
    let mut app = open_keyed();
    let _ = app.frame();
    app.press(Action::Vault(V::MoreAsk));
    let texts = page(&mut app);
    assert!(texts.contains(EN.vault_key_file), "{texts}");
    assert!(texts.contains(EN.vault_remove_yubikey), "{texts}");

    let mut app = open_vault();
    let _ = app.frame();
    app.press(Action::Vault(V::MoreAsk));
    let texts = page(&mut app);
    assert!(texts.contains(EN.vault_key_file), "{texts}");
    assert!(!texts.contains(EN.vault_remove_yubikey), "{texts}");
}

#[test]
fn a_read_only_vault_has_no_menu() {
    let [(_, file, _), _] = read_only_files();
    let mut app = process(vec![("ro.kdbx".into(), file)]);
    try_unlock(&mut app, PHRASE);
    app.press(Action::Nav(Screen::VaultContents));
    let _ = app.frame();
    app.press(Action::Vault(V::MoreAsk));
    assert_eq!(app.sheet, None);
    app.press(Action::Vault(V::VKeyAsk));
    assert_eq!(app.sheet, None);
}

#[test]
fn the_head_line_names_what_the_next_seal_writes() {
    let mut app = open_keyed();
    let texts = page(&mut app);
    assert!(texts.contains(EN.vault_needs_key_file), "{texts}");
    assert!(texts.contains(EN.vault_opens_with_yubikey), "{texts}");
    more_then(&mut app, V::VKeyRemove);
    let texts = head(&mut app);
    assert!(!texts.contains(EN.vault_needs_key_file), "{texts}");
    assert!(texts.contains(EN.vault_opens_with_yubikey), "{texts}");
    more_then(&mut app, V::YubiKeyRemoveAsk);
    app.press(Action::Vault(V::TrimGo));
    let texts = head(&mut app);
    assert!(!texts.contains(EN.vault_opens_with_yubikey), "{texts}");
}

#[test]
fn a_lock_seals_a_vault_whose_only_change_is_a_key_file_or_a_removed_secret() {
    let mut app = open_keyed();
    more_then(&mut app, V::YubiKeyRemoveAsk);
    app.press(Action::Vault(V::TrimGo));
    let out = lock_to_outbox(&mut app);
    assert!(out.iter().any(|f| f.0.ends_with(".kdbx")));
}

fn keyx_bytes(app: &Faraday, name: &str) -> Option<Vec<u8>> {
    app.outbox
        .iter()
        .find(|i| i.name == name)
        .map(|i| i.bytes.to_vec())
}

#[test]
fn each_open_vault_keeps_its_own_pending_keyx() {
    let a = testkit::kdbx_vault("A", PHRASE, 0x31, Vec::new()).unwrap();
    let b = testkit::kdbx_vault("B", PHRASE, 0x32, Vec::new()).unwrap();
    let mut app = process(vec![("a.kdbx".into(), a), ("b.kdbx".into(), b)]);
    try_unlock(&mut app, PHRASE);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(1)));
    let _ = app.frame();
    app.press(Action::Vault(V::FocusPassphrase));
    type_text(&mut app, PHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if app.vaults().open.len() == 2 {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 2);
    let on = |app: &mut Faraday, at: usize| {
        testkit::vault_current(app, at);
        app.press(Action::Nav(Screen::VaultContents));
    };
    on(&mut app, 0);
    more_then(&mut app, V::VKeyMake);
    let first = outbox_names(&app);
    assert_eq!(first.len(), 1);
    let old = keyx_bytes(&app, &first[0]).unwrap();
    on(&mut app, 1);
    more_then(&mut app, V::VKeyMake);
    assert_eq!(outbox_names(&app).len(), 2);
    on(&mut app, 0);
    more_then(&mut app, V::VKeyMake);
    let names = outbox_names(&app);
    assert_eq!(names.len(), 2, "{names:?}");
    assert_ne!(keyx_bytes(&app, &first[0]).unwrap(), old);
}

#[test]
fn taking_a_pending_keyx_out_of_the_outbox_takes_its_key_back() {
    let mut app = open_keyed();
    more_then(&mut app, V::VKeyMake);
    assert_eq!(app.vaults().open[0].changes, 1);
    app.press(Action::OutboxRemove(0));
    assert!(outbox_names(&app).is_empty());
    // Nothing is left to seal.
    assert_eq!(app.vaults().open[0].changes, 0);
    let out = lock_to_outbox(&mut app);
    assert!(!out.iter().any(|f| f.0.ends_with(".kdbx")), "sealed");
}

#[test]
fn a_key_file_made_twice_counts_once_and_taking_its_keyx_back_leaves_nothing_to_seal() {
    let mut app = open_keyed();
    more_then(&mut app, V::VKeyMake);
    more_then(&mut app, V::VKeyMake);
    assert_eq!(app.vaults().open[0].changes, 1);
    assert_eq!(outbox_names(&app).len(), 1);
    app.press(Action::OutboxRemove(0));
    assert!(outbox_names(&app).is_empty());
    assert_eq!(app.vaults().open[0].changes, 0);
    let out = lock_to_outbox(&mut app);
    assert!(!out.iter().any(|f| f.0.ends_with(".kdbx")), "sealed");
}

#[test]
fn a_key_file_chosen_then_made_counts_once() {
    let mut app = open_keyed();
    more_then(&mut app, V::VKeyChoose);
    app.press(Action::Vault(V::KeyFilePick(0)));
    app.press(Action::Vault(V::KeyFileChoose));
    more_then(&mut app, V::VKeyMake);
    assert_eq!(app.vaults().open[0].changes, 1);
    app.press(Action::OutboxRemove(0));
    // The chosen key was replaced by the made one, which is taken back:
    // no key file change is waiting.
    assert_eq!(app.vaults().open[0].changes, 0);
}

#[test]
fn taking_a_pending_keyx_out_of_the_outbox_leaves_a_real_change_to_seal_with_the_vaults_own_key() {
    let mut app = open_keyed();
    more_then(&mut app, V::VKeyMake);
    let id = app.vaults().open[0].id;
    let rec = faraday_vault::Record::new(faraday_vault::records::kind::ENTRY)
        .with(faraday_vault::records::field::TITLE, b"edit");
    testkit::vault_edit(&mut app, id, |s| s.insert(id, rec));
    testkit::vault_changed(&mut app, id);
    app.press(Action::OutboxRemove(0));
    let out = lock_to_outbox(&mut app);
    let sealed = file_named(&out, ".kdbx");
    let key = ("vault.keyx", KEY_BYTES.to_vec());
    assert!(opens(&sealed, &[key], true));
}

#[test]
fn keeping_the_secret_after_removing_it_leaves_nothing_to_seal() {
    let mut app = open_keyed();
    more_then(&mut app, V::YubiKeyRemoveAsk);
    app.press(Action::Vault(V::TrimGo));
    assert_eq!(app.vaults().open[0].changes, 1);
    more_then(&mut app, V::YubiKeyRemoveAsk);
    app.press(Action::Vault(V::TrimGo));
    assert_eq!(app.vaults().open[0].changes, 0);
    let out = lock_to_outbox(&mut app);
    assert!(!out.iter().any(|f| f.0.ends_with(".kdbx")), "sealed");
}

#[test]
fn change_passphrase_waits_while_the_two_differ() {
    let mut app = open_vault();
    app.press(Action::Vault(V::ChangePassphrase));
    type_text(&mut app, "one phrase");
    app.press(Action::Vault(V::FocusField(1)));
    type_text(&mut app, "another phrase");
    let _ = app.frame();
    assert!(
        app.drawn_texts().iter().any(|t| t == EN.pass_pair_differ),
        "the line says the two differ"
    );
    assert!(!app.offers(Action::Vault(V::FormSave)), "Save waits");
}
