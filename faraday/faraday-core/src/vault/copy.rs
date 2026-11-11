//! **Copy to another vault** (`docs/VAULT.md` §7.2): an item of the open
//! vault, copied into another open vault. The sheet lists the other open
//! vaults, each with whether it can take the item, worked out once when
//! the sheet opens; the copy is `vault_push_in`'s, so a share of a held
//! set is asked at its own dialog. A move is a copy
//! and then the removal from the original, which waits for that dialog's
//! Continue. A vault that is locked, or one not yet made, is reached
//! through its unlock or its creation, with the item held from the press
//! ([`CopyWait`]); it is dropped when the original closes or changes.

use faraday_crypto::work::Compute;
use faraday_keys::vault::{Staged, VaultId};
use faraday_vault::file::CEILING;
use faraday_vault::records::{field, kind};
use std::ops::Not;

use crate::strings::{EN, fill};
use crate::vault::{VaultAction, Work};
use crate::{Faraday, Screen, Sheet};

/// Where a row sends the item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dest {
    /// An open vault.
    Open(VaultId),
    /// A vault this session knows and is locked, by its identifier.
    Locked([u8; 32]),
    /// A vault made for it.
    New,
}

/// One vault on the sheet.
#[derive(Debug)]
pub struct CopyRow {
    /// Where it goes.
    pub to: Dest,
    /// Its name, or what pressing it does.
    pub name: String,
    /// Why it cannot take the item; `None` when it can.
    pub why: Option<String>,
}

/// The sheet while it is up.
#[derive(Debug)]
pub struct CopyTo {
    /// The vault the item is in.
    pub from: VaultId,
    /// The item's place in that vault.
    pub record: usize,
    /// That vault's version when the sheet was made: the place is only
    /// good while the vault is as it was.
    version: u64,
    /// The other open vaults, in the order they are open.
    pub rows: Vec<CopyRow>,
    /// The row waiting at the dialog (a move's always; a copy's when it
    /// leaves data behind).
    pub ask: Option<usize>,
    /// A move: the item leaves the vault it is in.
    pub mv: bool,
    /// Its vault's name, for the move's dialog and toast.
    pub from_name: String,
    /// Whether the item carries data Faraday does not read.
    pub unread: bool,
    /// What a seed carries that the person may leave out of the copy: the
    /// field's number and whether it is kept. Only the fields the seed has,
    /// each kept at first.
    pub keeps: Vec<(u8, bool)>,
}

/// The fields a seed (key record) has that the copy may leave out, each
/// kept.
fn keeps_of(app: &Faraday, from: VaultId, record: usize) -> Vec<(u8, bool)> {
    let Some(r) = app.vaults.records(from).get(record) else {
        return Vec::new();
    };
    if r.kind() != kind::KEY {
        return Vec::new();
    }
    [field::KEY_CODEX32, field::KEY_CHILD]
        .into_iter()
        .filter(|&n| r.has(n))
        .map(|n| (n, true))
        .collect()
}

/// The fields the choices leave out of the copy.
fn without(keeps: &[(u8, bool)]) -> Vec<u8> {
    keeps.iter().filter(|k| !k.1).map(|k| k.0).collect()
}

impl CopyTo {
    /// The fields the choices leave out of the copy.
    pub fn left_out(&self) -> Vec<u8> {
        without(&self.keeps)
    }

    /// The name of the vault the dialog is about.
    pub fn asked(&self) -> Option<&str> {
        self.ask
            .and_then(|n| self.rows.get(n))
            .map(|r| r.name.as_str())
    }
}

/// The sheet of vaults the selected item can go to, if one is open.
pub(crate) fn sheet(app: &Faraday) -> Option<&CopyTo> {
    app.vaults.state.copy_to.as_deref()
}

