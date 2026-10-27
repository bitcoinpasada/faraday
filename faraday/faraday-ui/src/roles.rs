//! Text by Material 3's type roles, and the size set's text field and
//! list item (`docs/LAYOUT.md` §8, §9): the pieces of [`Ui`] a screen
//! draws with a [`TypeRole`] in place of a size and a face.
//!
//! Each `_as` method is the method of the same name without it, at the
//! role's size and in its face, so a line converts as it stands:
//! `ui.text(x, y, 12.0, W::R, ink, s)` becomes
//! `ui.text_as(x, y, BODY_S, ink, s)`. Two differ in what they give
//! back: [`Ui::line_as`] is the role's line height, not the face's, and
//! [`Ui::wrap_as`] spaces its lines by it.

use crate::pal::*;
use crate::{Ui, W};
use faraday_draw::text;
use faraday_draw::tokens::{
    BODY_L, BODY_M, BODY_S, FIELD_EDGE, FIELD_EDGE_ACTIVE, FIELD_GAP, FIELD_H, FIELD_PAD,
    FIELD_RADIUS, GAP, LABEL_S, LIST_1, LIST_2, LIST_3, LIST_3_TOP, LIST_PAD, SHAPE_L, TypeFace,
    TypeRole,
};

impl From<TypeFace> for W {
    fn from(face: TypeFace) -> W {
        match face {
            TypeFace::Regular => W::R,
            TypeFace::Medium => W::S,
            TypeFace::Mono => W::M,
        }
    }
}

/// A text field's label kept outside it, above (`docs/LAYOUT.md` §8):
/// Material's body small.
pub const FIELD_LABEL: TypeRole = BODY_S;

/// What is typed in a text field: Material's body large.
pub const FIELD_TEXT: TypeRole = BODY_L;

/// A list item (Material 3 Lists): its headline, its supporting text
/// (none for a one-line item), what stands at its end and in what
/// colour, and what a press on it does.
pub struct ListItem<'s, A> {
    /// The headline: Material's body large.
    pub headline: &'s str,
    /// The supporting text under it, on one line or two: Material's body
    /// medium. Empty for a one-line item.
    pub supporting: &'s str,
    /// The trailing text, level with the headline: Material's label
    /// small.
    pub trailing: Option<(&'s str, Paint)>,
    /// What a press does.
    pub action: A,
}

