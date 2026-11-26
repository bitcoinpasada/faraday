//! The end of Sign a message, where the pair stands: the take-away is the
//! pane at the left in place of Your choices, a tick row for the file and
//! one button at its foot (off with nothing ticked, "Waiting for a stick"
//! with no stick, "Written and checked" after the write). Other sizes keep
//! the card as it was.

use faraday_core::file_menu::{FilePress, Row};
use faraday_core::strings::EN;
use faraday_core::ui::TextSize;
use faraday_core::{Action, Faraday, Screen, Sheet, StickInfo, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

fn signed_at(width: u16, height: u16) -> Faraday {
    let c = testkit::compute();
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
    app.press(Action::SignMessage);
    testkit::message_text(&mut app, "hello");
    app.press(Action::MSign);
    app.press(Action::MStep(faraday_core::mstep::SIGN));
    let _ = app.frame();
    app
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
    // Keys are in memory after the signing: the stick's arrival asks to
    // lock first, and the person leaves that for now.
    app.press(Action::NotNow);
    let _ = app.frame();
}

fn shown(app: &mut Faraday, text: &str) -> bool {
    let _ = app.frame();
    app.drawn_text_boxes().iter().any(|(t, _)| t == text)
}

fn file(app: &Faraday) -> String {
    app.outbox.first().expect("the file waits").name.to_string()
}

fn write_button() -> String {
    format!("Write to {}", EN.medium_stick_inline)
}

#[test]
fn the_pane_holds_the_take_away_at_the_last_step_on_both_wide_sizes() {
    for (w, h) in [(1366, 768), (1920, 1080)] {
        let mut app = signed_at(w, h);
        let name = file(&app);
        assert!(shown(&mut app, EN.take_away_head), "{w}x{h}");
        assert!(shown(&mut app, &name), "{w}x{h}");
        assert!(!shown(&mut app, EN.flow_your_choices), "{w}x{h}");
        assert!(app.offers(Action::MQr));
        // The card holds neither the file nor the code buttons: Show as QR
        // is offered once, and the PNG not at all.
        assert_eq!(app.offered_times(Action::MQr), 1, "{w}x{h}");
        assert!(!app.offers(Action::CodePng(faraday_core::Code::Message)));
        assert_eq!(
            app.drawn_text_boxes()
                .iter()
                .filter(|(t, _)| **t == *name)
                .count(),
            1,
            "the file is drawn once, in the pane"
        );
    }
}

#[test]
fn no_sheet_opens_at_the_end() {
    let app = signed_at(1366, 768);
    assert_eq!(app.sheet, None::<Sheet>);
}

#[test]
fn a_tick_is_the_visit_chosen_set_and_the_button_follows_it() {
    let mut app = signed_at(1366, 768);
    stick_in(&mut app);
    let name = file(&app);
    assert!(app.visit().out.contains(&*name), "ticked when made");
    assert!(app.offers(Action::TakeWrite));
    assert!(shown(&mut app, &write_button()));
    app.press(Action::VisitOut(0));
    let _ = app.frame();
    assert!(!app.visit().out.contains(&*name));
    assert!(!app.offers(Action::TakeWrite), "off with nothing ticked");
    assert!(shown(&mut app, &write_button()));
    app.press(Action::VisitOut(0));
    let _ = app.frame();
    assert!(app.offers(Action::TakeWrite));
}

#[test]
fn with_no_stick_the_button_reads_waiting_for_a_stick() {
    let mut app = signed_at(1366, 768);
    let waiting = format!("Waiting for {}", "a stick");
    assert!(shown(&mut app, &waiting));
    let n = app
        .drawn_text_boxes()
        .iter()
        .filter(|(t, _)| **t == *waiting)
        .count();
    assert_eq!(n, 2, "said on the file's row and on the button");
    assert!(!app.offers(Action::TakeWrite));
    assert!(app.outbox.len() == 1, "the file waits in the outbox");
}

#[test]
fn with_keys_in_memory_the_button_asks_to_lock_before_it_writes() {
    let mut app = signed_at(1366, 768);
    stick_in(&mut app);
    assert!(app.offers(Action::TakeWrite));
    app.press(Action::TakeWrite);
    assert_eq!(app.sheet, Some(Sheet::WriteOut));
    assert!(app.outbox.len() == 1, "the file still waits");
}

#[test]
fn back_to_the_step_before_brings_your_choices_back() {
    let mut app = signed_at(1366, 768);
    assert!(shown(&mut app, EN.take_away_head));
    app.press(Action::MStep(faraday_core::mstep::FORMAT));
    assert!(shown(&mut app, EN.flow_your_choices));
    assert!(!shown(&mut app, EN.take_away_head));
    app.press(Action::MStep(faraday_core::mstep::SIGN));
    assert!(shown(&mut app, EN.take_away_head));
}

fn sha_of(app: &Faraday) -> [u8; 32] {
    faraday_crypto::sha256_whole(&testkit::compute(), &app.outbox[0].bytes)
}

#[test]
fn written_and_checked_holds_for_the_same_name_and_the_same_bytes_only() {
    // Another message under the same name: a receipt of the same name and
    // other bytes does not make this file written.
    let mut app = signed_at(1366, 768);
    let name = file(&app);
    app.outbox.clear();
    testkit::receipt_written(&mut app, &name, [7; 32]);
    assert!(!shown(&mut app, EN.pull_line_written_checked));
    assert!(shown(&mut app, EN.message_make_the_file) || shown(&mut app, EN.made_state_make_again));
    // The right bytes: written and checked.
    let mut app = signed_at(1366, 768);
    let (name, sha) = (file(&app), sha_of(&app));
    app.outbox.clear();
    testkit::receipt_written(&mut app, &name, sha);
    assert!(shown(&mut app, EN.pull_line_written_checked));
    assert!(!shown(&mut app, EN.message_make_the_file));
    assert!(!shown(&mut app, EN.made_state_make_again));
}

#[test]
fn a_file_removed_in_files_is_offered_to_make_again_and_made() {
    let mut app = signed_at(1366, 768);
    let name = file(&app);
    app.press(Action::Nav(Screen::Files));
    app.press(Action::OutboxRemove(0));
    assert!(app.outbox.is_empty());
    app.press(Action::Back);
    app.press(Action::MStep(faraday_core::mstep::SIGN));
    assert!(shown(&mut app, EN.take_away_head));
    assert!(shown(&mut app, EN.message_make_the_file));
    assert!(app.offers(Action::MOut));
    app.press(Action::MOut);
    assert_eq!(file(&app), name, "made again under its name");
    assert!(app.visit().out.contains(&*name), "ticked");
    assert!(!shown(&mut app, EN.message_make_the_file));
}

/// The first signature's file still waits: the second's, removed, is named
/// as it will be made, beside it.
#[test]
fn a_removed_file_is_offered_under_the_name_it_is_made_under() {
    let mut app = signed_at(1366, 768);
    let first = file(&app);
    testkit::message_text(&mut app, "second");
    app.press(Action::MSign);
    let second = app.outbox[1].name.to_string();
    assert!(second.ends_with("-2.txt"), "{second}");
    app.press(Action::Nav(Screen::Files));
    app.press(Action::OutboxRemove(1));
    app.press(Action::Back);
    app.press(Action::MStep(faraday_core::mstep::SIGN));
    assert!(shown(&mut app, EN.message_make_the_file));
    // The row may cut the name in "…"; what it shows is the second's alone.
    let names: Vec<String> = app
        .drawn_text_boxes()
        .iter()
        .map(|(t, _)| t.to_string())
        .filter(|t| t.starts_with("message-") && !t.contains(' '))
        .collect();
    assert!(!names.is_empty());
    for name in names {
        let (start, end) = name.split_once('…').unwrap_or((&name, &name));
        let of = |file: &str| file.starts_with(start) && file.ends_with(end);
        assert!(of(&second) && !of(&first), "{name}");
    }
    app.press(Action::MOut);
    assert_eq!(app.outbox[1].name.as_str(), second);
}

#[test]
fn a_file_renamed_in_files_is_offered_to_make_again() {
    let mut app = signed_at(1366, 768);
    let name = file(&app);
    // What Rename in Files leaves: the same file under another name.
    app.press(Action::Nav(Screen::Files));
    app.outbox[0].name = faraday_text::Text::of("mine.txt");
    app.press(Action::Back);
    app.press(Action::MStep(faraday_core::mstep::SIGN));
    assert!(shown(&mut app, EN.message_make_the_file));
    assert!(app.offers(Action::MOut));
    assert!(!shown(&mut app, EN.pull_line_written_checked));
    app.press(Action::MOut);
    assert_eq!(app.outbox.len(), 2);
    assert!(app.outbox.iter().any(|i| i.name == *name));
}

#[test]
fn the_smaller_sizes_keep_the_card_with_its_file() {
    // The card's own file row (name and its Remove) at the sizes where the
    // pane does not stand. Every size below the expanded rail is one column,
    // where the card is never given to the pane: none of the sizes tried (800x600 to 1920x1080) reaches the card's
    // `pair_left` gate in the wide layout (1200x800 and up have the pane,
    // 1100x700 and down are one column), so the gate itself is held by the
    // wide-size test above (the file drawn once, in the pane).
    for (w, h) in [(480, 640), (320, 480), (1024, 600)] {
        let mut app = signed_at(w, h);
        let name = file(&app);
        assert!(!shown(&mut app, EN.take_away_head), "{w}x{h}");
        assert!(app.offers(Action::MQr), "{w}x{h}: Show as QR in the card");
        if w >= 480 {
            // At 320 wide the name is cut by the card's row.
            assert!(shown(&mut app, &name), "{w}x{h}: the file in the card");
        }
        assert!(
            app.offers(Action::OutboxRemove(0)),
            "{w}x{h}: its Remove, as before"
        );
        assert!(!shown(&mut app, &write_button()), "{w}x{h}");
    }
}

const SEED_MESSAGE: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// A message that is itself a seed has no file: nothing waits, no toast
/// says so, no Make the file is offered; the code still is.
#[test]
fn a_message_that_is_a_seed_makes_no_file_and_offers_its_code_only() {
    for (w, h) in [(1366, 768), (480, 640)] {
        let mut app = signed_at(w, h);
        app.press(Action::OutboxRemove(0));
        assert!(app.outbox.is_empty());
        app.press(Action::MStep(faraday_core::mstep::TEXT));
        testkit::message_text(&mut app, SEED_MESSAGE);
        app.press(Action::MSign);
        assert!(testkit::message(&app).unwrap().signed.is_some());
        assert!(app.outbox.is_empty(), "nothing is written");
        let toast = app.toast_text().unwrap_or_default().to_string();
        assert!(!toast.contains("waits"), "{toast}");
        assert_eq!(toast, EN.secret_text_those_are);
        app.press(Action::MStep(faraday_core::mstep::SIGN));
        assert!(!shown(&mut app, EN.message_make_the_file));
        assert!(!app.offers(Action::MOut));
        assert!(app.offers(Action::MQr), "the code stays offered");
        if w >= 1366 {
            // A code and no file: no button (WORKSPACE §15).
            assert!(!app.offers(Action::TakeWrite));
            assert!(!shown(&mut app, &write_button()));
            assert!(!shown(&mut app, "Waiting for a stick"));
        }
    }
}

/// An ordinary message is filed and says it waits, as before.
#[test]
fn an_ordinary_message_still_waits_as_a_file() {
    let app = signed_at(1366, 768);
    assert_eq!(app.outbox.len(), 1);
    assert!(app.toast_text().is_some_and(|t| t.contains("waits")));
}

/// The take-away's write sends its own file and nothing else the visit has
/// chosen; Files' Write still sends everything chosen.
#[test]
fn the_take_away_writes_only_its_own_rows_files() {
    let mut app = signed_at(1366, 768);
    stick_in(&mut app);
    let name = file(&app);
    // Another file chosen in the visit.
    testkit::outbox_file(&mut app, "other.txt", b"other");
    app.press(Action::VisitOut(1));
    assert!(app.visit().out.contains("other.txt"));
    // A clean process: nothing in memory to lock first.
    testkit::made_clean(&mut app);
    let _ = app.frame();
    assert!(app.visit().out.contains("other.txt"), "still chosen");
    app.press(Action::TakeWrite);
    let mut names = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let faraday_core::StorageCommand::Write { name, .. } = c {
            names.push(name.to_string());
        }
    }
    assert_eq!(names, vec![name], "only the take-away's own file");
}

