//! Scrolling as the shell API states it: `dy` pixels the content moves
//! up. A wheel notch (48 pixels) moves a page by 48, not half a page; a
//! list moves by the same pixels too, continuously, not a row at a time.

use faraday_core::testkit;
use faraday_core::{Action, Faraday, Screen, StorageEvent};
use osk_shell_api::{App, Event};

fn scroll(app: &mut Faraday, dy: i16) {
    app.event(Event::Scroll { x: 600, y: 400, dy });
}

#[test]
fn a_notch_moves_the_page_by_its_pixels_in_the_direction_asked() {
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let mut app = Faraday::new();
    app.storage(StorageEvent::Restored {
        inbox: vec![("spending-wallet.txt".into(), kit.descriptor.into_bytes())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
    app.press(Action::Backup(0));
    assert_eq!(app.screen, Screen::Backup);
    // Content up: the page scrolls down by the notch.
    scroll(&mut app, 48);
    assert_eq!(app.backup.as_ref().unwrap().scroll.y, 48.0);
    // Content down: back up, and no further than the top.
    scroll(&mut app, -96);
    assert_eq!(app.backup.as_ref().unwrap().scroll.y, 0.0);
}

#[test]
fn a_list_scrolls_by_the_same_pixels_continuously() {
    let mut app = Faraday::new();
    app.press(Action::Nav(Screen::Files));
    for _ in 0..5 {
        scroll(&mut app, 10);
    }
    // No row-snapping: five small steps are already fifty pixels moved,
    // not zero waiting for a row's worth to build up.
    assert_eq!(app.list_offset, 50.0);
    scroll(&mut app, -1000);
    assert_eq!(app.list_offset, 0.0, "scrolling up stops at the top");
}
