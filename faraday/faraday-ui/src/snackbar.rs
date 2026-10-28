//! Material 3's snackbar, the app's toast (`docs/THEMES.md` §7,
//! `docs/DESIGN.md` §4.11): a message over the page in the scheme's
//! inverse roles, so it reads as standing apart from the page in every
//! theme, mode and contrast. One shows at a time in Material; where two
//! must (a notice that stays while a toast comes and goes), the second
//! stands above the first ([`Ui::snackbar`] returns the room the first
//! took).

use crate::Ui;
use crate::pal::Role;
use faraday_draw::tokens::{
    ELEVATION_FAB, SHAPE_XS, TOAST_H, TOAST_MARGIN, TOAST_MAX_W, TOAST_MIN_W, TOAST_PAD,
    TOAST_PAD_Y, TOAST_STACK_GAP, TOAST_TEXT, WINDOW_MEDIUM,
};

/// The snackbar's container: inverse-surface.
pub const SNACKBAR_FILL: Role = Role::InverseSurface;
/// The snackbar's supporting text: inverse-on-surface.
pub const SNACKBAR_INK: Role = Role::InverseOnSurface;

impl<A: Copy + PartialEq> Ui<'_, A> {
    /// Costs measuring and breaking `text`. `text` as Material's
    /// snackbar in the width `cw` from `x`, its foot [`TOAST_MARGIN`]
    /// above `foot`: inverse-surface with extra-small corners at
    /// elevation level 3, its text in body medium and inverse-on-surface
    /// from its start, one line [`TOAST_H`] tall, wrapping where the width
    /// does not hold it. In a compact window (under [`WINDOW_MEDIUM`]) it
    /// is the window's width less the margins; wider, as wide as its text
    /// between [`TOAST_MIN_W`] and [`TOAST_MAX_W`], centred. How far it
    /// has faded in, its shadow with it, is `shown`, from 0 to 1, and
    /// `rise` how far below its place it still is. Returns the height it
    /// takes above `foot` with the gap above it: how far above `foot` a
    /// second one's foot goes. Faded out (`shown` 0), nothing is drawn
    /// and it takes no room: its time has run out.
    pub fn snackbar(
        &mut self,
        (x, cw, foot): (f32, f32, f32),
        text: &str,
        shown: f32,
        rise: f32,
    ) -> f32 {
        if shown <= 0.0 {
            return 0.0;
        }
        let fade = |r: Role| r.at(shown);
        let room = (cw - 2.0 * TOAST_MARGIN).max(0.0);
        let w = if cw < WINDOW_MEDIUM {
            room
        } else {
            let one = self.measure_as(TOAST_TEXT, text) + 2.0 * TOAST_PAD;
            one.clamp(TOAST_MIN_W, TOAST_MAX_W).min(room)
        };
        let line = (w - 2.0 * TOAST_PAD).max(0.0);
        let mut lines = 0usize;
        let mut from = 0;
        while let Some((_, next)) = self.snackbar_line(text, from, line) {
            lines += 1;
            from = next;
        }
        let lines = lines.max(1);
        let lh = self.line_as(TOAST_TEXT);
        let h = (lines as f32 * lh + 2.0 * TOAST_PAD_Y).max(TOAST_H);
        let tx = x + (cw - w) / 2.0;
        let ty = foot - TOAST_MARGIN - h + rise;
        self.shadow_faded((tx, ty, w, h, SHAPE_XS), ELEVATION_FAB, shown);
        self.wash(tx, ty, w, h, SHAPE_XS, fade(SNACKBAR_FILL));
        let mut y = ty + (h - lines as f32 * lh) / 2.0;
        let mut from = 0;
        while let Some((shown_line, next)) = self.snackbar_line(text, from, line) {
            let at = (tx + TOAST_PAD, y, lh);
            self.text_mid_as(at, TOAST_TEXT, fade(SNACKBAR_INK), shown_line);
            y += lh;
            from = next;
        }
        h + TOAST_STACK_GAP
    }

    /// The line of the toast's `text` that starts at byte `from`, broken
    /// at a space to fit `max` wide (a word longer than that stands
    /// alone), and the byte the next starts at; `None` past its end. A
    /// slice of `text`, so a toast drawn every frame makes no string.
    fn snackbar_line<'t>(&self, text: &'t str, from: usize, max: f32) -> Option<(&'t str, usize)> {
        let rest = &text[from..];
        let skip = rest.len() - rest.trim_start().len();
        let start = from + skip;
        let mut end = None;
        for word in text[start..].split_whitespace() {
            let stop = word.as_ptr() as usize - text.as_ptr() as usize + word.len();
            if end.is_some() && self.measure_as(TOAST_TEXT, &text[start..stop]) > max {
                break;
            }
            end = Some(stop);
        }
        end.map(|e| (&text[start..e], e))
    }
}
