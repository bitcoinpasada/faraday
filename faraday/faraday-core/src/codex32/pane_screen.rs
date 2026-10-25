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
    pane_at_now(m, cw, (app.is_compact(), app.roller_offered()))
}

/// [`pane_at`] from the state as it is: one-column pages or not, and
/// whether the computer's roller is offered. Arithmetic.
pub(crate) fn pane_at_now(
    m: &Codex32Make,
    cw: f32,
    (compact, roller): (bool, bool),
) -> Option<PaneAt> {
    if m.open() != Some(Card::Enter) || !demo_box_offered(m, roller) {
        return None;
    }
    let side = crate::screens::support_pane_room();
    if !compact && cw - side - t::MARGIN_WIDE >= t::CODEX32_MAIN_MIN {
        Some(PaneAt::Side(t::SUPPORT_PANE))
    } else {
        Some(PaneAt::Strip)
    }
}

/// The room the pane takes from the page at its right: none for the
/// strip; a supporting pane's for the card beside the page or the dice
/// sheet's `column`.
pub(crate) fn side_room(pane: Option<PaneAt>, column: bool) -> f32 {
    match pane {
        Some(PaneAt::Side(_)) => crate::screens::support_pane_room(),
        _ if column => crate::screens::support_pane_room(),
        _ => 0.0,
    }
}

/// Whether the dice sheet stands in a column at the right, the "Demo
/// only" card under it, for the entry in a page area `cw` wide in a
/// window `h` tall (`docs/LAYOUT.md` §16.1; owner, 2026-10-24): only
/// where the checksum worksheet still draws whole beside it and the
/// column fits the window's height, over the input panel at the foot.
/// Elsewhere both sheets stand in the page. Arithmetic.
pub(crate) fn column_now(
    m: &Codex32Make,
    cw: f32,
    h: f32,
    (class, compact, roller): (faraday_draw::geom::WindowClass, bool, bool),
) -> bool {
    let entering = m.open() == Some(Card::Enter)
        && m.dice_sheet_shown()
        && m.dice_sheet_open()
        && m.front().is_some();
    let side = crate::screens::support_pane_room();
    if compact || !entering || cw - side - t::MARGIN_WIDE < t::CODEX32_MAIN_MIN {
        return false;
    }
    let Some((sheet_w, sheet_h)) = m.sheet_least() else {
        return false;
    };
    let panel = super::input_panel::place_in(m, cw, h, class, side);
    let start = panel.start_room();
    let lead = if start > 0.0 {
        start - t::KEYGEN_COLUMN_IN
    } else {
        0.0
    };
    let work = crate::keygen::screen::codex32_work_w(cw, side) - lead;
    let tall = h - 2.0 * t::MARGIN_WIDE - panel.foot_h();
    let card = if demo_box_offered(m, roller) {
        t::SPACER + t::CODEX32_COLUMN_CARD_H
    } else {
        0.0
    };
    let column = t::CODEX32_DICE_COLUMN_H + card;
    sheet_w <= work
        && sheet_h <= tall
        && column <= h - t::SUPPORT_PANE_TOP - t::MARGIN_WIDE - panel.foot_h()
}

/// Draws the pane for the page area at `x0`, `cw` wide, in a window `h`
/// tall: with `column`, the dice sheet at the right and the card, if
/// any, under it, scrolling within themselves.
pub(crate) fn draw(
    app: &View,
    m: &Codex32Make,
    ui: &mut Ui,
    (pane, column): (Option<PaneAt>, bool),
    (x0, cw, h): (f32, f32, f32),
) {
    let roller = app.roller_offered();
    if column {
        let (x, top, w, view) = crate::samples::card::pane_place(app, (x0, cw, h));
        let card = pane.is_some();
        scrolled(ui, m, (x, top, w, view), (roller, card, true));
        return;
    }
    let Some(pane) = pane else {
        return;
    };
    match pane {
        PaneAt::Side(_) => {
            // The one place every "Demo only" card stands (`samples/card.rs`).
            let (x, top, w, view) = crate::samples::card::pane_place(app, (x0, cw, h));
            scrolled(ui, m, (x, top, w, view), (roller, true, false));
        }
        PaneAt::Strip if m.pane_open() => {
            let (x, w) = (x0, cw);
            let top = h * (1.0 - t::CODEX32_STRIP_OPEN_MAX);
            faraday_ui::demo::cover(ui, (x, top, w, h - top));
            let mut cy = top + t::GAP_SMALL;
            cy += more_row(ui, (x + t::PAD, cy, w - t::PAD - t::PAD), true);
            scrolled(ui, m, (x, cy, w, h - cy), (roller, false, false));
        }
        PaneAt::Strip => {
            let (x, w) = (x0, cw);
            let sh = strip_h(ui, m, w, roller);
            let top = h - sh;
            faraday_ui::demo::cover(ui, (x, top, w, sh));
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

/// "Demo only" with More (to open the whole card) or Less (back to the
/// strip) at the right. Returns the height.
fn more_row(ui: &mut Ui, (x, y, w): (f32, f32, f32), open: bool) -> f32 {
    ui.text_mid(x, y, t::TOUCH, t::BODY, W::S, Role::OnSurface, EN.demo_only);
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
/// headed and outlined as a card, else under the strip's own head. With
/// `dice`, the dice sheet over it, the card `SPACER` under the sheet.
fn scrolled(
    ui: &mut Ui,
    m: &Codex32Make,
    (x, y, w, view): (f32, f32, f32, f32),
    (roller, card, dice): (bool, bool, bool),
) {
    let clip = ui.rect(x, y, w, view);
    let off = m.pane_offset();
    ui.c.push_clip(clip);
    let mark = ui.hits.len();
    let sheet = if dice {
        super::dice_screen::dice_sheet(m, ui, (x, y - off, w), false, false) + t::SPACER
    } else {
        0.0
    };
    let top = y - off + sheet;
    let ch = if card {
        demo_content(m, ui, (x, top, w), roller, true)
    } else if dice {
        0.0
    } else {
        demo_content(m, ui, (x, top, w), roller, false)
    };
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
    if card && ch > 0.0 {
        // The card's edge, as much of it as shows.
        let (ct, cb) = (top.max(y), (top + ch).min(y + view));
        if cb > ct {
            faraday_ui::demo::outline(ui, (x, ct, w, cb - ct));
        }
    }
    let all = sheet + ch;
    ui.report_scroll_beside(Slot::DemoPane, clip, (all - view).max(0.0), off);
}
