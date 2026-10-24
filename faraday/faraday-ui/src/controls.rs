//! Material 3's control pieces (`docs/DESIGN.md` §4.13, §4.15, F108):
//! touch targets of at least 48 dp, state layers, the focus ring, the
//! segmented button and the progress indicator.

use crate::pal::*;
use crate::{Theme, Ui, W};
use faraday_draw::tokens::{
    FOCUS_OFFSET, FOCUS_RING, GAP, ICON_GAP, ICON_GLYPH_BUTTON, PROGRESS_H, PROGRESS_SPAN,
    SEGMENT_PAD, STATE_FOCUS, STATE_HOVER, STATE_PRESS, TOUCH, alpha_of, shape_full,
};
use faraday_draw::widgets::Icon;
use faraday_draw::{Color, Rect};

/// Where a press drawn this frame is taken, in pixels: `drawn`, the
/// control as drawn (after the clip), and `target`, the same centred
/// and grown to the touch floor, [`TOUCH`] square, where it is drawn
/// smaller. Two targets that would overlap share the gap between their
/// controls, each up to the middle, so a target never covers another's
/// control nor reaches past the middle of the gap. A press on a drawn
/// control takes it first; a press beside every control takes the
/// target it falls in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reach<A> {
    /// Where the press is taken.
    pub target: Rect,
    /// The control as drawn.
    pub drawn: Rect,
    /// What the press does.
    pub action: A,
}

/// Where the keyboard moves among `order`, the controls in the order a
/// frame drew them, for Tab (`back` false) or Shift+Tab from `at`: the
/// next or the one before, round at the ends; the first or the last
/// from none.
pub fn focus_step<A: Copy + PartialEq>(order: &[A], at: Option<A>, back: bool) -> Option<A> {
    let mut seen: Vec<A> = Vec::with_capacity(order.len());
    for a in order {
        if !seen.contains(a) {
            seen.push(*a);
        }
    }
    let n = seen.len();
    if n == 0 {
        return None;
    }
    let i = at.and_then(|c| seen.iter().position(|d| *d == c));
    let next = match (back, i) {
        (false, Some(i)) => (i + 1) % n,
        (false, None) => 0,
        (true, Some(i)) => (i + n - 1) % n,
        (true, None) => n - 1,
    };
    Some(seen[next])
}

/// The press `reach` takes at pixel (x, y): a target it falls in, the
/// last drawn first. A control drawn there is looked for before this.
pub fn reach_at<A: Copy>(reach: &[Reach<A>], x: i32, y: i32) -> Option<(A, Rect)> {
    reach
        .iter()
        .rev()
        .find(|r| r.target.contains(x, y))
        .map(|r| (r.action, r.target))
}

/// Targets `a` and `b`, of controls drawn at `da` and `db`, each cut
/// back to the middle of the gap between the controls, across the axis
/// they are furthest apart on; as they were when the controls overlap.
fn split(a: Rect, b: Rect, da: Rect, db: Rect) -> (Rect, Rect) {
    let (mut a, mut b) = (a, b);
    let gap_x = (db.x - da.right()).max(da.x - db.right());
    let gap_y = (db.y - da.bottom()).max(da.y - db.bottom());
    if gap_x < 0 && gap_y < 0 {
        return (a, b);
    }
    if gap_x >= gap_y {
        if da.right() <= db.x {
            let mid = (da.right() + db.x) / 2;
            a.w = a.w.min(mid - a.x);
            let r = b.right();
            b.x = b.x.max(mid);
            b.w = r - b.x;
        } else {
            let mid = (db.right() + da.x) / 2;
            b.w = b.w.min(mid - b.x);
            let r = a.right();
            a.x = a.x.max(mid);
            a.w = r - a.x;
        }
    } else if da.bottom() <= db.y {
        let mid = (da.bottom() + db.y) / 2;
        a.h = a.h.min(mid - a.y);
        let bt = b.bottom();
        b.y = b.y.max(mid);
        b.h = bt - b.y;
    } else {
        let mid = (db.bottom() + da.y) / 2;
        b.h = b.h.min(mid - b.y);
        let at = a.bottom();
        a.y = a.y.max(mid);
        a.h = at - a.y;
    }
    (a, b)
}

