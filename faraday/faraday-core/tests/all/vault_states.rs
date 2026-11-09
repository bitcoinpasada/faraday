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
