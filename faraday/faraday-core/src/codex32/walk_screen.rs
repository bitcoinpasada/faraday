//! The computer's walk on Make codex32 shares
//! (`docs/CODEX32.md` §5.5): Previous, Next and Play, how far Play runs
//! and how fast, and the step's working in plain arithmetic that the
//! Working area beside the pad reads (`tracker_screen.rs`). Previous,
//! Next, Play and Play's options stand in Faraday Desktop's host strip
//! (`docs/DESKTOP-HOST.md` §3.1), which reads them from here.
//! Every value is read from the walk's character and from the worksheet
//! the work set keeps (§7.4); nothing here works anything out but where
//! a line goes.

use faraday_keys::{Reveal, WorkSet};

use super::walk::{SPEEDS, Speed, Step, WalkAct, WalkChar};
use super::worksheet_screen::{Paper, letter, paper};
use super::{C32, Codex32Make, char_of};
use crate::fresh::roller::five::{REACHES, Reach};
use crate::strings::{EN, fill};
use crate::ui::Style;

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

/// One of the entry's demo aids as Faraday Desktop's host strip offers it
/// (`docs/DESKTOP-HOST.md` §3): its label, whether it takes a press,
/// whether it is the step's main press, and the press.
pub(crate) type HostAid = (&'static str, bool, bool, C32);

/// What the host strip offers for the entry: Demo's Use a sample while a
/// character is left, and the computer's walk, Previous, Next and Play
/// (Pause while it runs), where the entry's "Demo only" card would hold
/// them; none elsewhere. A lookup.
pub(crate) fn host_aids(m: &Codex32Make, roller: bool) -> Vec<HostAid> {
    let entering = m.open() == Some(super::Card::Enter);
    if !entering || !super::dice_screen::demo_box_offered(m, roller) {
        return Vec::new();
    }
    let mut aids = Vec::new();
    if m.demo() && m.front().is_some() {
        aids.push((EN.use_a_sample, true, false, C32::Sample));
    }
    if roller && m.way() == super::Way::Dice {
        for (label, style, act) in buttons(m) {
            aids.push((label, !style.is_off(), style == Style::Primary, act));
        }
    }
    aids
}

/// Whether the host strip offers Play's options: while the computer's
/// walk is offered. A lookup.
pub(crate) fn host_walks(m: &Codex32Make, roller: bool) -> bool {
    m.open() == Some(super::Card::Enter)
        && roller
        && m.way() == super::Way::Dice
        && super::dice_screen::demo_box_offered(m, roller)
}

/// A heading of Play's options and its choices: a name, whether it is
/// chosen, and its press.
pub(crate) type PlayOptions = (&'static str, Vec<(&'static str, bool, C32)>);

/// Play's options as the host's menu lists them: how far Play runs, then
/// how fast, each a heading and its choices, a name, whether it is
/// chosen and its press. A lookup.
pub(crate) fn host_play_options(m: &Codex32Make) -> [PlayOptions; 2] {
    let walk = m.walk();
    let reaches = REACHES
        .iter()
        .map(|&r| {
            (
                reach_name(r),
                walk.reach() == r,
                C32::Walk(WalkAct::Reach(r)),
            )
        })
        .collect();
    let speeds = SPEEDS
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let act = C32::Walk(WalkAct::Speed(i as u8));
            (speed_name(s), walk.speed() == s, act)
        })
        .collect();
    [
        (EN.codex32_walk_play_runs, reaches),
        (EN.codex32_walk_speed, speeds),
    ]
}
