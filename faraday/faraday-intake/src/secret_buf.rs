//! Text a person types that may be a secret: seed words, a passphrase,
//! dice rolls, a share.
//!
//! A growable buffer that moves to a larger one as it fills frees the old
//! buffer as it is, so every word typed before the move stays in freed
//! memory until the allocator reuses it. [`SecretText`] never leaves a
//! copy behind: it starts with room for a long passphrase and, should it
//! need more, copies into a buffer twice the size and wipes the old one
//! before freeing it. Clearing, replacing and dropping it wipe the whole
//! buffer, the bytes past the end included. It is a [`Text`] under a
//! name that marks what it holds.

use std::fmt;
use std::ops::Deref;

use faraday_text::Text;
use zeroize::Zeroize;

use crate::ROOM;

pub use crate::room;

/// Text for typed secrets, wiped whenever it moves, is cleared or is
/// dropped.
pub struct SecretText(Text);

impl Default for SecretText {
    fn default() -> Self {
        SecretText(Text::with_room(ROOM))
    }
}

impl SecretText {
    /// An empty one.
    pub fn new() -> SecretText {
        SecretText::default()
    }

    /// One holding `s`.
    pub fn of(s: &str) -> SecretText {
        let mut t = SecretText::new();
        t.push_str(s);
        t
    }

    /// Adds a character at the end.
    pub fn push(&mut self, c: char) {
        self.0.push(c);
    }

    /// Adds `s` at the end.
    pub fn push_str(&mut self, s: &str) {
        self.0.push_str(s);
    }

    /// Takes the last character off, wiping its bytes.
    pub fn pop(&mut self) -> Option<char> {
        self.0.pop()
    }

    /// Keeps the first `len` bytes, wiping the rest. `len` past the end,
    /// or not on a character boundary, keeps it all.
    pub fn truncate(&mut self, len: usize) {
        self.0.truncate(len);
    }

    /// Empties it, wiping the whole buffer.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Replaces what it holds with `s`.
    pub fn set(&mut self, s: &str) {
        self.0.set(s);
    }

    /// The text.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Deref for SecretText {
    type Target = str;

    fn deref(&self) -> &str {
        self.0.as_str()
    }
}

/// Never prints what it holds.
impl fmt::Debug for SecretText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecretText({} bytes)", self.0.len())
    }
}

impl PartialEq<str> for SecretText {
    fn eq(&self, other: &str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<&str> for SecretText {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

/// A copy is another [`SecretText`], wiped in its turn.
impl Clone for SecretText {
    fn clone(&self) -> Self {
        SecretText(self.0.duplicate())
    }
}

impl Zeroize for SecretText {
    fn zeroize(&mut self) {
        self.0.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_past_its_room_keeps_every_character() {
        let mut t = SecretText::new();
        let phrase = faraday_text::Text::repeated("abandon ", 200);
        for c in phrase.chars() {
            t.push(c);
        }
        assert_eq!(t, phrase.as_str());
        assert_eq!(t.pop(), Some(' '));
        t.truncate(7);
        assert_eq!(t, "abandon");
        t.set("zoo");
        assert_eq!(t, "zoo");
        t.clear();
        assert!(t.is_empty());
    }
}
