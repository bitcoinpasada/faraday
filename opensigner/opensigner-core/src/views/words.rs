//! The Words screen (`docs/DESIGN.md` §5 Words), shared by the Create
//! wizard and the Backup flow: one page of the words inside the secret
//! panel, the pager under it on a class that pages, the eye in the app
//! bar, and "Numbers" beside the primary action.
//!
//! The panel keeps one rectangle masked and revealed and with the
//! wordlist numbers on and off (§4.10), so nothing moves under the
//! finger that reveals it. Nothing here says what the words are for: an
//! orange-outlined panel is a secret and needs no line under it (§2.1).
//!
//! The same screen draws the steel rows of `docs/PLANNING.md` §8.2
//! item 6: the word's place in the wordlist as four digits and its first
//! four letters in capitals beside the word, which is what a numbered
//! plate and a letter punch take.

use alloc::string::String;
use alloc::vec::Vec;

use osk_bip::bip39::Language;
use osk_ui::components::secrets::WordWidth;
use osk_ui::geom::SizeClass;
use osk_ui::layout::Node;
use osk_ui::screens::{self, Action, Chrome, Pager, Words};

use crate::load::EntryList;
use crate::strings::Strings;
use crate::{ids, strings, text};

/// Words on one page of a `Small` panel: six is what 358 dp shows at a
/// size a person can copy onto paper. A secret panel never pages its
/// content, so this is also the rule that keeps one page inside one
/// panel.
pub(crate) const PAGE_WORDS: usize = 6;

/// Words on one page: as many as the panel holds at this class, which
/// is where [`osk_ui::tokens`] keeps the number.
pub(crate) fn per_page(class: SizeClass, width_dp: f32, lang: Language) -> usize {
    per_page_list(class, width_dp, EntryList::Bip39(lang))
}

/// The same over any list, which is what a SLIP-39 share's words need.
pub(crate) fn per_page_list(class: SizeClass, width_dp: f32, list: EntryList) -> usize {
    per_page_of(
        class,
        panel_width_dp(class, width_dp),
        text::word_width(list),
        false,
    )
}

/// The same for the steel rows, which are wider and so may page where
/// the words alone do not.
pub(crate) fn steel_per_page(class: SizeClass, width_dp: f32, lang: Language) -> usize {
    per_page_of(
        class,
        panel_width_dp(class, width_dp),
        text::steel_width(lang),
        true,
    )
}

/// The width in dp the panel is drawn across: the pane, capped on
/// `wide` the way every column is (`docs/DESIGN.md` §3).
fn panel_width_dp(class: SizeClass, pane_dp: f32) -> f32 {
    if class == SizeClass::Wide {
        pane_dp.min(osk_ui::tokens::COLUMN_MAX_WIDTH + 2.0 * osk_ui::tokens::PAD)
    } else {
        pane_dp
    }
}

fn per_page_of(class: SizeClass, width_dp: f32, width: WordWidth, steel: bool) -> usize {
    screens::words::per_page(
        class,
        osk_ui::components::secrets::panel_room(width_dp),
        width,
        false,
        steel,
    )
}

/// What the Words screen shows: the words themselves, or the steel rows
/// built from them.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rows {
    /// The words, with their wordlist numbers when "Numbers" is on.
    Words {
        /// Whether each word's place in the wordlist is beside it.
        numbers: bool,
    },
    /// `0001 · ABAN · abandon`, one row per word.
    Steel,
}

/// Everything the screen draws that is not the words themselves.
pub(crate) struct WordsScreen<'a> {
    /// The app bar's title, which names the thing: "Words", "Numbers
    /// for steel", "Part 1 of 3".
    pub title: String,
    /// The list the words are from.
    pub list: EntryList,
    /// The key's words, as wordlist indices.
    pub indices: &'a [u16],
    /// Whether the panel is showing.
    pub revealed: bool,
    /// Which rows the panel draws.
    pub rows: Rows,
    /// The page the pager is on.
    pub page: u8,
    /// The share of the eye's reveal still to run.
    pub remaining: Option<f32>,
    /// The action between "Numbers" and the primary, where a class has
    /// one: "Print template" on `wide`.
    pub extra: Option<Action>,
    /// The primary action.
    pub action: Action,
}

/// §5 Words: the panel, its pager, the eye, "Numbers" and the primary.
/// The primary leaves the screen; the pager walks the pages a class
/// holds.
pub(crate) fn words_screen(c: &Chrome<'_>, w: WordsScreen<'_>, s: &Strings) -> Node {
    let class = c.class();
    let steel = w.rows == Rows::Steel;
    let numbers = matches!(w.rows, Rows::Words { numbers: true });
    let all: Vec<String> = w
        .indices
        .iter()
        .map(|&i| {
            if steel {
                text::steel_row(w.list.language().unwrap_or(Language::English), i)
            } else {
                text::shown_word(w.list, i)
            }
        })
        .collect();
    let room = osk_ui::components::secrets::panel_room(c.column().width_dp);
    let width = if steel {
        text::steel_width(w.list.language().unwrap_or(Language::English))
    } else {
        text::word_width(w.list)
    };
    let pages = screens::words::pages(class, room, width, numbers, steel, &all);
    let page = usize::from(w.page).min(pages.len().saturating_sub(1));
    let (first, words) = pages
        .get(page)
        .map_or((1, Vec::new()), |(f, w)| (*f, w.clone()));
    let last = first + words.len().saturating_sub(1);
    // "Numbers" puts each word's place in the wordlist beside it, under
    // the same mask (§4.10). The page's own slice of them.
    let places = numbers.then(|| &w.indices[first - 1..first - 1 + words.len()]);
    // §5 Words: "the title is 'Words', the pager's label the page's
    // name, 'Words 1 to 6'". The pager names what it walks to; the title
    // names the thing, on every class and whether or not it pages.
    let title = w.title;
    let pager = (pages.len() > 1).then(|| Pager {
        prev: ids::WORDS_PREV,
        next: ids::WORDS_NEXT,
        label: strings::fill(
            s.words_range_title,
            &[&alloc::format!("{first}"), &alloc::format!("{last}")],
        ),
        at_start: page == 0,
        at_end: page + 1 >= pages.len(),
    });
    screens::words(
        c,
        Words {
            title: &title,
            words: &words,
            first,
            indices: places,
            width,
            revealed: w.revealed,
            panel: ids::CREATE_REVEAL,
            eye: Some(ids::SECRET_EYE),
            remaining: w.remaining,
            pager,
            steel,
            // §4.13: "a label that does not fit is shortened
            // ('Numbers'), never shrunk" — and half a 268 dp panel is
            // the one width the full label does not fit.
            numbers_action: (!steel).then(|| {
                Action::new(
                    ids::WORDS_NUMBERS,
                    if class == SizeClass::Small {
                        s.words_numbers_short
                    } else {
                        s.words_numbers_show
                    },
                )
            }),
            extra: w.extra,
            action: w.action,
        },
    )
}
