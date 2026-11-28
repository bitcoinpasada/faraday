//! The appearance molecule: Settings' Appearance rows, drawn the same in Faraday and the Starter
//! (09 §3.2; `docs/THEMES.md` §12, §14): Theme as chips, each filled in that theme's own
//! colours, Mode and Contrast as segmented buttons, and the Text size row.

use faraday_atoms::{Pressed, Shown, Ui};
use faraday_tokens::dims::{
    APPEARANCE_LABEL_GAP, APPEARANCE_ROW_GAP, BODY_M, FILTER_CHIP_H, LABEL_L, SEGMENTED_H,
};
use faraday_tokens::palette::Role;

use crate::chips::{self, ChipsModel};

/// One row of segmented choices: the words of each segment, what a press on it sends and which
/// is the chosen one.
pub struct Segments<'s, A> {
    pub items: Vec<(Shown<'s>, A)>,
    pub chosen: Option<usize>,
}

/// Theme, Mode and Contrast (`docs/THEMES.md` §12).
pub struct AppearanceModel<'s, A> {
    /// The words of the three rows: Theme, Mode and Contrast.
    pub labels: [Shown<'s>; 3],
    pub themes: ChipsModel<'s, A>,
    pub modes: Segments<'s, A>,
    pub contrasts: Segments<'s, A>,
    /// Each row's label in a column this wide with its control beside it; none puts it over it.
    pub label_w: Option<f32>,
}

/// The Text size row.
pub struct TextSizeModel<'s, A> {
    pub label: Shown<'s>,
    pub sizes: Segments<'s, A>,
    /// The label's column, as [`AppearanceModel::label_w`].
    pub label_w: Option<f32>,
    /// Every choice is drawn dimmed and takes no press (Speed while Motion is Off).
    pub dim: bool,
}

/// Where a row's control goes: its x, its width and its top, with the row's label drawn.
fn row_label<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
    label_w: Option<f32>,
    label: &Shown<'_>,
    first_line: f32,
) -> (f32, f32, f32) {
    ui.showing(label, |ui| match label_w {
        // The label level with the control's first line.
        Some(lw) => {
            let at = (x, y, first_line);
            ui.text_mid_as(at, BODY_M, Role::OnSurfaceVariant, label.as_str());
            (x + lw, w - lw, y)
        }
        None => {
            ui.text_as(x, y, BODY_M, Role::OnSurfaceVariant, label.as_str());
            (x, w, y + ui.line_as(BODY_M) + APPEARANCE_LABEL_GAP)
        }
    })
}

/// Draws a row of segmented choices at (x, y, w). Returns its height.
fn segments<A: Pressed>(s: &Segments<'_, A>, ui: &mut Ui<'_, A>, at: (f32, f32, f32)) -> f32 {
    let options: Vec<(&str, A)> = s
        .items
        .iter()
        .map(|(l, a)| (l.as_str(), a.clone()))
        .collect();
    let (x, y, w) = at;
    ui.choices((x, y, w, SEGMENTED_H), LABEL_L.size, &options, s.chosen)
}

/// Draws the rows from (x, y), `w` wide. Returns the height used.
pub fn draw<A: Pressed>(
    m: &AppearanceModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let mut at = y;
    for (i, label) in m.labels.iter().enumerate() {
        if i > 0 {
            at += APPEARANCE_ROW_GAP;
        }
        let line = if i == 0 {
            FILTER_CHIP_H
        } else {
            ui.button_height(SEGMENTED_H)
        };
        let (cx, cw, top) = row_label(ui, (x, at, w), m.label_w, label, line);
        let used = match i {
            0 => chips::draw(&m.themes, ui, (cx, top, cw)),
            1 => segments(&m.modes, ui, (cx, top, cw)),
            _ => segments(&m.contrasts, ui, (cx, top, cw)),
        };
        at = top + used;
    }
    at - y
}

