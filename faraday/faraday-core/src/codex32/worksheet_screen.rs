//! The checksum worksheet drawn as the codex32 booklet draws it
//! (`docs/CODEX32.md` §7.1, §7.5): the booklet's staircase, cell for
//! cell. The first row is boxes 1 to 16 (`MS1` and the first 13
//! characters), bolded and numbered; under it `+` and the residue of
//! `ms`; then, a step at a time, an `=` row (the sum, its two leading
//! cells lettered a, b, c, … in their bottom left corners, and the next
//! characters of the string brought down at its end, bolded and
//! numbered) and a `+` row (the checksum table's row the leading cells
//! select); last, `=` and the final row, which reads `SECRETSHARE32`.
//! Columns are the string's boxes, in groups of four; rows go in pairs.
//! The `=` rows' cells in the checksum's columns are pink.
//!
//! Where the whole sheet fits the room at a cell from
//! `CODEX32_CELL_FIT_MIN` to `CODEX32_CELL_FIT_MAX`, it is drawn whole at
//! the largest that fits, kept on the flow (`docs/LAYOUT.md` §16.1), and
//! the page follows the work. Else it is drawn in a view no taller than
//! [`t::codex32_sheet_view`] and as wide as its column, and pans both ways (`ui::Slot::Worksheet`, [`SheetPan`]): a drag
//! of the mouse or a finger, the wheel (Shift with it, across), the
//! arrow keys. The entry keeps its front cell in view
//! ([`super::Codex32Make::follow_front`]). Only the cells in the view are
//! drawn. Every value is read from the worksheet the work set keeps;
//! nothing here computes but the cells' places.

use crate::ui::Paint;
use faraday_draw::geom::WindowClass;
use faraday_draw::tokens as t;
use faraday_draw::tokens::{BODY_M, TITLE_S};
use faraday_keys::{Reveal, WorkSet};

use super::screen::share_name;
use super::{C32, Cell, Codex32Make, SheetPan, char_of};
use crate::Action;
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Slot, Ui, W};

/// Boxes 1 to 3 of every string, as the booklet prints them.
const PREFIX: &[u8] = b"MS1";

/// What a masked character is drawn as.
const MASK: &str = "\u{2022}";

/// What drawing reads of a checksum worksheet faraday-keys lends as a
/// paper form: its shape, and its cells through the worksheet's own
/// accessors, never copied. Made only by [`paper!`], at the place the
/// worksheet is borrowed, so the worksheet's type is never named outside
/// faraday-keys (`tools/lint-secrets.sh` rule 4e).
#[derive(Clone, Copy)]
pub(crate) struct Paper<'a> {
    /// Characters in the string, `ms1` included.
    pub(crate) chars: usize,
    /// Characters of checksum: 13, or 15 at 127 characters.
    pub(crate) len: usize,
    /// Steps of the ladder.
    pub(crate) steps: usize,
    /// The last box before the checksum.
    pub(crate) last_data: usize,
    /// The residue of `ms`.
    pub(crate) hrp: &'static [u8],
    /// What the final row must read.
    pub(crate) target: &'static [u8],
    /// The box of sum row `j`'s first cell.
    pub(crate) sum_box: &'a dyn Fn(usize) -> usize,
    /// Characters step `j` takes.
    pub(crate) width: &'a dyn Fn(usize) -> usize,
    /// The character given at box `b`.
    pub(crate) symbol: &'a dyn Fn(usize) -> Option<u8>,
    /// Cell `i` of sum row `j` (row `steps`: the final row).
    pub(crate) sum: &'a dyn Fn(usize, usize) -> Option<u8>,
    /// Cell `i` of lookup row `j`.
    pub(crate) lookup: &'a dyn Fn(usize, usize) -> Option<u8>,
}