fn stick_with(files: &[&str]) -> StickInfo {
    StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("one"),
        label: faraday_text::Text::of("FIRST"),
        boot: false,
        files: files
            .iter()
            .map(|n| (faraday_text::Text::of(n), 5))
            .collect(),
        vaults: Vec::new(),
    }
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

/// The write that waits for the lock and the restart sends the pane's file
/// only, though another file is chosen in the visit.
#[test]
fn the_write_through_the_lock_sends_only_the_panes_file() {
    let mut app = signed_at(1366, 768);
    stick_in(&mut app);
    let name = file(&app);
    testkit::outbox_file(&mut app, "other.txt", b"other");
    app.press(Action::VisitOut(1));
    assert!(app.visit().out.contains("other.txt"));
    app.press(Action::TakeWrite);
    assert_eq!(app.sheet, Some(Sheet::WriteOut));
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
    // The name is taken on the stick, so the write stops to ask; the other
    // file's tick pressed under that sheet still does not send it.
    next.storage(StorageEvent::Sticks(vec![stick_with(&[&name])]));
    assert_eq!(next.sheet, Some(Sheet::Taken));
    let other = next
        .outbox
        .iter()
        .position(|i| i.name == "other.txt")
        .expect("it waits");
    if !next.visit().out.contains("other.txt") {
        next.press(Action::VisitOut(other));
    }
    assert!(next.visit().out.contains("other.txt"), "chosen");
    next.press(Action::VisitKeepBoth);
    assert_eq!(writes(&mut next), vec![name], "the pane's file only");
}

