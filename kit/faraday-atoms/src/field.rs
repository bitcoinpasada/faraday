//! A field: its box and label, a typed line with caret and selection (09 §3.2).

use faraday_draw::Rect;
use faraday_tokens::palette::{Paint, Role};

use crate::Pressed;
use crate::hits::Shape;
use crate::offered::Offered;
use crate::shown::Shown;
use crate::ui::Ui;
use crate::ui::W;
use faraday_draw::text;
use faraday_tokens::dims::{
    BODY_L, BODY_S, DISABLED_CONTAINER, DISABLED_CONTENT, FIELD_EDGE, FIELD_EDGE_ACTIVE, FIELD_GAP,
    FIELD_PAD, FIELD_RADIUS,
};
use faraday_tokens::type_roles::TypeRole;

/// A text field: its label above, its box, the line typed in it and an error line under it
/// (04 §12.1).
pub struct FieldModel<'s, A> {
    pub label: Option<Shown<'s>>,
    /// What is typed (dots already for a masked field).
    pub text: Shown<'s>,
    /// The typing goes to it: primary edge, caret.
    pub focused: bool,
    pub error: Option<Shown<'s>>,
    /// The press that focuses it.
    pub offered: Offered<A>,
}

/// Draws the field with its label's top-left at (x, y), `w` wide. Returns the height it takes.
pub fn draw<A: Pressed>(
    m: &FieldModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let mut top = y;
    if let Some(label) = m.label {
        top += ui.field_label(x, top, label.as_str());
    }
    let error = m.error.map(|e| e.as_str());
    let fh = ui.field_h();
    let off = matches!(
        m.offered,
        Offered::No | Offered::Refused(_, crate::offered::Drawn::Dimmed)
    );
    let height = if off {
        ui.field_h()
    } else {
        ui.field_box((x, top, w), m.focused, error)
    };
    ui.showing(m.text, |ui| {
        ui.offered(&m.offered, |ui, press, off| match press {
            Some(action) if !off => {
                ui.hit_drawn(x, top, w, fh, action.clone());
                ui.typed(
                    x + FIELD_PAD,
                    top,
                    fh,
                    FIELD_TEXT.size,
                    W::R,
                    Role::OnSurface,
                    m.text.as_str(),
                    w - 2.0 * FIELD_PAD,
                    m.focused,
                    action,
                );
            }
            press => {
                ui.field_box_off((x, top, w));
                let ink = Role::OnSurface.at(DISABLED_CONTENT);
                ui.text_mid(
                    x + FIELD_PAD,
                    top,
                    fh,
                    FIELD_TEXT.size,
                    W::R,
                    ink,
                    m.text.as_str(),
                );
                if let Some(action) = press {
                    ui.hit_drawn(x, top, w, fh, action);
                }
            }
        })
    });
    top - y + height
}

/// The fill behind selected text: secondary-container, the selected colour
/// (`docs/THEMES.md` §7).
pub(crate) const SELECTION_FILL: Role = Role::SecondaryContainer;
/// Text on [`SELECTION_FILL`] where it is not drawn in on-surface: on-secondary-container.
pub(crate) const SELECTION_INK: Role = Role::OnSecondaryContainer;
/// A text field's label kept outside it, above (`docs/LAYOUT.md` §8): Material's body small.
pub const FIELD_LABEL: TypeRole = BODY_S;
/// What is typed in a text field: Material's body large.
pub const FIELD_TEXT: TypeRole = BODY_L;
/// The smallest rectangle holding both.
fn union(a: Rect, b: Rect) -> Rect {
    let (x, y) = (a.x.min(b.x), a.y.min(b.y));
    Rect::new(
        x,
        y,
        a.right().max(b.right()) - x,
        a.bottom().max(b.bottom()) - y,
    )
}

/// Where the caret and the selection are in the field typing goes to,
/// in characters from the field's start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edit<A> {
    /// The field they are in, by the action that focuses it. In any
    /// other field the caret is at the end and nothing is selected.
    pub field: Option<A>,
    /// The caret; at the end when `None`.
    pub caret: Option<usize>,
    /// The selection's other end, while something is selected.
    pub anchor: Option<usize>,
}

