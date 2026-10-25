//! The codex32 entry's input panel (`docs/LAYOUT.md` §16.3, F114): what
//! the person presses and what it fills, in one view. For dice it holds
//! the rolls of the character being entered (each die's first and second
//! roll, what the pair gives, the pairs struck as equal), the dice's size
//! and **Your dice**, the pad, its keys `PAD_KEY_W` by `TOUCH`; for
//! characters typed, the codex32 keyboard. It stands beside the sheets
//! where the work width holds it and the worksheet at its least fitting
//! cell, holding its place as the page scrolls; pinned at the window's
//! foot elsewhere, the page scrolling clear of it; in the page where it
//! would take more than `INPUT_PANEL_MAX` of the window's height, and on
//! compact windows, as before (`docs/CODEX32.md` §6.4.7).
//!
//! Where it stands and how tall it is are arithmetic on the window's size
//! in dp, the dice chosen and the way of entry ([`place`]), so drawing,
//! the page's room under its last row and the worksheet's fit
//! ([`PanelAt::foot_h`], [`PanelAt::start_room`]) take the same one. Drawing reads the flow's state.

use faraday_draw::geom::WindowClass;
use faraday_draw::tokens as t;
use faraday_draw::widgets::Icon;
use faraday_draw::widgets::keyboard::KeyboardKind;
use faraday_entropy::booklet::BOOKLET_DICE;
use faraday_ui::keyboard::Keys as OskKeys;

use super::pane_screen::side_room;
use super::screen::{MASK, a};
use super::{C32, Card, Codex32Make, OskPress, Way};
use crate::drawing::View;
use crate::fresh::roller::five_screen::{pairs, pairs_across, pairs_across_h, pairs_across_w};
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};

/// Where the input panel stands on this window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum PanelAt {
    /// A column at the work area's start, beside the sheets.
    Beside,
    /// Pinned at the window's foot across the page area, this tall.
    Foot(f32),
    /// In the page under the sheets, as before; or no panel at all.
    Page,
}

impl PanelAt {
    /// The room the panel takes at the page area's foot: its height at
    /// the foot, else none. A lookup.
    pub(crate) fn foot_h(self) -> f32 {
        match self {
            PanelAt::Foot(h) => h,
            _ => 0.0,
        }
    }

    /// The room the panel takes at the page area's start, before the
    /// page's column: the margin, the panel and the spacer beside it,
    /// else none. A lookup.
    pub(crate) fn start_room(self) -> f32 {
        match self {
            PanelAt::Beside => t::MARGIN_WIDE + t::CODEX32_ENTRY_W + t::SPACER,
            _ => 0.0,
        }
    }

    /// Whether the panel stands out of the page, beside it or at its
    /// foot, so the page itself draws neither the pad nor the rolls.
    pub(crate) fn out_of_page(self) -> bool {
        self != PanelAt::Page
    }
}

/// Where the input panel stands for the entry in a page area `cw` wide
/// in a window `h` tall (`docs/LAYOUT.md` §16.3). Arithmetic.
pub(crate) fn place(app: &View, m: &Codex32Make, cw: f32, h: f32) -> PanelAt {
    let shell = (app.window().class(), app.is_compact(), app.roller_offered());
    place_now(m, cw, h, shell)
}