/// The sheet before the lock says what that write sends and to which
/// stick: not that the visit writes everything waiting.
#[test]
fn the_sheet_before_the_lock_names_what_the_write_sends() {
    let mut app = signed_at(1366, 768);
    stick_in(&mut app);
    let name = file(&app);
    testkit::outbox_file(&mut app, "other.txt", b"other");
    app.press(Action::VisitOut(1));
    app.press(Action::TakeWrite);
    assert_eq!(app.sheet, Some(Sheet::WriteOut));
    let _ = app.frame();
    let drawn: Vec<String> = app
        .drawn_text_boxes()
        .iter()
        .map(|(t, _)| t.to_string())
        .collect();
    let drawn = drawn.join(" ");
    let want = format!("Lock, then write {name} to FIRST");
    assert!(drawn.contains(&want), "{drawn}");
    assert!(!drawn.contains(EN.write_out_lock_here), "{drawn}");
}

/// The idle lock is no press: the write the sheet asked about is not kept,
/// for the take-away's sheet as for a spend's.
#[test]
fn the_idle_lock_gives_the_waiting_write_up() {
    let mut app = signed_at(1366, 768);
    stick_in(&mut app);
    app.event(Event::Tick { now_ms: 0 });
    app.press(Action::TakeWrite);
    assert_eq!(app.sheet, Some(Sheet::WriteOut));
    while app.poll_storage().is_some() {}
    app.event(Event::Tick {
        now_ms: u64::from(app.idle_lock_min()) * 60_000,
    });
    assert!(app.restart_requested(), "the idle lock came");
    assert!(!keeps_a_write(&mut app));
}

