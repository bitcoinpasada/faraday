//! A ⋮ on every file (`docs/FLOWS.md` decision 61): each file row in
//! Files and on the stick visit keeps its one action as a button and the
//! rest behind its ⋮; a file For the stick is renamed before it is
//! written, a file on the stick is renamed on the stick, and Details says
//! a file's SHA-256.

use crate::common::*;
use faraday_core::file_menu::{FilePress, Row};
use faraday_core::testkit::Plain;
use faraday_core::wallet::FileKind;
use faraday_core::{Action, BootMedium, Faraday, Item, Screen, Sheet, StorageEvent};
use faraday_shell_api::{App, Event, Key};

fn file(name: &str, bytes: &[u8]) -> (String, Vec<u8>) {
    (name.to_string(), bytes.to_vec())
}

/// The ⋮'s menu of `row`, opened by its press: each choice's words.
fn menu_of(app: &mut Faraday, row: Row) -> Vec<String> {
    let _ = app.frame();
    let dots = Action::File(FilePress::Menu(row));
    scroll_to(app, dots);
    assert!(app.offers(dots), "{row:?} has a ⋮");
    app.press(dots);
    assert_eq!(app.sheet, Some(Sheet::FileMenu));
    app.files_menu()
        .menu
        .as_ref()
        .expect("its menu")
        .lines
        .iter()
        .map(|(l, _)| l.clone())
        .collect()
}

/// The wheel turned over the stick's file list until `action` is drawn:
/// a row below the window is reached by scrolling to it.
fn scroll_to(app: &mut Faraday, action: Action) {
    for _ in 0..20 {
        if app.offers(action) {
            return;
        }
        let names: Vec<String> = app.sticks()[0]
            .files
            .iter()
            .map(|(n, _)| String::from(&**n))
            .collect();
        let (x, y) = app
            .drawn_text_boxes()
            .into_iter()
            .find(|(t, _)| names.iter().any(|n| n.as_str() == &**t))
            .map(|(_, (x, y, w, h))| (x + w / 2, y + h / 2))
            .expect("a file of the stick is drawn");
        app.event(Event::Wheel {
            x: x as u16,
            y: y as u16,
            dy: 120,
        });
        app.settle();
        let _ = app.frame();
    }
}

/// The choice in the open menu that reads `label`, pressed.
fn choose(app: &mut Faraday, label: &str) {
    let a = app
        .files_menu()
        .menu
        .as_ref()
        .and_then(|m| m.lines.iter().find(|(l, _)| l == label).map(|(_, a)| *a))
        .unwrap_or_else(|| panic!("{label} is in the menu"));
    let _ = app.frame();
    assert!(app.offers(a), "{label} is drawn in the menu");
    app.press(a);
}

/// The Rename popup's field cleared and `stem` typed in it.
fn type_name(app: &mut Faraday, stem: &str) {
    assert_eq!(app.sheet, Some(Sheet::Rename));
    let n = app
        .files_menu()
        .renaming
        .as_ref()
        .map_or(0, |r| r.stem.chars().count());
    for _ in 0..n {
        app.event(Event::Key(Key::Backspace));
    }
    type_text(app, stem);
}

/// Rename in the open popup: whether it was offered, and pressed.
fn rename_offered(app: &mut Faraday) -> bool {
    let _ = app.frame();
    app.offers(Action::File(FilePress::RenameGo))
}

/// A process with these files For the stick and a stick holding
/// `on_stick`, on its visit, every file For the stick chosen.
fn visiting(outbox: Files, on_stick: Files) -> (Shell, Faraday) {
    visiting_as(false, outbox, on_stick)
}

/// [`visiting`], the stick's vault files ones the session saw open when
/// `known`.
fn visiting_as(known: bool, outbox: Files, on_stick: Files) -> (Shell, Faraday) {
    let mut shell = Shell::new();
    let mut d = Disk::empty("S1", "TESTSTICK");
    d.files = on_stick;
    shell.add(d);
    let mut app = process(Vec::new(), outbox, Vec::new());
    if known {
        shell.plug_known(&mut app, "S1");
    } else {
        shell.plug(&mut app, "S1");
    }
    shell.press(&mut app, Action::Nav(Screen::Files));
    let names: Vec<String> = app.outbox.iter().map(|i| String::from(&*i.name)).collect();
    for (i, n) in names.iter().enumerate() {
        if !app.visit().out.contains(n.as_str()) {
            shell.press(&mut app, Action::VisitOut(i));
        }
    }
    (shell, app)
}

#[test]
fn a_file_for_the_stick_renamed_is_written_under_its_new_name() {
    let mut app = process(
        Vec::new(),
        vec![file("savings-signed.psbt", b"psbt bytes")],
        Vec::new(),
    );
    app.press(Action::Nav(Screen::Files));
    let words = menu_of(&mut app, Row::Out(0));
    assert!(words.iter().any(|w| w == "Rename"), "{words:?}");
    choose(&mut app, "Rename");
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Rename savings-signed.psbt"),
        "{texts:?}"
    );
    assert!(texts.iter().any(|t| t == ".psbt"), "the extension, fixed");
    type_name(&mut app, "march-rent");
    assert!(rename_offered(&mut app));
    app.press(Action::File(FilePress::RenameGo));
    assert_eq!(app.sheet, None);
    assert_eq!(app.outbox[0].name, "march-rent.psbt");

    let mut shell = Shell::new();
    shell.add(Disk::empty("S1", "TESTSTICK"));
    shell.plug(&mut app, "S1");
    shell.press(&mut app, Action::Nav(Screen::Files));
    if !app.visit().out.contains("march-rent.psbt") {
        shell.press(&mut app, Action::VisitOut(0));
    }
    shell.press(&mut app, Action::VisitWrite);
    let d = shell.disk("S1");
    assert_eq!(
        d.file("march-rent.psbt").map(Vec::as_slice),
        Some(&b"psbt bytes"[..])
    );
    assert!(d.file("savings-signed.psbt").is_none());
}

#[test]
fn a_vault_renamed_goes_over_the_same_vault_on_the_stick_under_its_new_name() {
    let v = vault("Savings", "pass one", 7);
    let (mut shell, mut app) = visiting(
        vec![file("vault.kdbx", &v)],
        vec![file("vault.kdbx", &v), file("notes.txt", b"n")],
    );
    app.press(Action::Nav(Screen::Files));
    menu_of(&mut app, Row::Out(0));
    choose(&mut app, "Rename");
    assert_eq!(
        app.files_menu().renaming.as_ref().map(|r| r.ext.as_str()),
        Some(".kdbx"),
        "a vault keeps .kdbx"
    );
    type_name(&mut app, "savings");
    app.press(Action::File(FilePress::RenameGo));
    assert_eq!(app.outbox[0].name, "savings.kdbx");
    shell.press(&mut app, Action::Nav(Screen::Files));
    shell.press(&mut app, Action::VisitWrite);
    let vaults: Vec<&str> = shell
        .disk("S1")
        .files
        .iter()
        .map(|(n, _)| n.as_str())
        .filter(|n| n.ends_with(".kdbx"))
        .collect();
    assert_eq!(vaults, ["savings.kdbx"], "one copy, under its new name");
}

