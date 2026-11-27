//! The buttons molecule: a row of buttons, side by side when each label fits its share, one
//! under another when not, a label wider than the row broken onto more lines in a taller
//! button, or at the end of the row as wide as their labels (a dialog's) (09 §3.2).

use faraday_atoms::button::{ButtonModel, Look};
use faraday_atoms::{Pressed, Shown, Ui};
use faraday_tokens::dims::{BUTTON_H, BUTTON_PAD, BUTTON_PAD_V, DIALOG_ACTION_GAP, GAP, LABEL_L};

/// A row of buttons (04 §12.3): the sheet, the dialog and the compact menu all draw theirs
/// through it.
pub struct ButtonsModel<'s, A> {
    pub items: Vec<ButtonModel<'s, A>>,
    /// At the end of the row, each as wide as its label (a dialog's actions), the last at the
    /// right edge; one under another where they do not fit.
    pub end: bool,
    /// The words the row stacks by, one for each item, rather than the labels drawn: a button
    /// whose words change keeps its row and its box. Empty for the labels themselves.
    pub rule: Vec<Shown<'s>>,
}

impl<'s, A> ButtonsModel<'s, A> {
    /// No buttons.
    pub fn none() -> Self {
        ButtonsModel {
            items: Vec::new(),
            end: false,
            rule: Vec::new(),
        }
    }

    /// These buttons, sharing the width.
    pub fn row(items: Vec<ButtonModel<'s, A>>) -> Self {
        ButtonsModel {
            items,
            end: false,
            rule: Vec::new(),
        }
    }
}

/// Draws the row from (x, y), `w` wide. Returns its height.
pub fn draw<A: Pressed>(m: &ButtonsModel<'_, A>, ui: &mut Ui<'_, A>, at: (f32, f32, f32)) -> f32 {
    draw_as(m, m.end, ui, at)
}

/// [`draw`] at the end of the row when `end`, whatever the model says: a dialog's foot is at its
/// end on a wide panel and shares the width on a small one.
pub fn draw_as<A: Pressed>(
    m: &ButtonsModel<'_, A>,
    end: bool,
    ui: &mut Ui<'_, A>,
    at: (f32, f32, f32),
) -> f32 {
    draw_items(&m.items, &m.rule, end, ui, at)
}