impl Paper<'_> {
    pub(crate) fn chars(&self) -> usize {
        self.chars
    }

    pub(crate) fn checksum_len(&self) -> usize {
        self.len
    }

    pub(crate) fn steps(&self) -> usize {
        self.steps
    }

    pub(crate) fn last_data_box(&self) -> usize {
        self.last_data
    }

    pub(crate) fn hrp_row(&self) -> &'static [u8] {
        self.hrp
    }

    pub(crate) fn target(&self) -> &'static [u8] {
        self.target
    }

    pub(crate) fn sum_box(&self, j: usize) -> usize {
        (self.sum_box)(j)
    }

    pub(crate) fn step_width(&self, j: usize) -> usize {
        (self.width)(j)
    }

    pub(crate) fn lookup_box(&self, j: usize) -> usize {
        self.sum_box(j) + self.step_width(j)
    }

    pub(crate) fn symbol(&self, b: usize) -> Option<u8> {
        (self.symbol)(b)
    }

    pub(crate) fn sum_cell(&self, j: usize, i: usize) -> Option<u8> {
        (self.sum)(j, i)
    }

    pub(crate) fn lookup_cell(&self, j: usize, i: usize) -> Option<u8> {
        (self.lookup)(j, i)
    }

    pub(crate) fn final_cell(&self, i: usize) -> Option<u8> {
        self.sum_cell(self.steps, i)
    }
}

/// Runs `$body` with `$p`, the [`Paper`] of the lent worksheet `$sh`.
macro_rules! paper {
    ($sh:expr, |$p:ident| $body:expr) => {{
        let sh = $sh;
        let sum_box = |j| sh.sum_box(j);
        let width = |j| sh.step_width(j);
        let symbol = |b| sh.symbol(b);
        let sum = |j, i| sh.sum_cell(j, i);
        let lookup = |j, i| sh.lookup_cell(j, i);
        let $p = $crate::codex32::worksheet_screen::Paper {
            chars: sh.chars(),
            len: sh.checksum_len(),
            steps: sh.steps(),
            last_data: sh.last_data_box(),
            hrp: sh.hrp_row(),
            target: sh.target(),
            sum_box: &sum_box,
            width: &width,
            symbol: &symbol,
            sum: &sum,
            lookup: &lookup,
        };
        $body
    }};
}
pub(crate) use paper;

/// Where a checksum worksheet's cells are, by the window's class: pure
/// arithmetic on box and row numbers.
#[derive(Clone, Copy)]
pub(crate) struct Geom {
    /// A cell's side.
    pub(crate) cell: f32,
    /// The gap between groups of four columns and between pairs of rows.
    gap: f32,
    /// Room round the sheet for the front cell's outline.
    pad: f32,
}

impl Geom {
    /// The sheet's geometry in its view on a window of class `cls`.
    pub(crate) fn of(cls: WindowClass) -> Geom {
        Geom::at(t::codex32_cell(cls))
    }

    /// The sheet's geometry at a cell `cell` dp a side.
    pub(crate) fn at(cell: f32) -> Geom {
        Geom {
            cell,
            gap: t::CODEX32_SHEET_GAP,
            pad: t::CODEX32_FRONT_STROKE,
        }
    }

    /// The sheet's geometry as it is drawn: whole at the fitted cell
    /// `fit` where there is one, else in its view at the class's.
    pub(crate) fn drawn(fit: Option<f32>, cls: WindowClass) -> Geom {
        fit.map_or_else(|| Geom::of(cls), Geom::at)
    }

    /// The left edge of box `b`'s column, from the sheet's.
    fn col(&self, b: usize) -> f32 {
        let i = b.saturating_sub(1);
        self.pad + i as f32 * self.cell + (i / 4) as f32 * self.gap
    }

    /// The top of row `r` (0 the first row), from the sheet's.
    fn row(&self, r: usize) -> f32 {
        self.pad + r as f32 * self.cell + (r / 2) as f32 * self.gap
    }

    /// The whole sheet's size: across and down.
    fn size(&self, sh: Paper<'_>) -> (f32, f32) {
        (
            self.col(sh.chars()) + self.cell + self.pad,
            self.row(final_row(sh)) + self.cell + self.pad,
        )
    }

