//! Search in an open vault's items and in the TOTP codes (`docs/VAULT.md`
//! §12, `docs/TOTP.md` §5.3): Material's search bar over the list, which
//! keeps the items or codes whose title holds what is typed, live, case
//! not counted.
//!
//! A title is the public name a person gave the record: an entry's title,
//! a wallet's name, a wallet share's wallet, a GPG key's user ID. Nothing
//! secret is read to search: a note's title is its first line, a secret,
//! so Notes has no search bar; a Bitcoin key is named by its fingerprint,
//! worked out rather than kept, and Secure Boot holds one set, so neither
//! has one either.

use super::{CATEGORIES, Focus, VaultArea, Vaults};
use crate::Screen;
use faraday_keys::vault::{RecordView, Records, VaultId, gpg_field};
use faraday_vault::records::{field, kind};

/// Which search bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchIn {
    /// The items column of an open vault's contents.
    Items,
    /// The TOTP codes screen.
    Codes,
}

/// What a search bar's presses do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchAct {
    /// Typing goes to the bar.
    Focus(SearchIn),
    /// The bar's × empties it; typing still goes to it.
    Clear(SearchIn),
    /// Material's search view closed by Back: the bar is left with what
    /// it holds, as Done and Escape leave it.
    Leave(SearchIn),
}

/// What each search bar holds. Changed only through
/// [`Vaults::set_query`], which drops what the open vaults found
/// ([`Found`]).
#[derive(Default)]
pub struct Searches {
    items: String,
    codes: String,
}

/// What one open vault's searches find, kept on the vault beside what is
/// read of its records (`super::Kept`): for each kind of
/// [`CATEGORIES`], the places of its records the items list shows, in
/// record order; and the places of its entries the codes screen shows.
///
/// Invariant: a vault's `Found` is what its records and the two queries
/// give as they are now. Every change to the records goes through
/// [`Vaults::edit`] and every change to a query through
/// [`Vaults::set_query`], and each drops it; the next ask finds again,
/// and every frame asks before it draws ([`Vaults::warm`] from
/// `before_frame`), so drawing only looks it up.
/// `a_search_follows_an_entry_added_and_a_query_changed` changes both to
/// test it.
#[derive(Default)]
pub(crate) struct Found {
    items: [Vec<usize>; CATEGORIES.len()],
    codes: Vec<usize>,
}

/// Whether kind `category` of [`CATEGORIES`] has a search bar: Wallets,
/// Entries and GPG keys, whose records carry a public title. A lookup.
pub(crate) fn searchable(category: usize) -> bool {
    matches!(category, 1 | 2 | 4)
}

/// Record `r`'s title as the search reads it: public, never a secret.
fn title(r: RecordView<'_>) -> Option<&str> {
    match r.kind() {
        kind::ENTRY => r.public_text(field::TITLE),
        kind::WALLET => r.public_text(field::WALLET_NAME),
        kind::WALLET_SHARE => r.public_text(field::SHARE_WALLET_NAME),
        kind::GPG => r.public_text(gpg_field::USER_ID),
        _ => None,
    }
}

/// Whether `r`'s title holds `query`, already lower case; an empty query
/// is held by every record. Costs the title in lower case.
fn holds(r: RecordView<'_>, query: &str) -> bool {
    query.is_empty() || title(r).is_some_and(|t| t.to_lowercase().contains(query))
}

impl Found {
    /// Costs a pass over the records' titles. What `records` show under
    /// the queries `s`.
    fn of(records: &Records<'_>, s: &Searches) -> Found {
        let items_q = s.items.to_lowercase();
        let codes_q = s.codes.to_lowercase();
        let items = std::array::from_fn(|k| {
            let (_, kinds) = CATEGORIES[k];
            let q = if searchable(k) { items_q.as_str() } else { "" };
            records
                .iter()
                .filter(|(_, r)| kinds.contains(&r.kind()) && holds(*r, q))
                .map(|(i, _)| i)
                .collect()
        });
        let codes = records
            .of(kind::ENTRY)
            .filter(|(_, r)| holds(*r, &codes_q))
            .map(|(i, _)| i)
            .collect();
        Found { items, codes }
    }
}

impl Vaults {
    /// What search bar `s` holds.
    pub fn query(&self, s: SearchIn) -> &str {
        match s {
            SearchIn::Items => &self.search.items,
            SearchIn::Codes => &self.search.codes,
        }
    }

