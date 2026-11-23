//! Reads handwritten BIP39 words: a Rust rewrite of the Haller word and
//! detector networks (integer arithmetic, bit for bit the published logits
//! and maps) and of its dictionary decoder. See the README for the sources.
//!
//! Also reads a page: finds the words and line numbers, puts them in order and
//! chooses the form, asking what it cannot decide. No screen, camera or action
//! uses it yet. The weights are
//! compiled in; nothing is loaded at run time. The functions take no compute
//! token (`docs/COMPUTE.md`): like the QR scanner's worker, they run on a
//! worker thread of the caller's.

#![forbid(unsafe_code)]

use core::fmt;

mod clean;
mod decode;
mod detector;
mod find;
mod mathf;
mod nn;
mod order;
mod page;
mod read;
mod word;

pub use clean::{Cleaned, MAX_INK_WIDTH, clean_crop, unread_ink};
pub use decode::{Candidate, Scores, log_softmax, word_scores, word_scores_in};
pub use detector::{Maps, detector_maps};
pub use find::{Found, MAX_KERNELS, MAX_SIDE, Rect, find_words};
pub use order::MAX_NUMBER;
pub use page::{
    Crop, Extra, FIT_FLOOR, Form, LIST_MARGIN, List, MAX_CROP_PIXELS, MAX_READ,
    MAX_TOTAL_CROP_PIXELS, Number, Order, Page, PageWord, Question, bip39_counts, read_page,
    read_page_with, slip39_counts,
};
pub use read::{
    CLEAR_FREE_GAP, CLEAR_MARGIN, CLEAR_UNREAD_INK, Reading, UNCLEAR_CANDIDATES, bip39_words,
    read_crop, slip39_words,
};
pub use word::{CLASSES, Logits, frames, word_logits};

/// Rows of a word's ink map.
pub const INK_HEIGHT: usize = 32;
/// The widest ink map read: columns.
pub const MAX_WIDTH: usize = 4096;
/// Columns of the detector's input.
pub const DETECTOR_WIDTH: usize = 288;
/// Rows of the detector's input.
pub const DETECTOR_HEIGHT: usize = 384;
/// Channels of the detector's output.
pub const DETECTOR_OUTPUTS: usize = 11;
/// The decoder's beam, in log units.
pub const BEAM: f32 = 30.0;

/// Why a reading was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The input is not of the size the network takes (a frame whose side is
    /// over `MAX_SIDE` included).
    Size,
    /// The ink map is wider than `MAX_WIDTH`.
    TooWide,
    /// The word list is empty, too long, or has an empty word, a repeated
    /// word or a character other than a-z.
    WordList,
    /// A compiled-in table is inconsistent (cannot happen in a built crate).
    Internal,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Error::Size => "input of the wrong size",
            Error::TooWide => "ink map too wide",
            Error::WordList => "unusable word list",
            Error::Internal => "inconsistent tables",
        })
    }
}

impl std::error::Error for Error {}

/// Costs `word_logits`, `log_softmax` and `word_scores`; the likeliest words
/// of an ink map: logits, log-softmax and the decoder
/// with the standard beam.
pub fn read_word(ink: &[u8], width: usize) -> Result<Scores, Error> {
    let mut logits = word_logits(ink, width)?;
    log_softmax(&mut logits);
    word_scores(&logits, BEAM)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_crop;
#[cfg(test)]
mod tests_order;
#[cfg(test)]
mod tests_page;