/// [`place`] from the state as it is: the window's class, one-column
/// pages or not, and whether the computer's roller is offered.
/// Arithmetic.
pub(crate) fn place_now(
    m: &Codex32Make,
    cw: f32,
    h: f32,
    (class, compact, roller): (WindowClass, bool, bool),
) -> PanelAt {
    if m.open() != Some(Card::Enter) || m.work().is_none() || m.front().is_none() {
        return PanelAt::Page;
    }
    // The small panel and narrow windows keep the context panel over the
    // pad, all on the page (`docs/CODEX32.md` §6.4.7).
    if class == WindowClass::Compact {
        return PanelAt::Page;
    }
    let pane = super::pane_screen::pane_at_now(m, cw, (compact, roller));
    let work = crate::keygen::screen::codex32_work_w(cw, side_room(pane));
    let sheet_w = m.length().chars() as f32 * (t::CODEX32_CELL_FIT_MIN + 1.0);
    let beside = work >= t::CODEX32_ENTRY_W + t::SPACER + sheet_w
        && beside_h(m) <= h - t::SUPPORT_PANE_TOP - t::MARGIN_WIDE;
    if beside {
        return PanelAt::Beside;
    }
    let fh = foot_h(m, cw - 2.0 * t::MARGIN_WIDE);
    if fh <= t::INPUT_PANEL_MAX * h {
        PanelAt::Foot(fh)
    } else {
        PanelAt::Page
    }
}

/// The keys of the pad for the dice chosen: a key per face, Backspace
/// and Done.
fn keys(m: &Codex32Make) -> usize {
    usize::from(m.dice_choice().faces) + 2
}

/// How many keys a row of `w` holds, up to `PAD_KEYS_ROW_MAX`.
fn per_row(w: f32) -> usize {
    let n = ((w + t::GAP_SMALL) / (t::PAD_KEY_W + t::GAP_SMALL)) as usize;
    n.clamp(1, t::PAD_KEYS_ROW_MAX)
}

/// The width of a row of `n` keys.
fn row_w(n: usize) -> f32 {
    n as f32 * (t::PAD_KEY_W + t::GAP_SMALL) - t::GAP_SMALL
}

/// The dice's size in `w`: its label and the segmented choice, on one
/// row where `INPUT_SIZES_W` fits, else wrapped onto two.
fn sizes_h(w: f32) -> f32 {
    let rows = if w >= t::INPUT_SIZES_W { 1.0 } else { 2.0 };
    t::PAD + t::GAP_SMALL + rows * t::TOUCH + t::GAP
}

/// "Your dice" and the pad's rows in `w`.
fn pad_h(m: &Codex32Make, w: f32) -> f32 {
    let rows = keys(m).div_ceil(per_row(w));
    t::PAD + t::GAP_SMALL + rows as f32 * (t::TOUCH + t::GAP_SMALL) - t::GAP_SMALL
}

/// The codex32 keyboard's height in the panel.
fn keyboard_h() -> f32 {
    faraday_ui::keyboard::height(KeyboardKind::Codex32)
}

/// The inner width of the panel beside the sheets.
fn beside_inner() -> f32 {
    t::CODEX32_ENTRY_W - 2.0 * t::PAD
}

/// The rolls one under another, as the panel beside the sheets draws
/// them, with the pairs struck so far.
fn rolls_down_h(m: &Codex32Make) -> f32 {
    let struck = m.shown_rows().iter().filter(|r| r.bit().is_none()).count();
    let line = t::INPUT_ROLLS_LINE + t::ROLLER_GAP / 2.0;
    line + BOOKLET_DICE as f32 * (t::ROLLER_PAIR_DIE + t::ROLLER_FIVE_GAP) + struck as f32 * line
}

/// The Working area's height and the gap under it (`tracker_screen.rs`).
fn work_h(w: f32) -> f32 {
    super::tracker_screen::working_area_h(w) + t::GAP
}

/// The panel's height beside the sheets.
fn beside_h(m: &Codex32Make) -> f32 {
    let body = if m.way() == Way::Typed {
        work_h(beside_inner()) + keyboard_h()
    } else {
        rolls_down_h(m)
            + t::GAP
            + work_h(beside_inner())
            + sizes_h(beside_inner())
            + pad_h(m, beside_inner())
    };
    t::PAD + body + t::PAD
}

/// Whether the rolls stand in a row with the dice's size and the pad in
/// `inner`, and the width left for those two.
fn row_room(inner: f32) -> Option<f32> {
    let left = inner - pairs_across_w() - t::SPACER;
    (left >= t::INPUT_SIZES_W.max(beside_inner())).then_some(left)
}

