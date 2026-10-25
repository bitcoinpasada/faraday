//! The codex32 entry's input panel (`docs/LAYOUT.md` §16.3, F114): what
//! the person presses and what it fills, in one view. For dice it holds
//! the rolls of the character being entered (each die's first and second
//! roll, what the pair gives, the pair struck as equal), the step
//! tracker's Working area (`tracker_screen.rs`, `docs/CODEX32.md` §5.7),
//! always beside the pad, and the pad: "Your dice", a key per face and
//! Backspace, each `PAD_KEY_W` square (the dice's size is the Entropy
//! card's, owner 2026-10-25); for characters
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
use super::tracker_screen::{working_area_h, working_line_h, working_text_h};
use super::{C32, Card, Codex32Make, OskPress, Way};
use crate::drawing::View;
use crate::fresh::roller::five_screen::{
    pairs, pairs_across, pairs_across_h, pairs_across_w, pairs_row_h, struck_across,
};
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
    let pane = super::pane_screen::pane_at_now(m, cw, (class, compact, roller));
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
/// height. Of the ways the width allows, the shortest (owner, 2026-10-24
/// and 2026-10-25): all three in one row; the rolls beside the Working
/// area, the pad's one row under both; the rolls beside the Working area
/// over the pad; the Working area under the rolls beside the pad; or all
/// three one under another.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Foot {
    rolls: Option<(f32, f32, f32)>,
    work: (f32, f32, f32),
    pad: (f32, f32, f32),
    /// Where the Working area's Previous and Next stand when not beside
    /// its text, and the latest struck pair when not under the rolls: on
    /// the pad's row, after its keys.
    apart: Option<Apart>,
    /// The Working area drawn as one paragraph, head and working run on
    /// ([`super::tracker_screen::draw_line`]), its Previous and Next at
    /// `apart`'s buttons.
    paragraph: bool,
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
        if room < LABEL + t::PAD_KEYS_BESIDE_MIN * KEY - t::GAP_SMALL {
            return down;
        }
        let x = rw + t::SPACER;
        // The Working area over the pad, beside the rolls.
        let over = {
            let py = wa(room) + t::GAP;
            let body = rh.max(py + pad_h(m, room));
            Foot::framed(rolls, (x, 0.0, room), (x, py, room), body)
        };
        // The pad beside the rolls, the Working area under the rolls. A pad
        // that needs a second row there may take more, narrower, where that
        // leaves the Working area room enough to be shorter; a pad of one
        // row keeps it (owner, 2026-10-25).
        let least = pad_plan(m, room).rows;
        let most = if least > 1 { least + 2 } else { 1 };
        let under = (least..=most)
            .map(|rows| pad_rows(m, rows))
            .filter(|p| p.w <= room)
            .map(|p| {
                let left = inner - t::SPACER - p.w;
                let body = (rh + t::GAP + wa(left)).max(pad_tall(p));
                let pad = (left + t::SPACER, 0.0, p.w);
                Foot::framed(rolls, (0.0, rh + t::GAP, left), pad, body)
            })
            .min_by(|a, b| a.h.total_cmp(&b.h))
            .unwrap_or(down);
        // The rolls beside the Working area's text, the pad under both,
        // the latest struck pair and the Working area's Previous and Next
        // at the end of the pad's last row where it has room for them, so
        // the first row holds only what it draws (owner, 2026-10-25).
        let pad = pad_plan(m, inner);
        let spare = inner - pad.w - 2.0 * t::SPACER - 2.0 * t::TOUCH;
        let below = if spare >= t::INPUT_STRUCK_MIN_W {
            let top = pairs_row_h().max(working_text_h(room)) + t::GAP;
            let ph = pad_tall(pad);
            let last = top + ph - t::TOUCH;
            let apart = Apart {
                struck: Some((pad.w + t::SPACER, last, spare, t::TOUCH)),
                buttons: Some((inner - 2.0 * t::TOUCH, last)),
            };
            let mut f = Foot::framed(rolls, (x, 0.0, room), (0.0, top, pad.w), top + ph);
            f.apart = Some(apart);
            f
        } else {
            let top = rh.max(wa(room)) + t::GAP;
            let body = top + pad_h(m, inner);
            Foot::framed(rolls, (x, 0.0, room), (0.0, top, inner), body)
        };
        let mut ways = vec![over, under, down, below];
        ways.extend(Foot::one_row(m, inner, rw));
        ways.extend(Foot::paragraphs(m, inner, rw));
        ways.into_iter()
            .min_by(|a, b| a.h.total_cmp(&b.h))
            .unwrap_or(down)
    }

    /// All three in one row (owner, 2026-10-25: rolls | pad | working), in
    /// a panel `inner` wide whose rolls are `rw`: the pad beside the
    /// rolls, its keys in one row for six faces or fewer and in up to
    /// three for more, so the Working area keeps its buttons beside its
    /// text; the latest struck pair under the rolls or under the pad's
    /// keys. Arithmetic.
    fn one_row(m: &Codex32Make, inner: f32, rw: f32) -> Vec<Foot> {
        let (wa, rh) = (working_area_h, pairs_across_h());
        let rolls = Some((0.0, 0.0, rw));
        let x = rw + t::SPACER;
        let room = inner - x;
        let mut ways = Vec::new();
        let rows_most = if m.dice_choice().faces > 6 { 3 } else { 1 };
        for rows in 1..=rows_most {
            let pad = pad_rows(m, rows);
            let ww = room - pad.w - t::SPACER;
            if ww < t::CODEX32_WORKING_SIDE_MIN {
                continue;
            }
            let ph = pad_tall(pad);
            let work = (x + pad.w + t::SPACER, 0.0, ww);
            // The struck pair under the rolls, as tall as they draw it.
            let body = rh.max(ph).max(wa(ww));
            ways.push(Foot::framed(rolls, work, (x, 0.0, pad.w), body));
            // Or under the pad's keys, where that is shorter.
            let struck_y = ph + t::GAP_SMALL;
            let body = pairs_row_h()
                .max(struck_y + t::INPUT_ROLLS_LINE)
                .max(wa(ww));
            let mut f = Foot::framed(rolls, work, (x, 0.0, pad.w), body);
            f.apart = Some(Apart {
                struck: Some((
                    x + pad.lead(),
                    struck_y,
                    pad.w - pad.lead(),
                    t::INPUT_ROLLS_LINE,
                )),
                buttons: None,
            });
            ways.push(f);
        }
        ways
    }

    /// The arrangements with the Working area as one paragraph (owner,
    /// 2026-10-25), in a panel `inner` wide whose rolls are `rw`: the pad
    /// over the paragraph beside the rolls, Previous and Next at the
    /// pad's row's end and the latest struck pair under the rolls; and the
    /// rolls and the pad on one row, the struck pair at the keys' end or
    /// under them, over the paragraph across the panel with Previous and
    /// Next at its end. Arithmetic.
    fn paragraphs(m: &Codex32Make, inner: f32, rw: f32) -> Vec<Foot> {
        let (rh, row) = (pairs_across_h(), pairs_row_h());
        let rolls = Some((0.0, 0.0, rw));
        let x = rw + t::SPACER;
        let room = inner - x;
        let bw = 2.0 * t::TOUCH;
        let mut ways = Vec::new();
        let keys_room = room - t::GAP - bw;
        let pad = pad_plan(m, keys_room);
        if pad.w <= keys_room {
            let ty = pad_tall(pad) + t::GAP_SMALL;
            let body = rh.max(ty + working_line_h(room));
            let mut f = Foot::framed(rolls, (x, ty, room), (x, 0.0, pad.w), body);
            f.apart = Some(Apart {
                struck: None,
                buttons: Some((inner - bw, 0.0)),
            });
            f.paragraph = true;
            ways.push(f);
        }
        let pad = pad_plan(m, room);
        if pad.w <= room {
            let ph = pad_tall(pad);
            let spare = room - pad.w - t::SPACER;
            // Where the struck pair goes, and the first row's height.
            let (struck, top) = if spare >= t::INPUT_STRUCK_MIN_W {
                let at = (x + pad.w + t::SPACER, ph - t::TOUCH, spare, t::TOUCH);
                (Some(at), row.max(ph))
            } else if row.max(ph + t::GAP_SMALL + t::INPUT_ROLLS_LINE) <= rh.max(ph) {
                let lead = pad.lead();
                let at = (
                    x + lead,
                    ph + t::GAP_SMALL,
                    pad.w - lead,
                    t::INPUT_ROLLS_LINE,
                );
                (Some(at), row.max(ph + t::GAP_SMALL + t::INPUT_ROLLS_LINE))
            } else {
                (None, rh.max(ph))
            };
            // The paragraph's lines, at least as tall as the buttons'
            // circles; their 48 dp targets reach into the gaps round it.
            let tw = inner - t::GAP - bw;
            let lh = working_line_h(tw).max(t::ICON_BUTTON);
            let ty = top + t::GAP_SMALL;
            let mut f = Foot::framed(rolls, (0.0, ty, tw), (x, 0.0, pad.w), ty + lh);
            f.apart = Some(Apart {
                struck,
                buttons: Some((inner - bw, ty + (lh - t::TOUCH) / 2.0)),
            });
            f.paragraph = true;
            ways.push(f);
        }
        ways
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
            apart: None,
            paragraph: false,
            h: t::GAP + body + t::GAP,
        }
    }
}