impl<A> Default for Edit<A> {
    fn default() -> Self {
        Edit {
            field: None,
            caret: None,
            anchor: None,
        }
    }
}

impl<A> Edit<A> {
    /// The caret in a text `len` characters long.
    pub fn caret_in(&self, len: usize) -> usize {
        self.caret.map_or(len, |c| c.min(len))
    }

    /// The characters selected in a text `len` characters long, from
    /// and to, when any are.
    pub fn selected(&self, len: usize) -> Option<(usize, usize)> {
        let a = self.anchor?.min(len);
        let c = self.caret_in(len);
        (a != c).then(|| (a.min(c), a.max(c)))
    }

    /// Whether the whole of a text `len` characters long is selected.
    pub fn all(&self, len: usize) -> bool {
        len > 0 && self.selected(len) == Some((0, len))
    }
}

/// Where a line of typing drew its characters: the pixel column of the
/// edge before each character in view, the first of them `first`, and
/// the one after the last. A press or a drag there places the caret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldAt<A> {
    /// The action the field is pressed with.
    pub action: A,
    /// The first character in view.
    pub first: usize,
    /// The edges, in pixels.
    pub edges: Vec<i32>,
    /// The pixel rows the line stands in, top and bottom, for a field
    /// of several lines ([`Ui::typed_lines`]); `None` for one line.
    pub band: Option<(i32, i32)>,
}

impl<A> FieldAt<A> {
    /// The place between characters nearest pixel column `x`.
    pub fn index_at(&self, x: i32) -> usize {
        let j = self
            .edges
            .iter()
            .enumerate()
            .min_by_key(|(_, e)| (**e - x).abs())
            .map_or(0, |(j, _)| j);
        self.first + j
    }
}

