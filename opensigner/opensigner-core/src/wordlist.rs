//! Tools › Word list: the BIP-39 list of any of the ten languages,
//! looked up by word, by number, by binary or by hex, and browsed from
//! any word.
//!
//! Nothing here is a secret. A word of the published list is public data
//! and the screens say so by carrying no eye and masking nothing.
//!
//! The search by word is the Load wizard's own word entry
//! ([`LoadWizard`]): the same keyboard per language, the same dimmed
//! keys, the same candidate strip. The tool drives one wizard that never
//! reaches a second word — the word it commits is the word it opens.

use alloc::string::String;

use osk_bip::bip39::Language;

use crate::load::LoadWizard;

/// Words in every BIP-39 list.
pub const WORDS: usize = 2048;

/// What the field is searched by (`docs/DESIGN.md` §4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchBy {
    /// The language's own keyboard and candidate strip.
    Word,
    /// The 1-based number, 1 to 2048.
    Number,
    /// The eleven bits.
    Binary,
    /// The three hex digits, `000` to `7ff`.
    Hex,
}

impl SearchBy {
    /// The four, in the order the Choice lists them.
    pub const ALL: [SearchBy; 4] = [
        SearchBy::Word,
        SearchBy::Number,
        SearchBy::Binary,
        SearchBy::Hex,
    ];

    /// Characters the field takes.
    fn max_chars(self) -> usize {
        match self {
            // The word field is the wizard's, not this one's.
            SearchBy::Word => 0,
            SearchBy::Number => 4,
            SearchBy::Binary => 11,
            SearchBy::Hex => 3,
        }
    }

    /// Whether `c` is a character of this notation.
    fn takes(self, c: char) -> bool {
        match self {
            SearchBy::Word => false,
            SearchBy::Number => c.is_ascii_digit(),
            SearchBy::Binary => c == '0' || c == '1',
            SearchBy::Hex => c.is_ascii_hexdigit(),
        }
    }
}

/// Where the tool is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Which wordlist.
    Language,
    /// The search itself.
    Search,
}

/// The word list tool's state.
pub struct WordList {
    step: Step,
    lang: Language,
    by: SearchBy,
    /// What has been typed in the number, binary or hex notations.
    typed: String,
    /// The word search, which is the Load wizard's word entry.
    search: LoadWizard,
}

impl Default for WordList {
    fn default() -> Self {
        Self::new()
    }
}

impl WordList {
    /// The tool as it opens: the language step, English checked, the
    /// search by word.
    pub fn new() -> Self {
        WordList {
            step: Step::Language,
            lang: Language::English,
            by: SearchBy::Word,
            typed: String::new(),
            search: LoadWizard::new(),
        }
    }

    /// Which step.
    pub fn step(&self) -> Step {
        self.step
    }

    /// Goes to `step`.
    pub fn go(&mut self, step: Step) {
        self.step = step;
    }

    /// The wordlist being searched.
    pub fn language(&self) -> Language {
        self.lang
    }

    /// Sets the wordlist, which starts the search over: what was typed
    /// spells nothing in another language.
    pub fn set_language(&mut self, lang: Language) {
        if lang != self.lang {
            self.lang = lang;
            self.clear();
        }
        self.search.set_language(lang);
    }

    /// What the field is searched by.
    pub fn by(&self) -> SearchBy {
        self.by
    }

    /// Sets the notation, which starts the search over.
    pub fn set_by(&mut self, by: SearchBy) {
        if by != self.by {
            self.by = by;
            self.clear();
        }
    }

    /// The word search, for the candidates and the dimmed keys.
    pub fn search(&self) -> &LoadWizard {
        &self.search
    }

    /// The word search, for the keys and the taps.
    pub fn search_mut(&mut self) -> &mut LoadWizard {
        &mut self.search
    }

    /// What has been typed in the number, binary or hex notations.
    pub fn typed(&self) -> &str {
        &self.typed
    }

    /// Empties the field and the word search.
    pub fn clear(&mut self) {
        self.typed.clear();
        self.search = LoadWizard::new();
        self.search.set_language(self.lang);
    }

    /// Types one character; ignored where the notation has no such
    /// character or the field is full.
    pub fn push(&mut self, c: char) -> bool {
        let c = c.to_ascii_lowercase();
        if !self.by.takes(c) || self.typed.chars().count() >= self.by.max_chars() {
            return false;
        }
        self.typed.push(c);
        true
    }

    /// Removes the last character.
    pub fn pop(&mut self) {
        self.typed.pop();
    }

    /// The 0-based wordlist index what is typed names, when it names one.
    /// Binary and hex are read whole, so a part of either names nothing
    /// yet.
    pub fn index(&self) -> Option<u16> {
        let text = self.typed.as_str();
        if text.is_empty() {
            return None;
        }
        match self.by {
            SearchBy::Word => None,
            SearchBy::Number => {
                let n = text.parse::<u32>().ok()?;
                (1..=WORDS as u32).contains(&n).then(|| (n - 1) as u16)
            }
            SearchBy::Binary => (text.len() == 11).then(|| u16::from_str_radix(text, 2).ok())?,
            SearchBy::Hex => {
                let n = (text.len() == 3).then(|| u16::from_str_radix(text, 16).ok())??;
                (usize::from(n) < WORDS).then_some(n)
            }
        }
    }

    /// Whether what is typed names no word of the list and cannot be
    /// continued into one, which the caption line says and ✓ follows.
    pub fn out_of_range(&self) -> bool {
        let text = self.typed.as_str();
        match self.by {
            SearchBy::Word | SearchBy::Binary => false,
            // Every digit typed after the count passes 2048 keeps it
            // past 2048, so the reason stands from the digit that
            // crossed it.
            SearchBy::Number => text
                .parse::<u32>()
                .is_ok_and(|n| n == 0 || n > WORDS as u32),
            SearchBy::Hex => {
                text.len() == 3
                    && u16::from_str_radix(text, 16).is_ok_and(|n| usize::from(n) >= WORDS)
            }
        }
    }
}