#[test]
fn rename_refuses_an_empty_name_and_one_another_file_for_the_stick_has() {
    let mut app = process(
        Vec::new(),
        vec![file("a.psbt", b"a"), file("b.psbt", b"b")],
        Vec::new(),
    );
    app.press(Action::Nav(Screen::Files));
    menu_of(&mut app, Row::Out(0));
    choose(&mut app, "Rename");
    type_name(&mut app, "");
    assert!(!rename_offered(&mut app), "an empty name");
    assert!(app.drawn_texts().iter().any(|t| t == "Type a name"));
    type_name(&mut app, "B");
    assert!(!rename_offered(&mut app), "b.psbt is For the stick already");
    let texts = app.drawn_texts();
    assert!(
        texts
            .iter()
            .any(|t| t == "Another file for the stick is named B.psbt"),
        "{texts:?}"
    );
    type_name(&mut app, ".hidden");
    assert!(!rename_offered(&mut app), "a name starting with a dot");
    type_name(&mut app, "x:y");
    assert!(!rename_offered(&mut app), "a character a stick cannot name");
    app.press(Action::File(FilePress::RenameGo));
    assert_eq!(app.outbox[0].name, "a.psbt", "nothing renamed");
    app.event(Event::Key(Key::Escape));
    assert_eq!(app.sheet, None);
}

#[test]
fn a_file_on_the_stick_renamed_is_listed_under_its_new_name_with_its_bytes() {
    let (mut shell, mut app) = visiting(
        Vec::new(),
        vec![file("old-notes.txt", b"the notes"), file("x.psbt", b"p")],
    );
    let k = app.sticks()[0]
        .files
        .iter()
        .position(|(n, _)| n == "old-notes.txt")
        .unwrap();
    let words = menu_of(&mut app, Row::Stick(k));
    assert_eq!(words, ["Rename", "Delete from the stick", "Details"]);
    choose(&mut app, "Rename");
    type_name(&mut app, "notes");
    app.press(Action::File(FilePress::RenameGo));
    shell.pump(&mut app);
    let listed: Vec<&str> = app.sticks()[0]
        .files
        .iter()
        .map(|(n, _)| n.as_str())
        .collect();
    assert!(listed.contains(&"notes.txt"), "{listed:?}");
    assert!(!listed.contains(&"old-notes.txt"), "{listed:?}");
    assert_eq!(
        shell.disk("S1").file("notes.txt").map(Vec::as_slice),
        Some(&b"the notes"[..])
    );
    let _ = app.frame();
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "old-notes.txt renamed notes.txt"),
        "the visit says so"
    );
}

#[test]
fn a_rename_to_a_name_on_the_stick_is_refused_and_a_same_vault_named_only_for_copies_the_session_knows()
 {
    // `v` is a vault the session has seen (`vault` makes it known);
    // `unseen` is one it has not.
    let v = vault("Savings", "pass one", 7);
    let unseen = faraday_core::testkit::kdbx_vault("Other", "pass two", 9, Vec::new()).unwrap();
    let (mut shell, mut app) = visiting_as(
        true,
        Vec::new(),
        vec![
            file("a.txt", b"a"),
            file("B.txt", b"b"),
            file("vault.kdbx", &unseen),
            file("vault-copy.kdbx", &unseen),
            file("known.kdbx", &v),
            file("known-copy.kdbx", &v),
        ],
    );
    let at = |app: &Faraday, name: &str| {
        app.sticks()[0]
            .files
            .iter()
            .position(|(n, _)| n == name)
            .unwrap()
    };
    let before = shell.disk("S1").files.clone();
    let k = at(&app, "a.txt");
    menu_of(&mut app, Row::Stick(k));
    choose(&mut app, "Rename");
    type_name(&mut app, "b");
    app.press(Action::File(FilePress::RenameGo));
    shell.pump(&mut app);
    let k = at(&app, "vault-copy.kdbx");
    menu_of(&mut app, Row::Stick(k));
    choose(&mut app, "Rename");
    type_name(&mut app, "vault");
    app.press(Action::File(FilePress::RenameGo));
    shell.pump(&mut app);
    let k = at(&app, "known-copy.kdbx");
    menu_of(&mut app, Row::Stick(k));
    choose(&mut app, "Rename");
    type_name(&mut app, "known");
    app.press(Action::File(FilePress::RenameGo));
    shell.pump(&mut app);
    assert_eq!(shell.disk("S1").files, before, "nothing written");
    let log: Vec<&str> = app.visit().log.iter().map(|(l, _)| l.as_str()).collect();
    assert!(log.contains(&"b.txt is already on this stick"), "{log:?}");
    // Never opened or sealed here: two equal files are not known to be one vault.
    assert!(
        log.contains(&"vault.kdbx is already on this stick"),
        "{log:?}"
    );
    assert!(
        log.contains(&"known.kdbx on this stick is this same vault"),
        "{log:?}"
    );
}

#[test]
fn delete_from_the_stick_opens_the_sheet_with_that_file_alone_chosen() {
    let (_shell, mut app) = visiting(Vec::new(), vec![file("a.txt", b"a"), file("b.txt", b"b")]);
    let k = app.sticks()[0]
        .files
        .iter()
        .position(|(n, _)| n == "b.txt")
        .unwrap();
    menu_of(&mut app, Row::Stick(k));
    choose(&mut app, "Delete from the stick");
    assert_eq!(app.sheet, Some(Sheet::Delete));
    assert_eq!(
        app.delete_ask().map(|d| d.chosen.clone()),
        Some(vec!["b.txt".to_string()])
    );
    let _ = app.frame();
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "Delete 1 file with zeros")
    );
}

#[test]
fn a_vault_deleted_from_the_stick_takes_a_hold() {
    let v = vault("Savings", "pass one", 7);
    let (_shell, mut app) = visiting(Vec::new(), vec![file("vault.kdbx", &v)]);
    menu_of(&mut app, Row::Stick(0));
    choose(&mut app, "Delete from the stick");
    while app.poll_storage().is_some() {}
    let _ = app.frame();
    assert!(!app.offers(Action::DeleteGo), "no plain button");
    app.press(Action::DeleteGo);
    assert!(
        app.poll_storage().is_none(),
        "a press alone deletes nothing"
    );
    let hold = Action::Vault(faraday_core::vault::VaultAction::HoldWipe);
    assert!(app.offers(hold));
    let texts = app.drawn_texts();
    assert!(
        texts
            .iter()
            .any(|t| t == "Hold to delete 1 file with zeros"),
        "{texts:?}"
    );
}

#[test]
fn a_file_named_kdbx_holding_other_bytes_is_deleted_by_a_hold_too() {
    let (_shell, mut app) = visiting(Vec::new(), vec![file("notes.kdbx", b"just text")]);
    menu_of(&mut app, Row::Stick(0));
    choose(&mut app, "Delete from the stick");
    while app.poll_storage().is_some() {}
    let _ = app.frame();
    assert!(!app.offers(Action::DeleteGo), "no plain button");
    assert!(app.offers(Action::Vault(faraday_core::vault::VaultAction::HoldWipe)));
}

