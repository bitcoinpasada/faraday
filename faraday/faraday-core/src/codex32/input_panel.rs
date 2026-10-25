//! The codex32 entry's input panel (`docs/LAYOUT.md` §16.3, F114): what
//! the person presses and what it fills, in one view. For dice it holds
//! the rolls of the character being entered (each die's first and second
//! roll, what the pair gives, the pair struck as equal), the step
//! tracker's Working area (`tracker_screen.rs`, `docs/CODEX32.md` §5.7),
//! always beside the pad, and **Your dice**: the dice's size, one
//! button that opens Material's menu of the six sizes, and the pad, a
//! key per face and Backspace, each `PAD_KEY_W` square; for characters
//! typed, the codex32 keyboard with the Working area beside or over it.
//! Once every character is in, **Continue** stands in the pad's place:
//! the flow's one way on (owner, 2026-10-24). It stands beside the
//! sheets where the work width holds it and the worksheet at its least
//! fitting cell, holding its place as the page scrolls; pinned at the
//! window's foot elsewhere, the page scrolling clear of it, its parts
//! laid out the shortest way the width allows; in the page where it
//! would take more than `INPUT_PANEL_MAX` of the window's height, and on
//! compact windows, as before (`docs/CODEX32.md` §6.4.7).
//!
//! Where it stands, how tall it is and where each part goes are
//! arithmetic on the window's size in dp, the dice chosen and the way of
//! entry ([`place`], [`Foot`]), so drawing, the page's room under its
//! last row and the worksheet's fit ([`PanelAt::foot_h`],
//! [`PanelAt::start_room`]) take the same one. Drawing reads the flow's
//! state.

use faraday_draw::geom::WindowClass;
use faraday_draw::tokens as t;
use faraday_draw::widgets::Icon;
use faraday_draw::widgets::keyboard::KeyboardKind;
use faraday_entropy::booklet::BOOKLET_DICE;
use faraday_ui::keyboard::Keys as OskKeys;

use super::pane_screen::side_room;
use super::screen::{MASK, a};
use super::tour_screen::{choice, choice_h};
use super::tracker_screen::working_area_h;
use super::{C32, Card, Codex32Make, DIE_SIZES, OskPress, Way};
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
    /// foot, so the page itself draws neither the pad, the rolls, the
    /// Working area nor Continue.
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
    let pane = super::pane_screen::pane_at_now(m, cw, (compact, roller));
    let column = super::pane_screen::column_now(m, cw, h, (class, compact, roller));
    place_in(m, cw, h, class, side_room(pane, column))
}

/// [`place_now`] beside a supporting pane that takes `side` of the page
/// area's width. Arithmetic.
pub(crate) fn place_in(m: &Codex32Make, cw: f32, h: f32, class: WindowClass, side: f32) -> PanelAt {
    // While a character is left, the pad; once all are in, Continue.
    let shown = m.front().is_some() || m.strings().is_some();
    if m.open() != Some(Card::Enter) || m.work().is_none() || !shown {
        return PanelAt::Page;
    }
    // The small panel and narrow windows keep the context panel over the
    // pad, all on the page (`docs/CODEX32.md` §6.4.7).
    if class == WindowClass::Compact {
        return PanelAt::Page;
    }
    let work = crate::keygen::screen::codex32_work_w(cw, side);
    let sheet_w = m.length().chars() as f32 * (t::CODEX32_CELL_FIT_MIN + 1.0);
    let beside = work >= t::CODEX32_ENTRY_W + t::SPACER + sheet_w
        && beside_h(m) <= h - t::SUPPORT_PANE_TOP - t::MARGIN_WIDE;
    if beside {
        return PanelAt::Beside;
    }
    let fh = Foot::of(m, cw - 2.0 * t::MARGIN_WIDE).h;
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

/// How "Your dice" stands with the pad's keys.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Head {
    /// On the keys' one row, before the dice's button.
    InRow,
    /// Over the keys' one row.
    Over,
    /// In a row of its own with the dice's button and Backspace, the
    /// faces in rows under it.
    Grid,
}

/// How the pad's keys stand in a width: "Your dice", the dice's size
/// and the keys on one row where they fit, else over them; else the
/// faces in `rows` rows of `per`, as even as they go (1 to 10 over 11 to
/// 20 for twenty faces), under a row of "Your dice", the dice's size and
/// Backspace. `w` is the width it takes.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Pad {
    head: Head,
    per: usize,
    rows: usize,
    w: f32,
}

