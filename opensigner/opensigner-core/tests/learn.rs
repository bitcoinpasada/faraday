//! Learn: the list of pages behind the Home tile, and what a reader
//! finds on a page.

mod common;

use common::{Harness, PANEL, TINY};
use opensigner_core::{ScreenKind, ids, strings};
use osk_shell_api::DisplayInfo;

/// The Home tile that opens Learn.
const LEARN_TILE: usize = 4;

/// The pages, in the order the list shows them.
fn pages() -> [&'static strings::LearnPage; strings::LEARN_PAGES] {
    strings::EN.learn_pages()
}

fn open_learn(display: DisplayInfo) -> Harness {
    let mut h = Harness::new(display);
    h.tap(ids::at(ids::HOME_TILE_BASE, LEARN_TILE));
    h
}

/// Learn is reachable from Home with nothing loaded, and lists every
/// page by name.
#[test]
fn the_learn_tile_opens_the_list_of_pages() {
    let mut h = open_learn(PANEL);
    assert_eq!(h.app.screen(), ScreenKind::Learn);
    let labels = h.app.labels();
    for page in pages() {
        assert!(
            labels.iter().any(|l| l == page.title),
            "{:?} is not a row of the list: {labels:?}",
            page.title
        );
    }
    for (i, page) in pages().into_iter().enumerate() {
        assert!(
            h.app.reveal(ids::at(ids::LEARN_ROW_BASE, i)).is_some(),
            "the row for {:?} has no target",
            page.title
        );
    }
}

/// A row opens its page, and the page states its title and what its
/// first section is about.
#[test]
fn a_row_opens_the_page_it_names() {
    for (i, page) in pages().into_iter().enumerate() {
        let mut h = open_learn(PANEL);
        h.tap(ids::at(ids::LEARN_ROW_BASE, i));
        assert_eq!(h.app.screen(), ScreenKind::LearnPage);
        let texts = h.app.texts();
        let first = page.sections[0].heading;
        for word in [page.title, first] {
            assert!(
                texts.iter().any(|t| t == word),
                "{word:?} is not on the page: {texts:?}"
            );
        }
    }
}

/// Back from a page returns to the list, and back from the list returns
/// to Home.
#[test]
fn back_walks_out_of_learn_one_step_at_a_time() {
    let mut h = open_learn(PANEL);
    h.tap(ids::at(ids::LEARN_ROW_BASE, 0));
    assert_eq!(h.app.screen(), ScreenKind::LearnPage);
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Learn);
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Home);
}

/// Nothing in Learn needs a key: every page opens on a device that has
/// never held one, and the most any page offers is the one row that
/// opens the flow it is about.
#[test]
fn learn_asks_for_nothing_and_offers_at_most_one_row() {
    for i in 0..strings::LEARN_PAGES {
        let mut h = open_learn(PANEL);
        h.tap(ids::at(ids::LEARN_ROW_BASE, i));
        assert_eq!(h.app.screen(), ScreenKind::LearnPage);
        assert!(h.app.hold_buttons().is_empty());
        let labels = h.app.labels();
        assert!(labels.len() <= 1, "Learn page {i} offers {labels:?}");
        assert_eq!(
            labels.is_empty(),
            h.app.rect_of(ids::LEARN_TRY).is_none(),
            "what Learn page {i} offers is its own last row"
        );
    }
}

/// Every page reads whole on the smallest panel: each heading and each
/// paragraph is on the screen, none of them cut off, and a page too
/// long for the panel scrolls rather than hiding its end.
#[test]
fn every_page_reads_whole_on_the_smallest_panel() {
    for display in [TINY, PANEL] {
        for (i, page) in pages().into_iter().enumerate() {
            let mut h = open_learn(display);
            h.tap(ids::at(ids::LEARN_ROW_BASE, i));
            let texts = h.app.texts();
            let cut = h.app.cut_texts();
            for section in page.sections {
                let mut expected = vec![section.heading];
                expected.extend(section.paragraphs);
                for line in expected {
                    assert!(
                        texts.iter().any(|t| t == line),
                        "{:?} is missing {line:?} at {}x{}",
                        page.title,
                        display.width,
                        display.height
                    );
                    assert!(
                        !cut.iter().any(|t| t == line),
                        "{:?} cuts {line:?} at {}x{}",
                        page.title,
                        display.width,
                        display.height
                    );
                }
            }
            // What does not fit is inside the region that scrolls, not
            // below the fold of a frame that cannot be scrolled and not
            // beside it.
            let view = h.app.rect_of(ids::SCROLL).expect("the scrolling body");
            assert!(h.app.scroll_info(ids::SCROLL).is_some());
            for r in h.app.overflow() {
                assert!(
                    r.x >= view.x && r.right() <= view.right(),
                    "{:?} strands {r:?} beside its text at {}x{}",
                    page.title,
                    display.width,
                    display.height
                );
            }
        }
    }
}

/// The glossary is looked up rather than read, so its terms run in
/// alphabetical order.
#[test]
fn the_glossary_lists_its_terms_in_alphabetical_order() {
    let glossary = pages()
        .into_iter()
        .find(|p| p.title == strings::EN.learn_glossary.title)
        .expect("the glossary is one of the pages");
    let terms: Vec<String> = glossary
        .sections
        .iter()
        .map(|s| s.heading.to_lowercase())
        .collect();
    let mut sorted = terms.clone();
    sorted.sort();
    assert_eq!(terms, sorted, "the glossary is out of order: {terms:?}");
    for section in glossary.sections {
        assert_eq!(
            section.paragraphs.len(),
            1,
            "{:?} takes more than a line",
            section.heading
        );
    }
}