#[test]
fn details_shows_the_sha_256_of_the_bytes() {
    let c = faraday_core::testkit::compute();
    let bytes = b"what the file holds";
    let mut app = process(Vec::new(), vec![file("note.txt", bytes)], Vec::new());
    app.press(Action::Nav(Screen::Files));
    menu_of(&mut app, Row::Out(0));
    choose(&mut app, "Details");
    assert_eq!(app.sheet, Some(Sheet::Details));
    let sha = faraday_core::file_menu::sha_groups(&faraday_core::file_menu::sha256(&c, bytes));
    let texts = app.drawn_texts().plain();
    for line in &sha {
        assert!(texts.contains(line), "{line} in {texts:?}");
    }
    assert!(texts.iter().any(|t| t == "19 bytes"), "{texts:?}");

    // On the stick, read there for it.
    let (mut shell, mut app) = visiting(Vec::new(), vec![file("note.txt", bytes)]);
    menu_of(&mut app, Row::Stick(0));
    choose(&mut app, "Details");
    shell.pump(&mut app);
    let texts = app.drawn_texts().plain();
    assert!(
        texts.contains(&sha[0]) && texts.contains(&sha[1]),
        "{texts:?}"
    );
}

#[test]
fn a_psbt_no_loaded_wallet_can_sign_shows_check_a_signer_and_a_dots_menu() {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    faraday_core::testkit::inbox_push(
        &mut app,
        Item {
            name: faraday_text::Text::of("spend.psbt"),
            bytes: (b"psbt".to_vec()).into(),
            kind: FileKind::Psbt,
            secret: false,
            picture: None,
            sha: None,
            facts: None,
            wallet: None,
            psbt: None,
            outpoints: Vec::new(),
            unsigned_txid: None,
        },
    );
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    assert!(!app.offers(Action::StartSpend(0)), "no Sign");
    assert!(app.drawn_texts().iter().any(|t| t == "Check a signer"));
    assert_eq!(
        menu_of(&mut app, Row::In(0)),
        ["Check a signer", "Decode", "Details", "Remove"]
    );
    choose(&mut app, "Remove");
    assert_eq!(app.sheet, None, "a choice closes the menu");
    assert!(app.inbox().is_empty(), "Remove, through the ⋮");
}

#[test]
fn no_file_row_shows_a_remove_button() {
    for compact in [false, true] {
        let mut app = process(
            vec![file("notes.txt", b"some notes")],
            vec![file("spend-signed.psbt", b"p")],
            Vec::new(),
        );
        if compact {
            app.event(Event::Display(faraday_shell_api::DisplayInfo {
                width: 480,
                height: 640,
                dpi: 286,
                inset_bottom: 0,
                inset_top: 0,
                buttons: 0,
                camera_fixed: false,
                secure: faraday_shell_api::SecureHardware::None,
                boot: faraday_shell_api::BootState::Unknown,
                memory_mib: None,
            }));
        }
        app.press(Action::Nav(Screen::Files));
        let _ = app.frame();
        assert!(!app.offers(Action::InboxRemove(0)), "compact {compact}");
        assert!(!app.offers(Action::OutboxRemove(0)), "compact {compact}");
        // On the small panel In Faraday's one card shows few rows at a
        // time, the files further down.
        assert!(
            compact || app.offers(Action::File(FilePress::Menu(Row::Out(0)))),
            "For the stick's menu"
        );
        assert!(
            compact || app.offers(Action::File(FilePress::Menu(Row::In(0)))),
            "From the stick's menu"
        );
        assert!(!app.drawn_texts().iter().any(|t| t == "Remove"));
    }
}

#[test]
fn the_keyboard_opens_a_dots_menu_and_escape_closes_it() {
    let mut app = process(Vec::new(), vec![file("a.psbt", b"a")], Vec::new());
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    app.event(Event::Key(Key::Tab));
    app.event(Event::Key(Key::Enter));
    assert_eq!(app.sheet, Some(Sheet::FileMenu));
    app.event(Event::Key(Key::Escape));
    assert_eq!(app.sheet, None);
}

#[test]
fn a_sheet_for_the_stick_offers_show_as_qr_beside_make_pdf_on_desktop() {
    let mut app = process(
        Vec::new(),
        vec![file("plan.txt", b"faraday-sheet 1\nplan\n")],
        Vec::new(),
    );
    app.online = true;
    app.press(Action::Nav(Screen::Files));
    let words = menu_of(&mut app, Row::Out(0));
    assert!(words.iter().any(|w| w == "Make PDF"), "{words:?}");
    assert!(words.iter().any(|w| w == "Show as QR"), "{words:?}");
}

/// Rename stick on the visit (`docs/STICK-IDENTITY.md` §6): the popup
/// starts with the stick's name; a name typed in lower case is said and
/// written in capitals; a boot partition's name, a name FAT cannot hold
/// and one over 11 characters are refused; an empty name takes the label
/// away.
#[test]
fn a_stick_is_named_in_capitals_and_its_name_taken_away() {
    let (mut shell, mut app) = visiting(Vec::new(), vec![file("notes.txt", b"n")]);
    let name_stick = Action::File(FilePress::NameStick);
    assert!(offered(&mut app, name_stick));
    shell.press(&mut app, name_stick);
    assert_eq!(app.sheet, Some(Sheet::Rename));
    assert_eq!(
        app.files_menu().renaming.as_ref().map(|r| r.stem.as_str()),
        Some("TESTSTICK")
    );
    assert!(has(&mut app, "Rename stick"));
    type_name(&mut app, "coffee 1");
    assert!(has(&mut app, "Written as COFFEE 1"));
    assert!(rename_offered(&mut app));
    shell.press(&mut app, Action::File(FilePress::RenameGo));
    assert_eq!(app.sheet, None);
    assert_eq!(shell.disk("S1").label, "COFFEE 1");
    assert!(
        texts(&mut app)
            .iter()
            .any(|t| t.contains("This stick is named COFFEE 1")),
        "the visit says so"
    );

    shell.press(&mut app, name_stick);
    for bad in ["EFI", "bootfs", "faradayboot", "a.b", "twelve chars"] {
        type_name(&mut app, bad);
        assert!(!rename_offered(&mut app), "{bad} is refused");
    }
    type_name(&mut app, "");
    assert!(has(&mut app, "No name"));
    assert!(rename_offered(&mut app));
    shell.press(&mut app, Action::File(FilePress::RenameGo));
    assert_eq!(shell.disk("S1").label, "");
}

/// Files says when the stick a PC started from is not the device's own
/// (`docs/STICK-IDENTITY.md` §5.2): two sticks of the build were in, or it
/// was not found.
#[test]
fn files_says_when_the_stick_it_started_from_is_not_known() {
    let (_, mut app) = visiting(Vec::new(), Vec::new());
    app.press(Action::Nav(Screen::Files));
    let two = "Two sticks of this build are in";
    let lost = "The stick Faraday started from was not found";
    assert!(!has(&mut app, two) && !has(&mut app, lost));
    app.storage(StorageEvent::BootMedium(BootMedium::Two));
    assert!(has(&mut app, two));
    app.storage(StorageEvent::BootMedium(BootMedium::NotFound));
    assert!(has(&mut app, lost));
    app.storage(StorageEvent::BootMedium(BootMedium::Known));
    assert!(!has(&mut app, two) && !has(&mut app, lost));
}

