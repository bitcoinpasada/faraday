//! The Working area of the codex32 entry (`docs/CODEX32.md` §5.7), drawn
//! in the input panel beside the pad: "Working", where the step shown
//! stands ("Step 12 of 15 · character 1", or "Roll 6 · character 2" for
//! the character in progress), Previous and Next as Material's outlined
//! icon buttons, and the step's working in plain arithmetic, the same
//! text the Practice walk reads (`walk_screen.rs`), with what the person
//! enters next once nothing is left to step through. Its height is fixed
//! ([`working_area_h`]), so the pad under it never moves as a step's
//! text changes. Drawing reads the tracker's steps and the kept
//! worksheet; nothing here works anything out but where a line goes.

use faraday_draw::tokens as t;
use faraday_draw::tokens::{BODY_M, TITLE_S};
use faraday_draw::widgets::Icon;

use super::screen::{MASK, a};
use super::walk_screen::step_text;
use super::{C32, Codex32Make, Way};
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::Ui;
use crate::ui::pal::*;

/// Whether Previous and Next stand beside the text in an area `w` wide,
/// else in the first row with the head.
fn side(w: f32) -> bool {
    w >= t::CODEX32_WORKING_SIDE_MIN
}

/// The width of the step's working: beside the two buttons, or the whole
/// width under them.
fn text_w(w: f32) -> f32 {
    if side(w) {
        w - 2.0 * t::TOUCH - t::GAP
    } else {
        w
    }
}

/// The lines kept for a step's working in an area `w` wide: as many as
/// the longest step needs at that width. Arithmetic on tokens.
fn text_lines(w: f32) -> f32 {
    let lines = (t::CODEX32_WORKING_TEXT_MAX / text_w(w).max(1.0)).ceil();
    lines.max(t::CODEX32_WORKING_LINES_MIN)
}

/// The Working area's height in `w` with its Previous and Next standing
/// elsewhere ([`draw_beside`]): its head line and the lines the longest
/// step needs across the whole width, at least two. Arithmetic on tokens.
pub(crate) fn working_text_h(w: f32) -> f32 {
    let lines = (t::CODEX32_WORKING_TEXT_MAX / w.max(1.0)).ceil();
    (1.0 + lines.max(t::CODEX32_WORKING_LINES_MIN)) * t::CODEX32_WORKING_LINE
}

/// The Working area's height in `w`: its head row and the lines kept for
/// a step's working, whatever the step says, so the pad beside it never
/// moves. Arithmetic on tokens.
pub(crate) fn working_area_h(w: f32) -> f32 {
    let text = text_lines(w) * t::CODEX32_WORKING_LINE;
    if side(w) {
        (t::CODEX32_WORKING_LINE + text).max(t::TOUCH)
    } else {
        t::TOUCH + text
    }
}

/// The Working area at (`x`, `y`), `w` wide, for the codex32 entry open
/// in New key: the input panel's one call for it. Returns the height
/// used: [`working_area_h`], none where the entry is not open.
#[allow(dead_code)]
pub(crate) fn working_area(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let Some(m) = app.keygen.state().and_then(|k| k.codex32.as_ref()) else {
        return 0.0;
    };
    draw(m, ui, (x, y, w))
}

