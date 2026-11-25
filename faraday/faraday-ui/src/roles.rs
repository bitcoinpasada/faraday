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
use crate::{Shape, Ui, W};
use faraday_draw::text;
use faraday_draw::tokens::{
    BODY_L, BODY_M, BODY_S, BUTTON_PAD_V, DISABLED_CONTAINER, FIELD_EDGE, FIELD_EDGE_ACTIVE,
    FIELD_GAP, FIELD_H, FIELD_PAD, FIELD_PAD_V, FIELD_RADIUS, GAP, GAP_SMALL, ICON_SMALL, LABEL_L,
    LABEL_S, LIST_1, LIST_2, LIST_3, LIST_3_TOP, LIST_PAD, LIST_PAD_V, SHAPE_L, TEXT_FIT_STEP_PCT,
    TypeFace, TypeRole,
};
use faraday_draw::widgets::Icon;
use faraday_text::Text;

impl From<TypeFace> for W {
    fn from(face: TypeFace) -> W {
        match face {
            TypeFace::Regular => W::R,
            TypeFace::Medium => W::S,
            TypeFace::Mono => W::M,
        }
    }
}

/// Text broken into lines and not yet drawn ([`Ui::wrapped_as`]): what
/// it is, its role, where each line starts and ends in it, and the
/// height its lines take.
pub struct Wrapped<'s> {
    s: &'s str,
    role: TypeRole,
    lines: Lines,
    /// The width it was broken at.
    width: f32,
    /// The lines times the role's line height, as [`Ui::wrap_as`] returns.
    pub h: f32,
}

/// The lines of a [`Wrapped`]: broken for it, or kept by the caller.
enum Lines {
    Own(Vec<(usize, usize)>),
    Kept(std::rc::Rc<[(usize, usize)]>),
}

impl std::ops::Deref for Lines {
    type Target = [(usize, usize)];
    fn deref(&self) -> &[(usize, usize)] {
        match self {
            Lines::Own(v) => v,
            Lines::Kept(r) => r,
        }
    }
}

/// Everything a text is broken into lines from besides the text itself:
/// the font, the scale it is drawn at and the width in pixels. Two equal
/// bases break a text into the same lines ([`Ui::wrap_basis`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WrapBasis(faraday_draw::text::Font, faraday_draw::Scale, i32);

/// A text field's label kept outside it, above (`docs/LAYOUT.md` §8):
/// Material's body small.
pub(crate) const FIELD_LABEL: TypeRole = BODY_S;

