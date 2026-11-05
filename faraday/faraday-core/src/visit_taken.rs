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
use crate::strings::fill;
use faraday_draw::widgets::Icon;

use crate::compact::{buttons, scroll_sheet};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::{Action, Faraday, READ_MAX, Screen, Sheet, StorageCommand, sha256_of, stick};
use faraday_draw::tokens::BODY_M;

/// One row of the sheet: a file whose name is taken, and the name Keep
/// both writes it under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Taken {
    /// The name asked for.
    pub name: String,
    /// The name Keep both writes it under.
    pub keep_as: String,
}

impl Taken {
    /// The row as the sheet says it.
    pub fn line(&self) -> String {
        fill(
            EN.visit_taken_keep_both_n,
            &[&self.name.to_string(), &self.keep_as.to_string()],
        )
    }
}

/// A vault's write the disk process refused for its name, waiting for the
/// person's answer (`docs/VAULT.md` §6 step 1). What the sheet says was
/// decided when the refusal arrived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NameTaken {
    /// The stick's id.
    pub(crate) stick: String,
    /// The name the vault was to be written under.
    pub(crate) name: String,
    /// The file on the stick that has it.
    pub(crate) on_stick: String,
    /// That file's SHA-256, when it could be read.
    pub(crate) sha: Option<[u8; 32]>,
    /// The backup flow's write, not the stick visit's.
    pub(crate) flow: bool,
    /// The line under the title.
    pub(crate) body: String,
    /// The free name Save beside writes it under.
    pub(crate) beside: String,
    /// The SHA-256 of the bytes the write sent, which the answer sends
    /// again; none where the write is no longer known.
    pub(crate) sent: Option<[u8; 32]>,
}

/// Whether these bytes are a vault, whatever the file is named: the
/// magic and a vault length (`docs/VAULT-HARDENING.md` §2.1).
pub(crate) fn is_vault(bytes: &[u8]) -> bool {
    faraday_intake::is_vault(bytes)
}

/// The first of `name`, `stem-2.ext`, `stem-3.ext`… none of `taken`
/// (lower case), as `faraday_files` names a file kept beside another.
pub(crate) fn free_name(taken: &[String], name: &str) -> String {
    if !taken.contains(&name.to_ascii_lowercase()) {
        return name.to_string();
    }
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    let mut n = 2u32;
    loop {
        let beside = format!("{stem}-{n}{ext}");
        if !taken.contains(&beside.to_ascii_lowercase()) {
            return beside;
        }
        n += 1;
    }
}

/// The first of `name`, `stem-2.ext`, `stem-3.ext`… that no file waiting
/// in `outbox` is called, so that a file put there under a name it shares
/// with others goes beside the one waiting rather than over it.
pub(crate) fn outbox_free_name(outbox: &[crate::Item], name: &str) -> String {
    let taken: Vec<String> = outbox.iter().map(|i| i.name.to_ascii_lowercase()).collect();
    free_name(&taken, name)
}

/// The stick visit's area: its state, the pull waiting, the receipt of
/// what was written, the vault files written from, Files' row menus and
/// the For the stick list's scroll. The visit state is reached through
/// [`std::ops::Deref`] and [`std::ops::DerefMut`]; the rest through the
/// accessors below.
#[derive(Default)]
pub(crate) struct VisitArea {
    visit: crate::VisitState,
    /// What was pressed with a stick attached, done when it is pulled
    /// ([`Sheet::Pull`]).
    pull: Option<Action>,
    /// The flow a stick visit returns to when the stick is pulled.
    after_visit: Option<Screen>,
    /// What the last stick visit wrote, this power-on (§4.3).
    receipt: Option<crate::Receipt>,
    /// Each vault file read from a stick this power-on: its name, the
    /// hash of its bytes as read, and the stick's label (§3.5).
    vault_from: Vec<(String, [u8; 32], String)>,
    /// A file row's ⋮ menu, Rename and Details (`docs/FLOWS.md`
    /// decision 61).
    files_menu: crate::file_menu::FileMenus,
    /// How far Files' For the stick list is scrolled on a desktop, apart
    /// from From the stick's.
    files_out_offset: f32,
    /// The vault writes refused for their names, the first on the
    /// name-taken sheet ([`Sheet::NameTaken`]).
    name_taken: std::collections::VecDeque<NameTaken>,
    /// The names this write wrote, for the free name the sheet offers.
    wrote: Vec<String>,
}

impl std::ops::Deref for VisitArea {
    type Target = crate::VisitState;
    fn deref(&self) -> &crate::VisitState {
        &self.visit
    }
}

