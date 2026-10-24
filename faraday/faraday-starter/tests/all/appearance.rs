//! The Starter's Appearance (`docs/THEMES.md` §14): the ⋮ menu opens a
//! page with Faraday's Theme, Mode and Contrast; each choice changes
//! what the pages are drawn in, the held desktop app's Transfer too; a
//! new Starter opens in Cobalt, Light, Standard, since it keeps nothing.

use faraday_shell_api::App;
use faraday_starter::api::StarterEvent;
use faraday_starter::{Page, Starter, StarterAction};
use faraday_ui::{Contrast, Mode, Theme, ThemeName};

use super::starter::{display, linux, payload};

fn shown() -> Starter {
    let mut s = Starter::new();
    s.event(display(1280, 800, 160));
    s.starter_event(StarterEvent::Computer(linux(true)));
    s.starter_event(StarterEvent::Payload(Some(payload("0.2.0"))));
    s
}

/// The page opened from the ⋮ menu.
fn appearance(s: &mut Starter) {
    s.press(StarterAction::Menu);
    assert!(
        s.drawn_texts().iter().any(|t| t == "Appearance"),
        "in the menu"
    );
    s.press(StarterAction::Appearance);
    assert_eq!(s.page, Page::Appearance);
}

/// The pixel in the window's top right corner, the page under the bar.
fn corner(s: &mut Starter) -> [u8; 3] {
    let f = s.frame();
    let i = (4 * usize::from(f.width) - 8) * 4;
    [f.rgba[i], f.rgba[i + 1], f.rgba[i + 2]]
}

#[test]
fn a_new_starter_opens_in_cobalt_light_at_standard_contrast() {
    assert_eq!(
        shown().theme(),
        Theme::new(ThemeName::Cobalt, Mode::Light, Contrast::Standard)
    );
}

#[test]
fn the_page_offers_every_theme_mode_and_contrast_of_the_scheme_on_screen() {
    let mut s = shown();
    appearance(&mut s);
    let _ = s.drawn_texts();
    let now = s.theme();
    for name in ThemeName::ALL {
        let pick = StarterAction::Theme(Theme { name, ..now });
        assert!(s.offers_action(pick), "{name:?}");
    }
    for mode in Mode::ALL {
        assert!(s.offers_action(StarterAction::Theme(Theme { mode, ..now })));
    }
    for contrast in Contrast::ALL {
        let pick = StarterAction::Theme(Theme { contrast, ..now });
        assert!(s.offers_action(pick), "{contrast:?}");
    }
}

#[test]
fn each_choice_changes_what_is_drawn_and_back_leaves_it() {
    let mut s = shown();
    appearance(&mut s);
    let light = corner(&mut s);
    let dark = Theme::new(ThemeName::Frost, Mode::Dark, Contrast::High);
    for step in [
        Theme {
            name: ThemeName::Frost,
            ..s.theme()
        },
        Theme::new(ThemeName::Frost, Mode::Dark, Contrast::Standard),
        dark,
    ] {
        let before = s.frame().rgba.to_vec();
        s.press(StarterAction::Theme(step));
        assert_eq!(s.theme(), step);
        assert!(
            s.frame().rgba != before.as_slice(),
            "{step:?} draws as before"
        );
    }
    let page = corner(&mut s);
    assert!(page.iter().all(|&v| v < 64), "dark: {page:?}");
    assert_ne!(page, light);
    s.press(StarterAction::Back);
    assert_eq!(s.page, Page::Home);
    assert_eq!(s.theme(), dark, "and kept while the Starter runs");
}

#[test]
fn transfer_is_drawn_in_the_starters_scheme() {
    let mut s = shown();
    appearance(&mut s);
    let dark = Theme::new(ThemeName::Rose, Mode::Dark, Contrast::Medium);
    s.press(StarterAction::Theme(dark));
    s.press(StarterAction::ToolsPage);
    s.press(StarterAction::Transfer);
    assert!(s.showing_held());
    assert_eq!(s.held().theme(), dark);
    // The held app's page, under the Starter's bar, is dark too.
    let f = s.frame();
    let i = (400 * usize::from(f.width) + 1200) * 4;
    let px = [f.rgba[i], f.rgba[i + 1], f.rgba[i + 2]];
    assert!(px.iter().all(|&v| v < 64), "{px:?}");
}
