//! A file in Faraday and on the stick shown, the same file: the same name
//! (as FAT compares names) and the same bytes. Files shows its row green
//! with a check in both cards (decision 306).
//!
//! It is known when the bytes were last seen alike, never worked out while
//! drawing: when a file is copied in (the Inbox's bytes are the stick's),
//! when a write was read back and compared, and, for a file already on
//! both, by the shell hashing the stick's file and Faraday comparing it
//! with its own copy. Only names and sizes are kept, no hash.

use faraday_text::Text;

/// One file known to be the same on the stick `stick`.
pub(super) struct Mark {
    pub(super) stick: Text,
    pub(super) name: Text,
    pub(super) size: u64,
    /// A row For the stick may show it: false for a file written from the
    /// Outbox, which a later file of the same name and size is not.
    pub(super) out_row: bool,
}

/// The files known the same, and the stick files already sent to be hashed.
#[derive(Default)]
pub(crate) struct SameFiles {
    pub(super) marks: Vec<Mark>,
    pub(super) asked: Vec<(Text, Text, u64)>,
}

impl SameFiles {
    pub(super) fn put(&mut self, stick: &str, name: &str, size: u64, out_row: bool) {
        self.marks
            .retain(|m| !(*m.stick == *stick && m.name.eq_ignore_ascii_case(name)));
        self.marks.push(Mark {
            stick: Text::of(stick),
            name: Text::of(name),
            size,
            out_row,
        });
    }

    pub(super) fn find(&self, stick: &str, name: &str) -> Option<&Mark> {
        self.marks
            .iter()
            .find(|m| *m.stick == *stick && m.name.eq_ignore_ascii_case(name))
    }

    /// Forgets the sticks no longer listed.
    pub(crate) fn keep(&mut self, ids: &[Text]) {
        self.marks.retain(|m| ids.contains(&m.stick));
        self.asked.retain(|(s, ..)| ids.contains(s));
    }
}

pub(super) fn is_settings(name: &str) -> bool {
    crate::stick::settings::is_file(name)
}