/// How a button looks: Material's filled, tonal, outlined and text
/// buttons, a chosen option, and each kind not available now, drawn in
/// its own shape dimmed (`docs/DESIGN.md` §4.13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// The one action a screen leads with: filled in the accent.
    Primary,
    /// A second action that matters: filled in `SELECTED`, no edge.
    Tonal,
    /// Any other action: outlined in `OUTLINE`.
    Secondary,
    /// A quiet action beside others: a text button, its label in the
    /// accent.
    Ghost,
    /// The chosen one of a set of options: `SELECTED`, with a tick.
    Selected,
    /// An outlined button shown but not available now.
    Disabled,
    /// A filled button not available now: its fill kept, faint.
    PrimaryOff,
    /// A tonal button not available now.
    TonalOff,
    /// A text button not available now.
    GhostOff,
}

impl Style {
    /// This kind not available now, in its own shape.
    pub const fn off(self) -> Style {
        match self {
            Style::Primary | Style::PrimaryOff => Style::PrimaryOff,
            Style::Tonal | Style::TonalOff => Style::TonalOff,
            Style::Ghost | Style::GhostOff => Style::GhostOff,
            Style::Secondary | Style::Selected | Style::Disabled => Style::Disabled,
        }
    }

    /// This kind, or [`Style::off`] when `on` is false.
    pub const fn when(self, on: bool) -> Style {
        if on { self } else { self.off() }
    }

    /// An option of a set: [`Style::Selected`] when it is the one
    /// chosen, an outlined button when not.
    pub const fn chosen(on: bool) -> Style {
        if on {
            Style::Selected
        } else {
            Style::Secondary
        }
    }

    /// Whether it is shown but not available now.
    pub const fn is_off(self) -> bool {
        matches!(
            self,
            Style::Disabled | Style::PrimaryOff | Style::TonalOff | Style::GhostOff
        )
    }

    /// The colour of its label.
    pub const fn label(self) -> Color {
        match self {
            Style::Primary => ON_ACCENT,
            Style::Tonal | Style::Selected => ON_SELECTED,
            Style::Secondary => TEXT,
            Style::Ghost => ACCENT,
            Style::Disabled | Style::PrimaryOff | Style::TonalOff | Style::GhostOff => DIM,
        }
    }
}

impl Theme {
    /// The opacity of a filled button's fill while it is not available
    /// now: fainter in a high contrast theme, so its dimmed label still
    /// stands 7:1.
    pub fn off_fill(self) -> f32 {
        use faraday_draw::tokens::{STATE_OFF_FILL, STATE_OFF_FILL_HIGH_CONTRAST};
        if self.is_high_contrast() {
            STATE_OFF_FILL_HIGH_CONTRAST
        } else {
            STATE_OFF_FILL
        }
    }
}