impl<A: Pressed> Ui<'_, A> {
    /// The selection behind text `shown` drawn at (x, y) in a field of
    /// height `h`, when the field is focused and all of it is selected:
    /// for a field drawn as one string, whose caret stays at its end. The
    /// text drawn on it is written in [`SELECTION_INK`].
    #[allow(
        clippy::too_many_arguments,
        reason = "a selection's position, height, size, weight, text and state, as the text helpers take them"
    )]
    pub fn selection(&mut self, x: f32, y: f32, h: f32, size: f32, w: W, shown: &str, on: bool) {
        if on && self.edit.all(shown.chars().count()) {
            let tw = self.measure(size, w, shown);
            let _ = self.selected_band(x, y, h, tw);
        }
    }

    /// The band behind selected text `tw` wide from x, in a field `h`
    /// tall at y ([`Ui::selection_band`]). Returns where it is, in
    /// pixels.
    fn selected_band(&mut self, x: f32, y: f32, h: f32, tw: f32) -> Rect {
        use faraday_tokens::dims::{SELECTION_BAND, SELECTION_PAD};
        let bh = h * SELECTION_BAND;
        let at = (x - SELECTION_PAD, y + (h - bh) / 2.0);
        self.selection_band(at.0, at.1, tw + 2.0 * SELECTION_PAD, bh);
        self.rect(at.0, at.1, tw + 2.0 * SELECTION_PAD, bh)
    }

    /// The band behind selected text, at (x, y), `w` by `h`: one selection
    /// paint wherever text is selected, [`SELECTION_FILL`], the text on
    /// it in [`SELECTION_INK`], which reads on it at 4.5:1 or more in
    /// every scheme and 7:1 at High contrast.
    pub(crate) fn selection_band(&mut self, x: f32, y: f32, w: f32, h: f32) {
        use faraday_tokens::dims::SHAPE_XS;
        self.fill(x, y, w, h, SHAPE_XS, SELECTION_FILL);
    }

    /// One line of typing: `text`, dots
    /// already for a masked field, from x in the middle of a box `h`
    /// tall at y, at most `room` wide. While `focused`, what is selected
    /// is drawn behind it and the caret at its place, and the part in
    /// view is the end or, with the caret before that, from the caret.
    /// Where each character starts is kept for `action`, so a press or
    /// a drag there places the caret. Returns the width drawn.
    #[allow(
        clippy::too_many_arguments,
        reason = "a typed field's position, height, size, weight, colour, text, room, focus and action"
    )]
    pub fn typed(
        &mut self,
        x: f32,
        y: f32,
        h: f32,
        size: f32,
        w: W,
        color: impl Into<Paint>,
        text: &str,
        room: f32,
        focused: bool,
        action: A,
    ) -> f32 {
        let color: Paint = color.into();
        // The byte offset of each character's start, and of the end: the
        // text itself is never copied.
        let at: Vec<usize> = text
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(text.len()))
            .collect();
        let n = at.len() - 1;
        // Each edge by the advances so far, one pass: what
        // [`Ui::measure`] gives each start of the text.
        let face = self.face(size, w);
        let mut pen = 0u32;
        let edge: Vec<f32> = std::iter::once(0.0)
            .chain(text.chars().map(|c| {
                pen += face.advance_64(c);
                text::px_from_64(pen) as f32 / self.f
            }))
            .collect();
        let edit = if focused {
            self.edit.clone()
        } else {
            Edit::default()
        };
        let caret = edit.caret_in(n);
        let mut from = 0;
        while from < n && edge[n] - edge[from] > room {
            from += 1;
        }
        let mut to = n;
        if caret < from {
            from = caret;
            while to > from && edge[to] - edge[from] > room {
                to -= 1;
            }
        }
        let left = |i: usize| x + edge[i] - edge[from];
        let selected = edit.selected(n);
        let shown = &text[at[from]..at[to]];
        let band = selected.and_then(|(lo, hi)| {
            let (lo, hi) = (lo.clamp(from, to), hi.clamp(from, to));
            (hi > lo).then(|| self.selected_band(left(lo), y, h, edge[hi] - edge[lo]))
        });
        match band {
            // What is selected is written in the selection's own ink, the
            // rest beside it as asked.
            Some(b) => {
                // The row, with room for a letter's overhang at its ends.
                let row = self.rect(x - h, y, room + 2.0 * h, h);
                let before = Rect::new(row.x, row.y, (b.x - row.x).max(0), row.h);
                let after = Rect::new(b.right(), row.y, (row.right() - b.right()).max(0), row.h);
                for (clip, ink) in [
                    (before, color),
                    (after, color),
                    (Rect::new(b.x, row.y, b.w, row.h), SELECTION_INK.paint()),
                ] {
                    if clip.w > 0 {
                        self.c.push_clip(clip);
                        self.text_mid(x, y, h, size, w, ink, shown);
                        self.c.pop_clip();
                    }
                }
            }
            None => {
                self.text_mid(x, y, h, size, w, color, shown);
            }
        }
        if focused {
            self.note_focus(x, y, room, h);
            self.focus_action = Some(action.clone());
        }
        if focused && selected.is_none() {
            use faraday_tokens::dims::{CARET_MIN, CARET_OVER};
            let ch = (size + CARET_OVER).max(CARET_MIN);
            self.caret(left(caret) + 1.0, y + (h - ch) / 2.0, ch);
        }
        let edges = (from..=to).map(|i| self.px(left(i) + self.ox)).collect();
        self.fields.push(FieldAt {
            action,
            first: from,
            edges,
            band: None,
        });
        edge[to] - edge[from]
    }

    /// The caret of the field typing goes to, `h` units tall at (x, y),
    /// in the off half of its blink not drawn.
    pub fn caret(&mut self, x: f32, y: f32, h: f32) {
        self.caret_drawn = true;
        self.caret_foot = Some(y + h + self.oy + self.oy_px as f32 / self.f);
        self.note_focus(x, y, 2.0, h);
        if self.caret_on {
            self.fill(x, y, 2.0, h, 1.0, Role::Primary);
        }
    }

    /// Part of the field typing goes to, drawn at (x, y), `w` by `h`
    /// units: its box, its line or its caret, joined into
    /// [`Ui::focus_box`] with the label drawn just above it.
    pub fn note_focus(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let top = match self.label_at {
            Some((label, foot)) if (foot - (y + self.oy)).abs() < 0.5 => label - self.oy,
            _ => y,
        };
        let part = self.rect(x, top, w, y + h - top);
        self.focus_box = Some(self.focus_box.map_or(part, |b| union(b, part)));
    }

    /// The caret as a character after the text typed, for a field drawn
    /// as one string: a bar, or nothing in the off half of its blink.
    pub fn caret_char(&mut self) -> &'static str {
        self.caret_drawn = true;
        if self.caret_on { "|" } else { "" }
    }

    /// A chip inside the empty entry field at (`x`, `y`, `w`, `h`), at its
    /// text inset, secondary-styled and vertically centred
    /// (`docs/MODES.md` §6.1). Its hit target is the chip, at the touch
    /// floor's height on a small panel and the field's on a wide one.
    /// Returns the chip's width; the caller passes `label`, the shorter
    /// one where the field is narrower than the chip.
    pub fn field_action(&mut self, x: f32, y: f32, w: f32, h: f32, label: &str, action: A) -> f32 {
        use faraday_tokens::dims::SHAPE_S;
        use faraday_tokens::dims::{
            FIELD_ACTION_HEIGHT as CH, FIELD_ACTION_INSET as IN, FIELD_ACTION_TEXT as TX,
        };
        // A one-line slot: the field's inset either side of the chip and
        // the chip's own inside it; larger text fits down to Standard
        // (`docs/TEXT-SIZE.md` §3).
        let room = (w - 2.0 * IN) - 2.0 * IN;
        let tw = self.text_fit_width(TX, label, room);
        let width = (tw + 2.0 * IN).min(w - 2.0 * IN);
        let (cx, cy) = (x + IN, y + (h - CH) / 2.0);
        let hit_h = if self.compact {
            self.touch.max(CH)
        } else {
            h.max(CH)
        };
        let target = (cx, y + (h - hit_h) / 2.0, width, hit_h);
        let shape = Shape::new(cx, cy, width, CH, SHAPE_S);
        self.press_to(shape, target, Role::Primary.paint(), action);
        self.stroke(cx, cy, width, CH, SHAPE_S, Role::Outline);
        let band = (cx + (width - tw) / 2.0, cy, CH, room);
        self.text_fit_as(band, TX, Role::Primary, label);
        width
    }
}

