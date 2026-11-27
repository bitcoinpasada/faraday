//! Focus and the focus ring (04 §7.1).
//!
//! Every control that records a hit draws the focus ring round itself when its press, lifted to
//! the root's, is `Pointer::focused`.

use faraday_draw::Rect;
use faraday_tokens::dims::{
    FOCUS_OFFSET, FOCUS_RING, STATE_FOCUS, STATE_HOVER, STATE_PRESS, alpha_of,
};
use faraday_tokens::palette::{Paint, Role};

use crate::Pressed;
use crate::ui::Ui;

/// Where Tab (`back` false) or Shift+Tab moves the focus among `order`, the presses of
/// `FrameOut::reach` in drawing order: the next or the one before, round at the ends; the first
/// or the last from none. The runtime's, on a key the frame's focus takes.
pub fn step<R: Clone + PartialEq>(order: &[R], at: Option<&R>, back: bool) -> Option<R> {
    let mut seen: Vec<&R> = Vec::with_capacity(order.len());
    for a in order {
        if !seen.contains(&a) {
            seen.push(a);
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
    Some(seen[next].clone())
}

impl<A: Pressed> Ui<'_, A> {
    /// The state layer over a control at (x, y), w by h with corners of
    /// `r`: its content colour laid over it, faint while the pointer is
    /// over it, stronger while the keyboard is on it, strongest while it
    /// is pressed.
    pub fn state_layer(&mut self, at: (f32, f32, f32, f32, f32), content: Paint, action: &A) {
        let (x, y, w, h, r) = at;
        // The pointer is over the control's target while a press draws its layer, over the
        // layer's own box otherwise.
        let over = self.over.unwrap_or_else(|| self.pointer_over(x, y, w, h));
        // Over it by position, or its press is the one the last frame found under the pointer
        // (a control drawn twice for one press lights in both).
        let pressed = (over && self.down) || self.is_pressed(action);
        let opacity = if pressed {
            STATE_PRESS
        } else if self.is_focused(action) {
            STATE_FOCUS
        } else if over || self.is_hovered(action) {
            STATE_HOVER
        } else {
            return;
        };
        self.wash(x, y, w, h, r, content.with_alpha(alpha_of(opacity)));
    }

    /// The focus ring round a control at (x, y), w by h with corners of
    /// `r`, when the keyboard is on `action`: [`FOCUS_RING`] thick,
    /// [`FOCUS_OFFSET`] outside it, in secondary (`docs/THEMES.md` §7).
    pub fn focus_ring(&mut self, at: (f32, f32, f32, f32, f32), action: &A) {
        let (x, y, w, h, r) = at;
        if !self.is_focused(action) {
            return;
        }
        let out = FOCUS_OFFSET + FOCUS_RING;
        let rect = self.rect(x - out, y - out, w + 2.0 * out, h + 2.0 * out);
        let color = self.col(Role::Secondary);
        self.c
            .stroke_rounded_rect(rect, (r + out) * self.f, FOCUS_RING * self.f, color);
    }

    /// The focus ring round one segment of a segmented button at (x, y),
    /// w by h, when the keyboard is on `action`: round with corners of
    /// `r` at the button's own ends (`first`, `last`), square where the
    /// segment meets another, as the segment's shape is.
    pub fn segment_ring(
        &mut self,
        (x, y, w, h, r): (f32, f32, f32, f32, f32),
        (first, last): (bool, bool),
        action: &A,
    ) {
        if !self.is_focused(action) {
            return;
        }
        let out = FOCUS_OFFSET + FOCUS_RING;
        let ring = self.rect(x - out, y - out, w + 2.0 * out, h + 2.0 * out);
        // A ring reaching on past each inner end far enough that its
        // round corner there falls outside the segment's own, cut to the
        // segment's; a straight side closes it there.
        let reach = |end: bool| if end { 0.0 } else { r + out };
        let (left, right) = (reach(first), reach(last));
        let shape = self.rect(
            x - out - left,
            y - out,
            w + 2.0 * out + left + right,
            h + 2.0 * out,
        );
        let color = self.col(Role::Secondary);
        let thick = FOCUS_RING * self.f;
        self.c.push_clip(ring);
        self.c
            .stroke_rounded_rect(shape, (r + out) * self.f, thick, color);
        let t = (thick.round() as i32).max(1);
        if !first {
            self.c
                .fill_rect(Rect::new(ring.x, ring.y, t, ring.h), color);
        }
        if !last {
            self.c
                .fill_rect(Rect::new(ring.right() - t, ring.y, t, ring.h), color);
        }
        self.c.pop_clip();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Invariant: Tab goes through the controls in the order drawn and round, and Shift+Tab goes the other way.
    /// Regression: Tab skips a control drawn twice or stops at the last one instead of going round.
    /// Owner: kit
    #[test]
    fn tab_goes_through_the_controls_in_the_order_drawn_and_round() {
        let order = [3u8, 1, 3, 2];
        assert_eq!(step(&order, None, false), Some(3));
        assert_eq!(step(&order, Some(&3), false), Some(1));
        assert_eq!(step(&order, Some(&2), false), Some(3));
        assert_eq!(step(&order, Some(&3), true), Some(2));
        assert_eq!(step(&order, None, true), Some(2));
        assert_eq!(step::<u8>(&[], None, false), None);
    }
}
