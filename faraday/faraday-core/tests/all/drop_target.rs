//! A file dragged over Faraday Desktop's window (`docs/DESIGN.md` §4.17,
//! Window drop target): the page is edged in the accent while it is
//! over the window, as the board edges where a chip drops, and the edge
//! goes when the drag leaves or the file is dropped. The device, which
//! takes no drop, never edges it.

use faraday_core::wallet::Session;
use faraday_core::{Faraday, Screen, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

/// The app at `width` × `height` at 160 dpi, online (the desktop's) or
/// not, on Settings.
fn app(width: u16, height: u16, online: bool) -> Faraday {
    let mut app = Faraday::new();
    app.online = online;
    app.session = Session::on(testkit::NET);
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
    app.press(faraday_core::Action::Nav(Screen::Settings));
    // At rest: the frame read is Settings, not part of the fade to it.
    app.settle();
    app
}

/// The pixel at (x, y) of the frame drawn now.
fn pixel(app: &mut Faraday, x: usize, y: usize) -> [u8; 3] {
    let f = app.frame();
    let i = (y * usize::from(f.width) + x) * 4;
    [f.rgba[i], f.rgba[i + 1], f.rgba[i + 2]]
}

/// The page's right edge, half way down: the edge's place on every
/// layout.
fn edge(app: &mut Faraday, width: u16, height: u16) -> [u8; 3] {
    pixel(app, usize::from(width) - 1, usize::from(height) / 2)
}

#[test]
fn a_file_dragged_over_the_window_edges_the_page_until_the_drag_leaves() {
    for (w, h) in [(400, 800), (760, 1000), (1280, 800)] {
        let mut app = app(w, h, true);
        let before = edge(&mut app, w, h);
        app.storage(StorageEvent::Hovered(true));
        let over = edge(&mut app, w, h);
        assert_ne!(over, before, "{w} × {h}: not edged");
        app.storage(StorageEvent::Hovered(false));
        assert_eq!(edge(&mut app, w, h), before, "{w} × {h}: still edged");
    }
}

#[test]
fn a_drop_takes_the_edge_away() {
    let (w, h) = (1280, 800);
    let mut app = app(w, h, true);
    app.storage(StorageEvent::Hovered(true));
    let over = edge(&mut app, w, h);
    app.storage(StorageEvent::Dropped {
        path: "/nowhere/note.txt".into(),
        size: 6,
    });
    assert_eq!(app.screen, Screen::Transfer);
    app.settle();
    assert_ne!(edge(&mut app, w, h), over);
}

#[test]
fn the_device_takes_no_drop_and_draws_no_edge() {
    let (w, h) = (1280, 800);
    let mut app = app(w, h, false);
    let before = edge(&mut app, w, h);
    app.storage(StorageEvent::Hovered(true));
    assert_eq!(edge(&mut app, w, h), before);
}
