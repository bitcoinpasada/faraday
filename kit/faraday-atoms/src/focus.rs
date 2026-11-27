//! Focus and the focus ring (04 §7.1).
//!
//! Every control that records a hit draws the focus ring round itself when its press, lifted to
//! the root's, is `Pointer::focused`.

/// Where Tab (`back` false) or Shift+Tab moves the focus among `order`, the presses of
/// `FrameOut::reach` in drawing order: the next or the one before, round at the ends; the first
/// or the last from none. The runtime's, on a key the frame's focus takes.
pub fn step<R: Clone + PartialEq>(_order: &[R], _at: Option<&R>, _back: bool) -> Option<R> {
    todo!()
}
