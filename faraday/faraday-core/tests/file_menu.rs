//! A ⋮ on every file (`docs/FLOWS.md` decision 61): each file row in
//! Files and on the stick visit keeps its one action as a button and the
//! rest behind its ⋮; a file For the stick is renamed before it is
//! written, a file on the stick is renamed on the stick, and Details says
//! a file's SHA-256.

mod common;

use common::*;
use faraday_core::file_menu::{FilePress, Row};
use faraday_core::wallet::FileKind;
use faraday_core::{Action, Faraday, Item, Screen, Sheet};
use faraday_shell_api::{App, Event, Key};

fn file(name: &str, bytes: &[u8]) -> (String, Vec<u8>) {
    (name.to_string(), bytes.to_vec())
}

/// The ⋮'s menu of `row`, opened by its press: each choice's words.
fn menu_of(app: &mut Faraday, row: Row) -> Vec<String> {
    let _ = app.frame();
    let dots = Action::File(FilePress::Menu(row));
    assert!(app.offers(dots), "{row:?} has a ⋮");
    app.press(dots);
    assert_eq!(app.sheet, Some(Sheet::FileMenu));
    app.files_menu
        .menu
        .as_ref()
        .expect("its menu")
        .lines
        .iter()
        .map(|(l, _)| l.clone())
        .collect()
}

/// The choice in the open menu that reads `label`, pressed.
fn choose(app: &mut Faraday, label: &str) {
    let a = app
        .files_menu
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
        .files_menu
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
    let mut shell = Shell::new();
    let mut d = Disk::empty("S1", "TESTSTICK");
    d.files = on_stick;
    shell.add(d);
    let mut app = process(Vec::new(), outbox, Vec::new());
    shell.plug(&mut app, "S1");
    shell.press(&mut app, Action::Nav(Screen::Visit));
    let names: Vec<String> = app.outbox.iter().map(|i| i.name.clone()).collect();
    for (i, n) in names.iter().enumerate() {
        if !app.visit.out.contains(n) {
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
    shell.press(&mut app, Action::Nav(Screen::Visit));
    if !app.visit.out.contains("march-rent.psbt") {
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
    let v = vault("Savings", &["pass one"], 7);
    let (mut shell, mut app) = visiting(
        vec![file("vault.ofv", &v)],
        vec![file("vault.ofv", &v), file("notes.txt", b"n")],
    );
    app.press(Action::Nav(Screen::Files));
    menu_of(&mut app, Row::Out(0));
    choose(&mut app, "Rename");
    assert_eq!(
        app.files_menu.renaming.as_ref().map(|r| r.ext.as_str()),
        Some(".ofv"),
        "a vault keeps .ofv"
    );
    type_name(&mut app, "savings");
    app.press(Action::File(FilePress::RenameGo));
    assert_eq!(app.outbox[0].name, "savings.ofv");
    shell.press(&mut app, Action::Nav(Screen::Visit));
    shell.press(&mut app, Action::VisitWrite);
    let vaults: Vec<&str> = shell
        .disk("S1")
        .files
        .iter()
        .map(|(n, _)| n.as_str())
        .filter(|n| n.ends_with(".ofv"))
        .collect();
    assert_eq!(vaults, ["savings.ofv"], "one copy, under its new name");
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
    let k = app.sticks[0]
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
    let listed: Vec<&str> = app.sticks[0]
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
fn a_rename_to_a_name_on_the_stick_is_refused_and_a_same_vault_named() {
    let v = vault("Savings", &["pass one"], 7);
    let (mut shell, mut app) = visiting(
        Vec::new(),
        vec![
            file("a.txt", b"a"),
            file("B.txt", b"b"),
            file("vault.ofv", &v),
            file("vault-copy.ofv", &v),
        ],
    );
    let at = |app: &Faraday, name: &str| {
        app.sticks[0]
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
    let k = at(&app, "vault-copy.ofv");
    menu_of(&mut app, Row::Stick(k));
    choose(&mut app, "Rename");
    type_name(&mut app, "vault");
    app.press(Action::File(FilePress::RenameGo));
    shell.pump(&mut app);
    assert_eq!(shell.disk("S1").files, before, "nothing written");
    let log: Vec<&str> = app.visit.log.iter().map(|(l, _)| l.as_str()).collect();
    assert!(log.contains(&"b.txt is already on this stick"), "{log:?}");
    assert!(
        log.contains(&"vault.ofv on this stick is this same vault"),
        "{log:?}"
    );
}

#[test]
fn delete_from_the_stick_opens_the_sheet_with_that_file_alone_chosen() {
    let (_shell, mut app) = visiting(Vec::new(), vec![file("a.txt", b"a"), file("b.txt", b"b")]);
    let k = app.sticks[0]
        .files
        .iter()
        .position(|(n, _)| n == "b.txt")
        .unwrap();
    menu_of(&mut app, Row::Stick(k));
    choose(&mut app, "Delete from the stick");
    assert_eq!(app.sheet, Some(Sheet::Delete));
    assert_eq!(
        app.delete_ask.as_ref().map(|d| d.chosen.clone()),
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
    let v = vault("Savings", &["pass one"], 7);
    let (_shell, mut app) = visiting(Vec::new(), vec![file("vault.ofv", &v)]);
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
    let hold = Action::Vault(faraday_core::vaults::VaultAction::HoldWipe);
    assert!(app.offers(hold));
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "Hold to delete 1 file with zeros")
    );
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
    let texts = app.drawn_texts();
    for line in &sha {
        assert!(texts.contains(line), "{line} in {texts:?}");
    }
    assert!(texts.iter().any(|t| t == "19 bytes"), "{texts:?}");

    // On the stick, read there for it.
    let (mut shell, mut app) = visiting(Vec::new(), vec![file("note.txt", bytes)]);
    menu_of(&mut app, Row::Stick(0));
    choose(&mut app, "Details");
    shell.pump(&mut app);
    let texts = app.drawn_texts();
    assert!(
        texts.contains(&sha[0]) && texts.contains(&sha[1]),
        "{texts:?}"
    );
}

#[test]
fn a_psbt_row_shows_sign_and_a_dots_menu_of_details_and_remove() {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    app.inbox.push(Item {
        name: "spend.psbt".into(),
        bytes: b"psbt".to_vec(),
        kind: FileKind::Psbt,
        secret: false,
        picture: None,
        sha: None,
        wallet: None,
        psbt: None,
    });
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    assert!(app.offers(Action::StartSpend(0)), "Sign");
    assert!(app.drawn_texts().iter().any(|t| t == "Sign"));
    assert_eq!(menu_of(&mut app, Row::In(0)), ["Details", "Remove"]);
    choose(&mut app, "Remove");
    assert_eq!(app.sheet, None, "a choice closes the menu");
    assert!(app.inbox.is_empty(), "Remove, through the ⋮");
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
        assert!(app.offers(Action::File(FilePress::Menu(Row::In(0)))));
        // On the small panel For the stick is further down the page.
        assert!(compact || app.offers(Action::File(FilePress::Menu(Row::Out(0)))));
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
