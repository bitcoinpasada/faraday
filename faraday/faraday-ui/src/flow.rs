//! A paragraph of words in two faces, flowing as lines: a guide's text with
//! bold names among plain words.

use crate::Ui;
use crate::pal::*;
use faraday_draw::tokens::TypeRole;

/// One piece of a word in one face; `starts` is set on the first piece of a
/// word, so a bold name and the full stop after it are one word.
struct Piece<'a> {
    text: &'a str,
    bold: bool,
    starts: bool,
    width: f32,
}

/// One run of a laid-out line: the words of one face, joined by spaces,
/// and the run's left edge from the paragraph's.
struct Run {
    bold: bool,
    x: f32,
    text: String,
}

/// A paragraph laid out: its lines, each of runs, at the width and font it
/// was broken for. Kept by a caller, so a frame drawn again measures and
/// breaks nothing ([`Flow::draw`]).
pub struct Flow {
    role: TypeRole,
    line: f32,
    lines: Vec<Vec<Run>>,
}

impl Flow {
    /// The height the paragraph uses. A lookup.
    pub fn height(&self) -> f32 {
        self.lines.len() as f32 * self.line
    }

    /// Draws the lines in view from (x, y), the runs of a line counting as
    /// one line of text drawn. Costs one draw of each run in view. Returns
    /// the height used.
    pub fn draw<A: Copy + PartialEq>(&self, ui: &mut Ui<A>, (x, y): (f32, f32)) -> f32 {
        for (n, runs) in self.lines.iter().enumerate() {
            let cy = n as f32 * self.line;
            if !ui.shows(y + cy, self.line) {
                continue;
            }
            let mark = ui.c.lines_drawn();
            for r in runs {
                let role = if r.bold {
                    self.role.medium()
                } else {
                    self.role
                };
                ui.text_mid_as((x + r.x, y + cy, self.line), role, Role::OnSurface, &r.text);
            }
            ui.c.join_lines_since(mark);
        }
        self.height()
    }
}

/// The paragraph `spans` (a text and whether it is bold) laid out `w`
/// wide in `role`, the bold spans in its medium face. Words are broken at
/// spaces across spans, so punctuation stays with its word, and the words
/// of one face on a line are one run. Costs one measure of each piece of
/// a word and an allocation for each run.
pub fn lay_out<'s, A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    w: f32,
    role: TypeRole,
    spans: impl IntoIterator<Item = (&'s str, bool)> + Clone,
) -> Flow {
    let face = |bold: bool| if bold { role.medium() } else { role };
    let count = spans
        .clone()
        .into_iter()
        .map(|(t, _)| t.split(char::is_whitespace).count())
        .sum();
    let mut pieces: Vec<Piece> = Vec::with_capacity(count);
    let mut joins = false;
    for (text, bold) in spans {
        for (i, part) in text.split(char::is_whitespace).enumerate() {
            if part.is_empty() {
                continue;
            }
            pieces.push(Piece {
                text: part,
                bold,
                starts: !(i == 0 && joins),
                width: ui.measure_as(face(bold), part),
            });
        }
        if !text.is_empty() {
            joins = !text.ends_with(char::is_whitespace);
        }
    }
    let line = ui.line_as(role);
    let space = ui.measure_as(role, "0 0") - ui.measure_as(role, "00");
    let mut cx = 0.0_f32;
    let mut lines: Vec<Vec<Run>> = vec![Vec::new()];
    let mut i = 0;
    while i < pieces.len() {
        let end = pieces[i + 1..]
            .iter()
            .position(|p| p.starts)
            .map_or(pieces.len(), |n| i + 1 + n);
        let word = &pieces[i..end];
        let ww: f32 = word.iter().map(|p| p.width).sum();
        let mut spaced = false;
        if cx > 0.0 && cx + space + ww > w {
            lines.push(Vec::new());
            cx = 0.0;
        } else if cx > 0.0 {
            cx += space;
            spaced = true;
        }
        let runs = lines.last_mut().expect("a line");
        for (n, p) in word.iter().enumerate() {
            match runs.last_mut() {
                Some(r) if r.bold == p.bold => {
                    if n == 0 && spaced {
                        r.text.push(' ');
                    }
                    r.text.push_str(p.text);
                }
                _ => runs.push(Run {
                    bold: p.bold,
                    x: cx,
                    text: p.text.to_string(),
                }),
            }
            cx += p.width;
        }
        i = end;
    }
    Flow { role, line, lines }
}

/// The paragraph `spans` laid out and drawn from (x, y), `w` wide: the
/// two steps in one, for a caller that keeps nothing ([`lay_out`],
/// [`Flow::draw`]). Returns the height used.
pub fn word_flow<'s, A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (x, y, w): (f32, f32, f32),
    role: TypeRole,
    spans: impl IntoIterator<Item = (&'s str, bool)> + Clone,
) -> f32 {
    lay_out(ui, w, role, spans).draw(ui, (x, y))
}
