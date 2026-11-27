//! The options molecule: a list of options of which one is chosen, each a rounded box holding
//! its name and a line under or beside it (09 §3.2; 04 §12.3 `ChoiceModel`), and the kind rows
//! that draw through it.

use faraday_atoms::button::{ButtonModel, Style};
use faraday_atoms::hits::Shape;
use faraday_atoms::{Icon, Offered, Pressed, Shown, Ui};
use faraday_tokens::dims::{
    BODY_S, BUTTON_H, ICON_BUTTON, ICON_SMALL, TITLE_S, TOUCH, shape_full,
};
use faraday_tokens::palette::{Role, on_selected, selected_or};

/// An information button after a word of an option's name, whose rich tooltip says more. A press
/// on the button or on the word sends `press`.
pub struct OptionTip<'s, A> {
    /// The word of the name the button follows.
    pub word: Shown<'s>,
    /// What the tooltip says.
    pub text: Shown<'s>,
    pub press: A,
}

/// One option: its name, the line that says what it is, whether it is the chosen one and what a
/// press on it sends.
pub struct OptionRow<'s, A> {
    pub name: Shown<'s>,
    pub line: Shown<'s>,
    pub on: bool,
    pub press: A,
    pub tip: Option<OptionTip<'s, A>>,
}

/// A list of options, and the button after it that shows more of them.
pub struct ChoiceModel<'s, A> {
    pub rows: Vec<OptionRow<'s, A>>,
    /// The words and press of the button that shows more options, under the last.
    pub more: Option<(Shown<'s>, A)>,
    /// A press on an option or the button is taken.
    pub enabled: bool,
}

/// The room above a row's text, inside its box.
const TOP: f32 = 6.0;
/// A row's text from its left edge, and the room kept at both sides.
const INSET: f32 = 14.0;
/// The room a row's line leaves for its name on the wide layout.
const NAME_ROOM: f32 = 260.0;
/// A row's corner radius.
const ROW_R: f32 = 8.0;
/// One row's top to the next's, at the touch floor.
const ROW_STEP: f32 = 54.0;
/// The height the button after the rows takes.
const MORE_H: f32 = 44.0;

/// Draws the list from (x, y), `w` wide. Returns its height.
pub fn draw<A: Pressed>(m: &ChoiceModel<'_, A>, ui: &mut Ui<'_, A>, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = y;
    for r in &m.rows {
        let h = row(r, ui, m.enabled, (x, cy, w));
        cy += ROW_STEP - TOUCH + h;
    }
    if let Some((label, press)) = m.more.as_ref().filter(|_| m.enabled) {
        let b = ButtonModel {
            label: label.clone(),
            style: Style::Text,
            offered: Offered::Yes(press.clone()),
            chosen: false,
            icon: None,
        };
        faraday_atoms::button::draw(&b, ui, (x, cy, None, BUTTON_H));
        cy += MORE_H;
    }
    cy - y
}

/// One option of the list at (x, cy), `w` wide. Returns its height.
fn row<A: Pressed>(r: &OptionRow<'_, A>, ui: &mut Ui<'_, A>, enabled: bool, (x, cy, w): (f32, f32, f32)) -> f32 {
    let (on, name, line) = (r.on, r.name.as_str(), r.line.as_str());
    let size = TITLE_S;
    let left = x + INSET;
    let inner = w - 2.0 * INSET;
    let (names, lines) = if ui.compact {
        let room = if r.tip.is_some() { ICON_BUTTON } else { 0.0 };
        (
            lines_of(ui, size, name, inner - room),
            lines_of(ui, BODY_S, line, inner),
        )
    } else {
        (Vec::new(), Vec::new())
    };
    let (name_h, line_h) = (ui.line_as(size), ui.line_as(BODY_S));
    let line_at = cy + TOP + names.len().max(1) as f32 * name_h;
    let h = if ui.compact {
        (line_at + lines.len() as f32 * line_h + TOP - cy).max(TOUCH)
    } else {
        TOUCH
    };
    ui.fill(x, cy, w, h, ROW_R, selected_or(on, Role::SurfaceContainerHighest));
    ui.stroke(x, cy, w, h, ROW_R, selected_or(on, Role::OutlineVariant));
    if enabled {
        ui.press_box(x, cy, w, h, r.press.clone());
    }
    let (ink, muted) = (
        on_selected(on, Role::OnSurface),
        on_selected(on, Role::OnSurfaceVariant),
    );
    let mut tip = None;
    ui.showing(&r.name, |ui| {
        ui.showing(&r.line, |ui| {
            if ui.compact {
                for (n, text) in names.iter().enumerate() {
                    let ty = cy + TOP + n as f32 * name_h;
                    let end = left + ui.text_as(left, ty, size, ink, text);
                    if let Some(t) = &r.tip {
                        tip = tip.or(word_at(ui, (left, end, ty + name_h / 2.0), size, (t.word.as_str(), text)));
                    }
                }
                for (n, text) in lines.iter().enumerate() {
                    let ty = line_at + n as f32 * line_h;
                    ui.text_as(left, ty, BODY_S, muted, text);
                }
            } else {
                let end = left + ui.text_mid_as((left, cy, h), size, ink, name);
                let line = ui.fit_as(BODY_S, line, w - NAME_ROOM);
                ui.text_right_as((x + w - INSET, cy, h), BODY_S, muted, &line);
                if let Some(t) = &r.tip {
                    tip = word_at(ui, (left, end, cy + h / 2.0), size, (t.word.as_str(), name));
                }
            }
        });
    });
    if let (Some(at), Some(t)) = (tip, &r.tip) {
        shares_tip(ui, at, (cy, h), t);
    }
    h
}

