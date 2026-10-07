//! §4.1 Navigation: the hub, the app bar, menus, the sidebar and the
//! pager.

use alloc::string::String;
use alloc::vec::Vec;

use crate::geom::{Metrics, SizeClass};
use crate::layout::{Align, Id, Justify, Node};
use crate::organisms::{self, SidebarItem};
use crate::text::{Font, TextAlign};
use crate::tokens;
use crate::widgets::{Icon, Tone, Widget};

pub use crate::organisms::{app_bar, bar_action};

/// §4.1 Menu row: "Full-width row: optional icon, label, chevron. With
/// a value: the label above and the value below, both left-aligned,
/// chevron right (label above, value below, everywhere a row carries a
/// value)."
///
/// One item per row on every class (§2.4). A row with a value is the
/// two-line row of [`tokens::stacked_row`], the same height as a value
/// row, a key row and a reference row, so a menu of mixed kinds reads as
/// one list.
pub fn menu_row(
    class: SizeClass,
    id: Id,
    icon: Option<Icon>,
    label: impl Into<String>,
    value: Option<String>,
    tone: Tone,
) -> Node {
    let mut row = Widget::list_row(Some(id), label, value.clone(), None);
    if let Some(icon) = icon {
        row = row.with_icon(icon);
    }
    Node::widget(match value {
        Some(_) => row
            .with_height(tokens::stacked_row(class))
            .with_value_below(false, tone),
        None => row.with_height(tokens::menu_row(class)),
    })
}

/// §4.1 Menu row whose value is read character by character: the
/// wallets list, where a wallet with no name of its own is named by what
/// it is made of, "73c5da0a · SegWit" (§16.126). Everything else
/// about the row is [`menu_row`]'s.
pub fn menu_row_mono(
    class: SizeClass,
    id: Id,
    icon: Option<Icon>,
    label: impl Into<String>,
    value: String,
) -> Node {
    let mut row = Widget::list_row(Some(id), label, Some(value), None)
        .with_height(tokens::stacked_row(class))
        .with_value_below(true, Tone::Text);
    if let Some(icon) = icon {
        row = row.with_icon(icon);
    }
    Node::widget(row)
}

/// §4.11 Dimmed row: "a dead row is a two-line row, like a row with a
/// value" — the reason under the label, on every class, in two or three
/// words: "no camera", "needs Tier B".
///
/// A menu row with no hit target and the reason where its value would
/// be, so a menu keeps one row shape whether or not a row can be opened.
/// `reason` of `None` is a row dimmed because the feature is not built:
/// nothing is written under it, and the row is one line at the class's
/// own menu height.
pub fn dimmed_row(
    class: SizeClass,
    icon: Option<Icon>,
    label: impl Into<String>,
    reason: Option<String>,
) -> Node {
    let row = Widget::list_row(None, label, reason.clone(), None).with_enabled(false);
    let row = match icon {
        Some(icon) => row.with_icon(icon),
        None => row,
    };
    Node::widget(match reason {
        Some(_) => row
            .with_height(tokens::stacked_row(class))
            .with_value_below(false, Tone::Muted),
        None => row.with_height(tokens::menu_row(class)),
    })
}

/// One tile of the [`hub`] grid.
pub struct Tile {
    /// Application id.
    pub id: Id,
    /// The area's mark.
    pub icon: Icon,
    /// The area's name.
    pub label: String,
    /// Whether the area can be opened.
    pub enabled: bool,
    /// A count in the tile's corner, as Keys carries.
    pub badge: Option<String>,
}

/// §4.1 Hub: "Home: status line, then a 2-column grid of six equal
/// tiles. Never scrolls. A tile carries a count badge (Keys); a dead tile
/// (Learn) is dimmed and carries nothing else."
///
/// The columns and the tile's tallest aspect are tokens, so a `wide`
/// pane gets three columns and no tile becomes a column of its own.
pub fn hub(m: &Metrics, tiles: Vec<Tile>) -> Node {
    organisms::tile_grid(
        m,
        tiles
            .into_iter()
            .map(|t| organisms::TileSpec {
                id: t.id,
                icon: t.icon,
                label: t.label,
                enabled: t.enabled,
                badge: t.badge,
            })
            .collect(),
    )
}

