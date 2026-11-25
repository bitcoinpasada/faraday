//! The stick visit: every file Faraday reads comes in ticked but a
//! seed's, and Unselect all clears them; Select all on the For the stick
//! list chooses every file but an unprotected secret; each long list has
//! a scrollbar that drags it to the end at once, and the wheel moves the
//! list it is over and not the other. The chip says where pulling the
//! stick goes: Unlock when a vault was sealed for the visit, else Home.

use faraday_core::testkit::Plain;
use faraday_core::wallet::FileKind;
use faraday_core::{
    Action, Column, Faraday, Item, Screen, StickInfo, StorageCommand, StorageEvent,
};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};

/// A stick with `n` PSBTs, a picture and a file Faraday does not read.
fn with_stick(n: usize) -> Faraday {
    with_stick_at(n, (1366, 768, 160))
}

/// [`with_stick`] on a display `width` x `height` at `dpi`.
fn with_stick_at(n: usize, (width, height, dpi): (u16, u16, u16)) -> Faraday {
    let mut app = faraday_core::testkit::new_on_test_net();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    let mut files: Vec<(String, u64)> = (0..n)
        .map(|i| (format!("spend-{i:02}.psbt"), 300))
        .collect();
    files.push(("photo.png".to_string(), 9000));
    files.push(("notes.docx".to_string(), 9000));
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("S"),
        label: faraday_text::Text::of("TESTSTICK"),
        boot: false,
        files: files
            .iter()
            .map(|(n, s)| (faraday_text::Text::of(n), *s))
            .collect(),
        vaults: Vec::new(),
    }]));
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    app
}

#[test]
fn every_file_comes_in_ticked_and_unselect_all_clears_them() {
    let mut app = with_stick(5);
    assert_eq!(
        app.visit().inn.len(),
        7,
        "the PSBTs, the picture and a file of a kind Faraday does not know"
    );
    assert!(app.visit().inn.contains("photo.png"));
    assert!(app.visit().inn.contains("notes.docx"));
    assert!(app.offers(Action::VisitInAll));
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Unselect all"), "{texts:?}");
    // Select all stays on the For the stick list only.
    assert_eq!(
        texts.iter().filter(|t| *t == "Select all").count(),
        1,
        "{texts:?}"
    );
    app.press(Action::VisitInAll);
    assert!(app.visit().inn.is_empty());
}

#[test]
fn a_fresh_visit_has_the_psbts_and_descriptors_ticked_and_the_words_file_not() {
    let mut app = faraday_core::testkit::new_on_test_net();
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
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("S"),
        label: faraday_text::Text::of("TESTSTICK"),
        boot: false,
        files: vec![
            (faraday_text::Text::of("savings-unsigned.psbt"), 900),
            (faraday_text::Text::of("savings-descriptor.txt"), 300),
            (faraday_text::Text::of("savings-9a6a2580-words.txt"), 120),
            (faraday_text::Text::of("savings-9a6a2580-seedqr.png"), 4000),
            (faraday_text::Text::of("film.mp4"), 19 * 1024 * 1024),
        ],
        vaults: Vec::new(),
    }]));
    assert_eq!(app.screen, Screen::Files);
    let ticked: Vec<&str> = app.visit().inn.iter().map(|t| t.as_str()).collect();
    assert_eq!(ticked, ["savings-descriptor.txt", "savings-unsigned.psbt"]);
}

#[test]
fn dragging_the_scrollbar_to_the_bottom_shows_the_end_of_a_long_list() {
    let mut app = with_stick(60);
    let (x, y) = app
        .where_offered(Action::VisitBar(Column::Stick))
        .expect("a long list has no scrollbar");
    let (top, h, _, max) = app.visit().bar(Column::Stick).get().unwrap();
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    app.event(Event::Touch {
        x,
        y: (top + h + 40) as u16,
        phase: TouchPhase::Move,
    });
    assert_eq!(app.visit_in_offset(), max, "the list is not at its end");
    app.event(Event::Touch {
        x,
        y: top as u16,
        phase: TouchPhase::Move,
    });
    assert_eq!(
        app.visit_in_offset(),
        0.0,
        "dragging back up does not reach the top"
    );
    app.event(Event::Touch {
        x,
        y: top as u16,
        phase: TouchPhase::Up,
    });
}

#[test]
fn a_short_list_has_no_scrollbar() {
    let app = with_stick(3);
    assert!(!app.offers(Action::VisitBar(Column::Stick)));
    assert!(!app.offers(Action::VisitBar(Column::Outbox)));
}

/// A wallet backup's worth of Outbox files, `n` of them, and an
/// unprotected secret when `secret`; the visit opened again so it ticks
/// them as it does on arrival.
fn with_outbox(mut app: Faraday, n: usize, secret: bool) -> Faraday {
    for i in 0..n {
        app.outbox.push(Item {
            name: faraday_text::Text::of(&format!("backup-{i:02}.pdf")),
            bytes: (vec![b'%'; 2000]).into(),
            kind: FileKind::Pdf,
            secret: false,
            picture: None,
            sha: None,
            facts: None,
            wallet: None,
            psbt: None,
            outpoints: Vec::new(),
            unsigned_txid: None,
        });
    }
    if secret {
        app.outbox.push(Item {
            name: faraday_text::Text::of("seed-words.txt"),
            bytes: (b"words".to_vec()).into(),
            kind: FileKind::Text,
            secret: true,
            picture: None,
            sha: None,
            facts: None,
            wallet: None,
            psbt: None,
            outpoints: Vec::new(),
            unsigned_txid: None,
        });
    }
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    app
}

/// Turns the wheel at (x, y) by `dy` pixels and lets the glide land.
fn wheel(app: &mut Faraday, (x, y): (u16, u16), dy: i16) {
    app.event(Event::Wheel { x, y, dy });
    app.settle();
    let _ = app.frame();
}

fn tap(app: &mut Faraday, (x, y): (u16, u16)) {
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    let _ = app.frame();
}

/// The names a write sends to the stick, settings left out.
fn written(app: &mut Faraday) -> Vec<String> {
    let mut names = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Write { name, .. } = c
            && name.starts_with("backup-")
        {
            names.push(String::from(&*name));
        }
    }
    names
}

#[test]
fn the_wheel_over_the_outbox_brings_its_last_file_into_reach_and_leaves_the_stick_list() {
    let mut app = with_outbox(with_stick(30), 20, false);
    assert!(
        !app.offers(Action::VisitOut(19)),
        "the last of 20 Outbox files is on screen without scrolling"
    );
    let over = app.where_offered(Action::VisitOut(0)).unwrap();
    wheel(&mut app, over, 3000);
    assert_eq!(app.visit_in_offset(), 0.0, "the stick's file list moved");
    let last = app
        .where_offered(Action::VisitOut(19))
        .expect("the last Outbox file cannot be scrolled to");
    tap(&mut app, last);
    assert!(
        !app.visit().out.contains("backup-19.pdf"),
        "it did not untick"
    );
    app.press(Action::VisitWrite);
    let names = written(&mut app);
    assert_eq!(names.len(), 19, "{names:?}");
    assert!(!names.contains(&"backup-19.pdf".to_string()));
}

#[test]
fn the_wheel_over_the_stick_files_moves_them_and_not_the_outbox() {
    let mut app = with_outbox(with_stick(30), 20, false);
    let over = app.where_offered(Action::VisitIn(0)).unwrap();
    wheel(&mut app, over, 3000);
    assert!(
        app.visit_in_offset() > 0.0,
        "the stick's file list did not move"
    );
    assert_eq!(app.visit().out_offset, 0.0, "the Outbox list moved");
    assert!(app.offers(Action::VisitIn(29)));
    assert!(app.offers(Action::VisitOut(0)));
}

#[test]
fn dragging_the_outbox_scrollbar_to_the_bottom_shows_its_last_file() {
    let mut app = with_outbox(with_stick(3), 20, false);
    let (x, _) = app
        .where_offered(Action::VisitBar(Column::Outbox))
        .expect("20 Outbox files have no scrollbar");
    let (top, h, _, max) = app.visit().bar(Column::Outbox).get().unwrap();
    app.event(Event::Touch {
        x,
        y: top as u16,
        phase: TouchPhase::Down,
    });
    app.event(Event::Touch {
        x,
        y: (top + h + 40) as u16,
        phase: TouchPhase::Move,
    });
    app.event(Event::Touch {
        x,
        y: (top + h + 40) as u16,
        phase: TouchPhase::Up,
    });
    assert_eq!(app.visit().out_offset, max);
    let _ = app.frame();
    assert!(app.offers(Action::VisitOut(19)));
}

#[test]
fn select_all_on_the_outbox_leaves_an_unprotected_secret_alone() {
    for display in [(1366, 768, 160), (480, 640, 286)] {
        let mut app = with_outbox(with_stick_at(3, display), 20, true);
        // A small window's In Faraday tab holds the Outbox.
        app.press(Action::VisitTab(Column::Outbox));
        // The Pi's page scrolls as one: down to the Outbox.
        for _ in 0..20 {
            if app.offers(Action::VisitOutAll) {
                break;
            }
            let (w, h) = {
                let f = app.frame();
                (f.width, f.height)
            };
            wheel(&mut app, (w / 2, h / 2), 200);
        }
        assert!(app.offers(Action::VisitOutAll), "{display:?}");
        assert!(!app.visit().out.contains("seed-words.txt"));
        // With no setting changed there is no settings row: every file is
        // ticked, so the first press unticks them all.
        app.press(Action::VisitOutAll);
        assert!(app.visit().out.is_empty(), "{display:?}: not all unticked");
        app.press(Action::VisitWrite);
        assert!(written(&mut app).is_empty());
        app.press(Action::VisitOutAll);
        assert_eq!(app.visit().out.len(), 20, "{display:?}");
        assert!(!app.visit().out.contains("seed-words.txt"));
        // With the secret ticked by hand as well, every row is ticked
        // and a press unticks them all.
        let secret = app.outbox.len() - 1;
        app.press(Action::VisitOut(secret));
        app.press(Action::VisitOutAll);
        assert!(app.visit().out.is_empty());
    }
}

