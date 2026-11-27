//! The runtime's one `Memo` (05 §6): made by `Memo::new`, ended by `Memo::end_frame` at each
//! frame's end and `Memo::clear` at every lock and every end.

use crate::root::Root;
use crate::runtime::Runtime;

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
