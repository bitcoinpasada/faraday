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
use crate::ui::{Style, Ui, W};
use crate::{Action, Faraday, Sheet, StorageCommand, sha256_of, stick_settings};

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

/// Whether these bytes are a sealed vault, written by `write_vault`.
fn is_vault(name: &str, bytes: &[u8]) -> bool {
    name.ends_with(".ofv") && bytes.starts_with(b"OFVT")
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

impl Faraday {
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

    /// The files the write would write whose names are taken on the stick
    /// shown, each with what becomes of it.
    pub fn visit_taken(&self) -> Vec<Taken> {
        let Some(stick) = self.sticks.get(self.visit.stick) else {
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
            let fate = if is_vault(&name, &bytes) && *size == bytes.len() as u64 {
                Fate::SameVault
            } else {
                let keep_as = free_name(&names, &name);
                names.push(keep_as.to_ascii_lowercase());
                if is_vault(&name, &bytes) {
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
    pub(crate) fn visit_write_ask(&mut self) {
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
        let c = self.compute();
        if self.sheet == Some(Sheet::Taken) {
            self.sheet = None;
        }
        let taken: Vec<String> = std::mem::take(&mut self.visit.taken)
            .into_iter()
            .filter(|t| matches!(t.fate, Fate::File { .. }))
            .map(|t| t.name)
            .collect();
        let Some(stick) = self.sticks.get(self.visit.stick) else {
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
                stick_settings::FILE.to_string(),
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
            let over = if is_vault(&name, &bytes) {
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
            let renamed = self.files_menu.renamed.contains(&name);
            self.storage_out.push_back(StorageCommand::Write {
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
            cy += crate::compact::sheet_head(ui, x, cy, iw, Icon::File, TEXT, &title);
        } else {
            ui.icon(x, cy, 30.0, Icon::File, 18.0, TEXT);
            let t = ui.fit(20.0, W::S, &title, iw - 42.0);
            ui.text_mid(x + 42.0, cy, 30.0, 20.0, W::S, TEXT, &t);
            cy += 46.0;
        }
        for line in &rows {
            cy += ui.wrap(x, cy, iw, 13.0, W::M, TEXT, line) + 10.0;
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
