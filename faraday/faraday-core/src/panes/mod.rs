//! List and detail (`docs/LAYOUT.md` §5, §17): on expanded windows and
//! wider, Wallets, Files, Vaults and Backups show their list in a pane at
//! the start and the one chosen beside it, each scrolling on its own.
//! Settings is the other way round: its sections are a list only on a
//! compact window, each opening as its own page, and every section is on
//! one page from medium windows up. This file is the rule and the list
//! pane's state; where the panes stand and how a list item is drawn is
//! `screen.rs`. Which one is chosen is each area's own (the wallet, the
//! vault, the wallet Backups is on), but for Settings' section, which has
//! no area of its own and is kept here.

pub(crate) mod screen;

use crate::Screen;
use faraday_draw::geom::{Window, WindowClass};

/// The list pane's state: how far it is scrolled, and the Settings
/// section open on a compact window.
#[derive(Debug, Default)]
pub(crate) struct Panes {
    /// How far the list pane is scrolled, in design units; while a
    /// Settings section is open on a compact window, how far Settings'
    /// list was scrolled when it opened, to show it there again on Back.
    list_offset: f32,
    /// The Settings section open as its own page on a compact window: an
    /// index into `screens::settings`' sections; none shows the list.
    settings: Option<u8>,
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
    /// on its list.
    pub(crate) fn leave(&mut self) {
        *self = Panes::default();
    }

    /// The Settings section open as its own page, if one is.
    pub(crate) fn settings(&self) -> Option<u8> {
        self.settings
    }

    /// Settings' section `k` opened from its list, or none: the list
    /// again.
    pub(crate) fn choose_settings(&mut self, k: Option<u8>) {
        self.settings = k;
    }
}

/// Whether `screen` is a list and the one chosen from it (§5): Wallets,
/// Files, Vaults with a vault's Unlock, and Backups. A vault's
/// Contents are a list and detail of their own, and take the page.
/// A lookup.
pub(crate) fn has_list(screen: Screen) -> bool {
    matches!(
        screen,
        Screen::Wallets | Screen::Files | Screen::Vaults | Screen::Unlock | Screen::Backups
    )
}

/// Whether a window this size shows the list and the one chosen side by
/// side: expanded windows and wider (§5); a compact or medium one shows
/// the list, and the one chosen as its own page. A lookup.
pub(crate) fn side_by_side(window: Window) -> bool {
    window.class() >= WindowClass::Expanded
}

/// Whether a window this size shows Settings as a list of its sections,
/// each opening as its own page: compact windows (§17), where one card
/// at a time is what fits. Medium windows and wider show every section
/// on one page. A lookup.
pub(crate) fn settings_listed(window: Window) -> bool {
    window.class() == WindowClass::Compact
}