/// §4.1 App bar, secret screens: the eye, and the ring it becomes while
/// the 30 s reveal runs (§4.10: "while it does, the eye is drawn as a
/// ring that empties clockwise, which is the only countdown").
///
/// `remaining` is the share of the reveal still to run, which the caller
/// already holds; `None` is the plain eye of a masked screen. There is
/// no "Showing for N s" anywhere: the ring is the whole of it.
pub fn bar_eye(id: Id, remaining: Option<f32>) -> Node {
    match remaining {
        Some(fraction) => Node::widget(Widget::Ring {
            id,
            icon: Icon::Eye,
            fraction,
        }),
        None => organisms::bar_action(id, Icon::Eye),
    }
}

/// §4.1 App bar: the info button, which opens the Learn page for what
/// the screen is for (`docs/PLANNING.md` §16.105). It sits in the same
/// trailing slot as the eye and is drawn only where the eye is not, so
/// a screen carries one control there or none.
pub fn bar_info(id: Id) -> Node {
    organisms::bar_action(id, Icon::Info)
}

/// §4.1 Status line: "56 dp on Home: tier badge, network badge, then at
/// the right the lock icon button (tap locks now). Nothing else; no
/// countdown." Settings is a tile of the grid, not a glyph here.
pub fn status_line(badges: Node, lock: Option<Id>) -> Node {
    let mut row = Node::row()
        .height(tokens::STATUS_LINE)
        .padding(crate::geom::Edges::symmetric(tokens::PAD, 0.0))
        .gap(tokens::GAP_SMALL)
        .align(Align::Center)
        .child(badges)
        .child(Node::spacer());
    if let Some(id) = lock {
        row = row.child(organisms::bar_action(id, Icon::Lock));
    }
    row
}

/// §4.1 Sidebar (`wide`): "One row per area, the current one in the
/// accent. Dimmed and inert during a wizard, the scanner, the lock
/// screen and the hold screens."
pub fn sidebar(items: &[SidebarItem], dimmed: bool) -> Node {
    organisms::sidebar(items, dimmed)
}

/// §4.1 Pager: "`‹ label ›` in the bottom action slot. One grammar: the
/// page's name ('Code', 'Receive 2', 'Output 2 of 2')."
///
/// The one grammar is the caller's `label`; the component fixes the
/// shape. Either arrow goes dead at its end of the run, so the pager
/// says where in the run the page is without a step indicator.
pub fn pager(
    class: SizeClass,
    prev: Id,
    next: Id,
    label: impl Into<String>,
    at_start: bool,
    at_end: bool,
) -> Node {
    pager_at(
        class,
        prev,
        next,
        label,
        at_start,
        at_end,
        Font::semibold(tokens::LABEL),
    )
}

/// The same pager naming its page in the mono face: a page named by a
/// string that is read character by character, which is a word from a
/// list (§16.126).
pub fn pager_mono(
    class: SizeClass,
    prev: Id,
    next: Id,
    label: impl Into<String>,
    at_start: bool,
    at_end: bool,
) -> Node {
    pager_at(
        class,
        prev,
        next,
        label,
        at_start,
        at_end,
        Font::mono(tokens::LABEL),
    )
}

#[allow(clippy::too_many_arguments)]
fn pager_at(
    class: SizeClass,
    prev: Id,
    next: Id,
    label: impl Into<String>,
    at_start: bool,
    at_end: bool,
    font: Font,
) -> Node {
    let arrow = |id: Id, icon: Icon, off: bool| {
        Node::widget(Widget::IconButton {
            id,
            icon,
            size_dp: tokens::ICON_SMALL,
            tone: Tone::Primary,
            enabled: !off,
        })
    };
    Node::row()
        .align(Align::Center)
        .justify(Justify::Center)
        .height(tokens::pager_row(class))
        .child(arrow(prev, Icon::ChevronLeft, at_start))
        .child(
            Node::widget(Widget::text(label, font, Tone::Muted).align(TextAlign::Center))
                .weight(1.0),
        )
        .child(arrow(next, Icon::ChevronRight, at_end))
}
