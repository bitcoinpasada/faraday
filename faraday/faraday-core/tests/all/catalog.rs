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
    assert!(found("musig2").contains(&"MuSig2"));
    assert!(found("no such tool").is_empty());
}

#[test]
fn everyday_words_find_their_tools() {
    assert!(found("send").contains(&"Sign a transaction"));
    assert!(found("receive").contains(&"Check an address"));
    let recover = found("recover");
    for name in ["Restore from SLIP-39", "Restore an existing wallet"] {
        assert!(recover.contains(&name), "{name}");
    }
    let verify = found("verify");
    for name in ["Check", "Check an address", "Check a signed message"] {
        assert!(verify.contains(&name), "{name}");
    }
    assert!(found("2FA").contains(&"TOTP codes"));
    assert!(found("seed phrase").contains(&"Add a key"));
    assert!(found("send").len() < TILES.len() / 4);
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
fn a_tile_that_works_on_a_key_opens_with_no_key_loaded() {
    let mut app = shown();
    let i = tile("Child seeds and passwords");
    assert_eq!(app.tile_need(TILES[usize::from(i)].go), None);
    app.press(Action::Catalog(i));
    assert_eq!(app.screen, Screen::Bip85);
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
    app.press(Action::Catalog(tile("Restore from codex32 shares")));
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
fn the_make_a_new_key_tile_opens_new_key_on_start() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::Catalog(tile("Make a new key")));
    assert_eq!(app.screen, Screen::KeyGen);
    assert_eq!(
        app.keygen().unwrap().open,
        Some(faraday_core::keygen::kstep::START),
        "Start opens with nothing ticked"
    );
    let _ = app.frame();
    // Only the ways of making a key: a held one is Add a key's.
    app.press(Action::KStart(faraday_core::keygen::Start::Words.index()));
    assert_eq!(
        app.keygen().unwrap().open,
        Some(faraday_core::keygen::kstep::START)
    );
    app.press(Action::KStart(faraday_core::keygen::Start::Make.index()));
    assert_eq!(
        app.keygen().unwrap().start,
        faraday_core::keygen::Start::Make
    );
}

#[test]
fn the_common_group_comes_first_with_what_a_person_comes_for() {
    let first: Vec<&str> = TILES
        .iter()
        .take_while(|t| t.group == TILES[0].group)
        .map(|t| t.name)
        .collect();
    assert_eq!(
        first,
        [
            "Sign a transaction",
            "Make a new wallet",
            "Add a key",
            "Make a new key",
            "Check"
        ]
    );
    assert_eq!(TILES[0].group, "Common");
}

#[test]
fn the_add_a_key_tile_opens_the_page_that_takes_a_key_you_have() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::Catalog(tile("Add a key")));
    assert_eq!(app.screen, Screen::Entry);
}

#[test]
fn the_make_shares_and_make_parts_tiles_open_their_flows() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::Catalog(tile("Make codex32 shares")));
    assert_eq!(app.screen, Screen::KeyGen);
    assert_eq!(
        app.keygen().unwrap().start,
        faraday_core::keygen::Start::MakeCodex32
    );
    let mut app = faraday_core::testkit::started();
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::Catalog(tile("Make Seed XOR parts")));
    assert_eq!(app.screen, Screen::Convert);
}

#[test]
fn tools_has_one_tile_for_making_or_typing_a_key() {
    let mut app = shown();
    let texts = app.drawn_texts();
    assert_eq!(texts.iter().filter(|t| *t == "Make a new key").count(), 1);
    assert_eq!(texts.iter().filter(|t| *t == "Add a key").count(), 1);
}

fn type_find(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    let _ = app.frame();
}

fn shown_words(app: &mut Faraday) -> Vec<String> {
    app.drawn_texts()
        .iter()
        .map(|t| t.as_str().to_string())
        .collect()
}

fn compact() -> Faraday {
    let mut app = shown();
    app.event(Event::Display(DisplayInfo {
        width: 480,
        height: 640,
        dpi: 286,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
    app
}

/// Whether the results hold the Settings card titled `title`.
fn shows_card(app: &mut Faraday, title: &str) -> bool {
    shown_words(app).iter().any(|t| t == title)
}

#[test]
fn find_a_tool_shows_a_settings_card_for_any_word_on_it() {
    for (word, card) in [
        ("appearance", "Appearance"),
        ("contrast", "Appearance"),
        ("orange", "Appearance"),
        ("time", "Time"),
        ("about", "About"),
    ] {
        let mut app = shown();
        type_find(&mut app, word);
        assert!(shows_card(&mut app, card), "{word}");
        assert!(!app.drawn_texts().iter().any(|t| t == "No tool matches"));
    }
}

#[test]
fn a_card_in_tools_results_changes_the_same_setting_as_on_settings() {
    let mut app = shown();
    type_find(&mut app, "contrast");
    let mut high = app.theme();
    high.contrast = faraday_core::ui::Contrast::High;
    assert!(app.offers(Action::Theme(high)));
    app.press(Action::Theme(high));
    crate::common::settle(&mut app, 1);
    assert_eq!(app.theme().contrast, faraday_core::ui::Contrast::High);
    app.press(Action::Nav(Screen::Settings));
    let _ = app.frame();
    assert_eq!(app.theme().contrast, faraday_core::ui::Contrast::High);
    let mut standard = high;
    standard.contrast = faraday_core::ui::Contrast::Standard;
    assert!(
        app.offers(Action::Theme(standard)),
        "Settings shows the change"
    );
}

#[test]
fn a_card_in_the_results_stands_on_a_small_window_too() {
    let mut app = compact();
    type_find(&mut app, "text size");
    assert!(shows_card(&mut app, "Appearance"));
    let was = app.theme();
    let other = faraday_core::ui::ThemeName::ALL
        .into_iter()
        .find(|&n| n != was.name)
        .expect("more than one theme");
    let pick = faraday_core::ui::Theme::new(other, was.mode, was.contrast);
    assert!(app.offers(Action::Theme(pick)));
    app.press(Action::Theme(pick));
    crate::common::settle(&mut app, 1);
    assert_eq!(app.theme().name, other);
}

#[test]
fn a_word_nothing_has_says_no_tool_matches() {
    let mut app = shown();
    type_find(&mut app, "zzqxv");
    assert!(app.drawn_texts().iter().any(|t| t == "No tool matches"));
    assert!(!shows_card(&mut app, "Appearance"));
}

#[test]
fn tools_search_shows_a_settings_card_among_its_results_and_clearing_brings_the_tiles_back() {
    let mut app = shown();
    type_find(&mut app, "theme");
    assert!(shows_card(&mut app, "Appearance"));
    assert!(app.hit_box(Action::Catalog(0)).is_none());
    app.event(Event::Key(Key::Escape));
    let _ = app.frame();
    assert!(!shows_card(&mut app, "Appearance"));
    assert!(app.hit_box(Action::Catalog(0)).is_some());
}
