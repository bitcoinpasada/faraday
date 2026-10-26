//! List and detail's drawing (`docs/LAYOUT.md` §5): where the list pane
//! and the detail stand in the page area, and the list pane's items,
//! Material's two-line list items.

use crate::drawing::View;
use crate::ui::pal::*;
use crate::ui::{Paint, Ui, W};
use crate::{Action, Screen};
use faraday_draw::tokens::{
    COLUMN_MAX, GAP, LIST_1, LIST_2, LIST_HEADLINE, LIST_HEADLINE_Y, LIST_PAD, LIST_PANE,
    LIST_SUPPORTING, LIST_SUPPORTING_Y, LIST_TRAILING, MARGIN_WIDE, PANE_TOP, PANE_TOP_BAR,
    SHAPE_L, SPACER, STATE_PRESS,
};

/// Where the two panes stand: each one's left edge and width, and the
/// top both begin at.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Split {
    /// The list pane: its left edge and width.
    pub list: (f32, f32),
    /// The detail: its left edge and width, to the page area's right
    /// margin.
    pub detail: (f32, f32),
    /// Where both begin: under a wide page's title, or under the top app
    /// bar on the rail's windows.
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

/// Where the list pane and the detail stand in the page area at `x0`,
/// `cw` wide: the list `LIST_PANE` wide at the start, the `SPACER`, and
/// the detail to the area's right margin. Arithmetic.
pub(crate) fn split(ui: &Ui, x0: f32, cw: f32) -> Split {
    let top = if ui.compact { PANE_TOP_BAR } else { PANE_TOP };
    let lx = x0 + MARGIN_WIDE;
    let dx = lx + LIST_PANE + SPACER;
    Split {
        list: (lx, LIST_PANE),
        detail: (dx, (x0 + cw - MARGIN_WIDE - dx).max(0.0)),
        top,
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
    offset: f32,
    clip: faraday_draw::Rect,
}

impl ListPane {
    /// Starts a list pane at `(x, top)`, `w` wide and `h` tall, clipped
    /// to it.
    pub(crate) fn begin(app: &View, ui: &mut Ui, (x, top, w, h): (f32, f32, f32, f32)) -> Self {
        let clip = ui.rect(x, top, w, h);
        ui.c.push_clip(clip);
        let offset = app.scroll.panes().list_offset();
        ListPane {
            x,
            w,
            top,
            h,
            y: top - offset,
            offset,
            clip,
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

    /// Ends the pane: its clip goes, and its scroll is said, as far as
    /// what was drawn in it goes.
    pub(crate) fn end(self, ui: &mut Ui) {
        ui.c.pop_clip();
        let content = self.y + self.offset - self.top;
        ui.report_scroll_beside(
            crate::ui::Slot::ListPane,
            self.clip,
            content - self.h,
            self.offset,
        );
    }
}

/// A list item from (x, y), `w` wide, two lines, or one where it has no
/// supporting line; `chosen` in Material's selected state (secondary
/// container). Returns its height.
pub(crate) fn list_item(ui: &mut Ui, (x, y, w): (f32, f32, f32), it: &Item, chosen: bool) -> f32 {
    let one = it.supporting.is_empty();
    let ih = if one { LIST_1 } else { LIST_2 };
    if !ui.shows(y, ih) {
        return ih;
    }
    if chosen {
        ui.fill(x, y, w, ih, SHAPE_L, selected_or(true, Role::Surface));
    }
    if ui.is_pressed(it.action) {
        ui.wash(x, y, w, ih, SHAPE_L, Role::OnSurface.at(STATE_PRESS));
    }
    let ink = on_selected(chosen, Role::OnSurface);
    if one {
        let name = ui.fit(LIST_HEADLINE, W::S, it.headline, w - 2.0 * LIST_PAD);
        ui.text_mid(x + LIST_PAD, y, ih, LIST_HEADLINE, W::S, ink, &name);
        ui.hit(x, y, w, ih, it.action);
        return ih;
    }
    let inner = w - 2.0 * LIST_PAD;
    let mut room = inner;
    if let Some((t, tone)) = it.trailing {
        let tw = ui.measure(LIST_TRAILING, W::S, t);
        let tone = if chosen {
            on_selected(true, Role::OnSurface)
        } else {
            tone
        };
        ui.text_right(
            x + w - LIST_PAD,
            y + LIST_HEADLINE_Y,
            LIST_HEADLINE,
            LIST_TRAILING,
            W::S,
            tone,
            t,
        );
        room -= tw + GAP;
    }
    let name = ui.fit(LIST_HEADLINE, W::S, it.headline, room);
    ui.text(
        x + LIST_PAD,
        y + LIST_HEADLINE_Y,
        LIST_HEADLINE,
        W::S,
        ink,
        &name,
    );
    let line = ui.fit(LIST_SUPPORTING, W::R, it.supporting, inner);
    let muted = on_selected(chosen, Role::OnSurfaceVariant);
    ui.text(
        x + LIST_PAD,
        y + LIST_SUPPORTING_Y,
        LIST_SUPPORTING,
        W::R,
        muted,
        &line,
    );
    ui.hit(x, y, w, LIST_2, it.action);
    ih
}
