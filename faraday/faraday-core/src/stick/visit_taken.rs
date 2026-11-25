//! Writing over a file on a stick asks first (`docs/FLOWS.md` the stick
//! visit). Before a visit's Write, the files it would write whose names
//! are already on the stick, ignoring case, bring up **Already on the
//! stick**, a row each, and Replace, Keep both or Cancel. A vault is not
//! listed there: a vault whose name a file on the stick has, where nothing
//! proves that file is a copy of it, is asked by the disk process and
//! answered by the name-taken sheet (`docs/VAULT.md` §6 step 1). The
//! settings file is Faraday's own and not listed. With nothing taken there
//! is no sheet.

use crate::drawing::View;
use crate::storage_api::OnTaken;
use crate::strings::EN;
use crate::wallet::FileKind;
use crate::{Column, Item, ReceiptFile, VisitBarAt, Writing, seed_by_name};
use faraday_crypto::work::Compute;
use faraday_draw::widgets::Icon;
use faraday_text::{Text, text};
use std::collections::BTreeSet;

use crate::compact::{buttons, scroll_sheet};
use crate::secrets;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::{Action, Faraday, Screen, Sheet, StorageCommand, sha256_of, stick};
use faraday_draw::tokens::BODY_M;
use zeroize::Zeroizing;

/// One row of the sheet: a file whose name is taken, and the name Keep
/// both writes it under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Taken {
    /// The name asked for.
    pub name: Text,
    /// The name Keep both writes it under.
    pub keep_as: Text,
}

impl Taken {
    /// The row as the sheet says it.
    pub fn line(&self) -> Text {
        Text::fill(EN.visit_taken_keep_both_n, &[&self.name, &self.keep_as])
    }
}

/// A vault's write the disk process refused for its name, waiting for the
/// person's answer (`docs/VAULT.md` §6 step 1). What the sheet says was
/// decided when the refusal arrived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NameTaken {
    /// The stick's id.
    pub(crate) stick: Text,
    /// The name the vault was to be written under.
    pub(crate) name: Text,
    /// The file on the stick that has it.
    pub(crate) on_stick: Text,
    /// That file's SHA-256, when it could be read.
    pub(crate) sha: Option<[u8; 32]>,
    /// The backup flow's write, not the stick visit's.
    pub(crate) flow: bool,
    /// The line under the title.
    pub(crate) body: Text,
    /// The free name Save beside writes it under.
    pub(crate) beside: Text,
    /// The SHA-256 of the bytes the write sent, which the answer sends
    /// again; none where the write is no longer known.
    pub(crate) sent: Option<[u8; 32]>,
}

/// `name` split before its last dot (a leading dot is no extension).
fn stem_ext(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    }
}

/// `name` for suffix 1, else `stem-n.ext`.
fn suffixed(name: &str, n: u32) -> Text {
    if n <= 1 {
        return Text::of(name);
    }
    let (stem, ext) = stem_ext(name);
    text!("{stem}-{n}{ext}")
}

/// `s` in ASCII lower case.
fn lower(s: &str) -> Text {
    s.chars().map(|c| c.to_ascii_lowercase()).collect()
}

/// The first of `name`, `stem-2.ext`, `stem-3.ext`… none of `taken`,
/// compared without regard to case as FAT does, as `faraday_files` names
/// a file kept beside another. The one finder of such a name.
pub(crate) fn free_name(taken: &[impl AsRef<str>], name: &str) -> Text {
    let is_taken = |n: &str| taken.iter().any(|t| t.as_ref().eq_ignore_ascii_case(n));
    if !is_taken(name) {
        return Text::of(name);
    }
    let (stem, ext) = stem_ext(name);
    let mut n = 2u32;
    loop {
        let beside = text!("{stem}-{n}{ext}");
        if !is_taken(&beside) {
            return beside;
        }
        n += 1;
    }
}

/// The first of `name`, `stem-2.ext`, `stem-3.ext`… that no file waiting
/// in `outbox` is called, so that a file put there under a name it shares
/// with others goes beside the one waiting rather than over it.
pub(crate) fn outbox_free_name(outbox: &[crate::Item], name: &str) -> Text {
    let taken: Vec<Text> = outbox.iter().map(|i| lower(&i.name)).collect();
    free_name(&taken, name)
}

/// Where a file that goes beside the Outbox's goes ([`beside_plan`]).
pub(crate) enum Beside {
    /// The Outbox holds this file already, under this name.
    Same(Text),
    /// The file, made and classified under the name asked, to be put under
    /// the first name that is free.
    Put(Box<crate::Item>),
}

/// Costs a classification of the file (`Item::new`). What
/// [`crate::Faraday::put_outbox_beside`] does before it touches the Outbox:
/// `waiting` is the bytes of the file already there under `name`, if any.
pub(crate) fn beside_plan(
    c: &faraday_crypto::work::Compute,
    name: &str,
    bytes: Zeroizing<Vec<u8>>,
    waiting: Option<&[u8]>,
) -> Beside {
    if waiting == Some(bytes.as_slice()) {
        return Beside::Same(Text::of(name));
    }
    Beside::Put(Box::new(crate::Item::new(c, name, bytes)))
}

/// The stick visit's area: its state, the pull waiting, the receipt of
/// what was written, the vault files written from, Files' row menus and
/// the For the stick list's scroll. The visit state is read through
/// [`std::ops::Deref`]; it is written, like the rest, through the named
/// methods below.
#[derive(Default)]
pub(crate) struct VisitArea {
    visit: crate::VisitState,
    /// What was pressed with a stick attached, done when it is pulled
    /// ([`Sheet::Pull`]).
    pull: Option<Action>,
    /// The flow a stick visit returns to when the stick is pulled.
    after_visit: Option<Screen>,
    /// The visit came from Add a key.
    for_key: bool,
    /// What the pictures copied in so far have read of a transfer in
    /// parts, which can span several pictures. Kept apart from the camera
    /// sheet's, so nothing of it carries into a later scan.
    pictures_scan: Option<crate::ScanState>,
    /// What the last stick visit wrote, this power-on (§4.3).
    receipt: Option<crate::Receipt>,
    /// Each vault file read from a stick this power-on: its name, the
    /// hash of its bytes as read, and the stick's label (§3.5).
    vault_from: Vec<(Text, [u8; 32], Text)>,
    /// A file row's ⋮ menu, Rename and Details (`docs/FLOWS.md`
    /// decision 61).
    files_menu: crate::file_menu::FileMenus,
    /// The vault writes refused for their names, the first on the
    /// name-taken sheet ([`Sheet::NameTaken`]).
    name_taken: std::collections::VecDeque<NameTaken>,
    /// The names this write wrote, for the free name the sheet offers.
    wrote: Vec<Text>,
    /// The file For the stick that [`Sheet::OutboxDelete`] asks about, and
    /// what deleting it loses.
    outbox_delete: Option<(Text, crate::vault::more::Loss)>,
    /// The PSBT a stick's "Import and sign" brought in: its Spend shows the
    /// signing sequence.
    sign_stick: Option<Text>,
    /// The stick that PSBT came from, as a receipt names it: the stick
    /// "Lock and write" writes to when none is in at the press.
    sign_from: Option<Text>,
    /// The one PSBT on the stick on show that "Import and sign" names,
    /// worked out before a frame ([`Faraday::visit_sign_refresh`]).
    sign_file: Option<Text>,
    /// A press of "Lock and write" is on its way to the lock: the files
    /// For the stick are written when it comes back.
    lock_to_write: Option<WriteWait>,
    /// The lock was "Lock and write": the stick it names, or the next to
    /// arrive when none was in, is written with these files at once, and
    /// they are checked.
    write_waiting: Option<WriteWait>,
    /// The files For the stick were written by "Lock and write": the page
    /// ends on the card Finish has.
    signed_end: bool,
    /// The files known the same in Faraday and on a stick.
    pub(super) same: super::visit_same::SameFiles,
}

/// A stick told from another: by its FAT volume serial; a folder the shell
/// lists has none, and is told by where it is.
fn stick_identity(stick: &crate::StickInfo) -> Text {
    match stick.serial {
        Some(s) => text!("{s:08x}"),
        None => text!("id:{}", &*stick.id),
    }
}

/// The files a "Lock and write", or a take-away's write through the lock,
/// writes to the stick, and which stick.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct WriteWait {
    /// The stick in when it was pressed, as a visit's receipt names it;
    /// none when no stick was in.
    pub stick: Option<Text>,
    /// The stick's label, for the sheet that asks before the lock; not
    /// kept for the next process, which has no such sheet.
    pub label: Text,
    /// The spend's own wait: its write ends on the spend's end card.
    pub spend: bool,
    /// The files, by name.
    pub names: Vec<Text>,
}

impl WriteWait {
    /// What the next process keeps: the stick's identity on the first
    /// line (empty for none), "spend" or nothing on the second, then one
    /// name a line.
    pub(crate) fn encode(&self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(self.stick.as_deref().unwrap_or("").as_bytes());
        out.push(b'\n');
        if self.spend {
            out.extend_from_slice(b"spend");
        }
        for n in &self.names {
            out.push(b'\n');
            out.extend_from_slice(n.as_bytes());
        }
        out
    }

    /// What [`Self::encode`] kept; none when no name is left.
    pub(crate) fn decode(bytes: &[u8]) -> Option<WriteWait> {
        let mut lines = bytes.split(|b| *b == b'\n');
        let stick = lines
            .next()
            .filter(|l| !l.is_empty())
            .map(Text::from_utf8_lossy);
        let spend = lines.next() == Some(b"spend".as_slice());
        let names: Vec<Text> = lines
            .filter(|l| !l.is_empty())
            .map(Text::from_utf8_lossy)
            .collect();
        (!names.is_empty()).then_some(WriteWait {
            stick,
            label: Text::default(),
            spend,
            names,
        })
    }
}

impl std::ops::Deref for VisitArea {
    type Target = crate::VisitState;
    fn deref(&self) -> &crate::VisitState {
        &self.visit
    }
}

impl VisitArea {
    /// The PSBT brought in by "Import and sign".
    pub(crate) fn sign_stick(&self) -> Option<&Text> {
        self.sign_stick.as_ref()
    }

    /// The stick "Import and sign" brought the PSBT from.
    pub(crate) fn sign_from(&self) -> Option<&Text> {
        self.sign_from.as_ref()
    }

    /// "Import and sign" came from the stick `identity`.
    pub(crate) fn set_sign_from(&mut self, identity: Option<Text>) {
        self.sign_from = identity;
    }

    /// The PSBT the stick on show offers to sign, as last worked out.
    pub(crate) fn sign_file(&self) -> Option<&Text> {
        self.sign_file.as_ref()
    }

    /// "Import and sign" brings `name` in to sign.
    pub(crate) fn set_sign_stick(&mut self, name: Option<Text>) {
        self.sign_stick = name;
    }

    /// The PSBT the stick on show offers to sign.
    pub(crate) fn set_sign_file(&mut self, name: Option<Text>) {
        self.sign_file = name;
    }

