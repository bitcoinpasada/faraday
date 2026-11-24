//! A file's list of addresses (Check an address) shows each address
//! whole: grouped in fours, on one line where it fits, wrapped only
//! between groups where the room requires it, never cut.

use faraday_core::address_check::AddressPress;
use faraday_core::ui::TextSize;
use faraday_core::{Action, Faraday, Screen, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

const SHORT: &str = "mhLeXQvZ8zzN2idymESqznYm8EcZupZP4W";
const TAPROOT: &str = "tb1p8wpt9v4frpf3tkn0srd97pksgsxc5hs52lafxwru9kgeephvs7rqlqt9zj";
const SEGWIT: &str = "tb1q6rz28mcfaxtmd6v789l9rrlrusdprr9pqcpvkl";

fn window(app: &mut Faraday, width: u16, height: u16) {
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
}

fn groups(a: &str) -> Vec<String> {
    a.as_bytes()
        .chunks(4)
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect()
}

/// The page with a file of three addresses listed, at the window and text size.
fn listed(width: u16, height: u16, size: TextSize, first: &str) -> Faraday {
    let mut app = testkit::started();
    app.press(Action::TextSize(size));
    window(&mut app, width, height);
    let wallet = format!("wpkh({}/<0;1>/*)", testkit::key(0, "m/84h/1h/0h"));
    // The address under test stands first, so it is in view whatever the window.
    let filler = if first == SHORT { SEGWIT } else { SHORT };
    let text = format!("{first}\n{filler}\n");
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (faraday_text::Text::of("wallet.txt"), wallet.into_bytes()),
            (faraday_text::Text::of("list.txt"), text.into_bytes()),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Check an address")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    app.press(Action::AddressCheck(AddressPress::File(1)));
    let _ = app.frame();
    app
}

/// The lines drawn that are made of groups of an address, joined by a
/// space, per address.
fn shown(app: &mut Faraday, address: &str) -> Vec<String> {
    let _ = app.frame();
    let gs = groups(address);
    app.drawn_text_boxes()
        .into_iter()
        .map(|(t, _)| t.as_str().trim().to_string())
        .filter(|t| {
            let w: Vec<&str> = t.split(' ').collect();
            !w.is_empty() && w.iter().all(|g| gs.iter().any(|x| x == g))
        })
        .collect()
}

fn whole_at(width: u16, height: u16, size: TextSize) {
    for a in [SHORT, TAPROOT, SEGWIT] {
        let mut app = listed(width, height, size, a);
        let lines = shown(&mut app, a);
        let gs = groups(a);
        let mut seen = Vec::new();
        for l in &lines {
            for g in l.split(' ') {
                if gs.iter().any(|x| x == g) {
                    seen.push(g.to_string());
                }
            }
        }
        // Every group of the address stands on a line, in order.
        let mut at = 0;
        for g in &seen {
            if at < gs.len() && *g == gs[at] {
                at += 1;
            }
        }
        assert_eq!(at, gs.len(), "{a} at {width}x{height}: {lines:?}");
        assert!(
            !lines.iter().any(|l| l.contains('…')),
            "{a} at {width}x{height}: {lines:?}"
        );
    }
}

#[test]
fn every_address_of_a_files_list_is_whole_at_each_window() {
    // The page scrolls: the narrowest window is tall enough to hold the row.
    for (w, h) in [(1920, 1080), (1366, 768), (480, 640), (320, 720)] {
        whole_at(w, h, TextSize::Standard);
    }
}

#[test]
fn every_address_of_a_files_list_is_whole_at_larger_text() {
    whole_at(480, 640, TextSize::Large);
}

#[test]
fn a_short_address_stands_on_one_line_where_it_fits() {
    let mut app = listed(1366, 768, TextSize::Standard, SHORT);
    assert_eq!(shown(&mut app, SHORT).len(), 1);
}

#[test]
fn a_taproot_address_wraps_only_between_groups() {
    let mut app = listed(320, 480, TextSize::Standard, TAPROOT);
    let lines = shown(&mut app, TAPROOT);
    assert!(lines.len() > 1, "{lines:?}");
    let gs = groups(TAPROOT);
    for l in &lines {
        for g in l.split(' ') {
            assert!(gs.iter().any(|x| x == g), "a cut group {g:?} in {lines:?}");
        }
    }
}

#[test]
fn a_pressed_row_still_opens_its_address() {
    let mut app = listed(1366, 768, TextSize::Standard, SHORT);
    let _ = app.frame();
    assert!(
        app.hit_box(Action::AddressCheck(AddressPress::Row(0)))
            .is_some()
    );
    app.press(Action::AddressCheck(AddressPress::Row(0)));
    assert!(app.address_check_state().unwrap().list_lines().is_empty());
}
