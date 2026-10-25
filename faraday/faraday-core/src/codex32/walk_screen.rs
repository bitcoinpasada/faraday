//! The step-through as it is drawn in the "Demo only" card
//! (`docs/CODEX32.md` §5.5): Previous, Next and Play, how far Play runs
//! and how fast, and the step's working in plain arithmetic that the
//! Working area beside the pad reads (`tracker_screen.rs`); the card
//! itself draws no step's text but in the compact strip.
//! Every value is read from the walk's character and from the worksheet
//! the work set keeps (§7.4); nothing here works anything out but where
//! a line goes.

use faraday_draw::tokens as t;
use faraday_keys::{Reveal, WorkSet};

use super::screen::a;
use super::walk::{SPEEDS, Speed, Step, WalkAct, WalkChar};
use super::worksheet_screen::{Paper, letter, paper};
use super::{C32, Codex32Make, char_of};
use crate::fresh::roller::five::{REACHES, Reach};
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};

/// A symbol as the booklet writes it: a capital.
pub(super) fn up(v: Option<u8>) -> String {
    v.map_or("·".to_string(), |v| {
        char_of(v).to_ascii_uppercase().to_string()
    })
}

/// What step `step` of `ch` reads: its working in plain arithmetic.
/// Values are read from the character's faces and the worksheet.
pub fn step_text(work: &WorkSet, ch: &WalkChar, step: Step) -> String {
    match step {
        Step::First { k, die, .. } => {
            let v = ch.face(k).to_string();
            fill(EN.codex32_walk_first, &[&(die + 1).to_string(), &v, &v])
        }
        Step::Second { k, first, die, .. } => {
            let (f, v) = (ch.face(first), ch.face(k));
            let d = (die + 1).to_string();
            let (fs, vs) = (f.to_string(), v.to_string());
            match v.cmp(&f) {
                std::cmp::Ordering::Greater => fill(EN.codex32_walk_higher, &[&d, &vs, &vs, &fs]),
                std::cmp::Ordering::Less => fill(EN.codex32_walk_lower, &[&d, &vs, &vs, &fs]),
                std::cmp::Ordering::Equal => fill(EN.codex32_walk_equal, &[&d, &fs, &vs]),
            }
        }
        Step::Char if ch.typed() => {
            let c = up(Some(ch.value()));
            fill(EN.codex32_track_typed, &[&c, &(10 + ch.n).to_string()])
        }
        Step::Char => bits_text(ch),
        Step::Sum { j, i } => sum_text(work, ch.share, usize::from(j), usize::from(i)),
        Step::Table { j } => table_text(work, ch.share, usize::from(j)),
        Step::Below { b } => below_text(work, ch.share, usize::from(b)),
    }
}

/// "Bits 0 1 1 0 1 = 8 + 4 + 1 = 13. Leaf 13 of the tree is D. D goes in
/// box 10."
fn bits_text(ch: &WalkChar) -> String {
    let v = ch.value();
    let bits: Vec<String> = (0..5).rev().map(|p| (v >> p & 1).to_string()).collect();
    let terms: Vec<String> = (0..5)
        .rev()
        .filter(|&p| v >> p & 1 == 1)
        .map(|p| (1u8 << p).to_string())
        .collect();
    let sum = if terms.len() > 1 {
        format!("{} = {v}", terms.join(" + "))
    } else {
        v.to_string()
    };
    let c = up(Some(v));
    fill(
        EN.codex32_walk_bits,
        &[
            &bits.join(" "),
            &sum,
            &v.to_string(),
            &c,
            &c,
            &(10 + ch.n).to_string(),
        ],
    )
}

/// "Box 21, row c: 3 + W = N": a sum cell, the cell above it (or the
/// string's character, or the residue of `ms` on the first row) and the
/// table cell added, as §7.4's working reads them.
fn sum_text(work: &WorkSet, share: usize, j: usize, i: usize) -> String {
    work.worksheet(share, Reveal::PaperForm)
        .map(|sh| paper!(sh, |p| sum_text_of(p, j, i)))
        .unwrap_or_default()
}

