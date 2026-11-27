//! The word molecules: a grid of numbered word cells, the pills that offer the words a typed
//! prefix can become, and a seed's words listed in columns (04 §12.1, 09 §3.2).

use faraday_atoms::field::FieldAt;
use faraday_atoms::{Offered, Pressed, Shown, Ui};
use faraday_tokens::dims::{
    BODY_M, BODY_S, FIELD_EDGE, FIELD_EDGE_ACTIVE, GAP, GRID, LABEL_S, SHAPE_S, TOUCH,
};
use faraday_tokens::palette::{Paint, Role};

/// How a grid of cells is set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// A Latin list's words as typed: four to the row, each word as long as it is.
    Typed,
    /// The words taken on a list's own keyboard: six to the row, each cut to its cell.
    Keyed,
}

/// The tone a cell's word is drawn in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// A word taken.
    Taken,
    /// The word being typed.
    Typing,
    /// A word the list does not hold.
    Wrong,
}

/// Where a press on a cell's word lands: the places between its characters, kept for the
/// grid's press, which places the caret there (`docs/TEXT-CURSOR.md`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Places {
    /// Between each of the word's characters, the first at `first` of the text.
    Chars { first: usize },
    /// At the word's start and its end, the start at `first` of the text.
    Ends { first: usize },
    /// At the word's end alone, at `index` of the text.
    End { index: usize },
}

/// One cell of the grid.
pub struct WordCell<'s> {
    /// The word as it is shown: bullets where it is hidden; empty where the cell holds none.
    pub word: Shown<'s>,
    /// The tone it is drawn in.
    pub tone: Tone,
    /// The caret stands in this cell, before the character of the word at this index (the
    /// word's length for its end).
    pub caret: Option<usize>,
    /// Where a press on it lands.
    pub places: Places,
}

/// The words a typed prefix can still become, each a pill that takes it when pressed.
pub struct Pill<'s, A> {
    /// The word.
    pub word: Shown<'s>,
    /// What a press on it does: a word's pill, so the press itself (V11).
    pub press: A,
}

/// A seed's word in the list, with its place and its SeedQR number.
pub struct Listed<'s> {
    /// The word.
    pub word: Shown<'s>,
    /// Its number in the SeedQR; empty for none.
    pub number: Shown<'s>,
}

/// A grid of word cells, the pills under a keyed grid, or a seed's words in columns.
pub enum WordGridModel<'s, A> {
    /// Numbered cells, as many as `boxes`; the cell the typing goes to is `current`.
    Cells {
        layout: Layout,
        boxes: usize,
        cells: Vec<WordCell<'s>>,
        /// The cell typing goes to; its edge is the focused one.
        current: usize,
        /// What a press on a cell does: it places the caret there; none where the grid takes
        /// no press.
        offered: Offered<A>,
        /// The line one gap under the grid that says no real seed is typed here; empty for none.
        never: Shown<'s>,
        /// A line of the words the word being typed can become, under a typed grid; empty for
        /// none.
        candidates: Shown<'s>,
        /// The candidates line says nothing starts with what is typed.
        none_match: bool,
    },
    /// What the keys typed can be: a pill each, or `line` where there is none.
    Pills {
        /// What it says where `words` is empty.
        line: Shown<'s>,
        /// The line over the pills that says to press one; empty for none.
        hint: Shown<'s>,
        words: Vec<Pill<'s, A>>,
    },
    /// The seed's words in columns, four to the row, two on a small panel.
    Listed(Vec<Listed<'s>>),
}

/// A cell's number's inset from its left edge and the word's, by layout.
fn insets(layout: Layout) -> (f32, f32) {
    match layout {
        Layout::Typed => (10.0, 36.0),
        Layout::Keyed => (8.0, 30.0),
    }
}

/// A word cell's edge at (x, y, w, h): outline-variant, or, on the cell typing goes to,
/// Material's focused field indicator, primary and thicker.
fn cell_edge<A: Pressed>(ui: &mut Ui<'_, A>, (x, y, w, h): (f32, f32, f32, f32), current: bool) {
    let (edge, width) = if current {
        (Role::Primary, FIELD_EDGE_ACTIVE)
    } else {
        (Role::OutlineVariant, FIELD_EDGE)
    };
    let rect = ui.rect(x, y, w, h);
    let color = ui.col(Paint::from(edge));
    let f = ui.f;
    ui.c.stroke_rounded_rect(rect, SHAPE_S * f, width * f, color);
}

/// The places a press on a cell can land, `cols` from (x, y) in design units, the first at text
/// index `first`, kept for `press`.
fn edges<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y, h): (f32, f32, f32),
    (first, cols): (usize, &[f32]),
    press: &A,
) {
    let edges = cols.iter().map(|c| ui.px(x + c + ui.ox)).collect();
    let band = (ui.py(y), ui.px(y + h + ui.oy) + ui.oy_px);
    ui.fields.push(FieldAt {
        action: press.clone(),
        first,
        edges,
        band: Some(band),
    });
}

