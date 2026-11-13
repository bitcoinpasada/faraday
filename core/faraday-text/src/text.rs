//! The wiping text type.

use alloc::vec::Vec;
use core::borrow::Borrow;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::Deref;

use zeroize::Zeroize;

use crate::num::Num;

/// UTF-8 text in one buffer that is wiped whenever it moves, is cleared or
/// is dropped.
///
/// Only `&str` and `char` are written in, so the bytes are always UTF-8.
/// It has no `Display` and its `Debug` shows the length only.
pub struct Text {
    buf: Vec<u8>,
}

impl Text {
    /// An empty one with no buffer yet.
    pub const fn new() -> Text {
        Text { buf: Vec::new() }
    }

    /// An empty one with room for `bytes` bytes.
    pub fn with_room(bytes: usize) -> Text {
        Text {
            buf: Vec::with_capacity(bytes),
        }
    }

    /// One holding `s`, in a buffer of exactly its size.
    pub fn of(s: &str) -> Text {
        let mut t = Text::with_room(s.len());
        t.push_str(s);
        t
    }

    /// Fills a template's `{}` placeholders from `args`, in order, into a
    /// buffer of exactly the size of the result. A placeholder with no
    /// argument is dropped; a spare argument is ignored.
    pub fn fill(template: &str, args: &[&str]) -> Text {
        let holes = template.matches("{}").count();
        let used: usize = args.iter().take(holes).map(|a| a.len()).sum();
        let mut out = Text::with_room(template.len() - 2 * holes + used);
        let mut rest = template;
        let mut next = args.iter();
        while let Some(i) = rest.find("{}") {
            out.push_str(&rest[..i]);
            if let Some(a) = next.next() {
                out.push_str(a);
            }
            rest = &rest[i + 2..];
        }
        out.push_str(rest);
        out
    }

    /// `bytes` as text, each run of bytes that is not UTF-8 replaced by
    /// U+FFFD.
    pub fn from_utf8_lossy(bytes: &[u8]) -> Text {
        let mut out = Text::with_room(bytes.len());
        for chunk in bytes.utf8_chunks() {
            out.push_str(chunk.valid());
            if !chunk.invalid().is_empty() {
                out.push('\u{FFFD}');
            }
        }
        out
    }

    /// The decimal digits of `n`, on the stack.
    pub fn num(n: u64) -> Num {
        Num::from_u64(n)
    }

    /// The decimal digits of `n`, on the stack.
    pub fn int(n: i64) -> Num {
        Num::from_i64(n)
    }

    /// Makes room for `more` bytes without leaving a copy behind: a
    /// buffer of the larger of the need and twice the size, the old one
    /// wiped before it is freed.
    fn room(&mut self, more: usize) {
        let need = self.buf.len() + more;
        if need <= self.buf.capacity() {
            return;
        }
        let mut bigger = Vec::with_capacity(need.max(self.buf.capacity() * 2));
        bigger.extend_from_slice(&self.buf);
        self.buf.zeroize();
        self.buf = bigger;
    }

    /// Adds a character at the end.
    pub fn push(&mut self, c: char) {
        let mut one = [0u8; 4];
        let s = c.encode_utf8(&mut one);
        self.push_str(s);
        one.zeroize();
    }

    /// Adds `s` at the end.
    pub fn push_str(&mut self, s: &str) {
        self.room(s.len());
        self.buf.extend_from_slice(s.as_bytes());
    }

    /// Takes the last character off, wiping its bytes.
    pub fn pop(&mut self) -> Option<char> {
        let c = self.as_str().chars().next_back()?;
        let at = self.buf.len() - c.len_utf8();
        self.truncate(at);
        Some(c)
    }

    /// Keeps the first `len` bytes, wiping the rest. `len` past the end,
    /// or not on a character boundary, keeps it all.
    pub fn truncate(&mut self, len: usize) {
        if len >= self.buf.len() || !self.as_str().is_char_boundary(len) {
            return;
        }
        self.buf[len..].zeroize();
        self.buf.truncate(len);
    }

    /// Empties it, wiping the whole buffer and keeping it.
    pub fn clear(&mut self) {
        self.buf.zeroize();
    }

    /// Replaces what it holds with `s`.
    pub fn set(&mut self, s: &str) {
        self.clear();
        self.push_str(s);
    }

    /// The bytes the buffer has room for.
    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    /// The text.
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf).unwrap_or("")
    }

    /// Another one holding the same text, in its own buffer of the same
    /// size, wiped in its turn.
    pub fn duplicate(&self) -> Text {
        let mut t = Text::with_room(self.buf.capacity());
        t.push_str(self.as_str());
        t
    }
}

impl Default for Text {
    fn default() -> Self {
        Text::new()
    }
}

impl Deref for Text {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<str> for Text {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for Text {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Drop for Text {
    fn drop(&mut self) {
        self.buf.zeroize();
    }
}

impl Zeroize for Text {
    fn zeroize(&mut self) {
        self.buf.zeroize();
    }
}

/// A copy is another [`Text`], wiped in its turn.
impl Clone for Text {
    fn clone(&self) -> Self {
        self.duplicate()
    }
}

/// Never prints what it holds.
impl fmt::Debug for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Text({} bytes)", self.buf.len())
    }
}

impl fmt::Write for Text {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s);
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        self.push(c);
        Ok(())
    }
}

impl PartialEq for Text {
    fn eq(&self, other: &Text) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for Text {}

impl PartialEq<str> for Text {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for Text {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl Hash for Text {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl PartialOrd for Text {
    fn partial_cmp(&self, other: &Text) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Text {
    fn cmp(&self, other: &Text) -> core::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl FromIterator<char> for Text {
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Text {
        let mut t = Text::new();
        t.extend(iter);
        t
    }
}

impl<'a> FromIterator<&'a str> for Text {
    fn from_iter<I: IntoIterator<Item = &'a str>>(iter: I) -> Text {
        let mut t = Text::new();
        t.extend(iter);
        t
    }
}

impl Extend<char> for Text {
    fn extend<I: IntoIterator<Item = char>>(&mut self, iter: I) {
        for c in iter {
            self.push(c);
        }
    }
}

impl<'a> Extend<&'a str> for Text {
    fn extend<I: IntoIterator<Item = &'a str>>(&mut self, iter: I) {
        for s in iter {
            self.push_str(s);
        }
    }
}