    /// What the lock now coming writes, when it is "Lock and write".
    pub(crate) fn lock_to_write(&self) -> Option<&WriteWait> {
        self.lock_to_write.as_ref()
    }

    /// "Lock and write" was pressed (`Some`) or given up (`None`).
    pub(crate) fn set_lock_to_write(&mut self, what: Option<WriteWait>) {
        self.lock_to_write = what;
    }

    /// What is written when a stick arrives: the files of a "Lock and
    /// write" the process before this one ended with.
    pub(crate) fn write_waiting(&self) -> Option<&WriteWait> {
        self.write_waiting.as_ref()
    }

    /// The files written when a stick arrives, or none.
    pub(crate) fn set_write_waiting(&mut self, what: Option<WriteWait>) {
        self.write_waiting = what;
    }

    /// Whether the page shows the end card after a "Lock and write".
    pub(crate) fn signed_end(&self) -> bool {
        self.signed_end
    }

    /// The page shows the end card, or not.
    pub(crate) fn set_signed_end(&mut self, on: bool) {
        self.signed_end = on;
    }

    /// What was pressed with a stick attached, done when it is pulled.
    pub(crate) fn pull(&self) -> &Option<Action> {
        &self.pull
    }

    /// Waits `action` for the pull.
    pub(crate) fn set_pull(&mut self, action: Action) {
        self.pull = Some(action);
    }

    /// Nothing waits for the pull.
    pub(crate) fn clear_pull(&mut self) {
        self.pull = None;
    }

    /// What waited for the pull, taken to be done.
    pub(crate) fn take_pull(&mut self) -> Option<Action> {
        self.pull.take()
    }

    /// The flow a stick visit returns to when the stick is pulled.
    pub(crate) fn after_visit(&self) -> &Option<Screen> {
        &self.after_visit
    }

    /// The visit returns to `screen` when the stick is pulled.
    pub(crate) fn set_after_visit(&mut self, screen: Screen) {
        self.after_visit = Some(screen);
        self.for_key = false;
    }

    /// The visit returns to no flow.
    pub(crate) fn clear_after_visit(&mut self) {
        self.after_visit = None;
        self.for_key = false;
    }

    /// The visit came from Add a key: what it imports loads at the pull.
    pub(crate) fn for_key(&self) -> bool {
        self.for_key
    }

    /// Sets whether the visit came from Add a key.
    pub(crate) fn set_for_key(&mut self, on: bool) {
        self.for_key = on;
    }

    /// The flow the visit returns to, taken to go back to it.
    pub(crate) fn take_after_visit(&mut self) -> Option<Screen> {
        self.for_key = false;
        self.after_visit.take()
    }

    /// What the last write to a stick wrote.
    pub(crate) fn receipt(&self) -> &Option<crate::Receipt> {
        &self.receipt
    }

    /// `receipt` is what the last write to a stick wrote.
    pub(crate) fn set_receipt(&mut self, receipt: Option<crate::Receipt>) {
        self.receipt = receipt;
    }

    /// A file's row added to the receipt.
    pub(crate) fn receipt_push(&mut self, file: crate::ReceiptFile) {
        if let Some(r) = self.receipt.as_mut() {
            r.files.push(file);
        }
    }

    /// Vault files written from: name, hash and the stick's id.
    pub(crate) fn vault_from(&self) -> &Vec<(Text, [u8; 32], Text)> {
        &self.vault_from
    }

    /// The first vault file read from a stick that `matches`.
    pub(crate) fn vault_from_find(
        &self,
        matches: impl Fn(&(Text, [u8; 32], Text)) -> bool,
    ) -> Option<&(Text, [u8; 32], Text)> {
        self.vault_from.iter().find(|x| matches(x))
    }

    /// The vault file `name` was read from a stick: its hash and the
    /// stick's label, in place of any earlier read of that name.
    pub(crate) fn vault_from_set(&mut self, name: &str, sha: [u8; 32], label: impl AsRef<str>) {
        self.vault_from.retain(|(n, ..)| n != name);
        self.vault_from
            .push((Text::of(name), sha, Text::of(label.as_ref())));
    }

    /// A vault file read from a stick, added as it was.
    pub(crate) fn vault_from_add(
        &mut self,
        name: impl AsRef<str>,
        sha: [u8; 32],
        label: impl AsRef<str>,
    ) {
        self.vault_from
            .push((Text::of(name.as_ref()), sha, Text::of(label.as_ref())));
    }

    /// The file the delete sheet asks about, and what is lost with it.
    pub(crate) fn outbox_delete(&self) -> Option<&(Text, crate::vault::more::Loss)> {
        self.outbox_delete.as_ref()
    }

    /// The delete sheet asks about `ask`, or about nothing.
    pub(crate) fn outbox_delete_set(&mut self, ask: Option<(Text, crate::vault::more::Loss)>) {
        self.outbox_delete = ask;
    }

    /// The vault writes waiting for an answer to the name-taken sheet.
    pub(crate) fn name_taken(&self) -> &std::collections::VecDeque<NameTaken> {
        &self.name_taken
    }

    /// A refused write joins the queue.
    pub(crate) fn name_taken_push(&mut self, t: NameTaken) {
        self.name_taken.push_back(t);
    }

    /// The queue's free names, one for each entry in order.
    pub(crate) fn name_taken_set_besides(&mut self, besides: Vec<Text>) {
        for (t, b) in self.name_taken.iter_mut().zip(besides) {
            t.beside = b;
        }
    }

    /// The flow's entries leave the queue.
    pub(crate) fn name_taken_drop_flow(&mut self) {
        self.name_taken.retain(|t| !t.flow);
    }

    /// The entries for the sticks in `ids` leave the queue.
    pub(crate) fn name_taken_drop_sticks(&mut self, ids: &[Text]) {
        self.name_taken.retain(|t| !ids.contains(&t.stick));
    }

    /// Every entry leaves the queue, in order.
    pub(crate) fn name_taken_drain(&mut self) -> Vec<NameTaken> {
        self.name_taken.drain(..).collect()
    }

    /// The first entry leaves the queue.
    pub(crate) fn name_taken_pop(&mut self) -> Option<NameTaken> {
        self.name_taken.pop_front()
    }

    /// The queue is emptied.
    pub(crate) fn name_taken_clear(&mut self) {
        self.name_taken.clear();
    }

    /// A name this write wrote.
    pub(crate) fn wrote_add(&mut self, name: &str) {
        self.wrote.push(Text::of(name));
    }

    /// A new write starts with no names written.
    pub(crate) fn wrote_clear(&mut self) {
        self.wrote.clear();
    }

    /// Files' row menus.
    pub(crate) fn files_menu(&self) -> &crate::file_menu::FileMenus {
        &self.files_menu
    }

    /// The open ⋮ menu leaves; the keyboard stays on its ⋮ when it opened
    /// or moved in the menu.
    pub(crate) fn file_menu_leave(&mut self) {
        if let Some(m) = self.files_menu.menu.take()
            && m.keys
        {
            self.files_menu.focus = Some(m.row);
        }
    }

    /// A ⋮ menu opens; `focus` is the ⋮ the keyboard is on, when it opened it.
    pub(crate) fn file_menu_opened(
        &mut self,
        menu: crate::file_menu::FileMenu,
        focus: Option<crate::file_menu::Row>,
    ) {
        self.files_menu.menu = Some(menu);
        if focus.is_some() {
            self.files_menu.focus = focus;
        }
    }

    /// The keyboard moved in the open menu to its line `at`.
    pub(crate) fn file_menu_moved(&mut self, at: usize) {
        if let Some(m) = self.files_menu.menu.as_mut() {
            m.at = at;
            m.keys = true;
        }
    }

    /// The open menu is closed from the keyboard.
    pub(crate) fn file_menu_keys_on(&mut self) {
        if let Some(m) = self.files_menu.menu.as_mut() {
            m.keys = true;
        }
    }

    /// The keyboard is on the ⋮ of `row`, or on none.
    pub(crate) fn set_file_focus(&mut self, row: Option<crate::file_menu::Row>) {
        self.files_menu.focus = row;
    }

    /// The Rename popup and the Details sheet are gone.
    pub(crate) fn file_popups_close(&mut self) {
        self.files_menu.renaming = None;
        self.files_menu.details = None;
    }

    /// The Rename popup opens.
    pub(crate) fn set_renaming(&mut self, r: crate::file_menu::Renaming) {
        self.files_menu.renaming = Some(r);
    }

    /// Whether the popup open is the stick's own name.
    pub(crate) fn renaming_stick(&self) -> bool {
        self.files_menu
            .renaming
            .as_ref()
            .is_some_and(|r| matches!(r.what, crate::file_menu::Named::Stick(_)))
    }

    pub(crate) fn take_renaming(&mut self) -> Option<crate::file_menu::Renaming> {
        self.files_menu.renaming.take()
    }

    /// A character typed into the Rename popup's name.
    pub(crate) fn rename_type(&mut self, c: char) {
        if let Some(r) = self.files_menu.renaming.as_mut() {
            r.stem.push(c);
        }
    }

    /// The Rename popup's name loses its last character.
    pub(crate) fn rename_backspace(&mut self) {
        if let Some(r) = self.files_menu.renaming.as_mut() {
            r.stem.pop();
        }
    }

    /// The Details sheet opens.
    pub(crate) fn set_details(&mut self, d: crate::file_menu::Details) {
        self.files_menu.details = Some(d);
    }

    /// The hash of the file the Details sheet is about, when it is `name`
    /// on `stick`.
    pub(crate) fn details_hashed(
        &mut self,
        stick: &str,
        name: &str,
        sha: Result<[u8; 32], impl AsRef<str>>,
    ) {
        if let Some(d) = self.files_menu.details.as_mut()
            && d.stick.as_deref() == Some(stick)
            && d.name == name
        {
            d.sha = Some(sha.map_err(|e| e.as_ref().into()));
        }
    }

    /// The file `name` For the stick is no longer one the person renamed.
    pub(crate) fn renamed_remove(&mut self, name: &str) {
        self.files_menu.renamed.remove(name);
    }

    /// The file `from` For the stick was renamed `to`.
    pub(crate) fn renamed_to(&mut self, from: &str, to: &str) {
        self.files_menu.renamed.remove(from);
        self.files_menu.renamed.insert(to.into());
    }

    /// The files the person renamed, as they were kept.
    pub(crate) fn set_renamed(&mut self, names: impl IntoIterator<Item = impl AsRef<str>>) {
        self.files_menu.renamed = names.into_iter().map(|n| n.as_ref().into()).collect();
    }

    /// How far the visit's Outbox list is scrolled.
    pub(crate) fn out_offset(&self) -> f32 {
        self.visit.out_offset
    }

    /// The Outbox list is scrolled to `offset`.
    pub(crate) fn set_out_offset(&mut self, offset: f32) {
        self.visit.out_offset = offset;
    }

