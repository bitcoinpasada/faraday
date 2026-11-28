//! The read view the owner's `present` maps to a kit model.

use crate::kind::Kind;
use crate::state::{State, Which};

/// One field of the part, as its owner's `present` reads it.
pub struct Read<'s> {
    state: &'s State,
    which: Which,
    typing: bool,
    practice: bool,
}

impl State {
    /// The view of field `which`; `typing` is false while a stick is in the way, and the field
    /// then takes no typing; `practice` is `view.mode.practice`.
    pub fn read(&self, which: Which, typing: bool, practice: bool) -> Read<'_> {
        Read {
            state: self,
            which,
            typing,
            practice,
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

    /// Practice's `faraday` chip is offered: only in Practice, never for a KeePass prompt (its
    /// minimum is a rule the sample is under), and only while this field and the other of a pair
    /// are empty, so it never writes over what was typed.
    pub fn chip(&self) -> bool {
        self.practice
            && self.state.kind() != Kind::Kdbx
            && self.text().is_empty()
            && !self.state.other_typed(self.which)
    }
}
