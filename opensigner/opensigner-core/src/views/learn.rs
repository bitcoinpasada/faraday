//! Learn (`docs/DESIGN.md` §5): the Menu that lists the pages, each
//! page as a Document, and the first run's own page, "Start here".
//!
//! Nothing here reads a key, a session or a setting: Learn is the same
//! on a device that has never held anything. §4.12 allows the prose,
//! because a Document is one of the two places a sentence belongs.
//!
//! A page whose subject has a flow ends in one row, which opens that
//! flow; "Start here" ends in one action, which opens Add.

use alloc::string::String;
use alloc::vec::Vec;

use osk_ui::layout::Node;
use osk_ui::screens::{self, Action, Row, Section};
use osk_ui::widgets::{Icon, Tone};

use crate::learn_map::Topic;
use crate::strings::LearnPage;
use crate::{OpenSigner, TryIt, ids};

/// The page's sections, as the Document draws them.
fn sections(page: &LearnPage) -> Vec<Section> {
    page.sections
        .iter()
        .map(|section| Section {
            id: None,
            heading: String::from(section.heading),
            paragraphs: section
                .paragraphs
                .iter()
                .map(|p| String::from(*p))
                .collect(),
        })
        .collect()
}

impl OpenSigner {
    /// §5 Menu, "Learn": one row per page, in the order the pages are
    /// meant to be read.
    pub(crate) fn view_learn(&self) -> Node {
        let s = self.strings();
        let rows: Vec<Row> = s
            .learn_pages()
            .iter()
            .enumerate()
            .map(|(i, page)| Row::Menu {
                id: ids::at(ids::LEARN_ROW_BASE, i),
                icon: None,
                label: String::from(page.title),
                value: None,
                tone: Tone::Text,
            })
            .collect();
        self.with_chrome(Some(ids::BACK), |c| {
            screens::menu(c, s.home_learn, None, rows, Vec::new())
        })
    }

    /// The last row of page `i`: what its subject is done with, the
    /// row's label and its mark. `None` on a page whose subject is no
    /// one flow. The association lives in [`crate::learn_map`], beside
    /// the map that sends a working screen to its page.
    pub(crate) fn learn_try(&self, i: usize) -> Option<(TryIt, &'static str, Icon)> {
        let s = self.strings();
        Topic::of(s, i)?.try_it(s)
    }

    /// §5 Document: one Learn page, its sections in order, and the row
    /// that opens the flow it is about where it has one.
    pub(crate) fn view_learn_page(&self, i: usize) -> Node {
        let s = self.strings();
        let pages = s.learn_pages();
        let i = i.min(pages.len() - 1);
        let page = pages[i];
        let row = self.learn_try(i).map(|(_, label, icon)| Row::Action {
            id: ids::LEARN_TRY,
            icon,
            label: String::from(label),
        });
        self.with_chrome(Some(ids::BACK), |c| {
            screens::document_with(c, page.title, sections(page), row, None)
        })
    }

    /// §5 Document: the page the app bar's info button opens
    /// (`docs/PLANNING.md` §16.105), at the section the map named where
    /// it named one. No "Try it" row: the person is in the flow the page
    /// is about, and the chevron is the way back to it.
    pub(crate) fn view_learn_topic(&self, i: usize, section: Option<usize>) -> Node {
        let s = self.strings();
        let pages = s.learn_pages();
        let i = i.min(pages.len() - 1);
        let page = pages[i];
        let mut sections = sections(page);
        if let Some(n) = section
            && let Some(heading) = sections.get_mut(n)
        {
            heading.id = Some(ids::LEARN_HEADING);
        }
        self.with_chrome(Some(ids::BACK), |c| {
            screens::document_with(c, page.title, sections, None, None)
        })
    }

    /// §5 Document, "Start here": the first run's page, with the one
    /// action that opens Add. The chevron is what ends the first run,
    /// so the screen has no second action saying so.
    pub(crate) fn view_start_here(&self) -> Node {
        let s = self.strings();
        let page = &s.learn_start_here;
        self.with_chrome(Some(ids::BACK), |c| {
            screens::document_with(
                c,
                page.title,
                sections(page),
                None,
                Some(Action::new(ids::START_HERE_CONTINUE, s.action_continue)),
            )
        })
    }
}
