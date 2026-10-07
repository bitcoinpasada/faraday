//! Word lists: BIP-39's English list and the three EFF diceware lists,
//! every word with its number, on a sheet over the screen it was opened
//! from, so that a word a key or a passphrase is made of can be found in
//! its list and checked by hand. The words are `osk-bip`'s own lists, the
//! ones the keys and passphrases are made from.
//!
//! The sheet keeps the screen under it as it was: closing it returns to
//! New key at the card it was on, or to Create a vault's passphrases with
//! the rolls still typed.

use osk_bip::bip39::Language;
use osk_bip::diceware::List;

use crate::{Faraday, Sheet};

/// One of the lists the sheet shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordList {
    /// BIP-39, English: 2048 words, 11 bits each.
    Bip39,
    /// One of the EFF's diceware lists.
    Eff(List),
}

/// The lists, in the order the sheet offers them. Actions carry a list
/// by its place here.
pub const LISTS: [WordList; 4] = [
    WordList::Bip39,
    WordList::Eff(List::Large),
    WordList::Eff(List::Short1),
    WordList::Eff(List::Short2),
];

impl WordList {
    /// Its name on the sheet.
    pub fn name(self) -> &'static str {
        match self {
            WordList::Bip39 => "BIP-39 English",
            WordList::Eff(List::Large) => "EFF large",
            WordList::Eff(List::Short1) => "EFF short 1",
            WordList::Eff(List::Short2) => "EFF short 2",
        }
    }

    /// Its words, in order.
    pub fn words(self) -> &'static [&'static str] {
        match self {
            WordList::Bip39 => Language::English.words(),
            WordList::Eff(l) => l.words(),
        }
    }

    /// What names a word in this list besides its number: its 11 bits,
    /// highest first, for BIP-39; the dice that roll it, first die first,
    /// for an EFF list.
    pub fn code(self, index: usize) -> String {
        match self {
            WordList::Bip39 => format!("{index:011b}"),
            WordList::Eff(l) => {
                let n = l.dice_per_word();
                let mut faces = vec![b'1'; n];
                let mut rest = index;
                for f in faces.iter_mut().rev() {
                    *f = b'1' + (rest % 6) as u8;
                    rest /= 6;
                }
                String::from_utf8(faces).unwrap_or_default()
            }
        }
    }

    /// The heading over the codes.
    pub fn code_heading(self) -> &'static str {
        match self {
            WordList::Bip39 => "Bits",
            WordList::Eff(_) => "Dice",
        }
    }

    /// Its place in [`LISTS`].
    pub fn place(self) -> u8 {
        LISTS.iter().position(|l| *l == self).unwrap_or(0) as u8
    }
}

/// The sheet's state.
pub struct WordListState {
    /// The list on show.
    pub list: WordList,
    /// The word marked, by its place in the list (0-based).
    pub mark: Option<usize>,
    /// What is typed to find a word: the first word that begins with it
    /// is marked.
    pub find: String,
    /// Design units scrolled.
    pub scroll: f32,
    /// The furthest it can scroll, as last drawn.
    pub max: f32,
    /// The marked word is brought into view at the next frame.
    pub follow: bool,
}

/// What a press on the sheet does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordListAction {
    /// Open the sheet on list n of [`LISTS`], with this word marked.
    Open(u8, Option<u16>),
    /// Show list n of [`LISTS`].
    List(u8),
    /// Mark this word of the list on show.
    Mark(u16),
    /// Close the sheet, back to the screen under it.
    Close,
}

impl Faraday {
    /// One press on the sheet, or a link to it.
    pub(crate) fn wordlist_act(&mut self, a: WordListAction) {
        match a {
            WordListAction::Open(k, mark) => {
                let Some(&list) = LISTS.get(usize::from(k)) else {
                    return;
                };
                let mark = mark.map(usize::from).filter(|&m| m < list.words().len());
                self.wordlist = Some(WordListState {
                    list,
                    mark,
                    find: String::new(),
                    scroll: 0.0,
                    max: 0.0,
                    follow: mark.is_some(),
                });
                self.sheet = Some(Sheet::WordList);
            }
            WordListAction::List(k) => {
                if let (Some(w), Some(&list)) = (self.wordlist.as_mut(), LISTS.get(usize::from(k)))
                    && w.list != list
                {
                    w.list = list;
                    w.mark = None;
                    w.find.clear();
                    w.scroll = 0.0;
                }
            }
            WordListAction::Mark(i) => {
                if let Some(w) = self.wordlist.as_mut()
                    && usize::from(i) < w.list.words().len()
                {
                    w.mark = Some(usize::from(i));
                }
            }
            WordListAction::Close => self.wordlist_close(),
        }
    }

    /// Closes the sheet and forgets the word marked.
    pub(crate) fn wordlist_close(&mut self) {
        self.wordlist = None;
        if self.sheet == Some(Sheet::WordList) {
            self.sheet = None;
        }
    }

    /// Scrolls the sheet.
    pub(crate) fn wordlist_scroll(&mut self, dy: f32) {
        if let Some(w) = self.wordlist.as_mut() {
            w.scroll = (w.scroll + dy).clamp(0.0, w.max);
        }
    }

    /// Keys while the sheet is open: letters find a word, Backspace takes
    /// one back, the arrows scroll, Escape closes it. Every key is the
    /// sheet's while it is open, so none reaches the screen under it.
    pub(crate) fn wordlist_key(&mut self, key: osk_shell_api::Key) -> bool {
        use osk_shell_api::Key as K;
        if self.sheet != Some(Sheet::WordList) {
            return false;
        }
        let Some(w) = self.wordlist.as_mut() else {
            self.sheet = None;
            return true;
        };
        match key {
            K::Escape => self.wordlist_close(),
            K::Down => self.wordlist_scroll(60.0),
            K::Up => self.wordlist_scroll(-60.0),
            K::Char(c) if c.is_ascii_alphabetic() && w.find.len() < 12 => {
                w.find.push(c.to_ascii_lowercase());
                w.find_mark();
            }
            K::Backspace => {
                w.find.pop();
                w.find_mark();
            }
            _ => {}
        }
        self.dirty = true;
        true
    }
}

impl WordListState {
    /// Marks the first word that begins with what is typed.
    fn find_mark(&mut self) {
        if self.find.is_empty() {
            return;
        }
        if let Some(i) = self
            .list
            .words()
            .iter()
            .position(|w| w.starts_with(self.find.as_str()))
        {
            self.mark = Some(i);
            self.follow = true;
        }
    }
}