#[test]
fn a_vault_no_stick_holds_is_deleted_only_by_a_hold_that_says_what_is_lost() {
    let v = vault("Savings", "pass one", 7);
    let mut app = process(Vec::new(), vec![file("vault.kdbx", &v)], Vec::new());
    app.press(Action::Nav(Screen::Files));
    let lines = menu_of(&mut app, Row::Out(0));
    assert_eq!(lines.last().map(String::as_str), Some("Delete"));
    choose(&mut app, "Delete");
    assert_eq!(app.sheet, Some(Sheet::OutboxDelete));
    assert_eq!(app.outbox.len(), 1, "the press alone deletes nothing");
    let _ = app.frame();
    let texts = app.drawn_texts();
    for want in [
        "Delete vault.kdbx?",
        "This vault is on no stick. Everything in it is lost.",
        "Hold to delete the vault",
    ] {
        assert!(texts.iter().any(|t| t == want), "{want}: {texts:?}");
    }
    app.press(Action::Cancel);
    assert_eq!(app.outbox.len(), 1, "Cancel keeps it");
    menu_of(&mut app, Row::Out(0));
    choose(&mut app, "Delete");
    hold(&mut app, Action::OutboxDeleteHold);
    assert!(app.outbox.is_empty(), "the hold deletes it");
    assert_eq!(app.sheet, None);
}

#[test]
fn a_public_file_for_the_stick_is_removed_by_a_press() {
    let mut app = process(
        Vec::new(),
        vec![file("spend-signed.psbt", b"p")],
        Vec::new(),
    );
    app.press(Action::Nav(Screen::Files));
    let lines = menu_of(&mut app, Row::Out(0));
    assert_eq!(lines.last().map(String::as_str), Some("Remove"));
    choose(&mut app, "Remove");
    assert_eq!(app.sheet, None);
    assert!(app.outbox.is_empty(), "a press removes it");
}

#[test]
fn check_a_signer_is_offered_on_a_psbt_row_only() {
    let psbt = std::fs::read(format!(
        "{}/../../tools/vectors/psbt/bump-core-original.psbt",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let menu = |row: usize| {
        let mut app = faraday_core::testkit::started();
        app.storage(StorageEvent::Restored {
            inbox: faraday_core::files_from(vec![
                ("spend.psbt".to_string(), psbt.clone()),
                ("notes.txt".to_string(), b"hello".to_vec()),
            ]),
            outbox: Vec::new(),
            kept: Vec::new(),
        });
        app.press(Action::Nav(Screen::Files));
        menu_of(&mut app, Row::In(row))
    };
    assert!(menu(0).contains(&"Check a signer".to_string()));
    let other = menu(1);
    assert!(!other.contains(&"Check a signer".to_string()), "{other:?}");
}

#[test]
fn a_vaults_key_file_deleted_from_the_stick_takes_a_hold() {
    let (_shell, mut app) = visiting(Vec::new(), vec![file("savings.keyx", &[7u8; 32])]);
    menu_of(&mut app, Row::Stick(0));
    choose(&mut app, "Delete from the stick");
    while app.poll_storage().is_some() {}
    let _ = app.frame();
    assert!(!app.offers(Action::DeleteGo), "no plain button");
    assert!(app.offers(Action::Vault(faraday_core::vault::VaultAction::HoldWipe)));
    let texts = app.drawn_texts();
    assert!(
        texts
            .iter()
            .any(|t| t == "Hold to delete a vault's key file"),
        "{texts:?}"
    );
}

#[test]
fn decode_is_offered_on_a_psbt_and_a_finished_transaction_in_both_halves_of_files() {
    let psbt = std::fs::read(format!(
        "{}/../../tools/vectors/psbt/bump-core-original.psbt",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let mut app = faraday_core::testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            ("spend.psbt".to_string(), psbt.clone()),
            ("notes.txt".to_string(), b"hello".to_vec()),
        ]),
        outbox: faraday_core::files_from(vec![("sent.psbt".to_string(), psbt)]),
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Files));
    assert!(menu_of(&mut app, Row::In(0)).contains(&"Decode".to_string()));
    choose(&mut app, "Decode");
    assert_eq!(app.sheet, None, "a choice closes the menu");
    assert_eq!(app.screen, Screen::Decode);
    assert!(app.decode().is_some(), "the PSBT's transaction is shown");

    app.press(Action::Nav(Screen::Files));
    assert!(menu_of(&mut app, Row::Out(0)).contains(&"Decode".to_string()));
    choose(&mut app, "Decode");
    assert_eq!(app.screen, Screen::Decode);

    app.press(Action::Nav(Screen::Files));
    let text = menu_of(&mut app, Row::In(1));
    assert!(!text.contains(&"Decode".to_string()), "{text:?}");
}

#[test]
fn a_text_file_offers_check_an_address_and_opens_the_check_on_it() {
    let mut app = faraday_core::testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            "notes.txt".to_string(),
            b"bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq".to_vec(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Files));
    assert!(menu_of(&mut app, Row::In(0)).contains(&"Check an address".to_string()));
    choose(&mut app, "Check an address");
    assert_eq!(app.sheet, None, "a choice closes the menu");
    assert_eq!(app.screen, Screen::CheckAddress);
}

#[test]
fn a_psbt_of_a_loaded_wallet_shows_sign_and_a_text_file_shows_no_button() {
    use faraday_core::testkit;
    let k = testkit::kits()
        .into_iter()
        .find(|k| k.id == "savings")
        .unwrap();
    let (unsigned, _) = testkit::spend_of(&k).unwrap();
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (faraday_text::Text::of("spend.psbt"), unsigned.to_bytes()),
            (
                faraday_text::Text::of("savings-wallet.txt"),
                k.descriptor.clone().into_bytes(),
            ),
            (faraday_text::Text::of("notes.txt"), b"hello".to_vec()),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let at = |app: &Faraday, n: &str| app.inbox().iter().position(|i| i.name == n).unwrap();
    let (p, t) = (at(&app, "spend.psbt"), at(&app, "notes.txt"));
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    assert!(
        !app.offers(Action::StartSpend(p)),
        "no wallet loaded, no Sign"
    );
    let w = at(&app, "savings-wallet.txt");
    testkit::load_wallet(&mut app, w);
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    assert!(
        app.offers(Action::StartSpend(p)),
        "Sign with its wallet loaded"
    );
    assert!(!app.offers(Action::File(FilePress::Decode(Row::In(t)))));
    assert!(!app.offers(Action::Vault(faraday_core::vault::VaultAction::AddFile(t))));
}

/// The test file `name` of the shared fixtures.
fn fixture(name: &str) -> Vec<u8> {
    faraday_core::testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no fixture {name}"))
        .1
}

/// An app with `files` in its Inbox, in order.
fn holding(files: &[(&str, Vec<u8>)]) -> Faraday {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    for (name, bytes) in files {
        faraday_core::testkit::inbox_file(&mut app, name, bytes.clone());
    }
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    app
}