    /// Where box `b` stands as a given (bolded) cell: in the first row, or
    /// at the end of the `=` row it is brought down to; its left, top and
    /// side.
    pub(crate) fn box_spot(&self, sh: Paper<'_>, b: usize) -> Option<(f32, f32, f32, f32)> {
        let r = given_row(sh, b)?;
        Some((self.col(b), self.row(r), self.cell, self.cell))
    }

    /// Where `cell` stands on the sheet: its left, top, width and height;
    /// none for a cell of another kind of worksheet.
    pub(crate) fn cell_spot(&self, sh: Paper<'_>, cell: Cell) -> Option<(f32, f32, f32, f32)> {
        let at = |b: usize, r: usize| Some((self.col(b), self.row(r), self.cell, self.cell));
        match cell {
            Cell::Given { b, .. } => self.box_spot(sh, usize::from(b)),
            Cell::Hrp { i, .. } => at(4 + usize::from(i), 1),
            Cell::Sum { j, i, .. } => {
                let j = usize::from(j);
                at(sh.sum_box(j) + usize::from(i), 2 + 2 * j)
            }
            Cell::Lookup { j, i, .. } => {
                let j = usize::from(j);
                at(sh.lookup_box(j) + usize::from(i), 3 + 2 * j)
            }
            _ => None,
        }
    }
}

/// The largest whole cell from `CODEX32_CELL_FIT_MIN` to
/// `CODEX32_CELL_FIT_MAX` at which the whole sheet `sh` fits `room` (wide,
/// tall, dp), or none where it does not fit at the least
/// (`docs/LAYOUT.md` §16.1). Arithmetic.
pub(crate) fn fit_cell(sh: Paper<'_>, room: (f32, f32)) -> Option<f32> {
    let mut cell = t::CODEX32_CELL_FIT_MAX;
    while cell >= t::CODEX32_CELL_FIT_MIN {
        let (w, h) = Geom::at(cell).size(sh);
        if w <= room.0 && h <= room.1 {
            return Some(cell);
        }
        cell -= t::CODEX32_CELL_FIT_STEP;
    }
    None
}

/// The sheet's size at its least fitting cell, `CODEX32_CELL_FIT_MIN`:
/// across and down. Arithmetic.
pub(crate) fn least_size(sh: Paper<'_>) -> (f32, f32) {
    Geom::at(t::CODEX32_CELL_FIT_MIN).size(sh)
}

/// The smallest rectangle holding both `a` and `b` (left, top, width,
/// height).
pub(crate) fn union(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    let (x, y) = (a.0.min(b.0), a.1.min(b.1));
    let (r, bt) = ((a.0 + a.2).max(b.0 + b.2), (a.1 + a.3).max(b.1 + b.3));
    (x, y, r - x, bt - y)
}

/// The final row's number: after the first row, the residue's row and a
/// pair of rows per step.
fn final_row(sh: Paper<'_>) -> usize {
    2 + 2 * sh.steps()
}

/// The last box of the first row: `MS1` and the first row of data.
fn head_last(sh: Paper<'_>) -> usize {
    3 + sh.checksum_len()
}

/// The row box `b` is given in: 0 for the first row, else the `=` row of
/// the step that brings it down.
fn given_row(sh: Paper<'_>, b: usize) -> Option<usize> {
    if b == 0 || b > sh.chars() {
        return None;
    }
    if b <= head_last(sh) {
        return Some(0);
    }
    let len = sh.checksum_len();
    (0..sh.steps())
        .find(|&j| {
            let from = sh.sum_box(j) + len;
            (from..from + sh.step_width(j)).contains(&b)
        })
        .map(|j| 2 + 2 * j)
}