impl std::ops::DerefMut for VisitArea {
    fn deref_mut(&mut self) -> &mut crate::VisitState {
        &mut self.visit
    }
}

impl VisitArea {
    /// What was pressed with a stick attached, done when it is pulled.
    pub(crate) fn pull(&self) -> &Option<Action> {
        &self.pull
    }

    /// The press waiting for the pull, to change it.
    pub(crate) fn pull_mut(&mut self) -> &mut Option<Action> {
        &mut self.pull
    }

    /// The flow a stick visit returns to when the stick is pulled.
    pub(crate) fn after_visit(&self) -> &Option<Screen> {
        &self.after_visit
    }

    /// The flow the visit returns to, to change it.
    pub(crate) fn after_visit_mut(&mut self) -> &mut Option<Screen> {
        &mut self.after_visit
    }

    /// What the last write to a stick wrote.
    pub(crate) fn receipt(&self) -> &Option<crate::Receipt> {
        &self.receipt
    }

    /// The receipt, to change it.
    pub(crate) fn receipt_mut(&mut self) -> &mut Option<crate::Receipt> {
        &mut self.receipt
    }

    /// Vault files written from: name, hash and the stick's id.
    pub(crate) fn vault_from(&self) -> &Vec<(String, [u8; 32], String)> {
        &self.vault_from
    }

    /// The first vault file read from a stick that `matches`.
    pub(crate) fn vault_from_find(
        &self,
        matches: impl Fn(&(String, [u8; 32], String)) -> bool,
    ) -> Option<&(String, [u8; 32], String)> {
        self.vault_from.iter().find(|x| matches(x))
    }

    /// The vault files written from, to change them.
    pub(crate) fn vault_from_mut(&mut self) -> &mut Vec<(String, [u8; 32], String)> {
        &mut self.vault_from
    }

    /// The vault writes waiting for an answer to the name-taken sheet.
    pub(crate) fn name_taken(&self) -> &std::collections::VecDeque<NameTaken> {
        &self.name_taken
    }

    /// The queue of name-taken answers, to change it.
    pub(crate) fn name_taken_mut(&mut self) -> &mut std::collections::VecDeque<NameTaken> {
        &mut self.name_taken
    }

    /// A name this write wrote.
    pub(crate) fn wrote_add(&mut self, name: &str) {
        self.wrote.push(name.to_string());
    }

    /// A new write starts with no names written.
    pub(crate) fn wrote_clear(&mut self) {
        self.wrote.clear();
    }

    /// Files' row menus.
    pub(crate) fn files_menu(&self) -> &crate::file_menu::FileMenus {
        &self.files_menu
    }

    /// Files' row menus, to change them.
    pub(crate) fn files_menu_mut(&mut self) -> &mut crate::file_menu::FileMenus {
        &mut self.files_menu
    }

    /// How far the For the stick list is scrolled.
    pub(crate) fn files_out_offset(&self) -> &f32 {
        &self.files_out_offset
    }

    /// The scroll of the For the stick list, to change it.
    pub(crate) fn files_out_offset_mut(&mut self) -> &mut f32 {
        &mut self.files_out_offset
    }
}

impl Faraday {
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

    /// How far Files' For the stick list is scrolled on a desktop.
    pub fn files_out_offset(&self) -> f32 {
        *self.visit.files_out_offset()
    }

    /// What the visit's Write writes, the settings file aside: each
    /// chosen Outbox file, then each chosen Inbox file the visit lists,
    /// with its bytes and whether it came from the Inbox.
    fn visit_files(&self) -> Vec<(String, Vec<u8>, bool)> {
        let mut out: Vec<(String, Vec<u8>, bool)> = self
            .outbox
            .iter()
            .filter(|i| self.visit.out.contains(&i.name))
            .map(|i| (i.name.clone(), i.bytes.clone(), false))
            .collect();
        for k in self.visit_inbox_rows() {
            let item = &self.inbox[k];
            if self.visit.from_inbox.contains(&item.name) {
                out.push((item.name.clone(), item.bytes.clone(), true));
            }
        }
        out
    }

    /// The Inbox files a lock drops ([`crate::Item::dropped_at_lock`]),
    /// by name.
    pub fn lock_drops(&self) -> Vec<String> {
        self.inbox
            .iter()
            .filter(|i| i.dropped_at_lock())
            .map(|i| i.name.clone())
            .collect()
    }

    /// A write the disk refused: the receipt says why, and the visit's log
    /// says what becomes of the file.
    pub(crate) fn visit_write_failed(&mut self, name: &str, reason: &str) {
        self.receipt_add(name, Some(reason.to_string()));
        self.visit_write_logged(name, reason);
    }