/// What is typed in a text field: Material's body large.
pub(crate) const FIELD_TEXT: TypeRole = BODY_L;

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
    /// `role` as it is drawn here: Body small, a note, is Body medium
    /// where [`Ui::note_up`] says so, in the same face. A lookup.
    pub fn stepped(&self, role: TypeRole) -> TypeRole {
        if self.note_up
            && role.size == BODY_S.size
            && role.line == BODY_S.line
            && role.face != TypeFace::Medium
        {
            TypeRole {
                size: BODY_M.size,
                line: BODY_M.line,
                face: role.face,
            }
        } else {
            role
        }
    }

    /// The height of one line in `role`, in design units: its line height
    /// at this canvas's type size, times the text size.
    pub fn line_as(&self, role: TypeRole) -> f32 {
        let role = self.stepped(role);
        role.line * self.text_factor() * self.c.scale().factor() / self.f
    }

    /// What a line in `role` adds at the text size now over its height at
    /// Standard: nothing at Standard and inside [`Ui::fixed_text`].
    /// Arithmetic.
    pub fn line_grown_as(&self, role: TypeRole) -> f32 {
        let line = self.line_as(role);
        line - line / self.text_factor()
    }

    /// A line's advance laid out at Standard as `adv` after a line in
    /// `role`, grown by what that line grows at the text size now
    /// (`docs/TEXT-SIZE.md` §5): `adv` at Standard. Arithmetic.
    pub fn advance_as(&self, role: TypeRole, adv: f32) -> f32 {
        adv + self.line_grown_as(role)
    }

    /// [`Ui::advance_as`] after a line of `size` in face `w`, as
    /// [`Ui::line`] measures it. A lookup.
    pub fn advance(&self, size: f32, w: W, adv: f32) -> f32 {
        let line = self.line(size, w);
        adv + line - line / self.text_factor()
    }

    /// A height that text sets (`docs/TEXT-SIZE.md` §5): at Standard,
    /// and inside [`Ui::fixed_text`], `token` as it is; at a larger text
    /// size, `token` or `lines` (the height its text takes now) with
    /// `pad` above and below, whichever is larger. Arithmetic.
    pub fn row_height(&self, token: f32, lines: f32, pad: f32) -> f32 {
        if self.text_factor() <= 1.0 {
            return token;
        }
        token.max(lines + 2.0 * pad)
    }

    /// The height a button asked to be `h` tall is drawn: `h`, or its
    /// label's line and Material's content padding where larger text
    /// needs more. Arithmetic.
    pub fn button_height(&self, h: f32) -> f32 {
        self.row_height(h, self.line_as(LABEL_L), BUTTON_PAD_V)
    }

    /// A text field's height: [`FIELD_H`], or its line and Material's
    /// padding where larger text needs more. Arithmetic.
    pub fn field_h(&self) -> f32 {
        self.row_height(FIELD_H, self.line_as(FIELD_TEXT), FIELD_PAD_V)
    }

    /// The side of a row's leading icon asked to be `side`: it grows with
    /// the row's label, by the text size (`docs/TEXT-SIZE.md` §3).
    /// Arithmetic.
    pub fn row_icon(&self, side: f32) -> f32 {
        side * self.text_factor()
    }

    /// The factor `s` in `role` is drawn at in a one-line slot `max`
    /// wide (`docs/TEXT-SIZE.md` §3): the text size's own, or the largest
    /// step of [`TEXT_FIT_STEP_PCT`] under it that fits, never under
    /// Standard. Measures `s` once.
    pub(crate) fn fit_factor(&self, role: TypeRole, s: &str, max: f32) -> f32 {
        let role = self.stepped(role);
        self.fit_factor_of(self.measure_as(role, s), max)
    }

    /// [`Ui::fit_factor`] for a line `wide` at the text size now: the
    /// factor it is drawn at in a slot `max` wide. Arithmetic.
    pub(crate) fn fit_factor_of(&self, wide: f32, max: f32) -> f32 {
        let t = self.text_factor();
        let step = faraday_draw::tokens::percent(TEXT_FIT_STEP_PCT);
        // A line's width goes with its size, so one measure says where
        // each step would end.
        let wide = wide / t;
        let mut k = t;
        while k > 1.0 && wide * k > max {
            k = (k - step).max(1.0);
        }
        k
    }

    /// `s` in `role` in a one-line slot from x, `max` wide, centred in a
    /// band `h` tall from y: at the text size where it fits, else fitted
    /// down to Standard ([`Ui::fit_factor`]) and cut there with an
    /// ellipsis. Returns its width.
    pub fn text_fit_as(
        &mut self,
        (x, y, h, max): (f32, f32, f32, f32),
        role: TypeRole,
        color: impl Into<Paint>,
        s: &str,
    ) -> f32 {
        let role = self.stepped(role);
        let color: Paint = color.into();
        let k = self.fit_factor(role, s, max);
        let sized = TypeRole {
            size: role.size * k,
            line: role.line * k,
            ..role
        };
        self.fixed_text(|ui| {
            // No string is made for a line that fits.
            if ui.measure_as(sized, s) <= max {
                return ui.text_mid_as((x, y, h), sized, color, s);
            }
            let shown = ui.fit_as(sized, s, max);
            ui.text_mid_as((x, y, h), sized, color, &shown)
        })
    }

    /// How wide [`Ui::text_fit_as`] draws `s` in `role` in a slot `max`
    /// wide. Makes the cut string only where it does not fit.
    pub fn text_fit_width(&self, role: TypeRole, s: &str, max: f32) -> f32 {
        let role = self.stepped(role);
        let k = self.fit_factor(role, s, max);
        let t = self.text_factor();
        let wide = self.measure_as(role, s) * k / t;
        if wide <= max {
            return wide;
        }
        // Cut at Standard, as `text_fit_as` cuts it.
        let sized = TypeRole {
            size: role.size * k / t,
            line: role.line * k / t,
            ..role
        };
        self.measure_as(sized, &self.fit_as(sized, s, max))
    }

    /// The width of `s` in `role`.
    pub fn measure_as(&self, role: TypeRole, s: &str) -> f32 {
        let role = self.stepped(role);
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
        let role = self.stepped(role);
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
        let role = self.stepped(role);
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
        let role = self.stepped(role);
        self.text_right(right, y, h, role.size, role.face.into(), color, s);
    }

    /// `s` in `role` cut with an ellipsis to fit `max`.
    pub fn fit_as(&self, role: TypeRole, s: &str, max: f32) -> Text {
        let role = self.stepped(role);
        self.fit(role.size, role.face.into(), s, max)
    }

    /// How many lines [`Ui::wrap_as`] breaks `s` onto at `max` width:
    /// one at least.
    pub fn wrap_lines_as(&self, role: TypeRole, s: &str, max: f32) -> usize {
        let role = self.stepped(role);
        let face = self.face(role.size, role.face.into());
        text::wrap(&face, s, self.px(max)).len().max(1)
    }

    /// The height [`Ui::wrap_as`] takes for `s` at `width`, drawn
    /// nowhere.
    pub fn wrap_height_as(&self, role: TypeRole, s: &str, width: f32) -> f32 {
        let role = self.stepped(role);
        let face = self.face(role.size, role.face.into());
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
        let role = self.stepped(role);
        let laid = self.wrapped_as(role, s, width);
        self.draw_wrapped((x, y), color, &laid);
        laid.h
    }

    /// [`Ui::wrap_as`] drawing only the lines that can be seen
    /// ([`Ui::shows`]): a long text mostly scrolled out of its view draws
    /// the lines in view, and counts the rest by height alone. Returns the
    /// height used, the same.
    pub fn wrap_seen(
        &mut self,
        (x, y, width): (f32, f32, f32),
        role: TypeRole,
        color: impl Into<Paint>,
        s: &str,
    ) -> f32 {
        let laid = self.wrapped_as(role, s, width);
        self.draw_wrapped_seen((x, y), color, &laid)
    }

    /// [`Ui::draw_wrapped`] drawing only the lines that can be seen
    /// ([`Ui::shows`]), as [`Ui::wrap_seen`] does. Returns the height
    /// used.
    pub fn draw_wrapped_seen(
        &mut self,
        (x, y): (f32, f32),
        color: impl Into<Paint>,
        laid: &Wrapped<'_>,
    ) -> f32 {
        if !self.shows(y, laid.h) {
            return laid.h;
        }
        let color = self.ink(color.into());
        let role = laid.role;
        let face = self.face(role.size, role.face.into());
        let line = self.line_as(role);
        let x0 = self.px(x + self.ox);
        for (i, &(start, end)) in laid.lines.iter().enumerate() {
            let top = y + i as f32 * line;
            if self.shows(top, line) {
                let ly = self.px(top + self.oy) + self.oy_px;
                self.c
                    .text_with_face(&face, x0, ly, &laid.s[start..end], color);
            }
        }
        laid.h
    }

    /// `s` in `role` broken into lines at `width` as [`Ui::wrap_as`]
    /// breaks it, drawn nowhere: its height known before what goes under
    /// it is drawn (a row's press), then drawn with [`Ui::draw_wrapped`]
    /// in the same frame, so it is broken once.
    pub fn wrapped_as<'s>(&self, role: TypeRole, s: &'s str, width: f32) -> Wrapped<'s> {
        let role = self.stepped(role);
        let face = self.face(role.size, role.face.into());
        let lines = Lines::Own(text::wrap(&face, s, self.px(width)));
        self.laid(role, s, width, lines)
    }

    /// What [`Ui::wrapped_as`] breaks a text into lines from, besides the
    /// text: a caller that keeps the lines of a text between frames keeps
    /// them with this and uses them while it is equal.
    pub fn wrap_basis(&self, role: TypeRole, width: f32) -> WrapBasis {
        let role = self.stepped(role);
        WrapBasis(
            self.font(role.size, role.face.into()),
            self.c.scale(),
            self.px(width),
        )
    }

    /// The lines [`Ui::wrapped_as`] breaks `s` into at `width`.
    pub fn wrap_ranges(
        &self,
        role: TypeRole,
        s: &str,
        width: f32,
    ) -> std::rc::Rc<[(usize, usize)]> {
        let role = self.stepped(role);
        let face = self.face(role.size, role.face.into());
        text::wrap(&face, s, self.px(width)).into()
    }

    /// `s` laid out in `lines`, which [`Ui::wrap_ranges`] gave for `s` at
    /// the same `role` and `width` (equal [`Ui::wrap_basis`]).
    pub fn wrapped_with<'s>(
        &self,
        role: TypeRole,
        s: &'s str,
        width: f32,
        lines: std::rc::Rc<[(usize, usize)]>,
    ) -> Wrapped<'s> {
        let role = self.stepped(role);
        self.laid(role, s, width, Lines::Kept(lines))
    }

    fn laid<'s>(&self, role: TypeRole, s: &'s str, width: f32, lines: Lines) -> Wrapped<'s> {
        let role = self.stepped(role);
        let h = lines.len() as f32 * self.line_as(role);
        Wrapped {
            s,
            role,
            lines,
            width,
            h,
        }
    }

    /// `laid` drawn in `color` with its first line's top at (x, y), as
    /// [`Ui::wrap_as`] draws it.
    pub fn draw_wrapped(
        &mut self,
        (x, y): (f32, f32),
        color: impl Into<Paint>,
        laid: &Wrapped<'_>,
    ) {
        let color = self.ink(color.into());
        let role = laid.role;
        let face = self.face(role.size, role.face.into());
        let line = self.line_as(role);
        let x0 = self.px(x + self.ox);
        for (i, &(start, end)) in laid.lines.iter().enumerate() {
            let ly = self.px(y + self.oy + i as f32 * line) + self.oy_px;
            self.c
                .text_with_face(&face, x0, ly, &laid.s[start..end], color);
        }
    }

    /// Several lines of typing (`docs/TEXT-CURSOR.md` §3): `text`, dots
    /// already for a masked field, in `role` wrapped to `width` as
    /// [`Ui::wrap_as`] wraps it, from (x, y). While `focused`, what is
    /// selected is drawn behind it and the caret at its place. Where
    /// each line's characters start is kept for `action` with the line's
    /// rows, so a press there places the caret and Up and Down find the
    /// line above and below. Returns the height used.
    pub fn typed_lines(
        &mut self,
        (x, y, width): (f32, f32, f32),
        role: TypeRole,
        color: impl Into<Paint>,
        text: &str,
        typing: (bool, A),
    ) -> f32 {
        let role = self.stepped(role);
        let laid = self.wrapped_as(role, text, width);
        self.draw_typed((x, y), color, &laid, typing);
        laid.h
    }

    /// `laid`, broken once by [`Ui::wrapped_as`], drawn as several lines
    /// of typing from (x, y), as [`Ui::typed_lines`] draws it: what is
    /// selected, the caret while `focused`, and each line's edges kept
    /// for `action`.
    pub fn draw_typed(
        &mut self,
        (x, y): (f32, f32),
        color: impl Into<Paint>,
        laid: &Wrapped<'_>,
        (focused, action): (bool, A),
    ) {
        let (text, role, width) = (laid.s, laid.role, laid.width);
        let lines: &[(usize, usize)] = &laid.lines;
        let ink = self.ink(color.into());
        let selected_ink = self.ink(crate::SELECTION_INK.paint());
        let face = self.face(role.size, role.face.into());
        let line = self.line_as(role);
        let edit = if focused {
            self.edit
        } else {
            crate::Edit::default()
        };
        let n = text.chars().count();
        let caret = edit.caret_in(n);
        let selected = edit.selected(n);
        let x0 = self.px(x + self.ox);
        // The character index each line starts at, counted once.
        let mut index = 0usize;
        let mut byte = 0usize;
        let mut caret_at: Option<(f32, f32)> = None;
        for (i, &(start, end)) in lines.iter().enumerate() {
            index += text[byte..start].chars().count();
            let ly = y + i as f32 * line;
            // Each edge's column, by the characters' advances.
            let mut pen = 0u32;
            let mut cols = vec![0.0f32];
            for c in text[start..end].chars() {
                pen += face.advance_64(c);
                cols.push(text::px_from_64(pen) as f32 / self.f);
            }
            let count = cols.len() - 1;
            if let Some((lo, hi)) = selected {
                let (a, b) = (
                    lo.clamp(index, index + count),
                    hi.clamp(index, index + count),
                );
                if b > a {
                    let (l, r) = (cols[a - index], cols[b - index]);
                    self.selection_band(x + l, ly, r - l, line);
                }
            }
            let py = self.px(y + self.oy + i as f32 * line) + self.oy_px;
            self.c.text_with_face(&face, x0, py, &text[start..end], ink);
            if let Some((lo, hi)) = selected {
                let (a, b) = (
                    lo.clamp(index, index + count),
                    hi.clamp(index, index + count),
                );
                if b > a {
                    let (l, r) = (cols[a - index], cols[b - index]);
                    let clip = self.rect(x + l, ly, r - l, line);
                    self.c.push_clip(clip);
                    self.c
                        .text_with_face(&face, x0, py, &text[start..end], selected_ink);
                    self.c.pop_clip();
                }
            }
            // The caret stands on the line it falls in, or at the start
            // of the next where a line ends at a break.
            let last = i + 1 == lines.len();
            if caret >= index && (caret < index + count || (caret == index + count && last)) {
                caret_at = Some((x + cols[caret - index], ly));
            } else if caret_at.is_none() && caret >= index + count {
                // After the line, in the spaces a break leaves out: at
                // its end, and as far on as the spaces go.
                let gap: u32 = text[end..]
                    .chars()
                    .take(caret - index - count)
                    .map(|c| face.advance_64(c))
                    .sum();
                let past = (text::px_from_64(pen + gap) as f32 / self.f).min(width);
                caret_at = Some((x + past, ly));
            }
            let band = (
                py,
                self.px(y + self.oy + (i + 1) as f32 * line) + self.oy_px,
            );
            let edges = cols.iter().map(|c| self.px(x + c + self.ox)).collect();
            self.fields.push(crate::FieldAt {
                action,
                first: index,
                edges,
                band: Some(band),
            });
            index += count;
            byte = end;
        }
        if focused {
            // The line the caret stands on is what is kept in view above
            // the keyboard, not the whole text: a text taller than the
            // space above the keyboard shows the line being typed.
            let (cx, cy) = caret_at.unwrap_or((x, y));
            self.note_focus(x, cy, width, line);
            self.focus_action = Some(action);
            if selected.is_none() {
                let h = role.size * self.text_factor();
                self.caret(cx, cy + (line - h) / 2.0, h);
            }
        }
    }

    /// The lines a typed address is shown in at `room` wide, as ranges of
    /// its characters: whole groups of four, as many to a line as fit.
    fn address_lines(&self, role: TypeRole, text: &str, room: f32) -> Vec<(usize, usize)> {
        let face = self.face(role.size, role.face.into());
        // In 64ths of a pixel, as the text is laid out.
        let adv = |c: char| face.advance_64(c);
        let space = adv(' ');
        let room = ((room * self.f).max(0.0) as u32) << 6;
        let chars: Vec<char> = text.chars().collect();
        let mut lines = Vec::new();
        let (mut start, mut used) = (0usize, 0u32);
        let mut g = 0;
        while g < chars.len() {
            let end = (g + 4).min(chars.len());
            let w: u32 = chars[g..end].iter().map(|&c| adv(c)).sum();
            let with = if g == start { w } else { used + space + w };
            if g > start && with > room {
                lines.push((start, g));
                start = g;
                used = w;
            } else {
                used = with;
            }
            g = end;
        }
        if start < chars.len() || lines.is_empty() {
            lines.push((start, chars.len()));
        }
        lines
    }

    /// A typed address as a text field that grows to the lines it needs
    /// (`docs/TEXT-CURSOR.md` §3): its box at (x, y), `w` wide, the text
    /// in groups of four with a space drawn between them, never in the
    /// text, so the caret, the selection and a press count its
    /// characters. While `focused` the box is in primary and the caret
    /// and what is selected are drawn. Returns the height with the
    /// error line under it.
    pub fn typed_address(
        &mut self,
        (x, y, w): (f32, f32, f32),
        text: &str,
        error: Option<&str>,
        (focused, action): (bool, A),
    ) -> f32 {
        let role = self.stepped(BODY_L.mono());
        let room = w - 2.0 * FIELD_PAD;
        let lines = self.address_lines(role, text, room);
        let line = self.line_as(role);
        let min = self.field_h();
        let pad = ((min - line) / 2.0).max(0.0);
        let fh = (lines.len() as f32 * line + 2.0 * pad).max(min);
        let used = self.field_box_sized((x, y, w), fh, (focused, false), error);
        self.press_box(x, y, w, fh, action);
        let face = self.face(role.size, role.face.into());
        let ink = self.ink(Role::OnSurface.paint());
        let selected_ink = self.ink(crate::SELECTION_INK.paint());
        let edit = if focused {
            self.edit
        } else {
            crate::Edit::default()
        };
        let chars: Vec<char> = text.chars().collect();
        let n = chars.len();
        let caret = edit.caret_in(n);
        let selected = edit.selected(n);
        let space = face.advance_64(' ');
        let (tx, ty) = (x + FIELD_PAD, y + pad);
        let x0 = self.px(tx + self.ox);
        let mut caret_at: Option<(f32, f32)> = None;
        for (i, &(a0, a1)) in lines.iter().enumerate() {
            let ly = ty + i as f32 * line;
            // Where each character starts and ends in the grouped line.
            let (mut starts, mut cols) = (Vec::new(), vec![0.0f32]);
            let count = a1 - a0;
            let mut shown = faraday_text::Text::with_room(count + count / 4);
            let mut pen = 0u32;
            let px = |v: u32| text::px_from_64(v) as f32 / self.f;
            for (j, &c) in chars[a0..a1].iter().enumerate() {
                if j > 0 && j % 4 == 0 {
                    pen += space;
                    shown.push(' ');
                }
                starts.push(px(pen));
                pen += face.advance_64(c);
                cols.push(px(pen));
                shown.push(c);
            }
            let py = self.px(ly + self.oy) + self.oy_px;
            let band = selected.and_then(|(lo, hi)| {
                let (lo, hi) = (lo.clamp(a0, a1), hi.clamp(a0, a1));
                (hi > lo).then(|| (starts[lo - a0], cols[hi - a0]))
            });
            if let Some((l, r)) = band {
                self.selection_band(tx + l, ly, r - l, line);
            }
            self.c.text_with_face(&face, x0, py, shown.as_str(), ink);
            if let Some((l, r)) = band {
                let clip = self.rect(tx + l, ly, r - l, line);
                self.c.push_clip(clip);
                self.c
                    .text_with_face(&face, x0, py, shown.as_str(), selected_ink);
                self.c.pop_clip();
            }
            let last = i + 1 == lines.len();
            if caret >= a0 && (caret < a1 || (caret == a1 && last)) {
                caret_at = Some((tx + cols[caret - a0], ly));
            }
            let band_px = (py, self.px(ly + line + self.oy) + self.oy_px);
            let edges = cols.iter().map(|c| self.px(tx + c + self.ox)).collect();
            self.fields.push(crate::FieldAt {
                action,
                first: a0,
                edges,
                band: Some(band_px),
            });
            debug_assert_eq!(cols.len(), count + 1);
        }
        if focused {
            let (cx, cy) = caret_at.unwrap_or((tx, ty));
            self.note_focus(tx, cy, room, line);
            self.focus_action = Some(action);
            if selected.is_none() {
                let h = role.size * self.text_factor();
                self.caret(cx + 1.0, cy + (line - h) / 2.0, h);
            }
        }
        used
    }

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
    pub(crate) fn field_height(&self, w: f32, error: Option<&str>) -> f32 {
        let fh = self.field_h();
        match error {
            Some(e) => fh + FIELD_GAP + self.wrap_height_as(BODY_S, e, w - 2.0 * FIELD_PAD),
            None => fh,
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

    /// The room `it`'s headline has `w` wide: the item's inside, less
    /// its trailing text and the gap before it.
    fn list_head_room(&self, w: f32, it: &ListItem<'_, A>) -> f32 {
        let inner = w - 2.0 * LIST_PAD;
        match it.trailing {
            Some((t, _)) => inner - self.measure_as(LABEL_S, t) - GAP,
            None => inner,
        }
    }

    /// The height of one headline line: the mono face's where the headline is
    /// a whole address.
    fn head_line(&self, whole: bool) -> f32 {
        if whole {
            self.line_as(BODY_L.mono())
        } else {
            self.line_as(BODY_L)
        }
    }

    /// How many lines `it`'s headline takes `w` wide: one at Standard,
    /// where it is cut to fit; at a larger text size two where it does not
    /// fit on one (`docs/TEXT-SIZE.md` §3), the second cut to fit.
    fn list_head_lines(&self, w: f32, it: &ListItem<'_, A>, whole: bool) -> usize {
        if whole {
            let room = self.list_head_room(w, it);
            return (self.wrap_height_as(BODY_L.mono(), it.headline, room) / self.head_line(true))
                .round()
                .max(1.0) as usize;
        }
        if self.text_factor() <= 1.0
            || self.measure_as(BODY_L, it.headline) <= self.list_head_room(w, it)
        {
            1
        } else {
            2
        }
    }

    /// The height [`Ui::list_item`] takes for `it`, `w` wide:
    /// [`LIST_1`], [`LIST_2`] or [`LIST_3`], or its lines and Material's
    /// padding where larger text needs more.
    pub(crate) fn list_item_height(&self, w: f32, it: &ListItem<'_, A>, whole: bool) -> f32 {
        let (head, sup) = (self.head_line(whole), self.line_as(BODY_M));
        // A headline larger text breaks onto a second line adds it.
        let head = head * self.list_head_lines(w, it, whole) as f32;
        let (token, lines, pad) = match self.list_lines(w, it) {
            1 => (LIST_1, head, LIST_PAD_V),
            2 => (LIST_2, head + sup, LIST_PAD_V),
            _ => (LIST_3, head + 2.0 * sup, LIST_3_TOP),
        };
        // A whole address takes its lines at every text size.
        if whole {
            return token.max(lines + 2.0 * pad);
        }
        self.row_height(token, lines, pad)
    }

    /// A list item (Material 3 Lists) from (x, y), `w` wide, of one, two
    /// or three lines by its supporting text (the third line cut with an
    /// ellipsis); `chosen` in Material's selected state (secondary
    /// container). The whole item takes its press. Returns its height.
    pub fn list_item(&mut self, at: (f32, f32, f32), it: &ListItem<'_, A>, chosen: bool) -> f32 {
        self.list_item_as(at, it, chosen, true, false, false)
    }

    /// A list item as [`Ui::list_item`] draws it whose headline is an
    /// address already in groups of four ([`crate::grouped`]): in the mono
    /// face, never cut, on one line where it fits and wrapped between
    /// groups where the room requires it; the item grows by the lines.
    pub fn list_item_whole(
        &mut self,
        at: (f32, f32, f32),
        it: &ListItem<'_, A>,
        chosen: bool,
    ) -> f32 {
        self.list_item_as(at, it, chosen, true, true, false)
    }

    /// [`Ui::list_item_whole`] that takes no press.
    pub fn list_item_whole_still(&mut self, at: (f32, f32, f32), it: &ListItem<'_, A>) -> f32 {
        self.list_item_as(at, it, false, false, true, false)
    }

    /// A list item as [`Ui::list_item`] draws it with a chevron at its end,
    /// which says the item opens a page; its trailing text stands before
    /// the chevron. Returns its height.
    pub fn list_item_opens(&mut self, at: (f32, f32, f32), it: &ListItem<'_, A>) -> f32 {
        self.list_item_as(at, it, false, true, false, true)
    }

    /// A list item as [`Ui::list_item`] draws it that takes no press: a
    /// row that only states what is there (Material 3 Lists, a
    /// non-interactive item). Its `action` is not used. Returns its height.
    pub fn list_item_still(&mut self, at: (f32, f32, f32), it: &ListItem<'_, A>) -> f32 {
        self.list_item_as(at, it, false, false, false, false)
    }

    fn list_item_as(
        &mut self,
        (x, y, w): (f32, f32, f32),
        it: &ListItem<'_, A>,
        chosen: bool,
        pressed: bool,
        whole: bool,
        chevron: bool,
    ) -> f32 {
        // The chevron takes room at the item's end: the text lays out in
        // what it leaves, the press covers all.
        let full = w;
        let w = if chevron {
            w - ICON_SMALL - GAP_SMALL
        } else {
            w
        };
        let lines = self.list_lines(w, it);
        let h = self.list_item_height(w, it, whole);
        if !self.shows(y, h) {
            return h;
        }
        if chosen {
            self.fill(x, y, w, h, SHAPE_L, selected_or(true, Role::Surface));
        }
        let ink = on_selected(chosen, Role::OnSurface);
        if pressed {
            self.press(Shape::new(x, y, full, h, SHAPE_L), ink, it.action);
        }
        let (line, sup) = (self.head_line(whole), self.line_as(BODY_M));
        let head_lines = self.list_head_lines(w, it, whole);
        let head = line * head_lines as f32;
        let top = match lines {
            1 => y + (h - head) / 2.0,
            2 => y + (h - head - sup) / 2.0,
            _ => y + LIST_3_TOP,
        };
        let (tx, inner) = (x + LIST_PAD, w - 2.0 * LIST_PAD);
        let room = self.list_head_room(w, it);
        if let Some((t, tone)) = it.trailing {
            let tone = if chosen { ink } else { tone };
            self.text_right_as((x + w - LIST_PAD, top, line), LABEL_S, tone, t);
        }
        if whole {
            self.wrap_as((tx, top, room), BODY_L.mono(), ink, it.headline);
        } else if head_lines == 1 {
            let name = self.fit_as(BODY_L, it.headline, room);
            self.text_mid_as((tx, top, line), BODY_L, ink, &name);
        } else {
            let end = self.first_line_end(BODY_L, it.headline, room);
            let (one, rest) = it.headline.split_at(end);
            self.text_mid_as((tx, top, line), BODY_L, ink, one.trim_end());
            let rest = self.fit_as(BODY_L, rest.trim_start(), room);
            self.text_mid_as((tx, top + line, line), BODY_L, ink, &rest);
        }
        let muted = on_selected(chosen, Role::OnSurfaceVariant);
        let first = if lines < 3 {
            it.supporting.len()
        } else {
            self.first_line_end(BODY_M, it.supporting, inner)
        };
        let (one, rest) = it.supporting.split_at(first);
        self.text_mid_as((tx, top + head, sup), BODY_M, muted, one.trim_end());
        if lines == 3 {
            let rest = self.fit_as(BODY_M, rest.trim_start(), inner);
            self.text_mid_as((tx, top + head + sup, sup), BODY_M, muted, &rest);
        }
        if chevron {
            let size = ICON_SMALL;
            let at = (x + full - LIST_PAD - size, y + (h - size) / 2.0);
            self.icon(at.0, at.1, size, Icon::ChevronRight, size, muted);
        }
        h
    }

    /// Where `s` in `role` wrapped `max` wide breaks: the byte its first
    /// line ends at.
    fn first_line_end(&self, role: TypeRole, s: &str, max: f32) -> usize {
        let role = self.stepped(role);
        let face = self.face(role.size, role.face.into());
        text::wrap(&face, s, self.px(max))
            .first()
            .map_or(s.len(), |&(_, end)| end)
    }
}