/// [`draw_as`] for the buttons and rule words themselves: a content's row of buttons.
pub fn draw_items<A: Pressed>(
    items: &[ButtonModel<'_, A>],
    rule: &[Shown<'_>],
    end: bool,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    if items.is_empty() {
        return 0.0;
    }
    if end {
        let h = ui.button_height(BUTTON_H);
        let widths = end_widths(items, ui);
        let total = widths.iter().sum::<f32>() + DIALOG_ACTION_GAP * (items.len() - 1) as f32;
        if total <= w {
            let mut bx = x + w - total;
            for (b, bw) in items.iter().zip(widths) {
                faraday_atoms::button::draw(b, ui, (bx, y, Some(bw), h));
                bx += bw + DIALOG_ACTION_GAP;
            }
            return h;
        }
    }
    let n = items.len() as f32;
    let share = (w - (n - 1.0) * GAP) / n;
    let bh = ui.button_height(BUTTON_H);
    if fits(ui, w, items, rule) {
        for (i, b) in items.iter().enumerate() {
            let bx = x + i as f32 * (share + GAP);
            faraday_atoms::button::draw(b, ui, (bx, y, Some(share), bh));
        }
        return bh;
    }
    let mut by = y;
    for (i, b) in items.iter().enumerate() {
        let label = b.label.as_str();
        let (mut tall, lines) = stacked(ui, w, label);
        if let Some(held) = rule.get(i).map(Shown::as_str).filter(|h| *h != label) {
            tall = tall.max(stacked(ui, w, held).0);
        }
        if lines.is_empty() {
            faraday_atoms::button::draw(b, ui, (x, by, Some(w), tall));
        } else {
            let lines: Vec<&str> = lines.iter().map(|&(a, e)| label[a..e].trim_end()).collect();
            let look = if b.chosen {
                Look::Selected
            } else {
                Look::from(b.style)
            };
            ui.showing(&b.label, |ui| {
                ui.offered(&b.offered, |ui, press, off| {
                    ui.button_wrapped((x, by, w, tall), &lines, look.when(!off), press);
                });
            });
        }
        by += tall + GAP;
    }
    by - y - GAP
}

/// The height [`draw`] takes for the row `w` wide, by the same rule that decides whether it
/// stacks, with nothing drawn.
pub fn height<A: Pressed>(m: &ButtonsModel<'_, A>, ui: &Ui<'_, A>, w: f32) -> f32 {
    height_as(m, m.end, ui, w)
}

/// [`height`] of [`draw_as`].
pub fn height_as<A: Pressed>(m: &ButtonsModel<'_, A>, end: bool, ui: &Ui<'_, A>, w: f32) -> f32 {
    height_items(&m.items, &m.rule, end, ui, w)
}

/// [`height_as`] for the buttons and rule words themselves.
pub fn height_items<A: Pressed>(
    items: &[ButtonModel<'_, A>],
    rule: &[Shown<'_>],
    end: bool,
    ui: &Ui<'_, A>,
    w: f32,
) -> f32 {
    if items.is_empty() {
        return 0.0;
    }
    let bh = ui.button_height(BUTTON_H);
    if end {
        let total = end_widths(items, ui).iter().sum::<f32>()
            + DIALOG_ACTION_GAP * (items.len() - 1) as f32;
        if total <= w {
            return bh;
        }
    }
    if fits(ui, w, items, rule) {
        return bh;
    }
    let tall = |i: usize, label: &str| {
        let own = stacked(ui, w, label).0;
        match rule.get(i).map(Shown::as_str).filter(|h| *h != label) {
            Some(held) => own.max(stacked(ui, w, held).0),
            None => own,
        }
    };
    items
        .iter()
        .enumerate()
        .map(|(i, b)| tall(i, b.label.as_str()) + GAP)
        .sum::<f32>()
        - GAP
}

/// Each action's width at the end of a row: its label and the padding.
fn end_widths<A: Pressed>(items: &[ButtonModel<'_, A>], ui: &Ui<'_, A>) -> Vec<f32> {
    items
        .iter()
        .map(|b| ui.measure_as(LABEL_L, b.label.as_str()) + 2.0 * BUTTON_PAD)
        .collect()
}

/// How tall a button of a stacked row draws `label` at the width `w`, and the lines it is
/// broken onto at word breaks: none where it fits one line, at a larger text size once fitted
/// down to Standard (`docs/TEXT-SIZE.md` §3).
fn stacked<A: Pressed>(ui: &Ui<'_, A>, w: f32, label: &str) -> (f32, Vec<(usize, usize)>) {
    let bh = ui.button_height(BUTTON_H);
    let room = w - 2.0 * BUTTON_PAD;
    let wide = ui.measure_as(LABEL_L, label);
    let text = ui.text_factor();
    let one_line = if text > 1.0 {
        wide / text <= room
    } else {
        wide + BUTTON_PAD <= w
    };
    if one_line {
        return (bh, Vec::new());
    }
    let lines = ui.wrap_ranges(LABEL_L, label, room).to_vec();
    let tall = lines.len() as f32 * ui.line_as(LABEL_L) + 2.0 * BUTTON_PAD_V;
    (bh.max(tall), lines)
}

/// Whether every label (or the rule's words) fits its share of a row.
fn fits<A: Pressed>(
    ui: &Ui<'_, A>,
    w: f32,
    items: &[ButtonModel<'_, A>],
    rule: &[Shown<'_>],
) -> bool {
    let n = items.len() as f32;
    let share = (w - (n - 1.0) * GAP) / n;
    let words = |i: usize| rule.get(i).unwrap_or(&items[i].label).as_str();
    (0..items.len()).all(|i| ui.measure_as(LABEL_L, words(i)) + BUTTON_PAD <= share)
}