/// Draws the model from (x, y), `w` wide. Returns its height.
pub fn draw<A: Pressed>(
    m: &WordGridModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    match m {
        WordGridModel::Cells { .. } => cells(m, ui, (x, y, w)),
        WordGridModel::Pills { line, hint, words } => pills(ui, (x, y, w), (line, hint), words),
        WordGridModel::Listed(words) => listed(ui, (x, y, w), words),
    }
}

/// The grid of cells.
fn cells<A: Pressed>(
    m: &WordGridModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let WordGridModel::Cells {
        layout,
        boxes,
        cells,
        current,
        offered,
        never,
        candidates,
        none_match,
    } = m
    else {
        return 0.0;
    };
    let (cols, gap, cell_h) = match layout {
        Layout::Typed => (4, 10.0, 38.0),
        Layout::Keyed => (6, GRID, 36.0),
    };
    let (num_x, text_x) = insets(*layout);
    let cellw = (w - (cols - 1) as f32 * gap) / cols as f32;
    let pitch = TOUCH;
    let press = ui.offered(offered, |_, press, _| press);
    for k in 0..*boxes {
        let cx = x + (k % cols) as f32 * (cellw + gap);
        let cy = y + (k / cols) as f32 * pitch;
        // Each cell pressed in the whole of its place, a touch target.
        if let Some(a) = &press {
            ui.press_box(
                cx - gap / 2.0,
                cy - (pitch - cell_h) / 2.0,
                cellw + gap,
                pitch,
                a.clone(),
            );
        }
        ui.fill(cx, cy, cellw, cell_h, 8.0, Role::SurfaceContainerHighest);
        cell_edge(ui, (cx, cy, cellw, cell_h), k == *current);
        ui.text_mid_as(
            (cx + num_x, cy, cell_h),
            LABEL_S.regular(),
            Role::OnSurfaceVariant,
            &(k + 1).to_string(),
        );
        if let Some(cell) = cells.get(k) {
            cell_word(
                ui,
                (cx + text_x, cy, cell_h),
                (*layout, cellw),
                cell,
                press.as_ref(),
            );
        }
    }
    let rows_h = boxes.div_ceil(cols) as f32 * pitch;
    let mut cy = y + rows_h;
    match layout {
        Layout::Typed => {
            cy += 12.0;
            if !never.as_str().is_empty() {
                cy += ui.showing(never, |ui| {
                    ui.wrap_as((x, y + rows_h, w), BODY_M, Role::Warning, never.as_str())
                });
            }
            // Candidates for the word being typed.
            if !candidates.as_str().is_empty() {
                let tone = if *none_match {
                    Role::Error
                } else {
                    Role::OnSurfaceVariant
                };
                ui.text_as(x, cy, BODY_S.mono(), tone, candidates.as_str());
            }
            cy + 30.0 - y
        }
        Layout::Keyed => {
            if never.as_str().is_empty() {
                // The rows' pitch holds the gap under the last.
                return rows_h;
            }
            let line = ui.showing(never, |ui| {
                ui.wrap_as(
                    (x, y + rows_h + 2.0, w),
                    BODY_M,
                    Role::Warning,
                    never.as_str(),
                )
            });
            rows_h + 8.0 + GAP + line
        }
    }
}

/// One cell's word at (x, y), `h` tall, its caret and the places a press on it can land.
fn cell_word<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y, h): (f32, f32, f32),
    (layout, cellw): (Layout, f32),
    cell: &WordCell<'_>,
    press: Option<&A>,
) {
    ui.showing(&cell.word, |ui| {
        let ink = match cell.tone {
            Tone::Taken => Role::OnSurface,
            Tone::Typing => Role::Primary,
            Tone::Wrong => Role::Error,
        };
        let full = cell.word.as_str();
        let word = match layout {
            Layout::Typed => faraday_text::Text::of(full),
            Layout::Keyed => ui.fit_as(BODY_M.mono(), full, cellw - 36.0),
        };
        let tw = if word.is_empty() {
            0.0
        } else {
            ui.text_mid_as((x, y, h), BODY_M.mono(), ink, &word)
        };
        let edge = |ui: &Ui<'_, A>, j: usize| {
            let b = word.char_indices().nth(j).map_or(word.len(), |(b, _)| b);
            ui.measure_as(BODY_M.mono(), &word[..b])
        };
        if let Some(at) = cell.caret {
            let at_x = if at >= word.chars().count() {
                tw
            } else {
                edge(ui, at)
            };
            ui.caret(x + 1.0 + at_x, y + (h - 18.0) / 2.0, 18.0);
        }
        if let Some(a) = press {
            match cell.places {
                Places::Chars { first } => {
                    let cols: Vec<f32> = (0..=word.chars().count()).map(|j| edge(ui, j)).collect();
                    edges(ui, (x, y, h), (first, &cols), a);
                }
                Places::Ends { first } => edges(ui, (x, y, h), (first, &[0.0, tw]), a),
                Places::End { index } => edges(ui, (x, y, h), (index, &[tw]), a),
            }
        }
    });
}

