//! Where the "Demo only" card stands (`docs/CODEX32.md` §5.5, owner
//! 2026-10-24): a supporting pane (`docs/LAYOUT.md` §6) that holds its
//! place while the page scrolls beside it. On a window wide enough for
//! the entry beside it, the card stands at the right, its top below the
//! top-right strip of the network pill and the eye, `MARGIN_WIDE` from
//! the window's edge and `SPACER` from the page, and scrolls within
//! itself where it is taller than the window. Narrower, a strip pinned at
//! the window's foot holds the step-through's line, Previous, Next and
//! Play, and More opens the whole card over the page from the foot. The
//! card's content is `dice_screen::demo_content`; drawing reads the
//! flow's state and works nothing out.

use faraday_draw::tokens as t;

use super::dice_screen::{demo_box_offered, demo_content};
use super::screen::a;
use super::{C32, Card, Codex32Make, Way};
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Slot, Style, Ui, W};

/// How the "Demo only" card stands on this window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum PaneAt {
    /// Beside the page, at the right, this wide.
    Side(f32),
    /// Pinned at the window's foot.
    Strip,
}

/// Where the "Demo only" card stands beside a page area `cw` wide, or
/// `None` where the entry has none. Arithmetic.
pub(crate) fn pane_at(app: &View, m: &Codex32Make, cw: f32) -> Option<PaneAt> {
    if m.open() != Some(Card::Enter) || !demo_box_offered(m, app.roller_offered()) {
        return None;
    }
    let side = t::SUPPORT_PANE + t::SPACER + t::MARGIN_WIDE;
    if !app.is_compact() && cw - side - t::MARGIN_WIDE >= t::CODEX32_MAIN_MIN {
        Some(PaneAt::Side(t::SUPPORT_PANE))
    } else {
        Some(PaneAt::Strip)
    }
}

/// The room the pane takes from the page at its right: none for the
/// strip.
pub(crate) fn side_room(pane: Option<PaneAt>) -> f32 {
    match pane {
        Some(PaneAt::Side(w)) => w + t::SPACER + t::MARGIN_WIDE,
        _ => 0.0,
    }
}

/// Draws the pane for the page area at `x0`, `cw` wide, in a window `h`
/// tall.
pub(crate) fn draw(
    app: &View,
    m: &Codex32Make,
    ui: &mut Ui,
    pane: PaneAt,
    (x0, cw, h): (f32, f32, f32),
) {
    let roller = app.roller_offered();
    match pane {
        PaneAt::Side(pw) => {
            let x = x0 + cw - t::MARGIN_WIDE - pw;
            let top = t::SUPPORT_PANE_TOP;
            let view = (h - top - t::MARGIN_WIDE).max(t::TOUCH);
            scrolled(ui, m, (x, top, pw, view), (roller, true));
        }
        PaneAt::Strip if m.pane_open() => {
            let (x, w) = (x0, cw);
            let top = h * (1.0 - t::CODEX32_STRIP_OPEN_MAX);
            cover(ui, (x, top, w, h - top));
            let mut cy = top + t::GAP_SMALL;
            cy += more_row(ui, (x + t::PAD, cy, w - t::PAD - t::PAD), true);
            scrolled(ui, m, (x, cy, w, h - cy), (roller, false));
        }
        PaneAt::Strip => {
            let (x, w) = (x0, cw);
            let sh = strip_h(ui, m, w, roller);
            let top = h - sh;
            cover(ui, (x, top, w, sh));
            let (ix, iw) = (x + t::PAD, w - t::PAD - t::PAD);
            let mut cy = top + t::GAP_SMALL;
            cy += more_row(ui, (ix, cy, iw), false);
            if roller && m.way() == Way::Dice {
                super::walk_screen::strip_controls(m, ui, (ix, cy, iw));
            } else {
                faraday_ui::parts::wrap_buttons(
                    ui,
                    ix,
                    cy,
                    iw,
                    t::TOUCH,
                    &[(EN.use_a_sample, Style::Secondary, a(C32::Sample))],
                );
            }
        }
    }
}

/// The strip's height: its head row, and the step-through's line and
/// buttons (or Use a sample). Arithmetic on the font; the width is
/// not read (the line is cut to one row).
pub(crate) fn strip_h(ui: &Ui, m: &Codex32Make, _w: f32, roller: bool) -> f32 {
    let body = if roller && m.way() == Way::Dice {
        super::walk_screen::strip_controls_h(ui)
    } else {
        t::TOUCH + t::PAD
    };
    t::GAP_SMALL + t::TOUCH + body + t::GAP
}

/// The pane's surface over what is under it, with a rule at its top; a
/// press there reaches nothing under it.
fn cover(ui: &mut Ui, (x, y, w, h): (f32, f32, f32, f32)) {
    let area = ui.rect(x, y, w, h);
    // What the page put under the pane takes no press there.
    let mut k = 0;
    while k < ui.hits.len() {
        let r = ui.hits[k].0;
        if r.y >= area.y && r.x < area.x + area.w && r.x + r.w > area.x {
            ui.hits.remove(k);
            continue;
        }
        if r.y + r.h > area.y && r.x < area.x + area.w && r.x + r.w > area.x {
            ui.hits[k].0.h = area.y - r.y;
        }
        k += 1;
    }
    ui.fill(x, y, w, h, t::SHAPE_NONE, Role::Surface);
    ui.rule(x, y, w, Role::OutlineVariant);
}

/// "Demo only" with More (to open the whole card) or Less (back to the
/// strip) at the right. Returns the height.
fn more_row(ui: &mut Ui, (x, y, w): (f32, f32, f32), open: bool) -> f32 {
    ui.text_mid(
        x,
        y,
        t::TOUCH,
        t::BODY,
        W::S,
        Role::OnSurface,
        EN.codex32_demo_only,
    );
    let label = if open {
        EN.codex32_pane_less
    } else {
        EN.codex32_pane_more
    };
    let bw = ui.measure(t::LABEL, W::S, label) + t::PAD + t::PAD;
    ui.button(
        x + w - bw,
        y,
        Some(bw),
        t::TOUCH,
        label,
        Style::Secondary,
        a(C32::Pane),
    );
    t::TOUCH
}

/// The card's whole content in the area (x, y, w, view), scrolled by its
/// own offset where it is taller (`Slot::DemoPane`); with `card`,
/// headed and outlined as a card, else under the strip's own head.
fn scrolled(
    ui: &mut Ui,
    m: &Codex32Make,
    (x, y, w, view): (f32, f32, f32, f32),
    (roller, card): (bool, bool),
) {
    let clip = ui.rect(x, y, w, view);
    let off = m.pane_offset();
    ui.c.push_clip(clip);
    let mark = ui.hits.len();
    let ch = demo_content(m, ui, (x, y - off, w), roller, card);
    // What is scrolled out of the pane takes no press.
    let mut k = mark;
    while k < ui.hits.len() {
        let r = ui.hits[k].0.intersect(&clip);
        if r.w <= 0 || r.h <= 0 {
            ui.hits.remove(k);
        } else {
            ui.hits[k].0 = r;
            k += 1;
        }
    }
    ui.c.pop_clip();
    if card {
        ui.stroke(x, y, w, ch.min(view), t::SHAPE_M, Role::OutlineVariant);
    }
    ui.report_scroll_beside(Slot::DemoPane, clip, (ch - view).max(0.0), off);
}