    /// A write given up before anything was written: no receipt row, so
    /// that the proof of an earlier write stays; the log says why.
    fn visit_write_cancelled(&mut self, name: &str, reason: &str) {
        if let Some((_, _, sent, _)) = self.visit.writing.as_mut() {
            sent.retain(|(n, _)| n != name);
            if sent.is_empty() {
                self.visit.writing = None;
            }
        }
        self.visit_write_logged(name, reason);
    }

    /// The visit's log line for a file not written.
    fn visit_write_logged(&mut self, name: &str, reason: &str) {
        let stays = if self.visit.writing_inbox.remove(name) {
            EN.storage_event_it_stays.to_string()
        } else {
            fill(EN.storage_event_it_still, &[self.medium.for_the()])
        };
        let line = fill(EN.storage_event_not_written, &[name, reason, &stays]);
        self.visit.log.push((line, false));
    }

    /// The files the write would write whose names are taken on the stick
    /// shown, each with what becomes of it.
    pub fn visit_taken(&self) -> Vec<Taken> {
        let Some(stick) = self.stick.get(self.visit.stick) else {
            return Vec::new();
        };
        let mut names: Vec<String> = stick
            .files
            .iter()
            .map(|(n, _)| n.to_ascii_lowercase())
            .collect();
        let mut rows = Vec::new();
        for (name, bytes, _) in self.visit_files() {
            let on = stick
                .files
                .iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(&name));
            let Some(_) = on else {
                continue;
            };
            // A vault is asked by the disk process, which reads the file.
            if is_vault(&bytes) {
                continue;
            }
            let keep_as = free_name(&names, &name);
            names.push(keep_as.to_ascii_lowercase());
            rows.push(Taken { name, keep_as });
        }
        rows
    }

    /// The visit's Write: the sheet first when a file's name is taken and
    /// the choice decides what becomes of it; otherwise written at once.
    ///
    /// A process that is no longer clean asks to lock first, as Write from
    /// Files does; the disk process would refuse the write.
    pub(crate) fn visit_write_ask(&mut self) {
        if !self.clean() {
            self.sheet = Some(Sheet::WriteOut);
            return;
        }
        let taken = self.visit_taken();
        if !taken.is_empty() {
            self.visit.taken = taken;
            self.sheet = Some(Sheet::Taken);
        } else {
            self.visit_write(false);
        }
    }

    /// Writes the settings file when its row is on, and every chosen
    /// file; a taken file over the old one when `replace`.
    pub(crate) fn visit_write(&mut self, replace: bool) {
        if !self.clean() {
            self.visit.taken.clear();
            self.sheet = Some(Sheet::WriteOut);
            return;
        }
        let c = self.compute();
        if self.sheet == Some(Sheet::Taken) {
            self.sheet = None;
        }
        let taken: Vec<String> = std::mem::take(&mut self.visit.taken)
            .into_iter()
            .map(|t| t.name)
            .collect();
        let Some(stick) = self.stick.get(self.visit.stick) else {
            return;
        };
        let id = stick.id.clone();
        let label = stick.label.clone();
        // A stick is told from another by its FAT volume serial; a folder
        // the shell lists has none, and is told by where it is.
        let identity = match stick.serial {
            Some(s) => format!("{s:08x}"),
            None => format!("id:{}", stick.id),
        };
        self.visit.log.clear();
        self.name_taken_wrote_clear();
        let mut sends: Vec<(String, Vec<u8>, bool)> = Vec::new();

        if self.visit_settings_on() {
            sends.push((
                stick::settings::FILE.to_string(),
                self.settings_file_text().into_bytes(),
                false,
            ));
        }
        for (name, bytes, from_inbox) in self.visit_files() {
            if from_inbox {
                self.visit.writing_inbox.insert(name.clone());
            }
            // A vault goes over the same vault on the stick (same salt and
            // length, decided where the stick is read) only when this
            // session unlocked or wrote it: a copy from Files that it did
            // not could be older than what the stick holds.
            let over = if is_vault(&bytes) {
                !from_inbox || self.vault_is_this_sessions(&c, &bytes)
            } else {
                replace && taken.contains(&name)
            };
            sends.push((name, bytes, over));
        }
        // What this write sends, for its receipt.
        let sent: Vec<(String, [u8; 32])> = sends
            .iter()
            .map(|(n, b, _)| (n.clone(), sha256_of(&c, b)))
            .collect();
        for (name, bytes, replace) in sends {
            let vault = is_vault(&bytes);
            let renamed = self.visit.files_menu_mut().renamed.contains(&name);
            let copies = self.vault_copies(&c, &bytes);
            self.out.storage_out.push_back(StorageCommand::Write {
                stick: id.clone(),
                name,
                bytes,
                copies,
                replace,
                renamed,
                // A vault's name taken by a file nothing proves is asked.
                on_taken: if vault { OnTaken::Ask } else { OnTaken::Beside },
            });
        }
        if !sent.is_empty() {
            self.visit.writing = Some((label, identity, sent, false));
        }
    }
}

