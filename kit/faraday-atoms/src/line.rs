//! Text by Material 3's type roles (09 §3.2): the pieces of [`Ui`] a screen draws with a
//! [`TypeRole`] in place of a size and a face.
//!
//! Each `_as` method is the method of the same name without it, at the role's size and in its
//! face. Two differ in what they give back: [`Ui::line_as`] is the role's line height, not the
//! face's, and [`Ui::wrap_as`] spaces its lines by it.

use faraday_draw::text;
use faraday_text::Text;
use faraday_tokens::dims::{
    BODY_M, BODY_S, BUTTON_PAD_V, FIELD_H, FIELD_PAD_V, LABEL_L, TEXT_FIT_STEP_PCT,
};
use faraday_tokens::palette::Paint;
use faraday_tokens::type_roles::{TypeFace, TypeRole};

use crate::Pressed;
use crate::field::FIELD_TEXT;
use crate::shown::Shown;
use crate::ui::{Ui, W};

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
    /// The lines times the role's line height, as [`Ui::wrap_as`] returns.
    pub h: f32,
}

impl Wrapped<'_> {
    /// How many lines it was broken into.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
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

/// One line of text in a type role and a colour.
pub struct LineModel<'s> {
    pub text: Shown<'s>,
    pub role: TypeRole,
    pub color: Paint,
}

/// Draws the line with its top-left at (x, y), design units. Returns its width.
pub fn draw<A: Pressed>(m: &LineModel<'_>, ui: &mut Ui<'_, A>, (x, y): (f32, f32)) -> f32 {
    ui.showing(m.text, |ui| {
        ui.text_as(x, y, m.role, m.color, m.text.as_str())
    })
}

impl<A: Pressed> Ui<'_, A> {
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
    pub fn fit_factor(&self, role: TypeRole, s: &str, max: f32) -> f32 {
        let role = self.stepped(role);
        self.fit_factor_of(self.measure_as(role, s), max)
    }

    /// [`Ui::fit_factor`] for a line `wide` at the text size now: the
    /// factor it is drawn at in a slot `max` wide. Arithmetic.
    pub fn fit_factor_of(&self, wide: f32, max: f32) -> f32 {
        let t = self.text_factor();
        let step = faraday_tokens::dims::percent(TEXT_FIT_STEP_PCT);
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

    /// `s` in `role` cut in the middle with an ellipsis to fit `max`,
    /// its end kept ([`Ui::fit_middle`]).
    pub fn fit_middle_as(&self, role: TypeRole, s: &str, max: f32) -> Text {
        let role = self.stepped(role);
        self.fit_middle(role.size, role.face.into(), s, max)
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
        self.laid_lines(role, s, lines)
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
    /// the same `role` and width (equal [`Ui::wrap_basis`]).
    pub fn wrapped_with<'s>(
        &self,
        role: TypeRole,
        s: &'s str,
        lines: std::rc::Rc<[(usize, usize)]>,
    ) -> Wrapped<'s> {
        let role = self.stepped(role);
        self.laid_lines(role, s, Lines::Kept(lines))
    }

    fn laid_lines<'s>(&self, role: TypeRole, s: &'s str, lines: Lines) -> Wrapped<'s> {
        let role = self.stepped(role);
        let h = lines.len() as f32 * self.line_as(role);
        Wrapped { s, role, lines, h }
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
}