/// The two cells sum cell `i` of row `j` of `sh` adds: the cell above it
/// (or the string's character, or the residue of `ms` on the first row)
/// and the table cell.
pub(super) fn sum_inputs(sh: Paper<'_>, j: usize, i: usize) -> (Option<u8>, Option<u8>) {
    let b = sh.sum_box(j) + i;
    if j == 0 {
        (sh.symbol(b), sh.hrp_row().get(i).copied())
    } else {
        let w = sh.step_width(j - 1);
        let above = if i + w < sh.checksum_len() {
            sh.sum_cell(j - 1, i + w)
        } else {
            sh.symbol(b)
        };
        (above, sh.lookup_cell(j - 1, i))
    }
}

/// [`sum_text`] of a lent worksheet.
pub(super) fn sum_text_of(sh: Paper<'_>, j: usize, i: usize) -> String {
    let b = sh.sum_box(j) + i;
    let (above, add) = sum_inputs(sh, j, i);
    let lettered: usize = (0..j).map(|k| sh.step_width(k)).sum();
    fill(
        EN.codex32_walk_sum,
        &[
            &b.to_string(),
            &letter(lettered),
            &up(above),
            &up(add),
            &up(sh.sum_cell(j, i)),
        ],
    )
}

/// "Boxes 17–18 read M Z. Table row MZ is …, written from box 19."
fn table_text(work: &WorkSet, share: usize, j: usize) -> String {
    work.worksheet(share, Reveal::PaperForm)
        .map(|sh| paper!(sh, |p| table_text_of(p, j)))
        .unwrap_or_default()
}

/// [`table_text`] of a lent worksheet.
pub(super) fn table_text_of(sh: Paper<'_>, j: usize) -> String {
    let (from, w) = (sh.sum_box(j), sh.step_width(j));
    let lead: Vec<String> = (0..w).map(|i| up(sh.sum_cell(j, i))).collect();
    let row: String = (0..sh.checksum_len())
        .map(|i| up(sh.lookup_cell(j, i)))
        .collect();
    let to = sh.lookup_box(j).to_string();
    if w == 1 {
        fill(
            EN.codex32_walk_table_one,
            &[&from.to_string(), &lead[0], &lead[0], &row, &to],
        )
    } else {
        fill(
            EN.codex32_walk_table,
            &[
                &from.to_string(),
                &(from + w - 1).to_string(),
                &lead.join(" "),
                &lead.concat(),
                &row,
                &to,
            ],
        )
    }
}

/// "Box 36, from below: 3 + E = 7": a checksum box, the sum cell under
/// it and the table cell between, as §7.4's working reads them.
fn below_text(work: &WorkSet, share: usize, b: usize) -> String {
    let Some(sh) = work.worksheet(share, Reveal::PaperForm) else {
        return String::new();
    };
    let len = sh.checksum_len();
    let Some(j) = (1..=sh.steps()).find(|&j| (sh.sum_box(j)..sh.sum_box(j) + len).contains(&b))
    else {
        return String::new();
    };
    fill(
        EN.codex32_walk_below,
        &[
            &b.to_string(),
            &up(sh.sum_cell(j, b - sh.sum_box(j))),
            &up(sh.lookup_cell(j - 1, b - sh.lookup_box(j - 1))),
            &up(sh.symbol(b)),
        ],
    )
}

/// A reach's name, as Play's extent.
fn reach_name(r: Reach) -> &'static str {
    match r {
        Reach::Pass => EN.roller_reach_pass,
        Reach::Character => EN.roller_reach_character,
        Reach::Share => EN.roller_reach_share,
        Reach::End => EN.roller_to_the_end,
    }
}

/// A speed's name.
fn speed_name(s: Speed) -> &'static str {
    match s {
        Speed::Slow => EN.codex32_walk_slow,
        Speed::Normal => EN.codex32_walk_normal,
        Speed::Fast => EN.codex32_walk_fast,
        Speed::Fastest => EN.codex32_walk_fastest,
    }
}

