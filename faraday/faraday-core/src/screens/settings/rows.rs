//! Every row of Settings as an entry Find a tool can match and open
//! (`tools/search.rs`): its label, the everyday words that find it and the
//! section it lives in. A row added to a section is added here.

use super::{Part, part_title, parts};
use crate::strings::EN;

/// One row of Settings.
pub(crate) struct Row {
    /// What the row shows.
    pub label: &'static str,
    /// Everyday words that find it.
    pub words: &'static str,
    /// The section it is in, an index into the sections ([`parts`]).
    pub part: u8,
}

const fn row(label: &'static str, words: &'static str, part: Part) -> Row {
    Row {
        label,
        words,
        part: part as u8,
    }
}

/// Every row, in the order Settings shows them.
pub(crate) const ROWS: &[Row] = &[
    row(EN.settings_mode, EN.settings_find_mode, Part::Mode),
    row(EN.settings_lock, EN.settings_find_lock, Part::Session),
    row(
        EN.settings_lock_after,
        EN.settings_find_lock_after,
        Part::Session,
    ),
    row(
        EN.settings_power_off,
        EN.settings_find_power_off_after,
        Part::Session,
    ),
    row(
        EN.settings_power_off_button,
        EN.settings_find_power_off,
        Part::Session,
    ),
    row(
        EN.settings_remove_every,
        EN.settings_find_remove_keys,
        Part::Session,
    ),
    row(EN.settings_time, EN.settings_find_time, Part::Time),
    row(EN.tz_row_time_zone, EN.settings_find_time_zone, Part::Time),
    row(
        EN.own_randomness,
        EN.settings_find_randomness,
        Part::Randomness,
    ),
    row(EN.ui_theme_row, EN.settings_find_theme, Part::Appearance),
    row(
        EN.ui_contrast_row,
        EN.settings_find_contrast,
        Part::Appearance,
    ),
    row(
        EN.settings_text_size,
        EN.settings_find_text_size,
        Part::Appearance,
    ),
    row(
        EN.settings_reduce_motion,
        EN.settings_find_reduce_motion,
        Part::Appearance,
    ),
    row(
        EN.settings_rotation,
        EN.settings_find_rotation,
        Part::Display,
    ),
    row(EN.settings_scale, EN.settings_find_scale, Part::Display),
    row(
        EN.settings_amounts_signed,
        EN.settings_find_amounts,
        Part::Remembered,
    ),
    row(
        EN.settings_signatures_made,
        EN.settings_find_signatures,
        Part::Remembered,
    ),
    row(EN.settings_about, EN.settings_find_about, Part::About),
];

impl Row {
    /// The name of the section the row is in.
    pub(crate) fn section(&self) -> &'static str {
        parts()
            .get(usize::from(self.part))
            .map_or("", |&p| part_title(p))
    }

    /// Whether the row is in Mode's section, which opens its sheet on a
    /// compact window rather than a page.
    pub(crate) fn is_mode(&self) -> bool {
        self.part == Part::Mode as u8
    }

    /// Whether every word typed is in the row's label, words or section.
    pub(crate) fn matches(&self, find: &str) -> bool {
        crate::catalog::words_in(
            &format!("{} {} {}", self.label, self.words, self.section()),
            find,
        )
    }
}
