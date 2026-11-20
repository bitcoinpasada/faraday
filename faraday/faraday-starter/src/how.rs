//! **Cryptography** in the Starter (`docs/CRYPTO-LEARN.md` §8.4): the
//! blocks of the computing actions the Starter performs, which the
//! Guide destination's Cryptography row opens.

use crate::{Sheet, Starter};

/// The names the blocks find the Starter's pages by
/// (`faraday_guide::Block::screens`): `starter:` and the page's task,
/// for each page that performs a computing action.
const PAGES: [&str; 5] = [
    "starter:write",
    "starter:confirm",
    "starter:writing",
    "starter:done",
    "starter:start",
];

impl Starter {
    /// The blocks of every computing action the Starter performs, in the
    /// order of the pages' lists, each once. A lookup.
    pub fn how_blocks(&self) -> Vec<&'static faraday_guide::Block> {
        let mut out: Vec<&'static faraday_guide::Block> = Vec::new();
        for page in PAGES {
            for block in faraday_guide::how::screen_blocks(page) {
                if !out.iter().any(|b| b.id == block.id) {
                    out.push(block);
                }
            }
        }
        out
    }

    /// The Guide destination's **Cryptography** row pressed: the sheet
    /// opens; pressed with the sheet up, it closes.
    pub(crate) fn how_press(&mut self) {
        self.sheet = match self.sheet {
            None => Some(Sheet::How),
            Some(Sheet::How) => None,
            other => other,
        };
    }
}
