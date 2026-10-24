//! The themes tour (`docs/THEMES.md` §15, C4): each theme in Light and
//! Dark at Standard contrast, and Cobalt at Medium and High, not all
//! sixty schemes.

use crate::tour::{Tour, scroll_to};
use faraday_core::ui::{Contrast, Mode, Theme, ThemeName};
use faraday_core::{Action, Screen};

pub(crate) fn themes_tour(t: &mut Tour) -> Result<(), String> {
    t.load_kit()?;
    let standard = ThemeName::ALL.into_iter().flat_map(|name| {
        Mode::ALL
            .into_iter()
            .map(move |mode| Theme::new(name, mode, Contrast::Standard))
    });
    let cobalt = [Contrast::Medium, Contrast::High]
        .into_iter()
        .flat_map(|c| {
            Mode::ALL
                .into_iter()
                .map(move |mode| Theme::new(ThemeName::Cobalt, mode, c))
        });
    for theme in standard.chain(cobalt) {
        let slug = theme.slug();
        t.press(Action::Theme(theme));
        t.press(Action::Nav(Screen::WalletsStart));
        t.shot(&format!("theme-{slug}-wallets"))?;
        t.press(Action::Nav(Screen::Wallets));
        t.shot(&format!("theme-{slug}-list"))?;
        t.press(Action::Nav(Screen::Settings));
        t.shot(&format!("theme-{slug}-settings"))?;
    }
    // Settings' Appearance, scrolled to as a person would: in the scheme
    // a first start is in, its first chip, then its Reduce motion row;
    // then in a dark, a medium and a high contrast scheme.
    let first = Theme::default();
    t.press(Action::Theme(first));
    t.press(Action::Nav(Screen::Settings));
    scroll_to(t, Action::Theme(first));
    t.shot("settings-appearance")?;
    scroll_to(t, Action::ReduceMotion(!t.app.reduce_motion()));
    t.shot("settings-appearance-end")?;
    for theme in [
        Theme::new(ThemeName::Frost, Mode::Dark, Contrast::Standard),
        Theme::new(ThemeName::BitcoinOrange, Mode::Light, Contrast::Medium),
        Theme::new(ThemeName::Cobalt, Mode::Dark, Contrast::High),
    ] {
        t.press(Action::Theme(theme));
        t.press(Action::Nav(Screen::Settings));
        scroll_to(t, Action::Theme(theme));
        t.shot(&format!("settings-appearance-{}", theme.slug()))?;
    }
    Ok(())
}