impl<A: Copy + PartialEq> Ui<'_, A> {
    /// The height of one line in `role`, in design units: its line height
    /// at this canvas's type size.
    pub fn line_as(&self, role: TypeRole) -> f32 {
        role.line * self.c.scale().factor() / self.f
    }

    /// The width of `s` in `role`.
    pub fn measure_as(&self, role: TypeRole, s: &str) -> f32 {
        self.measure(role.size, role.face.into(), s)
    }

    /// `s` in `role` with its top-left at (x, y). Returns its width.
    pub fn text_as(
        &mut self,
        x: f32,
        y: f32,
        role: TypeRole,
        color: impl Into<Paint>,
        s: &str,
    ) -> f32 {
        self.text(x, y, role.size, role.face.into(), color, s)
    }

    /// `s` in `role` centred vertically in a band `h` tall from y.
    pub fn text_mid_as(
        &mut self,
        (x, y, h): (f32, f32, f32),
        role: TypeRole,
        color: impl Into<Paint>,
        s: &str,
    ) -> f32 {
        self.text_mid(x, y, h, role.size, role.face.into(), color, s)
    }

    /// `s` in `role` ending at `right`, centred in a band `h` tall from y.
    pub fn text_right_as(
        &mut self,
        (right, y, h): (f32, f32, f32),
        role: TypeRole,
        color: impl Into<Paint>,
        s: &str,
    ) {
        self.text_right(right, y, h, role.size, role.face.into(), color, s);
    }

    /// `s` in `role` cut with an ellipsis to fit `max`.
    pub fn fit_as(&self, role: TypeRole, s: &str, max: f32) -> String {
        self.fit(role.size, role.face.into(), s, max)
    }

    /// How many lines [`Ui::wrap_as`] breaks `s` onto at `max` width:
    /// one at least.
    pub fn wrap_lines_as(&self, role: TypeRole, s: &str, max: f32) -> usize {
        let face = self.font(role.size, role.face.into()).sized(self.c.scale());
        text::wrap(&face, s, self.px(max)).len().max(1)
    }

    /// The height [`Ui::wrap_as`] takes for `s` at `width`, drawn
    /// nowhere.
    pub fn wrap_height_as(&self, role: TypeRole, s: &str, width: f32) -> f32 {
        let face = self.font(role.size, role.face.into()).sized(self.c.scale());
        text::wrap(&face, s, self.px(width)).len() as f32 * self.line_as(role)
    }

    /// `s` in `role` wrapped to `width`, its lines `role`'s line height
    /// apart, the first with its top at y as [`Ui::text_as`] draws it.
    /// Returns the height used: the lines times the line height.
    pub fn wrap_as(
        &mut self,
        (x, y, width): (f32, f32, f32),
        role: TypeRole,
        color: impl Into<Paint>,
        s: &str,
    ) -> f32 {
        let color = self.ink(color.into());
        let face = self.font(role.size, role.face.into()).sized(self.c.scale());
        let lines = text::wrap(&face, s, self.px(width));
        let line = self.line_as(role);
        let x0 = self.px(x + self.ox);
        for (i, &(start, end)) in lines.iter().enumerate() {
            let ly = self.px(y + self.oy + i as f32 * line);
            self.c.text_with_face(&face, x0, ly, &s[start..end], color);
        }
        lines.len() as f32 * line
    }

    /// A text field's label kept outside it, at (x, y) above where the
    /// field goes. Returns the height it takes with the gap under it.
    pub fn field_label(&mut self, x: f32, y: f32, label: &str) -> f32 {
        self.text_as(x, y, FIELD_LABEL, Role::OnSurfaceVariant, label);
        self.line_as(FIELD_LABEL) + FIELD_GAP
    }

    /// An outlined text field's box (Material 3 Text fields) at (x, y),
    /// `w` wide and [`FIELD_H`] tall: a 1 dp outline at rest, 2 dp in
    /// primary while `focused`, 2 dp in error with `error` as a line under
    /// it while there is one. What is typed is the caller's, from
    /// [`FIELD_PAD`] in. Returns the height with the error line.
    pub fn field_box(
        &mut self,
        (x, y, w): (f32, f32, f32),
        focused: bool,
        error: Option<&str>,
    ) -> f32 {
        let (edge, width) = match (error, focused) {
            (Some(_), _) => (Role::Error, FIELD_EDGE_ACTIVE),
            (None, true) => (Role::Primary, FIELD_EDGE_ACTIVE),
            (None, false) => (Role::Outline, FIELD_EDGE),
        };
        let rect = self.rect(x, y, w, FIELD_H);
        let color = self.col(edge);
        let line = (width * self.f).max(1.0);
        self.c
            .stroke_rounded_rect(rect, FIELD_RADIUS * self.f, line, color);
        match error {
            Some(e) => {
                let at = (x + FIELD_PAD, y + FIELD_H + FIELD_GAP, w - 2.0 * FIELD_PAD);
                FIELD_H + FIELD_GAP + self.wrap_as(at, BODY_S, Role::Error, e)
            }
            None => FIELD_H,
        }
    }

    /// The height [`Ui::field_box`] takes `w` wide with `error`.
    pub fn field_height(&self, w: f32, error: Option<&str>) -> f32 {
        match error {
            Some(e) => FIELD_H + FIELD_GAP + self.wrap_height_as(BODY_S, e, w - 2.0 * FIELD_PAD),
            None => FIELD_H,
        }
    }

    /// How many lines `it` takes `w` wide: one with no supporting text,
    /// two where it fits on a line, three where it takes two.
    fn list_lines(&self, w: f32, it: &ListItem<'_, A>) -> usize {
        if it.supporting.is_empty() {
            1
        } else if self.measure_as(BODY_M, it.supporting) <= w - 2.0 * LIST_PAD {
            2
        } else {
            3
        }
    }

    /// The height [`Ui::list_item`] takes for `it`, `w` wide:
    /// [`LIST_1`], [`LIST_2`] or [`LIST_3`].
    pub fn list_item_height(&self, w: f32, it: &ListItem<'_, A>) -> f32 {
        match self.list_lines(w, it) {
            1 => LIST_1,
            2 => LIST_2,
            _ => LIST_3,
        }
    }

    /// A list item (Material 3 Lists) from (x, y), `w` wide, of one, two
    /// or three lines by its supporting text (the third line cut with an
    /// ellipsis); `chosen` in Material's selected state (secondary
    /// container). The whole item takes its press. Returns its height.
    pub fn list_item(
        &mut self,
        (x, y, w): (f32, f32, f32),
        it: &ListItem<'_, A>,
        chosen: bool,
    ) -> f32 {
        let lines = self.list_lines(w, it);
        let h = self.list_item_height(w, it);
        if !self.shows(y, h) {
            return h;
        }
        if chosen {
            self.fill(x, y, w, h, SHAPE_L, selected_or(true, Role::Surface));
        }
        let ink = on_selected(chosen, Role::OnSurface);
        self.state_layer((x, y, w, h, SHAPE_L), ink, it.action);
        let (head, sup) = (self.line_as(BODY_L), self.line_as(BODY_M));
        let top = match lines {
            1 => y + (h - head) / 2.0,
            2 => y + (h - head - sup) / 2.0,
            _ => y + LIST_3_TOP,
        };
        let (tx, inner) = (x + LIST_PAD, w - 2.0 * LIST_PAD);
        let mut room = inner;
        if let Some((t, tone)) = it.trailing {
            let tone = if chosen { ink } else { tone };
            self.text_right_as((x + w - LIST_PAD, top, head), LABEL_S, tone, t);
            room -= self.measure_as(LABEL_S, t) + GAP;
        }
        let name = self.fit_as(BODY_L, it.headline, room);
        self.text_mid_as((tx, top, head), BODY_L, ink, &name);
        let muted = on_selected(chosen, Role::OnSurfaceVariant);
        let first = self.list_supporting(it.supporting, inner, lines);
        let (one, rest) = it.supporting.split_at(first);
        self.text_mid_as((tx, top + head, sup), BODY_M, muted, one.trim_end());
        if lines == 3 {
            let rest = self.fit_as(BODY_M, rest.trim_start(), inner);
            self.text_mid_as((tx, top + head + sup, sup), BODY_M, muted, &rest);
        }
        self.focus_ring((x, y, w, h, SHAPE_L), it.action);
        self.hit_drawn(x, y, w, h, it.action);
        h
    }

    /// Where a list item's supporting text breaks: the byte its first line
    /// ends at, all of it on one line and two lines alike.
    fn list_supporting(&self, s: &str, inner: f32, lines: usize) -> usize {
        if lines < 3 {
            return s.len();
        }
        let face = self.font(BODY_M.size, W::R).sized(self.c.scale());
        text::wrap(&face, s, self.px(inner))
            .first()
            .map_or(s.len(), |&(_, end)| end)
    }
}
