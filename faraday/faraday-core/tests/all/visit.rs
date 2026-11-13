//! The stick visit: every file Faraday reads comes in ticked but a
//! seed's, and Unselect all clears them; Select all on the For the stick
//! list chooses every file but an unprotected secret; each long list has
//! a scrollbar that drags it to the end at once, and the wheel moves the
//! list it is over and not the other. The chip says where pulling the
//! stick goes: Unlock when a vault was sealed for the visit, else Home.

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
    let mut app = Faraday::new();
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
    files.push(("photo.png".into(), 9000));
    files.push(("notes.docx".into(), 9000));
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: "S".into(),
        label: "TESTSTICK".into(),
        boot: false,
        files,
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
    let mut app = Faraday::new();
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
        id: "S".into(),
        label: "TESTSTICK".into(),
        boot: false,
        files: vec![
            ("savings-unsigned.psbt".into(), 900),
            ("savings-descriptor.txt".into(), 300),
            ("savings-9a6a2580-words.txt".into(), 120),
            ("savings-9a6a2580-seedqr.png".into(), 4000),
            ("film.mp4".into(), 19 * 1024 * 1024),
        ],
        vaults: Vec::new(),
    }]));
    assert_eq!(app.screen, Screen::Files);
    let ticked: Vec<&str> = app.visit().inn.iter().map(String::as_str).collect();
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
    assert_eq!(app.list_offset, max, "the list is not at its end");
    app.event(Event::Touch {
        x,
        y: top as u16,
        phase: TouchPhase::Move,
    });
    assert_eq!(
        app.list_offset, 0.0,
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
            name: format!("backup-{i:02}.pdf"),
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
            name: "seed-words.txt".into(),
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
            names.push(name);
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
    assert_eq!(app.list_offset, 0.0, "the stick's file list moved");
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
    assert!(app.list_offset > 0.0, "the stick's file list did not move");
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
        // The settings are off on a stick that did not boot: the first
        // press ticks them too.
        app.press(Action::VisitOutAll);
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
    assert_eq!(app.list_offset, 0.0, "the stick's file list moved");
    assert!(
        app.visit().out.contains("backup-03.pdf"),
        "the drag ticked or unticked the file it started on"
    );
}

type Files = Vec<(String, Vec<u8>)>;

/// A process restored from `boxes` (Inbox, Outbox, kept), with the clock
/// known and `stick` attached when given.
fn process(boxes: (Files, Files, Files), stick: Option<&str>) -> Faraday {
    let mut app = Faraday::new();
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
        kept: boxes.2,
    });
    app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    if let Some(label) = stick {
        app.storage(StorageEvent::Sticks(vec![StickInfo {
            make: Default::default(),
            serial: None,
            id: "S".into(),
            label: label.into(),
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
                kept,
            );
        }
    }
    last
}

fn files_texts(app: &mut Faraday) -> Vec<String> {
    app.press(Action::Nav(Screen::Files));
    app.drawn_texts()
}

/// A write leaves a receipt (`docs/history/SIMPLIFY.md` §4.3): Files lists it
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
fn the_stick_pane_says_then_home_and_pulling_stays_on_files() {
    let wallet = faraday_core::testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "spending-wallet.txt")
        .unwrap();
    let mut app = visiting(vec![wallet]);
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Then: Home"), "{texts:?}");
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert_eq!(app.screen, Screen::Files, "nothing sends on: Files stays");
    let texts = app.drawn_texts();
    assert!(
        !texts.iter().any(|t| t.starts_with("Then:")),
        "the pane closes with the stick: {texts:?}"
    );
}