/// A key and the gap after it.
const KEY: f32 = t::PAD_KEY_W + t::GAP_SMALL;

/// The pad of the dice chosen in `w`. Arithmetic.
fn pad_plan(m: &Codex32Make, w: f32) -> Pad {
    let faces = usize::from(m.dice_choice().faces);
    let keys = t::DICE_MENU_BUTTON_W + t::GAP + (faces + 1) as f32 * KEY - t::GAP_SMALL;
    let label = t::INPUT_PAD_LABEL_W + t::GAP;
    let one = |head, w| Pad {
        head,
        per: faces + 1,
        rows: 1,
        w,
    };
    // Ten-faced dice keep their "0 is 10" over the row.
    if label + keys <= w && faces != 10 {
        return one(Head::InRow, label + keys);
    }
    if keys <= w {
        return one(Head::Over, keys);
    }
    let fit = (((w + t::GAP_SMALL) / KEY) as usize).max(1);
    let rows = faces.div_ceil(fit);
    let per = faces.div_ceil(rows);
    let head = label + t::DICE_MENU_BUTTON_W + t::GAP + t::PAD_KEY_W;
    Pad {
        head: Head::Grid,
        per,
        rows,
        w: (per as f32 * KEY - t::GAP_SMALL).max(head),
    }
}

/// "Your dice" as a section's label and the gap under it.
fn label_h() -> f32 {
    t::PAD + t::GAP_SMALL
}