    /// The stick shown is the one at `i`.
    pub(crate) fn set_stick(&mut self, i: usize) {
        self.visit.stick = i;
    }

    /// Whether On the stick is the tab shown on a small window.
    pub(crate) fn set_tab_stick(&mut self, on: bool) {
        self.visit.tab_stick = on;
    }

    /// The settings file's row as the person set it, or its default.
    pub(crate) fn set_settings(&mut self, on: Option<bool>) {
        self.visit.settings = on;
    }

    /// A line of what the last write or copy did.
    pub(crate) fn log_push(&mut self, line: (Text, bool)) {
        self.visit.log.push(line);
    }

    /// A secret file named `name` is on the stick `stick` (labelled
    /// `label`): this session wrote it there or read it from there. The
    /// notice's line is made now, not when it is drawn.
    pub(crate) fn unprotected_add(&mut self, stick: &str, label: &str, name: &str) {
        let at = match self.visit.unprotected.iter().position(|u| u.stick == stick) {
            Some(at) => at,
            None => {
                self.visit.unprotected.push(Unprotected {
                    stick: Text::of(stick),
                    ..Default::default()
                });
                self.visit.unprotected.len() - 1
            }
        };
        let u = &mut self.visit.unprotected[at];
        u.label = Text::of(label);
        u.names.insert(Text::of(name));
        u.make_line();
    }

    /// The file `name` is gone from the stick `stick`.
    pub(crate) fn unprotected_gone(&mut self, stick: &str, name: &str) {
        for u in self
            .visit
            .unprotected
            .iter_mut()
            .filter(|u| u.stick == stick)
        {
            u.names.remove(name);
            u.make_line();
        }
        self.visit.unprotected.retain(|u| !u.names.is_empty());
    }

    /// Only the sticks in `present` keep their notice.
    pub(crate) fn unprotected_keep(&mut self, present: &[Text]) {
        self.visit
            .unprotected
            .retain(|u| present.contains(&u.stick));
    }

    /// The notice of the stick `stick`: its line, and the names.
    pub(crate) fn unprotected_of(&self, stick: &str) -> Option<(&Text, Vec<Text>)> {
        self.visit
            .unprotected
            .iter()
            .find(|u| u.stick == stick)
            .map(|u| (&u.line, u.names.iter().cloned().collect()))
    }

    /// The pictures' scan, taken to read the next picture in it.
    pub(crate) fn take_pictures_scan(&mut self) -> Option<crate::ScanState> {
        self.pictures_scan.take()
    }

    /// The pictures' scan, kept for the next picture (`None` forgets it).
    pub(crate) fn set_pictures_scan(&mut self, scan: Option<crate::ScanState>) {
        self.pictures_scan = scan;
    }

    /// The log is emptied.
    pub(crate) fn log_clear(&mut self) {
        self.visit.log.clear();
    }

    /// The files whose names are taken, while the sheet asks.
    pub(crate) fn set_taken(&mut self, taken: Vec<Taken>, scope: Option<Vec<Text>>) {
        self.visit.taken = taken;
        self.visit.taken_scope = scope;
    }

    /// The files the take-away's write named when it opened the sheet that
    /// asks, given as the sheet stops asking; none for Files' Write.
    pub(crate) fn take_taken_scope(&mut self) -> Option<Vec<Text>> {
        self.visit.taken_scope.take()
    }

    /// The files whose names were taken, given as the sheet stops asking.
    pub(crate) fn take_taken(&mut self) -> Vec<Taken> {
        self.visit.taken_scope = None;
        std::mem::take(&mut self.visit.taken)
    }

    /// The names loaded as keys when the stick is pulled, given as it is.
    pub(crate) fn take_load_after(&mut self) -> std::collections::BTreeSet<Text> {
        std::mem::take(&mut self.visit.load_after)
    }

    /// Nothing is taken: the sheet is not asking.
    pub(crate) fn taken_clear(&mut self) {
        self.visit.taken.clear();
        self.visit.taken_scope = None;
    }

    /// The Outbox files chosen to write are `chosen`.
    pub(crate) fn set_out(&mut self, chosen: impl IntoIterator<Item = impl AsRef<str>>) {
        self.visit.out = chosen.into_iter().map(|n| Text::of(n.as_ref())).collect();
    }

    /// The Outbox file `name` is no longer chosen; whether it was.
    pub(crate) fn out_unchoose(&mut self, name: &str) -> bool {
        self.visit.out.remove(name)
    }

    /// Choosing `name` flipped.
    pub(crate) fn out_toggle(&mut self, name: impl AsRef<str>) {
        let name = name.as_ref();
        if !self.visit.out.remove(name) {
            self.visit.out.insert(Text::of(name));
        }
    }

    /// Nothing of the Outbox is chosen.
    pub(crate) fn out_clear(&mut self) {
        self.visit.out.clear();
    }

    /// Every name in `names` is chosen.
    pub(crate) fn out_extend(&mut self, names: impl IntoIterator<Item = impl AsRef<str>>) {
        self.visit
            .out
            .extend(names.into_iter().map(|n| Text::of(n.as_ref())));
    }

    /// The chosen Outbox file `from` was renamed `to`, and stays chosen.
    pub(crate) fn out_renamed(&mut self, from: &str, to: &str) {
        if self.visit.out.remove(from) {
            self.visit.out.insert(Text::of(to));
        }
    }

    /// The stick files chosen to copy are `chosen`, all by the visit's
    /// default and none by the person.
    pub(crate) fn set_inn(&mut self, chosen: impl IntoIterator<Item = impl AsRef<str>>) {
        self.visit.inn = chosen.into_iter().map(|n| Text::of(n.as_ref())).collect();
        self.visit.inn_own.clear();
    }

    /// As `set_inn`, but the person's own ticks of files still being read
    /// stay, so a read answered after the screen changed still sees them.
    pub(crate) fn set_inn_keeping_reads(
        &mut self,
        chosen: impl IntoIterator<Item = impl AsRef<str>>,
    ) {
        let reading = &self.visit.reading;
        self.visit.inn_own.retain(|n| reading.contains(n));
        self.visit.inn = chosen.into_iter().map(|n| Text::of(n.as_ref())).collect();
    }

    /// The stick file `name` is asked for and not yet answered.
    pub(crate) fn reading_has(&self, name: &str) -> bool {
        self.visit.reading.contains(name)
    }

    /// The person ticked the stick file `name` themselves.
    pub(crate) fn inn_is_own(&self, name: &str) -> bool {
        self.visit.inn_own.contains(name)
    }

    /// The stick file `name` is no longer chosen.
    pub(crate) fn inn_unchoose(&mut self, name: &str) {
        self.visit.inn.remove(name);
        self.visit.inn_own.remove(name);
    }

    /// Choosing the stick file `name` flipped by the person.
    pub(crate) fn inn_toggle(&mut self, name: impl AsRef<str>) {
        let name = name.as_ref();
        if self.visit.inn.remove(name) {
            self.visit.inn_own.remove(name);
        } else {
            self.visit.inn.insert(Text::of(name));
            self.visit.inn_own.insert(Text::of(name));
        }
    }

    /// No stick file is chosen.
    pub(crate) fn inn_clear(&mut self) {
        self.visit.inn.clear();
        self.visit.inn_own.clear();
    }

    /// Only the chosen stick files that `keep` says are kept.
    pub(crate) fn inn_retain(&mut self, keep: impl Fn(&str) -> bool) {
        self.visit.inn.retain(|n| keep(n));
        self.visit.inn_own.retain(|n| keep(n));
    }

    /// The chosen stick file `from` was renamed `to`, and stays chosen.
    pub(crate) fn inn_renamed(&mut self, from: &str, to: &str) {
        if self.visit.inn.remove(from) {
            self.visit.inn.insert(Text::of(to));
        }
        if self.visit.inn_own.remove(from) {
            self.visit.inn_own.insert(Text::of(to));
        }
    }

    /// A stick file is asked for and not yet answered.
    pub(crate) fn reading_add(&mut self, name: impl AsRef<str>) {
        self.visit.reading.insert(Text::of(name.as_ref()));
    }

    /// A stick file asked for is answered.
    pub(crate) fn reading_done(&mut self, name: &str) {
        self.visit.reading.remove(name);
    }

    /// `name` is loaded as keys when the stick is pulled.
    pub(crate) fn load_after_add(&mut self, name: impl AsRef<str>) {
        self.visit.load_after.insert(Text::of(name.as_ref()));
    }

    /// Every name in `names` is loaded as keys when the stick is pulled.
    pub(crate) fn load_after_extend(&mut self, names: impl IntoIterator<Item = impl AsRef<str>>) {
        self.visit
            .load_after
            .extend(names.into_iter().map(|n| Text::of(n.as_ref())));
    }

    /// `name` is no longer loaded as keys when the stick is pulled.
    pub(crate) fn load_after_remove(&mut self, name: &str) {
        self.visit.load_after.remove(name);
    }

    /// The copy of `name` kept as `kept` is loaded in its place.
    pub(crate) fn load_after_renamed(&mut self, name: &str, kept: &str) {
        if self.visit.load_after.remove(name) {
            self.visit.load_after.insert(Text::of(kept));
        }
    }

    /// The Inbox file `name` is chosen to write to the stick.
    pub(crate) fn inbox_pick(&mut self, name: impl AsRef<str>) {
        self.visit.from_inbox.insert(Text::of(name.as_ref()));
    }

    /// The Inbox file `name` is no longer chosen; whether it was.
    pub(crate) fn inbox_unpick(&mut self, name: &str) -> bool {
        self.visit.from_inbox.remove(name)
    }

    /// Every name in `names` is chosen from the Inbox.
    pub(crate) fn inbox_pick_all(&mut self, names: impl IntoIterator<Item = impl AsRef<str>>) {
        self.visit
            .from_inbox
            .extend(names.into_iter().map(|n| Text::of(n.as_ref())));
    }

    /// No Inbox file is chosen.
    pub(crate) fn inbox_pick_none(&mut self) {
        self.visit.from_inbox.clear();
    }

    /// The Inbox file `name` is sent to be written.
    pub(crate) fn writing_inbox_add(&mut self, name: impl AsRef<str>) {
        self.visit.writing_inbox.insert(Text::of(name.as_ref()));
    }

    /// The Inbox file `name` is answered; whether it was sent.
    pub(crate) fn writing_inbox_done(&mut self, name: &str) -> bool {
        self.visit.writing_inbox.remove(name)
    }

    /// The write under way starts as `writing`.
    pub(crate) fn set_writing(&mut self, label: &str, identity: &str, sent: &[(Text, [u8; 32])]) {
        let sent = sent.iter().map(|(n, h)| (n.duplicate(), *h)).collect();
        self.visit.writing = Some((Text::of(label), Text::of(identity), sent, false));
    }

