//! A ⋮ on every file row (`docs/FLOWS.md` decision 61, F54). A file row
//! in Files and on the stick visit keeps the one action it is for as a
//! button and puts the rest behind a ⋮ at its right end, drawn and
//! reached as the backup board's chips' menu is ([`crate::menu`]).
//!
//! - **From the stick**: Add to vault where it is not the row's button,
//!   Details, Remove. Faraday's copies are not renamed (item 3).
//! - **For the stick**: Show as QR or Make PDF, Rename, Details, Remove.
//!   A rename is the name the next visit writes the file under.
//! - **On the stick** (the visit): Rename, Delete from the stick (the
//!   Delete files sheet with that file alone chosen), Details. The disk
//!   process renames the file's directory entry, and refuses a name
//!   another file on the stick has, ignoring case.
//!
//! **Rename** is a popup: the name before its extension in a field, the
//! extension fixed after it. **Details** is a sheet: the name, the row's
//! kind line, the size in bytes and the SHA-256 in groups of eight.

use crate::strings::fill;
use faraday_crypto::work::Compute;
use faraday_draw::tokens;
use faraday_draw::widgets::Icon;
use faraday_shell_api::Key as KeyIn;

use crate::compact::{buttons, scroll_sheet};
use crate::drawing::View;
use crate::menu::{self, Line, Step};
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::{Action, Faraday, FileKind, Screen, Sheet, StorageCommand, stick};

/// A file row with a ⋮, by its place in its list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    /// Files' From the stick, by its place in the Inbox.
    In(usize),
    /// Files' For the stick, by its place in the Outbox.
    Out(usize),
    /// A file on the stick the visit shows, by its place in its listing.
    Stick(usize),
}

/// A press about a file row's ⋮ (`docs/FLOWS.md` decision 61).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilePress {
    /// The ⋮: the row's menu opened.
    Menu(Row),
    /// The menu, the Rename popup or the Details sheet closed with
    /// nothing done: Escape, Cancel, Close or a press outside.
    Close,
    /// **Rename**: its popup.
    Rename(Row),
    /// The Rename popup's field, pressed.
    Field,
    /// The Rename popup's **Rename**.
    RenameGo,
    /// **Details**: its sheet.
    Details(Row),
    /// **Delete from the stick**: the Delete files sheet with this file
    /// on the stick alone chosen.
    StickDelete(usize),
}

/// The Rename popup's field, as the caret and the on-screen keyboard
/// know it.
pub(crate) const FIELD: Action = Action::File(FilePress::Field);

/// Why the disk process refused a rename on a stick, nothing written
/// (`docs/FLOWS.md` decision 61 item 4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unrenamed {
    /// Another file on the stick has the name, ignoring case.
    Taken,
    /// That file is the same vault, by identity (`docs/VAULT.md` §6).
    SameVault,
    /// The stick refused, and why.
    Failed(String),
}

/// A file row's ⋮ menu while it is open: worked out on the press that
/// opened it, and read by the drawing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMenu {
    /// The row whose ⋮ opened it.
    pub row: Row,
    /// Its title: the file's name.
    pub title: String,
    /// Its choices, in order: what each reads, and its press.
    pub lines: Vec<(String, Action)>,
    /// The choice the arrows are on.
    pub at: usize,
    /// Open or moved by a key: the arrows' choice is drawn so.
    pub keys: bool,
    /// Where the ⋮ was drawn, in pixels: the popup goes under it.
    pub anchor: Option<(i32, i32, i32, i32)>,
}

/// The Rename popup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Renaming {
    /// The file renamed.
    pub row: Row,
    /// Its name now.
    pub from: String,
    /// What the field holds: the name before its extension.
    pub stem: String,
    /// The extension, fixed: `.ofv` for a vault; empty for a name with
    /// none.
    pub ext: String,
}