/// "Your dice", the dice's size and the pad's keys in `w`.
fn pad_h(m: &Codex32Make, w: f32) -> f32 {
    let p = pad_plan(m, w);
    match p.head {
        Head::InRow => t::TOUCH,
        Head::Over => label_h() + t::TOUCH,
        Head::Grid => t::TOUCH + t::GAP_SMALL + p.rows as f32 * KEY - t::GAP_SMALL,
    }
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

/// The panel's height beside the sheets: the Working area over the pad,
/// the keyboard or Continue.
fn beside_h(m: &Codex32Make) -> f32 {
    let iw = beside_inner();
    let work = working_area_h(iw) + t::GAP;
    let body = if complete(m) {
        work + choice_h(m) + t::TOUCH
    } else if m.way() == Way::Typed {
        work + keyboard_h()
    } else {
        rolls_down_h(m) + t::GAP + work + pad_h(m, iw)
    };
    t::PAD + body + t::PAD
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

/// Where the panel's parts stand at the window's foot, from its inner
/// top-left: the rolls (none once every character is in), the Working
/// area and the pad, or Continue, each (x, y, width), and the panel's
/// height. Of the ways the width allows, the shortest: the rolls beside
/// the Working area over the pad; the Working area under the rolls
/// beside the pad; or all three one under another (owner, 2026-10-24).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Foot {
    rolls: Option<(f32, f32, f32)>,
    work: (f32, f32, f32),
    pad: (f32, f32, f32),
    /// The panel's height, its gaps above and below included.
    pub(crate) h: f32,
}

impl Foot {
    /// The parts of the panel at the foot, `inner` wide inside its
    /// margins. Arithmetic.
    pub(crate) fn of(m: &Codex32Make, inner: f32) -> Foot {
        let wa = working_area_h;
        if complete(m) {
            // The Working area with the tour's choice under it, and
            // Continue at the row's end.
            let ww = inner - t::SPACER - t::INPUT_CONTINUE_W;
            let body = (wa(ww) + choice_h(m)).max(t::TOUCH);
            // Continue on the last row, level with the choice.
            let end = (
                inner - t::INPUT_CONTINUE_W,
                body - t::TOUCH,
                t::INPUT_CONTINUE_W,
            );
            return Foot::framed(None, (0.0, 0.0, ww), end, body);
        }
        if m.way() == Way::Typed {
            let (kw, beside) = keys_at_foot(inner);
            return match beside {
                Some((wx, ww)) => {
                    let body = keyboard_h().max(wa(ww));
                    Foot::framed(None, (wx, 0.0, ww), (0.0, 0.0, kw), body)
                }
                None => {
                    let top = wa(kw) + t::GAP;
                    Foot::framed(None, (0.0, 0.0, kw), (0.0, top, kw), top + keyboard_h())
                }
            };
        }
        let (rw, rh) = (pairs_across_w(), pairs_across_h());
        let rolls = Some((0.0, 0.0, rw));
        let down = {
            let (wy, py) = (rh + t::GAP, rh + t::GAP + wa(inner) + t::GAP);
            let body = py + pad_h(m, inner);
            Foot::framed(rolls, (0.0, wy, inner), (0.0, py, inner), body)
        };
        let room = inner - rw - t::SPACER;
        if room < t::PAD_KEYS_BESIDE_MIN * KEY - t::GAP_SMALL {
            return down;
        }
        let x = rw + t::SPACER;
        let over = {
            let py = wa(room) + t::GAP;
            let body = rh.max(py + pad_h(m, room));
            Foot::framed(rolls, (x, 0.0, room), (x, py, room), body)
        };
        let pw = pad_plan(m, room).w;
        let left = inner - t::SPACER - pw;
        let under = {
            let body = (rh + t::GAP + wa(left)).max(pad_h(m, pw));
            let pad = (left + t::SPACER, 0.0, pw);
            Foot::framed(rolls, (0.0, rh + t::GAP, left), pad, body)
        };
        [over, under, down]
            .into_iter()
            .min_by(|a, b| a.h.total_cmp(&b.h))
            .unwrap_or(down)
    }

    /// The parts, and the panel's height round a body `body` tall.
    fn framed(
        rolls: Option<(f32, f32, f32)>,
        work: (f32, f32, f32),
        pad: (f32, f32, f32),
        body: f32,
    ) -> Foot {
        Foot {
            rolls,
            work,
            pad,
            h: t::GAP + body + t::GAP,
        }
    }
}

/// Draws the panel where it stands out of the page, for the page area at
/// `x0`, `cw` wide, in a window `h` tall; in the page it is the codex32
/// screen's own.
pub(crate) fn draw(m: &Codex32Make, ui: &mut Ui, at: PanelAt, (x0, cw, h): (f32, f32, f32)) {
    let button = match at {
        PanelAt::Beside => {
            let (x, y, w) = (x0 + t::MARGIN_WIDE, t::SUPPORT_PANE_TOP, t::CODEX32_ENTRY_W);
            ui.fill(x, y, w, beside_h(m), t::SHAPE_M, Role::SurfaceContainerLow);
            let (ix, iw) = (x + t::PAD, beside_inner());
            let mut cy = y + t::PAD;
            let rolling = !complete(m) && m.way() == Way::Dice;
            if rolling {
                cy += pairs(ui, (ix, cy, iw), &super::dice_screen::pairs_of(m), None) + t::GAP;
            }
            cy += super::tracker_screen::draw(m, ui, (ix, cy, iw)) + t::GAP;
            if complete(m) {
                cy += choice(m, ui, (ix, cy, iw));
                continue_button(ui, ix, cy, iw);
                return;
            }
            if m.way() == Way::Typed {
                keyboard(ui, ix, cy, iw);
                return;
            }
            pad(m, ui, ix, cy, iw)
        }
        PanelAt::Foot(fh) => {
            let top = h - fh;
            cover(ui, (x0, top, cw, fh));
            let (ix, inner) = (x0 + t::MARGIN_WIDE, cw - 2.0 * t::MARGIN_WIDE);
            let f = Foot::of(m, inner);
            let at = |(px, py, pw): (f32, f32, f32)| (ix + px, top + t::GAP + py, pw);
            if let Some(r) = f.rolls {
                pairs_across(ui, at(r), &super::dice_screen::pairs_of(m));
            }
            let (wx, wy, ww) = at(f.work);
            let wh = super::tracker_screen::draw(m, ui, (wx, wy, ww));
            let (px, py, pw) = at(f.pad);
            if complete(m) {
                choice(m, ui, (wx, wy + wh, ww));
                continue_button(ui, px, py, pw);
                return;
            }
            if m.way() == Way::Typed {
                keyboard(ui, px, py, pw);
                return;
            }
            pad(m, ui, px, py, pw)
        }
        PanelAt::Page => return,
    };
    size_menu(m, ui, button, h);
}

/// **Continue** at the end of the row at (x, y), `w` wide: the flow's
/// one way on once every character is in.
fn continue_button(ui: &mut Ui, x: f32, y: f32, w: f32) {
    let label = EN.keygen_continue;
    let bw = ui.measure(t::LABEL, W::S, label) + 2.0 * t::SPACER;
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
/// low window (`docs/LAYOUT.md` §16.3): as in the panel, its menu under
/// or over the dice's button. Returns the height.
pub(crate) fn page_pad(m: &Codex32Make, ui: &mut Ui, (x, y, w): (f32, f32, f32), h: f32) -> f32 {
    let button = pad(m, ui, x, y, w);
    size_menu(m, ui, button, h);
    pad_h(m, w)
}

/// "Your dice", the dice's size as one button that opens their menu,
/// the face waiting for its units digit or the ten-faced dice's "0 is
/// 10", and the pad: a key per face and Backspace, each `PAD_KEY_W`
/// square; "Your dice", the button and the keys on one row where they
/// fit, else the label over that row, else the faces in even rows under
/// a row of "Your dice", the button and Backspace ([`pad_plan`]).
/// Returns where the dice's button is.
fn pad(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) -> (f32, f32, f32, f32) {
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
    let faces = usize::from(m.dice_choice().faces);
    let head = EN.codex32_your_dice_head;
    // "Your dice" at the start of a row 48 tall, cut to its room.
    let label_in_row = |ui: &mut Ui, y: f32| {
        let shown = ui.fit(t::LABEL, W::S, head, t::INPUT_PAD_LABEL_W);
        ui.text_mid(
            x,
            y,
            t::TOUCH,
            t::LABEL,
            W::S,
            Role::OnSurfaceVariant,
            &shown,
        );
    };
    let bw = t::DICE_MENU_BUTTON_W;
    let (button, keys_x, top, back) = match p.head {
        Head::InRow => {
            // A tens digit waiting shows in the label's place.
            match (&note, m.pending()) {
                (Some(n), Some(_)) => {
                    let shown = ui.fit(t::LABEL, W::M, n, t::INPUT_PAD_LABEL_W);
                    ui.text_mid(x, y, t::TOUCH, t::LABEL, W::M, ink, &shown);
                }
                _ => label_in_row(ui, y),
            }
            let bx = x + t::INPUT_PAD_LABEL_W + t::GAP;
            let kx = bx + bw + t::GAP;
            ((bx, y, bw, t::TOUCH), kx, y, (kx + faces as f32 * KEY, y))
        }
        Head::Over => {
            crate::screens::section_label(ui, x, y, head);
            if let Some(note) = &note {
                let lw = ui.measure(t::LABEL, W::S, head);
                let shown = ui.fit(t::LABEL, W::M, note, w - lw - t::GAP);
                ui.text(x + lw + t::GAP, y, t::LABEL, W::M, ink, &shown);
            }
            let ky = y + label_h();
            let kx = x + bw + t::GAP;
            ((x, ky, bw, t::TOUCH), kx, ky, (kx + faces as f32 * KEY, ky))
        }
        Head::Grid => {
            label_in_row(ui, y);
            // Backspace at the grid's end, never over the dice's button.
            let bx = x + t::INPUT_PAD_LABEL_W + t::GAP;
            let grid_end = x + p.w;
            let back = ((grid_end - t::PAD_KEY_W).max(bx + bw + t::GAP), y);
            if let Some(note) = &note {
                let nx = bx + bw + t::GAP;
                let shown = ui.fit(t::LABEL, W::M, note, back.0 - t::GAP - nx);
                ui.text_mid(nx, y, t::TOUCH, t::LABEL, W::M, ink, &shown);
            }
            let ky = y + t::TOUCH + t::GAP_SMALL;
            ((bx, y, bw, t::TOUCH), x, ky, back)
        }
    };
    size_button(m, ui, button);
    let key = |i: usize| {
        (
            keys_x + (i % p.per) as f32 * KEY,
            top + (i / p.per) as f32 * KEY,
        )
    };
    let shape = |(kx, ky): (f32, f32)| (kx, ky, Some(t::PAD_KEY_W), t::TOUCH, t::RADIUS_SMALL);
    for f in 1..=faces {
        let face = a(C32::Face(f as u8));
        ui.button_in(shape(key(f - 1)), &f.to_string(), Style::Secondary, face);
    }
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
    button
}

/// The dice's size as Material's outlined button with a drop-down
/// arrow: "d6 ▾"; a press opens the menu of sizes, or closes it.
fn size_button(m: &Codex32Make, ui: &mut Ui, (x, y, w, h): (f32, f32, f32, f32)) {
    let act = a(C32::SizeMenu);
    let r = t::shape_full(h);
    ui.button_in((x, y, Some(w), h, r), "", Style::Secondary, act);
    let label = fill(EN.codex32_die_size, &[&m.dice_choice().faces.to_string()]);
    let size = t::LABEL;
    let lw = ui.measure(size, W::S, &label);
    let aw = t::DROPDOWN_ARROW;
    let lx = x + (w - lw - t::GAP - aw) / 2.0;
    ui.text_mid(lx, y, h, size, W::S, Role::Primary, &label);
    arrow(ui, (lx + lw + t::GAP, y + h / 2.0), Role::Primary.paint());
}

/// Material's drop-down arrow, a triangle pointing down, its top edge's
/// left end at (x, y - a quarter of its width).
fn arrow(ui: &mut Ui, (x, cy): (f32, f32), ink: crate::ui::Paint) {
    let (f, ox, oy) = (ui.f, ui.ox, ui.oy);
    let aw = t::DROPDOWN_ARROW;
    let (top, half) = (cy - t::DROPDOWN_ARROW_RISE, t::DROPDOWN_ARROW_HALF);
    let pts = [(x, top), (x + aw, top), (x + half, top + half)];
    let px: Vec<(f32, f32)> = pts
        .iter()
        .map(|&(px, py)| ((px + ox) * f, (py + oy) * f))
        .collect();
    let c = ui.col(ink);
    ui.c.fill_contours(&[&px], c);
}

/// Material's menu of the dice's sizes, while open: under the dice's
/// button where it fits in the window `h` tall, else over it; each size
/// a row `TOUCH` tall, the size chosen in the selected state with its
/// tick, the one the keyboard is on ringed. A press outside it closes
/// it and reaches nothing under it.
fn size_menu(m: &Codex32Make, ui: &mut Ui, (bx, by, bw, bh): (f32, f32, f32, f32), h: f32) {
    let Some((at, keys)) = m.size_menu() else {
        return;
    };
    let pad = t::GAP;
    let (mw, mh) = (
        bw.max(t::DICE_MENU_W),
        DIE_SIZES.len() as f32 * t::TOUCH + 2.0 * pad,
    );
    let my = if by + bh + mh <= h - t::GAP {
        by + bh
    } else {
        (by - mh).max(t::GAP)
    };
    ui.hit_around(bx, my, mw, mh, a(C32::SizeMenu));
    ui.shadow(bx, my, mw, mh, t::SHAPE_XS, t::ELEVATION_MENU);
    ui.fill(bx, my, mw, mh, t::SHAPE_XS, Role::SurfaceContainer);
    let faces = m.dice_choice().faces;
    for (i, &n) in DIE_SIZES.iter().enumerate() {
        let ry = my + pad + i as f32 * t::TOUCH;
        let act = a(C32::Faces(i as u8));
        let chosen = n == faces;
        if chosen {
            ui.fill(bx, ry, mw, t::TOUCH, 0.0, Role::SecondaryContainer);
        }
        if keys && usize::from(at) == i {
            crate::menu::outline(
                ui,
                (bx, ry, mw, t::TOUCH),
                t::SHAPE_XS,
                t::FOCUS_STROKE,
                Role::Primary.paint(),
            );
        }
        let ic = t::ICON_GLYPH_BUTTON;
        let ink = if chosen {
            Role::OnSecondaryContainer
        } else {
            Role::OnSurface
        };
        if chosen {
            let iy = ry + (t::TOUCH - ic) / 2.0;
            ui.icon(bx + t::PAD, iy, ic, Icon::Done, ic, ink);
        }
        let label = fill(EN.codex32_die_size, &[&n.to_string()]);
        let lx = bx + t::PAD + ic + t::PAD;
        ui.text_mid(lx, ry, t::TOUCH, t::LABEL, W::S, ink, &label);
        ui.hit(bx, ry, mw, t::TOUCH, act);
    }
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