/// The letter of a leading cell, counted from 0: a to z, then aa, ab, …,
/// as the booklet letters them.
pub(crate) fn letter(n: usize) -> String {
    let mut n = n + 1;
    let mut out = Vec::new();
    while n > 0 {
        n -= 1;
        out.push(b'a' + (n % 26) as u8);
        n /= 26;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

/// A character of the sheet, as the booklet writes it: a capital.
fn upper(v: u8) -> String {
    char_of(v).to_ascii_uppercase().to_string()
}

/// One cell as drawn.
struct Drawn {
    x: f32,
    y: f32,
    /// What it reads, and its colour.
    text: Option<(String, Paint)>,
    /// Bolded: the string's own characters.
    bold: bool,
    /// In the checksum's columns of an `=` row.
    pink: bool,
    /// Its box number, top right (the bolded cells).
    number: Option<usize>,
    /// Its letter, bottom left (an `=` row's leading cells).
    letter: Option<String>,
    /// A final-row cell that differs from what the row must read.
    differs: bool,
    /// What a press on it opens.
    press: Option<Cell>,
    /// Which cell it is, for the step-through's marks.
    id: Option<Cell>,
}

/// The worksheet to draw and how.
#[derive(Clone, Copy)]
pub(crate) struct Sheet<'a> {
    /// The worksheet.
    pub(crate) sh: Paper<'a>,
    /// Its number, for the cells' presses.
    pub(crate) id: u8,
    /// The box being filled, outlined in the accent.
    pub(crate) front: Option<usize>,
    /// Characters are drawn as dots (`docs/CODEX32.md` §6.4.6).
    pub(crate) hide: bool,
    /// A press on a cell opens its working (§7.4).
    pub(crate) press: bool,
    /// Only the final row (Check a string's first look, §8.3).
    pub(crate) only_final: bool,
    /// The cells the step-through's step touches, outlined in the accent,
    /// the one worked out (`true`) more heavily; with any, the front is
    /// not outlined (§7.5: one place at a time).
    pub(crate) marks: &'a [(Cell, bool)],
}

/// The cells of a sheet as they are laid out, and the rows' marks.
struct Lay<'a> {
    s: Sheet<'a>,
    g: &'a Geom,
    out: Vec<Drawn>,
    /// Each row's first cell's place and its mark (`+` or `=`).
    marks: Vec<(f32, f32, &'static str)>,
}

impl Lay<'_> {
    /// A character as drawn: a capital, or a dot under the mask.
    fn shown(&self, v: Option<u8>) -> Option<(String, Paint)> {
        v.map(|v| {
            let t = if self.s.hide {
                MASK.to_string()
            } else {
                upper(v)
            };
            (t, Role::OnSurface.paint())
        })
    }

    /// Box `b` of the string, bolded and numbered, in row `r`.
    fn given(&mut self, b: usize, r: usize, pink: bool) {
        let sh = self.s.sh;
        let text = if b <= 3 {
            Some((
                char::from(PREFIX[b - 1]).to_string(),
                Role::OnSurface.paint(),
            ))
        } else {
            self.shown(sh.symbol(b))
        };
        let id = (b >= 4).then_some(Cell::Given {
            sheet: self.s.id,
            b: b as u8,
        });
        let press = id.filter(|_| sh.symbol(b).is_some());
        self.out.push(Drawn {
            x: self.g.col(b),
            y: self.g.row(r),
            text,
            bold: true,
            pink,
            number: Some(b),
            letter: None,
            differs: false,
            press,
            id,
        });
    }

    /// A worked cell in box `b`'s column of row `r`, reading `v`.
    fn plain(&mut self, b: usize, r: usize, v: Option<u8>, press: Cell) -> &mut Drawn {
        let text = self.shown(v);
        self.out.push(Drawn {
            x: self.g.col(b),
            y: self.g.row(r),
            text,
            bold: false,
            pink: false,
            number: None,
            letter: None,
            differs: false,
            press: v.is_some().then_some(press),
            id: Some(press),
        });
        let n = self.out.len() - 1;
        &mut self.out[n]
    }

    /// Row `r`'s mark before box `b`.
    fn mark(&mut self, b: usize, r: usize, m: &'static str) {
        self.marks.push((self.g.col(b), self.g.row(r), m));
    }

    /// The first row and the residue of `ms` under it.
    fn head(&mut self) {
        let (sh, id) = (self.s.sh, self.s.id);
        for b in 1..=head_last(sh) {
            self.given(b, 0, false);
        }
        self.mark(4, 1, "+");
        for (i, &v) in sh.hrp_row().iter().enumerate() {
            let cell = Cell::Hrp {
                sheet: id,
                i: i as u8,
            };
            self.plain(4 + i, 1, Some(v), cell);
        }
    }

    /// Step `j`: its `=` row, the leading cells lettered from `lettered`
    /// and the characters it brings down at its end, and its `+` row.
    fn step(&mut self, j: usize, lettered: usize) {
        let (sh, id) = (self.s.sh, self.s.id);
        let (len, last_data) = (sh.checksum_len(), sh.last_data_box());
        let (r, start) = (2 + 2 * j, sh.sum_box(j));
        self.mark(start, r, "=");
        for i in 0..len {
            let cell = Cell::Sum {
                sheet: id,
                j: j as u8,
                i: i as u8,
            };
            let d = self.plain(start + i, r, sh.sum_cell(j, i), cell);
            d.pink = start + i > last_data;
            if i < sh.step_width(j) {
                d.letter = Some(letter(lettered + i));
            }
        }
        for b in start + len..start + len + sh.step_width(j) {
            self.given(b, r, b > last_data);
        }
        let (r, start) = (r + 1, sh.lookup_box(j));
        self.mark(start, r, "+");
        for i in 0..len {
            let cell = Cell::Lookup {
                sheet: id,
                j: j as u8,
                i: i as u8,
            };
            self.plain(start + i, r, sh.lookup_cell(j, i), cell);
        }
    }

    /// The final row, in row `r`: what it reads, worked out, or, until it
    /// is, what it must read, faint, as the booklet's verification sheet
    /// prints it.
    fn last(&mut self, r: usize) {
        let (sh, id) = (self.s.sh, self.s.id);
        let (fin, start) = (sh.steps(), sh.sum_box(sh.steps()));
        self.mark(start, r, "=");
        for (i, &want) in sh.target().iter().enumerate() {
            let cell = Cell::Sum {
                sheet: id,
                j: fin as u8,
                i: i as u8,
            };
            let v = sh.final_cell(i);
            let d = self.plain(start + i, r, v, cell);
            d.differs = v.is_some_and(|v| v != want);
            d.text = match v {
                Some(v) if d.differs => Some((upper(v), Role::Error.paint())),
                Some(_) => d.text.take(),
                None => Some((upper(want), Role::OnSurfaceVariant.paint())),
            };
        }
    }
}

