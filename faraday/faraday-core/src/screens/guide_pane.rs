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
use faraday_draw::tokens as T;

use super::sheets::{GuideLook, guide_body};
use crate::Action;
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Slot, Ui};

/// Draws the guide's supporting pane for the page area at `x0`, `cw` wide,
/// in a window `h` tall, at the one place every supporting pane stands
/// (`hub_layout::guide_place`). Reads the state.
pub(super) fn draw(app: &View, ui: &mut Ui, (x0, cw, h): (f32, f32, f32)) {
    let (x, y, w, view) = super::hub_layout::guide_place((x0, cw, h));
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