/// Whether what the lock kept holds a write waiting for the next stick.
fn keeps_a_write(app: &mut Faraday) -> bool {
    let mut kept = None;
    while let Some(c) = app.poll_storage() {
        if let faraday_core::StorageCommand::SaveBoxes { kept: k, .. } = c {
            kept = Some(k);
        }
    }
    testkit::plain_kept(kept.expect("the lock kept the boxes"))
        .iter()
        .any(|(n, _)| n == "write-waiting")
}

/// Not now on the sheet gives the write up: a later Lock keeps none.
#[test]
fn not_now_before_the_lock_keeps_no_write_for_a_later_lock() {
    let mut app = signed_at(1366, 768);
    stick_in(&mut app);
    app.press(Action::TakeWrite);
    assert_eq!(app.sheet, Some(Sheet::WriteOut));
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    while app.poll_storage().is_some() {}
    app.press(Action::LockUnsealed);
    assert!(!keeps_a_write(&mut app));
}

/// The stick pulled and put back while the sheet that names the file is up:
/// the Lock sheet takes its place, names no file, and its Lock writes none.
#[test]
fn a_lock_sheet_that_takes_the_naming_sheets_place_gives_the_write_up() {
    let mut app = signed_at(1366, 768);
    stick_in(&mut app);
    app.press(Action::TakeWrite);
    assert_eq!(app.sheet, Some(Sheet::WriteOut));
    app.storage(StorageEvent::Sticks(Vec::new()));
    app.storage(StorageEvent::Sticks(vec![stick_with(&[])]));
    assert_eq!(app.sheet, Some(Sheet::Lock));
    while app.poll_storage().is_some() {}
    app.press(Action::LockUnsealed);
    assert!(!keeps_a_write(&mut app));
}

