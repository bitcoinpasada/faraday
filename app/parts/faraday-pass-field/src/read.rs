//! The read view the owner's `present` maps to a kit model.

use crate::state::{State, Which};

/// One field of the part, as its owner's `present` reads it.
pub struct Read<'s> {
    state: &'s State,
    which: Which,
    typing: bool,
}

impl State {
    /// The view of field `which`; `typing` is false while a stick is in the way, and the field
    /// then takes no typing.
    pub fn read(&self, which: Which, typing: bool) -> Read<'_> {
        Read {
            state: self,
            which,
            typing,
        }
    }
}

impl<'s> Read<'s> {
    fn text(&self) -> &'s str {
        match self.which {
            Which::First => self.state.text.as_str(),
            Which::Again => self.state.again.as_ref().map_or("", |a| a.as_str()),
        }
    }

    /// Which field this is.
    pub fn which(&self) -> Which {
        self.which
    }

    /// The field takes typing now (no stick in the way).
    pub fn typing(&self) -> bool {
        self.typing
    }

    /// Typing goes to this field.
    pub fn focused(&self) -> bool {
        self.typing && (self.state.on_again() == (self.which == Which::Again))
    }

    /// How many characters it holds: the dots.
    pub fn count(&self) -> usize {
        self.text().chars().count()
    }

    /// The text, only while the eye is open.
    pub fn revealed(&self) -> Option<&'s str> {
        self.state.shown().then(|| self.text())
    }

    /// The eye is open.
    pub fn shown(&self) -> bool {
        self.state.shown()
    }

    /// The passphrase waits for the eye to be opened once: the eye in primary.
    pub fn marked(&self) -> bool {
        self.state.waits()
    }
}
