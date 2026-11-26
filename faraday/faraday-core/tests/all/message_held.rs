//! A wallet whose key waits for its backup is not offered in Sign a
//! message: Sign is refused and says why, and no file, QR code or sheet
//! with the address is made. A signed message's file made before the key
//! was held stays in Files, and is neither written nor shown as a code
//! until the key is backed up.

use faraday_core::address_check::AddressPress;
use faraday_core::catalog::Go;
use faraday_core::file_menu::{FilePress, Row};
use faraday_core::scan_read::Way;
use faraday_core::strings::EN;
use faraday_core::work::Work;
use faraday_core::{
    Action, Ask, Code, Faraday, Screen, StickInfo, StorageCommand, StorageEvent, testkit,
};
use faraday_psbt::message::{Format, Signed};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

/// A key of the test words with its single-key wallet.
fn app() -> Faraday {
    let c = testkit::compute();
    let mut app = testkit::started();
    app.session = testkit::session();
    app.session
        .keys
        .add_words(
            &c,
            &testkit::test_words(testkit::TEST_SEEDS[0].0),
            "Bacon",
            None,
        )
        .expect("the key");
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    app.session
        .add_wallet_confirmed(
            &c,
            kit.name,
            &kit.descriptor,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .expect("the wallet");
    app
}

fn hold(app: &mut Faraday) {
    let fp = app.session.keys.last().unwrap().fingerprint;
    testkit::mark_made_here(app, fp);
}

fn back_up(app: &mut Faraday) {
    let fp = app.session.keys.last().unwrap().fingerprint;
    testkit::release_made(app, fp);
}

/// A signature of the wallet's first address, made beside the flow: what the
/// flow would hold had it signed.
fn signature(app: &Faraday) -> Signed {
    app.session
        .sign_message(&testkit::compute(), 0, 0, "hello", Format::Bip322)
        .expect("signs")
}

/// Sign a message open on the held wallet, chosen by hand.
fn held_flow() -> Faraday {
    let mut app = app();
    hold(&mut app);
    app.press(Action::SignMessage);
    app.press(Action::MWallet(0));
    app
}

/// The refusal said, and nothing made: no file and no code.
fn assert_refused(app: &Faraday) {
    let m = testkit::message(app).expect("open");
    assert_eq!(m.error.as_deref(), Some(EN.message_waits_backup));
    assert!(m.out_name.is_none());
    assert!(app.outbox.is_empty(), "no file");
    assert!(app.qr().is_none(), "no code");
}

#[test]
fn a_held_wallet_is_not_offered() {
    let mut app = app();
    assert_eq!(app.tile_need(Go::SignMessage), None);
    hold(&mut app);
    assert_eq!(
        app.tile_need(Go::SignMessage),
        Some(EN.message_waits_backup)
    );
    app.press(Action::SignMessage);
    assert_eq!(testkit::message(&app).unwrap().wallet, None);
    app.press(Action::SignMessageWith(0));
    assert_eq!(testkit::message(&app).unwrap().wallet, None);
}

#[test]
fn sign_is_refused() {
    let mut app = held_flow();
    let before = faraday_core::work::snapshot();
    app.press(Action::MSign);
    assert_eq!(
        faraday_core::work::since(&before).of(Work::Sign),
        0,
        "nothing was signed"
    );
    assert!(testkit::message(&app).unwrap().signed.is_none());
    assert_refused(&app);
}

#[test]
fn the_address_checks_sign_button_opens_nothing_for_a_held_key() {
    let mut app = app();
    let address = signature(&app).address;
    hold(&mut app);
    app.press(Action::ScanFor(Ask::Address));
    app.event(Event::Scanned {
        bytes: address.as_bytes().to_vec(),
    });
    let _ = app.frame();
    let way = app
        .address_check_state()
        .and_then(|s| s.ways().iter().position(|w| matches!(w, Way::SignAddress)))
        .expect("the result's way to Sign a message");
    app.press(Action::AddressCheck(AddressPress::Way(way as u8)));
    assert!(testkit::message(&app).is_none());
    assert_ne!(app.screen, Screen::Message);
    assert_eq!(app.toast_text(), Some(EN.message_waits_backup));
}

#[test]
fn the_file_is_refused() {
    let mut app = held_flow();
    let signed = signature(&app);
    testkit::message_put_signed(&mut app, signed);
    app.press(Action::MOut);
    assert_refused(&app);
}

#[test]
fn the_code_is_refused() {
    let mut app = held_flow();
    let signed = signature(&app);
    testkit::message_put_signed(&mut app, signed);
    app.press(Action::MQr);
    assert_refused(&app);
}

#[test]
fn the_code_and_its_sheet_are_not_built() {
    let mut app = held_flow();
    let signed = signature(&app);
    testkit::message_put_signed(&mut app, signed);
    let why = app
        .code_view(&testkit::compute(), Code::Message)
        .err()
        .expect("no code");
    assert_eq!(why.as_str(), EN.message_waits_backup);
}

#[test]
fn check_it_is_refused() {
    let mut app = held_flow();
    let signed = signature(&app);
    testkit::message_put_signed(&mut app, signed);
    app.press(Action::CheckMessageMade);
    assert!(!testkit::message_made_kept(&app));
    assert_eq!(app.screen, Screen::Message);
    assert_refused(&app);
}

#[test]
fn a_key_held_after_signing_drops_what_was_made() {
    let mut app = app();
    app.press(Action::SignMessage);
    app.press(Action::MSign);
    app.press(Action::CheckMessageMade);
    assert!(testkit::message_made_kept(&app));
    hold(&mut app);
    assert!(!testkit::message_made_kept(&app));
    let m = testkit::message(&app).unwrap();
    assert!(m.signed.is_none());
    assert!(m.out_name.is_none() && m.out_sha.is_none());
    assert!(!m.done[faraday_core::mstep::SIGN as usize]);
}

#[test]
fn a_picked_address_of_a_held_key_opens_nothing() {
    let mut app = app();
    testkit::message_on_address(&mut app, "bc1qpicked");
    assert!(testkit::message(&app).unwrap().at.is_some());
    app.press(Action::MDone);
    hold(&mut app);
    testkit::message_on_address(&mut app, "bc1qpicked");
    assert!(testkit::message(&app).is_none());
}

#[test]
fn a_flow_left_half_way_signs_once_the_key_is_backed_up() {
    let mut app = held_flow();
    app.press(Action::MSign);
    assert!(testkit::message(&app).unwrap().signed.is_none());
    back_up(&mut app);
    app.press(Action::MType);
    assert_eq!(app.tile_need(Go::SignMessage), None);
    app.press(Action::MSign);
    let m = testkit::message(&app).unwrap();
    assert!(m.signed.is_some());
    assert!(m.error.is_none());
    assert_eq!(app.outbox.len(), 1);
}

/// Sign a message signed with the key not held, its file For the stick and
/// ticked, on a screen wide enough for Files' rows.
fn signed_file() -> (Faraday, String) {
    let mut app = app();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.press(Action::SignMessage);
    testkit::message_text(&mut app, "hello");
    app.press(Action::MSign);
    let name = app.outbox.first().expect("the file").name.to_string();
    assert!(app.visit().out.contains(name.as_str()));
    (app, name)
}

fn row_says_it_waits(app: &mut Faraday) -> bool {
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    app.drawn_text_boxes()
        .iter()
        .any(|(t, _)| t == EN.message_waits_backup)
}

/// Show as QR on Files' first file: whether a code came, or the refusal.
fn code_shown(app: &mut Faraday) -> bool {
    app.press(Action::QrOutbox(0));
    let shown = app.qr().is_some();
    if !shown {
        assert_eq!(app.toast_text(), Some(EN.message_waits_backup));
    }
    shown
}

/// The app after a lock and the restart, over what the lock kept.
fn restarted(mut app: Faraday) -> Faraday {
    app.press(Action::Lock);
    let mut saved = None;
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { outbox, kept, .. } = c {
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
    next
}

/// A stick visit's Write with everything Select all takes, and `name`
/// ticked besides when `tick`: the names the write sends. `waits` says
/// that Select all is to leave `name` out.
fn visit_writes(app: &mut Faraday, name: &str, tick: bool, waits: bool) -> Vec<String> {
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("one"),
        label: faraday_text::Text::of("FIRST"),
        boot: false,
        files: Vec::new(),
        vaults: Vec::new(),
    }]));
    // The stick arrives with what it can take ticked: untick all, then
    // Select all.
    if !app.visit().out.is_empty() {
        app.press(Action::VisitOutAll);
    }
    assert!(app.visit().out.is_empty(), "all unticked");
    app.press(Action::VisitOutAll);
    assert_eq!(app.visit().out.contains(name), !waits, "Select all");
    if tick {
        testkit::visit_tick(app, name);
    }
    while app.poll_storage().is_some() {}
    app.press(Action::VisitWrite);
    let mut names = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Write { name, .. } = c {
            names.push(name.to_string());
        }
    }
    names
}

