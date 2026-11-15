//! Buffers a sheet's text passes through, wiped when they drop: a line of
//! what a sheet prints may be part of a key.

use zeroize::Zeroize;

/// A line of text built a word at a time, in room taken once, so it never
/// regrows and leaves a copy behind; wiped when it drops.
pub struct Line(Vec<u8>);

impl Line {
    /// An empty line with room for `n` bytes.
    pub fn with_capacity(n: usize) -> Self {
        Line(Vec::with_capacity(n))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn clear(&mut self) {
        self.0.zeroize();
    }

    pub fn push_space(&mut self) {
        self.0.push(b' ');
    }

    pub fn push_str(&mut self, s: &str) {
        self.0.extend_from_slice(s.as_bytes());
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or_default()
    }
}

impl Drop for Line {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// Wipes `v`'s bytes and empties it.
pub fn bytes(v: &mut Vec<u8>) {
    v.zeroize();
}