/// The codex32 keyboard's width at the foot, `inner` wide, and where the
/// Working area stands beside it (its left edge from the panel's start
/// and its width), where the room right of a keyboard at least
/// `CODEX32_KEYS_BESIDE_MIN` wide holds it; else above it.
fn keys_at_foot(inner: f32) -> (f32, Option<(f32, f32)>) {
    let kw = (inner - t::SPACER - beside_inner()).min(t::COLUMN_MAX);
    if kw >= t::CODEX32_KEYS_BESIDE_MIN {
        (kw, Some((kw + t::SPACER, inner - kw - t::SPACER)))
    } else {
        (inner.min(t::COLUMN_MAX), None)
    }
}

/// The panel's height at the window's foot, `inner` wide inside its
/// margins.
fn foot_h(m: &Codex32Make, inner: f32) -> f32 {
    let body = if m.way() == Way::Typed {
        match keys_at_foot(inner) {
            (_, Some((_, ww))) => keyboard_h().max(work_h(ww)),
            (kw, None) => work_h(kw) + keyboard_h(),
        }
    } else if let Some(left) = row_room(inner) {
        pairs_across_h().max(work_h(left) + sizes_h(left) + pad_h(m, left))
    } else {
        pairs_across_h() + t::GAP + work_h(inner) + sizes_h(inner) + pad_h(m, inner)
    };
    t::GAP + body + t::GAP
}

/// Draws the panel where it stands out of the page, for the page area at
/// `x0`, `cw` wide, in a window `h` tall; in the page it is the codex32
/// screen's own.
pub(crate) fn draw(m: &Codex32Make, ui: &mut Ui, at: PanelAt, (x0, cw, h): (f32, f32, f32)) {
    match at {
        PanelAt::Beside => {
            let (x, y, w) = (x0 + t::MARGIN_WIDE, t::SUPPORT_PANE_TOP, t::CODEX32_ENTRY_W);
            ui.fill(x, y, w, beside_h(m), t::SHAPE_M, Role::SurfaceContainerLow);
            let (ix, iw) = (x + t::PAD, beside_inner());
            let mut cy = y + t::PAD;
            if m.way() == Way::Typed {
                cy += super::tracker_screen::draw(m, ui, (ix, cy, iw)) + t::GAP;
                keyboard(ui, ix, cy, iw);
                return;
            }
            cy += pairs(ui, (ix, cy, iw), &super::dice_screen::pairs_of(m), None) + t::GAP;
            cy += super::tracker_screen::draw(m, ui, (ix, cy, iw)) + t::GAP;
            cy += super::dice_screen::sizes(m, ui, ix, cy, iw);
            pad(m, ui, ix, cy, iw);
        }
        PanelAt::Foot(fh) => {
            let top = h - fh;
            cover(ui, (x0, top, cw, fh));
            let (ix, inner) = (x0 + t::MARGIN_WIDE, cw - 2.0 * t::MARGIN_WIDE);
            let mut cy = top + t::GAP;
            if m.way() == Way::Typed {
                let (kw, beside) = keys_at_foot(inner);
                match beside {
                    Some((wx, ww)) => {
                        super::tracker_screen::draw(m, ui, (ix + wx, cy, ww));
                    }
                    None => cy += super::tracker_screen::draw(m, ui, (ix, cy, kw)) + t::GAP,
                }
                keyboard(ui, ix, cy, kw);
                return;
            }
            let rolls = super::dice_screen::pairs_of(m);
            let (px, pw) = if let Some(left) = row_room(inner) {
                pairs_across(ui, (ix, cy, pairs_across_w()), &rolls);
                let kw = row_w(per_row(left).min(keys(m)));
                let least = t::INPUT_SIZES_W.max(beside_inner());
                let px = ix + pairs_across_w() + t::SPACER;
                cy += super::tracker_screen::draw(m, ui, (px, cy, left)) + t::GAP;
                (px, kw.max(least).min(left))
            } else {
                cy += pairs_across(ui, (ix, cy, inner), &rolls) + t::GAP;
                cy += super::tracker_screen::draw(m, ui, (ix, cy, inner)) + t::GAP;
                (ix, inner)
            };
            cy += super::dice_screen::sizes(m, ui, px, cy, pw);
            pad(m, ui, px, cy, pw);
        }
        PanelAt::Page => {}
    }
}