#[test]
fn a_finger_dragged_on_the_outbox_moves_it_and_not_the_stick_list() {
    let mut app = with_outbox(with_stick(30), 20, false);
    let (x, y) = app.where_offered(Action::VisitOut(3)).unwrap();
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    for k in 1..=10u16 {
        app.event(Event::Touch {
            x,
            y: y - k * 20,
            phase: TouchPhase::Move,
        });
    }
    app.event(Event::Touch {
        x,
        y: y - 200,
        phase: TouchPhase::Up,
    });
    assert!(app.visit().out_offset > 0.0, "the Outbox did not move");
    assert_eq!(app.visit_in_offset(), 0.0, "the stick's file list moved");
    assert!(
        app.visit().out.contains("backup-03.pdf"),
        "the drag ticked or unticked the file it started on"
    );
}

type Files = Vec<(String, Vec<u8>)>;

/// A process restored from `boxes` (Inbox, Outbox, kept), with the clock
/// known and `stick` attached when given.
fn process(boxes: (Files, Files, Files), stick: Option<&str>) -> Faraday {
    let mut app = faraday_core::testkit::new_on_test_net();
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
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(boxes.0),
        outbox: faraday_core::files_from(boxes.1),
        kept: faraday_core::kept_from(boxes.2),
    });
    app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    if let Some(label) = stick {
        app.storage(StorageEvent::Sticks(vec![StickInfo {
            make: Default::default(),
            serial: None,
            id: faraday_text::Text::of("S"),
            label: faraday_text::Text::of(label),
            boot: false,
            files: Vec::new(),
            vaults: Vec::new(),
        }]));
    }
    let _ = app.frame();
    app
}

/// What the last save kept: the Inbox, the Outbox and the kept state.
fn saved(app: &mut Faraday) -> (Files, Files, Files) {
    let mut last = (Vec::new(), Vec::new(), Vec::new());
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes {
            inbox,
            outbox,
            kept,
        } = c
        {
            last = (
                faraday_core::testkit::plain_files(inbox),
                faraday_core::testkit::plain_files(outbox),
                faraday_core::testkit::plain_kept(kept),
            );
        }
    }
    last
}

fn files_texts(app: &mut Faraday) -> Vec<String> {
    app.press(Action::Nav(Screen::Files));
    app.drawn_texts().plain()
}

/// A write leaves a receipt: Files lists it
/// under For the stick with the file names, the files written leave the
/// list that waits, and it is still there after the lock that follows,
/// in the next process, where Home leads with it. A fresh process, after
/// power-off, has none.
#[test]
fn a_write_leaves_a_receipt_that_a_lock_keeps_and_power_off_does_not() {
    let wallet = faraday_core::testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "spending-wallet.txt")
        .unwrap();
    let mut app = process((Vec::new(), vec![wallet], Vec::new()), Some("TESTSTICK"));
    // The person has checked the clock: the receipt carries its time.
    app.press(Action::OpenSetTime);
    app.press(Action::SetTimeKeep);
    app.press(Action::Nav(Screen::Files));
    app.press(Action::VisitWrite);
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Write { stick, name, .. } = c {
            app.storage(StorageEvent::Written {
                stick,
                wrote_as: name.clone(),
                name,
            });
        }
    }
    assert!(
        !app.outbox.iter().any(|i| i.name == "spending-wallet.txt"),
        "a file written still waits"
    );
    let texts = files_texts(&mut app);
    assert!(
        texts.iter().any(|t| t == "Written to TESTSTICK at 04:00"),
        "no receipt on Files: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t == "spending-wallet.txt"),
        "the receipt names the file: {texts:?}"
    );
    assert!(texts.iter().any(|t| t == "verified"), "{texts:?}");

    // Pulled, then locked: the next process still lists it, and Home
    // leads with it.
    app.storage(StorageEvent::Sticks(Vec::new()));
    app.press(Action::Lock);
    let (inbox, outbox, kept) = saved(&mut app);
    let mut next = process((inbox.clone(), outbox.clone(), kept), None);
    let texts = files_texts(&mut next);
    assert!(
        texts.iter().any(|t| t == "Written to TESTSTICK at 04:00"),
        "the lock lost the receipt: {texts:?}"
    );
    next.press(Action::Nav(Screen::Home));
    let texts = next.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Written to TESTSTICK"),
        "Home does not lead with the receipt: {texts:?}"
    );

    // Power-off: the files may come back; the kept state does not.
    let mut fresh = process((inbox, outbox, Vec::new()), None);
    let texts = files_texts(&mut fresh);
    assert!(
        !texts.iter().any(|t| t.starts_with("Written to")),
        "a fresh process has a receipt: {texts:?}"
    );
}

/// A stick attached to a process restored with `outbox` For the stick.
fn visiting(outbox: Files) -> Faraday {
    let mut app = process((Vec::new(), outbox, Vec::new()), Some("TESTSTICK"));
    assert_eq!(app.screen, Screen::Files);
    let _ = app.frame();
    app
}

#[test]
fn files_says_no_then_chip_and_pulling_stays_on_files() {
    let wallet = faraday_core::testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "spending-wallet.txt")
        .unwrap();
    let mut app = visiting(vec![wallet]);
    let texts = app.drawn_texts();
    assert!(!texts.iter().any(|t| t.starts_with("Then:")), "{texts:?}");
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert_eq!(app.screen, Screen::Files, "nothing sends on: Files stays");
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t.starts_with("Waiting for")),
        "only On the stick goes back to its empty state: {texts:?}"
    );
}

#[test]
fn a_visit_for_a_vault_sealed_at_lock_goes_to_unlock_with_no_chip() {
    let vault = (
        "vault.kdbx".to_string(),
        faraday_core::testkit::test_vault().unwrap(),
    );
    let mut app = visiting(vec![vault]);
    let texts = app.drawn_texts();
    assert!(!texts.iter().any(|t| t.starts_with("Then:")), "{texts:?}");
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert_eq!(app.screen, Screen::Unlock);
}

#[test]
fn compact_files_has_both_tabs_with_and_without_a_stick() {
    let mut app = with_stick_at(3, (480, 640, 286));
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "In Faraday"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "On TESTSTICK"), "{texts:?}");
    assert!(!texts.iter().any(|t| t.starts_with("Then:")), "{texts:?}");
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert_eq!(app.screen, Screen::Files);
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "In Faraday"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "On the stick"), "{texts:?}");
}

#[test]
fn compact_on_the_stick_tab_with_no_stick_says_waiting() {
    let mut app = with_stick_at(3, (480, 640, 286));
    app.storage(StorageEvent::Sticks(Vec::new()));
    app.press(Action::VisitTab(Column::Stick));
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t.starts_with("Waiting for")),
        "{texts:?}"
    );
    assert!(!app.offers(Action::DeleteFiles), "no stick buttons");
}

#[test]
fn a_stick_pulled_in_the_middle_of_a_flow_returns_to_it_with_no_chip() {
    let mut app = faraday_core::testkit::new_on_test_net();
    app.press(Action::Nav(Screen::Restore));
    assert_eq!(app.screen, Screen::Restore);
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("S"),
        label: faraday_text::Text::of("TESTSTICK"),
        boot: false,
        files: Vec::new(),
        vaults: Vec::new(),
    }]));
    assert_eq!(app.screen, Screen::Files);
    let texts = app.drawn_texts();
    assert!(!texts.iter().any(|t| t.starts_with("Then:")), "{texts:?}");
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert_eq!(app.screen, Screen::Restore);
}

/// What the app asked the storage to do since last asked.
fn asked(app: &mut Faraday) -> Vec<StorageCommand> {
    std::iter::from_fn(|| app.poll_storage()).collect()
}

fn tick(app: &mut Faraday, now_ms: u64) {
    app.event(Event::Tick { now_ms });
}

/// The Delete sheet's hold: down at `t0`, held past its time, let go.
fn hold_go(app: &mut Faraday, t0: u64) {
    let _ = app.frame();
    let hold = Action::Vault(faraday_core::vault::VaultAction::HoldWipe);
    let (x, y) = app.where_offered(hold).expect("the hold is on the sheet");
    tick(app, t0);
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    tick(app, t0 + faraday_core::vault::HOLD_MS + 100);
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
}

