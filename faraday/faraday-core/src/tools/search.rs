//! One search over what Find a tool can open: the tiles of Tools, then the
//! cards of Settings, then the guide's glossary words. Tools' Find uses it; Home's search bar takes its
//! results from it too.

use crate::catalog::{TILES, matches};
use crate::screens::catalog::cards_matching;

/// What a typed search found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Hit {
    /// A tile of Tools: its index in `TILES`.
    Tile(usize),
    /// A card of Settings: its place among Settings' sections.
    Card(u8),
    /// A word of the guide's glossary: its place in
    /// `faraday_guide::glossary`.
    Word(u8),
}

/// What `find` finds: the tiles, then the cards of Settings, then the
/// glossary words that hold every word typed; nothing when nothing is typed.
pub(crate) fn hits(find: &str) -> Vec<Hit> {
    if find.trim().is_empty() {
        return Vec::new();
    }
    let tiles = TILES
        .iter()
        .enumerate()
        .filter(|(_, t)| matches(t, find))
        .map(|(i, _)| Hit::Tile(i));
    let settings = cards_matching(find).into_iter().map(Hit::Card);
    let words = faraday_guide::glossary()
        .iter()
        .enumerate()
        .filter(|(_, w)| crate::catalog::words_in(w.word, find))
        .filter_map(|(i, _)| u8::try_from(i).ok())
        .map(Hit::Word);
    tiles.chain(settings).chain(words).collect()
}