/// Costs a document write for each other open vault. Key records are
/// kept worked out from their vault's unlock, so a fingerprint is worked
/// out here only for one not kept. Opens the sheet for the selected item of the
/// vault shown, with whether each other open vault takes it. A move is
/// not offered from a read-only vault.
pub(crate) fn open(app: &mut Faraday, mv: bool) {
    let Some(record) = app.vault_selected_index() else {
        return;
    };
    let Some(from) = app.vaults.current_id() else {
        return;
    };
    if mv && app.vaults.store().read_only(from).is_some() {
        return;
    }
    let c = app.compute();
    let store = app.vaults.store();
    let keeps = keeps_of(app, from, record);
    let Some(staged) = store.stage(from, record, &without(&keeps)) else {
        return;
    };
    let from_name = app
        .vaults
        .open
        .iter()
        .find(|o| o.id == from)
        .map_or_else(String::new, |o| o.label(store));
    let mut rows: Vec<CopyRow> = app
        .vaults
        .open
        .iter()
        .filter(|o| o.id != from)
        .map(|o| CopyRow {
            to: Dest::Open(o.id),
            name: o.label(store),
            why: judge(app, &c, (from, record), (o.id, &from_name), mv, &staged),
        })
        .collect();
    // Each vault known and locked: its file in Files is what unlocks it.
    let files = app.vault_files();
    for s in &app.vaults.state.summaries {
        if app.vaults.open.iter().any(|o| o.vault_id == s.id) {
            continue;
        }
        let at_hand = files.iter().any(|f| {
            f.open.is_none()
                && app
                    .vaults
                    .summary_of_file(f.sha)
                    .is_some_and(|x| x.id == s.id)
        });
        let name = if s.name.is_empty() { &s.file } else { &s.name };
        rows.push(CopyRow {
            to: Dest::Locked(s.id),
            name: fill(EN.vaults_unlock, &[name]),
            why: (!at_hand).then(|| EN.vault_copy_no_file.to_string()),
        });
    }
    rows.push(CopyRow {
        to: Dest::New,
        name: EN.vault_copy_new.to_string(),
        why: fits(app, &c, &staged)
            .not()
            .then(|| EN.vaults_slot_too_small.to_string()),
    });
    app.vaults.state.copy_to = Some(Box::new(CopyTo {
        from,
        record,
        version: store.version(from),
        rows,
        ask: None,
        mv,
        from_name,
        unread: store.carries_unread(from, record),
        keeps,
    }));
    app.sheet = Some(Sheet::CopyTo);
}

/// Costs a document write for each open vault and each size tried. Choice
/// `k` pressed: kept or left out, and every row judged again on what will
/// be written.
pub(crate) fn keep(app: &mut Faraday, k: usize) {
    let Some(s) = app.vaults.state.copy_to.as_mut() else {
        return;
    };
    if s.ask.is_some() {
        return;
    }
    let Some(t) = s.keeps.get_mut(k) else {
        return;
    };
    t.1 = !t.1;
    let (from, record, mv) = (s.from, s.record, s.mv);
    let from_name = s.from_name.clone();
    let left = without(&s.keeps);
    let c = app.compute();
    let Some(staged) = app.vaults.store().stage(from, record, &left) else {
        return;
    };
    let whys: Vec<Option<Option<String>>> =
        app.vaults
            .state
            .copy_to
            .as_ref()
            .map_or_else(Vec::new, |s| {
                s.rows
                    .iter()
                    .map(|r| match r.to {
                        Dest::Open(to) => Some(judge(
                            app,
                            &c,
                            (from, record),
                            (to, &from_name),
                            mv,
                            &staged,
                        )),
                        Dest::New => Some(
                            fits(app, &c, &staged)
                                .not()
                                .then(|| EN.vaults_slot_too_small.to_string()),
                        ),
                        Dest::Locked(_) => None,
                    })
                    .collect()
            });
    if let Some(s) = app.vaults.state.copy_to.as_mut() {
        for (r, why) in s.rows.iter_mut().zip(whys) {
            if let Some(why) = why {
                r.why = why;
            }
        }
    }
}

