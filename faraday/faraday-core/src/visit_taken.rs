//! Writing over a file on a stick asks first (HANDOFF item 68.5,
//! `docs/FLOWS.md` the stick visit). Before a visit's Write, the files it
//! would write whose names are already on the stick, ignoring case, bring
//! up **Already on the stick**, a row each, and Replace, Keep both or
//! Cancel. A vault going back over itself is always replaced, a vault
//! under another vault's name never is, and the settings file is
//! Faraday's own and not listed. With nothing taken there is no sheet.

use crate::drawing::View;
use crate::strings::EN;
use crate::strings::fill;
use faraday_draw::widgets::Icon;

use crate::compact::{buttons, scroll_sheet};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::{Action, Faraday, READ_MAX, Screen, Sheet, StorageCommand, sha256_of, stick};
use faraday_draw::tokens::BODY_M;

/// What becomes of one file whose name is taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fate {
    /// A file: replaced, or kept beside the old one as `keep_as`.
    File {
        /// The name Keep both writes it under.
        keep_as: String,
    },
    /// The same vault, sealed again here: always written over its own
    /// file.
    SameVault,
    /// Another vault under the name: never replaced, written as
    /// `keep_as` whatever is chosen.
    OtherVault {
        /// The name it is written under.
        keep_as: String,
    },
}

/// One row of the sheet: the name asked for, and what becomes of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Taken {
    /// The name asked for.
    pub name: String,
    /// What becomes of it.
    pub fate: Fate,
}

impl Taken {
    /// The row as the sheet says it.
    pub fn line(&self) -> String {
        match &self.fate {
            Fate::File { keep_as } => fill(
                EN.visit_taken_keep_both_n,
                &[&self.name.to_string(), &keep_as.to_string()],
            ),
            Fate::SameVault => fill(EN.visit_taken_the_same, &[&self.name.to_string()]),
            Fate::OtherVault { keep_as } => fill(
                EN.visit_taken_another,
                &[&self.name.to_string(), &keep_as.to_string()],
            ),
        }
    }
}

/// Whether these bytes are a vault, whatever the file is named: the
/// magic and a vault length (`docs/VAULT-HARDENING.md` §2.1).
fn is_vault(bytes: &[u8]) -> bool {
    faraday_intake::is_vault(bytes)
}

/// The first of `name`, `stem-2.ext`, `stem-3.ext`… none of `taken`
/// (lower case), as `faraday_files` names a file kept beside another.
fn free_name(taken: &[String], name: &str) -> String {
    if !taken.contains(&name.to_ascii_lowercase()) {
        return name.to_string();
    }
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    (2..100)
        .map(|n| format!("{stem}-{n}{ext}"))
        .find(|c| !taken.contains(&c.to_ascii_lowercase()))
        .unwrap_or_else(|| name.to_string())
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
            let Some((_, size)) = on else {
                continue;
            };
            // A vault of the same length is taken for the same one; the
            // write itself checks its salt and never goes over another.
            let fate = if is_vault(&bytes) && *size == bytes.len() as u64 {
                Fate::SameVault
            } else {
                let keep_as = free_name(&names, &name);
                names.push(keep_as.to_ascii_lowercase());
                if is_vault(&bytes) {
                    Fate::OtherVault { keep_as }
                } else {
                    Fate::File { keep_as }
                }
            };
            rows.push(Taken { name, fate });
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
        if taken.iter().any(|t| matches!(t.fate, Fate::File { .. })) {
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
            .filter(|t| matches!(t.fate, Fate::File { .. }))
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
            let renamed = self.visit.files_menu_mut().renamed.contains(&name);
            self.out.storage_out.push_back(StorageCommand::Write {
                stick: id.clone(),
                name,
                bytes,
                replace,
                renamed,
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