/// The signed file `name` of an app whose key is now held: it stays in
/// Files, unticked, its row says why, it is no code, and a stick visit
/// writes the other file and not it, a tick left standing included. Backed
/// up, it is written, and it is a code when it is small enough for one
/// (`a_code`).
fn held_back_until_backed_up(mut app: Faraday, name: &str, a_code: bool) {
    let fp = app.session.keys.last().unwrap().fingerprint;
    assert!(app.outbox.iter().any(|i| i.name == name), "it stays");
    assert!(!app.visit().out.contains(name), "unticked");
    assert!(row_says_it_waits(&mut app));
    app.press(Action::VisitOut(0));
    assert!(!app.visit().out.contains(name), "no tick");
    assert!(!code_shown(&mut app));

    testkit::outbox_file(&mut app, "other.txt", b"other");
    // The lock and the restart: the next process knows who signed it.
    let mut next = restarted(app);
    testkit::mark_made_here(&mut next, fp);
    assert!(row_says_it_waits(&mut next));
    assert!(!code_shown(&mut next));
    let written = visit_writes(&mut next, name, true, true);
    assert!(written.iter().any(|n| n == "other.txt"), "{written:?}");
    assert!(!written.iter().any(|n| n == name), "{written:?}");

    testkit::release_made(&mut next, fp);
    // The refusal's toast says the same words: its time runs out first.
    next.event(Event::Tick {
        now_ms: 10 * faraday_draw::tokens::TOAST_SHOW_MS,
    });
    assert_eq!(next.toast_text(), None);
    next.press(Action::Nav(Screen::Files));
    assert!(!row_says_it_waits(&mut next));
    if a_code {
        assert!(code_shown(&mut next));
        next.press(Action::Cancel);
    } else {
        // Too large for a code: nothing says it waits.
        next.press(Action::QrOutbox(0));
        assert!(next.qr().is_none());
        assert_eq!(next.toast_text(), None);
    }
    let written = visit_writes(&mut next, name, true, false);
    assert!(written.iter().any(|n| n == name), "{written:?}");
}

