//! The themes tour (`docs/THEMES.md` §15, C4): each theme in Light and
//! Dark at Standard contrast, and Cobalt at Medium and High, not all
//! sixty schemes. On each: Home, Wallets, the backup board, a codex32
//! worksheet and Settings' Appearance.

use crate::tour::{Tour, scroll_to};
use faraday_core::ui::{Contrast, Mode, Theme, ThemeName};
use faraday_core::{Action, Screen};

/// The schemes the tour draws: each theme in Light and Dark at
/// Standard, and Cobalt at Medium and High.
fn schemes() -> impl Iterator<Item = Theme> {
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
    standard.chain(cobalt)
}

pub(crate) fn themes_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::board::{BoardPress as B, Chip, Thing, Zone};
    t.load_kit()?;
    // The screens each scheme draws, set up once: the backup board of
    // Savings with its keys typed and a seed placed.
    for n in 1..3 {
        t.press(Action::Entry(None));
        t.type_key(n);
    }
    let savings = t
        .app
        .session
        .wallets
        .iter()
        .position(|w| w.name == "Savings")
        .ok_or("no Savings")?;
    t.press(Action::Backup(savings));
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    t.press(Action::Board(B::Chip(Chip::List(Thing::Seed(0)))));
    t.press(Action::Board(B::Zone(Zone::Paper(1))));
    t.press(Action::Board(B::LetGo));
    t.press(Action::Board(B::Chip(Chip::List(Thing::Seed(1)))));
    for theme in schemes() {
        let slug = theme.slug();
        t.press(Action::Theme(theme));
        t.press(Action::Nav(Screen::Home));
        t.shot(&format!("theme-{slug}-home"))?;
        t.press(Action::Nav(Screen::Wallets));
        t.shot(&format!("theme-{slug}-wallets"))?;
        t.press(Action::Nav(Screen::Backup));
        t.shot(&format!("theme-{slug}-board"))?;
        t.press(Action::Nav(Screen::Settings));
        // Settings is a list of its sections; Appearance is the fifth.
        t.press(Action::SettingsSection(Some(4)));
        scroll_to(t, Action::Theme(theme));
        t.shot(&format!("theme-{slug}-appearance"))?;
    }
    // The codex32 worksheet part way, which replaces the backup flow
    // under way, in each scheme.
    crate::tours::codex32::worksheet(t);
    for theme in schemes() {
        t.press(Action::Theme(theme));
        t.shot(&format!("theme-{}-codex32", theme.slug()))?;
    }
    // Settings' Appearance, scrolled to as a person would: in the scheme
    // a first start is in, its first chip, then its Reduce motion row;
    // then in a dark, a medium and a high contrast scheme.
    let first = Theme::default();
    t.press(Action::Theme(first));
    t.press(Action::Nav(Screen::Settings));
    // Settings is a list of its sections; Appearance is the fifth.
    t.press(Action::SettingsSection(Some(4)));
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
        // Settings is a list of its sections; Appearance is the fifth.
        t.press(Action::SettingsSection(Some(4)));
        scroll_to(t, Action::Theme(theme));
        t.shot(&format!("settings-appearance-{}", theme.slug()))?;
    }
    Ok(())
}