/// Where the step shown stands, and what it reads: its working, with
/// what is entered next where it is the last step. Values are read from
/// the step's character and the kept worksheet. With `short`, where it
/// stands in the small panel's few characters ("12/18 · ch 7").
fn lines(m: &Codex32Make, short: bool) -> (String, String) {
    // Past the last checksum, the tour of the shares made and S.
    if let Some(l) = super::tour_screen::lines(m, short) {
        return l;
    }
    let pick = |long: &'static str, brief: &'static str| if short { brief } else { long };
    let next = next_text(m).or_else(|| super::tour_screen::next_text(m));
    let (Some((ch, s, step)), Some(work)) = (m.shown_step(), m.work()) else {
        let count = m.front().map_or(String::new(), |(_, n)| {
            fill(
                pick(EN.codex32_track_char, EN.codex32_track_char_short),
                &[&(n + 1).to_string()],
            )
        });
        return (count, next.unwrap_or_default());
    };
    let n = (ch.n + 1).to_string();
    let count = if ch.whole() {
        let len = ch.steps().len().to_string();
        fill(
            pick(EN.codex32_walk_count, EN.codex32_walk_count_short),
            &[&s.to_string(), &len, &n],
        )
    } else {
        fill(
            pick(EN.codex32_track_roll, EN.codex32_track_roll_short),
            &[&s.to_string(), &n],
        )
    };
    let mut text = step_text(work, ch, step);
    let at_end = if m.walk().shown().is_some() {
        !m.walk_has_more() && m.front().is_none()
    } else {
        m.track_at_end()
    };
    if let Some(next) = next.filter(|_| at_end) {
        text.push(' ');
        text.push_str(&next);
    }
    (count, text)
}

/// "Next: die 3, second roll.", or the box a typed character goes in.
fn next_text(m: &Codex32Make) -> Option<String> {
    let (_, n) = m.front()?;
    if m.way() == Way::Typed {
        return Some(fill(EN.codex32_track_next_box, &[&(10 + n).to_string()]));
    }
    let (die, second) = m.next_roll()?;
    let template = if second {
        EN.codex32_track_next_second
    } else {
        EN.codex32_track_next_first
    };
    Some(fill(template, &[&(die + 1).to_string()]))
}

/// Previous and Next, each with whether it has a step to go to: the
/// Practice walk's while it shows a step (New key sends the press there),
/// else the tracker's.
fn presses(m: &Codex32Make) -> [(C32, bool); 2] {
    let (back, next) = if m.tour_shown().is_some() {
        (true, true)
    } else if m.walk().shown().is_some() {
        (m.walk_can_back(), m.walk_can_next())
    } else {
        (m.track_can(-1), m.track_can(1) || m.tour_can(1))
    };
    [(C32::Step(-1), back), (C32::Step(1), next)]
}

/// The area itself: "Working" and where the step stands on its first
/// line, the step's working under it, and Previous and Next at its right
/// end, 48 dp targets with 40 dp circles. Returns its height.
pub(super) fn draw(m: &Codex32Make, ui: &mut Ui, at: (f32, f32, f32)) -> f32 {
    draw_beside(m, ui, at, None)
}

/// [`draw`], or with Previous and Next in a `TOUCH` row at `buttons`
/// (their pair's left edge and the row's top), the head and the step's
/// working then taking the whole width (`docs/LAYOUT.md` §16.3: at the
/// window's foot they stand at the pad's row's end). Returns the
/// height: [`working_area_h`], or [`working_text_h`].
pub(super) fn draw_beside(
    m: &Codex32Make,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    buttons: Option<(f32, f32)>,
) -> f32 {
    if m.work().is_none() {
        return 0.0;
    }
    let apart = buttons.is_some();
    let h = if apart {
        working_text_h(w)
    } else {
        working_area_h(w)
    };
    let beside = |w: f32| apart || side(w);
    let (count, text) = lines(m, !beside(w));
    let tw = if apart { w } else { text_w(w) };
    // The head's row: a line beside the buttons, or the buttons' own row.
    let head_h = if beside(w) {
        t::CODEX32_WORKING_LINE
    } else {
        t::TOUCH
    };
    // Narrow, the area is the small panel's Working tile, which names it.
    let cx = if beside(w) {
        let head = EN.codex32_track_head;
        x + ui.text_mid_as((x, y, head_h), TITLE_S, Role::OnSurface, head) + t::GAP
    } else {
        x
    };
    let count_end = if beside(w) {
        x + tw
    } else {
        x + w - 2.0 * t::TOUCH
    };
    let count = ui.fit_as(BODY_M, &count, count_end - cx);
    ui.text_mid_as((cx, y, head_h), BODY_M, Role::OnSurfaceVariant, &count);
    // The step's working, masked with the sheets (§6.4.6).
    let text = if m.masked() && !text.is_empty() {
        MASK.repeat(3)
    } else {
        text
    };
    ui.wrap_as((x, y + head_h, tw), BODY_M, Role::OnSurface, &text);
    let bt = (t::TOUCH - t::ICON_BUTTON) / 2.0;
    let row_h = if beside(w) { h } else { t::TOUCH };
    let (bx, by) = match buttons {
        Some((px, py)) => (px + bt, py + (t::TOUCH - t::ICON_BUTTON) / 2.0),
        None => (
            x + w - 2.0 * t::TOUCH + bt,
            y + (row_h - t::ICON_BUTTON) / 2.0,
        ),
    };
    arrows(m, ui, (bx, by));
    h
}

