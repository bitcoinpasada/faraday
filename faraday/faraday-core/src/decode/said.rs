//! A sentence of a check's finding or a comparison's line, with the address
//! it names kept apart from its words: the address is a part of its own, so
//! the screen draws it grouped on lines under the sentence and nothing finds
//! it again by reading text (`docs/DESIGN.md` §4.5, "Whole string").

/// One part of a [`Said`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Part {
    /// Words, drawn wrapped.
    Words(String),
    /// An address, whole; drawn grouped, in mono, under the words before it.
    Address(String),
    /// What stands where an address would, for an output that has none (a
    /// script's length), drawn as a line of its own.
    Script(String),
}

/// A sentence: its words and the addresses it names, in reading order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Said {
    /// The parts, top down.
    pub parts: Vec<Part>,
}

impl Said {
    /// A sentence of words alone.
    pub(crate) fn words(text: impl Into<String>) -> Self {
        Said {
            parts: vec![Part::Words(text.into())],
        }
    }

    /// This sentence, then `address` under it.
    pub(crate) fn address(mut self, address: impl Into<String>) -> Self {
        self.parts.push(Part::Address(address.into()));
        self
    }

    /// This sentence, then a script's length where there is no address.
    pub(crate) fn script(mut self, text: impl Into<String>) -> Self {
        self.parts.push(Part::Script(text.into()));
        self
    }

    /// This sentence, then more words under the part before.
    pub(crate) fn then(mut self, text: impl Into<String>) -> Self {
        self.parts.push(Part::Words(text.into()));
        self
    }

    /// The first words made by `f` from themselves.
    pub(crate) fn lead(mut self, f: impl FnOnce(&str) -> String) -> Self {
        if let Some(Part::Words(w)) = self.parts.first_mut() {
            *w = f(w);
        }
        self
    }
}
