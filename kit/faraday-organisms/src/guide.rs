//! The guide organism (09 §3.3), as far as the search shows its results: under a section label
//! the guide's words that match what is typed, each a list item whose press opens the guide at
//! its definition, or a line saying none matches. Tools' Find and Home's search both draw it.
//!
//! Only this part of the guide's page is built: the guide's contents, its pages and its blocks
//! are not (their source is `screens/sheets.rs` and `how.rs`, mapped to `guide.rs` and
//! `guide_block.rs`), so `GuidePageModel` holds the search results alone.

use faraday_atoms::{Pressed, Shown, Ui};
use faraday_molecules::row::{self, RowModel};
use faraday_tokens::dims::{BODY_M, GRID, HOME_RESULTS_LINE, TITLE_S};
use faraday_tokens::palette::Role;

/// The words the search found in the guide.
pub struct GuidePageModel<'s, A> {
    /// The section label over the words.
    pub label: Shown<'s>,
    /// One row for each word found: its headline the word, its press the guide at its
    /// definition.
    pub words: Vec<RowModel<'s, A>>,
    /// The line shown when nothing at all matched (a tool, a card or a word): `Some` only then.
    pub none: Option<Shown<'s>>,
}

/// Draws the results from (x, y), `w` wide. Returns the height used: none where no word was
/// found and the page found something else.
pub fn draw<A: Pressed>(
    m: &GuidePageModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    if let Some(none) = &m.none {
        ui.text_as(x, y, BODY_M, Role::OnSurfaceVariant, none.as_str());
        return ui.advance_as(BODY_M, HOME_RESULTS_LINE);
    }
    if m.words.is_empty() {
        return 0.0;
    }
    let mut cy = y;
    ui.showing(&m.label, |ui| {
        ui.text_as(x, cy, TITLE_S, Role::OnSurfaceVariant, m.label.as_str());
    });
    cy += ui.line_as(TITLE_S) + GRID;
    for word in &m.words {
        cy += row::draw(word, ui, (x, cy, w));
    }
    cy - y
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::Offered;
    use faraday_atoms::ui::testing::{P, at_rest, canvas};
    use faraday_atoms::{MotionStore, Theme};
    use faraday_draw::canvas::InkKind;

    fn word(headline: &'static str, press: u8) -> RowModel<'static, P> {
        RowModel {
            headline: Shown::plain(headline),
            supporting: Shown::plain(""),
            trailing: None,
            offered: Offered::Yes(P(press)),
            chosen: false,
            whole: false,
            chevron: false,
        }
    }

    /// The text lines `m` draws, and the height it takes.
    fn drawn(m: &GuidePageModel<'static, P>) -> (f32, Vec<String>) {
        let mut c = canvas(160);
        let mut store = MotionStore::new();
        let pointer = at_rest(None);
        let mut ui = Ui::new(&mut c, Theme::default(), 1.0, &mut store, &pointer, true);
        ui.clear(Role::Surface);
        let h = draw(m, &mut ui, (16.0, 16.0, 300.0));
        let lines = ui
            .finish()
            .ink
            .expect("the record is on")
            .0
            .into_iter()
            .filter_map(|i| match i.kind {
                InkKind::Text { text, .. } => Some(text.to_string()),
                _ => None,
            })
            .collect();
        (h, lines)
    }

    /// Invariant: A word typed in the search shows the guide's matching words, each as a row under the section label, and a search that matches nothing shows the one line that says so.
    /// Regression: The words found are left off the results, or the no-match line is drawn over them or left off an empty search.
    /// Owner: kit
    #[test]
    fn a_typed_word_shows_its_matching_guide_rows_and_no_match_shows_the_line() {
        let found = GuidePageModel {
            label: Shown::plain("Words"),
            words: vec![word("Passphrase", 1), word("Seed phrase", 2)],
            none: None,
        };
        let (h, lines) = drawn(&found);
        assert!(h > 0.0);
        for want in ["Words", "Passphrase", "Seed phrase"] {
            assert!(lines.iter().any(|l| l == want), "{want}: {lines:?}");
        }
        assert!(!lines.iter().any(|l| l == "No match"), "{lines:?}");

        let none = GuidePageModel {
            label: Shown::plain("Words"),
            words: Vec::new(),
            none: Some(Shown::plain("No match")),
        };
        let (h, lines) = drawn(&none);
        assert!(h > 0.0);
        assert_eq!(lines, ["No match"]);
    }
}