/// In a clean process, with a changed setting on the boot stick and a file
/// from the Inbox chosen, the take-away's write sends its own file only.
#[test]
fn the_settings_file_and_the_inbox_never_ride_the_take_aways_write() {
    let mut app = signed_at(1366, 768);
    stick_in(&mut app);
    let name = file(&app);
    testkit::made_clean(&mut app);
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        boot: true,
        ..stick_with(&[])
    }]));
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    testkit::inbox_file(&mut app, "wallet.txt", kit.descriptor.into_bytes());
    app.press(Action::TextSize(TextSize::Large));
    app.press(Action::VisitInbox(0));
    let _ = app.frame();
    assert!(app.visit_settings_on(), "the settings row is ticked");
    assert!(app.visit().from_inbox.contains("wallet.txt"), "chosen");
    app.press(Action::TakeWrite);
    assert_eq!(writes(&mut app), vec![name]);
}

/// A second press on the sheet's button, which the last frame still shows,
/// sends nothing more.
#[test]
fn a_second_press_on_the_taken_sheet_sends_nothing() {
    for again in [Action::VisitKeepBoth, Action::VisitReplace] {
        let mut app = signed_at(1366, 768);
        stick_in(&mut app);
        let name = file(&app);
        testkit::outbox_file(&mut app, "other.txt", b"other");
        app.press(Action::VisitOut(1));
        testkit::made_clean(&mut app);
        app.storage(StorageEvent::Sticks(vec![stick_with(&[&name])]));
        app.press(Action::TakeWrite);
        assert_eq!(app.sheet, Some(Sheet::Taken));
        app.press(again);
        assert_eq!(writes(&mut app), vec![name]);
        app.press(again);
        assert!(writes(&mut app).is_empty());
    }
}

