//! The runtime's `Memo` and its frame marks (05 §6).

use faraday_kernel::Slot;

use crate::root::Root;
use crate::runtime::Runtime;

/// Entries are keyed by (slot, the value's type, key), and kept while the last frame used them.
// Spec: 05 §6 gives the store's rule without its shape; `Memo` (kernel) has no constructor or
// mark of its own, so the frame marks are kept here.
pub(crate) struct Marks {
    /// The number of the frame being drawn.
    frame: u64,
    /// Each entry used in this frame: slot, the value's type and a hash of the key.
    used: Vec<(Slot, core::any::TypeId, u64)>,
}

impl<R: Root> Runtime<R> {
    /// An entry no frame used is dropped at that frame's end.
    pub(crate) fn memo_frame_end(&mut self) {
        todo!()
    }

    /// The store is emptied at every lock and every end.
    pub(crate) fn memo_empty(&mut self) {
        todo!()
    }
}
