//! the guide's supporting pane (`docs/LAYOUT.md` §6): on an extra-large
//! window, the guide stands in a pane at the right, beside the page
//! it explains, instead of the sheet over it. The pane holds its place
//! while the page scrolls beside it and scrolls within itself where it
//! is taller than the window. It stays open across screens and flows,
//! and its header row holds the close button, which puts it away
//! (Material 3 Side sheets, Icon buttons), so the
//! tabs under it run the pane's whole width; its tabs, page and contents
//! are the sheet's (`sheets::guide_body`). Which page it shows is kept
//! on the guide's state (`Faraday::guide_place`); drawing reads it.

use crate::ui::Region;
use crate::ui::Shape;
use faraday_draw::tokens as T;
use faraday_draw::widgets::Icon;

use super::sheets::{GuideLook, guide_body};
use crate::Action;
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Slot, Ui};

/// Draws the guide's supporting pane for the page area at `x0`, `cw` wide,
/// in a window `h` tall, at the one place every supporting pane stands
/// (`hub_layout::guide_place`), with the page's height where it was said.
/// Reads the state.
pub(super) fn draw(app: &View, ui: &mut Ui, (x0, cw, h): (f32, f32, f32)) {
    let (x, mut y, w, mut view) = super::hub_layout::guide_place((x0, cw, h));
    // Beside a pair or a page of one column the pane is as tall as the
    // page, tops and feet level; beside a page that says no height it has
    // the window's (`docs/WORKSPACE.md` §7.4). Its arrow lets it down to
    // the window's height and folds it back.
    let full = (y, view);
    let shorter = ui.pair_span.map(|(top, foot)| {
        let end = foot.min(h - T::MARGIN_WIDE);
        let top = top.max(0.0);
        (top, (end - top).max(T::TOUCH))
    });
    let tall = app.guide_pane_tall();
    if let Some(span) = shorter.filter(|_| !tall) {
        (y, view) = span;
    }
    ui.guide_drawn = Some((x, ui.oy + y, w, view));
    // Nothing of the page under the pane takes a press.
    let area = ui.rect(x, y, w, view);
    ui.drop_presses_inside(area);
    ui.fill(x, y, w, view, T::SHAPE_L, Role::SurfaceContainer);
    let look = GuideLook {
        pad: T::GUIDE_PANE_PAD,
        // The tabs run the pane's whole width, as Material's tabs run
        // their container's.
        tabs_edge: 0.0,
        tabs_top: T::GUIDE_PANE_TABS_TOP,
        after_tabs: T::PAD,
        rule: T::GAP,
        inset: 0.0,
        text_top: T::GAP,
        foot: super::sheets::guide_foot(ui),
        head: T::TITLE_L,
        heading: T::TITLE_S,
        body: T::BODY_M,
        after: [T::GAP, T::GAP_SMALL, T::GUIDE_PANE_PARA, T::GAP],
        clip_hits: true,
        searching: false,
    };
    let (clip, max) = guide_body(app, ui, &look, (x, y, w, view));
    let close_x = x + w - T::GAP - T::TOUCH;
    close_button(ui, close_x, y + T::GUIDE_PANE_CLOSE_TOP);
    // The arrow stands only where the page makes the pane shorter than
    // the window's.
    if shorter.is_some_and(|(_, span)| span < full.1) {
        tall_button(ui, close_x - T::TOUCH, y + T::GUIDE_PANE_CLOSE_TOP, tall);
    }
    // Its edges fade into the pane's own container.
    let region = Region::beside(Slot::GuidePane, clip, app.guide().scroll)
        .on(Role::SurfaceContainer)
        .bar_shown();
    ui.region_reported(region, max);
}

/// The pane's close button in a `TOUCH` square at (x, y): Material's
/// standard icon button ([`faraday_ui::nav::close_button`]).
fn close_button(ui: &mut Ui, x: f32, y: f32) {
    faraday_ui::nav::close_button(ui, x, y, Action::GuideClose, EN.guide_close);
}

/// The pane's arrow in a `TOUCH` square at (x, y), beside its close
/// button: a standard icon button (Material 3 Icon buttons), pointing down
/// to let the pane down to the window's height, up to fold it back.
fn tall_button(ui: &mut Ui, x: f32, y: f32, tall: bool) {
    let edge = (T::TOUCH - T::ICON_BUTTON) / 2.0;
    let (bx, by) = (x + edge, y + edge);
    let button = Shape::new(
        bx,
        by,
        T::ICON_BUTTON,
        T::ICON_BUTTON,
        T::shape_full(T::ICON_BUTTON),
    );
    let role = Role::OnSurfaceVariant;
    ui.press_to(
        button,
        (x, y, T::TOUCH, T::TOUCH),
        role.paint(),
        Action::GuideTall,
    );
    let (icon, tip) = if tall {
        (Icon::ChevronUp, EN.guide_collapse)
    } else {
        (Icon::ChevronDown, EN.guide_expand)
    };
    let s = T::NAV_ICON;
    let inset = (T::ICON_BUTTON - s) / 2.0;
    ui.icon(bx + inset, by + inset, s, icon, s, role);
    ui.tip_plain((x, y, T::TOUCH, T::TOUCH), Action::GuideTall, tip);
}