/// The Details sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Details {
    /// The file's name.
    pub name: String,
    /// The row's kind line.
    pub kind: String,
    /// Its size in bytes.
    pub size: u64,
    /// Its SHA-256; `None` while the stick is read for it.
    pub sha: Option<Result<[u8; 32], String>>,
    /// The stick it is on, for a file on the stick.
    pub stick: Option<String>,
}

/// What the file rows' ⋮s hold between presses.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileMenus {
    /// The menu open.
    pub menu: Option<FileMenu>,
    /// The Rename popup open.
    pub renaming: Option<Renaming>,
    /// The Details sheet open.
    pub details: Option<Details>,
    /// The ⋮ the keyboard is on: Tab and Shift+Tab move it, Enter or
    /// Space opens its menu.
    pub focus: Option<Row>,
    /// The files For the stick the person renamed, by their new names,
    /// until a visit writes them: the next write of each tells the disk
    /// process so, and a vault going over its same vault on that stick
    /// takes the new name. Any other stick keeps its own name for it.
    pub renamed: std::collections::BTreeSet<String>,
}

/// The characters a name on a stick cannot hold, beside control
/// characters (`docs/FLOWS.md` decision 61 item 2).
const NOT_IN_NAMES: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

/// The longest name, its extension with it.
const NAME_MAX: usize = 255;

/// A name split before its extension: `.ofv` for a vault, every file its
/// own; a name with no dot past its first character has none.
fn split(name: &str) -> (String, String) {
    match name.rfind('.') {
        Some(i) if i > 0 => (name[..i].to_string(), name[i..].to_string()),
        _ => (name.to_string(), String::new()),
    }
}

/// A SHA-256 as it is shown: its hex in groups of eight, four groups
/// a line.
pub fn sha_groups(sha: &[u8; 32]) -> [String; 2] {
    let group = |b: &[u8]| -> String { b.iter().map(|b| format!("{b:02x}")).collect() };
    let line = |half: &[u8]| -> String { half.chunks(4).map(group).collect::<Vec<_>>().join(" ") };
    [line(&sha[..16]), line(&sha[16..])]
}

/// Costs a SHA-256 pass over every byte: a file's SHA-256.
pub fn sha256(c: &Compute, bytes: &[u8]) -> [u8; 32] {
    crate::sha256_of(c, bytes)
}

impl Faraday {
    /// The file on the stick the visit shows at `k`, with its size: a
    /// lookup.
    fn stick_file(&self, k: usize) -> Option<(String, u64)> {
        self.stick.get(self.visit.stick)?.files.get(k).cloned()
    }

    /// Whether a file on the stick has a ⋮: every file Faraday lists but
    /// the settings file (the computer's own are not listed).
    pub(crate) fn stick_row_menu(name: &str) -> bool {
        !stick::settings::is_file(name)
    }

    /// The row's name, size and kind line: a lookup.
    fn file_row(&self, row: Row) -> Option<(String, u64, String)> {
        match row {
            Row::In(k) => self.inbox.get(k).map(|i| {
                (
                    i.name.clone(),
                    i.bytes.len() as u64,
                    crate::screens::kind_line(i.kind, i.bytes.len()),
                )
            }),
            Row::Out(k) => self.outbox.get(k).map(|i| {
                (
                    i.name.clone(),
                    i.bytes.len() as u64,
                    crate::screens::kind_line(i.kind, i.bytes.len()),
                )
            }),
            Row::Stick(k) => {
                let (name, size) = self.stick_file(k)?;
                let kind = match crate::stick_kind(&name).filter(|_| size <= crate::READ_MAX) {
                    Some(kind) => format!("{kind} · {}", crate::screens::size_text(size as usize)),
                    None => EN.visit_file_too_large.to_string(),
                };
                Some((name, size, kind))
            }
        }
    }

