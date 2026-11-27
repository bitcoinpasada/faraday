//! The pass molecule: the passphrase field's drawing, from the view the passphrase part gives
//! (09 §3.2; 04 §12.3; `docs/KEYS.md` §2).
//!
//! It draws only what the view holds: dots, or the revealed text, which comes in through
//! `Shown::secret` and is drawn from the borrow, never copied (V19, K12). The model keeps no
//! copy of the text, and a hidden field's model holds a count and no text at all.

use faraday_atoms::hits::{Shape, Unpressed};
use faraday_atoms::{Icon, Pressed, Shown, Ui, W};
use faraday_atoms::PlainRef;
use faraday_text::Text;
use faraday_tokens::dims::{
    BODY_M, FIELD_RADIUS, GAP, GAP_SMALL, PASS_CODE_INSET, PASS_CODE_TEXT, RADIUS_SMALL, SHAPE_S,
    STATE_PRESS, TYPED_INSET,
};
use faraday_tokens::palette::Role;

/// What the field holds, as the part's read view gives it. The variants are private: a revealed
/// text is made only by [`PassView::revealed`], through `Shown::secret`.
enum Held<'s> {
    /// Hidden by the eye: this many dots, and no text.
    Dots(usize),
    /// Revealed by the eye: the text, a secret on show.
    Revealed(Shown<'s>),
    /// A public value the field holds (Practice's passphrase): in its code box, unmasked, no eye.
    Public(Shown<'s>),
}

/// The view the field draws.
pub struct PassView<'s>(Held<'s>);

impl<'s> PassView<'s> {
    /// The eye is closed: `count` characters as dots.
    pub fn dots(count: usize) -> Self {
        PassView(Held::Dots(count))
    }

    /// The eye is open: the passphrase, borrowed from the part, as a secret on show.
    pub fn revealed(text: &'s str) -> Self {
        PassView(Held::Revealed(Shown::secret(text)))
    }

    /// A public passphrase, drawn in a code box with no eye.
    pub fn public(text: impl Into<PlainRef<'s>>) -> Self {
        PassView(Held::Public(Shown::plain(text)))
    }

    /// Whether the field holds nothing.
    pub fn is_empty(&self) -> bool {
        match &self.0 {
            Held::Dots(n) => *n == 0,
            Held::Revealed(t) | Held::Public(t) => t.as_str().is_empty(),
        }
    }
}

/// The passphrase field. What a press on the box or on the eye sends is held by the model; it is
/// always offered (a field that takes no typing is the part's own state, not drawn here).
pub struct PassModel<'s, A> {
    pub view: PassView<'s>,
    /// Typing goes to the field.
    pub focused: bool,
    /// The passphrase waits for the eye to be opened once: the eye in primary.
    pub marked: bool,
    /// What the empty, unfocused field says in place of its dots, in the mono face.
    pub hint: Option<Shown<'s>>,
    /// The words of the pill an empty field wears in place of dots ("No passphrase"); none draws
    /// the field empty.
    pub none: Option<Shown<'s>>,
    /// The eye's tooltip, Show or Hide by whether the text is on show.
    pub eye_tip: Shown<'s>,
    /// A press on the box.
    pub press: A,
    /// A press on the eye.
    pub eye: A,
}

/// The eye's glyph in the field.
const EYE_GLYPH: f32 = 15.0;

/// Draws the field from (x, y), `w` wide, as tall as a field is at the text size. Returns its
/// height.
pub fn draw<A: Pressed>(m: &PassModel<'_, A>, ui: &mut Ui<'_, A>, (x, y, w): (f32, f32, f32)) -> f32 {
    let h = ui.field_h();
    if let Held::Public(text) = &m.view.0 {
        code_field(ui, (x, y, w, h), text);
        ui.press_box(x, y, w, h, m.press.clone());
        return h;
    }
    ui.fill(x, y, w, h, RADIUS_SMALL, Role::SurfaceContainerHighest);
    let edge = if m.focused {
        Role::Primary
    } else {
        Role::OutlineVariant
    };
    ui.stroke(x, y, w, h, RADIUS_SMALL, edge);
    ui.press_box(x, y, w - h, h, m.press.clone());
    let room = w - TYPED_INSET - h;
    let empty = m.view.is_empty();
    match (&m.hint, &m.none) {
        (Some(hint), _) if empty && !m.focused => {
            ui.showing(hint, |ui| {
                let line = ui.fit_as(BODY_M.mono(), hint.as_str(), room);
                ui.text_mid_as((x + TYPED_INSET, y, h), BODY_M.mono(), Role::OnSurface, &line);
            });
            eye(m, ui, (x, y, w), h);
        }
        (_, Some(none)) if empty => {
            no_passphrase(ui, (x, y, h), none, m.focused);
        }
        _ => {
            typed(m, ui, (x + TYPED_INSET, y, h), room);
            eye(m, ui, (x, y, w), h);
        }
    }
    h
}

/// What is typed: the dots, or the revealed text drawn from its borrow.
fn typed<A: Pressed>(m: &PassModel<'_, A>, ui: &mut Ui<'_, A>, (x, y, h): (f32, f32, f32), room: f32) {
    let ink = Role::OnSurface;
    let at = (x, y, h);
    match &m.view.0 {
        Held::Revealed(text) => ui.showing(text, |ui| {
            let size = BODY_M.size;
            ui.typed(at.0, at.1, at.2, size, W::M, ink, text.as_str(), room, m.focused, m.press.clone());
        }),
        Held::Dots(n) => {
            let dots = Text::repeated("\u{2022}", *n);
            let size = BODY_M.size;
            ui.typed(at.0, at.1, at.2, size, W::M, ink, dots.as_str(), room, m.focused, m.press.clone());
        }
        Held::Public(_) => {}
    }
}