/// Zeroing the free space or the whole partition takes a hold, as
/// deleting a vault does: a press of the sheet's button sends nothing,
/// a hold let go too soon sends nothing, and the hold to its end starts
/// zeroing.
#[test]
fn zeroing_takes_a_hold() {
    use faraday_core::Sheet;
    for (what, label, first) in [
        (
            1,
            "Hold to zero the free space",
            StorageCommand::ZeroFree {
                stick: faraday_text::Text::of("S"),
                from: 2,
            },
        ),
        (
            2,
            "Hold to zero the whole partition",
            StorageCommand::MakeEmpty {
                stick: faraday_text::Text::of("S"),
            },
        ),
    ] {
        // Tall enough for the whole sheet, the hold at its foot.
        let mut app = with_stick_at(1, (1366, 1400, 160));
        app.press(Action::DeleteFiles);
        app.press(Action::DeleteWhat(what));
        asked(&mut app);
        let _ = app.frame();
        assert!(!app.offers(Action::DeleteGo), "no plain button");
        let texts = app.drawn_texts();
        assert!(texts.iter().any(|t| t == label), "{texts:?}");
        app.press(Action::DeleteGo);
        assert!(asked(&mut app).is_empty(), "a press alone zeroes nothing");
        assert_eq!(app.sheet, Some(Sheet::Delete));
        // Let go too soon.
        let hold = Action::Vault(faraday_core::vault::VaultAction::HoldWipe);
        let (x, y) = app.where_offered(hold).unwrap();
        tick(&mut app, 1_000);
        app.event(Event::Touch {
            x,
            y,
            phase: TouchPhase::Down,
        });
        tick(&mut app, 1_300);
        app.event(Event::Touch {
            x,
            y,
            phase: TouchPhase::Up,
        });
        tick(&mut app, 3_000);
        assert!(
            asked(&mut app).is_empty(),
            "a hold let go too soon zeroes nothing"
        );
        hold_go(&mut app, 5_000);
        assert_eq!(app.sheet, Some(Sheet::Zeroing));
        assert_eq!(asked(&mut app), vec![first]);
    }
}

/// Delete files is the one way in: a sheet that says it cannot be undone
/// and that only the files partition is written. Chosen files are deleted
/// only on its button, each with zeros, and the answer is said on the
/// visit; Cancel sends nothing.
#[test]
fn chosen_files_are_deleted_only_from_the_sheet() {
    use faraday_core::Sheet;
    let mut app = with_stick(2);
    asked(&mut app);
    assert!(app.offers(Action::DeleteFiles));
    app.press(Action::DeleteFiles);
    assert_eq!(app.sheet, Some(Sheet::Delete));
    assert_eq!(
        asked(&mut app),
        vec![StorageCommand::StickSpace {
            stick: faraday_text::Text::of("S")
        }]
    );
    let _ = app.frame();
    let text = app.drawn_texts().join(" ");
    assert!(text.contains("Delete files from TESTSTICK"), "{text}");
    assert!(text.contains("cannot be undone"), "{text}");
    assert!(text.contains("files partition"), "{text}");
    assert!(!app.offers(Action::DeleteGo), "nothing chosen yet");
    app.press(Action::Cancel);
    assert!(asked(&mut app).is_empty(), "Cancel deletes nothing");

    app.press(Action::DeleteFiles);
    asked(&mut app);
    app.press(Action::DeletePick(1));
    let _ = app.frame();
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "Delete 1 file with zeros")
    );
    app.press(Action::DeleteGo);
    assert_eq!(app.sheet, None);
    assert_eq!(
        asked(&mut app),
        vec![StorageCommand::Wipe {
            stick: faraday_text::Text::of("S"),
            name: faraday_text::Text::of("spend-01.psbt")
        }]
    );
    app.storage(StorageEvent::Wiped {
        stick: faraday_text::Text::of("S"),
        name: faraday_text::Text::of("spend-01.psbt"),
    });
    let _ = app.frame();
    assert!(!app.visit().inn.contains("spend-01.psbt"));
    let text = app.drawn_texts().join("\n");
    assert!(text.contains("spend-01.psbt deleted"), "{text}");
}

/// Files ticked to delete on one stick are deleted from that stick only:
/// another stick with files of the same names listed before it, a place
/// on the sheet is a place on the first stick, and nothing ticked on the
/// second one is unticked by the first one's delete.
#[test]
fn files_ticked_for_deletion_on_a_pulled_stick_touch_nothing_on_another() {
    let mut app = with_stick(2);
    asked(&mut app);
    app.press(Action::DeleteFiles);
    asked(&mut app);
    app.press(Action::DeletePick(1));
    let other = |files: &[&str]| StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("T"),
        label: faraday_text::Text::of("OTHERSTICK"),
        boot: false,
        files: files
            .iter()
            .map(|n| (faraday_text::Text::of(n), 300))
            .collect(),
        vaults: Vec::new(),
    };
    let first = app.sticks()[0].clone();
    app.storage(StorageEvent::Sticks(vec![
        other(&["notes.docx", "spend-01.psbt", "spend-00.psbt"]),
        first,
    ]));
    asked(&mut app);
    assert_eq!(app.sheet, Some(faraday_core::Sheet::Delete));
    // The sheet still lists the first stick's files, so its first place
    // is spend-00.psbt on the first stick, not notes.docx on the second.
    app.press(Action::DeletePick(0));
    app.press(Action::DeleteGo);
    assert_eq!(
        asked(&mut app),
        vec![
            StorageCommand::Wipe {
                stick: faraday_text::Text::of("S"),
                name: faraday_text::Text::of("spend-01.psbt")
            },
            StorageCommand::Wipe {
                stick: faraday_text::Text::of("S"),
                name: faraday_text::Text::of("spend-00.psbt")
            }
        ]
    );
    app.storage(StorageEvent::Wiped {
        stick: faraday_text::Text::of("S"),
        name: faraday_text::Text::of("spend-01.psbt"),
    });
    assert!(
        app.visit().inn.contains("spend-01.psbt"),
        "the second stick's file stays ticked"
    );
}

#[test]
fn a_delete_that_left_the_bytes_says_only_the_name_went() {
    let mut app = with_stick(2);
    asked(&mut app);
    app.press(Action::DeleteFiles);
    asked(&mut app);
    app.press(Action::DeletePick(1));
    app.press(Action::DeleteGo);
    asked(&mut app);
    app.storage(StorageEvent::WipedName {
        stick: faraday_text::Text::of("S"),
        name: faraday_text::Text::of("spend-01.psbt"),
    });
    let _ = app.frame();
    assert!(!app.visit().inn.contains("spend-01.psbt"));
    let text = app.drawn_texts().join("\n");
    assert!(
        text.contains("spend-01.psbt removed by name, its bytes left"),
        "{text}"
    );
    assert!(!text.contains("zeros over it first"), "{text}");
}

/// A file's size reads the same on the visit and on the Delete sheet,
/// in decimal units.
#[test]
fn a_files_size_reads_the_same_on_the_visit_and_the_delete_sheet() {
    let mut app = with_stick(1);
    asked(&mut app);
    let size = |app: &mut Faraday| -> Vec<String> {
        let _ = app.frame();
        app.drawn_texts()
            .plain()
            .into_iter()
            .filter(|t| t.contains("KB") || t.contains("KiB"))
            .collect()
    };
    let visit = size(&mut app);
    assert!(visit.iter().any(|t| t.contains("9.0\u{a0}KB")), "{visit:?}");
    app.press(Action::DeleteFiles);
    asked(&mut app);
    let sheet = size(&mut app);
    assert!(sheet.iter().any(|t| t == "9.0\u{a0}KB"), "{sheet:?}");
    assert!(sheet.iter().all(|t| !t.contains("8\u{a0}KB")), "{sheet:?}");
}

/// The free space, chosen on the same sheet: each step is asked for on
/// the tick after the last one's answer, so the screen draws between
/// them, and the end is said.
#[test]
fn the_free_space_is_zeroed_a_step_a_tick() {
    use faraday_core::Sheet;
    let mut app = with_stick(1);
    app.press(Action::DeleteFiles);
    app.press(Action::DeleteWhat(1));
    app.storage(StorageEvent::StickSpace {
        stick: faraday_text::Text::of("S"),
        free: 3 << 30,
        size: 7 << 30,
    });
    let _ = app.frame();
    let text = app.drawn_texts().join(" ");
    // 3 GiB, in the decimal units every size is written in.
    assert!(text.contains("3.2\u{a0}GB to write"), "{text}");
    assert!(text.contains("partition table"), "{text}");
    asked(&mut app);
    hold_go(&mut app, 1_000);
    assert_eq!(app.sheet, Some(Sheet::Zeroing));
    assert_eq!(
        asked(&mut app),
        vec![StorageCommand::ZeroFree {
            stick: faraday_text::Text::of("S"),
            from: 2
        }]
    );
    app.storage(StorageEvent::Zeroing {
        stick: faraday_text::Text::of("S"),
        next: 400,
        total: 1000,
        done: false,
    });
    assert!(asked(&mut app).is_empty(), "nothing before the next tick");
    tick(&mut app, 5_000);
    assert_eq!(
        asked(&mut app),
        vec![StorageCommand::ZeroFree {
            stick: faraday_text::Text::of("S"),
            from: 400
        }]
    );
    app.storage(StorageEvent::Zeroing {
        stick: faraday_text::Text::of("S"),
        next: 1002,
        total: 1000,
        done: true,
    });
    assert_eq!(app.sheet, None);
    assert!(app.zeroing().is_none());
    let _ = app.frame();
    let text = app.drawn_texts().join(" ");
    assert!(text.contains("Free space on TESTSTICK zeroed"), "{text}");
}

