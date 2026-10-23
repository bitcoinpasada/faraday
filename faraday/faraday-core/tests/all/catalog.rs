//! Tools: every flow on one page. Typing finds a tool by its name or a
//! standard's number; a tile opens its flow, or says what it needs first
//! and opens nothing.

use faraday_core::catalog::{TILES, matches};
use faraday_core::create::NewKind;
use faraday_core::{Action, Faraday, Screen, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};

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
    assert_eq!(testkit::catalog_find(&app), "slip");
    app.event(Event::Key(Key::Escape));
    assert!(testkit::catalog_find(&app).is_empty());
}

#[test]
fn the_musig2_tile_opens_create_with_musig2_chosen() {
    let mut app = shown();
    app.press(Action::Catalog(tile("MuSig2")));
    assert_eq!(app.screen, Screen::Create);
    assert_eq!(app.create().unwrap().kind, NewKind::MuSig);
}

#[test]
fn the_slip39_tile_opens_new_key_making_shares() {
    let mut app = shown();
    app.press(Action::Catalog(tile("SLIP-39 shares")));
    assert_eq!(app.screen, Screen::KeyGen);
    let k = app.keygen().unwrap();
    assert!(k.slip39);
    assert_eq!(
        k.open,
        Some(faraday_core::keygen::kstep::LENGTH),
        "Make SLIP-39 shares chosen, Start closed"
    );
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

#[test]
fn the_codex32_tile_opens_new_key_on_the_codex32_words_card() {
    use faraday_core::keygen::{Start, kstep};
    let mut app = faraday_core::testkit::started();
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::Catalog(tile("codex32")));
    assert_eq!(app.screen, Screen::KeyGen);
    let k = app.keygen().unwrap();
    assert_eq!(k.start, Start::Codex32);
    assert_eq!(k.open, Some(kstep::TYPED), "Start closed, on the next card");
    assert_eq!(
        faraday_core::testkit::entry(&app).form,
        faraday_core::forms::Form::Codex32
    );
}

#[test]
fn the_add_a_key_tile_opens_new_key_on_start() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::Catalog(tile("Add a key")));
    assert_eq!(app.screen, Screen::KeyGen);
    assert_eq!(
        app.keygen().unwrap().open,
        Some(faraday_core::keygen::kstep::START)
    );
}
