//! The ring of 512 lines (05 §8).

use std::collections::VecDeque;

use crate::root::Root;
use crate::runtime::Runtime;

/// The ring's size.
pub(crate) const LINES: usize = 512;

/// One line: the time, the generation, and a kind, with the message whole where it is public.
pub struct LogLine {
    pub time_ms: u64,
    pub generation: u32,
    /// `Event::kind()`, `Effect::kind()` or `Message::kind()`, as text.
    pub kind: &'static str,
    /// `Message::public()` as text; `None` for port traffic and for a withheld variant.
    pub public: Option<String>,
}

/// Held in memory, written nowhere, dropped at every lock and end.
pub(crate) struct Log {
    on: bool,
    lines: VecDeque<LogLine>,
}

impl Log {
    pub(crate) fn push(&mut self, _line: LogLine) {
        todo!()
    }

    pub(crate) fn drop_all(&mut self) {
        todo!()
    }
}

impl<R: Root> Runtime<R> {
    /// A port event or effect, logged by its kind (P15).
    pub(crate) fn log_port(&mut self, _kind: &'static str) {
        todo!()
    }

    /// A dispatch, logged by `Message::kind()` and `Message::public()` (R24).
    pub(crate) fn log_message(&mut self, _message: &dyn faraday_kernel::Message) {
        todo!()
    }
}