    /// `name` is no longer waited for in the write under way; the write is
    /// over when nothing is left.
    pub(crate) fn writing_drop_file(&mut self, name: &str) {
        if let Some((_, _, sent, _)) = self.visit.writing.as_mut() {
            sent.retain(|(n, _)| n != name);
            if sent.is_empty() {
                self.visit.writing = None;
            }
        }
    }

    /// The answer for `name` in the write under way: its hash as sent, and
    /// the new receipt when it is the write's first answer (made at `now`).
    /// `None` when `name` was not sent. The write is over when nothing is
    /// left.
    pub(crate) fn writing_answered(
        &mut self,
        name: &str,
        now: Option<u64>,
    ) -> Option<(Text, [u8; 32], Option<crate::Receipt>)> {
        let (label, stick, sent, started) = self.visit.writing.as_mut()?;
        let k = sent.iter().position(|(n, _)| n == name)?;
        let (name, hash) = sent.remove(k);
        let stick = stick.clone();
        let opened = (!*started).then(|| {
            *started = true;
            crate::Receipt {
                label: label.as_str().into(),
                stick: stick.as_str().into(),
                time: now,
                files: Vec::new(),
                by_flow: false,
            }
        });
        if sent.is_empty() {
            self.visit.writing = None;
        }
        Some((Text::of(&name), hash, opened))
    }
}

impl Faraday {
    /// Costs a comparison with a file of that name and what [`beside_plan`]
    /// costs. A file to the Outbox
    /// that goes beside a different file of its name
    /// and not over it: a file of that name holding the same bytes is left
    /// as it is. The name it ended up under.
    pub(crate) fn put_outbox_beside(
        &mut self,
        name: &str,
        bytes: impl Into<Zeroizing<Vec<u8>>>,
    ) -> Text {
        let bytes: Zeroizing<Vec<u8>> = bytes.into();
        let c = self.compute();
        let waiting = self
            .outbox
            .iter()
            .find(|i| i.name == name && i.bytes == bytes)
            .map(|i| i.bytes.as_slice());
        let plan = beside_plan(&c, name, bytes, waiting);
        self.beside_apply(plan)
    }

    /// Moves only. An item made by `Item::new` to the Outbox, over the file
    /// of its name. A secret file is refused, in every build: it enters the
    /// Outbox only through `outbox_secret`.
    pub(crate) fn put_item(&mut self, item: crate::Item) {
        if item.exposure() == crate::secrets::Exposure::Secret {
            return;
        }
        self.outbox.retain(|i| i.name != item.name);
        self.outbox.push(item);
        self.save_boxes();
    }

    /// Moves only. Puts the file a [`beside_plan`] made, under the first
    /// name of its own that no file waiting in the Outbox has.
    pub(crate) fn beside_apply(&mut self, plan: Beside) -> Text {
        match plan {
            Beside::Same(name) => name,
            Beside::Put(mut item) => {
                let free = outbox_free_name(&self.outbox, &item.name);
                item.name = Text::of(&free);
                self.put_item(*item);
                free
            }
        }
    }

    /// Costs a comparison with the files of those names. Like
    /// [`Faraday::put_outbox_beside`] for a pair of files that share a stem
    /// (a code's picture and its `.kef` file): the first suffix (none, `-2`,
    /// `-3`, …) at which each name is free in the Outbox and in Files, or
    /// held by a file of the same bytes, is used for both, so that the pair
    /// keeps one stem and goes beside any other and never over it when the
    /// flow's files move to Files. A file of the same bytes is left as it
    /// is. The names they ended up under.
    pub(crate) fn put_outbox_pair_beside_files(
        &mut self,
        png: Option<(&str, Vec<u8>)>,
        file: (&str, Vec<u8>),
    ) -> (Option<Text>, Text) {
        let files: Vec<(&str, &Vec<u8>)> = png
            .iter()
            .map(|(n, b)| (*n, b))
            .chain([(file.0, &file.1)])
            .collect();
        let free =
            |n: u32| -> Vec<Text> { files.iter().map(|(name, _)| suffixed(name, n)).collect() };
        let fits = |names: &[Text], this: &Faraday| {
            names.iter().zip(&files).all(|(name, (_, bytes))| {
                this.outbox
                    .iter()
                    .chain(this.inbox.iter())
                    .all(|i| !i.name.eq_ignore_ascii_case(name) || *i.bytes == **bytes)
            })
        };
        let mut n = 1;
        while !fits(&free(n), self) {
            n += 1;
        }
        let names = free(n);
        for (name, (_, bytes)) in names.iter().zip(&files) {
            if !self
                .outbox
                .iter()
                .any(|i| i.name == **name && *i.bytes == **bytes)
            {
                self.put_outbox(name, (*bytes).clone());
            }
        }
        let mut names = names.into_iter();
        let png_name = png.is_some().then(|| names.next()).flatten();
        (png_name, names.next().unwrap_or_default())
    }

    /// The stick visit's state.
    pub fn visit(&self) -> &crate::VisitState {
        &self.visit
    }

    /// What the last stick visit wrote, this power-on.
    pub fn receipt(&self) -> Option<&crate::Receipt> {
        self.visit.receipt().as_ref()
    }

    /// Files' row menus.
    pub fn files_menu(&self) -> &crate::file_menu::FileMenus {
        self.visit.files_menu()
    }

    /// How far Files' In Faraday list is scrolled.
    pub fn in_faraday_offset(&self) -> f32 {
        self.visit.out_offset()
    }

    /// What the visit's Write writes, the settings file aside: each
    /// chosen Outbox file, then each chosen Inbox file the visit lists,
    /// with its bytes and whether it came from the Inbox.
    ///
    /// `scope` is the take-away's own file names: then only the chosen
    /// Outbox files among them, never the Inbox's.
    fn visit_files(&self, scope: Option<&[Text]>) -> Vec<(Text, Zeroizing<Vec<u8>>, bool)> {
        let mut out: Vec<(Text, Zeroizing<Vec<u8>>, bool)> = self
            .outbox
            .iter()
            .filter(|i| self.visit.out.contains(&*i.name))
            .filter(|i| scope.is_none_or(|s| s.iter().any(|n| **n == *i.name)))
            .map(|i| (Text::of(&i.name), i.bytes.clone(), false))
            .collect();
        for k in self
            .visit_inbox_rows()
            .into_iter()
            .filter(|_| scope.is_none())
        {
            let item = &self.inbox[k];
            if self.visit.from_inbox.contains(&*item.name) {
                out.push((Text::of(&item.name), item.bytes.clone(), true));
            }
        }
        out
    }

    /// The Inbox files a lock drops ([`crate::Item::dropped_at_lock`]),
    /// by name.
    pub(crate) fn lock_drops(&self) -> Vec<Text> {
        self.inbox
            .iter()
            .filter(|i| i.dropped_at_lock())
            .map(|i| Text::of(&i.name))
            .collect()
    }

    /// A write the disk refused: the receipt says why, and the visit's log
    /// says what becomes of the file.
    pub(crate) fn visit_write_failed(&mut self, name: &str, reason: &str) {
        self.receipt_add(name, Some(Text::of(reason)));
        self.visit_write_logged(name, reason);
    }

    /// A write given up before anything was written: no receipt row, so
    /// that the proof of an earlier write stays; the log says why.
    fn visit_write_cancelled(&mut self, name: &str, reason: &str) {
        self.visit.writing_drop_file(name);
        self.visit_write_logged(name, reason);
    }

    /// The visit's log line for a file not written.
    fn visit_write_logged(&mut self, name: &str, reason: &str) {
        let stays = if self.visit.writing_inbox_done(name) {
            Text::of(EN.storage_event_it_stays)
        } else {
            Text::fill(EN.storage_event_it_still, &[self.medium.for_the()])
        };
        let line = Text::fill(EN.storage_event_not_written, &[name, reason, &stays]);
        self.visit.log_push((line, false));
    }