/// The whole partition, two passes: made empty first, then every cluster
/// zeroed twice.
#[test]
fn the_whole_partition_is_made_empty_then_zeroed_for_each_pass() {
    let mut app = with_stick_at(1, (1366, 1400, 160));
    app.press(Action::DeleteFiles);
    app.press(Action::DeleteWhat(2));
    app.press(Action::DeletePasses(2));
    asked(&mut app);
    hold_go(&mut app, 100);
    assert_eq!(
        asked(&mut app),
        vec![StorageCommand::MakeEmpty {
            stick: faraday_text::Text::of("S")
        }]
    );
    app.storage(StorageEvent::Zeroing {
        stick: faraday_text::Text::of("S"),
        next: 2,
        total: 0,
        done: false,
    });
    for pass in 0..2 {
        tick(&mut app, 1_000 + pass);
        assert_eq!(
            asked(&mut app),
            vec![StorageCommand::ZeroFree {
                stick: faraday_text::Text::of("S"),
                from: 2
            }],
            "pass {}",
            pass + 1
        );
        app.storage(StorageEvent::Zeroing {
            stick: faraday_text::Text::of("S"),
            next: 1002,
            total: 1000,
            done: true,
        });
    }
    assert!(app.zeroing().is_none());
    let _ = app.frame();
    let text = app.drawn_texts().join(" ");
    assert!(
        text.contains("TESTSTICK's partition zeroed, 2 passes"),
        "{text}"
    );
}

/// Stop ends it after the step under way; a stick pulled part way ends
/// it at once.
#[test]
fn zeroing_stops_on_stop_and_on_a_pull() {
    let mut app = with_stick(1);
    app.press(Action::DeleteFiles);
    app.press(Action::DeleteWhat(1));
    hold_go(&mut app, 100);
    asked(&mut app);
    app.press(Action::ZeroStop);
    app.storage(StorageEvent::Zeroing {
        stick: faraday_text::Text::of("S"),
        next: 502,
        total: 1000,
        done: false,
    });
    assert!(app.zeroing().is_none());
    tick(&mut app, 9_000);
    assert!(asked(&mut app).is_empty(), "no step after Stop");
    let _ = app.frame();
    assert!(app.drawn_texts().join(" ").contains("Stopped at 50%"));

    app.press(Action::DeleteFiles);
    app.press(Action::DeleteWhat(1));
    hold_go(&mut app, 20_000);
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert!(app.zeroing().is_none());
    assert_eq!(app.sheet, None);
}

/// A stick holding `on_stick`, the Outbox holding `outbox`, every one of
/// them chosen on the visit.
fn writing_over(on_stick: &[(&str, u64)], outbox: Vec<(&str, Vec<u8>)>) -> Faraday {
    let mut app = faraday_core::testkit::new_on_test_net();
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
    for (name, bytes) in outbox {
        app.outbox.push(Item {
            name: faraday_text::Text::of(name),
            bytes: bytes.into(),
            kind: FileKind::Pdf,
            secret: false,
            picture: None,
            sha: None,
            facts: None,
            wallet: None,
            psbt: None,
            outpoints: Vec::new(),
            unsigned_txid: None,
        });
    }
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("S"),
        label: faraday_text::Text::of("TESTSTICK"),
        boot: false,
        files: on_stick
            .iter()
            .map(|(n, s)| (faraday_text::Text::of(n), *s))
            .collect(),
        vaults: Vec::new(),
    }]));
    app.press(Action::Nav(Screen::Files));
    let names: Vec<String> = app.outbox.iter().map(|i| String::from(&*i.name)).collect();
    for (i, n) in names.iter().enumerate() {
        if !app.visit().out.contains(n.as_str()) {
            app.press(Action::VisitOut(i));
        }
    }
    let _ = app.frame();
    app
}

/// The files a write sends, settings left out: each name and whether it
/// goes over the old file.
fn writes(app: &mut Faraday) -> Vec<(String, bool)> {
    asked(app)
        .into_iter()
        .filter_map(|c| match c {
            StorageCommand::Write { name, replace, .. }
                if name != faraday_core::stick::settings::FILE =>
            {
                Some((String::from(&*name), replace))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn a_taken_name_asks_first_and_replace_writes_over_it() {
    let mut app = writing_over(
        &[("Backup-00.pdf", 10)],
        vec![
            ("backup-00.pdf", b"new".to_vec()),
            ("backup-01.pdf", b"new".to_vec()),
        ],
    );
    asked(&mut app);
    app.press(Action::VisitWrite);
    assert!(
        writes(&mut app).is_empty(),
        "nothing written before a choice"
    );
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Already on the stick"),
        "{texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|t| t == "backup-00.pdf · Keep both: backup-00-2.pdf"),
        "the row says the name Keep both gives: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t.starts_with("backup-01.pdf ·")),
        "a name not taken is not listed"
    );
    app.press(Action::VisitReplace);
    assert_eq!(
        writes(&mut app),
        [
            ("backup-00.pdf".to_string(), true),
            ("backup-01.pdf".to_string(), false)
        ]
    );
}

#[test]
fn keep_both_writes_beside_and_cancel_writes_nothing() {
    let outbox = || vec![("backup-00.pdf", b"new".to_vec())];
    let mut app = writing_over(&[("backup-00.pdf", 10)], outbox());
    asked(&mut app);
    app.press(Action::VisitWrite);
    app.press(Action::VisitKeepBoth);
    assert_eq!(writes(&mut app), [("backup-00.pdf".to_string(), false)]);

    let mut app = writing_over(&[("backup-00.pdf", 10)], outbox());
    asked(&mut app);
    app.press(Action::VisitWrite);
    app.press(Action::Cancel);
    assert!(writes(&mut app).is_empty(), "Cancel writes nothing");
    assert_eq!(app.screen, Screen::Files, "the visit stays open");
    // Nothing taken: no sheet, written at once.
    let mut app = writing_over(&[], outbox());
    asked(&mut app);
    app.press(Action::VisitWrite);
    assert_eq!(writes(&mut app), [("backup-00.pdf".to_string(), false)]);
}

/// The vault writes a visit sends, settings left out: each name and what
/// it does where a file has its name.
fn vault_writes(app: &mut Faraday) -> Vec<(String, bool, faraday_core::OnTaken)> {
    asked(app)
        .into_iter()
        .filter_map(|c| match c {
            StorageCommand::Write {
                name,
                replace,
                on_taken,
                ..
            } if name != faraday_core::stick::settings::FILE => {
                Some((String::from(&*name), replace, on_taken))
            }
            _ => None,
        })
        .collect()
}

/// A hold on the action's button: down at `t0`, held past its time, up.
fn hold_action(app: &mut Faraday, action: Action, t0: u64, hold_ms: u64) {
    let _ = app.frame();
    let (x, y) = app.where_offered(action).expect("the hold is on the sheet");
    tick(app, t0);
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    tick(app, t0 + hold_ms);
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
}

/// A visit writing `vault` as `vault.kdbx` on a stick that lists
/// `Vault.kdbx`, the disk process having refused the name as taken for a
/// file whose SHA-256 is `sha`.
fn name_taken_visit(sha: Option<[u8; 32]>) -> Faraday {
    let vault = faraday_core::testkit::test_vault().unwrap();
    let mut app = writing_over(&[("Vault.kdbx", 99)], vec![("vault.kdbx", vault)]);
    asked(&mut app);
    app.press(Action::VisitWrite);
    assert_eq!(
        vault_writes(&mut app),
        [("vault.kdbx".to_string(), true, faraday_core::OnTaken::Ask)],
        "a vault is sent asking"
    );
    app.storage(StorageEvent::NameTaken {
        stick: faraday_text::Text::of("S"),
        name: faraday_text::Text::of("vault.kdbx"),
        on_stick: faraday_text::Text::of("Vault.kdbx"),
        sha,
    });
    app
}

#[test]
fn a_vault_whose_name_a_file_has_brings_a_sheet_and_writes_nothing_yet() {
    let mut app = name_taken_visit(Some([7; 32]));
    assert!(asked(&mut app).is_empty());
    let texts = app.drawn_texts();
    for t in ["Hold: overwrite Vault.kdbx", "Save as vault-2.kdbx"] {
        assert!(texts.iter().any(|x| x == t), "{t}: {texts:?}");
    }
    assert!(
        texts.iter().any(|t| t.starts_with("Faraday cannot tell")),
        "{texts:?}"
    );
}

#[test]
fn holding_overwrite_sends_the_sha_shown_and_a_tap_does_not() {
    let mut app = name_taken_visit(Some([7; 32]));
    hold_action(&mut app, Action::NameTakenOverwrite, 100, 20);
    assert!(vault_writes(&mut app).is_empty(), "a tap sends nothing");
    hold_action(&mut app, Action::NameTakenOverwrite, 1_000, 200);
    assert!(
        vault_writes(&mut app).is_empty(),
        "a short hold sends nothing"
    );
    hold_action(
        &mut app,
        Action::NameTakenOverwrite,
        5_000,
        faraday_core::vault::HOLD_MS + 100,
    );
    assert_eq!(
        vault_writes(&mut app),
        [(
            "vault.kdbx".to_string(),
            true,
            faraday_core::OnTaken::Over([7; 32])
        )]
    );
}

#[test]
fn save_beside_sends_the_write_again_going_beside() {
    let mut app = name_taken_visit(Some([7; 32]));
    app.press(Action::NameTakenBeside);
    assert_eq!(
        vault_writes(&mut app),
        [(
            "vault.kdbx".to_string(),
            true,
            faraday_core::OnTaken::Beside
        )]
    );
}

#[test]
fn a_file_that_could_not_be_read_has_no_overwrite() {
    let mut app = name_taken_visit(None);
    let texts = app.drawn_texts();
    assert!(
        !texts.iter().any(|t| t.starts_with("Hold: overwrite")),
        "{texts:?}"
    );
    assert!(texts.iter().any(|t| t.starts_with("Save as ")), "{texts:?}");
}

#[test]
fn cancelling_the_sheet_writes_nothing_and_the_log_says_the_name_is_taken() {
    let mut app = name_taken_visit(Some([7; 32]));
    app.press(Action::Cancel);
    assert!(vault_writes(&mut app).is_empty());
    assert!(app.sheet.is_none());
    assert!(
        app.visit()
            .log
            .iter()
            .any(|(l, _)| l.starts_with("vault.kdbx not written: its name is taken")),
        "{:?}",
        app.visit().log
    );
    assert!(app.receipt().is_none(), "a cancel is no row of a receipt");
    assert!(app.outbox.iter().any(|i| i.name == "vault.kdbx"));
}

#[test]
fn keys_never_press_the_hold_to_overwrite() {
    // Whichever control the keyboard stops on, a key never writes over.
    for tabs in 1..=6 {
        for key in [
            faraday_shell_api::Key::Enter,
            faraday_shell_api::Key::Char(' '),
        ] {
            let mut app = name_taken_visit(Some([7; 32]));
            let _ = app.frame();
            for _ in 0..tabs {
                app.event(Event::Key(faraday_shell_api::Key::Tab));
            }
            app.event(Event::Key(key));
            assert!(
                !vault_writes(&mut app)
                    .iter()
                    .any(|(_, _, on)| matches!(on, faraday_core::OnTaken::Over(_))),
                "no key writes over"
            );
        }
    }
    let mut app = name_taken_visit(Some([7; 32]));
    let _ = app.frame();
    hold_action(
        &mut app,
        Action::NameTakenOverwrite,
        5_000,
        faraday_core::vault::HOLD_MS + 100,
    );
    assert_eq!(vault_writes(&mut app).len(), 1, "the hold still works");
}

#[test]
fn the_sheet_names_a_file_known_as_another_vault_and_only_then() {
    let vault = faraday_core::testkit::test_vault().unwrap();
    let sent = faraday_crypto::sha256(&vault);
    let body = |known: &[(u8, [u8; 32], &str)], file: [u8; 32]| {
        let mut app = writing_over(&[("Vault.kdbx", 99)], vec![("vault.kdbx", vault.clone())]);
        asked(&mut app);
        app.press(Action::VisitWrite);
        for (id, copy, name) in known {
            faraday_core::testkit::change_vault_summaries(&mut app, |summaries| {
                summaries.push(faraday_core::vault::VaultSummary {
                    file: faraday_text::Text::of("any.kdbx"),
                    id: [*id; 32],
                    copies: vec![*copy],
                    name: faraday_text::Text::of(name),
                    ..Default::default()
                });
            });
        }
        app.storage(StorageEvent::NameTaken {
            stick: faraday_text::Text::of("S"),
            name: faraday_text::Text::of("vault.kdbx"),
            on_stick: faraday_text::Text::of("Vault.kdbx"),
            sha: Some(file),
        });
        faraday_core::testkit::name_taken_body(&app).expect("the sheet asks")
    };
    let other = [7; 32];
    assert!(body(&[(1, sent, "Ours"), (2, other, "Savings")], other).contains("Savings"));
    // The bytes sent were never opened here: nobody knows they differ.
    let unknown = body(&[(2, other, "Savings")], other);
    assert!(!unknown.contains("Savings"));
    // The file was never opened here.
    assert!(!body(&[(1, sent, "Ours")], other).contains("Ours"));
}

#[test]
fn a_cancel_on_another_stick_leaves_the_receipt_of_the_last_write() {
    let vault = faraday_core::testkit::test_vault().unwrap();
    let mut app = writing_over(&[], vec![("vault.kdbx", vault.clone())]);
    asked(&mut app);
    app.press(Action::VisitWrite);
    asked(&mut app);
    app.storage(StorageEvent::Written {
        stick: faraday_text::Text::of("S"),
        name: faraday_text::Text::of("vault.kdbx"),
        wrote_as: faraday_text::Text::of("vault.kdbx"),
    });
    let first = app.receipt().expect("a receipt").clone();
    assert!(first.wrote("vault.kdbx"));
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("B"),
        label: faraday_text::Text::of("OTHERSTICK"),
        boot: false,
        files: vec![(faraday_text::Text::of("Vault.kdbx"), 99)],
        vaults: Vec::new(),
    }]));
    app.outbox.push(Item {
        name: faraday_text::Text::of("vault.kdbx"),
        bytes: (vault).into(),
        kind: FileKind::Pdf,
        secret: false,
        picture: None,
        sha: None,
        facts: None,
        wallet: None,
        psbt: None,
        outpoints: Vec::new(),
        unsigned_txid: None,
    });
    let at = app.outbox.len() - 1;
    if !app.visit().out.contains("vault.kdbx") {
        app.press(Action::VisitOut(at));
    }
    app.press(Action::VisitWrite);
    asked(&mut app);
    app.storage(StorageEvent::NameTaken {
        stick: faraday_text::Text::of("B"),
        name: faraday_text::Text::of("vault.kdbx"),
        on_stick: faraday_text::Text::of("Vault.kdbx"),
        sha: Some([7; 32]),
    });
    app.press(Action::Cancel);
    assert_eq!(app.receipt(), Some(&first), "the receipt stays");
}

