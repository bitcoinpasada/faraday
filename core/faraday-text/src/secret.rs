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

use core::fmt;
use core::ops::Deref;

use subtle::ConstantTimeEq;
use zeroize::Zeroize;

use crate::Text;

/// The room a seed phrase is built in, bytes: more than 24 words of the
/// longest BIP-39 list, so a seed phrase never moves.
pub const ROOM: usize = 512;

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

    /// Alike, compared in constant time over the bytes (the lengths are not hidden): for a
    /// comparison that decides access, where `==` is not to be used.
    pub fn ct_eq(&self, other: &SecretText) -> bool {
        self.as_str()
            .as_bytes()
            .ct_eq(other.as_str().as_bytes())
            .into()
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

    /// Invariant: A secret text keeps every character typed into it, however long, and gives each back as edited.
    /// Regression: The text is held in a fixed buffer that drops what passes its room.
    /// Owner: domain
    #[test]
    fn typing_past_its_room_keeps_every_character() {
        let mut t = SecretText::new();
        let phrase = Text::repeated("abandon ", 200);
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

    /// Invariant: Two secret texts compare alike only when every byte is the same, so a pair of passphrases that differ anywhere, or in length, is never taken as matching.
    /// Regression: The constant-time comparison stops at the shorter text and calls a prefix a match.
    /// Owner: domain
    #[test]
    fn a_secret_matches_only_the_same_bytes() {
        let of = |s: &str| {
            let mut t = SecretText::new();
            t.set(s);
            t
        };
        let a = of("correct horse");
        assert!(a.ct_eq(&of("correct horse")));
        assert!(!a.ct_eq(&of("correct horsf")));
        assert!(!a.ct_eq(&of("correct hors")));
        assert!(!a.ct_eq(&of("")));
    }
}