/// Files' Rename can put another file under the message file's name: that
/// is not the take-away's file. Its row makes the file, and Write sends
/// nothing.
#[test]
fn another_file_under_the_name_is_not_the_take_aways() {
    let mut app = signed_at(1366, 768);
    stick_in(&mut app);
    let name = file(&app);
    testkit::outbox_file(&mut app, "other.txt", b"other");
    app.press(Action::Nav(Screen::Files));
    app.outbox[0].name = faraday_text::Text::of("x.txt");
    app.outbox[1].name = faraday_text::Text::of(&name);
    app.press(Action::Back);
    app.press(Action::MStep(faraday_core::mstep::SIGN));
    testkit::made_clean(&mut app);
    // Ticked in Files, as the file now under the name would be.
    if !app.visit().out.contains(&*name) {
        app.press(Action::VisitOut(1));
    }
    let _ = app.frame();
    assert!(app.visit().out.contains(&*name), "its name is still ticked");
    assert!(app.offers(Action::MOut), "Make the file");
    assert!(!app.offers(Action::VisitOut(1)), "no tick row for it");
    app.press(Action::TakeWrite);
    assert!(writes(&mut app).is_empty());
    // Made again, it goes beside the other file and is the one sent.
    app.press(Action::MOut);
    app.press(Action::TakeWrite);
    let sent = writes(&mut app);
    assert_eq!(sent.len(), 1);
    assert_ne!(sent[0], name);
}

/// With the stick gone before the press acts, nothing waits through the
/// lock: not even for the stick an imported PSBT came from.
#[test]
fn a_press_with_no_stick_keeps_no_write() {
    let mut app = signed_at(1366, 768);
    stick_in(&mut app);
    testkit::imported_from(&mut app, "another");
    app.storage(StorageEvent::Sticks(Vec::new()));
    app.press(Action::TakeWrite);
    assert_eq!(app.sheet, Some(Sheet::WriteOut));
    while app.poll_storage().is_some() {}
    app.press(Action::Lock);
    assert!(!keeps_a_write(&mut app));
}

/// A message's write after the lock does not end on a spend's end card.
#[test]
fn a_messages_write_after_the_lock_is_not_a_spends_end() {
    let mut app = signed_at(1366, 768);
    stick_in(&mut app);
    app.press(Action::TakeWrite);
    while app.poll_storage().is_some() {}
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
    next.storage(StorageEvent::Sticks(vec![stick_with(&[])]));
    assert_eq!(writes(&mut next).len(), 1, "the write went");
    assert!(!testkit::signed_end(&next));
}

/// A press from a flow whose rows changed sends nothing of what it offered
/// before.
#[test]
fn a_press_after_the_rows_changed_sends_nothing_of_the_earlier_names() {
    let mut app = signed_at(1366, 768);
    stick_in(&mut app);
    testkit::made_clean(&mut app);
    let _ = app.frame();
    assert!(app.offers(Action::TakeWrite));
    // The format is changed: the signature, and so its file's row, go.
    app.press(Action::MFormat(true));
    app.press(Action::TakeWrite);
    assert!(writes(&mut app).is_empty());
}

/// The scope goes with the Already on the stick sheet, and is gone with it:
/// the take-away's Keep both sends its own file only, and Files' Write
/// after a cancelled sheet sends everything chosen.
#[test]
fn the_scope_lives_and_dies_with_the_sheet_it_opened() {
    let mut app = signed_at(1366, 768);
    stick_in(&mut app);
    let name = file(&app);
    testkit::outbox_file(&mut app, "other.txt", b"other");
    app.press(Action::VisitOut(1));
    testkit::made_clean(&mut app);
    app.storage(StorageEvent::Sticks(vec![stick_with(&[&name])]));
    let _ = app.frame();
    app.press(Action::TakeWrite);
    assert_eq!(app.sheet, Some(Sheet::Taken));
    app.press(Action::VisitKeepBoth);
    assert_eq!(writes(&mut app), vec![name.clone()]);
    // Cancelled, then Files' own Write.
    app.press(Action::TakeWrite);
    assert_eq!(app.sheet, Some(Sheet::Taken));
    app.press(Action::Cancel);
    assert!(writes(&mut app).is_empty());
    app.press(Action::VisitWrite);
    app.press(Action::VisitKeepBoth);
    let mut want = vec![name, "other.txt".to_string()];
    want.sort();
    assert_eq!(writes(&mut app), want);
}