#[test]
fn bytes_changed_after_the_question_are_not_written() {
    let mut app = name_taken_visit(Some([7; 32]));
    let last = app.outbox[0].bytes.len() - 1;
    app.outbox[0].bytes[last] ^= 1;
    app.press(Action::NameTakenBeside);
    assert!(vault_writes(&mut app).is_empty());
    assert!(app.sheet.is_none());
    assert!(
        app.visit()
            .log
            .iter()
            .any(|(l, _)| l.starts_with("vault.kdbx not written")),
        "{:?}",
        app.visit().log
    );
}

#[test]
fn a_file_changed_since_it_was_shown_is_not_written_and_the_receipt_says_why() {
    let mut app = name_taken_visit(Some([7; 32]));
    hold_action(
        &mut app,
        Action::NameTakenOverwrite,
        5_000,
        faraday_core::vault::HOLD_MS + 100,
    );
    assert_eq!(vault_writes(&mut app).len(), 1);
    let why = "Vault.kdbx changed since it was shown: not overwritten";
    app.storage(StorageEvent::WriteFailed {
        stick: faraday_text::Text::of("S"),
        name: faraday_text::Text::of("vault.kdbx"),
        reason: faraday_text::Text::of(why),
    });
    let receipt = app.receipt().expect("a receipt");
    let said = receipt
        .files
        .iter()
        .find(|f| f.name == "vault.kdbx")
        .unwrap();
    assert_eq!(said.failure.as_deref(), Some(why));
    assert!(app.outbox.iter().any(|i| i.name == "vault.kdbx"));
}

#[test]
fn the_free_name_follows_what_this_write_wrote() {
    let mut app = name_taken_visit(Some([7; 32]));
    app.storage(StorageEvent::Written {
        stick: faraday_text::Text::of("S"),
        name: faraday_text::Text::of("other.pdf"),
        wrote_as: faraday_text::Text::of("vault-2.kdbx"),
    });
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Save as vault-3.kdbx"),
        "{texts:?}"
    );
}

#[test]
fn a_lock_clears_the_questions() {
    let mut app = name_taken_visit(Some([7; 32]));
    app.press(Action::LockUnsealed);
    faraday_core::testkit::settle(&mut app);
    let texts = app.drawn_texts();
    assert!(
        !texts.iter().any(|t| t.starts_with("Save as ")),
        "{texts:?}"
    );
}

#[test]
fn two_clashes_in_one_write_ask_in_turn() {
    let vault = faraday_core::testkit::test_vault().unwrap();
    let mut app = writing_over(
        &[("Vault.kdbx", 99), ("Other.kdbx", 99)],
        vec![("vault.kdbx", vault.clone()), ("other.kdbx", vault)],
    );
    asked(&mut app);
    app.press(Action::VisitWrite);
    asked(&mut app);
    for (name, on_stick) in [("vault.kdbx", "Vault.kdbx"), ("other.kdbx", "Other.kdbx")] {
        app.storage(StorageEvent::NameTaken {
            stick: faraday_text::Text::of("S"),
            name: faraday_text::Text::of(name),
            on_stick: faraday_text::Text::of(on_stick),
            sha: Some([7; 32]),
        });
    }
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Save as vault-2.kdbx"),
        "{texts:?}"
    );
    app.press(Action::NameTakenBeside);
    let sent = vault_writes(&mut app);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, "vault.kdbx");
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Save as other-2.kdbx"),
        "the next is asked: {texts:?}"
    );
    app.press(Action::Cancel);
    assert!(app.sheet.is_none());
}

#[test]
fn a_stick_pulled_cancels_the_questions() {
    let mut app = name_taken_visit(Some([7; 32]));
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert!(app.sheet.is_none());
    assert!(
        app.outbox.iter().any(|i| i.name == "vault.kdbx"),
        "the file stays For the stick"
    );
    assert!(
        app.visit().log.iter().any(|(l, _)| l.contains("pulled")),
        "{:?}",
        app.visit().log
    );
}