    /// The row's ⋮ menu's choices, in decision 61's order and only those
    /// the row has (a map read): From the stick, Add to vault where it is
    /// not the row's button, Details and Remove; For the stick, Show as
    /// QR or Make PDF, Rename, Details and Remove; on the stick, Rename,
    /// Delete from the stick and Details.
    pub(crate) fn file_menu_lines(&self, row: Row) -> Vec<(String, Action)> {
        let mut out: Vec<(String, Action)> = Vec::new();
        let details = (
            EN.file_menu_details.to_string(),
            Action::File(FilePress::Details(row)),
        );
        match row {
            Row::In(k) => {
                let Some(item) = self.inbox.get(k) else {
                    return out;
                };
                // Only while a vault is open (F60).
                if matches!(item.kind, FileKind::Wallet | FileKind::Words)
                    && self.vault_way(Screen::Files).is_none()
                {
                    out.push((
                        EN.add_to_vault_button.to_string(),
                        Action::Vault(crate::vault::VaultAction::AddFile(k)),
                    ));
                }
                out.push(details);
                out.push((EN.files_remove.to_string(), Action::InboxRemove(k)));
            }
            Row::Out(k) => {
                let Some(item) = self.outbox.get(k) else {
                    return out;
                };
                // A vault is larger than a QR transfer carries: it goes
                // by stick only.
                if item.kind == FileKind::Sheet && self.online {
                    out.push((EN.files_make_pdf.to_string(), Action::PdfOutbox(k)));
                }
                if crate::qr_fits(item) {
                    out.push((EN.files_show_as_qr.to_string(), Action::QrOutbox(k)));
                }
                out.push((
                    EN.file_menu_rename.to_string(),
                    Action::File(FilePress::Rename(row)),
                ));
                out.push(details);
                out.push((EN.files_remove.to_string(), Action::OutboxRemove(k)));
            }
            Row::Stick(k) => {
                if !self
                    .stick_file(k)
                    .is_some_and(|(n, _)| Self::stick_row_menu(&n))
                {
                    return out;
                }
                out.push((
                    EN.file_menu_rename.to_string(),
                    Action::File(FilePress::Rename(row)),
                ));
                out.push((
                    fill(EN.file_menu_delete_from_medium, &[self.medium.noun()]),
                    Action::File(FilePress::StickDelete(k)),
                ));
                out.push(details);
            }
        }
        out
    }

    /// A press about a file row's ⋮.
    pub(crate) fn file_act(&mut self, p: FilePress) {
        let c = self.compute();
        // A choice in the menu leaves it.
        if matches!(
            p,
            FilePress::Rename(_) | FilePress::Details(_) | FilePress::StickDelete(_)
        ) {
            self.file_menu_leave();
        }
        match p {
            FilePress::Menu(row) => self.file_menu_open(row, false),
            FilePress::Close => {
                self.file_menu_leave();
                self.visit.files_menu_mut().renaming = None;
                self.visit.files_menu_mut().details = None;
                if matches!(
                    self.sheet,
                    Some(Sheet::FileMenu | Sheet::Rename | Sheet::Details)
                ) {
                    self.sheet = None;
                }
            }
            FilePress::Rename(row) => {
                let Some((from, _, _)) = self.file_row(row) else {
                    return;
                };
                let (stem, ext) = split(&from);
                self.visit.files_menu_mut().renaming = Some(Renaming {
                    row,
                    from,
                    stem,
                    ext,
                });
                self.sheet = Some(Sheet::Rename);
                self.scroll.reset_sheet();
            }
            FilePress::Field => {}
            FilePress::RenameGo => self.file_rename_go(),
            FilePress::Details(row) => {
                let Some((name, size, kind)) = self.file_row(row) else {
                    return;
                };
                // Faraday's own copies are hashed here; a file on the
                // stick is read for it by the shell.
                let (sha, stick) = match row {
                    Row::In(k) => (self.inbox.get(k).map(|i| Ok(sha256(&c, &i.bytes))), None),
                    Row::Out(k) => (self.outbox.get(k).map(|i| Ok(sha256(&c, &i.bytes))), None),
                    Row::Stick(_) => {
                        let id = self.stick.get(self.visit.stick).map(|s| s.id.clone());
                        if let Some(id) = &id {
                            self.out.storage_out.push_back(StorageCommand::Hash {
                                stick: id.clone(),
                                name: name.clone(),
                            });
                        }
                        (None, id)
                    }
                };
                self.visit.files_menu_mut().details = Some(Details {
                    name,
                    kind,
                    size,
                    sha,
                    stick,
                });
                self.sheet = Some(Sheet::Details);
                self.scroll.reset_sheet();
            }
            FilePress::StickDelete(k) => {
                let Some((name, _)) = self.stick_file(k) else {
                    return;
                };
                self.delete_open();
                if let Some(d) = self.stick.delete_ask_mut() {
                    d.chosen = vec![name];
                }
            }
        }
    }