    /// The files the write would write whose names are taken on the stick
    /// shown, each with what becomes of it.
    pub(crate) fn visit_taken(&self, scope: Option<&[Text]>) -> Vec<Taken> {
        let Some(stick) = self.stick.get(self.visit.stick) else {
            return Vec::new();
        };
        let mut names: Vec<Text> = stick.files.iter().map(|(n, _)| lower(n)).collect();
        let mut rows = Vec::new();
        for (name, bytes, _) in self.visit_files(scope) {
            let on = stick
                .files
                .iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(&name));
            let Some(_) = on else {
                continue;
            };
            // A vault is asked by the disk process, which reads the file.
            if faraday_codec::vault_file::is_vault_file(&bytes) {
                continue;
            }
            let keep_as = free_name(&names, &name);
            names.push(lower(&keep_as));
            rows.push(Taken { name, keep_as });
        }
        rows
    }

    /// The stick on show, told from another: by its FAT volume serial; a
    /// folder the shell lists has none, and is told by where it is.
    pub(crate) fn visit_stick_identity(&self) -> Option<Text> {
        self.stick.get(self.visit.stick).map(stick_identity)
    }

    /// The open spend's own results (the signed PSBT and the transaction)
    /// that are still in the Outbox.
    pub(crate) fn spend_outbox_names(&self) -> Vec<Text> {
        let Some(s) = self.wallets.spend() else {
            return Vec::new();
        };
        [s.out_names.signed.as_ref(), s.out_names.tx.as_ref()]
            .into_iter()
            .flatten()
            .filter(|n| self.outbox.iter().any(|i| *i.name == ***n))
            .cloned()
            .collect()
    }

    /// What "Lock and write" writes: the spend's results still in the
    /// Outbox, to the stick in now or else the one the PSBT was imported
    /// from. With neither, nothing is written by itself.
    pub(crate) fn spend_write_plan(&self) -> Option<WriteWait> {
        let names = self.spend_outbox_names();
        let stick = self
            .visit_stick_identity()
            .or_else(|| self.visit.sign_from().cloned())?;
        (!names.is_empty()).then(|| WriteWait {
            stick: Some(stick),
            label: Text::default(),
            spend: true,
            names,
        })
    }

    /// The write left through a lock for the stick that came: only the
    /// files it named, to the stick it named (any other stick has the
    /// ordinary visit).
    pub(crate) fn visit_write_waiting(&mut self) {
        let Some(wait) = self.visit.write_waiting().cloned() else {
            return;
        };
        if !self.clean() {
            // Given up: the Lock sheet asks first.
            self.visit.set_write_waiting(None);
            return;
        }
        // The stick it named, wherever it is among those in; no other is
        // written by itself, and none named writes nothing.
        let Some(named) = &wait.stick else {
            self.visit.set_write_waiting(None);
            return;
        };
        let Some(i) = self.stick.iter().position(|s| stick_identity(s) == *named) else {
            return;
        };
        self.visit.set_stick(i);
        self.visit.set_write_waiting(None);
        let names: Vec<Text> = wait
            .names
            .into_iter()
            .filter(|n| self.outbox.iter().any(|i| *i.name == **n))
            .collect();
        if names.is_empty() {
            return;
        }
        self.visit.set_out(names.iter());
        self.visit.inbox_pick_none();
        self.visit.set_settings(Some(false));
        self.visit.set_signed_end(wait.spend);
        self.visit_write_ask(Some(names));
        if self.sheet != Some(Sheet::Taken) {
            self.visit.set_settings(None);
        }
    }

    /// The visit's Write: the sheet first when a file's name is taken and
    /// the choice decides what becomes of it; otherwise written at once.
    ///
    /// A process that is no longer clean asks to lock first, as Write from
    /// Files does; the disk process would refuse the write.
    ///
    /// `scope` is the take-away's file names (`None` for Files' Write): it
    /// goes with the sheet that asks and is dropped as the write is sent.
    pub(crate) fn visit_write_ask(&mut self, scope: Option<Vec<Text>>) {
        if !self.clean() {
            self.sheet = Some(Sheet::WriteOut);
            return;
        }
        let taken = self.visit_taken(scope.as_deref());
        if !taken.is_empty() {
            self.visit.set_taken(taken, scope);
            self.sheet = Some(Sheet::Taken);
        } else {
            self.visit_write(false, scope.as_deref());
        }
    }

    /// The take-away's Write: the files its flow now offers that are
    /// ticked, read from the flow at the press. A process that is not
    /// clean locks first, and the lock keeps exactly these names for the
    /// write that follows; the settings file and the Inbox never ride it.
    fn take_write(&mut self) {
        let names: Vec<Text> = crate::screens::take_away_names(self)
            .into_iter()
            .filter(|n| self.visit.out.contains(&**n))
            .collect();
        if names.is_empty() {
            return;
        }
        if !self.clean() {
            // With no stick in nothing waits: the files stay For the
            // stick.
            let wait = self.stick.get(self.visit.stick).map(|stick| WriteWait {
                stick: Some(stick_identity(stick)),
                label: Text::of(&stick.label),
                spend: false,
                names,
            });
            self.visit.set_lock_to_write(wait);
            self.sheet = Some(Sheet::WriteOut);
            return;
        }
        self.visit_write_ask(Some(names));
    }

    /// Writes the settings file when its row is on, and every chosen
    /// file; a taken file over the old one when `replace`.
    ///
    /// `scope` is the take-away's file names, as in [`Self::visit_write_ask`].
    pub(crate) fn visit_write(&mut self, replace: bool, scope: Option<&[Text]>) {
        if !self.clean() {
            self.visit.taken_clear();
            self.sheet = Some(Sheet::WriteOut);
            return;
        }
        let c = self.compute();
        if self.sheet == Some(Sheet::Taken) {
            self.sheet = None;
        }
        let taken: Vec<Text> = self
            .visit
            .take_taken()
            .into_iter()
            .map(|t| t.name)
            .collect();
        let Some(stick) = self.stick.get(self.visit.stick) else {
            return;
        };
        let id = stick.id.clone();
        let label = stick.label.clone();
        let Some(identity) = self.visit_stick_identity() else {
            return;
        };
        self.visit.log_clear();
        self.name_taken_wrote_clear();
        let mut sends: Vec<(Text, Zeroizing<Vec<u8>>, bool)> = Vec::new();

        if scope.is_none() && self.visit_settings_on() {
            sends.push((
                Text::of(stick::settings::FILE),
                Zeroizing::new(self.settings_file_text().into_bytes()),
                false,
            ));
        }
        for (name, bytes, from_inbox) in self.visit_files(scope) {
            if from_inbox {
                self.visit.writing_inbox_add(&*name);
            }
            // A vault goes over the same vault on the stick (same salt and
            // length, decided where the stick is read) only when this
            // session unlocked or wrote it: a copy from Files that it did
            // not could be older than what the stick holds.
            let over = if faraday_codec::vault_file::is_vault_file(&bytes) {
                !from_inbox || self.vault_is_this_sessions(&c, &bytes)
            } else {
                replace && taken.contains(&name)
            };
            sends.push((name, bytes, over));
        }
        // What this write sends, for its receipt.
        let sent: Vec<(Text, [u8; 32])> = sends
            .iter()
            .map(|(n, b, _)| (n.clone(), sha256_of(&c, b)))
            .collect();
        for (name, bytes, replace) in sends {
            let vault = faraday_codec::vault_file::is_vault_file(&bytes);
            let renamed = self.visit.files_menu().renamed.contains(&*name);
            let copies = self.vault_copies(&c, &bytes);
            self.out.storage_out.push_back(StorageCommand::Write {
                stick: id.clone(),
                name: Text::of(&name),
                bytes,
                copies,
                replace,
                renamed,
                // A vault's name taken by a file nothing proves is asked.
                on_taken: if vault { OnTaken::Ask } else { OnTaken::Beside },
            });
        }
        if !sent.is_empty() {
            self.visit.set_writing(&label, &identity, &sent);
        }
    }
}

/// The Already on the stick sheet: a row per taken file, then Replace,
/// Keep both and Cancel.
pub(crate) fn taken_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let title = Text::fill(EN.visit_taken_already, &[app.medium.on_the()]);
    let rows: Vec<Text> = app.visit.taken.iter().map(Taken::line).collect();
    let place = if ui.compact {
        (8.0, w - 16.0)
    } else {
        let sw = (w - 48.0).min(640.0);
        ((w - sw) / 2.0, sw)
    };
    let margin = if ui.compact { 8.0 } else { 32.0 };
    let pad = if ui.compact { 16.0 } else { 32.0 };
    scroll_sheet(ui, place, h, margin, pad, &mut |ui, x, y, iw| {
        let mut cy = y;
        if ui.compact {
            cy += crate::compact::sheet_head(ui, x, cy, iw, Icon::File, Role::OnSurface, &title);
        } else {
            ui.icon(x, cy, 30.0, Icon::File, 18.0, Role::OnSurface);
            let t = ui.fit_as(faraday_draw::tokens::sheet_head(false), &title, iw - 42.0);
            ui.text_mid_as(
                (x + 42.0, cy, 30.0),
                faraday_draw::tokens::sheet_head(false),
                Role::OnSurface,
                &t,
            );
            cy += 46.0;
        }
        for line in &rows {
            cy += ui.wrap_as((x, cy, iw), BODY_M.mono(), Role::OnSurface, line) + 10.0;
        }
        cy += 8.0;
        cy += buttons(
            ui,
            x,
            cy,
            iw,
            &[
                (EN.visit_taken_replace, Style::Primary, Action::VisitReplace),
                (
                    EN.visit_taken_keep_both,
                    Style::Secondary,
                    Action::VisitKeepBoth,
                ),
                (EN.visit_taken_cancel, Style::Ghost, Action::Cancel),
            ],
        );
        cy - y
    });
}

/// The name-taken sheet: the first vault write the disk process refused
/// for its name (`docs/VAULT.md` §6 step 1): the title, one line, Hold to
/// overwrite where the file could be read, Save beside under a free name,
/// and Cancel.
pub(crate) fn name_taken_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(t) = app.visit.name_taken().front() else {
        return;
    };
    let title = Text::fill(EN.vault_taken_title, &[&t.name, app.medium.on_the()]);
    let overwrite = Text::fill(EN.vault_taken_overwrite, &[&t.on_stick]);
    let beside = Text::fill(EN.vault_taken_beside, &[&t.beside]);
    let head = (Icon::Warning.into(), Role::Warning.paint(), title.as_str());
    crate::compact::dialog(
        ui,
        (w, h, faraday_draw::tokens::DIALOG_MAX),
        head,
        &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += ui.wrap_as((x, cy, iw), BODY_M, Role::OnSurface, &t.body)
                + faraday_draw::tokens::GAP;
            // A hold stays in the dialog's body, away from its actions.
            if t.sha.is_some() {
                crate::vault::screen::hold_button_in(
                    app,
                    ui,
                    x,
                    cy,
                    iw,
                    &overwrite,
                    Action::NameTakenOverwrite,
                    faraday_ui::hold::HoldTone::Warning,
                );
                cy += ui
                    .hold_button_height(iw, &overwrite)
                    .max(ui.button_height(faraday_draw::tokens::BUTTON_H))
                    + faraday_draw::tokens::GAP;
            }
            cy - y
        },
        &[
            (EN.visit_taken_cancel, Style::Ghost, Action::Cancel),
            (&beside, Style::Primary, Action::NameTakenBeside),
        ],
    );
}

/// **Delete {name}?**: a file For the stick no stick holds, asked about
/// before it goes. What is lost is said once; the hold deletes it, Cancel
/// keeps it (Material's basic dialog with a destructive confirmation).
pub(crate) fn outbox_delete_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    use crate::vault::more::Loss;
    let Some((name, loss)) = app.visit.outbox_delete() else {
        return;
    };
    let is_vault = app
        .outbox
        .iter()
        .any(|i| i.name == **name && i.kind == crate::FileKind::Vault);
    let line = match loss {
        Loss::Since(label) => Text::fill(EN.outbox_delete_vault_since, &[label]),
        Loss::All if is_vault => Text::of(EN.outbox_delete_vault_none),
        Loss::All => Text::of(EN.outbox_delete_file_none),
    };
    let hold = if is_vault {
        EN.outbox_delete_hold_vault
    } else {
        EN.outbox_delete_hold_file
    };
    let title = Text::fill(EN.outbox_delete_title, &[name]);
    let head = (Icon::Error.into(), Role::Error.paint(), title.as_str());
    crate::compact::dialog(
        ui,
        (w, h, faraday_draw::tokens::DIALOG_MAX),
        head,
        &mut |ui, x, y, iw| {
            let mut cy = y;
            cy +=
                ui.wrap_as((x, cy, iw), BODY_M, Role::OnSurface, &line) + faraday_draw::tokens::GAP;
            crate::vault::screen::hold_button_in(
                app,
                ui,
                x,
                cy,
                iw,
                hold,
                Action::OutboxDeleteHold,
                faraday_ui::hold::HoldTone::Error,
            );
            cy += ui
                .hold_button_height(iw, hold)
                .max(ui.button_height(faraday_draw::tokens::BUTTON_H))
                + faraday_draw::tokens::GAP;
            cy - y
        },
        &[(EN.outbox_delete_cancel, Style::Ghost, Action::Cancel)],
    );
}

