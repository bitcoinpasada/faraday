//! One frame's drawing context, generic over the press type (04 §7.1).

use core::marker::PhantomData;

use faraday_draw::canvas::Canvas;
use faraday_port::values::{Anchor, Point};
use faraday_tokens::Theme;

use crate::Pressed;
use crate::frame_out::{FrameOut, Laid};
use crate::motion::MotionStore;

/// The last frame's pointer state.
// Spec: 04 §1 and 09 §3.2 name `Pointer` without a shape.
pub struct Pointer {
    pub at: Option<Point>,
    pub down: bool,
}

/// One frame's drawing context: canvas, scale, theme, pointer state, motion store, lift.
pub struct Ui<'f, A: Pressed> {
    canvas: &'f mut Canvas,
    theme: Theme,
    scale: f32,
    motion: &'f mut MotionStore,
    pointer: &'f Pointer,
    ink: bool,
    press: PhantomData<fn(A)>,
}

impl<'f, A: Pressed> Ui<'f, A> {
    /// The runtime's only.
    pub fn new(
        _canvas: &'f mut Canvas,
        _theme: Theme,
        _scale: f32,
        _motion: &'f mut MotionStore,
        _pointer: &'f Pointer,
        _ink: bool,
    ) -> Self {
        todo!()
    }

    /// The runtime's only.
    pub fn finish(self) -> FrameOut<A> {
        todo!()
    }

    /// A child page draws here; its presses are lifted to this one's.
    pub fn within<B: Pressed>(&mut self, _lift: fn(B) -> A, _f: impl FnOnce(&mut Ui<'_, B>)) {
        todo!()
    }

    /// A place a later Reveal or glide names. Stable across frames.
    pub fn anchor(&mut self, _id: Anchor) {
        todo!()
    }

    /// A layout fact an update reads.
    pub fn laid(&mut self, _id: Anchor, _fact: Laid, _to: fn(Laid) -> A) {
        todo!()
    }
}