#[test]
fn a_visit_for_a_vault_sealed_at_lock_says_then_unlock_and_goes_there() {
    let vault = (
        "vault.kdbx".to_string(),
        faraday_core::testkit::test_vault().unwrap(),
    );
    let mut app = visiting(vec![vault]);
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Then: Unlock"), "{texts:?}");
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert_eq!(app.screen, Screen::Unlock);
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
                stick: "S".into(),
                from: 2,
            },
        ),
        (
            2,
            "Hold to zero the whole partition",
            StorageCommand::MakeEmpty { stick: "S".into() },
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
        vec![StorageCommand::StickSpace { stick: "S".into() }]
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
            stick: "S".into(),
            name: "spend-01.psbt".into()
        }]
    );
    app.storage(StorageEvent::Wiped {
        stick: "S".into(),
        name: "spend-01.psbt".into(),
    });
    let _ = app.frame();
    assert!(!app.visit().inn.contains("spend-01.psbt"));
    let text = app.drawn_texts().join("\n");
    assert!(text.contains("spend-01.psbt deleted"), "{text}");
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
        stick: "S".into(),
        name: "spend-01.psbt".into(),
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
        stick: "S".into(),
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
            stick: "S".into(),
            from: 2
        }]
    );
    app.storage(StorageEvent::Zeroing {
        stick: "S".into(),
        next: 400,
        total: 1000,
        done: false,
    });
    assert!(asked(&mut app).is_empty(), "nothing before the next tick");
    tick(&mut app, 5_000);
    assert_eq!(
        asked(&mut app),
        vec![StorageCommand::ZeroFree {
            stick: "S".into(),
            from: 400
        }]
    );
    app.storage(StorageEvent::Zeroing {
        stick: "S".into(),
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
        vec![StorageCommand::MakeEmpty { stick: "S".into() }]
    );
    app.storage(StorageEvent::Zeroing {
        stick: "S".into(),
        next: 2,
        total: 0,
        done: false,
    });
    for pass in 0..2 {
        tick(&mut app, 1_000 + pass);
        assert_eq!(
            asked(&mut app),
            vec![StorageCommand::ZeroFree {
                stick: "S".into(),
                from: 2
            }],
            "pass {}",
            pass + 1
        );
        app.storage(StorageEvent::Zeroing {
            stick: "S".into(),
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
        stick: "S".into(),
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
    let mut app = Faraday::new();
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
            name: name.into(),
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
        id: "S".into(),
        label: "TESTSTICK".into(),
        boot: false,
        files: on_stick.iter().map(|(n, s)| (n.to_string(), *s)).collect(),
        vaults: Vec::new(),
    }]));
    app.press(Action::Nav(Screen::Files));
    let names: Vec<String> = app.outbox.iter().map(|i| i.name.clone()).collect();
    for (i, n) in names.iter().enumerate() {
        if !app.visit().out.contains(n) {
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
                Some((name, replace))
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
            } if name != faraday_core::stick::settings::FILE => Some((name, replace, on_taken)),
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
        stick: "S".into(),
        name: "vault.kdbx".into(),
        on_stick: "Vault.kdbx".into(),
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
            stick: "S".into(),
            name: "vault.kdbx".into(),
            on_stick: "Vault.kdbx".into(),
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
        stick: "S".into(),
        name: "vault.kdbx".into(),
        wrote_as: "vault.kdbx".into(),
    });
    let first = app.receipt().expect("a receipt").clone();
    assert!(first.wrote("vault.kdbx"));
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: "B".into(),
        label: "OTHERSTICK".into(),
        boot: false,
        files: vec![("Vault.kdbx".into(), 99)],
        vaults: Vec::new(),
    }]));
    app.outbox.push(Item {
        name: "vault.kdbx".into(),
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
        stick: "B".into(),
        name: "vault.kdbx".into(),
        on_stick: "Vault.kdbx".into(),
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
        stick: "S".into(),
        name: "vault.kdbx".into(),
        reason: why.into(),
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
        stick: "S".into(),
        name: "other.pdf".into(),
        wrote_as: "vault-2.kdbx".into(),
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
            stick: "S".into(),
            name: name.into(),
            on_stick: on_stick.into(),
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
            bytes: b"text".to_vec(),
        });
    }
    let _ = app.frame();
    assert!(being_read(&mut app), "{:?}", app.drawn_texts());
    app.storage(StorageEvent::QrRead {
        name: "photo.png".into(),
        bytes: b"\x89PNG".to_vec(),
        payloads: Vec::new(),
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
            name: "my-words.txt".into(),
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
        id: "S".into(),
        label: "TESTSTICK".into(),
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
        stick: "S".into(),
        name: "backup-00.pdf".into(),
        reason: faraday_files::proto::NOT_OPEN.into(),
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
        id: "S".into(),
        label: "TESTSTICK".into(),
        boot: false,
        files: vec![("spend-00.psbt".into(), 300)],
        vaults: Vec::new(),
    }]));
    assert_eq!(app.screen, Screen::Files);
    let _ = app.frame();
    assert!(app.offers(Action::VisitCopy), "Import is on Files");
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Then: Home"), "{texts:?}");
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
    assert!(app.holds_secret(), "the key did not load");
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: "S".into(),
        label: "TESTSTICK".into(),
        boot: false,
        files: vec![("a.psbt".into(), 300)],
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
