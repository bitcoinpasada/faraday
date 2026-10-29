//! Learn's supporting pane (`docs/LAYOUT.md` §6): on an extra-large
//! window, the Learn pages stand in a pane at the right, beside the page
//! they explain, instead of the sheet over it. The pane holds its place
//! while the page scrolls beside it and scrolls within itself where it
//! is taller than the window. Its header row holds the headline, the
//! pin, which keeps it open across screens, and the close button, which
//! puts it away (Material 3 Side sheets, Icon buttons), so the
//! tabs under it run the pane's whole width; its tabs and ticked steps
//! are the sheet's (`sheets::learn_body`). Which pages it shows is kept
//! on Learn's state (`Faraday::learn_place`); drawing reads them.

use faraday_draw::tokens as T;
use faraday_draw::widgets::Icon;

use super::sheets::{LearnLook, learn_body};
use crate::Action;
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::Shape;
use crate::ui::pal::*;
use crate::ui::{Slot, Ui, W};

/// Draws Learn's supporting pane for the page area at `x0`, `cw` wide,
/// in a window `h` tall, at the one place every supporting pane stands
/// (`screens::support_pane_place`). Reads the state.
pub(super) fn draw(app: &View, ui: &mut Ui, (x0, cw, h): (f32, f32, f32)) {
    let (x, y, w, view) = super::support_pane_place((x0, cw, h));
    // Nothing of the page under the pane takes a press.
    let area = ui.rect(x, y, w, view);
    ui.drop_presses_inside(area);
    ui.fill(x, y, w, view, T::SHAPE_L, Role::SurfaceContainer);
    let look = LearnLook {
        pad: T::LEARN_PANE_PAD,
        // The tabs run the pane's whole width, as Material's tabs run
        // their container's.
        tabs_edge: 0.0,
        tabs_top: T::LEARN_PANE_TABS_TOP,
        end: 0.0,
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
    let head = (x + T::LEARN_PANE_PAD, y + T::LEARN_PANE_CLOSE_TOP, T::TOUCH);
    ui.text_mid_as(head, T::TITLE_L, Role::OnSurface, EN.learn_pane_title);
    let close_x = x + w - T::GAP - T::TOUCH;
    pin_button(
        ui,
        close_x - T::TOUCH,
        y + T::LEARN_PANE_CLOSE_TOP,
        app.learn_pinned(),
    );
    close_button(ui, close_x, y + T::LEARN_PANE_CLOSE_TOP);
    if let Some((clip, max)) = shown {
        // Its edges fade into the pane's own container.
        let on = (clip, Role::SurfaceContainer);
        ui.report_scroll_beside_on(Slot::LearnPane, on, max, app.learn().scroll);
    }
}

/// The pane's pin in a `TOUCH` square at (x, y), the close button's
/// target next after it: Material's standard icon button as a toggle,
/// `ICON_BUTTON` across at the end of its target nearest the close
/// button, its state layer on hover and press, the push pin
/// `LEARN_PANE_PIN_GLYPH` across so it reads the size of the × (owner,
/// 2026-10-29). Off, outlined in on-surface-variant, as the ×; on,
/// filled in primary.
fn pin_button(ui: &mut Ui, x: f32, y: f32, on: bool) {
    let a = Action::LearnPin;
    let edge = (T::TOUCH - T::ICON_BUTTON) / 2.0;
    let (bx, by) = (x + 2.0 * edge, y + edge);
    let r = T::shape_full(T::ICON_BUTTON);
    let button = Shape::new(bx, by, T::ICON_BUTTON, T::ICON_BUTTON, r);
    let (icon, ink, tip) = if on {
        (Icon::Pinned, Role::Primary, EN.learn_unpin)
    } else {
        (Icon::Pin, Role::OnSurfaceVariant, EN.learn_pin)
    };
    ui.press_to(button, (x, y, T::TOUCH, T::TOUCH), ink.paint(), a);
    let size = T::LEARN_PANE_PIN_GLYPH;
    let glyph = (T::ICON_BUTTON - size) / 2.0;
    ui.icon(bx + glyph, by + glyph, size, icon, size, ink);
    ui.tip_plain((x, y, T::TOUCH, T::TOUCH), a, tip);
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
    let r = T::shape_full(T::ICON_BUTTON);
    let button = Shape::new(bx, by, T::ICON_BUTTON, T::ICON_BUTTON, r);
    ui.press_to(
        button,
        (x, y, T::TOUCH, T::TOUCH),
        Role::OnSurfaceVariant.paint(),
        a,
    );
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
    ui.tip_plain((x, y, T::TOUCH, T::TOUCH), a, EN.learn_close);
}