/// Costs a document write. The key records of `to` are kept worked out
/// from its unlock (`Vaults::key_identity` costs only for one not kept).
/// Why open vault `to` cannot take
/// the item at place `record` of vault `from`, or `None` when it can.
fn judge(
    app: &Faraday,
    c: &Compute,
    (from, record): (VaultId, usize),
    (to, from_name): (VaultId, &str),
    mv: bool,
    staged: &Staged,
) -> Option<String> {
    let store = app.vaults.store();
    if store.read_only(to).is_some() {
        Some(EN.vault_error_read_only.to_string())
    } else if store.holds_same(from, record, to, |r| {
        // Worked out once for each key record, kept by its id, which is a
        // hash of its words and passphrase: a record changed has another
        // id, so cannot answer with the old identity.
        app.vaults.key_identity(c, app.session.network(), r)
    }) {
        Some(EN.vault_copy_holds.to_string())
    } else if let Some(why) = app.vault_refuses(c, to, &[], &[staged]) {
        Some(why)
    } else if mv && app.vaults.state.moved_ring(from, to) {
        Some(fill(EN.vault_move_ring, &[from_name]))
    } else {
        None
    }
}

/// Costs a document write. Whether a new vault holds the item. The vault's
/// name is not typed yet, so it is judged with a name of the longest the
/// document takes ([`LABEL_MAX`](super::area::LABEL_MAX) bytes) made of the
/// one-byte character the document writes longest (`&`, five bytes): a
/// vault holds the item whatever name is typed.
fn fits(app: &Faraday, c: &Compute, staged: &Staged) -> bool {
    let store = app.vaults.store();
    let longest = "&".repeat(super::area::LABEL_MAX);
    store.fits_new(c, &longest, CEILING, &[staged])
}

/// Row `n` pressed: the copy is made, or, where the entry carries data
/// Faraday does not read, the dialog says so first. A dimmed row takes
/// no press.
pub(crate) fn pick(app: &mut Faraday, n: usize) {
    let Some(s) = app.vaults.state.copy_to.as_ref() else {
        return;
    };
    let Some(row) = s.rows.get(n).filter(|r| r.why.is_none()) else {
        return;
    };
    if s.ask.is_some() {
        return;
    }
    match row.to {
        Dest::Open(_) => {}
        Dest::Locked(id) => return unlock_for(app, id),
        Dest::New => return new_for(app),
    }
    if s.mv || s.unread {
        if let Some(s) = app.vaults.state.copy_to.as_mut() {
            s.ask = Some(n);
        }
        return;
    }
    make(app, n);
}

/// The dialog's **Copy**.
pub(crate) fn go(app: &mut Faraday) {
    if let Some(n) = app.vaults.state.copy_to.as_ref().and_then(|s| s.ask) {
        make(app, n);
    }
}

/// Costs what `vault_put_in` costs. Copies the item to row `n`'s vault
/// and closes the sheet. The secrets are copied inside `faraday-keys`.
fn make(app: &mut Faraday, n: usize) {
    let Some(s) = app.vaults.state.copy_to.take() else {
        return;
    };
    if app.sheet == Some(Sheet::CopyTo) {
        app.sheet = None;
    }
    let Some(row) = s.rows.get(n) else {
        return;
    };
    let Dest::Open(to) = row.to else {
        return;
    };
    let Some(v) = app.vaults.open.iter().position(|o| o.id == to) else {
        return;
    };
    // The item is held by its place: only the original as it was when the
    // sheet was made has the same item there.
    if app.vaults.store().version(s.from) != s.version {
        return;
    }
    let Some(staged) = app.vaults.store().stage(s.from, s.record, &s.left_out()) else {
        return;
    };
    if !s.mv {
        let said = fill(EN.vault_copied_to, &[&row.name]);
        app.vault_push_in(v, staged, &said);
        return;
    }
    let said = fill(EN.vault_moved_to, &[&row.name]);
    // The key's id as it arrives: what the target stores, after the ticks.
    let arrived = super::moved_key_id(staged.view());
    if !app.vault_push_in(v, staged, &said) {
        return;
    }
    // Asked a question first: the item leaves when the answer is Continue.
    if app.sheet == Some(Sheet::SaveAlso) {
        app.vault_hold_move(to, s.from, s.record);
    } else {
        app.vault_moved(s.from, s.record, to, arrived);
    }
}

/// The sheet closed: nothing is copied.
pub(crate) fn close(app: &mut Faraday) {
    app.vaults.state.copy_to = None;
    if app.sheet == Some(Sheet::CopyTo) {
        app.sheet = None;
    }
}