    /// A choice in an open ⋮ menu closes the menu before it acts.
    pub(crate) fn file_menu_chosen(&mut self, a: Action) {
        if self.sheet != Some(Sheet::FileMenu) || matches!(a, Action::File(_) | Action::Cancel) {
            return;
        }
        self.file_menu_leave();
        self.sheet = None;
    }

    /// The open menu gone: the keyboard stays on its ⋮ when the keyboard
    /// opened or moved in it.
    fn file_menu_leave(&mut self) {
        if let Some(m) = self.visit.files_menu_mut().menu.take()
            && m.keys
        {
            self.visit.files_menu_mut().focus = Some(m.row);
        }
    }

    /// Row `row`'s ⋮ menu opened: its choices worked out here, on the
    /// press (a map read). `keys`: opened by Enter or Space.
    fn file_menu_open(&mut self, row: Row, keys: bool) {
        let lines = self.file_menu_lines(row);
        let Some((title, _, _)) = self.file_row(row) else {
            return;
        };
        if lines.is_empty() {
            return;
        }
        let me = Action::File(FilePress::Menu(row));
        let anchor = self.pointer.rect_of(me).map(|r| (r.x, r.y, r.w, r.h));
        self.visit.files_menu_mut().menu = Some(FileMenu {
            row,
            title,
            lines,
            at: 0,
            keys,
            anchor,
        });
        if keys {
            self.visit.files_menu_mut().focus = Some(row);
        }
        self.sheet = Some(Sheet::FileMenu);
        self.scroll.reset_sheet();
    }

    /// Why the Rename popup's name cannot be used, as its line; `Err(None)`
    /// for the name unchanged, which is no line but nothing to do; else
    /// the new name with its extension.
    pub(crate) fn rename_checked(&self) -> Result<String, Option<String>> {
        let Some(r) = self.visit.files_menu().renaming.as_ref() else {
            return Err(None);
        };
        let stem = r.stem.trim();
        if stem.is_empty() {
            return Err(Some(EN.rename_empty.to_string()));
        }
        if stem.starts_with('.') {
            return Err(Some(EN.rename_dot.to_string()));
        }
        if stem
            .chars()
            .any(|c| c.is_control() || NOT_IN_NAMES.contains(&c))
        {
            return Err(Some(EN.rename_character.to_string()));
        }
        let to = format!("{stem}{}", r.ext);
        if to.chars().count() > NAME_MAX {
            return Err(Some(EN.rename_long.to_string()));
        }
        if to == r.from {
            return Err(None);
        }
        if let Row::Out(k) = r.row
            && self
                .outbox
                .iter()
                .enumerate()
                .any(|(j, i)| j != k && i.name.eq_ignore_ascii_case(&to))
        {
            return Err(Some(fill(
                EN.rename_taken_outbox,
                &[self.medium.for_the(), &to],
            )));
        }
        Ok(to)
    }

