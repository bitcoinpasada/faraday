//! The codex32 entry's input panel (`docs/LAYOUT.md` §16.3, F114): what
//! the person presses and what it fills, in one view. For dice it holds
//! the rolls of the character being entered (each die's first and second
//! roll, what the pair gives, the pair struck as equal) and the pad:
//! "Your dice", a key per face and Backspace, each `PAD_KEY_W` square
//! (the dice's size is the Entropy card's, owner 2026-10-25); for
//! characters typed, the codex32 keyboard.
//! Once every character is in, **Continue** stands in the pad's place:
//! the flow's one way on (owner, 2026-10-24). It stands beside the
//! sheets where the work width holds it and the worksheet at its least
//! fitting cell, holding its place as the page scrolls; pinned at the
//! window's foot elsewhere, the page scrolling clear of it, its parts
//! laid out the shortest way the width allows; in the page where it
//! would take more than `INPUT_PANEL_MAX` of the window's height, and on
//! the small panel (compact and short), as before (`docs/CODEX32.md`
//! §6.4.7).
//!
//! Where it stands, how tall it is and where each part goes are
//! arithmetic on the window's size in dp, the dice chosen and the way of
//! entry ([`place`], [`Foot`]), so drawing, the page's room under its
//! last row and the worksheet's fit ([`PanelAt::foot_h`],
//! [`PanelAt::lead`]) take the same one. Drawing reads the flow's
//! state.

use crate::ui::Region;
use faraday_draw::geom::{Window, WindowClass};
use faraday_draw::tokens as t;
use faraday_draw::tokens::{BODY_M, TITLE_S};
use faraday_draw::widgets::Icon;
use faraday_draw::widgets::keyboard::KeyboardKind;
use faraday_entropy::booklet::BOOKLET_DICE;
use faraday_ui::keyboard::Keys as OskKeys;

use super::pane_screen::side_room;
use super::screen::{MASK, a};
use super::{C32, Card, Codex32Make, OskPress, Way};
use crate::drawing::View;
use crate::fresh::roller::five_screen::{
    pairs, pairs_across, pairs_across_h, pairs_across_w, rolls_line,
};
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Slot, Style, Ui};

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

    /// The room the panel takes from the card's work width, at its start:
    /// the panel and the spacer beside it (the card's own margin is the
    /// container's), else none. A lookup.
    pub(crate) fn lead(self) -> f32 {
        match self {
            PanelAt::Beside => t::CODEX32_ENTRY_W + t::SPACER,
            _ => 0.0,
        }
    }

    /// Whether the panel stands out of the page, beside it or at its
    /// foot, so the page itself draws neither the pad, the rolls nor
    /// Continue.
    pub(crate) fn out_of_page(self) -> bool {
        self != PanelAt::Page
    }
}

/// Where the input panel stands for the entry in a page area `cw` wide
/// in a window `h` tall (`docs/LAYOUT.md` §16.3). Arithmetic.
pub(crate) fn place(app: &View, m: &Codex32Make, cw: f32, h: f32) -> PanelAt {
    let k = app.text_size().factor();
    place_now(m, cw, h, (app.window(), app.is_compact(), k))
}

/// [`place`] from the state as it is: the window (its class and whether
/// it is short), one-column pages or not, and the text factor.
/// Arithmetic.
pub(crate) fn place_now(m: &Codex32Make, cw: f32, h: f32, shell: (Window, bool, f32)) -> PanelAt {
    let column = super::pane_screen::column_now(m, cw, shell.1);
    place_in(m, (cw, h), (shell.0, shell.2), side_room(column))
}

/// [`place_now`] beside a supporting pane that takes `side` of the page
/// area's width, on window `win` at the text factor `k`. Arithmetic.
pub(crate) fn place_in(
    m: &Codex32Make,
    (cw, h): (f32, f32),
    (win, k): (Window, f32),
    side: f32,
) -> PanelAt {
    // While a character is left, the pad; once all are in, Continue.
    let shown = m.front().is_some() || m.strings().is_some();
    if m.open() != Some(Card::Enter) || m.work().is_none() || !shown {
        return PanelAt::Page;
    }
    // The small panel keeps the context panel over the pad, all on the
    // page (`docs/CODEX32.md` §6.4.7); a compact window that is not short
    // pins the panel at its foot, above the navigation bar.
    if win.class() == WindowClass::Compact && win.short() {
        return PanelAt::Page;
    }
    let work = crate::keygen::screen::codex32_work_w(cw, side);
    let sheet_w = m.length().chars() as f32 * (t::CODEX32_CELL_FIT_MIN + 1.0);
    // Beside the sheets wherever the work width holds it; taller than
    // the window, it scrolls within itself there.
    if work >= t::CODEX32_ENTRY_W + t::SPACER + sheet_w {
        return PanelAt::Beside;
    }
    let fh = Foot::of(m, cw - 2.0 * t::MARGIN_WIDE, k).h;
    if fh <= t::INPUT_PANEL_MAX * h {
        PanelAt::Foot(fh)
    } else {
        PanelAt::Page
    }
}