/// The struck pair's place (x, y, width, height) and the Working area's
/// Previous and Next (x, y) in a `TOUCH` row where they stand apart from
/// its text, from the panel's inner top-left.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Apart {
    struck: Option<(f32, f32, f32, f32)>,
    buttons: Option<(f32, f32)>,
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
            pad(m, ui, ix, cy, iw);
        }
        PanelAt::Foot(fh) => {
            let top = h - fh;
            cover(ui, (x0, top, cw, fh));
            let (ix, inner) = (x0 + t::MARGIN_WIDE, cw - 2.0 * t::MARGIN_WIDE);
            let f = Foot::of(m, inner);
            let at = |(px, py, pw): (f32, f32, f32)| (ix + px, top + t::GAP + py, pw);
            if let Some(r) = f.rolls {
                let pairs = super::dice_screen::pairs_of(m);
                let struck = f.apart.and_then(|a| a.struck);
                pairs_across(ui, at(r), &pairs, struck.is_none());
                if let Some((sx, sy, sw, sh)) = struck {
                    let (sx, sy, sw) = at((sx, sy, sw));
                    struck_across(ui, (sx, sy, sw, sh), &pairs);
                }
            }
            let (wx, wy, ww) = at(f.work);
            let buttons = f.apart.and_then(|a| a.buttons).map(|(bx, by)| {
                let (bx, by, _) = at((bx, by, 0.0));
                (bx, by)
            });
            let wh = match buttons {
                Some(b) if f.paragraph => super::tracker_screen::draw_line(m, ui, (wx, wy, ww), b),
                _ => super::tracker_screen::draw_beside(m, ui, (wx, wy, ww), buttons),
            };
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
            pad(m, ui, px, py, pw);
        }
        PanelAt::Page => {}
    }
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
        ui.fit(t::LABEL, W::S, EN.codex32_your_dice_head, lw)
    } else {
        String::new()
    };
    let role = Role::OnSurfaceVariant;
    match &note {
        None => {
            ui.text_mid(x, y, t::TOUCH, t::LABEL, W::S, role, &head);
        }
        Some(n) => {
            let lh = ui.line(t::LABEL, W::S);
            let top = y + (t::TOUCH - 2.0 * lh) / 2.0;
            ui.text(x, top, t::LABEL, W::S, role, &head);
            let shown = ui.fit(t::LABEL, W::M, n, lw);
            ui.text(x, top + lh, t::LABEL, W::M, ink, &shown);
        }
    }
    let faces = usize::from(m.dice_choice().faces);
    let kx = x + p.lead();
    let key = |i: usize| (kx + (i % p.per) as f32 * KEY, y + (i / p.per) as f32 * KEY);
    let shape = |(kx, ky): (f32, f32)| (kx, ky, Some(t::PAD_KEY_W), t::TOUCH, t::RADIUS_SMALL);
    for f in 1..=faces {
        let face = a(C32::Face(f as u8));
        ui.button_in(shape(key(f - 1)), &f.to_string(), Style::Secondary, face);
        let (fx, fy) = key(f - 1);
        let at = (fx, fy, t::PAD_KEY_W, t::TOUCH, t::RADIUS_SMALL);
        super::dice_screen::pressed_key(m, ui, at, f as u8);
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