impl Faraday {
    /// A vault's write the disk process refused for its name: queued for
    /// the name-taken sheet, which shows the first. What the sheet says is
    /// decided here, from what the session knows of the file.
    pub(crate) fn name_taken_event(&mut self, ev: crate::StorageEvent) {
        let crate::StorageEvent::NameTaken {
            stick,
            name,
            on_stick,
            sha,
        } = ev
        else {
            return;
        };
        let flow = self.flow_wants_written(&stick, &name);
        let sent = if flow {
            self.flow_pending_sha(&stick, &name)
        } else {
            self.visit.writing.as_ref().and_then(|(_, _, s, _)| {
                s.iter()
                    .find(|(n, _)| n.as_str() == &*name)
                    .map(|(_, h)| *h)
            })
        };
        // What the file is is said only where the session knows both the
        // bytes sent and the file as copies, of two different vaults.
        let ours = self.vaults.vault_id_of_file(sent);
        let body = match self.vaults.summary_of_file(sha) {
            _ if sha.is_none() => Text::of(EN.vault_taken_unreadable),
            Some(v) if ours.is_some_and(|id| id != v.id) => {
                let known = if v.name.is_empty() { &v.file } else { &v.name };
                Text::fill(EN.vault_taken_other, &[known])
            }
            _ => Text::of(EN.vault_taken_unknown),
        };
        let beside = self.name_taken_beside(&stick, &name);
        self.visit.name_taken_push(NameTaken {
            stick: Text::of(&stick),
            name: Text::of(&name),
            on_stick: Text::of(&on_stick),
            sha,
            flow,
            body,
            beside,
            sent,
        });
        self.name_taken_refresh();
        self.sheet = Some(Sheet::NameTaken);
    }

    /// Costs a SHA-256 of each file of that name, or of each file the visit
    /// sends. The bytes a write sent, found again by name and SHA-256 as the
    /// first send found them, and whether they go over the same vault: none where
    /// they are gone, changed or a secret.
    fn name_taken_sent(
        &self,
        c: &faraday_crypto::work::Compute,
        flow: bool,
        name: &str,
        sent: [u8; 32],
    ) -> Option<(Zeroizing<Vec<u8>>, bool)> {
        if flow {
            self.outbox
                .iter()
                .chain(self.inbox.iter())
                .find(|i| i.name == name && sha256_of(c, &i.bytes) == sent)
                .filter(|i| i.exposure() != crate::secrets::Exposure::Secret)
                .map(|i| (i.bytes.clone(), true))
        } else {
            self.visit_files(None)
                .into_iter()
                .find(|(n, b, _)| n == name && sha256_of(c, b) == sent)
                .map(|(_, bytes, from_inbox)| {
                    let replace = !from_inbox || self.vault_is_this_sessions(c, &bytes);
                    (bytes, replace)
                })
        }
    }

    /// The free name beside what `stick` lists and what this write wrote.
    fn name_taken_beside(&self, stick: &str, name: &str) -> Text {
        let mut names: Vec<Text> = self
            .stick
            .iter()
            .find(|s| s.id == stick)
            .map(|s| s.files.iter().map(|(n, _)| lower(n)).collect())
            .unwrap_or_default();
        names.extend(self.visit.wrote.iter().map(|n| lower(n)));
        free_name(&names, name)
    }

    /// The free names of the queue worked out again.
    fn name_taken_refresh(&mut self) {
        let besides: Vec<Text> = self
            .visit
            .name_taken()
            .iter()
            .map(|t| self.name_taken_beside(&t.stick, &t.name))
            .collect();
        self.visit.name_taken_set_besides(besides);
    }

    /// A file of this write was written as `wrote_as`.
    pub(crate) fn name_taken_wrote(&mut self, wrote_as: &str) {
        self.visit.wrote_add(wrote_as);
        self.name_taken_refresh();
    }

    /// A visit's file was written as `wrote_as`: its receipt row.
    pub(crate) fn visit_written(&mut self, name: &str, wrote_as: &str) {
        self.name_taken_wrote(wrote_as);
        self.receipt_add(name, None);
        // A made key file written to the stick from the Outbox is no longer
        // a pending item; a copy from the Inbox of the same name is not it.
        if !self.visit.writing_inbox.contains(name) {
            crate::vault::more::written(self, name);
        }
    }

    /// A new write starts with no names written.
    pub(crate) fn name_taken_wrote_clear(&mut self) {
        self.visit.wrote_clear();
    }

    /// The flow's write ended: the questions of its files go with it.
    pub(crate) fn name_taken_flow_ended(&mut self) {
        self.visit.name_taken_drop_flow();
        self.name_taken_next();
    }

    /// The next entry of the queue is shown, or the sheet closes.
    fn name_taken_next(&mut self) {
        if self.visit.name_taken().is_empty() {
            if self.sheet == Some(Sheet::NameTaken) {
                self.sheet = None;
            }
        } else {
            self.name_taken_refresh();
            self.sheet = Some(Sheet::NameTaken);
        }
    }

    /// The first entry answered with Save beside, or with Overwrite (held)
    /// where `over`: the write sent again, the bytes as the first send had
    /// them.
    pub(crate) fn name_taken_answer(&mut self, over: bool) {
        let Some(t) = self.visit.name_taken().front().cloned() else {
            return;
        };
        if !self.clean() {
            // Every entry is given up as Cancel gives it up.
            let all: Vec<NameTaken> = self.visit.name_taken_drain();
            for t in all {
                self.name_taken_refused(t);
            }
            self.sheet = Some(Sheet::WriteOut);
            return;
        }
        let on_taken = match (over, t.sha) {
            (true, Some(sha)) => OnTaken::Over(sha),
            _ => OnTaken::Beside,
        };
        self.visit.name_taken_pop();
        let c = self.compute();
        // The same bytes and flags as the first send found, by name and
        // SHA-256; none is a cancel.
        let sent = t
            .sent
            .filter(|_| !t.flow || self.flow_wants_written(&t.stick, &t.name))
            .and_then(|h| self.name_taken_sent(&c, t.flow, &t.name, h));
        match sent {
            Some((bytes, replace)) => {
                let renamed = !t.flow && self.visit.files_menu().renamed.contains(&*t.name);
                let copies = self.vault_copies(&c, &bytes);
                self.out.storage_out.push_back(StorageCommand::Write {
                    stick: Text::of(&t.stick),
                    name: Text::of(&t.name),
                    bytes,
                    copies,
                    replace,
                    renamed,
                    on_taken,
                });
            }
            None => self.name_taken_refused(t),
        }
        self.name_taken_next();
    }

    /// Cancel, Back or the scrim on the sheet: nothing is written. A visit's
    /// file stays For the stick, the log says why and the receipt is not
    /// touched; a flow's write does not count and can be pressed again.
    pub(crate) fn name_taken_cancel(&mut self) {
        let Some(t) = self.visit.name_taken_pop() else {
            self.sheet = None;
            return;
        };
        self.name_taken_refused(t);
        self.name_taken_next();
    }

    /// One entry given up: as Cancel does it.
    fn name_taken_refused(&mut self, t: NameTaken) {
        if t.flow {
            // Every file of that write still to ask goes with it.
            self.visit.name_taken_drop_flow();
            self.flow_name_taken(&t.name);
        } else {
            self.visit_write_cancelled(&t.name, EN.visit_taken_name_taken);
        }
    }

    /// Entries for a stick no longer attached are cancelled.
    pub(crate) fn name_taken_pulled(&mut self) {
        let gone: Vec<NameTaken> = self
            .visit
            .name_taken()
            .iter()
            .filter(|t| !self.stick.iter().any(|s| s.id == *t.stick))
            .cloned()
            .collect();
        if gone.is_empty() {
            return;
        }
        let ids: Vec<Text> = gone.iter().map(|t| t.stick.clone()).collect();
        self.visit.name_taken_drop_sticks(&ids);
        for t in gone {
            // A flow's write ends with its own pulled problem.
            if !t.flow {
                let reason = Text::fill(EN.visit_taken_pulled, &[self.medium.noun()]);
                self.visit_write_cancelled(&t.name, &reason);
            }
        }
        self.name_taken_next();
    }
}

impl Faraday {
    /// Whether Files shows the stick pane: a stick attached and nothing
    /// held (F235). A lookup.
    pub fn stick_pane(&self) -> bool {
        self.screen == Screen::Files && !self.stick.is_empty() && !self.holds_secret()
    }

    /// Whether the stick is being visited: its pane is on screen and the
    /// boot import's list is not. Files opening on that list at boot is
    /// not a visit; a press on Files after Later, or Import, is. The stick's
    /// visit badge stays on Files after either until the stick is visited
    /// (the base's way too: Import with the stick left in marks nothing).
    pub(crate) fn stick_visit_on(&self) -> bool {
        self.stick_pane() && !self.import_tile_up()
    }

    /// The last stick pulled: Files stays on Files, only On the stick going
    /// back to its empty state; a flow the stick came in during, or a vault
    /// sealed for the visit, takes the person back to it, to the flow or to
    /// Unlock (F290).
    ///
    /// Returns the flow Add a key was opened from, when the pull went back
    /// to it: the key files imported for it load into its pick.
    pub(crate) fn visit_pulled(&mut self) -> Option<Screen> {
        let mut for_key = None;
        // A return screen with Files not on screen is stale: nothing keeps it.
        if self.screen != Screen::Files {
            self.leave_visit_return();
        }
        // The import's own page is Files: it stays, with Put the stick
        // back in to import.
        if self.screen == Screen::Files && !self.import_tile_up() {
            // A vault sealed for the visit opens again, unless a flow waits.
            let unlock = self
                .visit
                .after_visit()
                .is_none()
                .then(|| self.visit_unlocks())
                .flatten();
            let was_for_key = self.visit.for_key();
            if let Some(back) = self.visit.take_after_visit() {
                self.screen = back;
                for_key = was_for_key.then_some(back);
                if back == Screen::SpendBackup {
                    self.spend_backup_settle();
                }
            }
            if let Some(i) = unlock {
                self.screen = Screen::Home;
                self.vault_act(crate::vault::VaultAction::Open(i));
            }
        }
        for_key
    }

