//! The stick visit's Import list: Select all chooses every file Faraday
//! reads, pictures of QR codes with them, and a second press none, and a
//! long list has a scrollbar that drags it to the end at once.

use faraday_core::{Action, Faraday, Screen, StickInfo, StorageEvent};
use osk_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};

/// A stick with `n` PSBTs, a picture and a file Faraday does not read.
fn with_stick(n: usize) -> Faraday {
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
fn select_all_chooses_every_readable_file_and_then_none() {
    let mut app = with_stick(5);
    assert!(app.offers(Action::VisitInAll));
    app.press(Action::VisitInAll);
    assert_eq!(
        app.visit.inn.len(),
        6,
        "the PSBTs and the picture, and no kind Faraday does not read"
    );
    assert!(app.visit.inn.contains("photo.png"));
    app.press(Action::VisitInAll);
    assert!(app.visit.inn.is_empty());
}

#[test]
fn dragging_the_scrollbar_to_the_bottom_shows_the_end_of_a_long_list() {
    let mut app = with_stick(60);
    let (x, y) = app
        .where_offered(Action::VisitBar)
        .expect("a long list has no scrollbar");
    let (top, h, _, max) = app.visit.bar.get().unwrap();
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
    assert!(!app.offers(Action::VisitBar));
}