impl<A: Pressed> Ui<'_, A> {
    /// A text field's label kept outside it, at (x, y) above where the
    /// field goes. Returns the height it takes with the gap under it.
    pub fn field_label(&mut self, x: f32, y: f32, label: &str) -> f32 {
        self.text_as(x, y, FIELD_LABEL, Role::OnSurfaceVariant, label);
        let h = self.line_as(FIELD_LABEL) + FIELD_GAP;
        self.label_at = Some((y + self.oy, y + h + self.oy));
        h
    }

    /// An outlined text field's box (Material 3 Text fields) at (x, y),
    /// `w` wide and [`FIELD_H`] tall: a 1 dp outline at rest, 2 dp in
    /// primary while `focused`, 2 dp in error with `error` as a line under
    /// it while there is one. What is typed is the caller's, from
    /// [`FIELD_PAD`] in. Returns the height with the error line.
    pub fn field_box(&mut self, at: (f32, f32, f32), focused: bool, error: Option<&str>) -> f32 {
        let fh = self.field_h();
        self.field_box_sized(at, fh, (focused, true), error)
    }

    /// [`Ui::field_box`] `fh` tall: a field that grows with its lines.
    /// `(focused, whole)`: `whole` keeps the whole field in view above a
    /// keyboard; a field that grows with its text leaves it to the caller,
    /// which keeps the line being typed. Returns the height with the
    /// error line.
    pub fn field_box_sized(
        &mut self,
        (x, y, w): (f32, f32, f32),
        fh: f32,
        (focused, whole): (bool, bool),
        error: Option<&str>,
    ) -> f32 {
        let (edge, width) = match (error, focused) {
            (Some(_), _) => (Role::Error, FIELD_EDGE_ACTIVE),
            (None, true) => (Role::Primary, FIELD_EDGE_ACTIVE),
            (None, false) => (Role::Outline, FIELD_EDGE),
        };
        if focused && whole {
            let h = self.field_height(w, error).max(fh);
            self.note_focus(x, y, w, h);
        }
        let rect = self.rect(x, y, w, fh);
        let color = self.col(edge);
        let line = (width * self.f).max(1.0);
        self.c
            .stroke_rounded_rect(rect, FIELD_RADIUS * self.f, line, color);
        match error {
            Some(e) => {
                let at = (x + FIELD_PAD, y + fh + FIELD_GAP, w - 2.0 * FIELD_PAD);
                fh + FIELD_GAP + self.wrap_as(at, BODY_S, Role::Error, e)
            }
            None => fh,
        }
    }

    /// A disabled outlined text field's box (Material 3 Text fields,
    /// disabled state): one even 1 dp outline at on-surface 12%, and
    /// nothing behind or over it, a trailing icon's area included. What
    /// the field holds is drawn by the caller at on-surface 38%
    /// (`DISABLED_CONTENT`).
    pub fn field_box_off(&mut self, (x, y, w): (f32, f32, f32)) {
        let rect = self.rect(x, y, w, self.field_h());
        let color = self.col(Role::OnSurface.at(DISABLED_CONTAINER));
        let line = (FIELD_EDGE * self.f).max(1.0);
        self.c
            .stroke_rounded_rect(rect, FIELD_RADIUS * self.f, line, color);
    }

    /// A field's supporting text (Material 3 Text fields) under it at
    /// (x, y), `w` wide, in body small and `color`: while `focused` it
    /// is part of the field typing goes to ([`Ui::focus_box`]), kept in
    /// view above the keyboard with it. Returns its height.
    pub fn field_supporting(
        &mut self,
        (x, y, w): (f32, f32, f32),
        color: impl Into<Paint>,
        text: &str,
        focused: bool,
    ) -> f32 {
        let h = self.wrap_as((x, y, w), BODY_S, color, text);
        if focused {
            self.note_focus(x, y, w, h);
        }
        h
    }

    /// The height [`Ui::field_box`] takes `w` wide with `error`.
    pub fn field_height(&self, w: f32, error: Option<&str>) -> f32 {
        let fh = self.field_h();
        match error {
            Some(e) => fh + FIELD_GAP + self.wrap_height_as(BODY_S, e, w - 2.0 * FIELD_PAD),
            None => fh,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ui::testing::*;
    use faraday_draw::canvas::InkKind;

    /// Invariant: A field in error says what is wrong in a line under the field, in the error colour, and takes that line's room; a field at rest shows no such line, and its label stands above it.
    /// Regression: The error line is drawn over the field, past its room, or in the text colour.
    /// Owner: kit
    #[test]
    fn a_field_in_error_says_why_under_it() {
        let why = "Not an address on this network";
        let mut canvas = canvas(160);
        let (mut rest, mut wrong, mut top, mut field_h) = (0.0, 0.0, 0.0, 0.0);
        frame(&mut canvas, &at_rest(None), |ui| {
            ui.clear(page());
            field_h = ui.field_h();
            top = 16.0 + ui.field_label(16.0, 16.0, "Address");
            rest = ui.field_box((16.0, top, 280.0), false, None);
            wrong = ui.field_box((320.0, top, 280.0), false, Some(why));
        });
        assert!(wrong > rest, "{wrong} {rest}");
        let error = error_in(crate::Theme::default());
        let mut said = false;
        for i in canvas.ink() {
            if let InkKind::Text { text, color, .. } = &i.kind {
                if text == "Address" {
                    assert!(
                        i.rect.bottom() as f32 <= top,
                        "the label is inside the field"
                    );
                }
                if text == why {
                    said = true;
                    assert_eq!(*color, error);
                    assert!(i.rect.x >= 320, "the error line is not under its field");
                    assert!(
                        i.rect.y as f32 >= top + field_h,
                        "{:?} not under the field",
                        i.rect
                    );
                    assert!(
                        i.rect.bottom() as f32 <= top + wrong,
                        "{:?} past its room",
                        i.rect
                    );
                }
            }
        }
        assert!(said, "the error is not shown");
    }
}