    /// Handles the stick visit's actions (`Action::Visit*`).
    pub(crate) fn visit_act(&mut self, action: Action) {
        // Nothing of the stick is read, copied or written before Import:
        // the import's list stands where the stick's own card does, so
        // every action of that card is refused, except the ways off the
        // page (From the stick, the tab and the bar are views).
        if self.import_tile_up()
            && !matches!(
                action,
                Action::VisitFrom(..)
                    | Action::VisitTab(..)
                    | Action::VisitBar(..)
                    | Action::NameTakenBeside
                    | Action::NameTakenOverwrite
            )
        {
            return;
        }
        match action {
            Action::VisitStick(i) => {
                self.visit.set_stick(i);
                self.list_offset = 0.0;
                {
                    let v = self.visit_default_in();
                    self.visit.set_inn(v);
                }
                self.visit.set_settings(None);
                self.visit_same_ask();
            }
            Action::VisitSettings => {
                self.visit.set_settings(Some(!self.visit_settings_on()));
            }
            Action::VisitOut(i) => {
                if let Some(item) = self
                    .outbox
                    .get(i)
                    .filter(|it| !self.in_the_backup(&it.name))
                {
                    let n = item.name.clone();
                    self.visit.out_toggle(n);
                }
            }
            Action::VisitIn(i) => {
                self.vaults.read_free();
                if let Some(stick) = self.stick.get(self.visit.stick)
                    && let Some((n, size)) = stick.files.get(i)
                    && stick.unread_why(&self.vaults, n, *size).is_none()
                {
                    let n = n.clone();
                    self.visit.inn_toggle(n);
                }
            }
            // Unselect all: everything comes in by default (§4.6).
            Action::VisitInAll => self.visit.inn_clear(),
            Action::VisitOutAll => {
                // Every row but an unprotected secret, which is ticked one
                // at a time (FLOWS.md decision 6); from the Inbox, every
                // public or sealed file, and none that may be a secret.
                let all = self.visit_all_out();
                let inbox = self.visit_inbox_plain();
                let changed = self.settings_changed();
                let every = (!changed || self.visit_settings_on())
                    && all.is_subset(&self.visit.out)
                    && inbox.is_subset(&self.visit.from_inbox);
                if every {
                    self.visit.out_clear();
                    for n in &inbox {
                        self.visit.inbox_unpick(n);
                    }
                } else {
                    self.visit.out_extend(all);
                    self.visit.inbox_pick_all(inbox);
                }
                if changed {
                    self.visit.set_settings(Some(!every));
                }
            }
            Action::VisitInbox(i) => self.visit_inbox(i),
            Action::VisitBar(_) => {}
            Action::VisitTab(c) => {
                self.visit.set_tab_stick(c == crate::Column::Stick);
                self.list_offset = 0.0;
            }
            Action::VisitWrite => self.visit_write_ask(None),
            // Only the files the take-away's flow offers now, ticked: not
            // the settings file, other chosen files or the Inbox's.
            Action::TakeWrite => self.take_write(),
            Action::VisitReplace | Action::VisitKeepBoth => {
                // Only the sheet that asks is answered: a second press on
                // its last frame has no scope left to hold it.
                if self.sheet != Some(Sheet::Taken) {
                    return;
                }
                let scope = self.visit.take_taken_scope();
                self.visit_write(matches!(action, Action::VisitReplace), scope.as_deref());
            }
            Action::NameTakenBeside => self.name_taken_answer(false),
            Action::NameTakenOverwrite => self.name_taken_answer(true),
            Action::VisitCopy if self.visit.for_key() => self.act(Action::VisitCopyAndLoad),
            Action::VisitCopy => self.visit_copy(),
            Action::VisitCopyAndLoad => {
                let inn = self.visit.inn.clone();
                self.visit.load_after_extend(inn);
                self.visit_copy();
                // Straight to the Inbox, where they arrive; the stick's
                // pull loads them from there.
                self.act(Action::Nav(Screen::Files));
            }
            Action::VisitCopyAndSign => {
                let sign = self.visit_sign_file();
                // Only the PSBT the label names comes in.
                if let Some(name) = &sign {
                    self.visit.set_inn([name]);
                }
                self.visit.set_sign_stick(sign);
                let from = self.visit_stick_identity();
                self.visit.set_sign_from(from);
                self.visit_copy();
            }
            // From Add a key: the key file imported here loads at the pull,
            // and the pull goes back to where Add a key was opened from.
            Action::VisitFrom(Screen::Entry) => {
                if !self.stick.is_empty() {
                    let back = self.entry.state().back.unwrap_or(Screen::Home);
                    self.visit.set_after_visit(back);
                    self.visit.set_for_key(true);
                    // The stick's own card, as after Later: the import waits.
                    self.import.set_tile(false);
                    self.act(Action::Nav(Screen::Files));
                }
            }
            Action::VisitFrom(back) => {
                if !self.stick.is_empty() && !self.holds_secret() {
                    self.visit.set_after_visit(back);
                    self.import.set_tile(false);
                    self.act(Action::Nav(Screen::Files));
                }
            }
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }
}

/// What a flow put For the stick, said in a toast (`toast_out_waits` and
/// its kin in the strings file, each its own sentence).
impl Faraday {
    /// A file a flow made and put For the stick: "{name} waits for the
    /// stick".
    pub(crate) fn toast_out(&mut self, name: &str) {
        self.toast_waits(EN.toast_out_waits, &[name]);
    }

    /// Files a flow made and put For the stick, by how many: one by its
    /// name ([`Self::toast_out`]), more as "3 files wait for the stick".
    pub(crate) fn toast_out_files(&mut self, names: &[impl AsRef<str>]) {
        match names {
            [] => {}
            [name] => self.toast_out(name.as_ref()),
            more => self.toast_waits(EN.toast_out_files_wait, &[&Text::num(more.len() as u64)]),
        }
    }

    /// A toast `template` names the medium in: its `{}` are `what`, then
    /// "for the stick" or "for the card or stick".
    pub(crate) fn toast_waits(&mut self, template: &str, what: &[&str]) {
        let mut args = what.to_vec();
        args.push(self.medium.for_the());
        let text = Text::fill(template, &args);
        self.toast(&text);
    }
}

/// The visit screen's choices.
#[derive(Default)]
pub struct VisitState {
    /// The stick shown, by index.
    pub stick: usize,
    /// On a small window, whether Files' tab On the stick is the one
    /// shown (else In Faraday).
    pub tab_stick: bool,
    /// Outbox files chosen to write, by name.
    pub out: BTreeSet<faraday_text::Text>,
    /// Stick files chosen to copy, by name.
    pub inn: BTreeSet<faraday_text::Text>,
    /// The stick files the person ticked themselves, as opposed to the
    /// visit's defaults: a file that turns out to hold a seed is kept
    /// only when its tick is here (`Faraday::visit_read_in`).
    pub inn_own: BTreeSet<faraday_text::Text>,
    /// Stick files asked for and not yet answered, by name: a picture's
    /// codes are read off the shell's thread, one picture at a time.
    pub reading: BTreeSet<faraday_text::Text>,
    /// The settings file's row as the person set it; `None` for its
    /// default (`Faraday::visit_settings_on`).
    pub settings: Option<bool>,
    /// What the last write or copy did, one line per file.
    pub log: Vec<(faraday_text::Text, bool)>,
    /// Names from an "Import and load" copy, by name: loaded as keys the
    /// moment the stick is pulled, no further press needed.
    pub load_after: BTreeSet<faraday_text::Text>,
    /// How far the Outbox list is scrolled, design units. The stick's
    /// files scroll by [`Faraday::list_offset`].
    pub out_offset: f32,
    /// Where each column's scrollbar was drawn, by [`VisitState::bar`].
    pub bars: [std::cell::Cell<Option<VisitBarAt>>; 2],
    /// Inbox files chosen to write, by name: a copy from one stick to
    /// another. One that may be a secret is chosen only on the secret
    /// sheet.
    pub from_inbox: BTreeSet<faraday_text::Text>,
    /// Inbox files sent to be written and not yet answered, by name.
    pub writing_inbox: BTreeSet<faraday_text::Text>,
    /// The write under way: the stick's label, and each file sent with
    /// the SHA-256 of its bytes. Its first answer starts a new receipt.
    pub writing: Option<Writing>,
    /// The files the write would write whose names are taken on the
    /// stick, while the Already on the stick sheet asks.
    pub taken: Vec<Taken>,
    /// With the take-away's write on the Already on the stick sheet: the
    /// only files it may send (public names). `None` for Files' Write.
    pub taken_scope: Option<Vec<faraday_text::Text>>,
    /// The sticks this session wrote a secret file on, or read one from,
    /// while they are in: names only, no content.
    pub unprotected: Vec<Unprotected>,
}

impl VisitState {
    /// Where `column`'s scrollbar was drawn, `None` when its list fits.
    pub fn bar(&self, column: Column) -> &std::cell::Cell<Option<VisitBarAt>> {
        &self.bars[match column {
            Column::Outbox => 0,
            Column::Stick => 1,
        }]
    }
}

impl Faraday {
    /// `name`, `size` bytes, was copied in from, or written to and read
    /// back on, the stick `stick`; `out_row` is whether a row For the
    /// stick of that name may show it.
    pub(crate) fn visit_same_mark(&mut self, stick: &str, name: &str, size: u64, out_row: bool) {
        if !super::visit_same::is_settings(name) {
            self.visit.same.put(stick, name, size, out_row);
        }
    }

    /// Whether the file `name` is in both places on the stick shown: known
    /// the same, and the listing still has it at that size. `size` is the
    /// Faraday copy's, where a row has one; `out_row` is whether the row
    /// is For the stick's.
    pub(crate) fn visit_same(&self, name: &str, size: Option<u64>, out_row: bool) -> bool {
        let Some(stick) = self.stick.get(self.visit.stick) else {
            return false;
        };
        let Some(m) = self.visit.same.find(&stick.id, name) else {
            return false;
        };
        size.is_none_or(|s| s == m.size)
            && (m.out_row || !out_row)
            && stick
                .files
                .iter()
                .any(|(n, len)| n.eq_ignore_ascii_case(name) && *len == m.size)
    }

    /// Whether a written and matched file of the last receipt is in both
    /// places: the receipt names the stick on show.
    pub(crate) fn visit_receipt_same(&self, r: &crate::Receipt, name: &str) -> bool {
        r.files.iter().any(|f| f.verified && f.name == name)
            && self.visit_stick_identity().is_some_and(|i| *i == *r.stick)
            && self.visit_same(name, None, false)
    }

    /// Asks the shell to hash each file of the stick on show that Faraday
    /// holds a copy of by name and size, and has not been checked: the
    /// answer says whether the two are the same ([`Faraday::visit_same_hashed`]).
    pub(crate) fn visit_same_ask(&mut self) {
        // The stick's files are read for it: not before Import (F130),
        // which the import's list standing for that stick says, or the
        // boot stick's first look, where the import is not yet started.
        let first_look = self.stick.settings().is_none()
            && self
                .stick
                .iter()
                .any(|s| s.boot && s.files.iter().any(|(n, _)| !super::settings::is_file(n)));
        if self.holds_secret() || first_look {
            return;
        }
        let Some(stick) = self.stick.get(self.visit.stick) else {
            return;
        };
        if self.import.unasked()
            && self.import.tile()
            && self
                .import
                .state()
                .is_some_and(|i| i.stick == stick.id.as_str())
        {
            return;
        }
        let held = |name: &str, size: u64| {
            self.outbox
                .iter()
                .chain(self.inbox.iter())
                .any(|i| i.name.eq_ignore_ascii_case(name) && i.bytes.len() as u64 == size)
        };
        let ask: Vec<(Text, u64)> =
            stick
                .files
                .iter()
                .filter(|(n, size)| {
                    !super::visit_same::is_settings(n)
                        && held(n, *size)
                        && self
                            .visit
                            .same
                            .find(&stick.id, n)
                            .is_none_or(|m| m.size != *size)
                        && !self.visit.same.asked.iter().any(|(s, a, z)| {
                            *s == stick.id && a.eq_ignore_ascii_case(n) && z == size
                        })
                })
                .map(|(n, size)| (n.clone(), *size))
                .collect();
        let id = stick.id.clone();
        for (name, size) in ask {
            self.visit.same.asked.push((id.clone(), name.clone(), size));
            self.out.storage_out.push_back(StorageCommand::Hash {
                stick: id.clone(),
                name,
            });
        }
    }