    /// The Rename popup's **Rename**: a file For the stick takes the new
    /// name, which the next visit writes it under; a file on the stick is
    /// asked of the disk process.
    fn file_rename_go(&mut self) {
        let Ok(to) = self.rename_checked() else {
            return;
        };
        let Some(r) = self.visit.files_menu_mut().renaming.take() else {
            return;
        };
        self.sheet = None;
        match r.row {
            Row::Out(k) => {
                let Some(item) = self.outbox.get_mut(k) else {
                    return;
                };
                item.name = to.clone();
                self.visit.files_menu_mut().renamed.remove(&r.from);
                self.visit.files_menu_mut().renamed.insert(to.clone());
                // Chosen on a visit under its old name, it stays chosen.
                if self.visit.out.remove(&r.from) {
                    self.visit.out.insert(to.clone());
                }
                // A vault open here seals again under the file's new name.
                if let Some(h) = (item.kind == FileKind::Vault)
                    .then(|| faraday_vault::read_header(&item.bytes).ok())
                    .flatten()
                {
                    self.vaults.rename_open(&h.salt, &r.from, &to);
                }
                self.save_boxes();
            }
            Row::Stick(_) => {
                if let Some(s) = self.stick.get(self.visit.stick) {
                    self.out.storage_out.push_back(StorageCommand::Rename {
                        stick: s.id.clone(),
                        from: r.from,
                        to,
                    });
                }
            }
            Row::In(_) => {}
        }
    }

    /// A rename on a stick answered: the visit says so, a file chosen
    /// stays chosen under its new name, and a backup's copy is named
    /// anew (`docs/FLOWS.md` decision 61 items 4 and 6). The listing is
    /// the shell's next.
    pub(crate) fn file_renamed(
        &mut self,
        stick: &str,
        from: &str,
        to: &str,
        refused: Option<Unrenamed>,
    ) {
        let noun = self.medium.noun();
        let line = match refused {
            None => {
                if self.visit.inn.remove(from) {
                    self.visit.inn.insert(to.to_string());
                }
                if let Some((label, serial)) = self
                    .stick
                    .iter()
                    .find(|s| s.id == stick)
                    .map(|s| (s.label.clone(), s.serial))
                {
                    self.flows.rename_written(&label, serial, from, to);
                }
                (fill(EN.rename_done, &[from, to]), true)
            }
            Some(Unrenamed::Taken) => (fill(EN.rename_taken_on_medium, &[to, noun]), false),
            Some(Unrenamed::SameVault) => (fill(EN.rename_same_vault, &[to, noun]), false),
            Some(Unrenamed::Failed(why)) => (fill(EN.rename_not_done, &[from, &why]), false),
        };
        self.visit.log.push(line);
    }

    /// A file on a stick hashed, for the Details sheet that asked.
    pub(crate) fn file_hashed(&mut self, stick: &str, name: &str, sha: Result<[u8; 32], String>) {
        if let Some(d) = self.visit.files_menu_mut().details.as_mut()
            && d.stick.as_deref() == Some(stick)
            && d.name == name
        {
            d.sha = Some(sha);
        }
    }

