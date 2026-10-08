//! Text a person types that may be a secret: seed words, a passphrase,
//! dice rolls, a share.
//!
//! `Zeroizing<String>` wipes a string's buffer when it is dropped, but a
//! `String` that grows past its capacity moves to a larger buffer and
//! frees the old one as it is, so every word typed before the move stays
//! in freed memory until the allocator reuses it. [`SecretText`] never
//! leaves a copy behind: it starts with room for a long passphrase and,
//! should it need more, copies into a buffer twice the size and wipes the
//! old one before freeing it. Clearing, replacing and dropping it wipe
//! the whole buffer, the bytes past the end included.

use std::fmt;
use std::ops::Deref;

use zeroize::Zeroize;

/// The room a fresh one starts with, bytes: more than 24 words of the
/// longest BIP-39 list, so a seed phrase never moves.
const ROOM: usize = 512;

/// An empty `Zeroizing<String>` with room for a whole seed phrase, for
/// building one a word at a time: a string that has to grow leaves its
/// old buffer behind unwiped.
pub fn room() -> zeroize::Zeroizing<String> {
    zeroize::Zeroizing::new(String::with_capacity(ROOM))
}

/// `bytes` as lowercase hex, written into a buffer of the right size
/// with no piece of it made anywhere else: for a private key.
pub fn hex(bytes: &[u8]) -> zeroize::Zeroizing<String> {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = zeroize::Zeroizing::new(String::with_capacity(bytes.len() * 2));
    for b in bytes {
        out.push(char::from(DIGITS[usize::from(b >> 4)]));
        out.push(char::from(DIGITS[usize::from(b & 15)]));
    }
    out
}

/// A string for typed secrets, wiped whenever it moves, is cleared or is
/// dropped.
pub struct SecretText {
    text: String,
}

impl Default for SecretText {
    fn default() -> Self {
        SecretText {
            text: String::with_capacity(ROOM),
        }
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

    /// Makes room for `more` bytes without leaving a copy behind.
    fn room(&mut self, more: usize) {
        let need = self.text.len() + more;
        if need <= self.text.capacity() {
            return;
        }
        let mut bigger = String::with_capacity(need.max(self.text.capacity() * 2));
        bigger.push_str(&self.text);
        self.text.zeroize();
        self.text = bigger;
    }

    /// Adds a character at the end.
    pub fn push(&mut self, c: char) {
        self.room(c.len_utf8());
        self.text.push(c);
    }

    /// Adds `s` at the end.
    pub fn push_str(&mut self, s: &str) {
        self.room(s.len());
        self.text.push_str(s);
    }

    /// Takes the last character off, wiping its bytes.
    pub fn pop(&mut self) -> Option<char> {
        let c = self.text.chars().next_back()?;
        let at = self.text.len() - c.len_utf8();
        self.truncate(at);
        Some(c)
    }

    /// Keeps the first `len` bytes, wiping the rest. `len` past the end,
    /// or not on a character boundary, keeps it all.
    pub fn truncate(&mut self, len: usize) {
        if len >= self.text.len() || !self.text.is_char_boundary(len) {
            return;
        }
        // What is cut still sits in the buffer past the end: written over
        // with zeros, which needs no more room, and cut again.
        let tail = self.text.len() - len;
        self.text.truncate(len);
        self.text.extend(std::iter::repeat_n('\0', tail));
        self.text.truncate(len);
    }

    /// Empties it, wiping the whole buffer.
    pub fn clear(&mut self) {
        // Zeroes every byte of the buffer and keeps it.
        self.text.zeroize();
    }

    /// Replaces what it holds with `s`.
    pub fn set(&mut self, s: &str) {
        self.clear();
        self.push_str(s);
    }

    /// The text.
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl Deref for SecretText {
    type Target = str;

    fn deref(&self) -> &str {
        &self.text
    }
}

impl Drop for SecretText {
    fn drop(&mut self) {
        self.text.zeroize();
    }
}

/// Never prints what it holds.
impl fmt::Debug for SecretText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecretText({} bytes)", self.text.len())
    }
}

impl PartialEq<str> for SecretText {
    fn eq(&self, other: &str) -> bool {
        self.text == other
    }
}

impl PartialEq<&str> for SecretText {
    fn eq(&self, other: &&str) -> bool {
        self.text == *other
    }
}

/// A copy is another [`SecretText`], wiped in its turn.
impl Clone for SecretText {
    fn clone(&self) -> Self {
        SecretText::of(&self.text)
    }
}

impl Zeroize for SecretText {
    fn zeroize(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_past_its_room_keeps_every_character() {
        let mut t = SecretText::new();
        let phrase = "abandon ".repeat(200);
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
