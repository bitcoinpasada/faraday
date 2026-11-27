//! The multi-select molecule: a list of rows each with a checkbox at its start, any number
//! ticked (`docs/DESIGN.md` §4.2, Multi choice; 09 §3.2).

use faraday_atoms::{Offered, Pressed, Shown, Ui};
use faraday_tokens::dims::{
    BODY_L, BODY_M, DISABLED_CONTENT, ICON_GLYPH_BUTTON, LIST_1, LIST_2, LIST_INDENT, LIST_PAD_V,
    LIST_RESERVE, MULTI_CHECK_X,
};
use faraday_tokens::palette::{Paint, Role};

/// One row: its label, the value under it where it has one, whether it is ticked, and what a
/// press on it does.
pub struct MultiRow<'s, A> {
    /// The label: Material's body large.
    pub label: Shown<'s>,
    /// The muted line under the label, as a value row has; empty for none.
    pub value: Shown<'s>,
    /// It is ticked.
    pub on: bool,
    /// What a tap ticks or unticks. A row not offered is dimmed.
    pub offered: Offered<A>,
}

/// A list of rows: one full-width row per option, a checkbox at its start.
pub struct MultiModel<'s, A> {
    /// The rows, in the order shown. Each is two lines tall where any has a value, else one.
    pub rows: Vec<MultiRow<'s, A>>,
}

/// Draws the list from (x, y), `w` wide. A tap ticks or unticks its row. Returns the list's
/// height; Continue goes under it.
pub fn draw<A: Pressed>(
    m: &MultiModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let valued = m.rows.iter().any(|r| !r.value.as_str().is_empty());
    // As tall as larger text needs (`docs/TEXT-SIZE.md` §5).
    let (token, lines) = if valued {
        (LIST_2, ui.line_as(BODY_L) + ui.line_as(BODY_M))
    } else {
        (LIST_1, ui.line_as(BODY_L))
    };
    let rh = ui.row_height(token, lines, LIST_PAD_V);
    for (i, r) in m.rows.iter().enumerate() {
        let ry = y + i as f32 * rh;
        ui.showing(&r.label, |ui| {
            ui.showing(&r.value, |ui| {
                ui.offered(&r.offered, |ui, press, dimmed| {
                    row(r, ui, (x, ry, w, rh), !dimmed);
                    if let Some(a) = press {
                        ui.hit(x, ry, w, rh, a);
                    }
                })
            })
        });
        if i + 1 < m.rows.len() {
            ui.rule(
                x + LIST_INDENT,
                ry + rh,
                w - LIST_INDENT,
                Role::OutlineVariant,
            );
        }
    }
    m.rows.len() as f32 * rh
}

/// One row's checkbox and text in the box (x, y, w, h).
fn row<A: Pressed>(
    r: &MultiRow<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, ry, w, rh): (f32, f32, f32, f32),
    enabled: bool,
) {
    ui.checkbox(
        x + MULTI_CHECK_X,
        ry + (rh - ICON_GLYPH_BUTTON) / 2.0,
        r.on,
        enabled,
    );
    let label = ui.fit_as(BODY_L, r.label.as_str(), w - LIST_RESERVE);
    let dim: Paint = Role::OnSurface.at(DISABLED_CONTENT);
    let fg = if enabled {
        Role::OnSurface.paint()
    } else {
        dim
    };
    if r.value.as_str().is_empty() {
        ui.text_mid_as((x + LIST_INDENT, ry, rh), BODY_L, fg, &label);
        return;
    }
    let (lh, vh) = (ui.line_as(BODY_L), ui.line_as(BODY_M));
    let top = ry + (rh - lh - vh) / 2.0;
    ui.text_mid_as((x + LIST_INDENT, top, lh), BODY_L, fg, &label);
    let value = ui.fit_as(BODY_M, r.value.as_str(), w - LIST_RESERVE);
    let vc = if enabled {
        Role::OnSurfaceVariant.paint()
    } else {
        dim
    };
    ui.text_mid_as((x + LIST_INDENT, top + lh, vh), BODY_M, vc, &value);
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::{P, at_rest, canvas};
    use faraday_atoms::{MotionStore, Theme};

    /// Invariant: A row of a multi-select list takes a press over its whole row, and a row that is not offered takes none.
    /// Regression: A tap on an unavailable row ticks it, or a row's tap target covers only its checkbox.
    /// Owner: kit
    #[test]
    fn a_row_takes_a_press_over_its_whole_row_unless_it_is_not_offered() {
        let mut c = canvas(160);
        let mut store = MotionStore::new();
        let pointer = at_rest(None);
        let mut ui: Ui<'_, P> = Ui::new(&mut c, Theme::default(), 1.0, &mut store, &pointer, true);
        let row = |label: &'static str, offered| MultiRow {
            label: Shown::plain(label),
            value: Shown::plain(""),
            on: false,
            offered,
        };
        let m = MultiModel {
            rows: vec![
                row("Passphrase", Offered::Yes(P(1))),
                row("Hidden", Offered::No),
            ],
        };
        let h = draw(&m, &mut ui, (0.0, 0.0, 300.0));
        let hits = ui.finish().hits;
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].press, P(1));
        assert_eq!(hits[0].rect.w, 300);
        assert!(h > 0.0);
    }
}