    /// The shell hashed `name` on `stick`: if a file of Faraday's has the
    /// same bytes, the two are marked the same. A failed read is asked
    /// again at the next listing.
    pub(crate) fn visit_same_hashed(
        &mut self,
        stick: &str,
        name: &str,
        sha: &Result<[u8; 32], Text>,
    ) {
        let Some(at) = self
            .visit
            .same
            .asked
            .iter()
            .position(|(s, n, _)| **s == *stick && n.eq_ignore_ascii_case(name))
        else {
            return;
        };
        let Ok(sha) = sha else {
            self.visit.same.asked.remove(at);
            return;
        };
        let size = self.visit.same.asked[at].2;
        let c = self.compute();
        let same = |i: &Item| i.name.eq_ignore_ascii_case(name) && i.bytes.len() as u64 == size;
        let out = self.outbox.iter().any(|i| same(i) && i.hash(&c) == *sha);
        let inn = self.inbox.iter().any(|i| same(i) && i.hash(&c) == *sha);
        if out || inn {
            self.visit_same_mark(stick, name, size, out);
        }
    }

    /// A write's answer onto the receipt: the first answer of a write
    /// replaces the last receipt with a new one.
    pub(crate) fn receipt_add(&mut self, name: &str, failure: Option<faraday_text::Text>) {
        let c = self.compute();
        let now = self.checked_now();
        let Some((name, hash, opened)) = self.visit.writing_answered(name, now) else {
            return;
        };
        if let Some(r) = opened {
            self.visit.set_receipt(Some(r));
        }
        let hash = self.hash_to_keep(&c, hash);
        self.visit.receipt_push(ReceiptFile {
            name: name.as_str().into(),
            hash,
            verified: failure.is_none(),
            failure: failure.map(|f| f.as_str().into()),
        });
    }

    /// Costs a read of the free memory (one small file). What a visit ticks to come in: every file
    /// on the stick shown that Faraday reads, but a seed's by its name
    /// (which comes in through its own tick), one over 18 MiB, and one
    /// From the stick already by name and size.
    pub(crate) fn visit_default_in(&mut self) -> std::collections::BTreeSet<faraday_text::Text> {
        // Which vault files fit is by what is free now.
        self.vaults.read_free();
        let Some(stick) = self.stick.get(self.visit.stick) else {
            return Default::default();
        };
        stick
            .files
            .iter()
            .filter(|(n, size)| {
                stick.kind_of(n).is_some()
                    && stick.unread_why(&self.vaults, n, *size).is_none()
                    && !seed_by_name(n)
                    && !self
                        .inbox
                        .iter()
                        .any(|i| i.name == **n && i.bytes.len() as u64 == *size)
            })
            .map(|(n, _)| faraday_text::Text::of(n))
            .collect()
    }

    /// Costs a hash of a whole file (through `hash`).
    /// [`Faraday::visit_all_out`], but with more than one vault file
    /// waiting, one vault for this stick: the
    /// first the last write did not write. The rest wait for the next
    /// stick.
    pub(crate) fn visit_default_out(
        &self,
        c: &Compute,
    ) -> std::collections::BTreeSet<faraday_text::Text> {
        let mut out = self.visit_all_out();
        let vaults: Vec<&Item> = self
            .outbox
            .iter()
            .filter(|i| i.kind == FileKind::Vault)
            .collect();
        if vaults.len() > 1 {
            let written = |i: &Item| {
                self.visit
                    .receipt()
                    .as_ref()
                    .is_some_and(|r| r.holds(&i.name, &i.hash(c)))
            };
            let one = vaults
                .iter()
                .find(|i| !written(i))
                .or(vaults.first())
                .map(|i| i.name.clone());
            for i in &vaults {
                if Some(&i.name) != one.as_ref() {
                    out.remove(&*i.name);
                }
            }
        }
        out
    }

    /// Keeps [`Self::visit_sign_file`] for the frame to read; costs a walk
    /// of the stick's file names, without a copy of them.
    pub(crate) fn visit_sign_refresh(&mut self) {
        let now = self.visit_sign_file();
        if now.as_deref() != self.visit.sign_file().map(|t| &**t) {
            self.visit.set_sign_file(now);
        }
    }

    /// The one PSBT on the stick on show, when it is ticked to come in
    /// and the stick holds no other: "Import and sign" names it.
    pub(crate) fn visit_sign_file(&self) -> Option<Text> {
        let stick = self.stick.get(self.visit.stick)?;
        let mut psbts = stick.files.iter().filter(|(n, _)| {
            n.len()
                .checked_sub(5)
                .is_some_and(|at| n.as_bytes()[at..].eq_ignore_ascii_case(b".psbt"))
        });
        let (name, _) = psbts.next()?;
        if psbts.next().is_some() || !self.visit.inn.contains(&**name) {
            return None;
        }
        Some(Text::of(name))
    }

    /// Reads every chosen stick file into the Inbox.
    pub(crate) fn visit_copy(&mut self) {
        if let Some(stick) = self.stick.get(self.visit.stick) {
            let id = stick.id.clone();
            self.visit.log_clear();
            self.vaults.read_free();
            for name in self.visit.inn.clone() {
                // A vault file this computer has not the memory for now is
                // not read; the row says what it needs.
                let size = stick
                    .files
                    .iter()
                    .find(|(n, _)| **n == *name)
                    .map_or(0, |(_, size)| *size);
                if stick.unread_why(&self.vaults, &name, size).is_some() {
                    self.visit.inn_unchoose(&name);
                    continue;
                }
                // A picture's QR codes are read, not its bytes.
                let stick = id.clone();
                if self.visit.reading_has(&name) {
                    continue;
                }
                self.visit.reading_add(name.clone());
                self.out.storage_out.push_back(
                    if faraday_text::Text::ascii_lowercase(&name).ends_with(".png") {
                        StorageCommand::ReadQr {
                            stick,
                            name: faraday_text::Text::of(&name),
                        }
                    } else {
                        StorageCommand::Read {
                            stick,
                            name: faraday_text::Text::of(&name),
                        }
                    },
                );
            }
        }
    }

    /// A stick visit column's list scrolled to where its scrollbar is
    /// held, at pixel row `y`: the thumb's middle under the finger.
    pub(crate) fn visit_drag(&mut self, column: Column, y: i32) {
        let Some((top, h, thumb, max)) = self.visit.bar(column).get() else {
            return;
        };
        let room = (h - thumb).max(1) as f32;
        let at = ((y - top) as f32 - thumb as f32 / 2.0).clamp(0.0, room);
        let offset = at / room * max;
        match column {
            Column::Outbox => self.visit.set_out_offset(offset),
            Column::Stick => self.list_offset = offset,
        }
    }
}

/// A stick holding secret files this session wrote or read: its notice on
/// the visit. Names only (`keys.secret-files`).
#[derive(Default)]
pub struct Unprotected {
    stick: Text,
    label: Text,
    names: std::collections::BTreeSet<Text>,
    line: Text,
}

impl Unprotected {
    fn make_line(&mut self) {
        let names: Vec<&str> = self.names.iter().map(|n| &**n).collect();
        let list = Text::joined(&names, ", ");
        self.line = match self.names.len() {
            1 => Text::fill(EN.unprotected_one, &[&self.label, &list]),
            n => Text::fill(
                EN.unprotected_many,
                &[&self.label, &Text::num(n as u64), &list],
            ),
        };
    }
}

impl Faraday {
    /// The vault, by place in [`Faraday::vault_files`], a stick visit
    /// unlocks once the stick is pulled: a locked one sealed for it, still
    /// For the stick or written by the last write.
    fn visit_unlocks(&self) -> Option<usize> {
        self.vault_files().iter().position(|f| {
            f.open.is_none()
                && (f.in_outbox
                    || self
                        .visit
                        .receipt()
                        .as_ref()
                        .is_some_and(|r| r.wrote(&f.name)))
        })
    }

    /// Every Outbox file a visit may write unasked: all but an
    /// unprotected secret.
    fn visit_all_out(&self) -> std::collections::BTreeSet<faraday_text::Text> {
        self.outbox
            .iter()
            .filter(|i| i.exposure() != secrets::Exposure::Secret)
            .filter(|i| !self.in_the_backup(&i.name))
            .map(|i| faraday_text::Text::of(&i.name))
            .collect()
    }

    /// Whether the stick on show holds the Inbox file `item`, by name and
    /// size.
    fn visit_stick_holds(&self, item: &Item) -> bool {
        self.stick.get(self.visit.stick).is_some_and(|s| {
            s.files
                .iter()
                .any(|(n, len)| **n == *item.name && *len == item.bytes.len() as u64)
        })
    }

    /// The Inbox files a stick visit's left column shows, by index, each
    /// with whether the stick on show holds it already (by name and size):
    /// those rows are drawn greyed and cannot be ticked.
    pub(crate) fn visit_inbox_shown(&self) -> Vec<(usize, bool)> {
        self.inbox
            .iter()
            .enumerate()
            .map(|(k, i)| (k, self.visit_stick_holds(i)))
            .collect()
    }

    /// The Inbox files a stick visit offers to write, by index: every one
    /// but those the stick shown holds already, by name and size.
    pub fn visit_inbox_rows(&self) -> Vec<usize> {
        self.visit_inbox_shown()
            .into_iter()
            .filter(|&(_, held)| !held)
            .map(|(k, _)| k)
            .collect()
    }

    /// The Inbox files Select all chooses on a visit: the public and
    /// sealed ones it lists, never one that may be a secret.
    fn visit_inbox_plain(&self) -> BTreeSet<faraday_text::Text> {
        self.visit_inbox_rows()
            .into_iter()
            .filter_map(|k| self.inbox.get(k))
            .filter(|i| i.copy_ack().is_none())
            .map(|i| faraday_text::Text::of(&i.name))
            .collect()
    }
}

// A visit's files are the Inbox's and the Outbox's; a vault write waiting for its name's answer is not kept.
impl crate::at_lock::AtLock for VisitArea {
    fn holds(&self) -> crate::at_lock::Holds {
        // A transfer in parts read from pictures cannot be classed, so it
        // counts as held, as the camera's parts do.
        crate::at_lock::Holds::typed(
            self.pictures_scan
                .as_ref()
                .is_some_and(crate::ScanState::in_parts),
        )
    }
    fn lock(&mut self) {
        self.pictures_scan = None;
        self.taken_clear();
        self.name_taken_clear();
        self.clear_after_visit();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_secret_file_is_not_put_in_the_outbox_beside_a_public_one() {
        let mut app = crate::testkit::started();
        let c = crate::testkit::compute();
        let before = app.outbox.len();
        let mut secret = crate::Item::new(&c, "key.txt", b"secret".to_vec());
        secret.secret = true;
        app.put_item(secret);
        assert_eq!(app.outbox.len(), before);
        app.put_item(crate::Item::new(&c, "note.txt", b"hello".to_vec()));
        assert_eq!(app.outbox.len(), before + 1);
    }
}
