//! The themes tour.

use crate::tour::{Tour, scroll_to};
use faraday_core::ui::{Preset, ThemeGroup};
use faraday_core::{Action, Screen};

pub(crate) fn themes_tour(t: &mut Tour) -> Result<(), String> {
    t.load_kit()?;
    for theme in Preset::ALL {
        t.press(Action::Theme(theme));
        t.press(Action::Nav(Screen::WalletsStart));
        t.shot(&format!("theme-{}-wallets", theme.id()))?;
        t.press(Action::Nav(Screen::Wallets));
        t.shot(&format!("theme-{}-list", theme.id()))?;
        t.press(Action::Nav(Screen::Settings));
        t.shot(&format!("theme-{}-settings", theme.id()))?;
    }
    // Settings' Appearance in the theme a first start is in: the tiles in
    // their groups, from the first group's first tile to the last group's
    // last, scrolled to as a person would.
    t.press(Action::Theme(Preset::default()));
    t.press(Action::Nav(Screen::Settings));
    let first = ThemeGroup::ALL[0].themes().next();
    let last = ThemeGroup::ALL[ThemeGroup::ALL.len() - 1].themes().last();
    if let (Some(first), Some(last)) = (first, last) {
        scroll_to(t, Action::Theme(first));
        t.shot("settings-appearance")?;
        scroll_to(t, Action::Theme(last));
        t.shot("settings-appearance-end")?;
    }
    Ok(())
}
