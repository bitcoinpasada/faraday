//! Navigation by window class (`docs/LAYOUT.md` §3): which navigation a
//! window gets, the destinations it offers, and (in `screen.rs`) the
//! rail, the bar, the top app bar and the status chips as Faraday draws
//! them with `faraday_ui::nav`. The expanded rail is the sidebar
//! (`screens/sidebar.rs`).

pub(crate) mod screen;

use crate::Screen;
use faraday_draw::geom::{Window, WindowClass};

/// A press on a top app bar's ⋮ menu (`docs/LAYOUT.md` §3, DESIGN §4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuPress {
    /// Opens it: with `None` the bar's windows' menu (Settings, Power
    /// off); with what a compact bar had no room for, that bar's overflow.
    Open(Option<Overflow>),
    /// A screen chosen in it: it closes and goes there.
    Go(Screen),
    /// A tool of the bar chosen in its overflow: it closes and the tool
    /// does what its button on the bar does.
    Tool(BarTool),
}

/// The tools a compact top app bar puts in its overflow menu when they
/// leave no room on it, in the order they go there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarTool {
    /// The ?: the page's Learn.
    Help,
    /// The keyboard button.
    Keyboard,
    /// Make codex32 shares' eye.
    Eye,
}

/// Which of a compact bar's tools are in its overflow menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Overflow {
    /// The ?.
    pub help: bool,
    /// The keyboard button.
    pub keyboard: bool,
    /// The keyboard was up when the menu opened (the menu, a sheet,
    /// takes typing from the page while open): its row puts it away.
    pub keyboard_up: bool,
    /// The eye.
    pub eye: bool,
}

impl Overflow {
    /// Whether it holds anything: the bar then shows its ⋮.
    pub fn any(self) -> bool {
        self.help || self.keyboard || self.eye
    }
}

/// The navigation a window gets (`docs/LAYOUT.md` §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nav {
    /// Compact and short (the Pi's panel): Home is the menu, and every
    /// other page has the 64 dp top app bar with its way back.
    Menu,
    /// Compact, not short: the navigation bar at the foot and the top
    /// app bar over one-column pages.
    Bar,
    /// Medium and expanded, and any low window that is not compact and
    /// short (the 7-inch panel): the navigation rail at the left and the
    /// top app bar over one-column pages beside it.
    Rail,
    /// Large and extra-large, not low: the expanded rail (the sidebar)
    /// beside the wide pages.
    ExpandedRail,
}

impl Nav {
    /// The navigation of `window`. Held inside the Starter, the screens
    /// keep their one-column rule (`compact`, under 1280 dp): the menu,
    /// or the wide pages under the holder's own top, which the frame
    /// draws in place of the expanded rail. A lookup.
    pub fn of(window: &Window, held: bool, compact: bool) -> Nav {
        if held {
            return if compact {
                Nav::Menu
            } else {
                Nav::ExpandedRail
            };
        }
        let class = window.class();
        if class == WindowClass::Compact && window.short() {
            Nav::Menu
        } else if class >= WindowClass::Large && !window.low() {
            Nav::ExpandedRail
        } else if class == WindowClass::Compact && !window.low() {
            Nav::Bar
        } else {
            Nav::Rail
        }
    }

    /// The rail or the bar: one-column pages under the top app bar.
    pub fn beside(self) -> bool {
        matches!(self, Nav::Bar | Nav::Rail)
    }
}

/// The destination screen `s` belongs to, the one lit while it shows:
/// a wallet's pages and flows light Wallets, a vault's light Vaults. A
/// lookup.
pub(crate) fn destination_of(s: Screen) -> Screen {
    match s {
        Screen::Wallets
        | Screen::Family
        | Screen::Vanity
        | Screen::Spend
        | Screen::Entry
        | Screen::KeyGen
        | Screen::Bip85
        | Screen::Silent
        | Screen::Explore
        | Screen::Lightning
        | Screen::Tools
        | Screen::Backup
        | Screen::Backups
        | Screen::Message
        | Screen::CheckMessage
        | Screen::Create
        | Screen::Restore => Screen::WalletsStart,
        Screen::CreateVault | Screen::Unlock | Screen::VaultContents => Screen::Vaults,
        s => s,
    }
}