    /// Search bar `s` now holds `text`: what every open vault found is
    /// dropped ([`Found`]), and each kind's list shows its first item.
    pub(crate) fn set_query(&mut self, s: SearchIn, text: String) {
        match s {
            SearchIn::Items => self.search.items = text,
            SearchIn::Codes => self.search.codes = text,
        }
        for o in &mut self.open {
            o.read.found = std::cell::OnceCell::new();
        }
        if s == SearchIn::Items {
            self.item = [0; 7];
        }
    }

    /// Costs what [`Found::of`] costs the first time after a change, a
    /// lookup after. What open vault `id`'s searches find.
    fn found(&self, id: VaultId) -> Option<&Found> {
        let o = self.open.iter().find(|o| o.id == id)?;
        Some(
            o.read
                .found
                .get_or_init(|| Found::of(&self.store.records(id), &self.search)),
        )
    }

    /// Costs what [`Vaults::found`] costs. The places of open vault
    /// `id`'s records the items list of kind `category` shows, in record
    /// order: those its search bar's query holds.
    pub fn shown(&self, id: VaultId, category: usize) -> &[usize] {
        self.found(id)
            .and_then(|f| f.items.get(category))
            .map_or(&[], Vec::as_slice)
    }

    /// Costs what [`Vaults::found`] costs. The places of open vault
    /// `id`'s entries the codes screen shows.
    pub fn shown_codes(&self, id: VaultId) -> &[usize] {
        self.found(id).map_or(&[], |f| f.codes.as_slice())
    }

    /// Costs what [`Vaults::shown`] costs. The place in the slot of the
    /// item selected in the kind shown, among those the list shows.
    pub fn selected_place(&self) -> Option<usize> {
        let id = self.current_id()?;
        let shown = self.shown(id, self.category);
        let k = self.item[self.category].min(shown.len().saturating_sub(1));
        shown.get(k).copied()
    }

    /// Costs what [`Vaults::shown`] costs. Selects the record at `place`
    /// in the kind shown, where the list shows it.
    pub(crate) fn select_place(&mut self, place: usize) {
        let Some(id) = self.current_id() else {
            return;
        };
        if let Some(k) = self
            .shown(id, self.category)
            .iter()
            .position(|&i| i == place)
        {
            self.item[self.category] = k;
        }
    }

    /// Costs what [`Vaults::shown`] costs. Where the record at `place`
    /// stands in the kind shown's list, the items search emptied first
    /// when it is not among those shown: a code's row opens its entry.
    pub(crate) fn place_shown(&mut self, place: usize) -> usize {
        let Some(id) = self.current_id() else {
            return 0;
        };
        if !self.shown(id, self.category).contains(&place) {
            self.set_query(SearchIn::Items, String::new());
        }
        let shown = self.shown(id, self.category);
        shown.iter().position(|&i| i == place).unwrap_or(0)
    }
}

impl VaultArea {
    /// A search bar pressed: typing goes to it, or its × empties it.
    pub(crate) fn search_act(&mut self, a: SearchAct) {
        match a {
            SearchAct::Focus(s) => self.state.focus = Some(Focus::Search(s)),
            SearchAct::Clear(s) => {
                self.state.set_query(s, String::new());
                self.state.focus = Some(Focus::Search(s));
            }
            SearchAct::Leave(_) => self.state.focus = None,
        }
    }

    /// Typing on `screen` while a search bar has it: a character goes on
    /// the end, Backspace takes the last, Enter and Escape leave the bar
    /// with what it holds. Returns whether the key was taken.
    pub(crate) fn search_key(&mut self, screen: Screen, key: faraday_shell_api::Key) -> bool {
        use faraday_shell_api::Key as K;
        let Some(Focus::Search(s)) = self.state.focus else {
            return false;
        };
        if !matches!(screen, Screen::VaultContents | Screen::Codes) {
            return false;
        }
        let mut text = self.state.query(s).to_string();
        match key {
            K::Char(c) if !c.is_control() => text.push(c),
            K::Backspace => {
                text.pop();
            }
            K::Enter | K::Escape => {
                self.state.focus = None;
                return true;
            }
            _ => return false,
        }
        self.state.set_query(s, text);
        true
    }
}