/// Rename in Files, by its presses: the file For the stick at `k` takes
/// `stem`, its extension kept.
fn rename_in_files(app: &mut Faraday, k: usize, stem: &str) {
    app.press(Action::File(FilePress::Menu(Row::Out(k))));
    app.press(Action::File(FilePress::Rename(Row::Out(k))));
    assert_eq!(app.sheet, Some(Sheet::Rename));
    let had = app
        .files_menu()
        .renaming
        .as_ref()
        .map_or(0, |r| r.stem.chars().count());
    for _ in 0..had {
        app.event(Event::Key(Key::Backspace));
    }
    for ch in stem.chars() {
        app.event(Event::Key(Key::Char(ch)));
    }
    app.press(Action::File(FilePress::RenameGo));
    assert_eq!(app.sheet, None);
}

/// The stick pulled under the sheet that names the file, and Lock pressed
/// on it. In the next process another file takes the name in Files: the
/// write that waited does not send it.
#[test]
fn another_file_under_the_name_after_the_lock_is_not_written() {
    let mut app = signed_at(1366, 768);
    stick_in(&mut app);
    let name = file(&app);
    testkit::outbox_file(&mut app, "other.txt", b"other");
    app.press(Action::TakeWrite);
    assert_eq!(app.sheet, Some(Sheet::WriteOut));
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert_eq!(app.sheet, Some(Sheet::WriteOut));
    while app.poll_storage().is_some() {}
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
    next.press(Action::Nav(Screen::Files));
    let at = |app: &Faraday, n: &str| app.outbox.iter().position(|i| i.name == n).expect(n);
    let k = at(&next, &name);
    rename_in_files(&mut next, k, "moved");
    let k = at(&next, "other.txt");
    rename_in_files(&mut next, k, name.trim_end_matches(".txt"));
    assert_eq!(&*next.outbox[k].bytes, b"other");
    assert_eq!(next.outbox[k].name, *name);
    while next.poll_storage().is_some() {}
    next.storage(StorageEvent::Sticks(vec![stick_with(&[])]));
    assert!(writes(&mut next).is_empty());
}

/// A write kept with names only, no SHA-256, writes nothing, though the
/// file and the stick are the ones it names.
#[test]
fn a_waiting_write_kept_by_name_only_writes_nothing() {
    let mut app = signed_at(1366, 768);
    stick_in(&mut app);
    let name = file(&app);
    app.press(Action::TakeWrite);
    while app.poll_storage().is_some() {}
    app.press(Action::Lock);
    let mut saved = None;
    while let Some(c) = app.poll_storage() {
        if let faraday_core::StorageCommand::SaveBoxes { outbox, kept, .. } = c {
            saved = Some((outbox, kept));
        }
    }
    let (outbox, mut kept) = saved.expect("the lock kept the boxes");
    let (_, wait) = kept
        .iter_mut()
        .find(|(k, _)| k == "write-waiting")
        .expect("the write waits");
    *wait = format!("id:one\n\n{name}").into_bytes().into();
    let mut next = testkit::started();
    next.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox,
        kept,
    });
    while next.poll_storage().is_some() {}
    next.storage(StorageEvent::Sticks(vec![stick_with(&[])]));
    assert!(writes(&mut next).is_empty());
}

/// The take-away keeps no write for a stick whose identity holds a
/// newline.
#[test]
fn a_stick_identity_with_a_newline_keeps_no_take_away_write() {
    let mut app = signed_at(1366, 768);
    let mut stick = stick_with(&[]);
    stick.id = faraday_text::Text::of("one\ntwo");
    app.storage(StorageEvent::Sticks(vec![stick]));
    app.press(Action::NotNow);
    let _ = app.frame();
    app.press(Action::TakeWrite);
    assert_eq!(app.sheet, Some(Sheet::WriteOut));
    while app.poll_storage().is_some() {}
    app.press(Action::Lock);
    assert!(!keeps_a_write(&mut app));
}
