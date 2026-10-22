//! The themes tour.

use crate::tour::Tour;
use faraday_core::{Action, Screen};

pub(crate) fn themes_tour(t: &mut Tour) -> Result<(), String> {
    t.load_kit()?;
    for theme in faraday_core::ui::Theme::ALL {
        t.press(Action::Theme(theme));
        t.press(Action::Nav(Screen::WalletsStart));
        t.shot(&format!("theme-{}-wallets", theme.id()))?;
        t.press(Action::Nav(Screen::Wallets));
        t.shot(&format!("theme-{}-list", theme.id()))?;
        t.press(Action::Nav(Screen::Settings));
        t.shot(&format!("theme-{}-settings", theme.id()))?;
    }
    Ok(())
}
