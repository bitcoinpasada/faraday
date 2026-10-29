//! **How Faraday does it** in the Starter (`docs/CRYPTO-LEARN.md` §8.4):
//! which pages perform a computing action, and their blocks.

use crate::{Page, Sheet, Starter};

impl Starter {
    /// The name the blocks find the page on show by
    /// (`faraday_learn::Block::screens`): `starter:` and the page's
    /// task, where the page performs a computing action. The Start page
    /// only while the BitLocker recovery key is offered. A lookup.
    fn how_page(&self) -> Option<&'static str> {
        match self.page {
            Page::Mode => Some("starter:write"),
            Page::Confirm => Some("starter:confirm"),
            Page::Writing => Some("starter:writing"),
            Page::Done => Some("starter:done"),
            Page::Start if self.bitlocker_offered() => Some("starter:start"),
            _ => None,
        }
    }

    /// The blocks of the computing actions the page on show performs, in
    /// the manifest's order; none while the held desktop app shows. A
    /// lookup.
    pub fn how_blocks(&self) -> Vec<&'static faraday_learn::Block> {
        if self.showing || self.showing_demo() {
            return Vec::new();
        }
        self.how_page()
            .map_or_else(Vec::new, faraday_learn::how::for_screen)
    }

    /// Whether the page on show has blocks: the top app bar then carries
    /// **How Faraday does it**. A lookup.
    pub fn how_offered(&self) -> bool {
        !self.showing
            && !self.showing_demo()
            && self.how_page().is_some_and(faraday_learn::how::offered)
    }

    /// **How Faraday does it** pressed: the sheet opens; pressed again
    /// or with the sheet up, it closes.
    pub(crate) fn how_press(&mut self) {
        self.sheet = match self.sheet {
            None if self.how_offered() => Some(Sheet::How),
            Some(Sheet::How) => None,
            other => other,
        };
    }
}