/// A row of chips from `x`, wrapping in `w`: each a name, whether it is
/// chosen, and its press. Returns the height.
fn chips(ui: &mut Ui, (x, y, w): (f32, f32, f32), items: &[(&str, bool, WalkAct)]) -> f32 {
    let (mut bx, mut cy) = (x, y);
    for &(name, on, act) in items {
        let bw = ui.measure(t::ROLLER_LABEL + 1.0, W::S, name)
            + t::ROLLER_GAP
            + t::ROLLER_GAP
            + t::ROLLER_GAP;
        if bx > x && bx + bw > x + w {
            bx = x;
            cy += t::ROLLER_CHIP + t::ROLLER_GAP / 2.0;
        }
        let style = if on { Style::Primary } else { Style::Secondary };
        bx += ui.button(
            bx,
            cy,
            Some(bw),
            t::ROLLER_CHIP,
            name,
            style,
            a(C32::Walk(act)),
        ) + t::ROLLER_GAP / 2.0;
    }
    cy + t::ROLLER_CHIP - y
}

/// The step-through at (`x`, `y`) in `w`: the heads, Previous, Next and
/// Play, then Play's extent and speed. Returns the height.
pub(super) fn draw(m: &Codex32Make, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = y;
    let head = t::ROLLER_LABEL + 1.0;
    ui.text(
        x,
        cy,
        head,
        W::S,
        Role::OnSurface,
        EN.roller_or_let_faraday_roll,
    );
    cy += ui.line(head, W::S) + t::ROLLER_GAP / 2.0;
    ui.text(
        x,
        cy,
        t::ROLLER_LABEL,
        W::R,
        Role::OnSurfaceVariant,
        EN.roller_by_this_computer,
    );
    cy += ui.line(t::ROLLER_LABEL, W::R) + t::ROLLER_GAP;
    // The rolls themselves are on the dice sheet beside it: the card
    // holds what the computer does and the step's working.
    cy += controls(m, ui, (x, cy, w)) + t::ROLLER_GAP;
    cy += options(m, ui, (x, cy, w));
    cy - y
}

/// Where the walk stands and the step's line for the compact strip:
/// "Step 14 of 19 · character 3", and its working, or what Next does
/// first.
fn lines(m: &Codex32Make) -> (String, String) {
    match (m.walk().shown(), m.work()) {
        (Some((ch, s, step)), Some(work)) => (
            fill(
                EN.codex32_walk_count,
                &[
                    &s.to_string(),
                    &ch.steps().len().to_string(),
                    &(ch.n + 1).to_string(),
                ],
            ),
            step_text(work, ch, step),
        ),
        _ => {
            let n = m.front().map_or(0, |(_, n)| n + 1);
            (
                fill(EN.codex32_walk_ready, &[&n.to_string()]),
                String::new(),
            )
        }
    }
}