/// Draws the Text size row from (x, y), `w` wide. Returns the height used.
pub fn draw_text_size<A: Pressed>(
    m: &TextSizeModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let line = ui.button_height(SEGMENTED_H);
    let (cx, cw, top) = row_label(ui, (x, y, w), m.label_w, &m.label, line);
    let used = if m.dim {
        let options: Vec<(&str, A)> = m
            .sizes
            .items
            .iter()
            .map(|(l, a)| (l.as_str(), a.clone()))
            .collect();
        let off = vec![true; options.len()];
        ui.choices_with(
            (cx, top, cw, SEGMENTED_H),
            LABEL_L.size,
            &options,
            m.sizes.chosen,
            &off,
        )
    } else {
        segments(&m.sizes, ui, (cx, top, cw))
    };
    top + used - y
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chips::ThemeChip;
    use faraday_atoms::Theme;
    use faraday_atoms::ui::testing::*;
    use faraday_text::Text;

    fn segs(words: &'static [&'static str], chosen: usize, base: u8) -> Segments<'static, P> {
        Segments {
            items: words
                .iter()
                .enumerate()
                .map(|(i, w)| (Shown::plain(*w), P(base + i as u8)))
                .collect(),
            chosen: Some(chosen),
        }
    }

    /// Invariant: The three rows come one under another and every choice in them takes a press: each theme chip, each mode and each contrast.
    /// Regression: A row of the settings overlaps the next, or one of its choices takes no press.
    /// Owner: kit
    #[test]
    fn the_rows_stack_and_every_choice_is_a_press() {
        let current = Theme::default();
        let names: Vec<Text> = Theme::all()
            .filter(|t| t.mode == current.mode && t.contrast == current.contrast)
            .map(|t| Text::of(&format!("{:?}", t.name)))
            .collect();
        let items = Theme::all()
            .filter(|t| t.mode == current.mode && t.contrast == current.contrast)
            .enumerate()
            .map(|(i, t)| ThemeChip {
                name: t.name,
                label: Shown::plain(&names[i]),
                press: P(i as u8),
            })
            .collect();
        let m = AppearanceModel {
            labels: [
                Shown::plain("Theme"),
                Shown::plain("Mode"),
                Shown::plain("Contrast"),
            ],
            themes: ChipsModel {
                current,
                items,
                default_mark: Shown::plain("Default"),
            },
            modes: segs(&["Light", "Dark"], 0, 20),
            contrasts: segs(&["Standard", "Medium", "High"], 0, 30),
            label_w: None,
        };
        let mut canvas = canvas(160);
        let mut h = 0.0;
        let out = frame(&mut canvas, &at_rest(None), |ui| {
            ui.clear(page());
            h = draw(&m, ui, (8.0, 8.0, 320.0));
        });
        let mut got: Vec<u8> = out.hits.iter().map(|h| h.press.0).collect();
        got.sort_unstable();
        let want: Vec<u8> = (0..10).chain(20..22).chain(30..33).collect();
        assert_eq!(got, want);
        assert!(h > 150.0, "the rows share a place: {h}");
    }

    /// Invariant: A labelled row of choices sends a press for each choice, and none while it is dimmed.
    /// Regression: Speed stays pressable while Motion is Off, or a live row loses a choice.
    /// Owner: kit
    #[test]
    fn a_dimmed_row_takes_no_press() {
        let row = |dim| TextSizeModel {
            label: Shown::plain("Speed"),
            sizes: segs(&["Quick", "Medium", "Relaxed"], 1, 40),
            label_w: None,
            dim,
        };
        let presses = |dim| {
            let m = row(dim);
            let mut canvas = canvas(120);
            let out = frame(&mut canvas, &at_rest(None), |ui| {
                ui.clear(page());
                draw_text_size(&m, ui, (8.0, 8.0, 320.0));
            });
            out.hits.iter().map(|h| h.press.0).collect::<Vec<u8>>()
        };
        assert_eq!(presses(false), [40, 41, 42]);
        assert!(presses(true).is_empty());
    }
}