#[test]
fn a_picture_asked_for_says_it_is_being_read_until_its_codes_come() {
    let mut app = with_stick(1);
    app.press(Action::VisitCopy);
    let (mut asked, mut reads) = (Vec::new(), Vec::new());
    while let Some(c) = app.poll_storage() {
        match c {
            StorageCommand::ReadQr { name, .. } => asked.push(name),
            StorageCommand::Read { stick, name } => reads.push((stick, name)),
            _ => {}
        }
    }
    assert_eq!(asked, ["photo.png"], "the picture is read for its codes");
    let _ = app.frame();
    let being_read = |app: &mut Faraday| {
        app.drawn_texts()
            .iter()
            .any(|t| t.starts_with("Being read"))
    };
    assert!(being_read(&mut app), "{:?}", app.drawn_texts());
    // The other files come at once; the picture's codes on a later pass
    // of the shell's storage.
    for (stick, name) in reads {
        app.storage(StorageEvent::Read {
            stick,
            name,
            bytes: b"text".to_vec().into(),
        });
    }
    let _ = app.frame();
    assert!(being_read(&mut app), "{:?}", app.drawn_texts());
    app.storage(StorageEvent::QrRead {
        stick: faraday_text::Text::of("S1"),
        name: faraday_text::Text::of("photo.png"),
        bytes: (b"\x89PNG".to_vec()).into(),
        payloads: (Vec::new()).into(),
    });
    let _ = app.frame();
    assert!(!being_read(&mut app), "{:?}", app.drawn_texts());
}

/// Puts a words file in the Inbox and loads a key with the stick out, so
/// the process holds a secret and is no longer clean; the stick goes back
/// in, its Lock sheet is put off with Not now, and the visit is opened.
/// The app keeps keys and a stick apart, so this stands for a visit that
/// stops being clean before its stick is closed to the process.
fn made_unclean(app: &mut Faraday) {
    faraday_core::testkit::inbox_push(
        app,
        Item {
            name: faraday_text::Text::of("my-words.txt"),
            bytes: (b"words".to_vec()).into(),
            kind: FileKind::Words,
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
    let stick = StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("S"),
        label: faraday_text::Text::of("TESTSTICK"),
        boot: false,
        files: Vec::new(),
        vaults: Vec::new(),
    };
    app.storage(StorageEvent::Sticks(Vec::new()));
    app.press(Action::Entry(None));
    for c in faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(faraday_shell_api::Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    faraday_core::testkit::keep_published(app);
    assert!(!app.clean());
    app.storage(StorageEvent::Sticks(vec![stick]));
    if app.sheet.is_some() {
        app.press(Action::Cancel);
    }
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
}

#[test]
fn write_on_a_clean_visit_writes_at_once() {
    let mut app = writing_over(&[], vec![("backup-00.pdf", b"new".to_vec())]);
    asked(&mut app);
    app.press(Action::VisitWrite);
    assert_ne!(app.sheet, Some(faraday_core::Sheet::WriteOut));
    assert_eq!(writes(&mut app), vec![("backup-00.pdf".to_string(), false)]);
}

#[test]
fn write_on_a_visit_that_is_no_longer_clean_asks_to_lock_and_names_what_a_lock_drops() {
    let mut app = writing_over(&[], vec![("backup-00.pdf", b"new".to_vec())]);
    made_unclean(&mut app);
    assert_eq!(app.screen, Screen::Files);
    asked(&mut app);
    app.press(Action::VisitWrite);
    assert_eq!(app.sheet, Some(faraday_core::Sheet::WriteOut));
    assert!(writes(&mut app).is_empty(), "nothing reaches the disk");
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Dropped from Files"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "my-words.txt"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "Lock"), "{texts:?}");
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    assert!(writes(&mut app).is_empty());
}

/// The disk process's refusal of a process that is no longer clean is
/// said on the visit in the app's words, never as the disk's token.
#[test]
fn the_disk_refusal_to_a_process_that_is_not_clean_is_said_in_the_apps_words() {
    let mut app = writing_over(&[], vec![("backup-00.pdf", b"new".to_vec())]);
    app.storage(StorageEvent::WriteFailed {
        stick: faraday_text::Text::of("S"),
        name: faraday_text::Text::of("backup-00.pdf"),
        reason: faraday_text::Text::of(faraday_files::proto::NOT_OPEN),
    });
    let _ = app.frame();
    let texts = app.drawn_texts().join("\n");
    assert!(
        texts.contains("not open to Faraday until it locks"),
        "{texts}"
    );
    assert!(!texts.contains(faraday_files::proto::NOT_OPEN), "{texts}");
}

/// A stick arriving with nothing held opens Files on the stick pane: the
/// chip's line and the pane's Import and Write.
#[test]
fn a_stick_arriving_opens_files_with_the_stick_pane() {
    let mut app = with_stick(2);
    app.press(Action::Nav(Screen::Home));
    app.storage(StorageEvent::Sticks(Vec::new()));
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("S"),
        label: faraday_text::Text::of("TESTSTICK"),
        boot: false,
        files: vec![(faraday_text::Text::of("spend-00.psbt"), 300)],
        vaults: Vec::new(),
    }]));
    assert_eq!(app.screen, Screen::Files);
    let _ = app.frame();
    assert!(app.offers(Action::VisitCopy), "Import is on Files");
    let texts = app.drawn_texts();
    assert!(!texts.iter().any(|t| t.starts_with("Then:")), "{texts:?}");
}

/// The bar and the rail keep the same destinations as a stick comes and
/// goes: Tools is never displaced (F235).
#[test]
fn the_navigation_does_not_change_as_a_stick_comes_and_goes() {
    for display in [(480, 800, 160), (1366, 768, 160)] {
        let mut app = with_stick_at(1, display);
        app.press(Action::Nav(Screen::Home));
        let with = [
            Screen::Home,
            Screen::WalletsStart,
            Screen::Files,
            Screen::Vaults,
            Screen::Catalog,
        ]
        .map(|s| app.offers(Action::Nav(s)));
        app.storage(StorageEvent::Sticks(Vec::new()));
        app.press(Action::Nav(Screen::Home));
        let without = [
            Screen::Home,
            Screen::WalletsStart,
            Screen::Files,
            Screen::Vaults,
            Screen::Catalog,
        ]
        .map(|s| app.offers(Action::Nav(s)));
        assert_eq!(with, without, "{display:?}");
        assert!(with.iter().all(|o| *o), "{display:?}: five destinations");
    }
}

/// Only the rows left ticked on Files are written to the stick.
#[test]
fn writing_the_ticked_rows_writes_those_files_to_the_stick() {
    let files = (0..3)
        .map(|i| (format!("spend-{i}.psbt"), vec![i as u8; 20]))
        .collect();
    let mut app = visiting(files);
    app.press(Action::VisitOut(1));
    app.press(Action::VisitWrite);
    let mut names = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Write { name, .. } = c {
            names.push(name);
        }
    }
    assert!(names.iter().any(|n| n == "spend-0.psbt"), "{names:?}");
    assert!(!names.iter().any(|n| n == "spend-1.psbt"), "{names:?}");
    assert!(names.iter().any(|n| n == "spend-2.psbt"), "{names:?}");
}

/// With something held, Files says Lock to use the stick and shows no
/// stick pane.
#[test]
fn with_something_held_files_says_lock_to_use_the_stick_and_has_no_pane() {
    let mut app = process((Vec::new(), Vec::new(), Vec::new()), None);
    app.press(Action::Entry(None));
    for c in faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(faraday_shell_api::Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    faraday_core::testkit::keep_published(&mut app);
    assert!(app.holds_secret(), "the key did not load");
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("S"),
        label: faraday_text::Text::of("TESTSTICK"),
        boot: false,
        files: vec![(faraday_text::Text::of("a.psbt"), 300)],
        vaults: Vec::new(),
    }]));
    app.press(Action::NotNow);
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t.contains("lock to use it")),
        "{texts:?}"
    );
    assert!(!app.offers(Action::VisitCopy));
    assert!(!app.offers(Action::VisitWrite));
}

/// A stick visit with a wallet of several keys in the Inbox, loaded.
fn visit_with_a_multisig_wallet() -> Faraday {
    let mut app = with_stick(0);
    let p = "m/48'/1'/0'/0'";
    let descriptor = format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
        faraday_core::testkit::key(0, p),
        faraday_core::testkit::key(1, p),
        faraday_core::testkit::key(2, p)
    );
    app.storage(StorageEvent::Read {
        stick: faraday_text::Text::of("S"),
        name: faraday_text::Text::of("shared-wallet.txt"),
        bytes: descriptor.into_bytes().into(),
    });
    app.press(Action::InboxLoad);
    app
}

#[test]
fn a_wallet_of_several_keys_loaded_on_a_visit_comes_back_to_the_visit_after_cancel() {
    let mut app = visit_with_a_multisig_wallet();
    assert_eq!(app.sheet, Some(faraday_core::Sheet::WalletConfirm));
    assert!(app.session.wallets.is_empty(), "nothing before the hold");
    app.press(Action::WalletCancel);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::Files);
    assert!(app.session.wallets.is_empty());
}

#[test]
fn a_wallet_of_several_keys_loaded_on_a_visit_comes_back_to_the_visit_after_the_hold() {
    let mut app = visit_with_a_multisig_wallet();
    assert_eq!(app.sheet, Some(faraday_core::Sheet::WalletConfirm));
    app.press(Action::WalletHold);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::Files);
    assert_eq!(app.session.wallets.len(), 1);
}

#[test]
fn scrolling_the_stick_list_leaves_the_page_where_it_was() {
    let mut app = with_outbox(with_stick(30), 20, false);
    let over = app.where_offered(Action::VisitIn(0)).unwrap();
    wheel(&mut app, over, 3000);
    assert!(app.visit_in_offset() > 0.0, "the stick's list did not move");
    assert_eq!(app.list_offset, 0.0, "the page moved with the list");
}

/// Where the text `want` is drawn, its first box (x, y, w, h), pixels.
fn drawn_at(app: &mut Faraday, want: &str) -> Option<(i32, i32, i32, i32)> {
    let _ = app.frame();
    app.drawn_text_boxes()
        .into_iter()
        .find(|(t, _)| t == want)
        .map(|(_, b)| b)
}