/// An empty field's content: the pill that says there is no passphrase at the start of the box
/// `h` tall at (x, y), and the caret after it while the field is focused.
fn no_passphrase<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y, h): (f32, f32, f32),
    label: &Shown<'_>,
    focused: bool,
) {
    let ph = 24.0;
    let pw = ui.showing(label, |ui| {
        ui.pill_in(x + 8.0, y + (h - ph) / 2.0, ph, label.as_str(), Role::OnSurfaceVariant)
    });
    if focused {
        ui.caret(x + 14.0 + pw, y + (h - 18.0) / 2.0, 18.0);
    }
}

/// The eye inside the field's right end, over any long text: `h` is the field's height.
fn eye<A: Pressed>(m: &PassModel<'_, A>, ui: &mut Ui<'_, A>, (x, y, w): (f32, f32, f32), h: f32) {
    let shown = matches!(m.view.0, Held::Revealed(_));
    let ex = x + w - h;
    let inset = Role::SurfaceContainerHighest;
    let (bx, by, side) = (ex + 8.0, y + 8.0, h - 16.0);
    ui.fill(bx, by, side, side, 6.0, inset);
    if ui.is_pressed(&m.eye) {
        ui.wash(bx, by, side, side, 6.0, Role::OnSurface.at(STATE_PRESS));
    }
    let icon = if shown { Icon::EyeOff } else { Icon::Eye };
    let tint = if shown || m.marked {
        Role::Primary
    } else {
        Role::OnSurfaceVariant
    };
    ui.icon(ex, y, h, icon, EYE_GLYPH, tint);
    let rect = (ex, y, h, h);
    ui.press_unpressed(
        Shape::new(ex, y, h, h, SHAPE_S),
        Role::OnSurface.paint(),
        Unpressed::HeldSecret,
        m.eye.clone(),
    );
    ui.tip_plain(rect, &m.eye, m.eye_tip.as_str());
}

/// A public passphrase in its code box at the field's text inset, unmasked
/// (`docs/MODES.md` §6.2). No eye.
fn code_field<A: Pressed>(ui: &mut Ui<'_, A>, (x, y, w, h): (f32, f32, f32, f32), text: &Shown<'_>) {
    ui.fill(x, y, w, h, FIELD_RADIUS, Role::SurfaceContainerHighest);
    ui.stroke(x, y, w, h, FIELD_RADIUS, Role::Outline);
    let size = PASS_CODE_TEXT;
    let line = ui.line(size, W::M) + 2.0 * GAP_SMALL;
    let top = y + (h - line) / 2.0;
    let bw = ui.measure(size, W::M, text.as_str()) + 2.0 * GAP;
    ui.fill(x + PASS_CODE_INSET, top, bw, line, RADIUS_SMALL, Role::SurfaceContainerHighest);
    ui.showing(text, |ui| {
        ui.text_mid(x + PASS_CODE_INSET + GAP, top, line, size, W::M, Role::OnSurface, text.as_str());
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::*;
    use faraday_draw::canvas::InkKind;

    const PASSPHRASE: &str = "correct horse";

    fn model(view: PassView<'_>) -> PassModel<'_, P> {
        PassModel {
            view,
            focused: false,
            marked: false,
            hint: None,
            none: None,
            eye_tip: Shown::plain("Show"),
            press: P(1),
            eye: P(2),
        }
    }

    /// The frame of a field and every text drawn on it.
    fn drawn(m: &PassModel<'_, P>) -> (faraday_atoms::FrameOut<P>, Vec<String>) {
        let mut canvas = canvas(160);
        let out = frame(&mut canvas, &at_rest(None), |ui| {
            ui.clear(page());
            draw(m, ui, (16.0, 16.0, 320.0));
        });
        let texts = canvas
            .ink()
            .iter()
            .filter_map(|i| match &i.kind {
                InkKind::Text { text, .. } => Some(text.as_str().to_string()),
                _ => None,
            })
            .collect();
        (out, texts)
    }

    /// Invariant: A passphrase on show sets the frame's secret flag and its ink is marked secret; one hidden by the eye draws dots only, no character of it, and sets no flag.
    /// Regression: A hidden passphrase's text reaches the screen, or a revealed one is drawn without telling the frame, so the lock check misses it.
    /// Owner: kit
    #[test]
    fn a_revealed_passphrase_marks_the_frame_and_a_hidden_one_draws_no_character_of_it() {
        let shown = model(PassView::revealed(PASSPHRASE));
        let (out, texts) = drawn(&shown);
        assert!(out.secret, "a revealed passphrase leaves the frame unmarked");
        assert!(texts.iter().any(|t| t.contains(PASSPHRASE)), "{texts:?}");

        let count = PASSPHRASE.chars().count();
        let hidden = model(PassView::dots(count));
        let (out, texts) = drawn(&hidden);
        assert!(!out.secret);
        let leaked = |t: &String| t.chars().any(|c| c != '\u{2022}' && PASSPHRASE.contains(c));
        assert!(!texts.iter().any(leaked), "{texts:?}");
        assert!(texts.iter().any(|t| t.chars().count() == count));
    }

    /// Invariant: The eye and the box are two presses; a press on the eye never reaches the box's.
    /// Regression: The box's press covers the eye, so opening the passphrase focuses the field instead.
    /// Owner: kit
    #[test]
    fn the_eye_takes_its_own_press_apart_from_the_box() {
        let (out, _) = drawn(&model(PassView::dots(3)));
        let eye = out.hits.iter().find(|h| h.press == P(2)).expect("no eye");
        let field = out.hits.iter().find(|h| h.press == P(1)).expect("no box");
        assert!(field.rect.right() <= eye.rect.x + 1, "the box runs under the eye");
    }
}
