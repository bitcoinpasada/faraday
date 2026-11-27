//! The wiping frame buffer, painting the call list, the last `FrameOut` (05 §2.5).

use faraday_atoms::FrameOut;
use faraday_crypto::Secret;
use faraday_draw::canvas::Calls;
use faraday_kernel::ViewOrder;
use faraday_port as port;

use crate::root::Root;
use crate::runtime::Runtime;

/// What the last frame keeps until the next (§2.5): the buffer, the output, the waiting orders.
pub(crate) struct Painted<R: Root> {
    /// P13: wiping.
    buffer: Secret<Vec<u8>>,
    calls: Option<Calls>,
    changed: Vec<port::frame::Rect>,
    out: Option<FrameOut<R::Press>>,
    /// The number of the last frame drawn.
    number: u64,
    /// `Fx::View` orders waiting for their anchor to be drawn.
    orders: Vec<ViewOrder>,
    /// The last laid fact sent for each `Anchor` (V17).
    sent: Vec<port::values::Anchor>,
}

impl<R: Root> Runtime<R> {
    /// §2.5, steps 1 to 7: build `View` and `Show`, call `R::frame`, finish, paint, move the
    /// motion store, send the laid facts, keep the frame for a fade when it is not secret.
    pub(crate) fn draw(&mut self) {
        todo!()
    }

    /// The canvas replays its call list into the buffer, recording the changed rectangles.
    pub(crate) fn paint(&mut self) {
        todo!()
    }

    /// R9: a press hit-tested against the last frame handed out.
    pub(crate) fn press_at(&mut self, _point: port::values::Point) {
        todo!()
    }

    /// §7.2 step 7: the buffer zeroed, the call list and the frame's output dropped.
    pub(crate) fn wipe_frame(&mut self) {
        todo!()
    }
}
