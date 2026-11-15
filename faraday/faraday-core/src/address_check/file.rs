//! Check an address's **From a file** (`docs/DESIGN.md` §5, Check an
//! address): the text files in Files, one read for its addresses. One
//! address is checked as typed; several are listed, each checked against
//! the first 100 receive and change addresses of every loaded wallet and
//! key account, which are derived once for the whole list and not again
//! for each address.

use std::collections::HashMap;

use super::AddressCheckState;
use crate::address_find::{self, Account, Look};
use crate::item::Item;
use crate::strings::EN;
use crate::wallet::FileKind;
use crate::{Action, Faraday};
use faraday_bip::bitcoin::ScriptBuf;
use faraday_bip::bitcoin::secp256k1::Secp256k1;
use faraday_crypto::work::Compute;
use faraday_text::Text;

/// The most a file may be and still be read: 64 KiB.
const FILE_MAX: usize = 64 * 1024;

/// How far into each chain a listed address is looked for, as a typed
/// address is.
const DEPTH: u32 = 100;

/// One address of a file, listed with its answer.
pub(crate) struct Row {
    /// The address as the file wrote it.
    pub(crate) address: Text,
    /// What was found, or why the address was not searched.
    pub(crate) line: Text,
    /// Where the look that found it or missed it is kept while another
    /// row is shown; `None` for a row that was not searched, or while it
    /// is the one shown.
    look: Option<Look>,
}

impl Row {
    /// Whether a press on the row shows an answer.
    pub(crate) fn opens(&self) -> bool {
        self.look.is_some()
    }
}

/// The addresses a file holds, listed.
pub(crate) struct Listed {
    /// How many different addresses the file holds.
    total: usize,
    /// The first of them.
    rows: Vec<Row>,
    /// The row whose answer the page shows now.
    shown: Option<usize>,
}

impl Listed {
    /// How many addresses the file holds, past those listed too.
    pub(crate) fn total(&self) -> usize {
        self.total
    }

    /// The rows listed.
    pub(crate) fn rows(&self) -> &[Row] {
        &self.rows
    }
}

impl AddressCheckState {
    /// Where the button that opened the menu of files is.
    pub(crate) fn files_at(&self) -> Option<(i32, i32, i32, i32)> {
        self.files_at
    }

    /// The addresses of a file, listed, when it held several.
    pub(crate) fn list(&self) -> Option<&Listed> {
        self.list.as_ref()
    }

    /// What the list says, a line each: its count, then each address
    /// with its answer; empty when no list is shown.
    pub fn list_lines(&self) -> Vec<Text> {
        let Some(l) = self.list.as_ref().filter(|l| l.shown.is_none()) else {
            return Vec::new();
        };
        let mut out = vec![Text::fill(
            EN.address_check_read_count,
            &[&Text::num(l.rows.len() as u64), &Text::num(l.total as u64)],
        )];
        for r in &l.rows {
            let mut one = r.address.clone();
            one.push_str(": ");
            one.push_str(&r.line);
            out.push(one);
        }
        out
    }

    /// Whether a row's answer is shown in the list's place.
    pub(crate) fn row_shown(&self) -> bool {
        self.list.as_ref().is_some_and(|l| l.shown.is_some())
    }
}

/// Whether `item` is one the menu offers: a text file, a wallet
/// description to be named, or what is not text to be said so.
fn offered(item: &Item) -> bool {
    matches!(
        item.kind,
        FileKind::Text | FileKind::Wallet | FileKind::Other
    )
}

/// The files the menu offers, newest first, each as its place in the
/// Inbox and its name. A lookup.
pub(crate) fn files(app: &Faraday) -> Vec<(usize, Text)> {
    app.inbox
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, f)| offered(f))
        .map(|(i, f)| (i, f.name.clone()))
        .collect()
}

/// Opens the menu of files under the button that asked for it.
pub(crate) fn open_menu(app: &mut Faraday) {
    let at = app
        .pointer
        .rect_of(Action::AddressCheck(super::AddressPress::Files))
        .map(|r| (r.x, r.y, r.w, r.h));
    if files(app).is_empty() {
        return;
    }
    if let Some(s) = app.tools.address_check.state.as_mut() {
        s.files_at = at;
        app.sheet = Some(crate::Sheet::AddressFiles);
    }
}