/// Where `word` stands in `shown`, which starts at `x`: its left edge, its width, where the
/// line's text ends and the line's middle.
fn word_at<A: Pressed>(
    ui: &Ui<'_, A>,
    (x, end, mid): (f32, f32, f32),
    size: faraday_tokens::type_roles::TypeRole,
    (word, shown): (&str, &str),
) -> Option<(f32, f32, f32, f32)> {
    if word.is_empty() {
        return None;
    }
    let at = shown.find(word)?;
    let from = x + ui.measure_as(size, &shown[..at]);
    Some((from, ui.measure_as(size, word), end, mid))
}

/// Material's info icon button after the word, and the rich tooltip on both.
fn shares_tip<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (word, word_w, end, mid): (f32, f32, f32, f32),
    (y, h): (f32, f32),
    tip: &OptionTip<'_, A>,
) {
    let (bx, by, side) = info_button(ui, (end, mid), &tip.press);
    ui.tip_plain((bx, by, side, side), &tip.press, tip.text.as_str());
    ui.tip_plain((word, y, word_w, h), &tip.press, tip.text.as_str());
}

/// Material's info icon button (an [`ICON_BUTTON`] round, an [`ICON_SMALL`] glyph in the
/// secondary ink, a touch target taking the press only on the button) with its left edge at
/// `end`, centred on `mid`. Returns the target's (x, y, side).
fn info_button<A: Pressed>(ui: &mut Ui<'_, A>, (end, mid): (f32, f32), press: &A) -> (f32, f32, f32) {
    // The button is a full touch target around the icon button.
    let side = ui.touch.max(ICON_BUTTON);
    let (bx, by) = (end - (side - ICON_BUTTON) / 2.0, mid - side / 2.0);
    ui.press(
        Shape::new(bx, by, side, side, shape_full(side)),
        Role::OnSurface.paint(),
        press.clone(),
    );
    let inset = (side - ICON_BUTTON) / 2.0;
    ui.icon(bx + inset, by + inset, ICON_BUTTON, Icon::Info, ICON_SMALL, Role::OnSurfaceVariant);
    (bx, by, side)
}

/// `s` broken at its spaces into lines no wider than `max` in `size`, a word wider than `max`
/// on a line of its own: the lines drawn one by one, so where each word lands is known. Slices
/// of `s`, nothing copied.
fn lines_of<'s, A: Pressed>(
    ui: &Ui<'_, A>,
    size: faraday_tokens::type_roles::TypeRole,
    s: &'s str,
    max: f32,
) -> Vec<&'s str> {
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
            Some(from) if ui.measure_as(size, &s[from..word_end]) > max => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::*;

    fn list(enabled: bool) -> ChoiceModel<'static, P> {
        let row = |i: u8, on: bool| OptionRow {
            name: Shown::plain("Name"),
            line: Shown::plain("What it is"),
            on,
            press: P(i),
            tip: None,
        };
        ChoiceModel {
            rows: vec![row(1, true), row(2, false)],
            more: Some((Shown::plain("More"), P(9))),
            enabled,
        }
    }

    fn drawn(m: &ChoiceModel<'_, P>) -> (Vec<u8>, f32) {
        let mut canvas = canvas(160);
        let mut h = 0.0;
        let out = frame(&mut canvas, &at_rest(None), |ui| {
            ui.clear(page());
            h = draw(m, ui, (8.0, 8.0, 320.0));
        });
        (out.hits.iter().map(|h| h.press.0).collect(), h)
    }

    /// Invariant: Each option and the button after the list take a press while the list is enabled, and none while it is not.
    /// Regression: A list that is not available still takes presses, or its button under the rows is lost.
    /// Owner: kit
    #[test]
    fn a_list_takes_presses_only_while_it_is_enabled() {
        let (on, tall) = drawn(&list(true));
        assert_eq!(on, [1, 2, 9]);
        let (off, short) = drawn(&list(false));
        assert!(off.is_empty(), "{off:?}");
        assert!(tall > short, "the button takes room under the rows");
    }
}