/// The three buttons' labels, styles and presses: Previous, Next and
/// Play (Pause while it runs).
fn buttons(m: &Codex32Make) -> [(&'static str, Style, C32); 3] {
    let walk = m.walk();
    let style = |on: bool, s: Style| if on { s } else { Style::Disabled };
    let next = m.walk_can_next();
    let play = if walk.playing() {
        (EN.codex32_walk_pause, Style::Secondary, WalkAct::Pause)
    } else {
        (
            EN.codex32_walk_play,
            style(next, Style::Secondary),
            WalkAct::Play,
        )
    };
    [
        (
            EN.codex32_previous,
            style(m.walk_can_back(), Style::Secondary),
            C32::Walk(WalkAct::Previous),
        ),
        (
            EN.codex32_next,
            style(next, Style::Primary),
            C32::Walk(WalkAct::Next),
        ),
        (play.0, play.1, C32::Walk(play.2)),
    ]
}

/// Previous, Next and Play in one row at (`x`, `y`) in `w`, each at a
/// place that depends on no step's text: Play and Pause share the
/// wider's width, and the row starts at `x` or ends at the right edge
/// (`right`). Measures only the labels, which never change. Returns the
/// height.
fn button_row(m: &Codex32Make, ui: &mut Ui, (x, y, w): (f32, f32, f32), right: bool) -> f32 {
    let size = t::LABEL;
    let items = buttons(m);
    let play_w = ui.measure(size, W::S, EN.codex32_walk_play).max(ui.measure(
        size,
        W::S,
        EN.codex32_walk_pause,
    ));
    let text = [
        ui.measure(size, W::S, EN.codex32_previous),
        ui.measure(size, W::S, EN.codex32_next),
        play_w,
    ];
    let gap = t::GAP_SMALL;
    // A narrow row takes the padding in, not the buttons' places.
    let pad = ((w - 2.0 * gap - text.iter().sum::<f32>()) / 3.0).clamp(t::GAP, t::SPACER);
    let total = text.iter().sum::<f32>() + 3.0 * pad + 2.0 * gap;
    let mut bx = if right { x + w - total } else { x };
    for (k, (label, style, act)) in items.into_iter().enumerate() {
        let bw = text[k] + pad;
        let label = ui.fit(size, W::S, label, bw - t::GAP);
        ui.button(bx, y, Some(bw), t::TOUCH, &label, style, a(act));
        bx += bw + gap;
    }
    t::TOUCH
}

/// The height [`strip_controls`] takes: the step's line on one row and
/// the buttons under it. Arithmetic on the font.
pub(super) fn strip_controls_h(ui: &Ui) -> f32 {
    ui.line(t::LABEL, W::S) + t::GAP_SMALL + t::TOUCH
}

/// The step-through in the pinned strip at (`x`, `y`) in `w`: the step's
/// line on one row, cut where it is longer (More opens the whole card),
/// and Previous, Next and Play at the strip's right end. Returns the
/// height.
pub(super) fn strip_controls(m: &Codex32Make, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let (count, working) = lines(m);
    let line = if working.is_empty() {
        count
    } else {
        format!("{count}. {working}")
    };
    let line = ui.fit(t::LABEL, W::S, &line, w);
    ui.text(x, y, t::LABEL, W::S, Role::OnSurfaceVariant, &line);
    let cy = y + strip_controls_h(ui) - t::TOUCH;
    button_row(m, ui, (x, cy, w), true);
    strip_controls_h(ui)
}

/// Previous, Next and Play (or Pause) at their place, in one row. Where
/// the walk stands and the step's working are the Working area's, beside
/// the pad (`docs/CODEX32.md` §5.7), so the card draws neither (owner,
/// 2026-10-25). Returns the height.
fn controls(m: &Codex32Make, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    button_row(m, ui, (x, y, w), false)
}

/// Previous, Next and Play (or Pause) one under another at (`x`, `y`),
/// each `w` wide and `TOUCH` tall, `GAP` apart: the narrow column's
/// buttons (`docs/LAYOUT.md` §16.5), at places no step's text moves.
/// Returns the height.
pub(super) fn stacked(m: &Codex32Make, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = y;
    for (k, (label, style, act)) in buttons(m).into_iter().enumerate() {
        if k > 0 {
            cy += t::GAP;
        }
        let label = ui.fit(t::LABEL, W::S, label, w - t::PAD);
        ui.button(x, cy, Some(w), t::TOUCH, &label, style, a(act));
        cy += t::TOUCH;
    }
    cy - y
}

/// How far Play runs and how fast. Returns the height.
fn options(m: &Codex32Make, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = y;
    let walk = m.walk();
    ui.text(
        x,
        cy,
        t::ROLLER_LABEL,
        W::R,
        Role::OnSurfaceVariant,
        EN.codex32_walk_play_runs,
    );
    cy += ui.line(t::ROLLER_LABEL, W::R) + t::ROLLER_GAP / 2.0;
    let reaches: Vec<(&str, bool, WalkAct)> = REACHES
        .iter()
        .map(|&r| (reach_name(r), walk.reach() == r, WalkAct::Reach(r)))
        .collect();
    cy += chips(ui, (x, cy, w), &reaches) + t::ROLLER_GAP;
    ui.text(
        x,
        cy,
        t::ROLLER_LABEL,
        W::R,
        Role::OnSurfaceVariant,
        EN.codex32_walk_speed,
    );
    cy += ui.line(t::ROLLER_LABEL, W::R) + t::ROLLER_GAP / 2.0;
    let speeds: Vec<(&str, bool, WalkAct)> = SPEEDS
        .iter()
        .enumerate()
        .map(|(i, &s)| (speed_name(s), walk.speed() == s, WalkAct::Speed(i as u8)))
        .collect();
    cy += chips(ui, (x, cy, w), &speeds);
    cy - y
}
