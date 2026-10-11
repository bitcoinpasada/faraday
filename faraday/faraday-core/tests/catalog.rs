//! Tools: every flow on one page. Typing finds a tool by its name or a
//! standard's number; a tile opens its flow, or says what it needs first
//! and opens nothing.

use faraday_core::catalog::{TILES, matches};
use faraday_core::create::NewKind;
use faraday_core::{Action, Faraday, Screen};
use osk_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};

fn shown() -> Faraday {
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
    for i in 0..4u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x10 + i; 32])));
    }
    app.press(Action::Nav(Screen::Catalog));
    app
}

fn tile(name: &str) -> u8 {
    TILES.iter().position(|t| t.name == name).unwrap() as u8
}

fn found(find: &str) -> Vec<&'static str> {
    TILES
        .iter()
        .filter(|t| matches(t, find))
        .map(|t| t.name)
        .collect()
}

#[test]
fn a_bip_number_finds_its_tools() {
    assert_eq!(found("85"), ["Child seeds and passwords"]);
    assert_eq!(found("bip-352"), ["Silent payments"]);
    assert!(found("frost").contains(&"FROST threshold"));
    assert!(found("musig2").contains(&"MuSig2"));
    assert!(found("no such tool").is_empty());
}

#[test]
fn a_tag_s_hyphen_need_not_be_typed_to_find_it() {
    assert_eq!(found("bip85"), found("bip-85"));
    assert_eq!(found("bip 85"), found("bip-85"));
    assert!(!found("bip85").is_empty());
}

#[test]
fn typing_on_the_page_goes_to_find_a_tool() {
    let mut app = shown();
    for c in "slip".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    assert_eq!(app.catalog_find, "slip");
    app.event(Event::Key(Key::Escape));
    assert!(app.catalog_find.is_empty());
}

#[test]
fn the_musig2_tile_opens_create_with_musig2_chosen() {
    let mut app = shown();
    app.press(Action::Catalog(tile("MuSig2")));
    assert_eq!(app.screen, Screen::Create);
    assert_eq!(app.create.as_ref().unwrap().kind, NewKind::MuSig);
}

#[test]
fn the_slip39_tile_opens_new_key_making_shares() {
    let mut app = shown();
    app.press(Action::Catalog(tile("SLIP-39 shares")));
    assert_eq!(app.screen, Screen::KeyGen);
    assert!(app.keygen.as_ref().unwrap().slip39);
}

#[test]
fn a_calculator_tile_opens_that_calculator() {
    let mut app = shown();
    app.press(Action::Catalog(tile("Units")));
    assert_eq!(app.screen, Screen::Tools);
}

#[test]
fn a_tile_that_needs_a_key_says_so_and_opens_nothing() {
    let mut app = shown();
    let i = tile("Child seeds and passwords");
    assert_eq!(
        app.tile_need(TILES[usize::from(i)].go),
        Some("Load a key first")
    );
    app.press(Action::Catalog(i));
    assert_eq!(app.screen, Screen::Catalog);
}

#[test]
fn tools_is_in_the_sidebar() {
    let mut app = shown();
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    assert!(app.offers(Action::Nav(Screen::Catalog)));
}
