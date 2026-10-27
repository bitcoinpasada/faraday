//! The tour of the shares made and S as it is drawn (`docs/CODEX32.md`
//! §5.7, `tour.rs`): what each step reads in the Working area, where the
//! tour stands ("Step 3 of 375 · Share E"), the choice **Show the other
//! shares and S as they fill** under the Working area once every
//! character is in, and a share made's checksum worked as the booklet's
//! verification on its tab while the tour is on it. Every value is read
//! from the kept translation worksheets and checks; nothing here works
//! anything out but where a line goes.

use faraday_draw::geom::WindowClass;
use faraday_draw::tokens as t;
use faraday_draw::tokens::TITLE_S;
use faraday_keys::{Reveal, WorkSet};

use super::screen::{a, glyph, target_name};
use super::tour::{CHECK_SHEET, TourStep};
use super::walk::Step;
use super::walk_screen::{sum_inputs, sum_text_of, table_text_of, up};
use super::worksheet_screen::{self, Paper, Sheet, paper};
use super::{C32, Codex32Make};
use crate::strings::{EN, fill};
use crate::ui::Ui;
use crate::ui::pal::*;

/// An initial share's name, its capital: "A".
fn input_name(i: u8) -> String {
    char::from(i.to_ascii_uppercase()).to_string()
}

/// What tour step `step` reads: its working in plain arithmetic, read
/// from the kept translation worksheet and the share's check.
pub fn tour_text(m: &Codex32Make, work: &WorkSet, step: TourStep) -> String {
    let n = step.sheet();
    let Some(ts) = work.live_translation(n, Reveal::PaperForm) else {
        return String::new();
    };
    let tg = ts.target_index();
    match step {
        TourStep::Symbol { i, .. } => {
            let i = usize::from(i);
            let v = ts.symbol(i);
            let (name, sym, val) = (input_name(ts.input_index(i)), glyph(v), v.to_string());
            if !ts.is_recovery() {
                let to = input_name(tg);
                return fill(EN.codex32_tour_symbol, &[&name, &to, &sym, &val]);
            }
            let pairs: Vec<String> = (0..ts.threshold())
                .filter(|&j| j != i)
                .map(|j| glyph(ts.pair(i, j)))
                .collect();
            if pairs.len() < 2 {
                fill(EN.codex32_tour_recovery_one, &[&name, &sym, &val])
            } else {
                let fused = pairs.join(" × ");
                fill(EN.codex32_tour_recovery, &[&name, &fused, &sym, &val])
            }
        }
        TourStep::Translate { i, b, .. } => {
            let (i, b) = (usize::from(i), usize::from(b));
            let sym = glyph(ts.symbol(i));
            fill(
                EN.codex32_tour_translate,
                &[
                    &input_name(ts.input_index(i)),
                    &b.to_string(),
                    &up(ts.given_cell(i, b)),
                    &sym,
                    &up(ts.translated_cell(i, b)),
                    &sym,
                ],
            )
        }
        TourStep::Sum { b, .. } => {
            let b = usize::from(b);
            let parts: Vec<String> = (0..ts.threshold())
                .map(|i| up(ts.translated_cell(i, b)))
                .collect();
            fill(
                EN.codex32_tour_sum,
                &[
                    &b.to_string(),
                    &target_name(tg),
                    &parts.join(" + "),
                    &up(ts.sum_cell(b)),
                ],
            )
        }
        TourStep::Check { step, .. } => m
            .tour_check(n)
            .and_then(|ch| ch.worksheet(Reveal::PaperForm))
            .map(|sh| paper!(sh, |p| check_text(p, step)))
            .unwrap_or_default(),
        TourStep::Holds { .. } => m
            .tour_check(n)
            .and_then(|ch| ch.worksheet(Reveal::PaperForm))
            .map(|sh| paper!(sh, |p| holds_text(p, &target_name(tg))))
            .unwrap_or_default(),
    }
}

/// A step of a share's checksum check: a sum row's cell or a table row
/// as the walk reads them, a cell of the final row with what it should
/// read.
fn check_text(sh: Paper<'_>, step: Step) -> String {
    match step {
        Step::Sum { j, i } if usize::from(j) == sh.steps() => {
            let (j, i) = (usize::from(j), usize::from(i));
            let (above, add) = sum_inputs(sh, j, i);
            fill(
                EN.codex32_tour_final,
                &[
                    &(sh.sum_box(j) + i).to_string(),
                    &up(above),
                    &up(add),
                    &up(sh.sum_cell(j, i)),
                    &up(sh.target().get(i).copied()),
                ],
            )
        }
        Step::Sum { j, i } => sum_text_of(sh, usize::from(j), usize::from(i)),
        Step::Table { j } => table_text_of(sh, usize::from(j)),
        _ => String::new(),
    }
}