/// Every character is in: Continue stands in the pad's place.
fn complete(m: &Codex32Make) -> bool {
    m.front().is_none()
}

/// How the pad's keys stand in a width: "Your dice" at the start of
/// the first row (and under it the face waiting for its units digit, or
/// ten-faced dice's "0 is 10"), then a key per face and Backspace in
/// `rows` rows of `per`, as few rows as the width allows (owner,
/// 2026-10-25: the dice's size is chosen on the Entropy card, so the pad
/// has no button for it). `w` is the width it takes.
/// Dice of fewer than ten faces leave "Your dice" out where the keys
/// fit one row only without it (owner, 2026-10-25: the keys explain
/// themselves); they have no face waiting and no "0 is 10" to show.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Pad {
    per: usize,
    rows: usize,
    w: f32,
    label: bool,
}

impl Pad {
    /// The room before the keys: "Your dice" and its gap, or none.
    fn lead(self) -> f32 {
        if self.label { LABEL } else { 0.0 }
    }
}

/// A key and the gap after it.
const KEY: f32 = t::PAD_KEY_W + t::GAP_SMALL;

/// "Your dice" before the keys and the gap after it.
const LABEL: f32 = t::INPUT_PAD_LABEL_W + t::GAP;

/// The pad's keys: a key per face and Backspace.
fn keys(m: &Codex32Make) -> usize {
    usize::from(m.dice_choice().faces) + 1
}

/// The pad with its keys in `rows` rows, as even as they go. Arithmetic.
fn pad_rows(m: &Codex32Make, rows: usize) -> Pad {
    let rows = rows.clamp(1, keys(m));
    let per = keys(m).div_ceil(rows);
    let rows = keys(m).div_ceil(per);
    Pad {
        per,
        rows,
        w: LABEL + per as f32 * KEY - t::GAP_SMALL,
        label: true,
    }
}

/// The pad of the dice chosen in `w`: the fewest rows that fit, "Your
/// dice" left out where that alone puts the keys of dice of fewer than
/// ten faces in one row. Arithmetic.
fn pad_plan(m: &Codex32Make, w: f32) -> Pad {
    let fit = (((w - LABEL + t::GAP_SMALL) / KEY) as usize).max(1);
    let p = pad_rows(m, keys(m).div_ceil(fit));
    let bare = Pad {
        per: keys(m),
        rows: 1,
        w: keys(m) as f32 * KEY - t::GAP_SMALL,
        label: false,
    };
    if (p.rows > 1 || p.w > w) && m.dice_choice().faces < 10 && bare.w <= w {
        bare
    } else {
        p
    }
}

/// The pad's height: its rows of keys. A lookup.
fn pad_tall(p: Pad) -> f32 {
    p.rows as f32 * KEY - t::GAP_SMALL
}

/// The pad's height in `w`.
fn pad_h(m: &Codex32Make, w: f32) -> f32 {
    pad_tall(pad_plan(m, w))
}

/// The codex32 keyboard's height in the panel.
fn keyboard_h() -> f32 {
    faraday_ui::keyboard::height(KeyboardKind::Codex32, false)
}

/// The inner width of the panel beside the sheets.
fn beside_inner() -> f32 {
    t::CODEX32_ENTRY_W - 2.0 * t::PAD
}

/// The rolls one under another, as the panel beside the sheets draws
/// them, with the pairs struck so far, at the text factor `k`.
fn rolls_down_h(m: &Codex32Make, k: f32) -> f32 {
    let struck = m.char_rows().iter().filter(|r| r.bit().is_none()).count();
    let line = rolls_line(k) + t::ROLLER_GAP / 2.0;
    line + BOOKLET_DICE as f32 * (t::ROLLER_PAIR_DIE + t::ROLLER_FIVE_GAP) + struck as f32 * line
}

/// The panel's height beside the sheets at the text factor `k`: the
/// rolls over the pad, the keyboard or Continue.
fn beside_h(m: &Codex32Make, k: f32) -> f32 {
    let iw = beside_inner();
    let body = if complete(m) {
        t::TOUCH
    } else if m.way() == Way::Typed {
        keyboard_h()
    } else {
        rolls_down_h(m, k) + t::GAP + pad_h(m, iw)
    };
    t::PAD + body + t::PAD
}

