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

/// The paragraph `spans` (a text and whether it is bold) from (x, y), `w`
/// wide, in `role`, the bold spans in its medium face. Words are broken at
/// spaces across spans, so punctuation stays with its word. Only the lines
/// in view are drawn, and the words of one face on a line are drawn as one
/// run, the line counting as one line of text drawn. Costs one measure of
/// each piece of a word and one draw of each run in view, and two
/// allocations. Returns the
/// height used.
pub fn word_flow<'s, A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (x, y, w): (f32, f32, f32),
    role: TypeRole,
    spans: impl IntoIterator<Item = (&'s str, bool)> + Clone,
) -> f32 {
    let face = |bold: bool| if bold { role.medium() } else { role };
    // One count of the pieces and the bytes, so the two buffers are made
    // once each at their size.
    let (count, bytes) = spans.clone().into_iter().fold((0, 0), |(n, b), (t, _)| {
        (n + t.split(char::is_whitespace).count(), b + t.len())
    });
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
    let space = ui.measure_as(role, "a a") - ui.measure_as(role, "aa");
    let (mut cx, mut cy) = (0.0_f32, 0.0_f32);
    // The run being gathered: its text, face and left edge.
    let mut run = String::with_capacity(bytes);
    let (mut run_bold, mut run_x) = (false, 0.0_f32);
    // A line's runs count as one line of text drawn.
    let mut mark = ui.c.lines_drawn();
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
            flush(ui, (x, y, line), role, &mut run, run_bold, (run_x, cy));
            ui.c.join_lines_since(mark);
            mark = ui.c.lines_drawn();
            cx = 0.0;
            cy += line;
        } else if cx > 0.0 {
            cx += space;
            spaced = true;
        }
        for (n, p) in word.iter().enumerate() {
            if !run.is_empty() && p.bold != run_bold {
                flush(ui, (x, y, line), role, &mut run, run_bold, (run_x, cy));
            }
            if run.is_empty() {
                (run_bold, run_x) = (p.bold, cx);
            } else if n == 0 && spaced {
                run.push(' ');
            }
            run.push_str(p.text);
            cx += p.width;
        }
        i = end;
    }
    flush(ui, (x, y, line), role, &mut run, run_bold, (run_x, cy));
    ui.c.join_lines_since(mark);
    cy + line
}

/// Draws the gathered run, if its line is in view, and empties it.
fn flush<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (x, y, line): (f32, f32, f32),
    role: TypeRole,
    run: &mut String,
    bold: bool,
    (run_x, cy): (f32, f32),
) {
    if !run.is_empty() && ui.shows(y + cy, line) {
        let role = if bold { role.medium() } else { role };
        ui.text_mid_as((x + run_x, y + cy, line), role, Role::OnSurface, run);
    }
    run.clear();
}
