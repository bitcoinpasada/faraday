//! The kind rows molecule: a card's option rows, each a name and a second line, one ticked, and
//! a quiet button under them that shows the rest (`docs/DESIGN.md` §4.2, Option row; 09 §3.2).

use faraday_atoms::button::Look;
use faraday_atoms::{Offered, Pressed, Shown, Ui};
use faraday_tokens::dims::{BODY_S, BUTTON_H, TITLE_S, TOUCH};
use faraday_tokens::palette::{Role, on_selected, selected_or};
use faraday_tokens::type_roles::TypeRole;

/// One row: its name, its second line and whether it is the one ticked.
pub struct KindRow<'s, A> {
    /// The row's name: Material's title small.
    pub name: Shown<'s>,
    /// Its second line, a fact of the row: at the row's end on a wide window, under the name on
    /// a compact one.
    pub line: Shown<'s>,
    /// It is the one ticked.
    pub on: bool,
    /// What a press does; `Offered::No` draws the row and takes no press.
    pub offered: Offered<A>,
}

/// The rows of a card that asks one thing, and the button that shows the rest.
pub struct KindRowsModel<'s, A> {
    /// The rows, in the order shown.
    pub rows: Vec<KindRow<'s, A>>,
    /// The label of the button under the rows and what it does, where there are more to show;
    /// drawn only where it is offered.
    pub more: Option<(Shown<'s>, Offered<A>)>,
}

/// A row's corner radius.
const RADIUS: f32 = 8.0;
/// A row's text inset from its edge.
const INSET: f32 = 14.0;
/// The gap above a row's first line on a compact window.
const TOP: f32 = 6.0;
/// The room a wide row's name keeps clear of its line.
const LINE_ROOM: f32 = 260.0;
/// How far one row's top is from the next, where each is a touch target tall.
const PITCH: f32 = 54.0;
/// What the quiet button under the rows takes.
const MORE_H: f32 = 44.0;

/// `s` broken at its spaces into lines no wider than `max` in `role`, a word wider than `max`
/// on a line of its own. Slices of `s`, nothing copied.
fn lines_of<'s, A: Pressed>(ui: &Ui<'_, A>, role: TypeRole, s: &'s str, max: f32) -> Vec<&'s str> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    let mut end = 0;
    for (at, word) in s.split(' ').scan(0, |pos, w| {
        let at = *pos;
        *pos += w.len() + 1;
        Some((at, w))
    }) {
        if word.is_empty() {
            continue;
        }
        let word_end = at + word.len();
        match start {
            None => start = Some(at),
            Some(from) if ui.measure_as(role, &s[from..word_end]) > max => {
                out.push(&s[from..end]);
                start = Some(at);
            }
            Some(_) => {}
        }
        end = word_end;
    }
    if let Some(from) = start {
        out.push(&s[from..end]);
    }
    out
}

/// Draws the rows from (x, y), `w` wide, and the button under them where there is one and it
/// takes a press. On a compact window the name and the line each wrap and the row grows to hold
/// them. Returns the height.
pub fn draw<A: Pressed>(
    m: &KindRowsModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let mut cy = y;
    for r in &m.rows {
        let h = row(r, ui, (x, cy, w));
        cy += PITCH - TOUCH + h;
    }
    if let Some((label, offered)) = &m.more {
        let pressed = ui.offered(offered, |_, press, _| press);
        if let Some(a) = pressed {
            ui.showing(label, |ui| {
                ui.button(x, cy, None, BUTTON_H, label.as_str(), Look::Ghost, a);
            });
            cy += MORE_H;
        }
    }
    cy - y
}

/// One row at (x, y), `w` wide. Returns its height.
fn row<A: Pressed>(r: &KindRow<'_, A>, ui: &mut Ui<'_, A>, (x, y, w): (f32, f32, f32)) -> f32 {
    ui.showing(&r.name, |ui| {
        ui.showing(&r.line, |ui| {
            let (name, line) = (r.name.as_str(), r.line.as_str());
            let left = x + INSET;
            let inner = w - 2.0 * INSET;
            let (names, lines) = if ui.compact {
                (
                    lines_of(ui, TITLE_S, name, inner),
                    lines_of(ui, BODY_S, line, inner),
                )
            } else {
                (Vec::new(), Vec::new())
            };
            let (name_h, line_h) = (ui.line_as(TITLE_S), ui.line_as(BODY_S));
            let line_at = y + TOP + names.len().max(1) as f32 * name_h;
            let h = if ui.compact {
                (line_at + lines.len() as f32 * line_h + TOP - y).max(TOUCH)
            } else {
                TOUCH
            };
            ui.fill(
                x,
                y,
                w,
                h,
                RADIUS,
                selected_or(r.on, Role::SurfaceContainerHighest),
            );
            ui.stroke(x, y, w, h, RADIUS, selected_or(r.on, Role::OutlineVariant));
            ui.offered(&r.offered, |ui, press, _| {
                if let Some(a) = press {
                    ui.press_box(x, y, w, h, a);
                }
            });
            let ink = on_selected(r.on, Role::OnSurface);
            let muted = on_selected(r.on, Role::OnSurfaceVariant);
            if ui.compact {
                for (n, text) in names.iter().enumerate() {
                    ui.text_as(left, y + TOP + n as f32 * name_h, TITLE_S, ink, text);
                }
                for (n, text) in lines.iter().enumerate() {
                    ui.text_as(left, line_at + n as f32 * line_h, BODY_S, muted, text);
                }
            } else {
                ui.text_mid_as((left, y, h), TITLE_S, ink, name);
                let line = ui.fit_as(BODY_S, line, w - LINE_ROOM);
                ui.text_right_as((x + w - INSET, y, h), BODY_S, muted, &line);
            }
            h
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::{P, at_rest, canvas};
    use faraday_atoms::{MotionStore, Theme};

    fn model(more: Offered<P>) -> KindRowsModel<'static, P> {
        let row = |name: &'static str, on: bool, p: u8| KindRow {
            name: Shown::plain(name),
            line: Shown::plain("Native SegWit"),
            on,
            offered: Offered::Yes(P(p)),
        };
        KindRowsModel {
            rows: vec![row("Single key", true, 1), row("Multisig", false, 2)],
            more: Some((Shown::plain("More kinds"), more)),
        }
    }

    /// Invariant: Each kind row takes a press over its whole row, and the button that shows more kinds is drawn, with its press, only where it is offered.
    /// Regression: A kind row takes no press, or the More kinds button stays on screen after every kind is shown.
    /// Owner: kit
    #[test]
    fn each_kind_row_takes_a_press_and_more_kinds_only_where_offered() {
        let drawn = |more: Offered<P>| {
            let mut c = canvas(160);
            let mut store = MotionStore::new();
            let pointer = at_rest(None);
            let mut ui: Ui<'_, P> =
                Ui::new(&mut c, Theme::default(), 1.0, &mut store, &pointer, true);
            let h = draw(&model(more), &mut ui, (16.0, 16.0, 400.0));
            let presses: Vec<u8> = ui.finish().hits.iter().map(|h| h.press.0).collect();
            (h, presses)
        };
        let (with, presses) = drawn(Offered::Yes(P(9)));
        assert_eq!(presses, vec![1, 2, 9]);
        let (without, presses) = drawn(Offered::No);
        assert_eq!(presses, vec![1, 2]);
        assert!(with > without);
    }
}