#[test]
fn a_held_psbt_offers_sign_once_its_wallet_is_loaded() {
    let mut app = holding(&[
        ("spending-unsigned.psbt", fixture("spending-unsigned.psbt")),
        ("spending-wallet.txt", fixture("spending-wallet.txt")),
    ]);
    assert_eq!(
        menu_of(&mut app, Row::In(0)),
        ["Check a signer", "Decode", "Details", "Remove"],
        "no wallet loaded: only the checks"
    );
    app.press(Action::File(FilePress::Close));
    faraday_core::testkit::load_wallet(&mut app, 1);
    assert_eq!(app.session.wallets.len(), 1);
    app.press(Action::Nav(Screen::Files));
    assert_eq!(
        menu_of(&mut app, Row::In(0)),
        ["Sign", "Check a signer", "Decode", "Details", "Remove"]
    );
}

#[test]
fn a_signed_psbt_of_the_spend_under_way_offers_add_to_signatures() {
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "savings")
        .unwrap();
    let (unsigned, _) = faraday_core::testkit::spend_of(&kit).unwrap();
    let spend = || ("spend.psbt", unsigned.to_bytes());
    let wallet = || ("savings-wallet.txt", kit.descriptor.clone().into_bytes());
    // Another device signs it.
    let mut a = holding(&[spend(), wallet()]);
    faraday_core::testkit::load_wallet(&mut a, 1);
    add_key(&mut a, "zebra");
    a.press(Action::StartSpend(0));
    faraday_core::testkit::steps_before_sign_done(&mut a);
    a.press(Action::SignHere);
    let from_a = a.spend().unwrap().spend.psbt.to_bytes();

    // Here no key is loaded, so none is left to sign it with.
    let mut b = holding(&[spend(), wallet(), ("from-a.psbt", from_a)]);
    faraday_core::testkit::load_wallet(&mut b, 1);
    b.press(Action::StartSpend(0));
    b.press(Action::Nav(Screen::Files));
    assert_eq!(
        menu_of(&mut b, Row::In(2)),
        [
            "Add to signatures",
            "Check a signer",
            "Decode",
            "Details",
            "Remove"
        ]
    );
}

#[test]
fn a_held_message_file_offers_check() {
    let sig = "H".to_string() + &"A".repeat(86) + "=";
    let text = format!("bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq\n{sig}\nhello\n");
    let mut app = holding(&[("msg.txt", text.into_bytes())]);
    assert_eq!(app.inbox()[0].kind, FileKind::Message);
    assert_eq!(
        menu_of(&mut app, Row::In(0)),
        ["Check the signature", "Details", "Remove"]
    );
}

#[test]
fn a_file_no_flow_takes_offers_only_details_and_remove() {
    let mut app = holding(&[("photo.jpg", vec![0xff, 0xd8, 0xff, 0xe0])]);
    assert_eq!(menu_of(&mut app, Row::In(0)), ["Details", "Remove"]);
}

/// Faraday with a wallet file in Files, the wallet loaded from it when
/// `loaded`.
fn with_wallet_file(loaded: bool) -> Faraday {
    let text = format!(
        "wpkh({}/<0;1>/*)",
        faraday_core::testkit::key(0, "m/84h/1h/0h")
    );
    let mut app = process(
        vec![file("wallet.txt", text.as_bytes())],
        Vec::new(),
        Vec::new(),
    );
    if loaded {
        app.press(Action::LoadWallet(0));
    }
    app.press(Action::Nav(Screen::Files));
    app
}

#[test]
fn a_wallet_file_not_loaded_offers_load_and_the_flows_that_take_it() {
    let mut app = with_wallet_file(false);
    let menu = menu_of(&mut app, Row::In(0));
    assert_eq!(menu[0], "Load the wallet", "{menu:?}");
    assert!(menu.iter().any(|l| l == "Spend from backup"), "{menu:?}");
}

/// The wallet is loaded once on the page: by the summary card, whose heading
/// is its own, and not by its file's row; the row's menu keeps it.
#[test]
fn a_wallet_the_summary_card_loads_has_no_second_load_on_its_files_row() {
    for size in [(1366, 768, 160), (480, 640, 286)] {
        let mut app = with_wallet_file(false);
        app.event(Event::Display(faraday_shell_api::DisplayInfo {
            width: size.0,
            height: size.1,
            dpi: size.2,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: false,
            secure: faraday_shell_api::SecureHardware::None,
            boot: faraday_shell_api::BootState::Unknown,
            memory_mib: None,
        }));
        let _ = app.frame();
        let texts = app.drawn_texts();
        let n = |t: &str| texts.iter().filter(|x| ***x == *t).count();
        assert_eq!(n("Found in these files"), 1, "{size:?} {texts:?}");
        assert!(n("From the stick") <= 1, "{size:?} {texts:?}");
        assert_eq!(n("Load the wallet"), 1, "the card's, {size:?} {texts:?}");
    }
    let mut app = with_wallet_file(false);
    let menu = menu_of(&mut app, Row::In(0));
    assert_eq!(menu[0], "Load the wallet", "{menu:?}");
}

#[test]
fn a_wallet_file_loaded_offers_no_load_but_back_up_and_spend_from_backup() {
    let mut app = with_wallet_file(true);
    let menu = menu_of(&mut app, Row::In(0));
    assert!(!menu.iter().any(|l| l == "Load the wallet"), "{menu:?}");
    assert!(menu.iter().any(|l| l == "Back up"), "{menu:?}");
    choose(&mut app, "Spend from backup");
    assert_eq!(app.screen, Screen::SpendBackup);
    assert_eq!(app.session.wallets.len(), 1, "that file's wallet, once");
}

#[test]
fn a_wallet_file_loaded_offers_check_an_address_and_one_not_loaded_does_not() {
    let mut app = with_wallet_file(false);
    let menu = menu_of(&mut app, Row::In(0));
    assert!(!menu.iter().any(|l| l == "Check an address"), "{menu:?}");
    let mut app = with_wallet_file(true);
    let menu = menu_of(&mut app, Row::In(0));
    assert!(menu.iter().any(|l| l == "Check an address"), "{menu:?}");
    choose(&mut app, "Check an address");
    assert_eq!(app.screen, Screen::CheckAddress);
}

#[test]
fn a_locked_vault_file_in_files_offers_unlock() {
    let v = vault("Savings", "pass one", 7);
    let mut app = process(vec![file("vault.kdbx", &v)], Vec::new(), Vec::new());
    app.press(Action::Nav(Screen::Files));
    let menu = menu_of(&mut app, Row::In(0));
    assert_eq!(menu[0], "Unlock", "{menu:?}");
}

fn wallet_text(n: usize) -> Vec<u8> {
    format!(
        "wpkh({}/<0;1>/*)",
        faraday_core::testkit::key(n, "m/84h/1h/0h")
    )
    .into_bytes()
}

fn place(app: &Faraday, name: &str) -> usize {
    app.inbox()
        .iter()
        .position(|i| i.name.as_str() == name)
        .expect(name)
}