/// Costs what the pasted-text classifier, the address finder and the
/// searches cost: the first 100 receive and change addresses of each
/// wallet and key account, derived once. The file at place `at` of the
/// Inbox, read: one address is checked as typed, several are listed, and
/// a file that is not text, a wallet description or a secret is refused.
pub(crate) fn read(app: &mut Faraday, c: &Compute, at: usize) {
    app.sheet = None;
    let Some(item) = app.inbox.get(at) else {
        return;
    };
    if item.kind == FileKind::Wallet {
        return super::refuse(app, Text::of(EN.address_check_descriptor));
    }
    let bytes: &[u8] = &item.bytes;
    if bytes.is_empty() || bytes.len() > FILE_MAX {
        return super::refuse(app, Text::of(EN.address_check_not_text));
    }
    let text = match std::str::from_utf8(bytes) {
        Ok(t) if !t.trim().is_empty() => Text::of(t),
        _ => return super::refuse(app, Text::of(EN.address_check_not_text)),
    };
    if let Some(why) = crate::scan_key::pasted_secret(c, text.as_str(), false) {
        if let Some(s) = app.tools.address_check.state.as_mut() {
            s.forget();
            s.typed.clear();
            s.refusal = Some(Text::of(why));
        }
        return;
    }
    let found = faraday_codec::address_text::find(text.as_str());
    match found.addresses.as_slice() {
        [] => super::refuse(app, Text::of(EN.address_check_none_in_file)),
        [one] => super::check_text(app, c, one),
        many => list(app, c, many, found.total),
    }
}

/// Costs a key derivation for each standard account of a loaded key and
/// two address derivations for each index to 100 of each wallet and
/// account, once. `addresses` listed, each with its answer.
fn list(app: &mut Faraday, c: &Compute, addresses: &[String], total: usize) {
    let accounts = address_find::unmade(c, &app.session);
    let wallets = &app.session.wallets;
    if wallets.len() + accounts.len() == 0 {
        if let Some(s) = app.tools.address_check.state.as_mut() {
            s.forget();
            s.load_first = true;
        }
        return;
    }
    let secp = Secp256k1::verification_only();
    let mut owned: HashMap<ScriptBuf, (usize, bool, u32)> = HashMap::new();
    let policies = wallets
        .iter()
        .map(|w| w.policy())
        .chain(accounts.iter().map(|a| &a.policy));
    for (place, policy) in policies.enumerate() {
        for index in 0..=DEPTH {
            for change in [false, true] {
                if let Some(s) = address_find::script(c, &secp, policy, change, index) {
                    owned.entry(s).or_insert((place, change, index));
                }
            }
        }
    }
    let rows: Vec<Row> = addresses
        .iter()
        .filter_map(|text| row(app, text, &accounts, &owned))
        .collect();
    if let Some(s) = app.tools.address_check.state.as_mut() {
        s.forget();
        s.typed.clear();
        s.list = Some(Listed {
            total,
            rows,
            shown: None,
        });
    }
}

/// One address of the file with its answer, from the scripts the wallets
/// and accounts own; `None` for text that is no address.
fn row(
    app: &Faraday,
    text: &str,
    accounts: &[Account],
    owned: &HashMap<ScriptBuf, (usize, bool, u32)>,
) -> Option<Row> {
    let address = Text::of(text);
    let addr = match app.address_read(text)? {
        Ok(addr) => addr,
        Err(why) => {
            return Some(Row {
                address,
                line: why,
                look: None,
            });
        }
    };
    let target = addr.script_pubkey();
    let mut shown = Text::new();
    let _ = std::fmt::Write::write_fmt(&mut shown, format_args!("{addr}"));
    let mut look = Look::new(
        (shown, target.clone()),
        &app.session.wallets,
        accounts.to_vec(),
    );
    if let Some(&(place, change, index)) = owned.get(&target) {
        look.found = Some((place, change, index));
        look.settle(&app.session);
    }
    let line = line_of(&look);
    Some(Row {
        address,
        line,
        look: Some(look),
    })
}

/// What a row says of its look: the Result line and, for a find, its
/// place.
fn line_of(look: &Look) -> Text {
    let (answer, _) = address_find::answer(look);
    let mut line = answer.head.clone();
    if look.found.is_some()
        && let Some(place) = answer.rows.get(1).and_then(|r| r.lines.first())
    {
        line.push_str(" · ");
        line.push_str(place);
    }
    line
}

/// Shows the answer of row `i` in the list's place.
pub(crate) fn show_row(app: &mut Faraday, i: usize) {
    let Some(s) = app.tools.address_check.state.as_mut() else {
        return;
    };
    let Some(list) = s.list.as_mut() else {
        return;
    };
    let Some(look) = list.rows.get_mut(i).and_then(|r| r.look.take()) else {
        return;
    };
    list.shown = Some(i);
    s.look = Some(look);
    s.refresh();
}

/// Back from a row's answer to the list; whether it was shown.
pub(crate) fn back_to_list(app: &mut Faraday) -> bool {
    let Some(s) = app.tools.address_check.state.as_mut() else {
        return false;
    };
    let Some(list) = s.list.as_mut() else {
        return false;
    };
    let Some(i) = list.shown.take() else {
        return false;
    };
    if let Some(row) = list.rows.get_mut(i)
        && let Some(look) = s.look.take()
    {
        row.line = line_of(&look);
        row.look = Some(look);
    }
    s.answer = None;
    s.ways.clear();
    true
}
