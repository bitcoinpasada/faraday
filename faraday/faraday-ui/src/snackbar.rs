//! Material 3's snackbar, the app's toast (`docs/THEMES.md` §7,
//! `docs/DESIGN.md` §4.11): a message over the page in the scheme's
//! inverse roles, so it reads as standing apart from the page in every
//! theme, mode and contrast. One shows at a time in Material; where two
//! must (a notice that stays while a toast comes and goes), the second
//! stands above the first ([`Ui::snackbar`] returns the room the first
//! took).

use crate::pal::Role;
use crate::{Shape, Ui, W};
use faraday_draw::tokens::{
    ELEVATION_FAB, ICON_BUTTON, ICON_GLYPH, SHAPE_XS, TOAST_H, TOAST_MARGIN, TOAST_MAX_W,
    TOAST_MIN_W, TOAST_PAD, TOAST_PAD_Y, TOAST_STACK_GAP, TOAST_TEXT, TOUCH, WINDOW_MEDIUM,
    shape_full,
};

/// The snackbar's container: inverse-surface.
pub(crate) const SNACKBAR_FILL: Role = Role::InverseSurface;
/// The snackbar's supporting text: inverse-on-surface.
pub(crate) const SNACKBAR_INK: Role = Role::InverseOnSurface;

impl<A: Copy + PartialEq> Ui<'_, A> {
    /// Costs measuring and breaking `text`. `text` as Material's
    /// snackbar in the width `cw` from `x`, its foot [`TOAST_MARGIN`]
    /// above `foot`: inverse-surface with extra-small corners at
    /// elevation level 3, its text in body medium and inverse-on-surface,
    /// each line centred (owner, 2026-10-29; Material starts it at the
    /// start), one line [`TOAST_H`] tall, wrapping where the width does
    /// not hold it. In a compact window (under [`WINDOW_MEDIUM`]) it is
    /// the window's width less the margins; wider, as wide as its text
    /// between [`TOAST_MIN_W`] and [`TOAST_MAX_W`], centred. Where the
    /// frame's floating action button stands in its way, it stands above
    /// the button, as Material places it. How far it has faded in, its
    /// shadow with it, is `shown`, from 0 to 1, and `rise` how far below
    /// its place it still is. Returns the height it takes above `foot`
    /// with the gap above it: how far above `foot` a second one's foot
    /// goes. Faded out (`shown` 0), nothing is drawn and it takes no
    /// room: its time has run out.
    pub fn snackbar(&mut self, place: (f32, f32, f32), text: &str, shown: f32, rise: f32) -> f32 {
        self.snackbar_with(place, text, (shown, rise), None)
    }

    /// Costs measuring and breaking `text`. [`snackbar`](Self::snackbar)
    /// with Material's close affordance: an icon button at its end, a
    /// [`TOUCH`] square in the snackbar's ink, whose press does `close`.
    /// The text keeps the room left beside it. The button is drawn at
    /// the end of the call, so its press target is the topmost there.
    pub fn snackbar_closable(
        &mut self,
        place: (f32, f32, f32),
        text: &str,
        (shown, rise): (f32, f32),
        close: A,
    ) -> f32 {
        self.snackbar_with(place, text, (shown, rise), Some(close))
    }

    fn snackbar_with(
        &mut self,
        (x, cw, foot): (f32, f32, f32),
        text: &str,
        (shown, rise): (f32, f32),
        close: Option<A>,
    ) -> f32 {
        if shown <= 0.0 {
            return 0.0;
        }
        let fade = |r: Role| r.at(shown);
        let side = if close.is_some() { TOUCH } else { 0.0 };
        let (w, lines, lh, h) = self.snackbar_size(cw, text, side);
        let line = (w - 2.0 * TOAST_PAD - side).max(0.0);
        let tx = x + (cw - w) / 2.0;
        let lift = self.above_fab(tx, w, foot - TOAST_MARGIN);
        let ty = foot - TOAST_MARGIN - lift - h + rise;
        self.shadow_faded((tx, ty, w, h, SHAPE_XS), ELEVATION_FAB, shown);
        self.wash(tx, ty, w, h, SHAPE_XS, fade(SNACKBAR_FILL));
        let mut y = ty + (h - lines as f32 * lh) / 2.0;
        let mut from = 0;
        while let Some((shown_line, next)) = self.snackbar_line(text, from, line) {
            let lw = self.measure_as(TOAST_TEXT, shown_line).min(line);
            let at = (tx + TOAST_PAD + (line - lw) / 2.0, y, lh);
            self.text_mid_as(at, TOAST_TEXT, fade(SNACKBAR_INK), shown_line);
            y += lh;
            from = next;
        }
        if let Some(action) = close {
            // Material's standard icon button, centred on the end of the
            // snackbar's height in a touch target; the × is the
            // multiplication sign, as the Learn pane's close is.
            let (bx, by) = (tx + w - TOUCH, ty + (h - TOUCH) / 2.0);
            let edge = (TOUCH - ICON_BUTTON) / 2.0;
            let ink = fade(SNACKBAR_INK);
            let shape = Shape::new(
                bx + edge,
                by + edge,
                ICON_BUTTON,
                ICON_BUTTON,
                shape_full(ICON_BUTTON),
            );
            self.press_to(shape, (bx, by, TOUCH, TOUCH), ink, action);
            let mark = "\u{d7}";
            let mw = self.measure(ICON_GLYPH, W::R, mark);
            self.text_mid(
                bx + edge + (ICON_BUTTON - mw) / 2.0,
                by + edge,
                ICON_BUTTON,
                ICON_GLYPH,
                W::R,
                ink,
                mark,
            );
        }
        lift + h + TOAST_STACK_GAP
    }

    /// What a snackbar with `text` takes in a window `cw` wide: its
    /// width, its lines, a line's height and its own height. A lookup.
    fn snackbar_size(&mut self, cw: f32, text: &str, side: f32) -> (f32, usize, f32, f32) {
        let room = (cw - 2.0 * TOAST_MARGIN).max(0.0);
        let w = if cw < WINDOW_MEDIUM {
            room
        } else {
            let one = self.measure_as(TOAST_TEXT, text) + 2.0 * TOAST_PAD + side;
            one.clamp(TOAST_MIN_W, TOAST_MAX_W).min(room)
        };
        let line = (w - 2.0 * TOAST_PAD - side).max(0.0);
        let mut lines = 0usize;
        let mut from = 0;
        while let Some((_, next)) = self.snackbar_line(text, from, line) {
            lines += 1;
            from = next;
        }
        let lines = lines.max(1);
        let lh = self.line_as(TOAST_TEXT);
        let h = (lines as f32 * lh + 2.0 * TOAST_PAD_Y).max(TOAST_H);
        (w, lines, lh, h)
    }

    /// How far above its foot a snackbar with `text` puts the next one:
    /// its height and the gap, as [`snackbar`](Self::snackbar) returns
    /// them where no button lifts it. A lookup.
    pub fn snackbar_room(&mut self, cw: f32, text: &str) -> f32 {
        self.snackbar_size(cw, text, 0.0).3 + TOAST_STACK_GAP
    }

    /// As [`snackbar_room`](Self::snackbar_room) for a snackbar with its
    /// close button ([`snackbar_closable`](Self::snackbar_closable)). A
    /// lookup.
    pub fn snackbar_closable_room(&mut self, cw: f32, text: &str) -> f32 {
        self.snackbar_size(cw, text, TOUCH).3 + TOAST_STACK_GAP
    }

    /// How far a snackbar `w` wide from `tx`, its foot at `bottom`,
    /// goes up to stand [`TOAST_STACK_GAP`] above the frame's floating
    /// action button where the button is under it; 0 where it is not. A
    /// lookup.
    fn above_fab(&self, tx: f32, w: f32, bottom: f32) -> f32 {
        let Some((fx, fy, fw, _)) = self.fab_at else {
            return 0.0;
        };
        let (fx, fy) = (fx - self.ox, fy - self.oy);
        let beside = fx >= tx + w || fx + fw <= tx;
        if beside || fy >= bottom {
            return 0.0;
        }
        bottom - fy + TOAST_STACK_GAP
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