/// The Already on the stick sheet: a row per taken file, then Replace,
/// Keep both and Cancel.
pub(crate) fn taken_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let title = fill(EN.visit_taken_already, &[app.medium.on_the()]);
    let rows: Vec<String> = app.visit.taken.iter().map(Taken::line).collect();
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
    let title = fill(EN.vault_taken_title, &[&t.name, app.medium.on_the()]);
    let overwrite = fill(EN.vault_taken_overwrite, &[&t.on_stick]);
    let beside = fill(EN.vault_taken_beside, &[&t.beside]);
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
                cy += ui.button_height(faraday_draw::tokens::BUTTON_H) + faraday_draw::tokens::GAP;
            }
            cy - y
        },
        &[
            (EN.visit_taken_cancel, Style::Ghost, Action::Cancel),
            (&beside, Style::Primary, Action::NameTakenBeside),
        ],
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
            self.visit
                .writing
                .as_ref()
                .and_then(|(_, _, s, _)| s.iter().find(|(n, _)| *n == name).map(|(_, h)| *h))
        };
        // What the file is is said only where the session knows both the
        // bytes sent and the file as copies, of two different vaults.
        let ours = self.vaults.vault_id_of_file(sent);
        let body = match self.vaults.summary_of_file(sha) {
            _ if sha.is_none() => EN.vault_taken_unreadable.to_string(),
            Some(v) if ours.is_some_and(|id| id != v.id) => {
                let known = if v.name.is_empty() { &v.file } else { &v.name };
                fill(EN.vault_taken_other, &[known])
            }
            _ => EN.vault_taken_unknown.to_string(),
        };
        let beside = self.name_taken_beside(&stick, &name);
        self.visit.name_taken_mut().push_back(NameTaken {
            stick,
            name,
            on_stick,
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
    ) -> Option<(Vec<u8>, bool)> {
        if flow {
            self.outbox
                .iter()
                .chain(self.inbox.iter())
                .find(|i| i.name == name && sha256_of(c, &i.bytes) == sent)
                .filter(|i| i.exposure() != crate::secrets::Exposure::Secret)
                .map(|i| (i.bytes.clone(), true))
        } else {
            self.visit_files()
                .into_iter()
                .find(|(n, b, _)| n == name && sha256_of(c, b) == sent)
                .map(|(_, bytes, from_inbox)| {
                    let replace = !from_inbox || self.vault_is_this_sessions(c, &bytes);
                    (bytes, replace)
                })
        }
    }

    /// The free name beside what `stick` lists and what this write wrote.
    fn name_taken_beside(&self, stick: &str, name: &str) -> String {
        let mut names: Vec<String> = self
            .stick
            .iter()
            .find(|s| s.id == stick)
            .map(|s| {
                s.files
                    .iter()
                    .map(|(n, _)| n.to_ascii_lowercase())
                    .collect()
            })
            .unwrap_or_default();
        names.extend(self.visit.wrote.iter().map(|n| n.to_ascii_lowercase()));
        free_name(&names, name)
    }

    /// The free names of the queue worked out again.
    fn name_taken_refresh(&mut self) {
        let besides: Vec<String> = self
            .visit
            .name_taken()
            .iter()
            .map(|t| self.name_taken_beside(&t.stick, &t.name))
            .collect();
        for (t, b) in self.visit.name_taken_mut().iter_mut().zip(besides) {
            t.beside = b;
        }
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
    }

    /// A new write starts with no names written.
    pub(crate) fn name_taken_wrote_clear(&mut self) {
        self.visit.wrote_clear();
    }

    /// The flow's write ended: the questions of its files go with it.
    pub(crate) fn name_taken_flow_ended(&mut self) {
        self.visit.name_taken_mut().retain(|t| !t.flow);
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
            let all: Vec<NameTaken> = self.visit.name_taken_mut().drain(..).collect();
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
        self.visit.name_taken_mut().pop_front();
        let c = self.compute();
        // The same bytes and flags as the first send found, by name and
        // SHA-256; none is a cancel.
        let sent = t
            .sent
            .filter(|_| !t.flow || self.flow_wants_written(&t.stick, &t.name))
            .and_then(|h| self.name_taken_sent(&c, t.flow, &t.name, h));
        match sent {
            Some((bytes, replace)) => {
                let renamed = !t.flow && self.visit.files_menu_mut().renamed.contains(&t.name);
                let copies = self.vault_copies(&c, &bytes);
                self.out.storage_out.push_back(StorageCommand::Write {
                    stick: t.stick,
                    name: t.name,
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
        let Some(t) = self.visit.name_taken_mut().pop_front() else {
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
            self.visit.name_taken_mut().retain(|o| !o.flow);
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
            .filter(|t| !self.stick.iter().any(|s| s.id == t.stick))
            .cloned()
            .collect();
        if gone.is_empty() {
            return;
        }
        let ids: Vec<String> = gone.iter().map(|t| t.stick.clone()).collect();
        self.visit
            .name_taken_mut()
            .retain(|t| !ids.contains(&t.stick));
        for t in gone {
            // A flow's write ends with its own pulled problem.
            if !t.flow {
                let reason = fill(EN.visit_taken_pulled, &[self.medium.noun()]);
                self.visit_write_cancelled(&t.name, &reason);
            }
        }
        self.name_taken_next();
    }
}

impl Faraday {
    /// Handles the stick visit's actions (`Action::Visit*`).
    pub(crate) fn visit_act(&mut self, action: Action) {
        match action {
            Action::VisitStick(i) => {
                self.visit.stick = i;
                self.visit.inn = self.visit_default_in();
                self.visit.settings = None;
            }
            Action::VisitSettings => {
                self.visit.settings = Some(!self.visit_settings_on());
            }
            Action::VisitOut(i) => {
                if let Some(item) = self
                    .outbox
                    .get(i)
                    .filter(|it| !self.in_the_backup(&it.name))
                {
                    let n = item.name.clone();
                    if !self.visit.out.remove(&n) {
                        self.visit.out.insert(n);
                    }
                }
            }
            Action::VisitIn(i) => {
                if let Some(stick) = self.stick.get(self.visit.stick)
                    && let Some((n, size)) = stick.files.get(i)
                    && *size <= READ_MAX
                {
                    let n = n.clone();
                    if !self.visit.inn.remove(&n) {
                        self.visit.inn.insert(n);
                    }
                }
            }
            // Unselect all: everything comes in by default (§4.6).
            Action::VisitInAll => self.visit.inn.clear(),
            Action::VisitOutAll => {
                // Every row but an unprotected secret, which is ticked one
                // at a time (FLOWS.md decision 6); from the Inbox, every
                // public or sealed file, and none that may be a secret.
                let all = self.visit_all_out();
                let inbox = self.visit_inbox_plain();
                let every = self.visit_settings_on()
                    && all.is_subset(&self.visit.out)
                    && inbox.is_subset(&self.visit.from_inbox);
                if every {
                    self.visit.out.clear();
                    for n in &inbox {
                        self.visit.from_inbox.remove(n);
                    }
                } else {
                    self.visit.out.extend(all);
                    self.visit.from_inbox.extend(inbox);
                }
                self.visit.settings = Some(!every);
            }
            Action::VisitInbox(i) => self.visit_inbox(i),
            Action::VisitBar(_) => {}
            Action::VisitWrite => self.visit_write_ask(),
            Action::VisitReplace => self.visit_write(true),
            Action::VisitKeepBoth => self.visit_write(false),
            Action::NameTakenBeside => self.name_taken_answer(false),
            Action::NameTakenOverwrite => self.name_taken_answer(true),
            Action::VisitCopy => self.visit_copy(),
            Action::VisitCopyAndLoad => {
                let inn = self.visit.inn.clone();
                self.visit.load_after.extend(inn);
                self.visit_copy();
                // Straight to the Inbox, where they arrive; the stick's
                // pull loads them from there.
                self.act(Action::Nav(Screen::Files));
            }
            Action::VisitFrom(back) => {
                if !self.stick.is_empty() && !self.holds_secret() {
                    *self.visit.after_visit_mut() = Some(back);
                    self.act(Action::Nav(Screen::Visit));
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
    pub(crate) fn toast_out_files(&mut self, names: &[String]) {
        match names {
            [] => {}
            [name] => self.toast_out(name),
            more => self.toast_waits(EN.toast_out_files_wait, &[&more.len().to_string()]),
        }
    }

    /// A toast `template` names the medium in: its `{}` are `what`, then
    /// "for the stick" or "for the SD card".
    pub(crate) fn toast_waits(&mut self, template: &str, what: &[&str]) {
        let mut args = what.to_vec();
        args.push(self.medium.for_the());
        let text = fill(template, &args);
        self.toast(&text);
    }
}