/// Every cell of the sheet, in rows; with each row's first cell's place
/// and its mark (`+` or `=`).
fn cells(s: Sheet<'_>, g: &Geom) -> (Vec<Drawn>, Vec<(f32, f32, &'static str)>) {
    let mut lay = Lay {
        s,
        g,
        out: Vec::new(),
        marks: Vec::new(),
    };
    if s.only_final {
        lay.last(0);
    } else {
        lay.head();
        let mut lettered = 0;
        for j in 0..s.sh.steps() {
            lay.step(j, lettered);
            lettered += s.sh.step_width(j);
        }
        lay.last(final_row(s.sh));
    }
    (lay.out, lay.marks)
}

/// Draws the sheet `s` at (x, y), `w` wide: whole at the fitted cell
/// `fit` where there is one, else panned by `pan` in a view, and says so
/// to the scroll machinery. Returns the height drawn.
pub(crate) fn draw(
    ui: &mut Ui,
    s: Sheet<'_>,
    pan: SheetPan,
    (x, y, w): (f32, f32, f32),
    (cls, fit): (WindowClass, Option<f32>),
) -> f32 {
    let ink = Role::OnSurface;
    // The final row alone is always in its view.
    let fit = fit.filter(|_| !s.only_final);
    let g = Geom::drawn(fit, cls);
    let (cw, ch) = if s.only_final {
        // The final row alone, from the column left of its first cell,
        // where its `=` stands.
        (g.size(s.sh).0, g.cell + 2.0 * g.pad)
    } else {
        g.size(s.sh)
    };
    // Shifted so the final row alone starts at the view's left.
    let sx = if s.only_final {
        g.col(s.sh.sum_box(s.sh.steps()) - 1) - g.pad
    } else {
        0.0
    };
    // Whole, the view is the sheet: nothing pans.
    let (vw, vh) = if fit.is_some() {
        (w.min(cw), ch)
    } else {
        (w, ch.min(t::codex32_sheet_view(cls)))
    };
    let max = ((cw - sx - vw).max(0.0), (ch - vh).max(0.0));
    let (px, py) = (pan.x.clamp(0.0, max.0), pan.y.clamp(0.0, max.1));
    let (ox, oy) = (x - sx - px, y - py);
    let clip = ui.rect(x, y, vw, vh);
    ui.c.push_clip(clip);
    let mark = ui.presses_mark();
    let cut_from = (ui.c.cut_mark(), ui.cut.len());
    let (cells, marks) = cells(s, &g);
    let seen = |cx: f32, cy: f32| {
        cx + ox + g.cell >= x && cx + ox <= x + vw && cy + oy + g.cell >= y && cy + oy <= y + vh
    };
    let size = fit.map_or(t::codex32_cell_char(cls), |c| c * t::CODEX32_CELL_CHAR_PART);
    // The sheet is the booklet's grid: its cells are sized for the
    // characters they hold, so what is written in them keeps its size at
    // every text size (`docs/TEXT-SIZE.md` §3).
    ui.fixed_text(|ui| {
        for &(cx, cy, m) in &marks {
            if seen(cx - g.cell, cy) {
                ui.text_right(
                    cx + ox - t::CODEX32_MARK_GAP,
                    cy + oy,
                    g.cell,
                    size,
                    W::M,
                    ink,
                    m,
                );
            }
        }
    });
    let shown: Vec<&Drawn> = cells.iter().filter(|d| seen(d.x, d.y)).collect();
    // Fills, then the thin rules, then the bold ones over them, then the
    // front: cells share their rules, as on paper.
    for d in &shown {
        if d.pink {
            ui.fill(
                d.x + ox,
                d.y + oy,
                g.cell,
                g.cell,
                0.0,
                Role::ChecksumContainer,
            );
        }
    }
    thin(ui, &shown, (ox, oy), g.cell);
    for d in &shown {
        ui.fixed_text(|ui| one(ui, d, (ox, oy), g.cell, size));
        if let Some(cell) = d.press.filter(|_| s.press && !s.hide) {
            ui.hit_quiet(
                d.x + ox,
                d.y + oy,
                g.cell,
                g.cell,
                Action::Codex32(C32::Cell(cell)),
            );
        }
    }
    frames(ui, &s, &g, &shown, (ox, oy));
    // What is panned out of the view takes no press.
    ui.clip_presses_since(mark, clip);
    // What a view that pans sideways holds past its sides is scrolled
    // there, not cut off.
    if max.0 > 0.0 {
        ui.c.scrolled_across(cut_from.0);
        ui.cut.truncate(cut_from.1);
    }
    ui.c.pop_clip();
    if fit.is_some() {
        // No view of its own: where it stands, for the page to follow.
        ui.report_scroll_in(Slot::WorksheetWhole, clip, 0.0, 0.0);
    } else {
        ui.report_scroll_both(Slot::Worksheet, clip, max, (px, py));
    }
    vh
}