/// Previous and Next as Material's outlined icon buttons, the first's
/// circle at (`bx`, `by`), the second a `TOUCH` after it.
fn arrows(m: &Codex32Make, ui: &mut Ui, (bx, by): (f32, f32)) {
    let [(back, back_on), (next, next_on)] = presses(m);
    let prev_tip = EN.tip_previous_step;
    ui.outlined_icon_button((bx, by), Icon::ChevronLeft, prev_tip, back_on, a(back));
    let next_tip = EN.tip_next_step;
    let nx = bx + t::TOUCH;
    ui.outlined_icon_button((nx, by), Icon::ChevronRight, next_tip, next_on, a(next));
}

/// The lines the Working area takes as one paragraph `w` wide: its head,
/// where the step stands and the longest step's working, run on (owner,
/// 2026-10-25). Arithmetic on tokens.
fn line_lines(w: f32) -> f32 {
    ((t::CODEX32_WORKING_TEXT_MAX + t::CODEX32_WORKING_HEAD_W) / w.max(1.0))
        .ceil()
        .max(1.0)
}

/// The Working area's height as one paragraph `w` wide ([`draw_line`]),
/// whatever the step says, so nothing beside it moves. Arithmetic on
/// tokens.
pub(crate) fn working_line_h(w: f32) -> f32 {
    line_lines(w) * t::CODEX32_WORKING_LINE
}

/// The Working area as one paragraph at (`x`, `y`), `w` wide: "Working",
/// where the step stands and the step's working run on from the same
/// line, wrapping under it only where they do not fit; Previous and Next
/// at `buttons` (their pair's left edge and the row's top, a `TOUCH`
/// row). Returns [`working_line_h`].
pub(super) fn draw_line(
    m: &Codex32Make,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    (px, py): (f32, f32),
) -> f32 {
    if m.work().is_none() {
        return 0.0;
    }
    let line = t::CODEX32_WORKING_LINE;
    let (count, text) = lines(m, false);
    let text = if m.masked() && !text.is_empty() {
        MASK.repeat(3)
    } else {
        text
    };
    let head = EN.codex32_track_head;
    let mut cx = x + ui.text_mid_as((x, y, line), TITLE_S, Role::OnSurface, head) + t::GAP;
    if !count.is_empty() {
        let count = ui.fit_as(BODY_M, &count, (x + w - cx).max(0.0));
        let ink = Role::OnSurfaceVariant;
        cx += ui.text_mid_as((cx, y, line), BODY_M, ink, &count) + t::GAP;
    }
    // The working runs on from the head's line, the rest wrapped under.
    let room = x + w - cx;
    let mut first = String::new();
    let mut rest = text.as_str();
    for (i, _) in text.match_indices(' ').chain([(text.len(), "")]) {
        if ui.measure_as(BODY_M, &text[..i]) > room {
            break;
        }
        first = text[..i].to_string();
        rest = text[i..].trim_start();
    }
    ui.text_mid_as((cx, y, line), BODY_M, Role::OnSurface, &first);
    if !rest.is_empty() {
        ui.wrap_as((x, y + line, w), BODY_M, Role::OnSurface, rest);
    }
    let bt = (t::TOUCH - t::ICON_BUTTON) / 2.0;
    arrows(m, ui, (px + bt, py + bt));
    working_line_h(w)
}
