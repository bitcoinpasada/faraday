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
    pub kind: String,
    /// `Message::public()` as text; `None` for port traffic and for a withheld variant.
    pub public: Option<String>,
}

/// Held in memory, written nowhere, dropped at every lock and end.
pub(crate) struct Log {
    on: bool,
    lines: VecDeque<LogLine>,
}

impl Log {
    /// Off and empty.
    pub(crate) fn new() -> Self {
        Log {
            on: false,
            lines: VecDeque::new(),
        }
    }

    /// Turns the log on or off; off, it holds nothing.
    pub(crate) fn set(&mut self, on: bool) {
        self.on = on;
        if !on {
            self.lines.clear();
        }
    }

    /// A line, when the log is on; the oldest goes when the ring is full.
    pub(crate) fn push(&mut self, line: LogLine) {
        if !self.on {
            return;
        }
        if self.lines.len() == LINES {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    pub(crate) fn drop_all(&mut self) {
        self.lines.clear();
    }

    pub(crate) fn lines(&self) -> impl Iterator<Item = &LogLine> + '_ {
        self.lines.iter()
    }
}

impl<R: Root> Runtime<R> {
    /// Turn the log on or off (§8). The dev card's verbose run, the tour root and tests.
    pub fn log(&mut self, on: bool) {
        self.log.set(on);
    }

    /// The lines held, the oldest first.
    pub fn log_lines(&self) -> impl Iterator<Item = &LogLine> + '_ {
        self.log.lines()
    }

    /// A port event or effect, logged by its kind (P15).
    pub(crate) fn log_port(&mut self, kind: &dyn core::fmt::Debug) {
        if !self.log.on {
            return;
        }
        let line = LogLine {
            time_ms: self.clock.now_ms,
            generation: self.generation,
            kind: format!("{kind:?}"),
            public: None,
        };
        self.log.push(line);
    }

    /// A dispatch, logged by `Message::kind()` and `Message::public()` (R24).
    pub(crate) fn log_message(&mut self, message: &dyn faraday_kernel::Message) {
        if !self.log.on {
            return;
        }
        let line = LogLine {
            time_ms: self.clock.now_ms,
            generation: self.generation,
            kind: message.kind().to_string(),
            public: message.public().map(|public| format!("{public:?}")),
        };
        self.log.push(line);
    }
}
