//! Multi-choice rows and the code box (`docs/DESIGN.md` §4.2, §4.3): the
//! pieces of [`Ui`] that list rows with a checkbox and that show a value
//! whose exact characters must be plain.

use crate::pal::*;
use crate::{Ui, W};
use faraday_draw::tokens::{
    BODY_L, BODY_M, GAP, GAP_SMALL, ICON_GLYPH_BUTTON, LIST_1, LIST_2, LIST_INDENT, LIST_PAD_V,
    LIST_RESERVE, MULTI_CHECK_X, RADIUS_SMALL,
};

impl<A: Copy + PartialEq> Ui<'_, A> {
    /// A multi-choice list (`docs/DESIGN.md` §4.2, Multi choice): one
    /// full-width row per option, a checkbox at its start, any number
    /// ticked; a tap ticks or unticks its row. A row not enabled is
    /// dimmed and takes no tap. The rows are `(label, ticked, enabled,
    /// action)`. Returns the list's height; Continue goes under it.
    pub fn multi_list(&mut self, x: f32, y: f32, w: f32, rows: &[(String, bool, bool, A)]) -> f32 {
        self.multi_rows(x, y, w, rows, &[], LIST_1)
    }

    /// [`Ui::multi_list`] with a value under each row's label, as a value
    /// row has (`docs/DESIGN.md` §4.2): `values[i]` is the muted line
    /// under `rows[i]`'s label. A row with no value has none. Each row is
    /// [`LIST_2`] tall, a two-line list item, or taller where larger text
    /// needs it. Returns the list's height.
    pub fn multi_list_valued(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        rows: &[(String, bool, bool, A)],
        values: &[String],
    ) -> f32 {
        self.multi_rows(x, y, w, rows, values, LIST_2)
    }

    /// The rows of [`Ui::multi_list`] and [`Ui::multi_list_valued`], each
    /// `rh` tall.
    fn multi_rows(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        rows: &[(String, bool, bool, A)],
        values: &[String],
        rh: f32,
    ) -> f32 {
        // As tall as larger text needs (`docs/TEXT-SIZE.md` §5).
        let lines = if values.is_empty() {
            self.line_as(BODY_L)
        } else {
            self.line_as(BODY_L) + self.line_as(BODY_M)
        };
        let rh = self.row_height(rh, lines, LIST_PAD_V);
        for (i, (label, on, enabled, action)) in rows.iter().enumerate() {
            let ry = y + i as f32 * rh;
            let cy = ry + (rh - ICON_GLYPH_BUTTON) / 2.0;
            self.checkbox(x + MULTI_CHECK_X, cy, *on, *enabled);
            let shown = self.fit_as(BODY_L, label, w - LIST_RESERVE);
            let fg = if *enabled {
                Role::OnSurface.paint()
            } else {
                Role::OnSurface.at(faraday_draw::tokens::DISABLED_CONTENT)
            };
            match values.get(i) {
                Some(value) => {
                    let (lh, vh) = (self.line_as(BODY_L), self.line_as(BODY_M));
                    let top = ry + (rh - lh - vh) / 2.0;
                    self.text_mid_as((x + LIST_INDENT, top, lh), BODY_L, fg, &shown);
                    let value = self.fit_as(BODY_M, value, w - LIST_RESERVE);
                    let vc = if *enabled {
                        Role::OnSurfaceVariant.paint()
                    } else {
                        Role::OnSurface.at(faraday_draw::tokens::DISABLED_CONTENT)
                    };
                    self.text_mid_as((x + LIST_INDENT, top + lh, vh), BODY_M, vc, &value);
                }
                None => {
                    self.text_mid_as((x + LIST_INDENT, ry, rh), BODY_L, fg, &shown);
                }
            }
            if i + 1 < rows.len() {
                self.rule(
                    x + LIST_INDENT,
                    ry + rh,
                    w - LIST_INDENT,
                    Role::OutlineVariant,
                );
            }
            if *enabled {
                self.hit(x, ry, w, rh, *action);
            }
        }
        rows.len() as f32 * rh
    }

    /// Text in a code box: the exact characters of a value that must be
    /// read as they are, in the mono face on a tinted, rounded box, padded
    /// by tokens, with no quotes. `x` and `y` are the box's top left. The
    /// box is [`Ui::code_height`] tall at `size`. Returns its width.
    pub fn code_box(&mut self, x: f32, y: f32, size: f32, text: &str) -> f32 {
        let width = self.measure(size, W::M, text) + 2.0 * GAP;
        let h = self.code_height(size);
        self.fill(x, y, width, h, RADIUS_SMALL, Role::SurfaceContainerHighest);
        self.text_mid(x + GAP, y, h, size, W::M, Role::OnSurface, text);
        width
    }

    /// The height of a [`Ui::code_box`] at `size`.
    pub fn code_height(&self, size: f32) -> f32 {
        self.line(size, W::M) + 2.0 * GAP_SMALL
    }

    /// A line `lead` followed by `code` in its code box, the box on the
    /// next line where it does not fit beside the lead in `w`. The lead
    /// is `color`, at `size` in the regular face. Returns the height.
    pub fn code_line(
        &mut self,
        (x, y, w): (f32, f32, f32),
        size: f32,
        color: impl Into<Paint>,
        lead: &str,
        code: &str,
    ) -> f32 {
        let color: Paint = color.into();
        let lead_w = self.measure(size, W::R, lead);
        let box_w = self.measure(size, W::M, code) + 2.0 * GAP;
        let h = self.code_height(size);
        if lead_w + GAP + box_w <= w {
            self.text_mid(x, y, h, size, W::R, color, lead);
            self.code_box(x + lead_w + GAP, y, size, code);
            return h;
        }
        let lead_h = self.wrap(x, y, w, size, W::R, color, lead);
        self.code_box(x, y + lead_h + GAP_SMALL, size, code);
        lead_h + GAP_SMALL + h
    }
}