/// The sheet's heavy frames over its cells: the bold ones, the ones that
/// differ, the marked cells and the front box.
fn frames(ui: &mut Ui, s: &Sheet<'_>, g: &Geom, shown: &[&Drawn], (ox, oy): (f32, f32)) {
    for d in shown.iter().filter(|d| d.bold || d.differs) {
        let c = if d.differs {
            Role::Error.paint()
        } else {
            Role::OnSurface.paint()
        };
        frame(ui, d.x + ox, d.y + oy, g.cell, t::CODEX32_SHEET_BOLD, c);
    }
    for d in shown {
        if let Some(&(_, main)) = d.id.and_then(|id| s.marks.iter().find(|(c, _)| *c == id)) {
            let s = t::CODEX32_FRONT_STROKE;
            let th = if main { s + s } else { s };
            frame(ui, d.x + ox, d.y + oy, g.cell, th, Role::Primary.paint());
        }
    }
    if let Some((fx, fy, side, _)) = s
        .front
        .filter(|_| !s.only_final && s.marks.is_empty())
        .and_then(|b| g.box_spot(s.sh, b))
    {
        frame(
            ui,
            fx + ox,
            fy + oy,
            side,
            2.0 * t::CODEX32_FRONT_STROKE,
            Role::Primary.paint(),
        );
    }
}