/// What the keys typed can be: a pill each, or the line that says there is none.
fn pills<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
    (line, hint): (&Shown<'_>, &Shown<'_>),
    words: &[Pill<'_, A>],
) -> f32 {
    let mut cy = y;
    if words.is_empty() {
        ui.text_mid_as((x, cy, 30.0), BODY_S, Role::OnSurfaceVariant, line.as_str());
        return 34.0;
    }
    if !hint.as_str().is_empty() {
        ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, hint.as_str());
        cy += ui.advance_as(BODY_S, 22.0);
    }
    // The candidates over the keys keep their size, as the keys do (`docs/TEXT-SIZE.md` §3).
    ui.fixed_text(|ui| {
        let mut bx = x;
        for p in words {
            ui.showing(&p.word, |ui| {
                let bw = ui.measure_as(BODY_M.mono(), p.word.as_str()) + 24.0;
                if bx > x && bx + bw > x + w {
                    bx = x;
                    cy += 32.0;
                }
                bx += ui.word_pill(bx, cy, p.word.as_str(), p.press.clone()) + 6.0;
            });
        }
        cy + 36.0 - y
    })
}

/// A seed's words, each with its place in the list and its SeedQR number, in four columns, two
/// on a small panel. Returns the height.
fn listed<A: Pressed>(ui: &mut Ui<'_, A>, (x, y, w): (f32, f32, f32), words: &[Listed<'_>]) -> f32 {
    let cols = if ui.compact { 2 } else { 4 };
    let colw = w / cols as f32;
    let rows = words.len().div_ceil(cols);
    for (i, wd) in words.iter().enumerate() {
        let cx = x + (i / rows) as f32 * colw;
        let ry = y + (i % rows) as f32 * 26.0;
        ui.text_as(
            cx,
            ry,
            BODY_S,
            Role::OnSurfaceVariant,
            &format!("{:>2}", i + 1),
        );
        ui.showing(&wd.word, |ui| {
            ui.showing(&wd.number, |ui| {
                if ui.compact {
                    ui.text_as(
                        cx + 20.0,
                        ry,
                        BODY_S.mono(),
                        Role::OnSurface,
                        wd.word.as_str(),
                    );
                    ui.text_right_as(
                        (cx + colw - 8.0, ry - 3.0, 20.0),
                        LABEL_S.mono(),
                        Role::OnSurfaceVariant,
                        wd.number.as_str(),
                    );
                } else {
                    ui.text_as(
                        cx + 26.0,
                        ry,
                        BODY_M.mono(),
                        Role::OnSurface,
                        wd.word.as_str(),
                    );
                    ui.text_as(
                        cx + 104.0,
                        ry,
                        BODY_S.mono(),
                        Role::OnSurfaceVariant,
                        wd.number.as_str(),
                    );
                }
            });
        });
    }
    rows as f32 * 26.0 + 16.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::{P, at_rest, canvas};
    use faraday_atoms::{MotionStore, Theme};

    fn typed(press: Offered<P>) -> WordGridModel<'static, P> {
        let cell = |w: &'static str, first: usize, caret: Option<usize>| WordCell {
            word: Shown::plain(w),
            tone: Tone::Taken,
            caret,
            places: Places::Chars { first },
        };
        WordGridModel::Cells {
            layout: Layout::Typed,
            boxes: 12,
            cells: vec![cell("abandon", 0, None), cell("ability", 8, Some(3))],
            current: 1,
            offered: press,
            never: Shown::plain(""),
            candidates: Shown::plain(""),
            none_match: false,
        }
    }

    /// Invariant: A grid of word cells takes a press over each of its cells, kept with the places between the word's characters so that the press puts the caret there, and takes none where the grid is not offered.
    /// Regression: A press on a word cell places no caret, or lands at the cell's start whatever character was pressed, or a grid not offered still takes presses.
    /// Owner: kit
    #[test]
    fn a_press_on_a_word_cell_is_kept_with_the_places_between_its_characters() {
        let run = |offered: Offered<P>| {
            let mut c = canvas(160);
            let mut store = MotionStore::new();
            let pointer = at_rest(None);
            let mut ui: Ui<'_, P> =
                Ui::new(&mut c, Theme::default(), 1.0, &mut store, &pointer, true);
            draw(&typed(offered), &mut ui, (0.0, 0.0, 400.0));
            let out = ui.finish();
            out.hits.len()
        };
        assert_eq!(run(Offered::Yes(P(1))), 12);
        assert_eq!(run(Offered::No), 0);
    }
}