/// Where the panel's parts stand at the window's foot, from its inner
/// top-left: the rolls (none once every character is in), and the pad,
/// the keyboard or Continue, each (x, y, width), and the panel's height.
/// Of the ways the width allows, the shortest: the pad beside the rolls,
/// or under them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Foot {
    rolls: Option<(f32, f32, f32)>,
    pad: (f32, f32, f32),
    /// The panel's height, its gaps above and below included.
    pub(crate) h: f32,
}

impl Foot {
    /// The parts of the panel at the foot, `inner` wide inside its
    /// margins, at the text factor `k`. Arithmetic.
    pub(crate) fn of(m: &Codex32Make, inner: f32, k: f32) -> Foot {
        if complete(m) {
            return Foot::framed(None, (0.0, 0.0, inner), t::TOUCH);
        }
        if m.way() == Way::Typed {
            let kw = inner.min(t::COLUMN_MAX);
            return Foot::framed(None, (0.0, 0.0, kw), keyboard_h());
        }
        let (rw, rh) = (pairs_across_w(), pairs_across_h(k));
        let rolls = Some((0.0, 0.0, rw));
        let down = {
            let py = rh + t::GAP;
            Foot::framed(rolls, (0.0, py, inner), py + pad_h(m, inner))
        };
        let room = inner - rw - t::SPACER;
        if room < LABEL + t::PAD_KEYS_BESIDE_MIN * KEY - t::GAP_SMALL {
            return down;
        }
        // The pad beside the rolls.
        let pad = pad_plan(m, room);
        let beside = Foot::framed(rolls, (rw + t::SPACER, 0.0, pad.w), rh.max(pad_tall(pad)));
        if beside.h <= down.h { beside } else { down }
    }

    /// The parts, and the panel's height round a body `body` tall.
    fn framed(rolls: Option<(f32, f32, f32)>, pad: (f32, f32, f32), body: f32) -> Foot {
        Foot {
            rolls,
            pad,
            h: t::GAP + body + t::GAP,
        }
    }
}

/// Draws the panel where it stands out of the page, for the page area at
/// `x0`, `cw` wide, in a window `h` tall; in the page it is the codex32
/// screen's own. Beside the sheets it stands at the card column's edge in
/// the container's `room`, its top `top` (under the card's head).
pub(crate) fn draw(
    m: &Codex32Make,
    ui: &mut Ui,
    at: PanelAt,
    (x0, cw, h): (f32, f32, f32),
    (room, top): (&crate::flow::Room, f32),
) {
    match at {
        PanelAt::Beside => beside(m, ui, (room.x, top, h)),
        PanelAt::Foot(fh) => {
            let top = h - fh;
            cover(ui, (x0, top, cw, fh));
            let (ix, inner) = (x0 + t::MARGIN_WIDE, cw - 2.0 * t::MARGIN_WIDE);
            let f = Foot::of(m, inner, ui.text_factor());
            let at = |(px, py, pw): (f32, f32, f32)| (ix + px, top + t::GAP + py, pw);
            if let Some(r) = f.rolls {
                pairs_across(ui, at(r), &super::dice_screen::pairs_of(m), true);
            }
            let (px, py, pw) = at(f.pad);
            if complete(m) {
                continue_button(ui, px, py, pw);
                return;
            }
            if m.way() == Way::Typed {
                keyboard(ui, px, py, pw);
                return;
            }
            pad(m, ui, px, py, pw);
        }
        PanelAt::Page => {}
    }
}

/// The panel beside the sheets, at the card column's edge `x`, in a
/// window `h` tall: its top `y` (under the card's head, at the least
/// `SUPPORT_PANE_TOP`), as tall as its parts up to the window's foot less
/// the margin, scrolled within itself by its own offset where it is taller
/// (`Slot::InputPanel`), as a supporting pane is (`docs/LAYOUT.md` §6).
fn beside(m: &Codex32Make, ui: &mut Ui, (x, y, h): (f32, f32, f32)) {
    let w = t::CODEX32_ENTRY_W;
    let y = y.max(t::SUPPORT_PANE_TOP);
    let all = beside_h(m, ui.text_factor());
    let view = (h - y - t::MARGIN_WIDE).max(t::TOUCH).min(all);
    ui.fill(x, y, w, view, t::SHAPE_M, Role::SurfaceContainerLow);
    let clip = ui.rect(x, y, w, view);
    let off = m.panel_offset().clamp(0.0, (all - view).max(0.0));
    let open = ui.region_open(Region::beside(Slot::InputPanel, clip, off).from_zero(off));
    let mark = ui.presses_mark();
    beside_parts(m, ui, x, y);
    // What is scrolled out of the panel takes no press.
    ui.clip_presses_since(mark, clip);
    ui.region_close(open, (all - view).max(0.0));
}

