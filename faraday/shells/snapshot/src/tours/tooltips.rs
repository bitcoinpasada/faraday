//! The tooltips tour (`docs/DESIGN.md` §4.12, Tooltip): the plain
//! tooltip on the top app bar's ?, on the backup board's ×, and on the
//! codex32 sheets' eye, with the pointer resting on each, in Cobalt Light
//! and Dark.

use crate::tour::{Tour, scroll_to};
use faraday_core::board::{BoardPress as B, Chip, Thing, Zone};
use faraday_core::codex32::C32;
use faraday_core::ui::{Contrast, Mode, Theme, ThemeName};
use faraday_core::{Action, Screen};
use faraday_shell_api::{App, Event};

/// The pointer rests on `action`: its tooltip is up in the next shot.
fn rest_on(t: &mut Tour, action: Action) -> Result<(), String> {
    scroll_to(t, action);
    let (x, y) = t
        .app
        .where_offered(action)
        .ok_or_else(|| format!("{action:?} is not offered"))?;
    t.app.event(Event::Hover { x, y });
    t.tick();
    Ok(())
}

pub(crate) fn tooltips_tour(t: &mut Tour) -> Result<(), String> {
    t.load_kit()?;
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
    for mode in Mode::ALL {
        let theme = Theme::new(ThemeName::Cobalt, mode, Contrast::Standard);
        let slug = theme.slug();
        t.press(Action::Theme(theme));
        // Home offers no cryptographic action, so on a small panel the ? is
        // on its bar, not in the ⋮ with Cryptography.
        t.press(Action::Nav(Screen::Home));
        rest_on(t, Action::Learn)?;
        t.shot(&format!("tip-{slug}-learn"))?;
        t.app.event(Event::HoverEnd);
        t.press(Action::Nav(Screen::Backup));
        rest_on(t, Action::Board(B::Off(Zone::Paper(1), Thing::Seed(0))))?;
        t.shot(&format!("tip-{slug}-board-cross"))?;
        t.app.event(Event::HoverEnd);
    }
    crate::tours::codex32::worksheet(t);
    for mode in Mode::ALL {
        let theme = Theme::new(ThemeName::Cobalt, mode, Contrast::Standard);
        t.press(Action::Theme(theme));
        rest_on(t, Action::Codex32(C32::Eye))?;
        t.shot(&format!("tip-{}-eye", theme.slug()))?;
        t.app.event(Event::HoverEnd);
    }
    Ok(())
}