/// The cells' thin rules, on the lines neighbours share: a vertical
/// rule left of each cell, and for each run of cells side by side the
/// rules over and under it and the one closing it.
fn thin(ui: &mut Ui, shown: &[&Drawn], (ox, oy): (f32, f32), s: f32) {
    let px = 1.0 / ui.f;
    let mut start = None;
    for (k, d) in shown.iter().enumerate() {
        let (x, y) = (d.x + ox, d.y + oy);
        ui.vrule(x, y, s + px, Role::OutlineVariant);
        let from = *start.get_or_insert(x);
        let end = x + s;
        let joins = shown
            .get(k + 1)
            .is_some_and(|n| (n.y + oy - y).abs() < px && (n.x + ox - end).abs() < px);
        if !joins {
            ui.vrule(end, y, s + px, Role::OutlineVariant);
            ui.rule(from, y, end - from + px, Role::OutlineVariant);
            ui.rule(from, y + s, end - from + px, Role::OutlineVariant);
            start = None;
        }
    }
}

/// A rule `th` thick round the square at (x, y), centred on its edge.
fn frame(ui: &mut Ui, x: f32, y: f32, s: f32, th: f32, c: Paint) {
    let h = th / 2.0;
    ui.fill(x - h, y - h, s + th, th, 0.0, c);
    ui.fill(x - h, y + s - h, s + th, th, 0.0, c);
    ui.fill(x - h, y - h, th, s + th, 0.0, c);
    ui.fill(x + s - h, y - h, th, s + th, 0.0, c);
}

/// A cell's character, its box number top right and its letter bottom
/// left. The character is centred in what the number leaves of the
/// cell, as the booklet's characters stand left of their numbers.
fn one(ui: &mut Ui, d: &Drawn, (ox, oy): (f32, f32), s: f32, size: f32) {
    let (x, y) = (d.x + ox, d.y + oy);
    let note = t::CODEX32_CELL_NOTE;
    let inset = t::CODEX32_CELL_NOTE_INSET;
    let mut room = s;
    // On a checksum square, everything in it is in the fill's on-role.
    let note_c = if d.pink {
        Role::OnChecksumContainer
    } else {
        Role::OnSurfaceVariant
    };
    if let Some(n) = d.number {
        let n = n.to_string();
        let nw = ui.measure(note, W::R, &n);
        ui.text(
            x + s - inset - nw,
            y - t::CODEX32_CELL_NOTE_RISE,
            note,
            W::R,
            note_c,
            &n,
        );
        room = s - inset - nw;
    }
    if let Some((text, c)) = &d.text {
        let tw = ui.measure(size, W::M, text);
        let left = ((room - tw) / 2.0).max(inset).min((s - tw) / 2.0);
        let c = if d.pink {
            Role::OnChecksumContainer.paint()
        } else {
            *c
        };
        ui.text_mid(x + left, y, s, size, W::M, c, text);
    }
    if let Some(l) = &d.letter {
        let lh = ui.line(note, W::R);
        ui.text(x + inset, y + s - lh, note, W::R, note_c, l);
    }
}