#[test]
fn a_file_made_before_the_hold_is_held_back() {
    let (mut app, name) = signed_file();
    hold(&mut app);
    held_back_until_backed_up(app, &name, true);
}

/// The signed message's picture, made by `presses` before the hold, is held
/// back as the text file is.
fn picture_is_held_back(presses: &[Action]) {
    let (mut app, text) = signed_file();
    for press in presses {
        app.press(*press);
    }
    if app.qr().is_some() {
        app.press(Action::Cancel);
    }
    let picture = app
        .outbox
        .iter()
        .map(|i| i.name.to_string())
        .find(|n| n.ends_with(".png"))
        .expect("the picture");
    // The text file goes, so the first row is the picture's.
    app.press(Action::Nav(Screen::Files));
    assert_eq!(app.outbox[0].name.as_str(), text);
    app.press(Action::OutboxRemove(0));
    if app.sheet.is_some() {
        app.press(Action::OutboxDeleteHold);
    }
    assert_eq!(app.outbox.len(), 1);
    // More than a file shown as a code may hold.
    assert!(app.outbox[0].bytes.len() > 256 * 1024);
    hold(&mut app);
    held_back_until_backed_up(app, &picture, false);
}

#[test]
fn a_picture_for_the_stick_made_before_the_hold_is_held_back() {
    picture_is_held_back(&[Action::CodePng(Code::Message)]);
}

#[test]
fn a_picture_from_the_code_sheet_made_before_the_hold_is_held_back() {
    picture_is_held_back(&[Action::MQr, Action::QrPng]);
}

#[test]
fn a_file_of_a_flow_ended_with_done_is_held_back() {
    let (mut app, name) = signed_file();
    app.press(Action::MDone);
    assert!(testkit::message(&app).is_none());
    hold(&mut app);
    held_back_until_backed_up(app, &name, true);
}

#[test]
fn a_file_signed_before_the_number_changed_is_held_back() {
    let (mut app, name) = signed_file();
    app.press(Action::MIndex(1));
    assert!(testkit::message(&app).unwrap().signed.is_none());
    hold(&mut app);
    held_back_until_backed_up(app, &name, true);
}

#[test]
fn a_renamed_signed_file_is_held_back_still() {
    let (mut app, name) = signed_file();
    app.press(Action::Nav(Screen::Files));
    app.press(Action::File(FilePress::Rename(Row::Out(0))));
    app.event(Event::Key(Key::Char('x')));
    app.press(Action::File(FilePress::RenameGo));
    let renamed = app.outbox[0].name.to_string();
    assert_ne!(renamed, name);
    hold(&mut app);
    assert!(!code_shown(&mut app));
}

#[test]
fn another_file_under_the_name_is_not_held_back() {
    let (mut app, name) = signed_file();
    app.press(Action::OutboxRemove(0));
    if app.sheet.is_some() {
        app.press(Action::OutboxDeleteHold);
    }
    assert!(app.outbox.is_empty());
    testkit::outbox_file(&mut app, &name, b"other");
    hold(&mut app);
    assert!(!row_says_it_waits(&mut app));
    assert!(code_shown(&mut app));
}
