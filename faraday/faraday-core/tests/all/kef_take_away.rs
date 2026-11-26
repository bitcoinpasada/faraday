//! The end of the Krux code tool where the pair stands (`docs/WORKSPACE.md`
//! §15): the take-away holds the code's picture and file, ticked, with one
//! button that writes them, and never a file that may be a secret.

use faraday_core::kef_form::KefAct;
use faraday_core::kef_tool::{KefToolAction as K, Pick};
use faraday_core::strings::EN;
use faraday_core::{Action, Faraday, Screen, Sheet, StickInfo, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};

fn type_in(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// The tool at its made code, at `width` by `height`.
fn made_at(width: u16, height: u16) -> Faraday {
    let mut app = testkit::started();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x40 + i; 32])));
    }
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Krux code")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    app.press(Action::KefTool(K::Pick(Pick::Type)));
    type_in(&mut app, "a note for later");
    app.press(Action::KefTool(K::Next));
    app.press(Action::KefTool(K::Form(KefAct::Options)));
    app.press(Action::KefTool(K::Form(KefAct::Rounds)));
    for _ in 0..8 {
        app.event(Event::Key(Key::Backspace));
    }
    type_in(&mut app, "10000");
    app.press(Action::KefTool(K::Form(KefAct::Back)));
    app.press(Action::KefTool(K::Form(KefAct::Field)));
    type_in(&mut app, "a passphrase");
    app.press(Action::KefTool(K::Form(KefAct::FieldAgain)));
    type_in(&mut app, "a passphrase");
    app.press(Action::KefTool(K::Form(KefAct::Make)));
    testkit::settle(&mut app);
    let _ = app.frame();
    app
}

fn names(app: &Faraday) -> Vec<String> {
    app.outbox.iter().map(|i| i.name.to_string()).collect()
}

fn shown(app: &mut Faraday, text: &str) -> bool {
    let _ = app.frame();
    app.drawn_text_boxes().iter().any(|(t, _)| t == text)
}

/// The file rows drawn for the code's files: a name alone, whole or cut.
fn rows(app: &mut Faraday) -> usize {
    let _ = app.frame();
    app.drawn_text_boxes()
        .iter()
        .filter(|(t, _)| t.starts_with("krux-v20-") && !t.contains(' '))
        .count()
}

fn stick_in(app: &mut Faraday) {
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("one"),
        label: faraday_text::Text::of("FIRST"),
        boot: false,
        files: Vec::new(),
        vaults: Vec::new(),
    }]));
}

/// A stick comes in. The tool holds what was typed, so its arrival asks to
/// lock first, and the person leaves that for now.
fn stick_arrives(app: &mut Faraday) {
    stick_in(app);
    app.press(Action::NotNow);
    let _ = app.frame();
}

/// Write, the lock it asks for and the restart: what the next process
/// then sends to the stick.
fn written_through_the_lock(app: &mut Faraday) -> Vec<String> {
    app.press(Action::TakeWrite);
    assert_eq!(app.sheet, Some(Sheet::WriteOut));
    assert_eq!(writes(app), Vec::<String>::new(), "nothing before the lock");
    app.press(Action::Lock);
    let mut saved = None;
    while let Some(c) = app.poll_storage() {
        if let faraday_core::StorageCommand::SaveBoxes { outbox, kept, .. } = c {
            saved = Some((outbox, kept));
        }
    }
    let (outbox, kept) = saved.expect("the lock kept the boxes");
    let mut next = testkit::started();
    next.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox,
        kept,
    });
    while next.poll_storage().is_some() {}
    stick_in(&mut next);
    writes(&mut next)
}

fn writes(app: &mut Faraday) -> Vec<String> {
    let mut names = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let faraday_core::StorageCommand::Write { name, .. } = c {
            names.push(name.to_string());
        }
    }
    names.sort();
    names
}

#[test]
fn the_made_code_s_picture_and_file_are_in_the_take_away_ticked_and_written() {
    for (w, h) in [(1366, 768), (1920, 1080)] {
        let mut app = made_at(w, h);
        let mut made = names(&app);
        made.sort();
        assert_eq!(made.len(), 2, "{w}x{h}");
        assert_eq!(rows(&mut app), 2, "{w}x{h}");
        assert!(shown(&mut app, EN.take_away_head), "{w}x{h}");
        assert!(!shown(&mut app, EN.flow_your_choices), "{w}x{h}");
        for name in &made {
            assert!(app.visit().out.contains(&**name), "ticked: {name}");
        }
        assert!(shown(&mut app, EN.kef_tool_make_another_code));
        stick_arrives(&mut app);
        assert_eq!(app.screen, Screen::KefTool, "{w}x{h}");
        assert_eq!(rows(&mut app), 2, "the stick changes no row, {w}x{h}");
        assert!(app.offers(Action::TakeWrite), "{w}x{h}");
        assert_eq!(written_through_the_lock(&mut app), made, "{w}x{h}");
    }
}

#[test]
fn a_file_that_may_be_a_secret_is_neither_offered_nor_sent() {
    let mut app = made_at(1366, 768);
    // Another file waits, chosen, and it is a secret one.
    testkit::outbox_file(&mut app, "words.txt", b"a secret");
    app.outbox.last_mut().unwrap().secret = true;
    app.press(Action::VisitOut(app.outbox.len() - 1));
    // And one of the code's own two is marked as one that may be.
    let held = app
        .outbox
        .iter_mut()
        .find(|i| i.name.ends_with(".kef"))
        .expect("the code's file");
    held.secret = true;
    let held = held.name.to_string();
    let picture = names(&app)
        .into_iter()
        .find(|n| n.ends_with(".png"))
        .expect("the picture");
    stick_arrives(&mut app);
    assert!(
        app.visit().out.contains(&*held),
        "chosen, and still not sent"
    );
    assert_eq!(rows(&mut app), 1, "the picture's row only");
    assert!(!shown(&mut app, "words.txt"));
    assert_eq!(written_through_the_lock(&mut app), vec![picture]);
}

#[test]
fn a_removed_file_is_made_again_under_its_name() {
    let mut app = made_at(1366, 768);
    let before = names(&app);
    let gone = before.iter().position(|n| n.ends_with(".kef")).unwrap();
    // Without leaving: leaving the tool drops it and its take-away.
    app.press(Action::OutboxRemove(gone));
    assert!(shown(&mut app, EN.kef_tool_make_the_file));
    // The picture, unticked by the person, stays so.
    let picture_name = before.iter().find(|n| n.ends_with(".png")).unwrap().clone();
    app.press(Action::VisitOut(0));
    assert!(!app.visit().out.contains(&*picture_name));
    app.press(Action::KefTool(K::SaveFiles));
    let mut after = names(&app);
    after.sort();
    let mut before = before;
    before.sort();
    assert_eq!(after, before);
    let file_name = after.iter().find(|n| n.ends_with(".kef")).unwrap();
    assert!(app.visit().out.contains(&**file_name), "ticked when made");
    assert!(!app.visit().out.contains(&*picture_name));
    assert!(!shown(&mut app, EN.kef_tool_make_the_file));
}

#[test]
fn the_smaller_size_keeps_the_card_alone() {
    let mut app = made_at(480, 640);
    assert!(!shown(&mut app, EN.take_away_head));
}