/// The panel's parts beside the sheets from its top-left (x, y): the
/// rolls and the pad, the keyboard or Continue.
fn beside_parts(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32) {
    let (ix, iw) = (x + t::PAD, beside_inner());
    let mut cy = y + t::PAD;
    let rolling = !complete(m) && m.way() == Way::Dice;
    if rolling {
        cy += pairs(ui, (ix, cy, iw), &super::dice_screen::pairs_of(m), None) + t::GAP;
    }
    if complete(m) {
        continue_button(ui, ix, cy, iw);
        return;
    }
    if m.way() == Way::Typed {
        keyboard(ui, ix, cy, iw);
        return;
    }
    pad(m, ui, ix, cy, iw);
}

/// **Continue** at the end of the row at (x, y), `w` wide: the flow's
/// one way on once every character is in.
fn continue_button(ui: &mut Ui, x: f32, y: f32, w: f32) {
    let label = EN.keygen_continue;
    let bw = ui.measure_as(TITLE_S, label) + 2.0 * t::SPACER;
    ui.button(
        x + w - bw,
        y,
        Some(bw),
        t::TOUCH,
        label,
        Style::Primary,
        a(C32::Next),
    );
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

/// The pad drawn in the page, where the panel would take too much of a
/// low window (`docs/LAYOUT.md` §16.3). Returns the height.
pub(crate) fn page_pad(m: &Codex32Make, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    pad(m, ui, x, y, w);
    pad_h(m, w)
}

/// "Your dice", under it the face waiting for its units digit or
/// ten-faced dice's "0 is 10", and the pad: a key per face and
/// Backspace, each `PAD_KEY_W` square, in as few rows as `w` allows
/// ([`pad_plan`]). The dice's size is the Entropy card's.
fn pad(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) {
    let p = pad_plan(m, w);
    let note = match m.pending() {
        Some(_) if m.masked() => Some(fill(EN.codex32_face_pending, &[MASK])),
        Some(d) => Some(fill(EN.codex32_face_pending, &[&d.to_string()])),
        None if m.dice_choice().faces == 10 => Some(EN.codex32_d10_zero.to_string()),
        None => None,
    };
    let ink = if m.pending().is_some() {
        Role::Primary
    } else {
        Role::OnSurfaceVariant
    };
    let lw = t::INPUT_PAD_LABEL_W;
    let head = if p.label {
        EN.codex32_your_dice_head
    } else {
        ""
    };
    let role = Role::OnSurfaceVariant;
    // One-line slots before the keys: larger text fits down to Standard
    // there (`docs/TEXT-SIZE.md` §3).
    match &note {
        None => {
            ui.text_fit_as((x, y, t::TOUCH, lw), TITLE_S, role, head);
        }
        Some(n) => {
            let lh = ui.line_as(TITLE_S);
            let top = y + (t::TOUCH - 2.0 * lh) / 2.0;
            ui.text_fit_as((x, top, lh, lw), TITLE_S, role, head);
            ui.text_fit_as((x, top + lh, lh, lw), BODY_M.mono(), ink, n);
        }
    }
    let faces = usize::from(m.dice_choice().faces);
    let kx = x + p.lead();
    let key = |i: usize| (kx + (i % p.per) as f32 * KEY, y + (i / p.per) as f32 * KEY);
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
}

/// Whether an input panel `fh` tall stands at the foot of a page `h`
/// tall on window `win`: not on the small panel, whose pads stand in the
/// page, and at most `INPUT_PANEL_MAX` of the height (`docs/LAYOUT.md`
/// §16.3). Arithmetic.
pub(crate) fn foot_fits(win: Window, fh: f32, h: f32) -> bool {
    let small = win.class() == WindowClass::Compact && win.short();
    !small && fh <= t::INPUT_PANEL_MAX * h
}

/// The panel's surface at the foot over what is under it, with a rule
/// along its top; a press there reaches nothing under it.
pub(crate) fn cover(ui: &mut Ui, (x, y, w, h): (f32, f32, f32, f32)) {
    let area = ui.rect(x, y, w, h);
    ui.cover_presses(area);
    ui.fill(x, y, w, h, t::SHAPE_NONE, Role::SurfaceContainerLow);
    ui.rule(x, y, w, Role::OutlineVariant);
}