/// The check's end: its final row as it should read, or the first box
/// where it does not.
fn holds_text(sh: Paper<'_>, name: &str) -> String {
    let j = sh.steps();
    let differs =
        (0..sh.checksum_len()).find(|&i| sh.sum_cell(j, i) != sh.target().get(i).copied());
    match differs {
        None => fill(EN.codex32_tour_holds, &[name]),
        Some(i) => fill(
            EN.codex32_tour_fails,
            &[name, &(sh.sum_box(j) + i).to_string()],
        ),
    }
}

/// Where the tour stands and what its step reads, for the Working area;
/// at its last step with what Next does there. With `short`, as the small
/// panel's tile says it ("12/375 · E"). None while it shows no step.
pub(super) fn lines(m: &Codex32Make, short: bool) -> Option<(String, String)> {
    let (n, step) = m.tour_shown()?;
    let work = m.work()?;
    let tg = work.targets().get(step.sheet()).copied().unwrap_or(b's');
    let len = m.tour().steps().len();
    let count = if short {
        let letter = input_name(tg);
        fill(
            EN.codex32_tour_count_short,
            &[&n.to_string(), &len.to_string(), &letter],
        )
    } else {
        fill(
            EN.codex32_tour_count,
            &[&n.to_string(), &len.to_string(), &target_name(tg)],
        )
    };
    let mut text = tour_text(m, work, step);
    if n == len {
        text.push(' ');
        text.push_str(EN.codex32_tour_next_card);
    }
    Some((count, text))
}

/// What Next does at the last checksum box, where the tour is ready:
/// "Next: Share E."
pub(super) fn next_text(m: &Codex32Make) -> Option<String> {
    if !m.tour_ready() {
        return None;
    }
    let tg = *m.work()?.targets().first()?;
    Some(fill(EN.codex32_tour_next, &[&target_name(tg)]))
}

/// The choice's height: a list row once every character is in and there
/// are shares made, else none. A lookup.
pub(crate) fn choice_h(m: &Codex32Make) -> f32 {
    let made = m
        .work()
        .is_some_and(|w| !w.targets().is_empty() && !w.is_given());
    if m.strings().is_some() && made {
        t::TOUCH
    } else {
        0.0
    }
}

/// **Show the other shares and S as they fill**: Material's switch at a
/// list row's end, at (`x`, `y`) in `w`, under the Working area once
/// every character is in. Returns its height ([`choice_h`]).
pub(crate) fn choice(m: &Codex32Make, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let h = choice_h(m);
    if h > 0.0 {
        let on = m.tour().on();
        ui.switch_row((x, y, w), EN.codex32_tour_choice, on, a(C32::Tour));
    }
    h
}

/// Share made `t`'s checksum, worked as the booklet's verification, on
/// its tab while the tour is on it: headed, the cells of the step shown
/// outlined, in the entry's view of the worksheet (or whole, at the
/// fitted cell). Its cells open nothing. Returns the height.
pub(super) fn check_sheet(
    m: &Codex32Make,
    n: usize,
    ui: &mut Ui,
    (x, y, w, cls, hide): (f32, f32, f32, WindowClass, bool),
) -> f32 {
    let (Some(sh), Some(work)) = (
        m.tour_check(n).and_then(|c| c.worksheet(Reveal::PaperForm)),
        m.work(),
    ) else {
        return 0.0;
    };
    let tg = work.targets().get(n).copied().unwrap_or(b's');
    let head = fill(EN.codex32_tour_check_head, &[&target_name(tg)]);
    let hh = ui.wrap_as((x, y, w), TITLE_S, Role::OnSurface, &head) + t::GAP_SMALL;
    let marks = m.tour_check_cells(n);
    let s = |p| Sheet {
        sh: p,
        id: CHECK_SHEET + n as u8,
        front: None,
        hide,
        press: false,
        only_final: false,
        marks: &marks,
    };
    let fit = (cls, m.sheet_cell());
    let h = paper!(sh, |p| worksheet_screen::draw(
        ui,
        s(p),
        m.pan(),
        (x, y + hh, w),
        fit
    ));
    hh + h + t::GAP
}
