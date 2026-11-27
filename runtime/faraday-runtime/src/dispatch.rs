//! Orders, asks and replies, changes served (05 §3).

use std::collections::HashMap;

use faraday_kernel::{ChangeId, Effects, Fault, Message, Top, shared};
use faraday_port as port;

use crate::queue::Part;
use crate::root::{Addr, Root};
use crate::runtime::Runtime;

/// A kept reply, turned into the message of the top state that asked.
pub(crate) type KeptReply<R> = Box<dyn FnOnce(port::Answer) -> (Addr<R>, Fault) + Send>;

/// A kept change reply, likewise.
pub(crate) type KeptChange<R> = Box<dyn FnOnce(shared::Done) -> (Addr<R>, Fault) + Send>;

/// The reply table: each id to its reply and its top state (§3.1).
pub(crate) struct Replies<R: Root> {
    asks: HashMap<(u32, u32), (Part, KeptReply<R>)>,
    changes: HashMap<(u32, u32), (Part, KeptChange<R>)>,
    next_ask: u32,
    next_change: u32,
}

impl<R: Root> Runtime<R> {
    /// Walk the effects a top state returned, in order (§3).
    // Spec: 03 gives `Effects` no way to read its list; the runtime needs one.
    pub(crate) fn walk<M: Message>(&mut self, _from: Part, _effects: Effects<M, Top>) {
        todo!()
    }

    /// `Fx::Ask`: a new `AskId`, the reply kept, `Effect::Ask` appended.
    pub(crate) fn ask(&mut self, _from: Part, _ask: port::Ask, _reply: KeptReply<R>) {
        todo!()
    }

    /// `Fx::Change`: a new `ChangeId`, the reply kept, `Owner::given` pushed to its owner.
    pub(crate) fn change(
        &mut self,
        _from: Part,
        _change: shared::AnyChange,
        _reply: KeptChange<R>,
    ) {
        todo!()
    }

    /// `Fx::Done`: the kept reply under `id` called, the asker's message pushed.
    pub(crate) fn done(&mut self, _id: ChangeId, _done: shared::Done) {
        todo!()
    }

    /// `Event::Answer`: the kept reply called; an answer not in the table is dropped and wiped
    /// (R11, R12).
    pub(crate) fn answered(&mut self, _id: port::AskId, _answer: port::Answer) {
        todo!()
    }

    /// `Fx::Out`: handled as 05 §1.1 says.
    pub(crate) fn top(&mut self, _from: Part, _top: Top) {
        todo!()
    }

    /// One message to its top state through `route` (A9), with `View` built as R13 says.
    pub(crate) fn dispatch(&mut self, _addr: Addr<R>) {
        todo!()
    }
}