/// The sheet gone another way (a sheet over it, a lock) is **Cancel**; an
/// item held for an unlock or a creation that is no longer on its way, or
/// whose original closed or changed, is dropped: leaving the form or the
/// Unlock screen, whichever way, ends the wait. True when it dropped
/// something.
pub(crate) fn settle(app: &mut Faraday) -> bool {
    let mut dropped = false;
    if let Some(w) = app.vaults.state.copy_wait.as_ref() {
        let store = app.vaults.store();
        let same =
            app.vaults.open.iter().any(|o| o.id == w.from) && store.version(w.from) == w.version;
        let working = app.vaults.working;
        let on_its_way = if w.to.is_none() {
            app.screen == Screen::CreateVault || working == Some(Work::Create)
        } else {
            app.screen == Screen::Unlock
                || app.sheet == Some(Sheet::OlderCopy)
                || working == Some(Work::Unlock)
        };
        if !same || !on_its_way {
            app.vaults.state.copy_wait = None;
            dropped = true;
        }
    }
    if app.vaults.state.copy_to.is_some() && app.sheet != Some(Sheet::CopyTo) {
        app.vaults.state.copy_to = None;
        return true;
    }
    dropped
}

/// The item held for a vault to unlock or make, while it waits: the
/// original is held by its place, which is only good while the original
/// is as it was ([`OpenVaults::version`](faraday_keys::vault::OpenVaults::version)),
/// so any edit, seal or lock of it ends the wait.
#[derive(Debug)]
pub struct CopyWait {
    /// The vault the item is in.
    from: VaultId,
    /// That vault's version when the row was pressed.
    version: u64,
    /// The item's place in it.
    record: usize,
    /// A move.
    mv: bool,
    /// The original's name, for the dialog.
    from_name: String,
    /// Whether the item carries data Faraday does not read.
    unread: bool,
    /// The choices of what a seed keeps, as at the press.
    keeps: Vec<(u8, bool)>,
    /// The identifier of the vault it goes to; `None` for a new vault
    /// until it is made.
    to: Option<[u8; 32]>,
    /// The vault is made for it.
    new: bool,
    /// The contents as they were: the kind shown, whether it was chosen,
    /// the item in each kind and whether the page was open.
    view: (usize, bool, [usize; 7], bool),
}

/// Takes the sheet's state into a wait for vault `to` (a new vault when
/// `None`) and closes the sheet.
fn hold(app: &mut Faraday, to: Option<[u8; 32]>) -> bool {
    let Some(s) = app.vaults.state.copy_to.take() else {
        return false;
    };
    if app.sheet == Some(Sheet::CopyTo) {
        app.sheet = None;
    }
    let st = &app.vaults.state;
    app.vaults.state.copy_wait = Some(Box::new(CopyWait {
        from: s.from,
        version: app.vaults.store().version(s.from),
        record: s.record,
        mv: s.mv,
        from_name: s.from_name,
        unread: s.unread,
        keeps: s.keeps,
        to,
        new: to.is_none(),
        view: (st.category, st.category_chosen, st.item, st.item_open),
    }));
    true
}

/// The locked vault `id` pressed: its unlock, the item waiting. Nothing is
/// held if its file is no longer at hand.
fn unlock_for(app: &mut Faraday, id: [u8; 32]) {
    let files = app.vault_files();
    let Some(at) = files.iter().position(|f| {
        f.open.is_none()
            && app
                .vaults
                .summary_of_file(f.sha)
                .is_some_and(|s| s.id == id)
    }) else {
        return;
    };
    if hold(app, Some(id)) {
        app.vault_act(VaultAction::OpenFrom(at, Screen::VaultContents));
        if app.screen != Screen::Unlock {
            app.vaults.state.copy_wait = None;
        }
    }
}

/// Costs a document write. **New vault with this item** pressed: Create a
/// vault, when the item fits one.
fn new_for(app: &mut Faraday) {
    let Some((from, record, left)) = app
        .vaults
        .state
        .copy_to
        .as_ref()
        .map(|s| (s.from, s.record, s.left_out()))
    else {
        return;
    };
    let c = app.compute();
    let Some(staged) = app.vaults.store().stage(from, record, &left) else {
        return;
    };
    if !fits(app, &c, &staged) {
        return;
    }
    // A fresh Create drops any wait, so the wait is taken after it: it is
    // then the wait of the form just made.
    app.vault_act(VaultAction::Create);
    hold(app, None);
}

