//! Chunked-string planning (`docs/PLANNING.md` §13): groups of four,
//! alternating emphasis, monospace, the whole string on screen.
//!
//! A line takes as many groups as the width holds, so a wide box reads a
//! string in fewer lines than a narrow one. The planner takes the largest
//! monospace size at which the whole string fits the box, shrinking the
//! type — which puts more groups on a line — before it lets the string
//! wrap onto more of them. Only a box too small at the smallest size
//! falls back to an explicit pager.
//!
//! The planner is pure arithmetic over pixel widths so it can be tested
//! without a display.

use alloc::vec::Vec;

use crate::widgets::tokens::{self, CHUNK_THRESHOLD};

/// Number of characters per chunk.
pub const CHUNK: usize = tokens::CHUNK_GROUP;

/// Whether `text` is long enough to chunk. Only strings longer than
/// [`CHUNK_THRESHOLD`] characters are grouped: an eight-character
/// fingerprint is read as one word, an address is not.
pub fn is_chunked(text: &str) -> bool {
    text.chars().count() > CHUNK_THRESHOLD
}

/// Splits `text` into chunks of [`CHUNK`] characters (the last may be
/// shorter). A string at or under [`CHUNK_THRESHOLD`] characters is one
/// chunk. Byte ranges into `text`.
pub fn chunks(text: &str) -> Vec<(usize, usize)> {
    if !is_chunked(text) {
        return alloc::vec![(0, text.len())];
    }
    split(text)
}

/// Splits `text` into groups of [`CHUNK`] characters unconditionally.
fn split(text: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut count = 0;
    for (i, c) in text.char_indices() {
        if count == CHUNK {
            out.push((start, i));
            start = i;
            count = 0;
        }
        count += 1;
        let _ = c;
    }
    if start < text.len() || out.is_empty() {
        out.push((start, text.len()));
    }
    out
}

/// How many chunks fit per line, and how many lines that makes.
///
/// `chunk_w` is the width of a full chunk, `gap` the space between chunks,
/// all in pixels.
pub fn fit_lines(n_chunks: usize, chunk_w: i32, gap: i32, max_w: i32) -> (usize, usize) {
    let per_line = (((max_w + gap) / (chunk_w + gap).max(1)).max(1)) as usize;
    let lines = n_chunks.div_ceil(per_line).max(1);
    (per_line, lines)
}

/// The chosen presentation of a chunked string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plan {
    /// Index into the size list that was tried (0 = largest).
    pub size_index: usize,
    /// Chunks per line.
    pub per_line: usize,
    /// Lines shown at once (per page when paged).
    pub lines: usize,
    /// Chunks per page; `None` when everything fits without paging.
    pub per_page: Option<usize>,
    /// Number of pages (1 when not paged).
    pub pages: usize,
}

/// Picks the largest size at which all `n_chunks` fit in `max_w × max_h`
/// as full groups, or falls back to a pager at the smallest size.
///
/// `metrics[i]` is `(chunk_w, gap, line_h)` for size `i`, largest first.
/// `label_h` is the height reserved for the "chars a–b of n" line when
/// paging.
pub fn plan(
    n_chunks: usize,
    metrics: &[(i32, i32, i32)],
    max_w: i32,
    max_h: i32,
    label_h: i32,
) -> Plan {
    for (i, &(chunk_w, gap, line_h)) in metrics.iter().enumerate() {
        let (per_line, lines) = fit_lines(n_chunks, chunk_w, gap, max_w);
        // A group that is wider than the box is not read, it is guessed,
        // so the size has to hold one across before its lines count.
        if chunk_w <= max_w && (lines as i32) * line_h <= max_h {
            return Plan {
                size_index: i,
                per_line,
                lines,
                per_page: None,
                pages: 1,
            };
        }
    }
    // Pager at the smallest size: below the floor the grid gives way,
    // because a chunk that does not fit cannot be read at all.
    let i = metrics.len().saturating_sub(1);
    let (chunk_w, gap, line_h) = metrics.get(i).copied().unwrap_or((1, 0, 1));
    let (per_line, _) = fit_lines(n_chunks, chunk_w, gap, max_w);
    let rows = (((max_h - label_h) / line_h.max(1)).max(1)) as usize;
    let per_page = per_line * rows;
    let pages = n_chunks.div_ceil(per_page).max(1);
    Plan {
        size_index: i,
        per_line,
        lines: rows,
        per_page: Some(per_page),
        pages,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_are_groups_of_four_with_a_short_tail() {
        let t = "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh";
        let c = chunks(t);
        assert_eq!(c.len(), 11);
        assert_eq!(&t[c[0].0..c[0].1], "bc1q");
        assert_eq!(&t[c[10].0..c[10].1], "lh");
    }

    #[test]
    fn short_strings_are_never_chunked() {
        // A fingerprint is eight characters: one chunk, no groups.
        assert!(!is_chunked("73c5da0a"));
        assert_eq!(chunks("73c5da0a"), alloc::vec![(0, 8)]);
        assert_eq!(chunks("").len(), 1);
        assert_eq!(chunks("abcd").len(), 1);
        // The threshold is sixteen characters.
        assert!(!is_chunked("0123456789abcdef"));
        assert_eq!(chunks("0123456789abcdef").len(), 1);
        assert!(is_chunked("0123456789abcdefg"));
        assert_eq!(chunks("0123456789abcdefg").len(), 5);
    }
}
