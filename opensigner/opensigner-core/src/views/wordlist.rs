//! Tools › Word list (`docs/DESIGN.md` §5): the language Choice, the
//! search Entry, and one Record per word with a pager that walks the
//! list.
//!
//! Nothing on these screens is a secret: the BIP-39 lists are published,
//! so there is no eye and nothing is masked.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_bip::bip39::Language;
use osk_ui::components;
use osk_ui::layout::Node;
use osk_ui::screens::{self, Above, Action, Candidates, Chrome, Entry, FactRow, Pager, Record};
use osk_ui::widgets::Tone;
use osk_ui::widgets::keyboard::{self, ALL_KEYS, KeyboardKind};

use crate::load::EntryList;
use crate::strings::Strings;
use crate::views::load::language_items;
use crate::wordlist::{SearchBy, Step, WordList};
use crate::{OpenSigner, ids, text};

/// The name of a notation, for the mode row and its Choice.
pub(crate) fn search_by_name(by: SearchBy, s: &Strings) -> &'static str {
    match by {
        SearchBy::Word => s.wordlist_by_word,
        SearchBy::Number => s.wordlist_by_number,
        SearchBy::Binary => s.wordlist_by_binary,
        SearchBy::Hex => s.wordlist_by_hex,
    }
}

/// The eleven bits of a wordlist index.
pub(crate) fn binary(index: u16) -> String {
    (0..11)
        .rev()
        .map(|bit| if index >> bit & 1 == 1 { '1' } else { '0' })
        .collect()
}

/// The three hex digits of a wordlist index.
pub(crate) fn hex(index: u16) -> String {
    alloc::format!("{index:03x}")
}

impl OpenSigner {
    pub(crate) fn view_word_list(&self, w: &WordList) -> Node {
        self.with_chrome(Some(ids::BACK), |c| match w.step() {
            Step::Language => self.wordlist_language(c, w),
            Step::Search => self.wordlist_search(c, w),
        })
    }

    /// §5 Choice, "Which language?": the ten wordlists, English checked.
    fn wordlist_language(&self, c: &Chrome<'_>, w: &WordList) -> Node {
        let s = self.strings();
        screens::choice(
            c,
            s.load_language_title,
            language_items(w.language(), ids::WORDLIST_LANG_BASE),
            Action::new(ids::WORDLIST_LANG_CONTINUE, s.action_continue),
        )
    }

    /// §5 Entry, "Word list": the mode row and the Browse row above the
    /// field, then the field and the keyboard the notation is typed on.
    fn wordlist_search(&self, c: &Chrome<'_>, w: &WordList) -> Node {
        let s = self.strings();
        let mode = || FactRow::mode(ids::WORDLIST_BY, s.wordlist_by, search_by_name(w.by(), s));
        if w.by() == SearchBy::Word {
            return self.wordlist_by_word(c, w, mode());
        }
        let (kind, error) = match w.by() {
            SearchBy::Number => (
                KeyboardKind::Pin,
                w.out_of_range().then_some(s.wordlist_out_of_range),
            ),
            SearchBy::Binary => (KeyboardKind::Binary, None),
            SearchBy::Hex => (
                KeyboardKind::Hex,
                w.out_of_range().then_some(s.wordlist_hex_out_of_range),
            ),
            SearchBy::Word => unreachable!("drawn above"),
        };
        screens::entry(
            c,
            Entry {
                title: s.wordlist_search_title,
                value: String::from(w.typed()),
                mono: true,
                above: Above::Fact(mode()),
                candidates: None,
                words: None,
                eye: None,
                keyboard: (ids::WORDLIST_KEYBOARD, kind),
                // §4.3: ✓ is dead until what is typed names a word. The
                // search by word has no ✓ (§16.118); these are the
                // number, binary and hex pads.
                enabled: if w.index().is_some() {
                    ALL_KEYS
                } else {
                    ALL_KEYS | keyboard::DONE_DISABLED
                },
                error: error.map(String::from),
            },
        )
    }

    /// The same Entry with the language's own keyboard and candidate
    /// strip, which is the Load wizard's word entry doing the search.
    fn wordlist_by_word(&self, c: &Chrome<'_>, w: &WordList, mode: FactRow) -> Node {
        let s = self.strings();
        let lang = w.language();
        let search = w.search();
        let candidates: Vec<String> = search
            .candidates()
            .map(|i| text::shown_word(EntryList::Bip39(lang), i))
            .collect();
        let accepting = search.accepting();
        let one_char = lang.max_display_chars() == 1;
        let more = search.more_candidates(osk_ui::tokens::candidates(c.class(), one_char));
        let error =
            (candidates.is_empty() && !search.prefix().is_empty() && !search.awaiting_tone())
                .then(|| String::from(s.wordlist_no_match));
        screens::entry(
            c,
            Entry {
                title: s.wordlist_search_title,
                value: text::shown_prefix(EntryList::Bip39(lang), search.prefix()),
                mono: true,
                above: Above::Fact(mode),
                candidates: Some(Candidates {
                    first: ids::WORDLIST_CANDIDATES,
                    second: ids::WORDLIST_CANDIDATES_2,
                    words: candidates,
                    accepting,
                    more,
                    one_char,
                    selected: search.selected_cell(),
                }),
                words: None,
                eye: None,
                keyboard: (ids::WORDLIST_KEYBOARD, search.keyboard()),
                enabled: search.enabled_keys(),
                error,
            },
        )
    }

    /// §5 Record: one word of one list, and the pager that walks the
    /// list from it.
    pub(crate) fn view_word(&self, lang: Language, index: u16) -> Node {
        let s = self.strings();
        let word = text::shown_word(EntryList::Bip39(lang), index);
        self.with_chrome(Some(ids::BACK), |c| {
            let rows = vec![
                // §16.126: a word's number, index, bits and hex are all
                // read character by character.
                components::Record::mono(s.word_number_row, alloc::format!("{}", index + 1)),
                components::Record::mono(s.word_index_row, alloc::format!("{index}")),
                components::Record::mono(s.word_binary_row, binary(index)),
                components::Record::mono(s.word_hex_row, hex(index)),
                components::Record::text(
                    s.word_language_row,
                    text::language_name(lang),
                    Tone::Text,
                ),
            ];
            // §16.126: the page is named by the word itself, which is
            // copied character by character, in the app bar and in the
            // pager alike.
            screens::record_mono_title(
                c,
                Record {
                    title: &word,
                    key: None,
                    network: components::Network::Mainnet,
                    rows,
                    warnings: Vec::new(),
                    pager: Some(Pager {
                        prev: ids::WORD_PREV,
                        next: ids::WORD_NEXT,
                        label: word.clone(),
                        at_start: index == 0,
                        at_end: usize::from(index) + 1 == crate::wordlist::WORDS,
                    }),
                    action: None,
                },
            )
        })
    }
}