/// Costs what listing the files and starting an unlock cost. The vault was
/// made: the item goes to it, so it is unlocked at once with
/// the passphrase just typed twice, and the item put in when it is open.
/// False when nothing waits for a new vault.
pub(crate) fn made(
    app: &mut Faraday,
    name: &str,
    id: [u8; 16],
    (passphrase, parts): (&str, &mut Option<faraday_keys::UnlockParts>),
) -> bool {
    let Some(w) = app
        .vaults
        .state
        .copy_wait
        .as_mut()
        .filter(|w| w.new && w.to.is_none())
    else {
        return false;
    };
    let mut vault_id = [0u8; 32];
    for (k, b) in id.iter().enumerate() {
        vault_id[2 * k] = b"0123456789abcdef"[usize::from(b >> 4)];
        vault_id[2 * k + 1] = b"0123456789abcdef"[usize::from(b & 15)];
    }
    w.to = Some(vault_id);
    let at = app.vault_files().iter().position(|f| f.name == name);
    let Some(at) = at.filter(|_| app.may_load_keys()) else {
        app.vaults.state.copy_wait = None;
        return false;
    };
    app.vaults.drop_create();
    app.vaults
        .unlock_with(at, passphrase, Screen::VaultContents);
    app.vaults.set_made_parts(parts.take());
    app.screen = Screen::Unlock;
    crate::vault::stretch::defer(app, Work::Unlock);
    true
}

/// The wait ended without an item: the older-copy dialog's **Keep locked**.
pub(crate) fn drop_wait(app: &mut Faraday) {
    app.vaults.state.copy_wait = None;
}

/// Costs a document write and what the copy or move costs. A vault was
/// just unlocked: if it is the one an item waits for, the item is judged
/// as for an open vault, and the copy is made, or the move's dialog
/// put up, or a toast says why nothing is made.
pub(crate) fn unlocked(app: &mut Faraday) {
    let Some(w) = app.vaults.state.copy_wait.take() else {
        return;
    };
    let Some((to, name)) = app
        .vaults
        .open
        .last()
        .filter(|o| Some(o.vault_id) == w.to)
        .map(|o| (o.id, o.label(app.vaults.store())))
    else {
        return;
    };
    // The original must be as it was when the row was pressed: it holds
    // the item by its place.
    let Some(current) = app.vaults.open.iter().position(|o| o.id == w.from) else {
        return;
    };
    let refused = |app: &mut Faraday, why: &str| {
        let line = if w.mv {
            EN.vault_move_refused
        } else {
            EN.vault_copy_refused
        };
        app.toast(&fill(line, &[&name, why]));
    };
    if app.vaults.store().version(w.from) != w.version {
        return refused(app, EN.vault_copy_changed);
    }
    if app.sheet.is_some() {
        return refused(app, EN.vault_copy_sheet_up);
    }
    let (category, chosen, item, item_open) = w.view;
    app.vaults
        .restore_view(current, category, chosen, item, item_open);
    app.screen = Screen::VaultContents;
    let c = app.compute();
    let Some(staged) = app
        .vaults
        .store()
        .stage(w.from, w.record, &without(&w.keeps))
    else {
        return;
    };
    if let Some(why) = judge(
        app,
        &c,
        (w.from, w.record),
        (to, &w.from_name),
        w.mv,
        &staged,
    ) {
        let line = if w.mv {
            EN.vault_move_refused
        } else {
            EN.vault_copy_refused
        };
        app.toast(&fill(line, &[&name, &why]));
        return;
    }
    app.vaults.state.copy_to = Some(Box::new(CopyTo {
        from: w.from,
        record: w.record,
        version: w.version,
        rows: vec![CopyRow {
            to: Dest::Open(to),
            name,
            why: None,
        }],
        ask: None,
        mv: w.mv,
        from_name: w.from_name,
        unread: w.unread,
        keeps: w.keeps,
    }));
    app.sheet = Some(Sheet::CopyTo);
    if w.mv || w.unread {
        if let Some(s) = app.vaults.state.copy_to.as_mut() {
            s.ask = Some(0);
        }
    } else {
        make(app, 0);
    }
}
