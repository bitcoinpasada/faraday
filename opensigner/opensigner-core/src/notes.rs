//! Notes and recovery sheets (`docs/PLANNING.md` §16.112 rule 2).
//!
//! A note is text a person types on this device or reads from a plain
//! file: instructions for an heir, where the parts of a backup are,
//! what a wallet is for. It is shown whole on a Document screen and
//! exported plain or encrypted, and nothing keeps it past the session.
//!
//! A note is a secret. It is held in a buffer that zeroizes on drop and
//! is never in heap text, which is why this file is listed in
//! `tools/lint-secrets.sh` and holds no heap text. What the Document
//! draws is the text itself rather than a secret panel: a note is what
//! a person reads, and a screen that hides it hides the whole point of
//! it. The hiding a note needs is the passphrase it is exported under.
//!
//! A recovery sheet is a wallet's descriptor, the name the person gave
//! it and a note, which is the document an heir is left with.

use alloc::vec::Vec;

use osk_crypto::{Zeroize, ZeroizeOnDrop};
use zeroize::Zeroizing;

/// Characters a note takes. Long enough for a page of instructions and
/// short enough to draw whole on the smallest panel without paging.
pub const MAX_NOTE: usize = 4096;

/// One note, as UTF-8 bytes.
pub struct Note {
    text: Zeroizing<Vec<u8>>,
}

impl Zeroize for Note {
    fn zeroize(&mut self) {
        self.text.zeroize();
    }
}

impl Drop for Note {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for Note {}

impl Default for Note {
    fn default() -> Self {
        Self::new()
    }
}

impl Note {
    /// An empty note.
    pub fn new() -> Self {
        Note {
            text: Zeroizing::new(Vec::new()),
        }
    }

    /// The note `bytes` spell, or `None` where they are not UTF-8 or
    /// are longer than a note takes. What "Read a file" and an opened
    /// [`osk_backup::oskb::KIND_NOTE`] payload both go through.
    pub fn from_bytes(bytes: &[u8]) -> Option<Note> {
        let text = core::str::from_utf8(bytes).ok()?;
        if text.chars().count() > MAX_NOTE {
            return None;
        }
        let mut note = Note::new();
        note.text.extend_from_slice(bytes);
        Some(note)
    }

    /// The text.
    pub fn text(&self) -> &str {
        core::str::from_utf8(&self.text).unwrap_or("")
    }

    /// The bytes, for the one call that seals them.
    pub fn bytes(&self) -> &[u8] {
        &self.text
    }

    /// Characters typed.
    pub fn chars(&self) -> usize {
        self.text().chars().count()
    }

    /// Whether nothing is in it.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Types one character; ignored where the note is full.
    pub fn push(&mut self, c: char) {
        if self.chars() >= MAX_NOTE {
            return;
        }
        let mut buf = [0u8; 4];
        self.text
            .extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
    }

    /// Removes the last character.
    pub fn pop(&mut self) {
        let n = match self.text().chars().next_back() {
            Some(c) => c.len_utf8(),
            None => return,
        };
        let keep = self.text.len() - n;
        self.text.truncate(keep);
    }

    /// Replaces the whole note, which is what a file or an opened
    /// payload hands it.
    pub fn set(&mut self, other: Note) {
        self.text.zeroize();
        self.text.extend_from_slice(other.bytes());
    }

    /// Empties it.
    pub fn clear(&mut self) {
        self.text.zeroize();
        self.text.clear();
    }
}