/// Share `share`'s checksum worksheet during the entry, its front box
/// outlined (§6.4.3, §6.4.7). Returns the height.
pub(crate) fn entry_sheet(
    m: &Codex32Make,
    work: &WorkSet,
    share: usize,
    front: Option<usize>,
    ui: &mut Ui,
    (x, y, w, cls, hide): (f32, f32, f32, WindowClass, bool),
) -> f32 {
    let Some(sh) = work.worksheet(share, Reveal::PaperForm) else {
        return 0.0;
    };
    let head = fill(EN.codex32_checksum_of, &[&share_name(work, share)]);
    let hh = ui.wrap_as((x, y, w), TITLE_S, Role::OnSurface, &head) + t::GAP_SMALL;
    let marks = m.walk_cells(share);
    let s = |sh| Sheet {
        sh,
        id: share as u8,
        front,
        hide,
        press: true,
        only_final: false,
        marks: &marks,
    };
    let fit = (cls, m.sheet_cell());
    let h = paper!(sh, |p| draw(ui, s(p), m.pan(), (x, y + hh, w), fit));
    hh + h + t::GAP
}

/// The whole checksum worksheet of share `share`, for the review (§6.5),
/// headed with its marking: "from your rolls" or "as typed".
pub(crate) fn review_sheet(
    m: &Codex32Make,
    work: &WorkSet,
    share: usize,
    ui: &mut Ui,
    (x, y, w, cls, hide): (f32, f32, f32, WindowClass, bool),
) -> f32 {
    let Some(sh) = work.worksheet(share, Reveal::PaperForm) else {
        return 0.0;
    };
    let typed = m.made_of(share, 0).is_some_and(super::Made::typed);
    let head = fill(
        EN.codex32_checksum_of_marked,
        &[
            &share_name(work, share),
            if m.sample() {
                EN.c32r_the_sample
            } else if typed || m.given_kind().is_some() {
                EN.codex32_as_typed
            } else {
                EN.codex32_from_your_rolls
            },
        ],
    );
    let hh = ui.wrap_as((x, y, w), TITLE_S, Role::OnSurface, &head) + t::GAP_SMALL;
    let s = |sh| Sheet {
        sh,
        id: share as u8,
        front: None,
        hide,
        press: true,
        only_final: false,
        marks: &[],
    };
    let fit = (cls, m.sheet_cell());
    let h = paper!(sh, |p| draw(ui, s(p), m.pan(), (x, y + hh, w), fit));
    hh + h + t::GAP
}

/// Check a string's worksheet (§8.3), drawn on the codex32 form: its
/// final row, or with `whole` the whole verification sheet; a final row
/// that differs from `SECRETSHARE32` is marked where it differs, and a
/// line names the first box. Its cells open nothing. Returns the height.
pub(crate) fn check_sheet(
    app: &View,
    check: &faraday_keys::StringCheck,
    (whole, pan): (bool, SheetPan),
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let Some(sh) = check.worksheet(Reveal::PaperForm) else {
        return 0.0;
    };
    let s = |sh| Sheet {
        sh,
        id: 0,
        front: None,
        hide: false,
        press: false,
        only_final: !whole,
        marks: &[],
    };
    let cls = app.window().class();
    let mut h = paper!(sh, |p| draw(ui, s(p), pan, (x, y, w), (cls, None))) + t::GAP;
    if let Some(i) = sh.first_difference() {
        let b = sh.sum_box(sh.steps()) + i;
        let line = fill(EN.c32r_final_differs, &[&b.to_string()]);
        h += ui.wrap_as((x, y + h, w), BODY_M, Role::Error, &line) + t::GAP;
    }
    h
}

#[cfg(test)]
mod tests {
    use super::letter;

    #[test]
    fn the_leading_cells_are_lettered_as_the_booklet_letters_them() {
        let l: Vec<String> = [0, 1, 25, 26, 27, 31, 57]
            .iter()
            .map(|&n| letter(n))
            .collect();
        assert_eq!(l, ["a", "b", "z", "aa", "ab", "af", "bf"]);
    }
}
