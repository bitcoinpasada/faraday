//! The flow text molecule: a paragraph of words in two faces, flowing as lines: a guide's text
//! with bold names among plain words (09 §3.2).

use faraday_atoms::{Pressed, Shown, Ui};
use faraday_tokens::palette::Role;
use faraday_tokens::type_roles::TypeRole;

/// A paragraph: runs of words, each plain or bold, broken at spaces across runs, so punctuation
/// stays with its word.
pub struct FlowText<'s> {
    /// A run's text and whether it is bold (the role's medium face).
    pub runs: Vec<(Shown<'s>, bool)>,
    pub role: TypeRole,
}

/// One piece of a word in one face; `starts` is set on the first piece of a word, so a bold
/// name and the full stop after it are one word.
struct Piece<'a> {
    text: &'a str,
    bold: bool,
    starts: bool,
    width: f32,
}

/// One run of a laid-out line: the words of one face, joined by spaces, and the run's left edge
/// from the paragraph's.
struct Run {
    bold: bool,
    x: f32,
    text: String,
}

/// The paragraph broken into lines at a width: its lines, each of runs.
fn lay_out<A: Pressed>(m: &FlowText<'_>, ui: &Ui<'_, A>, w: f32) -> Vec<Vec<Run>> {
    let role = m.role;
    let face = |bold: bool| if bold { role.medium() } else { role };
    let mut pieces: Vec<Piece> = Vec::new();
    let mut joins = false;
    for (shown, bold) in &m.runs {
        let text = shown.as_str();
        for (i, part) in text.split(char::is_whitespace).enumerate() {
            if part.is_empty() {
                continue;
            }
            pieces.push(Piece {
                text: part,
                bold: *bold,
                starts: !(i == 0 && joins),
                width: ui.measure_as(face(*bold), part),
            });
        }
        if !text.is_empty() {
            joins = !text.ends_with(char::is_whitespace);
        }
    }
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
    lines
}

/// The height the paragraph takes `w` wide, with nothing drawn.
pub fn height<A: Pressed>(m: &FlowText<'_>, ui: &Ui<'_, A>, w: f32) -> f32 {
    lay_out(m, ui, w).len() as f32 * ui.line_as(m.role)
}

/// Draws the lines in view from (x, y), `w` wide, the runs of a line counting as one line of
/// text drawn. Returns the height used.
pub fn draw<A: Pressed>(m: &FlowText<'_>, ui: &mut Ui<'_, A>, (x, y, w): (f32, f32, f32)) -> f32 {
    let lines = lay_out(m, ui, w);
    let line = ui.line_as(m.role);
    for (n, runs) in lines.iter().enumerate() {
        let cy = n as f32 * line;
        if !ui.shows(y + cy, line) {
            continue;
        }
        let mark = ui.c.lines_drawn();
        for r in runs {
            let role = if r.bold { m.role.medium() } else { m.role };
            ui.text_mid_as((x + r.x, y + cy, line), role, Role::OnSurface, &r.text);
        }
        ui.c.join_lines_since(mark);
    }
    lines.len() as f32 * line
}
