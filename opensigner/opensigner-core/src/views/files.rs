//! Files (`docs/DESIGN.md` §5 Menu): the list a shell that can list what
//! it holds answers a file request with, one value row per file, in the
//! order the shell gave them.
//!
//! Nothing here reads a key or a session: the screen is the shell's
//! listing and the two things a person needs to tell one file from
//! another, its name and when it was written.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_ui::layout::Node;
use osk_ui::screens::{self, Row};
use osk_ui::widgets::{Icon, Tone};

use crate::strings::{Strings, fill1};
use crate::{FileListing, OpenSigner, ids, text};

/// Bytes in a kilobyte, as a file manager counts them.
const KB: u64 = 1024;

impl OpenSigner {
    /// §5 Menu, "Files": one value row per file, the name above and the
    /// date below, newest first as the shell ordered them. A tap is the
    /// answer; an empty list says there is nothing to read.
    pub(crate) fn view_files(&self, list: &FileListing) -> Node {
        let s = self.strings();
        let rows: Vec<Row> = if list.entries.is_empty() {
            vec![Row::Dimmed {
                icon: Some(Icon::File),
                label: String::from(s.files_none),
                reason: list.place.clone(),
            }]
        } else {
            list.entries
                .iter()
                .enumerate()
                // The name is what a person chooses by, so it is the
                // row's value line, the bold one; the date is its caption.
                .map(|(i, e)| Row::Value {
                    id: ids::at(ids::FILES_ROW_BASE, i),
                    label: match e.modified {
                        Some(secs) => text::file_time(secs),
                        None => file_size(e.size, s),
                    },
                    value: e.name.clone(),
                    tone: Tone::Text,
                })
                .collect()
        };
        self.with_chrome(Some(ids::BACK), |c| {
            screens::menu(c, s.files_title, None, rows, Vec::new())
        })
    }
}

/// What a file's row says where the shell has no clock to date it by:
/// its size, in whole kilobytes once it has one.
fn file_size(size: u64, s: &Strings) -> String {
    if size < KB {
        return fill1(s.files_bytes, &alloc::format!("{size}"));
    }
    fill1(s.files_kb, &alloc::format!("{}", size / KB))
}