    /// A key for a file's ⋮ (`docs/BACKUP-FLOW.md` §17.20 item 5, as the
    /// board's): in an open menu the arrows move, Enter or Space chooses
    /// and Escape closes it back to its ⋮; in the Rename popup typing
    /// goes to its field, Enter renames and Escape closes; on Files and
    /// the visit, Tab and Shift+Tab move among the ⋮s on screen and Enter
    /// or Space opens the one the keyboard is on. Returns whether the key
    /// was taken.
    pub(crate) fn file_key(&mut self, key: KeyIn) -> bool {
        match self.sheet {
            Some(Sheet::FileMenu) => {
                let Some(m) = self.visit.files_menu_mut().menu.as_ref() else {
                    return false;
                };
                let n = m.lines.len();
                match menu::step(key, m.at, n) {
                    Some(Step::To(at)) => {
                        if let Some(m) = self.visit.files_menu_mut().menu.as_mut() {
                            m.at = at;
                            m.keys = true;
                        }
                    }
                    Some(Step::Choose) => {
                        if let Some(&(_, a)) = m.lines.get(m.at) {
                            self.press(a);
                        }
                    }
                    // Back to its ⋮, the keyboard on it.
                    Some(Step::Close) => {
                        if let Some(m) = self.visit.files_menu_mut().menu.as_mut() {
                            m.keys = true;
                        }
                        self.file_act(FilePress::Close);
                    }
                    None => {}
                }
                true
            }
            Some(Sheet::Rename) => {
                let Some(r) = self.visit.files_menu_mut().renaming.as_mut() else {
                    return false;
                };
                match key {
                    KeyIn::Char(c) if !c.is_control() => r.stem.push(c),
                    KeyIn::Backspace => {
                        r.stem.pop();
                    }
                    KeyIn::Enter => self.file_rename_go(),
                    KeyIn::Escape => self.file_act(FilePress::Close),
                    _ => {}
                }
                true
            }
            Some(Sheet::Details) => {
                if matches!(key, KeyIn::Escape | KeyIn::Enter) {
                    self.file_act(FilePress::Close);
                }
                true
            }
            None if matches!(self.screen, Screen::Files | Screen::Visit) => {
                let mut dots: Vec<Row> = Vec::new();
                for (_, a) in self.pointer.hits() {
                    if let Action::File(FilePress::Menu(r)) = a
                        && !dots.contains(r)
                    {
                        dots.push(*r);
                    }
                }
                let focus = self
                    .visit
                    .files_menu_mut()
                    .focus
                    .filter(|r| dots.contains(r));
                match key {
                    KeyIn::Tab | KeyIn::BackTab if !dots.is_empty() => {
                        self.visit.files_menu_mut().focus =
                            menu::tab(&dots, focus, key == KeyIn::BackTab);
                        true
                    }
                    KeyIn::Enter | KeyIn::Char(' ') if focus.is_some() => {
                        if let Some(r) = focus {
                            self.file_menu_open(r, true);
                        }
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }
}

/// A row's ⋮ at (x, y), `h` high: opens its menu. Returns its width.
pub(crate) fn dots(app: &View, ui: &mut Ui, x: f32, y: f32, h: f32, row: Row) -> f32 {
    menu::button(
        ui,
        x,
        y,
        h,
        Action::File(FilePress::Menu(row)),
        app.visit.files_menu().focus == Some(row),
    )
}

/// The open ⋮ menu, as every ⋮ menu is drawn ([`menu::draw`]).
pub(crate) fn menu_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(m) = app.visit.files_menu().menu.as_ref() else {
        return;
    };
    let lines: Vec<Line> = m
        .lines
        .iter()
        .map(|(l, a)| Line::Item(l.clone(), *a))
        .collect();
    menu::draw(
        ui,
        w,
        h,
        &menu::View {
            title: &m.title,
            icon: Icon::File,
            lines: &lines,
            at: m.at,
            keys: m.keys,
            anchor: m.anchor,
            cancel: (EN.file_menu_cancel, Action::File(FilePress::Close)),
        },
    );
}

/// Where a small sheet goes: the panel's width less its gutters on the
/// small panel, else centred and no wider than `max`.
fn place(ui: &Ui, w: f32, max: f32) -> ((f32, f32), f32, f32) {
    if ui.compact {
        ((8.0, w - 16.0), 8.0, 16.0)
    } else {
        let sw = (w - 48.0).min(max);
        (((w - sw) / 2.0, sw), 32.0, 32.0)
    }
}

/// The sheet's title, with its glyph. Returns its height.
fn head(ui: &mut Ui, x: f32, y: f32, iw: f32, title: &str) -> f32 {
    if ui.compact {
        return crate::compact::sheet_head(ui, x, y, iw, Icon::File, Role::OnSurface, title);
    }
    ui.icon(x, y, 30.0, Icon::File, 18.0, Role::OnSurface);
    let t = ui.fit(20.0, W::S, title, iw - 42.0);
    ui.text_mid(x + 42.0, y, 30.0, 20.0, W::S, Role::OnSurface, &t);
    46.0
}

/// The Rename popup: the name before its extension in a field, the
/// extension fixed after it, the line that says why a name cannot be
/// used, and **Rename** and **Cancel**.
pub(crate) fn rename_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(r) = app.visit.files_menu().renaming.as_ref() else {
        return;
    };
    let title = fill(EN.rename_title, &[&r.from]);
    let checked = app.rename_checked();
    let (place, margin, pad) = place(ui, w, 480.0);
    scroll_sheet(ui, place, h, margin, pad, &mut |ui, x, y, iw| {
        let mut cy = y;
        cy += head(ui, x, cy, iw, &title);
        let fh = 40.0;
        let ew = if r.ext.is_empty() {
            0.0
        } else {
            ui.measure(14.0, W::M, &r.ext) + tokens::GAP_SMALL
        };
        let fw = iw - ew;
        ui.fill(x, cy, fw, fh, 8.0, Role::SurfaceContainerHighest);
        ui.stroke(x, cy, fw, fh, 8.0, Role::Primary);
        ui.typed(
            x + 12.0,
            cy,
            fh,
            14.0,
            W::M,
            Role::OnSurface,
            &r.stem,
            fw - 24.0,
            true,
            FIELD,
        );
        ui.hit(x, cy, fw, fh, FIELD);
        if ew > 0.0 {
            ui.text_mid(
                x + fw + tokens::GAP_SMALL,
                cy,
                fh,
                14.0,
                W::M,
                Role::OnSurfaceVariant,
                &r.ext,
            );
        }
        cy += fh + tokens::GAP_SMALL;
        if let Err(Some(why)) = &checked {
            cy += ui.wrap(x, cy, iw, 13.0, W::R, Role::Error, why) + tokens::GAP_SMALL;
        }
        cy += tokens::GAP_SMALL;
        let go = Style::Primary.when(checked.is_ok());
        cy += buttons(
            ui,
            x,
            cy,
            iw,
            &[
                (EN.rename_button, go, Action::File(FilePress::RenameGo)),
                (
                    EN.rename_cancel,
                    Style::Secondary,
                    Action::File(FilePress::Close),
                ),
            ],
        );
        cy - y
    });
}

/// The Details sheet: the name, the kind line, the size in bytes and the
/// SHA-256 in groups of eight; nothing to press but **Close**.
pub(crate) fn details_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(d) = app.visit.files_menu().details.as_ref() else {
        return;
    };
    let sha: Vec<String> = match &d.sha {
        Some(Ok(s)) => sha_groups(s).to_vec(),
        Some(Err(why)) => vec![fill(EN.details_not_read, &[why])],
        None => vec![EN.details_reading.to_string()],
    };
    let size = fill(EN.details_bytes, &[&d.size.to_string()]);
    let rows: [(&str, &str); 3] = [
        (EN.details_name, &d.name),
        (EN.details_kind, &d.kind),
        (EN.details_size, &size),
    ];
    let (place, margin, pad) = place(ui, w, 560.0);
    scroll_sheet(ui, place, h, margin, pad, &mut |ui, x, y, iw| {
        let mut cy = y;
        cy += head(ui, x, cy, iw, EN.file_menu_details);
        for (label, value) in rows {
            ui.text(x, cy, 12.0, W::S, Role::OnSurfaceVariant, label);
            cy += 18.0;
            cy += ui.wrap(x, cy, iw, 14.0, W::M, Role::OnSurface, value) + tokens::GAP;
        }
        ui.text(x, cy, 12.0, W::S, Role::OnSurfaceVariant, EN.details_sha);
        cy += 18.0;
        for line in &sha {
            let shown = ui.fit(14.0, W::M, line, iw);
            ui.text(x, cy, 14.0, W::M, Role::OnSurface, &shown);
            cy += 22.0;
        }
        cy += tokens::GAP;
        cy += buttons(
            ui,
            x,
            cy,
            iw,
            &[(
                EN.details_close,
                Style::Secondary,
                Action::File(FilePress::Close),
            )],
        );
        cy - y
    });
}