#[test]
fn spend_from_backup_on_a_file_not_loaded_takes_it_and_has_no_back_up() {
    let mut app = with_wallet_file(false);
    let menu = menu_of(&mut app, Row::In(0));
    assert!(!menu.iter().any(|l| l == "Back up"), "{menu:?}");
    choose(&mut app, "Spend from backup");
    assert_eq!(app.screen, Screen::SpendBackup);
    assert!(
        app.spend_backup_wallet().is_some(),
        "the flow took the file"
    );
}

#[test]
fn back_up_on_a_loaded_wallet_file_opens_that_wallet() {
    let mut app = with_wallet_file(true);
    let _ = menu_of(&mut app, Row::In(0));
    choose(&mut app, "Back up");
    assert_eq!(app.backup().map(|b| b.wallet), Some(0));
}

#[test]
fn back_up_on_a_loaded_wallet_file_opens_the_file_s_wallet_when_it_is_the_second() {
    let mut app = process(
        vec![
            file("first.txt", &wallet_text(0)),
            file("second.txt", &wallet_text(1)),
        ],
        Vec::new(),
        Vec::new(),
    );
    let (first, second) = (place(&app, "first.txt"), place(&app, "second.txt"));
    faraday_core::testkit::load_wallet(&mut app, first);
    faraday_core::testkit::load_wallet(&mut app, second);
    assert_eq!(app.session.wallets.len(), 2);
    app.press(Action::Nav(Screen::Files));
    let second = place(&app, "second.txt");
    let _ = menu_of(&mut app, Row::In(second));
    choose(&mut app, "Back up");
    assert_eq!(app.backup().map(|b| b.wallet), Some(1));
}

#[test]
fn two_wallets_under_one_file_name_the_file_is_taken_for_its_own_wallet() {
    let mut app = process(
        vec![file("wallet.txt", &wallet_text(0))],
        Vec::new(),
        Vec::new(),
    );
    app.press(Action::LoadWallet(0));
    app.press(Action::Nav(Screen::Files));
    // The first file goes; another wallet comes in under the same name.
    app.press(Action::InboxRemove(0));
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("wallet.txt"),
            wallet_text(1),
        )]),
        outbox: Vec::new(),
        kept: faraday_core::kept_from(Vec::<(String, Vec<u8>)>::new()),
    });
    app.press(Action::Nav(Screen::Files));
    let menu = menu_of(&mut app, Row::In(0));
    assert_eq!(menu[0], "Load the wallet", "{menu:?}");
    assert!(!menu.iter().any(|l| l == "Back up"), "{menu:?}");
    assert_eq!(app.session.wallets.len(), 1, "the first wallet stays");
}

#[test]
fn spend_from_backup_on_a_refused_file_says_so_on_what_do_you_have() {
    let mut app = process(
        vec![file("wallet.txt", &wallet_text(0))],
        Vec::new(),
        Vec::new(),
    );
    // A file the flow refuses as a wallet: a descriptor that is a bare key.
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("wallet.txt"),
            faraday_core::testkit::key(0, "m/84h/1h/0h").into_bytes(),
        )]),
        outbox: Vec::new(),
        kept: faraday_core::kept_from(Vec::<(String, Vec<u8>)>::new()),
    });
    app.press(Action::SpendBackup(
        faraday_core::spend_backup::SpendBackupAction::FromFile(0),
    ));
    assert_eq!(app.screen, Screen::SpendBackup);
    assert!(app.spend_backup_wallet().is_none());
}

/// A spend of the "spending" kit's wallet under way (its Check not done
/// unless `checked`), with `other.txt`, a wallet file of another wallet,
/// in the Inbox.
fn spend_under_way(checked: bool) -> Faraday {
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let psbt = faraday_core::testkit::unsigned(&kit).unwrap();
    let mut app = process(
        vec![
            file("spend.psbt", &psbt.to_bytes()),
            file("spending-wallet.txt", kit.descriptor.as_bytes()),
            file("other.txt", &wallet_text(2)),
        ],
        Vec::new(),
        Vec::new(),
    );
    let wallet_at = place(&app, "spending-wallet.txt");
    faraday_core::testkit::load_wallet(&mut app, wallet_at);
    app.press(Action::Entry(None));
    for c in faraday_core::testkit::test_words("bacon").chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    let psbt_at = place(&app, "spend.psbt");
    app.press(Action::StartSpend(psbt_at));
    if checked {
        faraday_core::testkit::steps_before_sign_done(&mut app);
    }
    assert!(app.spend().is_some());
    app
}

#[test]
fn a_spend_of_one_wallet_under_way_is_asked_about_when_another_wallets_file_is_chosen() {
    let mut app = spend_under_way(true);
    app.press(Action::Nav(Screen::Files));
    let other = place(&app, "other.txt");
    let _ = menu_of(&mut app, Row::In(other));
    choose(&mut app, "Spend from backup");
    // Something is lost: asked first, the spend and the file untouched.
    assert!(
        matches!(app.sheet, Some(Sheet::CancelFlow(_))),
        "{:?}",
        app.sheet
    );
    assert!(app.spend().is_some());
    assert_eq!(app.session.wallets.len(), 1);
    app.press(Action::CancelFlowNow);
    assert!(app.spend().is_none());
    // Asked again, the file is taken and the flow is its own.
    app.press(Action::Nav(Screen::Files));
    let _ = menu_of(&mut app, Row::In(other));
    choose(&mut app, "Spend from backup");
    assert_eq!(app.session.wallets.len(), 2);
    assert!(app.spend().is_none());
    assert_eq!(app.screen, Screen::SpendBackup);
}

#[test]
fn unlock_in_a_files_menu_opens_that_vault_file() {
    let v = vault("Savings", "pass one", 7);
    let mut app = process(vec![file("vault.kdbx", &v)], Vec::new(), Vec::new());
    app.press(Action::Nav(Screen::Files));
    let _ = menu_of(&mut app, Row::In(0));
    choose(&mut app, "Unlock");
    assert_ne!(app.screen, Screen::Files);
    unlock(&mut app, "pass one");
    assert_eq!(app.vaults().open.len(), 1);
}

#[test]
fn an_open_vault_files_menu_has_no_unlock() {
    let v = vault("Savings", "pass one", 7);
    let mut app = process(vec![file("vault.kdbx", &v)], Vec::new(), Vec::new());
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(faraday_core::vault::VaultAction::Open(0)));
    unlock(&mut app, "pass one");
    app.press(Action::Nav(Screen::Files));
    let menu = menu_of(&mut app, Row::In(0));
    assert!(!menu.iter().any(|l| l == "Unlock"), "{menu:?}");
}

fn check_done(app: &Faraday) -> bool {
    app.spend().unwrap().done[faraday_core::wallet::step::CHECK as usize]
}

#[test]
fn yes_my_money_shows_does_not_mark_another_wallets_spend() {
    use faraday_core::spend_backup::{SpendBackupAction as F, page};
    let mut app = spend_under_way(false);
    app.press(Action::Nav(Screen::SpendBackup));
    let other = place(&app, "other.txt");
    app.press(Action::SpendBackup(F::UseWallet(other)));
    assert_eq!(app.session.wallets.len(), 2);
    app.press(Action::SpendBackup(F::Next(page::CHECK)));
    assert!(!check_done(&app), "the spend is the first wallet's");
}

