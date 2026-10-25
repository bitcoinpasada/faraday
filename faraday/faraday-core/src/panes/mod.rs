//! List and detail (`docs/LAYOUT.md` §5, §17): on expanded windows and
//! wider, Wallets, Files, Vaults, Backups and Settings show their list in
//! a pane at the start and the one chosen beside it, each scrolling on its
//! own. This file is the rule and the list pane's state; where the panes
//! stand and how a list item is drawn is `screen.rs`. Which one is chosen
//! is each area's own (the wallet, the vault, the wallet Backups is on),
//! but for Settings' section, which has no area of its own and is kept
//! here.

pub(crate) mod screen;

use crate::Screen;
use faraday_draw::geom::{Window, WindowClass};

/// The list pane's state: how far it is scrolled, and the Settings
/// section its detail shows.
#[derive(Debug, Default)]
pub(crate) struct Panes {
    /// How far the list pane is scrolled, in design units.
    list_offset: f32,
    /// Settings' section on the detail's side: an index into
    /// `screens::settings`' sections, the first when none was chosen.
    settings: u8,
}

impl Panes {
    /// How far the list pane is scrolled.
    pub(crate) fn list_offset(&self) -> f32 {
        self.list_offset
    }

    /// The list pane's offset, for the wheel, a trackpad and a finger to
    /// move.
    pub(crate) fn list_offset_mut(&mut self) -> &mut f32 {
        &mut self.list_offset
    }

    /// Another destination opened: its list from the top, and Settings
    /// on its first section.
    pub(crate) fn leave(&mut self) {
        *self = Panes::default();
    }

    /// The Settings section the detail shows.
    pub(crate) fn settings(&self) -> u8 {
        self.settings
    }

    /// Settings' section `k` chosen in the list pane.
    pub(crate) fn choose_settings(&mut self, k: u8) {
        self.settings = k;
    }
}

/// Whether `screen` is a list and the one chosen from it (§5): Wallets,
/// Files, Vaults with a vault's Unlock, Backups and Settings. A vault's
/// Contents are a list and detail of their own, and take the page.
/// A lookup.
pub(crate) fn has_list(screen: Screen) -> bool {
    matches!(
        screen,
        Screen::Wallets
            | Screen::Files
            | Screen::Vaults
            | Screen::Unlock
            | Screen::Backups
            | Screen::Settings
    )
}

/// Whether a window this size shows the list and the one chosen side by
/// side: expanded windows and wider (§5); a compact or medium one shows
/// the list, and the one chosen as its own page. A lookup.
pub(crate) fn side_by_side(window: Window) -> bool {
    window.class() >= WindowClass::Expanded
}
