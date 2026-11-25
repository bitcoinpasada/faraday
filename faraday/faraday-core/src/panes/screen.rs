//! List and detail's drawing (`docs/LAYOUT.md` §5): where the list pane
//! and the detail stand in the page area, and the list pane's items,
//! Material's two-line list items.

use crate::drawing::View;
use crate::screens::hub_layout::{self, Left, Narrow, Panes, Place, Tile};
use crate::ui::{Moves, Region};
use crate::ui::{Paint, Ui};
use crate::{Action, Screen};
use faraday_draw::tokens::{COLUMN_MAX, PANE_TOP_BAR};
use faraday_ui::roles::ListItem;

/// Where the two panes stand: each one's left edge and width, and the
/// top both begin at.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Split {
    /// The list pane: its left edge and width.
    pub list: (f32, f32),
    /// The detail: its left edge and width, to the page area's right
    /// margin.
    pub detail: (f32, f32),
    /// Where both begin, under the top app bar.
    pub top: f32,
}

impl Split {
    /// The detail's reading width: its text, fields and buttons at most
    /// `COLUMN_MAX` (§14); a chart or a table takes the whole detail.
    pub(crate) fn reading(&self) -> f32 {
        self.detail.1.min(COLUMN_MAX)
    }
}

/// Whether `screen` shows its list and the one chosen side by side in
/// this window (§5). A lookup.
pub(crate) fn shown(app: &View, screen: Screen) -> bool {
    super::has_list(screen) && super::side_by_side(app.window())
}

/// List and detail's panes: the list pane at the start and the detail
/// filling the rest.
const PANES: Panes = Panes {
    left: Left::Own,
    tile: Tile::Detail,
    stacks: false,
    narrow: Narrow::OneColumn,
};

/// Whether a page area `cw` wide holds the list pane and, beside it, a
/// detail at least as wide. A chart's side sheet narrows the page area by
/// its width; where the rest is too narrow for both, the one chosen
/// stands alone in it until the sheet closes, as on a medium window (§5),
/// rather than squeezed beside the list. Arithmetic.
pub(crate) fn fits(cw: f32) -> bool {
    PANES.place(cw, false).is_some()
}

/// Where the chosen one stands alone in the page area at `x0`, `cw`
/// wide, the list pane given way ([`fits`]): the detail from margin to
/// margin. Arithmetic.
pub(crate) fn alone(x0: f32, cw: f32) -> Split {
    let (x, w) = hub_layout::detail_alone(cw);
    Split {
        list: (x0 + x, 0.0),
        detail: (x0 + x, w),
        top: PANE_TOP_BAR,
    }
}

/// Where the list pane and the detail stand in the page area at `x0`,
/// `cw` wide, as the pane container places them. Arithmetic.
pub(crate) fn split(x0: f32, cw: f32) -> Split {
    let p = PANES.place(cw, false).unwrap_or_else(|| {
        let (x, w) = hub_layout::detail_alone(cw);
        Place {
            card_x: x,
            card_w: w,
            pane_x: x,
            pane_w: 0.0,
            gap: 0.0,
        }
    });
    Split {
        list: (x0 + p.pane_x, p.pane_w),
        detail: (x0 + p.card_x, p.card_w),
        top: PANE_TOP_BAR,
    }
}

/// One item of a list pane: its headline, its supporting line, a
/// trailing word in its own colour, and what a press does.
pub(crate) struct Item<'a> {
    /// The item's name.
    pub headline: &'a str,
    /// The line under it.
    pub supporting: &'a str,
    /// A word at its end (a wallet's Can spend), in its colour.
    pub trailing: Option<(&'a str, Paint)>,
    /// What a press on it does: choose it.
    pub action: Action,
}

/// A list pane being drawn: a column `w` wide from (x, top), `h` tall,
/// scrolled by its own offset (`Slot::ListPane`). Items are drawn down
/// it; what follows them (a section's label, a button) is drawn at
/// [`ListPane::y`] and moved past with [`ListPane::advance`].
pub(crate) struct ListPane {
    x: f32,
    w: f32,
    top: f32,
    h: f32,
    y: f32,
    open: faraday_ui::Open,
}

impl ListPane {
    /// Starts a list pane at `(x, top)`, `w` wide and `h` tall, clipped
    /// to it.
    pub(crate) fn begin(app: &View, ui: &mut Ui, rect: (f32, f32, f32, f32)) -> Self {
        Self::begin_as(app, ui, rect, Moves::No)
    }

    /// [`ListPane::begin`] for a pane whose items depend on nothing but
    /// their state and the pane's offset ([`Moves::Yes`]).
    pub(crate) fn begin_moving(app: &View, ui: &mut Ui, rect: (f32, f32, f32, f32)) -> Self {
        Self::begin_as(app, ui, rect, Moves::Yes)
    }

    fn begin_as(
        app: &View,
        ui: &mut Ui,
        (x, top, w, h): (f32, f32, f32, f32),
        moves: Moves,
    ) -> Self {
        let clip = ui.rect(x, top, w, h);
        let offset = app.scroll.panes().list_offset();
        let open = ui.region_open(
            Region::beside(crate::ui::Slot::ListPane, clip, offset)
                .from_zero(offset)
                .moves(moves),
        );
        ListPane {
            x,
            w,
            top,
            h,
            y: top,
            open,
        }
    }

    /// The pane's left edge, its width and where the next thing goes.
    pub(crate) fn at(&self) -> (f32, f32, f32) {
        (self.x, self.w, self.y)
    }

    /// Moves the next thing `dy` down.
    pub(crate) fn advance(&mut self, dy: f32) {
        self.y += dy;
    }

    /// A list item, two lines, or one where it has no supporting line;
    /// `chosen` is the one the detail shows, in Material's selected state
    /// (secondary container).
    pub(crate) fn item(&mut self, ui: &mut Ui, it: &Item, chosen: bool) {
        self.y += list_item(ui, (self.x, self.y, self.w), it, chosen);
    }

    /// A list item as [`ListPane::item`] with a chevron at its end: it
    /// opens a page.
    pub(crate) fn item_opens(&mut self, ui: &mut Ui, it: &Item) {
        let item = ListItem {
            headline: it.headline,
            supporting: it.supporting,
            trailing: it.trailing,
            action: it.action,
        };
        self.y += ui.list_item_opens((self.x, self.y, self.w), &item);
    }

    /// Ends the pane: its clip goes, and its scroll is said, as far as
    /// what was drawn in it goes.
    pub(crate) fn end(self, ui: &mut Ui) {
        let content = self.y - self.top;
        ui.region_close(self.open, content - self.h);
    }
}

/// A list item from (x, y), `w` wide, of one, two or three lines by its
/// supporting line (`Ui::list_item`); `chosen` in Material's selected
/// state (secondary container). Returns its height.
pub(crate) fn list_item(ui: &mut Ui, at: (f32, f32, f32), it: &Item, chosen: bool) -> f32 {
    let item = ListItem {
        headline: it.headline,
        supporting: it.supporting,
        trailing: it.trailing,
        action: it.action,
    };
    ui.list_item(at, &item, chosen)
}