#[test]
fn files_draws_both_cards_with_a_stick_and_without_one() {
    let mut with = visiting(Vec::new());
    for want in ["In Faraday", "On the stick"] {
        assert!(drawn_at(&mut with, want).is_some(), "{want} with a stick");
    }
    with.storage(StorageEvent::Sticks(Vec::new()));
    for want in ["In Faraday", "On the stick"] {
        assert!(drawn_at(&mut with, want).is_some(), "{want} with no stick");
    }
    let texts = with.drawn_texts();
    assert!(
        texts.iter().any(|t| t.starts_with("Waiting for")),
        "{texts:?}"
    );
}

#[test]
fn with_no_stick_on_the_stick_has_no_buttons() {
    let mut app = visiting(Vec::new());
    app.storage(StorageEvent::Sticks(Vec::new()));
    let _ = app.frame();
    for (what, action) in [
        ("Import", Action::VisitCopy),
        ("Delete files", Action::DeleteFiles),
        ("Import and load", Action::VisitCopyAndLoad),
        ("Write", Action::VisitWrite),
    ] {
        assert!(!app.offers(action), "{what} is offered with no stick");
    }
    let texts = app.drawn_texts();
    assert!(
        !texts.iter().any(|t| t.starts_with("Rename")),
        "Rename stick with no stick: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t.starts_with("In memory")),
        "no explanation under In Faraday's list: {texts:?}"
    );
}

#[test]
fn rename_stick_is_in_the_heading_of_on_the_stick() {
    let mut app = visiting(Vec::new());
    let head = drawn_at(&mut app, "On the stick").expect("the card's heading");
    let rename = drawn_at(&mut app, "Rename stick").expect("Rename stick");
    assert!(rename.0 > head.0, "Rename stick is right of the heading");
    assert!(
        (rename.1 - head.1).abs() < head.3 * 2,
        "on the heading's row"
    );
}

/// Files with no stick in, `inbox` in the Inbox.
fn in_files(inbox: Files) -> Faraday {
    let mut app = process((inbox, Vec::new(), Vec::new()), None);
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    app
}

#[test]
fn a_long_inbox_summary_is_one_line_that_opens_whole() {
    // Every wallet of the test stick in the Inbox: a long summary.
    let mut app = in_files(faraday_core::testkit::files().unwrap());
    assert!(
        app.offers(Action::InboxSummary),
        "folded: a Show to open it"
    );
    assert!(
        !app.offers(Action::InboxLoad),
        "its Load is inside, folded away"
    );
    app.press(Action::InboxSummary);
    let _ = app.frame();
    assert!(app.offers(Action::InboxLoad), "open: its Load shows");
    app.press(Action::InboxSummary);
    let _ = app.frame();
    assert!(!app.offers(Action::InboxLoad), "folded again");
}

#[test]
fn a_short_inbox_summary_stands_open() {
    let one = faraday_core::testkit::files()
        .unwrap()
        .into_iter()
        .filter(|(n, _)| n == "spending-wallet.txt")
        .collect();
    let mut app = in_files(one);
    let _ = app.frame();
    assert!(app.offers(Action::InboxLoad));
    assert!(!app.offers(Action::InboxSummary), "nothing to fold");
}

#[test]
fn an_incomplete_share_says_how_many_keys_are_in_hand_on_its_row() {
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "savings")
        .unwrap();
    let share = faraday_core::testkit::public_files(kit.name, &kit.descriptor)
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "savings-share-1-of-3.txt")
        .unwrap();
    let mut app = in_files(vec![share]);
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t.contains("keys in hand")),
        "one share of a 2 of 3: {texts:?}"
    );
}

/// A row's main button leaves where the name would keep too little room:
/// on a wide window it is drawn, on a 360 px panel it is not, and pressing
/// the row does it, the tick box keeping the tick.
#[test]
fn a_rows_main_button_leaves_a_narrow_row_so_its_name_keeps_room() {
    let psbt = faraday_core::testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n.ends_with(".psbt"))
        .unwrap();
    let at = |width: u16| {
        let mut app = process(
            (vec![psbt.clone()], Vec::new(), Vec::new()),
            Some("TESTSTICK"),
        );
        app.event(Event::Display(DisplayInfo {
            width,
            height: 1600,
            dpi: 286,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: false,
            secure: SecureHardware::None,
            boot: BootState::Unknown,
            memory_mib: None,
        }));
        app.press(Action::Nav(Screen::Files));
        app.press(Action::VisitTab(Column::Outbox));
        let _ = app.frame();
        app
    };
    let mut wide = at(1000);
    let texts = wide.drawn_texts();
    assert!(texts.iter().any(|t| t == "Check a signer"), "{texts:?}");
    let mut app = at(360);
    let texts = app.drawn_texts();
    assert!(!texts.iter().any(|t| t == "Check a signer"), "{texts:?}");
    // The dropped button is still one press away.
    let k = app
        .inbox()
        .iter()
        .position(|i| i.name == psbt.0.as_str())
        .expect("the PSBT is not in the Inbox");
    let (x, _) = app
        .where_offered(Action::SignerCheck(
            faraday_core::decode::check::SignerPress::File(false, k),
        ))
        .expect("pressing the row does not open the check");
    let (tx, _) = app
        .where_offered(Action::VisitInbox(k))
        .expect("the tick box is gone");
    assert!(tx < x, "the tick box is not left of the row");
}

/// With a stick in, On the stick says what pulling it does next: here, a
/// vault the Inbox holds locked asks for its passphrase.
#[test]
fn on_the_stick_says_what_pulling_the_stick_does_next() {
    let vault = (
        "vault.kdbx".to_string(),
        faraday_core::testkit::test_vault().unwrap(),
    );
    let mut app = process((vec![vault], Vec::new(), Vec::new()), Some("TESTSTICK"));
    app.press(Action::Nav(Screen::Files));
    let texts = app.drawn_texts();
    assert!(
        texts
            .iter()
            .any(|t| t == "Remove the stick to unlock vault.kdbx"),
        "{texts:?}"
    );
}

/// On a small panel the stick's error line shows above the In Faraday
/// list too, not only on the On the stick tab.
#[test]
fn on_a_small_panel_the_boot_stick_error_shows_on_both_tabs() {
    let mut app = with_stick_at(3, (480, 640, 286));
    app.storage(StorageEvent::BootMedium(faraday_core::BootMedium::Two));
    let two = "Two sticks of this build are in";
    let _ = app.frame();
    assert!(
        app.drawn_texts().iter().any(|t| t.starts_with(two)),
        "In Faraday"
    );
    app.press(Action::VisitTab(Column::Stick));
    assert!(
        app.drawn_texts().iter().any(|t| t.starts_with(two)),
        "On the stick"
    );
}

/// The From the stick heading carries how many files are held.
#[test]
fn the_from_the_stick_heading_has_its_count() {
    let files = vec![
        ("a.txt".to_string(), b"a".to_vec()),
        ("b.txt".to_string(), b"b".to_vec()),
        ("c.txt".to_string(), b"c".to_vec()),
    ];
    let mut app = process((files, Vec::new(), Vec::new()), Some("TESTSTICK"));
    app.press(Action::Nav(Screen::Files));
    let boxes = app.drawn_text_boxes();
    let head = boxes
        .iter()
        .find(|(t, _)| &**t == "From the stick")
        .expect("heading")
        .1;
    assert!(
        boxes.iter().any(|(t, (x, y, ..))| &**t == "3"
            && *x > head.0 + head.2
            && (*y - head.1).abs() < head.3),
        "{boxes:?}"
    );
}

/// On the small panel the On the stick tab with an unprotected secret on
/// the stick and many files is one page that scrolls: the Import buttons
/// and Delete unprotected secrets are reached, and no two of its press
/// targets overlap at any scroll.
#[test]
fn on_a_small_panel_the_stick_tab_scrolls_to_every_action_without_overlap() {
    let mut app = with_stick_at(60, (480, 640, 286));
    app.outbox.push(Item {
        name: faraday_text::Text::of("backup-words.txt"),
        bytes: b"words".to_vec().into(),
        kind: faraday_core::wallet::FileKind::Words,
        secret: true,
        picture: None,
        sha: None,
        facts: None,
        wallet: None,
        psbt: None,
        outpoints: Vec::new(),
        unsigned_txid: None,
    });
    app.storage(StorageEvent::Written {
        stick: faraday_text::Text::of("S"),
        name: faraday_text::Text::of("backup-words.txt"),
        wrote_as: faraday_text::Text::of("backup-words.txt"),
    });
    app.press(Action::Nav(Screen::Files));
    app.press(Action::VisitTab(Column::Stick));
    let _ = app.frame();
    let actions = [
        Action::VisitTab(Column::Outbox),
        Action::VisitTab(Column::Stick),
        Action::DeleteFiles,
        Action::VisitCopy,
        Action::VisitCopyAndLoad,
        Action::DeleteUnprotected,
    ];
    let mut reached = Vec::new();
    for _ in 0..60 {
        let boxes: Vec<_> = actions
            .iter()
            .filter_map(|a| app.hit_box(*a).map(|b| (*a, b)))
            .collect();
        for (i, (a, b)) in boxes.iter().enumerate() {
            reached.push(*a);
            for (c, d) in &boxes[i + 1..] {
                assert!(!overlaps(*b, *d), "{a:?} {b:?} overlaps {c:?} {d:?}");
            }
        }
        wheel(&mut app, (240, 400), 300);
    }
    for a in [
        Action::DeleteFiles,
        Action::VisitCopy,
        Action::VisitCopyAndLoad,
        Action::DeleteUnprotected,
    ] {
        assert!(reached.contains(&a), "{a:?} was never reached");
    }
}

