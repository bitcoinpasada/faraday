//! The stick visit: Select all on the Import list chooses every file
//! Faraday reads, pictures of QR codes with them, and a second press none;
//! Select all on the Outbox list chooses every file but an unprotected
//! secret; each long list has a scrollbar that drags it to the end at
//! once, and the wheel moves the list it is over and not the other.

use faraday_core::wallet::FileKind;
use faraday_core::{
    Action, Column, Faraday, Item, Screen, StickInfo, StorageCommand, StorageEvent,
};
use osk_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};

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
        id: "S".into(),
        label: "TESTSTICK".into(),
        boot: false,
        files,
    }]));
    app.press(Action::Nav(Screen::Visit));
    let _ = app.frame();
    app
}

#[test]
fn select_all_chooses_every_file_and_then_none() {
    let mut app = with_stick(5);
    assert!(app.offers(Action::VisitInAll));
    app.press(Action::VisitInAll);
    assert_eq!(
        app.visit.inn.len(),
        7,
        "the PSBTs, the picture and a file of a kind Faraday does not know"
    );
    assert!(app.visit.inn.contains("photo.png"));
    assert!(app.visit.inn.contains("notes.docx"));
    app.press(Action::VisitInAll);
    assert!(app.visit.inn.is_empty());
}

#[test]
fn dragging_the_scrollbar_to_the_bottom_shows_the_end_of_a_long_list() {
    let mut app = with_stick(60);
    let (x, y) = app
        .where_offered(Action::VisitBar(Column::Stick))
        .expect("a long list has no scrollbar");
    let (top, h, _, max) = app.visit.bar(Column::Stick).get().unwrap();
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
            bytes: vec![b'%'; 2000],
            kind: FileKind::Pdf,
            secret: false,
            picture: None,
        });
    }
    if secret {
        app.outbox.push(Item {
            name: "seed-words.txt".into(),
            bytes: b"words".to_vec(),
            kind: FileKind::Text,
            secret: true,
            picture: None,
        });
    }
    app.press(Action::Nav(Screen::Visit));
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
    let mut app = with_outbox(with_stick_at(30, (960, 640, 160)), 20, false);
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
        !app.visit.out.contains("backup-19.pdf"),
        "it did not untick"
    );
    app.press(Action::VisitWrite);
    let names = written(&mut app);
    assert_eq!(names.len(), 19, "{names:?}");
    assert!(!names.contains(&"backup-19.pdf".to_string()));
}

#[test]
fn the_wheel_over_the_stick_files_moves_them_and_not_the_outbox() {
    let mut app = with_outbox(with_stick_at(30, (960, 640, 160)), 20, false);
    let over = app.where_offered(Action::VisitIn(0)).unwrap();
    wheel(&mut app, over, 3000);
    assert!(app.list_offset > 0.0, "the stick's file list did not move");
    assert_eq!(app.visit.out_offset, 0.0, "the Outbox list moved");
    assert!(app.offers(Action::VisitIn(29)));
    assert!(app.offers(Action::VisitOut(0)));
}

#[test]
fn dragging_the_outbox_scrollbar_to_the_bottom_shows_its_last_file() {
    let mut app = with_outbox(with_stick_at(3, (960, 640, 160)), 20, false);
    let (x, _) = app
        .where_offered(Action::VisitBar(Column::Outbox))
        .expect("20 Outbox files have no scrollbar");
    let (top, h, _, max) = app.visit.bar(Column::Outbox).get().unwrap();
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
    assert_eq!(app.visit.out_offset, max);
    let _ = app.frame();
    assert!(app.offers(Action::VisitOut(19)));
}

#[test]
fn select_all_on_the_outbox_leaves_an_unprotected_secret_alone() {
    for display in [(960, 640, 160), (480, 640, 286)] {
        let mut app = with_outbox(with_stick_at(3, display), 20, true);
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
        assert!(!app.visit.out.contains("seed-words.txt"));
        // The settings are off on a stick that did not boot: the first
        // press ticks them too.
        app.press(Action::VisitOutAll);
        app.press(Action::VisitOutAll);
        assert!(app.visit.out.is_empty(), "{display:?}: not all unticked");
        app.press(Action::VisitWrite);
        assert!(written(&mut app).is_empty());
        app.press(Action::VisitOutAll);
        assert_eq!(app.visit.out.len(), 20, "{display:?}");
        assert!(!app.visit.out.contains("seed-words.txt"));
        // With the secret ticked by hand as well, every row is ticked
        // and a press unticks them all.
        let secret = app.outbox.len() - 1;
        app.press(Action::VisitOut(secret));
        app.press(Action::VisitOutAll);
        assert!(app.visit.out.is_empty());
    }
}

#[test]
fn a_finger_dragged_on_the_outbox_moves_it_and_not_the_stick_list() {
    let mut app = with_outbox(with_stick_at(30, (960, 640, 160)), 20, false);
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
    assert!(app.visit.out_offset > 0.0, "the Outbox did not move");
    assert_eq!(app.list_offset, 0.0, "the stick's file list moved");
    assert!(
        app.visit.out.contains("backup-03.pdf"),
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
        inbox: boxes.0,
        outbox: boxes.1,
        kept: boxes.2,
    });
    app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    if let Some(label) = stick {
        app.storage(StorageEvent::Sticks(vec![StickInfo {
            id: "S".into(),
            label: label.into(),
            boot: false,
            files: Vec::new(),
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
            last = (inbox, outbox, kept);
        }
    }
    last
}

fn files_texts(app: &mut Faraday) -> Vec<String> {
    app.press(Action::Nav(Screen::Files));
    app.drawn_texts()
}

/// A write leaves a receipt (`docs/SIMPLIFY.md` §4.3): Files lists it
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
    app.press(Action::Nav(Screen::Visit));
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