/// The codex32 keyboard for characters typed, its keys this flow's.
fn keyboard(ui: &mut Ui, x: f32, y: f32, w: f32) {
    faraday_ui::keyboard::keyboard(
        ui,
        x,
        w,
        y,
        KeyboardKind::Codex32,
        OskKeys::default(),
        ui.compact,
        |p| a(C32::Key(p)),
    );
}

/// "Your dice", the face waiting for its units digit or the ten-faced
/// dice's "0 is 10" beside it, and the pad: a key per face, Backspace
/// and Done, each `PAD_KEY_W` by `TOUCH`, as many a row as fit in `w`.
fn pad(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) {
    crate::screens::section_label(ui, x, y, EN.codex32_your_dice_head);
    let note = match m.pending() {
        Some(_) if m.masked() => Some(fill(EN.codex32_face_pending, &[MASK])),
        Some(d) => Some(fill(EN.codex32_face_pending, &[&d.to_string()])),
        None if m.dice_choice().faces == 10 => Some(EN.codex32_d10_zero.to_string()),
        None => None,
    };
    if let Some(note) = note {
        let lw = ui.measure(t::LABEL, W::S, EN.codex32_your_dice_head);
        let ink = if m.pending().is_some() {
            Role::Primary
        } else {
            Role::OnSurfaceVariant
        };
        ui.text(x + lw + t::GAP, y, t::LABEL, W::M, ink, &note);
    }
    let top = y + t::PAD + t::GAP_SMALL;
    let n = per_row(w);
    let faces = usize::from(m.dice_choice().faces);
    let key = |i: usize| {
        (
            x + (i % n) as f32 * (t::PAD_KEY_W + t::GAP_SMALL),
            top + (i / n) as f32 * (t::TOUCH + t::GAP_SMALL),
        )
    };
    let shape = |(kx, ky): (f32, f32)| (kx, ky, Some(t::PAD_KEY_W), t::TOUCH, t::RADIUS_SMALL);
    for f in 1..=faces {
        let face = a(C32::Face(f as u8));
        ui.button_in(shape(key(f - 1)), &f.to_string(), Style::Secondary, face);
    }
    let back = key(faces);
    let press = a(C32::Key(OskPress::Back));
    ui.button_in(shape(back), "", Style::Secondary, press);
    let ic = t::ICON_SMALL;
    ui.icon(
        back.0 + (t::PAD_KEY_W - ic) / 2.0,
        back.1 + (t::TOUCH - ic) / 2.0,
        ic,
        Icon::Delete,
        t::CODEX32_TAB_GLYPH,
        Role::OnSurface,
    );
    let done = a(C32::Key(OskPress::Done));
    ui.button_in(shape(key(faces + 1)), EN.compact_done, Style::Primary, done);
}

/// The panel's surface at the foot over what is under it, with a rule
/// along its top; a press there reaches nothing under it.
fn cover(ui: &mut Ui, (x, y, w, h): (f32, f32, f32, f32)) {
    let area = ui.rect(x, y, w, h);
    let mut k = 0;
    while k < ui.hits.len() {
        let r = ui.hits[k].0;
        let across = r.x < area.x + area.w && r.x + r.w > area.x;
        if across && r.y >= area.y {
            ui.hits.remove(k);
            continue;
        }
        if across && r.y + r.h > area.y {
            ui.hits[k].0.h = area.y - r.y;
        }
        k += 1;
    }
    ui.fill(x, y, w, h, t::SHAPE_NONE, Role::SurfaceContainerLow);
    ui.rule(x, y, w, Role::OutlineVariant);
}