/// Another network's wallet file, with its origin: it is refused.
const MAINNET: &str = "wpkh([73c5da0a/84h/0h/0h]xpub6CatWdiZiodmUeTDp8LT5or8nmbKNcuyvz7WyksVFkKB4RHwCD3XyuvPEbvqAQY3rAPshWcMLoP2fMFMKHPJ4ZeZXYVUhLv1VMrjPC7PW6V/0/*)";

#[test]
fn a_file_taken_for_another_wallet_starts_the_flow_again_on_a_page() {
    use faraday_core::spend_backup::{Open, SpendBackupAction as F};
    let mut app = spend_under_way(false);
    // The first wallet is open from its card, then its spend is ended.
    app.press(Action::SpendBackup(F::SpendFrom(0)));
    assert!(app.spend_backup().from_card);
    assert!(app.spend().is_some());
    app.press(Action::RemoveWallet(0));
    assert!(app.spend().is_none());
    assert!(app.spend_backup().from_card, "{:?}", app.spend_backup());
    // Another wallet's file is taken with nothing to ask.
    app.press(Action::Nav(Screen::Files));
    let other = place(&app, "other.txt");
    let _ = menu_of(&mut app, Row::In(other));
    choose(&mut app, "Spend from backup");
    assert!(app.sheet.is_none(), "{:?}", app.sheet);
    assert_eq!(app.spend_backup_wallet(), Some(0));
    assert!(!app.spend_backup().from_card, "the file's flow has no card");
    assert!(matches!(app.spend_backup().open, Some(Open::Page(_))));
    assert!(app.spend().is_none());
}

fn spend_identity(app: &Faraday) -> (Option<usize>, Vec<bool>) {
    let s = app.spend().expect("the spend is open");
    (s.wallet, s.done.to_vec())
}

#[test]
fn a_cancelled_sheet_leaves_the_spend_as_it_was() {
    let mut app = spend_under_way(false);
    let before = spend_identity(&app);
    app.press(Action::Nav(Screen::Files));
    let other = place(&app, "other.txt");
    let _ = menu_of(&mut app, Row::In(other));
    choose(&mut app, "Spend from backup");
    assert!(matches!(app.sheet, Some(Sheet::CancelFlow(_))));
    app.press(Action::Cancel);
    assert!(app.sheet.is_none());
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(spend_identity(&app), before, "a cancelled sheet leaves it");
}

#[test]
fn another_wallets_file_taken_leaves_no_seed_of_the_first_as_a_stray() {
    use faraday_core::spend_backup::Came;
    let mut app = spend_under_way(false);
    // A seed typed in the flow came in for the first wallet.
    app.press(Action::Nav(Screen::SpendBackup));
    app.press(Action::SpendBackup(
        faraday_core::spend_backup::SpendBackupAction::SpendFrom(0),
    ));
    app.press(Action::Entry(None));
    for c in faraday_core::testkit::test_words("zebra").chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    assert!(
        app.spend_backup()
            .came
            .iter()
            .any(|c| matches!(c, Came::Seed(_))),
        "the typed seed came in"
    );
    // The first wallet's spend is gone, its seed still came in.
    app.press(Action::RemoveWallet(0));
    assert!(app.spend().is_none());
    assert!(!app.spend_backup().came.is_empty());
    eprintln!(
        "DBG {:?} {}",
        app.spend_backup().wallet,
        app.session.wallets.len()
    );
    app.press(Action::Nav(Screen::Files));
    let other = place(&app, "other.txt");
    let _ = menu_of(&mut app, Row::In(other));
    choose(&mut app, "Spend from backup");
    assert!(app.sheet.is_none(), "{:?}", app.sheet);
    let came = &app.spend_backup().came;
    assert_eq!(
        came.len(),
        1,
        "{came:?} {:?} {}",
        app.spend_backup().wallet,
        app.session.wallets.len()
    );
    assert!(matches!(&came[0], Came::File(n) if n.as_str() == "other.txt"));
}

#[test]
fn unlock_is_not_offered_while_a_stick_is_in() {
    let v = vault("Savings", "pass one", 7);
    let mut app = process(vec![file("vault.kdbx", &v)], Vec::new(), Vec::new());
    let mut shell = Shell::new();
    shell.add(Disk::empty("S1", "TESTSTICK"));
    shell.plug(&mut app, "S1");
    shell.press(&mut app, Action::Nav(Screen::Files));
    let menu = menu_of(&mut app, Row::In(0));
    assert!(!menu.iter().any(|l| l == "Unlock"), "{menu:?}");
}

#[test]
fn a_secret_file_whose_wallet_was_not_read_offers_no_spend_from_backup() {
    const TPRV: &str = "tprv8ZgxMBicQKsPe5YMU9gHen4Ez3ApihUfykaqUorj9t6FDqy3nP6eoXiAo2ssvpAjoLroQxHqr3R5nE3a5dU3DHTjTgJDd7zrbniJr6nrCzd";
    let read = format!("wpkh({TPRV}/84h/1h/0h/<0;1>/*)");
    let mut app = process(
        vec![file("secret.txt", read.as_bytes())],
        Vec::new(),
        Vec::new(),
    );
    app.press(Action::Nav(Screen::Files));
    let menu = menu_of(&mut app, Row::In(0));
    assert!(!menu.iter().any(|l| l == "Spend from backup"), "{menu:?}");
}

#[test]
fn spend_from_backup_on_a_refused_wallet_file_takes_no_wallet_and_keeps_the_words_guess() {
    use faraday_core::spend_backup::SpendBackupAction as F;
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    // A guess from typed words stands.
    app.press(Action::Nav(Screen::SpendBackup));
    app.press(Action::SpendBackup(F::TypeWords));
    app.press(Action::Entry(None));
    for c in faraday_core::testkit::test_words("bacon").chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    app.press(Action::SpendBackup(F::Everything));
    let state = |app: &Faraday| {
        let s = app.spend_backup();
        (
            s.guess.is_some(),
            format!("{:?}", s.route),
            format!("{:?}", s.seeds),
        )
    };
    let before = (
        app.session.wallets.len(),
        app.spend_backup_wallet(),
        state(&app),
    );
    assert!(before.2.0, "the words made a guess");
    // A wallet file whose wallet cannot be taken: a descriptor that is a
    // bare key.
    faraday_core::testkit::inbox_file(&mut app, "mainnet.txt", MAINNET.as_bytes().to_vec());
    let at = place(&app, "mainnet.txt");
    app.press(Action::SpendBackup(F::FromFile(at)));
    assert_eq!(app.screen, Screen::SpendBackup);
    assert!(
        app.spend_backup().error.is_some(),
        "{:?} {:?} {:?}",
        app.sheet,
        app.toast_text(),
        app.spend_backup()
    );
    assert_eq!(
        (
            app.session.wallets.len(),
            app.spend_backup_wallet(),
            state(&app)
        ),
        before
    );
}

/// Coldcard's export of one key's single-sig accounts and its silent
/// payments account (Sparrow's test file).
const CC_SP: &str = include_str!("../vectors/interop/sparrow/cc-singlesig-sp-keystore-1.json");

