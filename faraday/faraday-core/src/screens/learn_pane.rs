//! Learn's supporting pane (`docs/LAYOUT.md` §6): on an extra-large
//! window, the Learn pages stand in a pane at the right, beside the page
//! they explain, instead of the sheet over it. The pane holds its place
//! while the page scrolls beside it and scrolls within itself where it
//! is taller than the window; its tabs and ticked steps are the sheet's
//! (`sheets::learn_body`), and its close button puts it away. Which
//! pages it shows is kept on Learn's state (`Faraday::learn_place`);
//! drawing reads them.

use faraday_draw::tokens as T;

use super::sheets::{LearnLook, learn_body};
use crate::Action;
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Slot, Ui, W};

/// Draws Learn's supporting pane for the page area at `x0`, `cw` wide,
/// in a window `h` tall, at the one place every supporting pane stands
/// (`screens::support_pane_place`). Reads the state.
pub(super) fn draw(app: &View, ui: &mut Ui, (x0, cw, h): (f32, f32, f32)) {
    let (x, y, w, view) = super::support_pane_place((x0, cw, h));
    // Nothing of the page under the pane takes a press.
    let area = ui.rect(x, y, w, view);
    ui.hits.retain(|(r, _)| {
        !(r.x >= area.x
            && r.y >= area.y
            && r.right() <= area.right()
            && r.bottom() <= area.bottom())
    });
    ui.fill(x, y, w, view, T::SHAPE_L, Role::SurfaceContainer);
    let look = LearnLook {
        pad: T::LEARN_PANE_PAD,
        tabs_top: T::LEARN_PANE_PAD,
        end: T::TOUCH,
        one_tab_row: true,
        after_tabs: T::PAD,
        rule: T::GAP,
        inset: 0.0,
        text_top: T::GAP,
        foot: T::LEARN_PANE_PAD,
        head: T::TITLE_L,
        heading: T::TITLE_M,
        body: T::BODY_M,
        after: [T::GAP, T::GAP_SMALL, T::LEARN_PANE_PARA, T::GAP],
        clip_hits: true,
    };
    let shown = learn_body(app, ui, &look, (x, y, w, view));
    close_button(ui, x + w - T::GAP - T::TOUCH, y + T::LEARN_PANE_CLOSE_TOP);
    if let Some((clip, max)) = shown {
        ui.report_scroll_beside(Slot::LearnPane, clip, max, app.learn().scroll);
    }
}

/// The pane's close button in a `TOUCH` square at (x, y): Material's
/// standard icon button, `ICON_BUTTON` across in the middle of its
/// target, its state layer on hover and press. The close mark is the
/// multiplication sign, as the vault search's clear is: the icon face
/// has no close glyph.
fn close_button(ui: &mut Ui, x: f32, y: f32) {
    let a = Action::LearnClose;
    let edge = (T::TOUCH - T::ICON_BUTTON) / 2.0;
    let (bx, by) = (x + edge, y + edge);
    let layer = if ui.is_pressed(a) {
        Some(T::STATE_PRESS)
    } else if ui.is_hovered(a) {
        Some(T::STATE_HOVER)
    } else {
        None
    };
    if let Some(at) = layer {
        let r = T::shape_full(T::ICON_BUTTON);
        let c = Role::OnSurfaceVariant.at(at);
        ui.wash(bx, by, T::ICON_BUTTON, T::ICON_BUTTON, r, c);
    }
    let mark = "×";
    let mw = ui.measure(T::ICON_GLYPH, W::R, mark);
    let (mx, size) = (bx + (T::ICON_BUTTON - mw) / 2.0, T::ICON_GLYPH);
    ui.text_mid(
        mx,
        by,
        T::ICON_BUTTON,
        size,
        W::R,
        Role::OnSurfaceVariant,
        mark,
    );
    ui.hit_tip((x, y, T::TOUCH, T::TOUCH), a, EN.learn_close);
}