/// Two boxes overlap.
fn overlaps(a: (u16, u16, u16, u16), b: (u16, u16, u16, u16)) -> bool {
    a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
}

#[test]
fn write_stands_in_reach_under_a_long_list_and_writes_what_is_ticked() {
    let mut app = with_outbox(with_stick(30), 40, false);
    let (w, h) = (1366_u16, 768_u16);
    let at = app
        .where_offered(Action::VisitWrite)
        .expect("Write is not offered with a long list");
    assert!(at.0 < w && at.1 < h, "Write is off the screen: {at:?}");
    app.press(Action::VisitOut(3));
    tap(&mut app, at);
    let names = written(&mut app);
    assert_eq!(names.len(), 39, "{names:?}");
    assert!(!names.contains(&"backup-03.pdf".to_string()));
}

#[test]
fn write_with_nothing_ticked_does_nothing() {
    let mut app = with_outbox(with_stick(3), 4, false);
    app.press(Action::VisitOutAll);
    assert!(app.visit().out.is_empty(), "Unselect all left files ticked");
    if let Some(at) = app.where_offered(Action::VisitWrite) {
        tap(&mut app, at);
    }
    app.press(Action::VisitWrite);
    assert!(written(&mut app).is_empty());
}

#[test]
fn write_row_on_the_wide_layout_says_what_it_does_and_writes_the_ticked_files() {
    let mut app = with_outbox(with_stick(3), 4, false);
    app.press(Action::VisitOut(1));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "The checked files, to the stick"),
        "{texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t.starts_with("Write 3")),
        "the wide row carries no count: {texts:?}"
    );
    let at = app
        .where_offered(Action::VisitWrite)
        .expect("Write is not offered");
    tap(&mut app, at);
    assert_eq!(written(&mut app).len(), 3);
}

/// Both panes of the wide Files stand as two equal columns, level, and
/// every file's row and press is offered once.
fn assert_two_equal_columns(app: &mut Faraday, outbox: usize, stick: usize, what: &str) {
    let _ = app.frame();
    let offered = app.offered_actions();
    for i in 0..outbox {
        let n = offered
            .iter()
            .filter(|a| **a == Action::VisitOut(i))
            .count();
        assert_eq!(n, 1, "{what}: In Faraday row {i} offered {n} times");
    }
    for i in 0..stick {
        let n = offered.iter().filter(|a| **a == Action::VisitIn(i)).count();
        assert_eq!(n, 1, "{what}: stick row {i} offered {n} times");
    }
    let l = app.hit_box(Action::VisitOut(0)).expect("a row at the left");
    let r = app.hit_box(Action::VisitIn(0)).expect("a row at the right");
    assert_eq!(l.2, r.2, "{what}: the columns are as wide as each other");
    assert!(l.0 + l.2 < r.0, "{what}: In Faraday stands at the left");
    let boxes = app.drawn_text_boxes();
    let top = |s: &str| {
        boxes
            .iter()
            .find(|(t, _)| t.starts_with(s))
            .unwrap_or_else(|| panic!("{what}: no {s:?}"))
            .1
            .1
    };
    assert_eq!(top("In Faraday"), top("On "), "{what}: level");
}

#[test]
fn wide_files_are_two_equal_columns_level_with_every_row_offered_once() {
    for size in [(1366, 768, 160), (1920, 1080, 160)] {
        let mut app = with_outbox(with_stick_at(3, size), 2, false);
        assert_two_equal_columns(&mut app, 2, 3, &format!("{size:?}"));
    }
}

#[test]
fn a_row_removed_or_written_leaves_the_wide_columns_right() {
    for size in [(1366, 768, 160), (1920, 1080, 160)] {
        let mut app = with_outbox(with_stick_at(3, size), 3, false);
        app.press(Action::OutboxRemove(0));
        assert_two_equal_columns(&mut app, 2, 3, &format!("removed {size:?}"));
        app.press(Action::VisitWrite);
        let _ = app.frame();
        let l = app.hit_box(Action::VisitIn(0)).map(|b| b.2);
        let r = app.hit_box(Action::VisitOut(0)).map(|b| b.2);
        if let (Some(l), Some(r)) = (l, r) {
            assert_eq!(l, r, "written {size:?}");
        }
    }
}

/// In Faraday on a `size` display with `n` Outbox files, its own tab up.
fn in_faraday(size: (u16, u16, u16), n: usize) -> Faraday {
    let mut app = with_outbox(with_stick_at(3, size), n, false);
    if app.offers(Action::VisitTab(Column::Outbox)) {
        app.press(Action::VisitTab(Column::Outbox));
        let _ = app.frame();
    }
    app
}

/// The "more below" line on screen, with its box (x, y, w, h).
fn more_line(app: &mut Faraday) -> Option<(String, (i32, i32, i32, i32))> {
    let _ = app.frame();
    app.drawn_text_boxes()
        .into_iter()
        .find(|(t, _)| t.contains(" more · scroll to see "))
        .map(|(t, b)| (String::from(&*t), b))
}

#[test]
fn the_list_says_how_many_rows_are_below_and_one_reads_it() {
    let size = (1366, 768, 160);
    let lines: Vec<Option<String>> = (0..40)
        .map(|n| more_line(&mut in_faraday(size, n)).map(|l| l.0))
        .collect();
    let first = lines
        .iter()
        .position(Option::is_some)
        .expect("never overflows");
    assert!(first > 0, "{size:?}: no files already overflow");
    assert_eq!(
        lines[first].as_deref(),
        Some("1 more · scroll to see it"),
        "{size:?}"
    );
    assert_eq!(
        lines[first + 1].as_deref(),
        Some("2 more · scroll to see them"),
        "{size:?}"
    );
}

#[test]
fn the_more_line_stands_clear_above_the_write_row_at_every_size() {
    for size in [(1366, 768, 160), (480, 640, 286), (320, 480, 160)] {
        let mut app = in_faraday(size, 40);
        let (_, line) = more_line(&mut app).unwrap_or_else(|| panic!("{size:?}: no line"));
        let write = app
            .drawn_text_boxes()
            .into_iter()
            .find(|(t, _)| t.starts_with("Write"))
            .unwrap_or_else(|| panic!("{size:?}: no Write row"))
            .1;
        assert!(
            line.1 + line.3 <= write.1,
            "{size:?}: line {line:?} meets the Write row {write:?}"
        );
    }
}

#[test]
fn scrolling_to_the_end_takes_the_more_line_away() {
    let mut app = in_faraday((1366, 768, 160), 20);
    assert!(more_line(&mut app).is_some(), "no line to begin with");
    let over = app.where_offered(Action::VisitOut(0)).unwrap();
    wheel(&mut app, over, 3000);
    assert!(more_line(&mut app).is_none(), "the line stays at the end");
}

/// The Outbox and Inbox rows' foot: the line's top where one stands.
fn foot_of(size: (u16, u16, u16)) -> i32 {
    (0..60)
        .find_map(|n| more_line(&mut in_faraday(size, n)))
        .expect("never overflows")
        .1
        .1
}

#[test]
fn only_the_empty_groups_line_past_the_foot_says_nothing() {
    let size = (1366, 768, 160);
    let foot = foot_of(size);
    // The most files whose rows all end at or above the foot: the Inbox's
    // heading and its Empty line stand below, partly or wholly hidden.
    let fits = |n: usize| {
        let mut app = in_faraday(size, n);
        let _ = app.frame();
        let last = format!("backup-{:02}", n - 1);
        app.drawn_text_boxes()
            .into_iter()
            .find(|(t, _)| t.starts_with(&last))
            .is_some_and(|(_, b)| b.1 - 6 + 56 <= foot)
    };
    let n = (1..60).take_while(|&n| fits(n)).last().expect("none fits");
    let mut app = in_faraday(size, n);
    assert!(
        more_line(&mut app).is_none(),
        "{n} files: a line for no hidden row"
    );
}

#[test]
fn one_file_partly_hidden_reads_one_more() {
    let size = (1366, 768, 160);
    let first = (0..60)
        .find(|&n| more_line(&mut in_faraday(size, n)).is_some())
        .expect("never overflows");
    let mut app = in_faraday(size, first);
    let (text, line) = more_line(&mut app).unwrap();
    assert_eq!(text, "1 more · scroll to see it");
    let last = app
        .drawn_text_boxes()
        .into_iter()
        .find(|(t, _)| t.starts_with(&format!("backup-{:02}", first - 1)))
        .expect("last file not drawn");
    assert!(last.1.1 < line.1, "the last file starts below the foot");
}

#[test]
fn the_stick_lists_number_is_its_files_below_the_foot() {
    let size = (1366, 768, 160);
    for n in [20usize, 26, 31] {
        let mut app = with_stick_at(n, size);
        let line = more_line(&mut app).unwrap_or_else(|| panic!("{n}: no line"));
        let said: usize = line.0.split(' ').next().unwrap().parse().unwrap();
        let foot = line.1.1;
        // A file is hidden unless its row, name line 6 px under its top
        // and 56 px tall, ends at or above the foot.
        let all: Vec<_> = app.drawn_text_boxes();
        let shown = all
            .iter()
            .filter(|(t, b)| {
                (t.ends_with(".psbt") || t == "photo.png" || t == "notes.docx")
                    && b.1 - 6 + 56 <= foot
            })
            .count();
        assert_eq!(said, n + 2 - shown, "{n} files");
    }
}