impl<A: Copy + PartialEq> Ui<'_, A> {
    /// In a high contrast theme, the edge to draw in place of a fill of
    /// `c` when `c` is one of the theme's colours made see-through as a
    /// tint: a tint under text or an icon would lower its contrast below
    /// the theme's 7:1, so what it marks is edged in its colour at full
    /// strength instead. `None` elsewhere.
    pub(crate) fn tint_edge(&self, c: Color) -> Option<Color> {
        use faraday_draw::tokens::HIGH_CONTRAST_TINT_MAX;
        (self.theme.is_high_contrast() && c.a < HIGH_CONTRAST_TINT_MAX && self.theme.has_role(c))
            .then(|| self.col(c.with_alpha(255)))
    }

    /// `c` at full strength in a high contrast theme where it is one of
    /// the theme's colours made see-through: an edge, a rule or a mark
    /// keeps its 3:1 there. `c` as it is elsewhere.
    pub(crate) fn solid(&self, c: Color) -> Color {
        if self.theme.is_high_contrast() && c.a < 255 && self.theme.has_role(c) {
            c.with_alpha(255)
        } else {
            c
        }
    }

    /// A filled rectangle with rounded corners in `color` as it is asked
    /// for in every theme, see-through included: a state layer, a
    /// control not available now, something fading in or out.
    pub fn wash(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
        let color = self.col(color);
        let rect = self.rect(x, y, w, h);
        if r <= 0.0 {
            self.c.fill_rect(rect, color);
        } else {
            self.c.fill_rounded_rect(rect, r * self.f, color);
        }
    }

    /// Keeps where `drawn`, a press just offered (pixels, clipped), is
    /// taken: grown to the touch floor inside `clip`, sharing any overlap
    /// with the targets before it ([`Reach`]).
    pub(crate) fn reach_for(&mut self, drawn: Rect, clip: Rect, action: A) {
        let floor = self.px(TOUCH);
        let gw = (floor - drawn.w).max(0);
        let gh = (floor - drawn.h).max(0);
        let mut target = Rect::new(
            drawn.x - gw / 2,
            drawn.y - gh / 2,
            drawn.w + gw,
            drawn.h + gh,
        )
        .intersect(&clip);
        if gw > 0 || gh > 0 {
            for r in self.reach.iter_mut() {
                if r.target.intersect(&target).is_empty() {
                    continue;
                }
                let (a, b) = split(r.target, target, r.drawn, drawn);
                r.target = a;
                target = b;
            }
        }
        self.reach.push(Reach {
            target,
            drawn,
            action,
        });
    }

    /// Forgets every press drawn so far this frame: what is under a
    /// sheet is out of reach.
    pub fn clear_hits(&mut self) {
        self.hits.clear();
        self.reach.clear();
    }

    /// The state layer over a control at (x, y), w by h with corners of
    /// `r`: its content colour laid over it, faint while the pointer is
    /// over it, stronger while the keyboard is on it, strongest while it
    /// is pressed.
    pub fn state_layer(&mut self, at: (f32, f32, f32, f32, f32), content: Color, action: A) {
        let (x, y, w, h, r) = at;
        let opacity = if self.is_pressed(action) {
            STATE_PRESS
        } else if self.focused == Some(action) {
            STATE_FOCUS
        } else if self.is_hovered(action) {
            STATE_HOVER
        } else {
            return;
        };
        self.wash(x, y, w, h, r, content.with_alpha(alpha_of(opacity)));
    }

    /// The focus ring round a control at (x, y), w by h with corners of
    /// `r`, when the keyboard is on `action`: [`FOCUS_RING`] thick,
    /// [`FOCUS_OFFSET`] outside it, in the accent as it is read.
    pub fn focus_ring(&mut self, at: (f32, f32, f32, f32, f32), action: A) {
        let (x, y, w, h, r) = at;
        if self.focused != Some(action) {
            return;
        }
        let out = FOCUS_OFFSET + FOCUS_RING;
        let rect = self.rect(x - out, y - out, w + 2.0 * out, h + 2.0 * out);
        let color = self.theme.color(ACCENT_TEXT);
        self.c
            .stroke_rounded_rect(rect, (r + out) * self.f, FOCUS_RING * self.f, color);
    }

    /// [`Ui::hit`] for a control with corners of `r`: its state layer and
    /// focus ring follow its shape.
    pub fn hit_shaped(&mut self, at: (f32, f32, f32, f32, f32), action: A) {
        let (x, y, w, h, r) = at;
        if !self.dimmed.contains(&action) {
            self.state_layer((x, y, w, h, r), TEXT, action);
            self.focus_ring((x, y, w, h, r), action);
        }
        self.hit_drawn(x, y, w, h, action);
    }

    /// The width of one segment of a segmented button over `labels` at
    /// `size`: the widest label, its padding and the tick's room, so no
    /// segment changes width as the choice moves.
    pub fn segment_width(&self, size: f32, labels: &[&str]) -> f32 {
        let widest = labels
            .iter()
            .map(|l| self.measure(size, W::S, l))
            .fold(0.0, f32::max);
        widest + 2.0 * SEGMENT_PAD + ICON_GLYPH_BUTTON + ICON_GAP
    }

    /// A segmented button (Material's): `options` side by side in one
    /// outlined container `h` tall at (x, y), round at its ends, each
    /// segment [`Ui::segment_width`] wide; the chosen one filled in
    /// `SELECTED` with a tick. `slide`, while the choice moves, is where
    /// the fill is, in segments from the first. Returns its width.
    pub fn segmented(
        &mut self,
        (x, y, h): (f32, f32, f32),
        size: f32,
        options: &[(&str, A)],
        chosen: Option<usize>,
        slide: Option<f32>,
    ) -> f32 {
        let labels: Vec<&str> = options.iter().map(|o| o.0).collect();
        let sw = self.segment_width(size, &labels);
        let total = sw * options.len() as f32;
        let r = shape_full(h);
        let shape = self.rect(x, y, total, h);
        let at = slide.or(chosen.map(|c| c as f32));
        if let Some(at) = at {
            let seg = self.rect(x + at * sw, y, sw, h).intersect(&shape);
            let fill = self.col(SELECTED);
            self.c.push_clip(seg);
            self.c.fill_rounded_rect(shape, r * self.f, fill);
            self.c.pop_clip();
        }
        for (i, &(label, action)) in options.iter().enumerate() {
            let sx = x + i as f32 * sw;
            let on = chosen == Some(i);
            let seg = self.rect(sx, y, sw, h).intersect(&shape);
            let layer = if on { ON_SELECTED } else { TEXT };
            self.c.push_clip(seg);
            self.state_layer((x, y, total, h, r), layer, action);
            self.c.pop_clip();
            if i > 0 {
                self.vrule(sx, y, h, OUTLINE);
            }
            let tw = self.measure(size, W::S, label);
            if on {
                let gw = ICON_GLYPH_BUTTON + ICON_GAP + tw;
                let gx = sx + (sw - gw) / 2.0;
                let iy = y + (h - ICON_GLYPH_BUTTON) / 2.0;
                self.icon(
                    gx,
                    iy,
                    ICON_GLYPH_BUTTON,
                    Icon::Done,
                    ICON_GLYPH_BUTTON,
                    ON_SELECTED,
                );
                let lx = gx + ICON_GLYPH_BUTTON + ICON_GAP;
                self.text_mid(lx, y, h, size, W::S, ON_SELECTED, label);
            } else {
                self.text_mid(sx + (sw - tw) / 2.0, y, h, size, W::S, TEXT, label);
            }
            self.focus_ring((sx, y, sw, h, r), action);
            self.hit_drawn(sx, y, sw, h, action);
        }
        self.stroke(x, y, total, h, r, OUTLINE);
        total
    }

    /// A choice of one among `options` in a width `w` from (x, y): a
    /// segmented button `h` tall where it fits, else the options as
    /// buttons wrapping onto rows, the chosen one [`Style::Selected`].
    /// Returns the height it took.
    pub fn choices(
        &mut self,
        (x, y, w, h): (f32, f32, f32, f32),
        size: f32,
        options: &[(&str, A)],
        chosen: Option<usize>,
    ) -> f32 {
        let labels: Vec<&str> = options.iter().map(|o| o.0).collect();
        if self.segment_width(size, &labels) * options.len() as f32 <= w {
            self.segmented((x, y, h), size, options, chosen, None);
            return h;
        }
        let pad = 2.0 * SEGMENT_PAD + ICON_GLYPH_BUTTON + ICON_GAP;
        let (mut bx, mut by) = (x, y);
        for (i, &(label, action)) in options.iter().enumerate() {
            let bw = (self.measure(size, W::S, label) + pad).min(w);
            if bx > x && bx + bw > x + w {
                bx = x;
                by += h + GAP;
            }
            let shown = self.fit(size, W::S, label, bw - pad);
            let style = Style::chosen(chosen == Some(i));
            bx += self.button(bx, by, Some(bw), h, &shown, style, action) + GAP;
        }
        by - y + h
    }

    /// A linear progress indicator `w` wide at (x, y), [`PROGRESS_H`]
    /// tall: `done`, 0 to 1, of the track filled; or, for work with no
    /// count (`done` `None`), a bar moving along it, `phase` of the way
    /// through its pass, or with reduce motion (`phase` `None`) the
    /// whole track in a fainter accent, still.
    pub fn progress(&mut self, (x, y, w): (f32, f32, f32), done: Option<f32>, phase: Option<f32>) {
        let h = PROGRESS_H;
        let r = shape_full(h);
        self.fill(x, y, w, h, r, SELECTED);
        match (done, phase) {
            (Some(d), _) => {
                let dw = w * d.clamp(0.0, 1.0);
                if dw > 0.0 {
                    self.fill(x, y, dw, h, r, ACCENT);
                }
            }
            (None, Some(p)) => {
                // The bar enters at the left, crosses and leaves at the
                // right, clipped to the track.
                let span = w * PROGRESS_SPAN;
                let at = x - span + (w + span) * p.clamp(0.0, 1.0);
                let track = self.rect(x, y, w, h);
                self.c.push_clip(track);
                self.fill(at, y, span, h, r, ACCENT);
                self.c.pop_clip();
            }
            (None, None) => {
                self.wash(x, y, w, h, r, ACCENT.with_alpha(alpha_of(0.5)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_draw::Canvas;
    use faraday_shell_api::{BootState, DisplayInfo, SecureHardware};

    /// A canvas 640 × 480 at 160 dpi: a pixel a unit.
    fn canvas() -> Canvas {
        Canvas::new(&DisplayInfo {
            width: 640,
            height: 480,
            dpi: 160,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: false,
            secure: SecureHardware::None,
            boot: BootState::Unknown,
            memory_mib: None,
        })
    }

    /// The pixel at (x, y), red, green and blue.
    fn pixel(c: &Canvas, x: usize, y: usize) -> (u8, u8, u8) {
        let f = c.frame();
        let i = (y * usize::from(f.width) + x) * 4;
        (f.rgba[i], f.rgba[i + 1], f.rgba[i + 2])
    }

    #[test]
    fn a_press_just_beside_a_small_button_still_presses_it() {
        let mut c = canvas();
        let mut hits = Vec::new();
        let mut ui: Ui<u8> = Ui::new(&mut c, 1.0, &mut hits, None);
        ui.button(100.0, 100.0, Some(80.0), 30.0, "Go", Style::Primary, 1);
        // 30 tall: 9 above and 9 below take the press too.
        assert_eq!(reach_at(&ui.reach, 140, 92).map(|r| r.0), Some(1));
        assert_eq!(reach_at(&ui.reach, 140, 138).map(|r| r.0), Some(1));
        assert_eq!(reach_at(&ui.reach, 140, 80), None);
    }

    #[test]
    fn two_buttons_close_together_share_the_gap_between_them() {
        let mut c = canvas();
        let mut hits = Vec::new();
        let mut ui: Ui<u8> = Ui::new(&mut c, 1.0, &mut hits, None);
        ui.button(100.0, 100.0, Some(80.0), 30.0, "Up", Style::Secondary, 1);
        ui.button(100.0, 138.0, Some(80.0), 30.0, "Down", Style::Secondary, 2);
        // The gap is 130 to 138: each takes it up to 134.
        assert_eq!(reach_at(&ui.reach, 140, 133).map(|r| r.0), Some(1));
        assert_eq!(reach_at(&ui.reach, 140, 134).map(|r| r.0), Some(2));
        // Neither reaches over the other's button.
        let up = ui.reach.iter().find(|r| r.action == 1).map(|r| r.target);
        assert!(up.is_some_and(|t| t.bottom() <= 138));
    }

    #[test]
    fn tab_goes_through_the_controls_in_the_order_drawn_and_round() {
        let order = [3u8, 1, 3, 2];
        assert_eq!(focus_step(&order, None, false), Some(3));
        assert_eq!(focus_step(&order, Some(3), false), Some(1));
        assert_eq!(focus_step(&order, Some(2), false), Some(3));
        assert_eq!(focus_step(&order, Some(3), true), Some(2));
        assert_eq!(focus_step(&order, None, true), Some(2));
        assert_eq!(focus_step::<u8>(&[], None, false), None);
    }

    /// What is drawn just outside a button's left end, with the keyboard
    /// on `focused`.
    fn beside_a_button(focused: Option<u8>) -> (u8, u8, u8) {
        let mut c = canvas();
        c.clear(Color::rgb(0x19, 0x21, 0x2a));
        let mut hits = Vec::new();
        {
            let mut ui: Ui<u8> = Ui::new(&mut c, 1.0, &mut hits, None);
            ui.focused = focused;
            ui.button(100.0, 100.0, Some(80.0), 40.0, "Go", Style::Secondary, 1);
        }
        pixel(&c, 96, 120)
    }

    #[test]
    fn the_control_the_keyboard_is_on_is_ringed() {
        assert_ne!(beside_a_button(Some(1)), beside_a_button(None));
        assert_eq!(beside_a_button(Some(2)), beside_a_button(None));
    }

    /// Inside a button of `style` away from its label, and whether it
    /// took a press.
    fn inside_a_button(style: Style) -> ((u8, u8, u8), bool) {
        let mut c = canvas();
        c.clear(Color::rgb(0x19, 0x21, 0x2a));
        let mut hits = Vec::new();
        {
            let mut ui: Ui<u8> = Ui::new(&mut c, 1.0, &mut hits, None);
            ui.button(100.0, 100.0, Some(120.0), 40.0, "", style, 1);
        }
        (pixel(&c, 130, 120), !hits.is_empty())
    }

    #[test]
    fn a_filled_button_not_available_keeps_its_fill() {
        let page = (0x19, 0x21, 0x2a);
        let (filled, pressable) = inside_a_button(Style::Primary.when(false));
        assert_ne!(filled, page);
        assert!(!pressable, "a button not available takes no press");
        assert_eq!(inside_a_button(Style::Secondary.when(false)).0, page);
        assert!(inside_a_button(Style::Primary).1);
    }

    #[test]
    fn nothing_is_drawn_smaller_than_the_smallest_text() {
        let mut c = canvas();
        let mut hits = Vec::new();
        let ui: Ui<u8> = Ui::new(&mut c, 1.0, &mut hits, None);
        for w in [W::R, W::S, W::M] {
            let small = ui.measure(8.0, w, "Box 17");
            assert_eq!(small, ui.measure(10.0, w, "Box 17"));
            assert!(ui.measure(14.0, w, "Box 17") > small);
        }
    }

    #[test]
    fn every_segment_of_a_segmented_button_takes_its_own_press() {
        let mut c = canvas();
        let mut hits = Vec::new();
        let mut ui: Ui<u8> = Ui::new(&mut c, 1.0, &mut hits, None);
        let options = [("One", 1u8), ("Two", 2), ("Three", 3)];
        let w = ui.segmented((20.0, 20.0, 40.0), 14.0, &options, Some(1), None);
        let seg = w / 3.0;
        for (i, (_, a)) in options.iter().enumerate() {
            let x = (20.0 + seg * (i as f32 + 0.5)) as i32;
            let found = ui.hits.iter().rev().find(|(r, _)| r.contains(x, 40));
            assert_eq!(found.map(|h| h.1), Some(*a));
        }
    }
}