#[test]
fn spend_from_backup_on_a_two_account_export_asks_the_account_and_takes_the_one_chosen() {
    use faraday_core::wallet::Account;
    let mut app = process(
        vec![file("cc.json", CC_SP.as_bytes())],
        Vec::new(),
        Vec::new(),
    );
    app.press(Action::Nav(Screen::Files));
    let _ = menu_of(&mut app, Row::In(0));
    choose(&mut app, "Spend from backup");
    assert_eq!(app.sheet, Some(Sheet::Account), "{:?}", app.toast_text());
    assert!(
        app.session.wallets.is_empty(),
        "nothing taken before the choice"
    );
    app.press(Action::LoadAccount(Some(Account::Silent)));
    assert_eq!(app.session.wallets.len(), 1);
    assert!(app.session.wallets[0].policy().silent().is_some());
    assert_eq!(app.spend_backup_wallet(), Some(0));
}

const HELD_WORDS: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
/// SLIP-39 vector 4, one of its two shares.
const HELD_SHARE: &str = "shadow pistol academic always adequate wildlife fancy gross oasis \
    cylinder mustang wrist rescue view short owner flip making coding armed";

/// Files with one held file of `text`.
fn held_file(text: &str) -> Faraday {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    faraday_core::testkit::inbox_file_at(&mut app, 0, "backup.txt", text.as_bytes().to_vec());
    app.press(Action::Nav(Screen::Files));
    app
}

fn stick_in(app: &mut Faraday) {
    app.storage(StorageEvent::Sticks(vec![faraday_core::StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("a"),
        label: faraday_text::Text::of("STICK"),
        boot: false,
        files: Vec::new(),
        vaults: Vec::new(),
    }]));
}

/// The ⋮'s `label` of the held file, pressed.
fn pick(app: &mut Faraday, label: &str) {
    menu_of(app, Row::In(0));
    choose(app, label);
    let _ = app.frame();
}

fn slip39_lines(app: &Faraday) -> usize {
    faraday_core::testkit::entry(app)
        .parts
        .lines(faraday_core::forms::Form::Slip39)
        .len()
}

#[test]
fn a_words_file_restores_with_its_key_and_a_loaded_key_is_refused() {
    let mut app = held_file(HELD_WORDS);
    assert_eq!(
        menu_of(&mut app, Row::In(0))[..4],
        [
            "Load this key",
            "Restore",
            "Spend from backup",
            "Check a backup"
        ]
    );
    choose(&mut app, "Restore");
    assert_eq!(app.screen, Screen::Restore);
    assert_eq!(app.session.keys.all().count(), 1);
    app.press(Action::Nav(Screen::Files));
    pick(&mut app, "Restore");
    assert_eq!(
        app.session.keys.all().count(),
        1,
        "refused as typed words are"
    );
}

#[test]
fn a_words_file_opens_spend_from_backup_and_not_with_a_stick_in() {
    let mut app = held_file(HELD_WORDS);
    pick(&mut app, "Spend from backup");
    assert_eq!(app.screen, Screen::SpendBackup);
    assert_eq!(app.session.keys.all().count(), 1);

    let mut app = held_file(HELD_WORDS);
    stick_in(&mut app);
    app.press(Action::Nav(Screen::Files));
    pick(&mut app, "Spend from backup");
    assert_ne!(app.screen, Screen::SpendBackup);
    assert!(app.session.keys.is_empty(), "no key while a stick is in");
}

#[test]
fn a_words_file_is_checked_as_typed_words_and_not_with_a_stick_in() {
    let mut app = held_file(HELD_WORDS);
    pick(&mut app, "Check a backup");
    assert_eq!(app.screen, Screen::SeedCheck);
    app.press(Action::SeedCheck(
        faraday_core::seed_check::SeedAction::NoPass,
    ));
    let held = app.session.keys.check_facts().expect("the check holds it");
    assert_eq!(held.fingerprint.to_string(), "73c5da0a");
    assert!(app.session.keys.is_empty(), "no key was loaded");

    let mut app = held_file(HELD_WORDS);
    stick_in(&mut app);
    app.press(Action::Nav(Screen::Files));
    pick(&mut app, "Check a backup");
    assert_ne!(app.screen, Screen::SeedCheck);
    assert!(app.session.keys.check_facts().is_none());
}

#[test]
fn a_share_file_restores_as_the_first_share_and_not_with_a_stick_in() {
    let mut app = held_file(HELD_SHARE);
    let lines = menu_of(&mut app, Row::In(0));
    assert_eq!(lines[..3], ["Restore", "Check a backup", "Load this key"]);
    assert!(!lines.iter().any(|l| l == "Spend from backup"));
    choose(&mut app, "Restore");
    assert_eq!(app.screen, Screen::Entry);
    assert_eq!(slip39_lines(&app), 1);

    let mut app = held_file(HELD_SHARE);
    stick_in(&mut app);
    app.press(Action::Nav(Screen::Files));
    pick(&mut app, "Restore");
    assert_ne!(app.screen, Screen::Entry);
    assert_eq!(slip39_lines(&app), 0);
}

#[test]
fn a_share_file_is_checked_as_a_scanned_share_and_not_with_a_stick_in() {
    let mut app = held_file(HELD_SHARE);
    pick(&mut app, "Check a backup");
    assert_eq!(app.screen, Screen::SeedCheck);
    assert_eq!(slip39_lines(&app), 1);

    let mut app = held_file(HELD_SHARE);
    stick_in(&mut app);
    app.press(Action::Nav(Screen::Files));
    pick(&mut app, "Check a backup");
    assert_ne!(app.screen, Screen::SeedCheck);
    assert_eq!(slip39_lines(&app), 0);
}

#[test]
fn a_share_file_loads_as_the_first_share_on_add_a_key_and_not_with_a_stick_in() {
    let mut app = held_file(HELD_SHARE);
    pick(&mut app, "Load this key");
    assert_eq!(app.screen, Screen::Entry);
    assert_eq!(slip39_lines(&app), 1);
    assert!(app.session.keys.is_empty(), "one of two shares is no key");

    let mut app = held_file(HELD_SHARE);
    stick_in(&mut app);
    app.press(Action::Nav(Screen::Files));
    pick(&mut app, "Load this key");
    assert_ne!(app.screen, Screen::Entry);
    assert_eq!(slip39_lines(&app), 0);
}

/// SLIP-39 vector 4, the other of its two shares.
const OTHER_SHARE: &str = "shadow pistol academic acid actress prayer class unknown daughter \
    sweater depict flip twice unkind craft early superior advocate guest smoking";

#[test]
fn a_share_file_has_a_row_button_once_the_held_shares_make_the_seed() {
    let restore = Action::RestoreSeedFile(0);
    let mut app = held_file(HELD_SHARE);
    let _ = app.frame();
    assert!(!app.offers(restore), "one of two shares: no row button");

    faraday_core::testkit::inbox_file_at(&mut app, 1, "other.txt", OTHER_SHARE.into());
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    scroll_to(&mut app, restore);
    app.press(restore);
    assert_eq!(app.screen, Screen::Entry);
    assert_eq!(slip39_lines(&app), 1);
}
