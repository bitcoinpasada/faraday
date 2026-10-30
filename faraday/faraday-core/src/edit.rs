//! The caret and the selection in the field typing goes to, and the
//! mouse's back and forward buttons (`docs/history/NEW-WALLET.md` §13.1, §13.2).
//!
//! An edit inside a field, rather than at its end, is made with the
//! field's own keys: the characters after the edit are taken off with
//! Backspace, the edit is made, and they are typed back. Every field then
//! keeps what it already does with a key — which characters it takes,
//! the errors a key clears, its length — and the dice and coin box takes
//! its rolls out of the entropy and puts them back in, so the box and
//! the key never disagree. A secret's characters are only ever popped
//! and pushed, which wipe what they drop; the characters typed back are
//! held in a [`SecretText`] meanwhile, and nothing is copied out of a
//! field anywhere else.

use faraday_shell_api::Key as KeyIn;

use crate::motion;
use crate::secret_text::SecretText;
use crate::ui::{Edit, FieldAt};
use crate::vault::{self, VaultAction as V};
use crate::{Action, Faraday, Screen, Sheet, keygen};

/// Where the caret and the selection are in the field typing goes to,
/// whether an edit inside a field is under way, whether Shift is held,
/// where the last frame drew each field's characters, and the caret's
/// blink. Which field typing goes to is worked out by the glue on
/// [`Faraday`], which reaches every area that has a field; this holds
/// only what typing itself keeps. Holds no text: the characters stay in
/// the fields, which wipe what they drop.
pub(crate) struct TypingArea {
    /// Where the caret and the selection are in the field typing goes
    /// to (`docs/history/NEW-WALLET.md` §13.2).
    edit: Edit,
    /// An edit inside a field is being typed through the field's own
    /// keys.
    editing: bool,
    /// Shift is held.
    shift: bool,
    /// Where the last frame drew each line of typing's characters.
    fields: Vec<FieldAt>,
    /// The tick the text caret's blink last started over at: a key or a
    /// press, so it shows while typing goes on.
    caret_at: u64,
    /// The last frame drew a caret, and whether it showed.
    caret_drawn: Option<bool>,
}

impl TypingArea {
    pub(crate) fn new() -> Self {
        Self {
            edit: Edit::default(),
            editing: false,
            shift: false,
            fields: Vec::new(),
            caret_at: 0,
            caret_drawn: None,
        }
    }

    /// The caret and the selection now in `target`, the field typing
    /// goes to: the kept ones if they are of that field, else the caret
    /// at the end.
    pub(crate) fn edit_in(&self, target: Option<Action>) -> Edit {
        match target {
            Some(t) if self.edit.field == Some(t) => self.edit,
            t => Edit {
                field: t,
                ..Edit::default()
            },
        }
    }

    /// Sets the caret and the selection.
    pub(crate) fn set_edit(&mut self, e: Edit) {
        self.edit = e;
    }

    /// Puts the caret at the end of the kept field's text.
    pub(crate) fn caret_to_end(&mut self) {
        self.edit.caret = None;
    }

    /// Whether an edit inside a field is under way.
    pub(crate) fn editing(&self) -> bool {
        self.editing
    }

    /// Sets whether an edit inside a field is under way.
    pub(crate) fn set_editing(&mut self, on: bool) {
        self.editing = on;
    }

    /// Whether Shift is held.
    pub(crate) fn shift(&self) -> bool {
        self.shift
    }

    /// Sets whether Shift is held.
    pub(crate) fn set_shift(&mut self, held: bool) {
        self.shift = held;
    }

    /// Keeps where the frame just drawn put each field's characters.
    pub(crate) fn set_fields(&mut self, fields: Vec<FieldAt>) {
        self.fields = fields;
    }

    /// Whether the last frame drew a line of typing pressed with
    /// `action`: a field.
    pub(crate) fn is_field(&self, action: Action) -> bool {
        self.fields.iter().any(|f| f.action == action)
    }

    /// The lines the last frame drew of the field pressed with `action`,
    /// first to last: one for a field of one line.
    fn lines(&self, action: Action) -> &[FieldAt] {
        let Some(end) = self.fields.iter().rposition(|f| f.action == action) else {
            return &[];
        };
        let start = self.fields[..end]
            .iter()
            .rposition(|f| f.action != action || f.band.is_none())
            .map_or(0, |i| i + 1);
        // A field of one line is its last drawing alone.
        let start = if self.fields[end].band.is_none() {
            end
        } else {
            start
        };
        &self.fields[start..=end]
    }

    /// Which of `lines` holds character `i`: the one it falls in, or the
    /// one it ends where the next starts later, as the caret is drawn.
    fn line_of(lines: &[FieldAt], i: usize) -> Option<usize> {
        let ends = |f: &FieldAt| f.first + f.edges.len().saturating_sub(1);
        lines
            .iter()
            .rposition(|f| f.first <= i && (i < ends(f) || f.first == i))
            .or_else(|| lines.iter().rposition(|f| f.first <= i))
    }

    /// The last drawn edge before character `i` of the field pressed
    /// with `action`, in pixels, with its line's middle row on a field of
    /// several lines.
    fn edge(&self, action: Action, i: usize) -> Option<(i32, Option<i32>)> {
        let lines = self.lines(action);
        let f = &lines[Self::line_of(lines, i)?];
        let x = f.edges.get(i.checked_sub(f.first)?).copied()?;
        Some((x, f.band.map(|(t, b)| (t + b) / 2)))
    }

    /// The character index nearest pixel (x, y) in the field pressed
    /// with `action`, as the last frame drew it: on the line under `y`,
    /// or the nearest line.
    fn index_at(&self, action: Action, (x, y): (i32, i32)) -> Option<usize> {
        let lines = self.lines(action);
        // The line under `y`, and of a row of cells the one under `x`.
        let line = lines.iter().min_by_key(|f| {
            let rows = match f.band {
                Some((top, _)) if y < top => top - y,
                Some((_, bottom)) if y >= bottom => y - bottom + 1,
                _ => 0,
            };
            let (l, r) = (
                f.edges.first().copied().unwrap_or(x),
                f.edges.last().copied().unwrap_or(x),
            );
            (rows, (l - x).max(x - r).max(0))
        })?;
        Some(line.index_at(x))
    }

    /// The character index on the line above (`up`) or below the one
    /// character `i` is on, nearest its column, in the field pressed with
    /// `action`: `None` on the first or last line, or a field of one.
    fn line_step(&self, action: Action, i: usize, up: bool) -> Option<usize> {
        let lines = self.lines(action);
        let at = Self::line_of(lines, i)?;
        let f = &lines[at];
        let x = *f.edges.get(i - f.first).or(f.edges.last())?;
        let to = if up { at.checked_sub(1)? } else { at + 1 };
        Some(lines.get(to)?.index_at(x))
    }

    /// Restarts the caret's blink at `now`: a key or a press.
    pub(crate) fn blink_from(&mut self, now: u64) {
        self.caret_at = now;
    }

    /// Whether the caret shows at `now`: it blinks, on for a half
    /// period and off for one, starting on at the last key or press.
    pub(crate) fn caret_on(&self, now: u64, steady: bool) -> bool {
        steady || (now.saturating_sub(self.caret_at) / motion::CARET_HALF_MS).is_multiple_of(2)
    }

    /// Keeps whether the frame just drawn showed a caret.
    pub(crate) fn set_caret_drawn(&mut self, shown: Option<bool>) {
        self.caret_drawn = shown;
    }

    /// Whether the last frame's caret differs from what it is now.
    pub(crate) fn caret_changed(&self, now: bool) -> bool {
        self.caret_drawn.is_some_and(|on| on != now)
    }
}

/// A GPG key's paperkey line being typed (`docs/GPG-PAPERKEY.md` §4.5),
/// by the press that focuses it.
pub(crate) const GPG_LINE: Action = Action::Flow(crate::backup_flow::steps::FlowAction::Restore(
    crate::backup_flow::steps::RestorePress::Field,
));

/// A field that takes typing as soon as it is shown, with no press to
/// focus it: a press on it only places the caret
/// (`docs/TEXT-CURSOR.md` §4, C2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Here {
    /// Tools' Find a tool.
    Find,
    /// A word list's find.
    WordFind,
    /// Explore's path, after its `m/`.
    Path,
    /// Explore's Find an address.
    Query,
    /// BIP-85's index.
    Index,
    /// Vanity's prefix, after the address kind's fixed characters.
    Prefix,
    /// A chart sheet's field: a key's holder, a vault's name, or a
    /// passphrase to check, shown as dots.
    Chart,
    /// Sign a message's text, several lines; a press on it while it is
    /// not typed in starts typing, as **Type** does.
    Message,
    /// Add a key's share, string or part, on a form other than words; a
    /// press on it while the passphrase takes typing moves typing here.
    Entry,
    /// What Tools' calculator reads.
    Tools,
    /// Add a key's words typed in Latin letters, a cell a word.
    Words,
    /// Check my copy's words typed back, a cell a word.
    CheckWords,
    /// The aezeed's words; a press on them while its passphrase takes
    /// typing moves typing here.
    Aezeed,
}

impl Faraday {
    /// The field of [`Here`] that typing goes to now, or the potential
    /// wallet's or the aezeed's passphrase while typing goes there. Only
    /// lookups.
    fn here(&self) -> Option<Action> {
        use crate::explore::Page;
        if self.sheet == Some(Sheet::WordList) {
            return self
                .wordlist
                .state()
                .is_some()
                .then_some(Action::TypeHere(Here::WordFind));
        }
        if self.chart_typing() {
            return Some(Action::TypeHere(Here::Chart));
        }
        if self.sheet.is_none() && self.gpg_restore_typing() {
            return Some(GPG_LINE);
        }
        if self.sheet == Some(Sheet::Potential) {
            return self
                .inbox
                .potential()
                .is_some_and(|p| p.typing)
                .then_some(Action::PotentialType);
        }
        if self.sheet.is_some() {
            return None;
        }
        let here = match self.screen {
            Screen::Catalog => Here::Find,
            Screen::Explore => match self.explore.state()?.page {
                Page::Path => Here::Path,
                Page::Find => Here::Query,
                _ => return None,
            },
            Screen::Bip85 if self.bip85.state()?.open == Some(crate::bip85::pstep::INDEX) => {
                Here::Index
            }
            Screen::Vanity if self.vanity.typing_prefix() => Here::Prefix,
            Screen::Lightning => {
                let l = self.lightning.state()?;
                if l.source != Some(crate::lightning::NodeSource::Aezeed) {
                    return None;
                }
                if l.on_passphrase {
                    return Some(Action::LPassphrase);
                }
                Here::Aezeed
            }
            Screen::Tools if self.tools.state().is_some() => Here::Tools,
            Screen::Backup => {
                let c = self.backup.state()?.check.as_ref()?;
                if c.passphrase || c.keys.is_some() {
                    return None;
                }
                Here::CheckWords
            }
            _ => return None,
        };
        Some(Action::TypeHere(here))
    }

    /// A press on a field of [`Here`]: on a message not being typed it
    /// starts typing, as **Type** does; on Add a key's string or the
    /// aezeed's words while their passphrase takes typing, it moves typing
    /// to them; on any other the press only places the caret.
    pub(crate) fn type_here(&mut self, h: Here) {
        match h {
            Here::Message if self.message.state().is_some_and(|m| !m.typing) => {
                self.act(Action::MType);
            }
            Here::Entry if self.entry.state().on_passphrase => self.act(Action::EntryPassphrase),
            Here::Aezeed if self.lightning.state().is_some_and(|l| l.on_passphrase) => {
                self.act(Action::LPassphrase);
            }
            _ => {}
        }
    }

    /// A key for a field whose own keys are handled before the caret's
    /// (`docs/TEXT-CURSOR.md` §4, C2): whether the caret took it.
    pub(crate) fn here_key(&mut self, key: KeyIn) -> bool {
        self.here_first() && self.edit_key(key)
    }

    /// On a small panel the page followed Add a key's string above the
    /// keyboard: codex32's correction, opened in its place now (and not
    /// `before`), is shown from the page's top.
    pub(crate) fn correction_from_top(&mut self, before: bool) {
        if self.compact && !before && self.entry.state().correction.is_open() {
            if let Some(slot) = self.page_slot() {
                *slot = 0.0;
            }
            self.sheet_offset = 0.0;
        }
    }

    /// The paperkey line typed on screen, while step 4 takes typing.
    fn gpg_restore_live(&self) -> Option<&crate::backup_flow::gpg_restore::RestoreLive> {
        self.flows.on().and_then(|i| self.flows.restoring(i))
    }

    /// Holds the paperkey line on screen from ending by itself while an
    /// edit inside it is typed, and lets it end once done.
    fn gpg_restore_hold(&mut self, on: bool) {
        if let Some(i) = self.flows.on() {
            self.flows.restore_hold(i, on);
        }
    }

    /// Whether the field typing goes to has keys of its own that are
    /// handled before the caret's: a word list's find, a chart sheet's
    /// field, the potential wallet's passphrase, Tools' Find a tool.
    pub(crate) fn here_first(&self) -> bool {
        matches!(
            self.here(),
            Some(
                Action::TypeHere(Here::WordFind | Here::Find | Here::Chart)
                    | Action::PotentialType
                    | GPG_LINE
            )
        )
    }

    /// Whether step 4 of a GPG key's restore on screen takes typing: its
    /// paperkey line (`docs/GPG-PAPERKEY.md` §4.5), on the hex keyboard.
    pub(crate) fn gpg_restore_typing(&self) -> bool {
        self.screen == Screen::BackupFlow
            && self
                .flows
                .on()
                .is_some_and(|i| self.flows.restore_typing(i))
    }

    /// Whether typing goes to a text field now.
    pub(crate) fn typing_field(&self) -> bool {
        if self.chart_typing() {
            return true;
        }
        if self.sheet == Some(Sheet::Import) {
            return self.vaults.focus == Some(vault::Focus::Passphrase);
        }
        if self.sheet == Some(Sheet::SeedRead) {
            return self.seed_read.state().is_some_and(|s| s.focus.is_some());
        }
        if self.sheet == Some(Sheet::Sweep) {
            return self.sweep_typing();
        }
        if self.sheet == Some(Sheet::PassphraseSeeds) {
            return self.vaults.focus == Some(vault::Focus::Prompt);
        }
        if self.sheet == Some(Sheet::Rename) {
            return self.visit.files_menu().renaming.is_some();
        }
        if self.sheet == Some(Sheet::Register) {
            return self.register_typing();
        }
        match self.screen {
            Screen::Family => self.vaults.focus.is_some() || self.seeds_typing(),
            Screen::Restore => self.seeds_typing(),
            Screen::Unlock
            | Screen::CreateVault
            | Screen::VaultContents
            | Screen::Codes
            | Screen::BackupFlow => {
                self.vaults.focus.is_some()
                    || (self.screen == Screen::BackupFlow
                        && (self.backup.state().is_some_and(|b| b.naming.is_some())
                            || self.gpg_restore_typing()))
            }
            Screen::Entry => true,
            Screen::SetTime => self.set_time.focus().is_some(),
            // A passphrase on paper typed back for Check my copy.
            Screen::Backup => self
                .backup
                .state()
                .and_then(|b| b.check.as_ref())
                .is_some_and(|c| c.passphrase),
            Screen::Wallets => self.wallets.renaming().is_some(),
            Screen::Message => self.message.state().is_some_and(|m| m.typing),
            Screen::KeyGen => self.keygen.state().is_some_and(|k| {
                !k.locked
                    && ((k.focus.is_some() && k.open == Some(keygen::kstep::KEY))
                        || (k.open == Some(keygen::kstep::ENTER) && k.typing && k.flip_or_roll())
                        || k.open == Some(keygen::kstep::TYPED))
            }),
            Screen::Create => self
                .create
                .state()
                .is_some_and(|c| c.pass_slot.is_some() && c.pass_focus.is_some()),
            _ => false,
        }
    }

    /// The field typing goes to, by the action that focuses it: one
    /// that takes a caret, a selection and an edit inside it.
    pub(crate) fn typing_target(&self) -> Option<Action> {
        use vault::Focus as F;
        // A place's name on Back up's plan or a flow's plan card
        // (`docs/BACKUP-FLOW.md` §17.18).
        if (self.screen == Screen::Backup
            || (self.screen == Screen::BackupFlow && self.vaults.focus.is_none()))
            && let Some(p) = self.backup.state().and_then(|b| b.naming)
        {
            return Some(Action::BName(u8::try_from(p).unwrap_or(u8::MAX)));
        }
        if let Some(a) = self.here() {
            return Some(a);
        }
        if !self.typing_field() {
            return None;
        }
        if self.sheet == Some(Sheet::Import) {
            return Some(Action::Vault(V::FocusPassphrase));
        }
        if self.sheet == Some(Sheet::Rename) {
            return Some(crate::file_menu::FIELD);
        }
        if self.sheet == Some(Sheet::Register) {
            return Some(Action::RegisterField);
        }
        if self.sheet == Some(Sheet::Sweep) {
            return Some(Action::Sweep(crate::seeds::sweep::SweepAction::Field));
        }
        if self.sheet == Some(Sheet::SeedRead) {
            return self
                .seed_read
                .state()?
                .focus
                .map(|f| Action::SeedRead(crate::seeds::read::Choice::Field(f)));
        }
        match self.screen {
            Screen::Family | Screen::Restore if self.seeds_typing() => self
                .seeds()?
                .focus
                .map(|f| Action::Seeds(crate::seeds::SeedsAction::Focus(f))),
            Screen::Unlock
            | Screen::CreateVault
            | Screen::VaultContents
            | Screen::BackupFlow
            | Screen::Family
            | Screen::Codes => Some(Action::Vault(match self.vaults.focus? {
                F::Passphrase => V::FocusPassphrase,
                F::Phrase(i, second) => V::CFocus(i, second),
                F::Field(k) => V::FocusField(k),
                F::Dice => V::Dice(self.vaults.dice.as_ref().map_or(0, |d| d.0)),
                F::Rolls => V::Roller(crate::fresh::roller::Act::Count),
                F::Prompt => V::FocusPrompt,
                F::Name => V::CName,
                F::Search(s) => V::Search(vault::SearchAct::Focus(s)),
            })),
            Screen::Entry => self.entry_target(),
            Screen::Wallets => Some(Action::Rename),
            Screen::Message => Some(Action::TypeHere(Here::Message)),
            Screen::KeyGen => {
                let k = self.keygen.state()?;
                match k.open {
                    Some(keygen::kstep::KEY) => k.focus.map(Action::KPassField),
                    // A key typed: Add a key's form, as on Add a key.
                    Some(keygen::kstep::TYPED) => self.entry_target(),
                    _ => Some(Action::KTyping(true)),
                }
            }
            Screen::Create => self.create.state()?.pass_focus.map(Action::CPassField),
            Screen::SetTime => self.set_time.focus().map(Action::SetTimeFocus),
            _ => None,
        }
    }

    /// The field of Add a key's form typing goes to: its passphrase, its
    /// words typed in Latin letters, or its string (not while codex32's
    /// correction or a clash stands in place of it).
    fn entry_target(&self) -> Option<Action> {
        let e = self.entry.state();
        if e.on_passphrase {
            return Some(Action::EntryPassphrase);
        }
        if e.form == crate::forms::Form::Words {
            return e.keys.is_none().then_some(Action::TypeHere(Here::Words));
        }
        (!e.correction.is_open() && e.codex32.clash.is_none())
            .then_some(Action::TypeHere(Here::Entry))
    }

    /// What the field typing goes to holds, as a copy wiped when it is
    /// dropped.
    fn typing_text(&self) -> Option<SecretText> {
        use crate::seeds::{Focus as SF, SeedsAction as S};
        use vault::Focus as F;
        let text: &str = match self.typing_target()? {
            Action::Seeds(S::Focus(f)) => {
                let s = self.seeds()?;
                match f {
                    SF::Cosigner(k) => s.cosigners.get(usize::from(k))?,
                    SF::Path => s.custom.as_deref()?,
                }
            }
            Action::Vault(_) => {
                let v = &self.vaults;
                match self.vaults.focus? {
                    F::Passphrase => &v.passphrase.text,
                    F::Phrase(i, second) => {
                        let (a, b) = v.create.as_ref()?.phrases.get(i)?;
                        if second { &b.text } else { &a.text }
                    }
                    F::Field(k) => &v.form.as_ref()?.fields.get(k)?.2.text,
                    F::Dice => &v.dice.as_ref()?.1.text,
                    F::Rolls => v.roller.typed(),
                    F::Prompt => &v.prompt.as_ref()?.text.text,
                    F::Name => &v.create.as_ref()?.name.text,
                    F::Search(s) => v.query(s),
                }
            }
            Action::EntryPassphrase => &self.entry.state().passphrase,
            Action::SeedRead(_) => self.seed_read.state()?.field()?,
            Action::Rename => self.wallets.renaming().map(String::as_str)?,
            crate::file_menu::FIELD => &self.visit.files_menu().renaming.as_ref()?.stem,
            Action::RegisterField => &self.wallets.spend()?.register.as_ref()?.name,
            Action::BName(p) => self.backup.state()?.names.get(usize::from(p))?,
            Action::KPassField(0) => &self.keygen.state()?.passphrase,
            Action::KPassField(_) => &self.keygen.state()?.passphrase2,
            Action::KTyping(_) => &self.keygen.state()?.entered,
            Action::CPassField(0) => &self.create.state()?.pass,
            Action::CPassField(_) => &self.create.state()?.pass2,
            Action::TypeHere(h) => match h {
                Here::Find => self.tools.find(),
                Here::WordFind => &self.wordlist.state()?.find,
                Here::Path => &self.explore.state()?.path,
                Here::Query => &self.explore.state()?.query,
                Here::Index => &self.bip85.state()?.index,
                Here::Message => &self.message.state()?.text,
                Here::Entry => &self.entry.state().typed,
                Here::Tools => &self.tools.state()?.typed,
                Here::Words => &self.entry.state().typed,
                Here::CheckWords => &self.backup.state()?.check.as_ref()?.typed,
                Here::Aezeed => &self.lightning.state()?.typed,
                Here::Chart => {
                    let w = self.glance.work();
                    match self.glance.chart()?.1 {
                        crate::glance::sheet::Target::PassCheck(..) => &w.secret,
                        _ => &w.text,
                    }
                }
                Here::Prefix => {
                    let v = self.vanity.state()?;
                    let net = self.session.network();
                    let text = v.grind.prefix.as_str();
                    &text[v
                        .grind
                        .prefix
                        .fixed_len(v.grind.script, net)
                        .min(text.len())..]
                }
            },
            Action::PotentialType => &self.inbox.potential()?.passphrase,
            GPG_LINE => self.gpg_restore_live()?.typed(),
            Action::LPassphrase => &self.lightning.state()?.passphrase,
            Action::SetTimeFocus(crate::set_time::Focus::Date) => &self.set_time.date().text,
            Action::SetTimeFocus(crate::set_time::Focus::Time) => &self.set_time.time().text,
            _ => return None,
        };
        Some(SecretText::of(text))
    }

    /// Whether the field typing goes to is one of several lines, where
    /// Up and Down move the caret a line (`docs/TEXT-CURSOR.md` §3).
    fn several_lines(&self, target: Action) -> bool {
        match target {
            Action::TypeHere(Here::Message) | GPG_LINE => true,
            // Wrapped on a small panel, one line on a wider window.
            Action::TypeHere(Here::Entry) => self.compact,
            Action::Vault(V::FocusField(k)) => self.vaults.form.as_ref().is_some_and(|f| {
                f.fields
                    .get(k)
                    .is_some_and(|field| vault::multiline(f.kind, field.0))
            }),
            _ => false,
        }
    }

    /// The caret and the selection now: those of the field typing goes
    /// to, or the caret at the end of any other.
    pub(crate) fn edit_now(&self) -> Edit {
        self.typing.edit_in(self.typing_target())
    }

    /// Where the last frame drew the edge before character `i` of the
    /// field pressed with `action`, in pixels, at the middle of the
    /// field's height: for tests that press or drag there.
    pub fn where_typed(&self, action: Action, i: usize) -> Option<(u16, u16)> {
        let (x, row) = self.typing.edge(action, i)?;
        let y = match row {
            Some(y) => u16::try_from(y).ok()?,
            None => self.where_offered(action)?.1,
        };
        Some((u16::try_from(x).ok()?, y))
    }

    /// Where the press now down fell among what the last frame drew for
    /// `action`: the nearest of its edges, as an index (a lookup).
    pub(crate) fn pressed_index(&self, action: Action) -> Option<usize> {
        self.typing.index_at(action, self.pointer.down_at())
    }

    /// Whether all of the field typing goes to is selected.
    pub fn selected_all(&self) -> bool {
        self.typing_text()
            .is_some_and(|t| self.edit_now().all(t.chars().count()))
    }

    /// A key for the field typing goes to that moves the caret or
    /// changes the text inside it, rather than at its end. Returns
    /// whether it took the key; one it did not take goes to the field
    /// as it always has, and is typed at the end.
    pub(crate) fn edit_key(&mut self, key: KeyIn) -> bool {
        if self.typing.editing() {
            return false;
        }
        let Some(target) = self.typing_target() else {
            return false;
        };
        let Some(text) = self.typing_text() else {
            return false;
        };
        let n = text.chars().count();
        let mut e = self.edit_now();
        let caret = e.caret_in(n);
        let selected = e.selected(n);
        match key {
            KeyIn::Up | KeyIn::Down if self.several_lines(target) => {
                let Some(to) = self.typing.line_step(target, caret, key == KeyIn::Up) else {
                    // On the first line or the last: the key goes on, to
                    // scroll as it would.
                    return false;
                };
                e.anchor = if self.typing.shift() {
                    e.anchor.or(Some(caret)).filter(|&a| a != to)
                } else {
                    None
                };
                e.caret = (to < n).then_some(to);
                self.typing.set_edit(e);
                true
            }
            KeyIn::Left | KeyIn::Right => {
                let left = key == KeyIn::Left;
                let to = match selected {
                    Some((lo, hi)) if !self.typing.shift() => {
                        if left {
                            lo
                        } else {
                            hi
                        }
                    }
                    _ if left => caret.saturating_sub(1),
                    _ => (caret + 1).min(n),
                };
                e.anchor = if self.typing.shift() {
                    e.anchor.or(Some(caret))
                } else {
                    None
                };
                e.caret = (to < n).then_some(to);
                // A selection with both ends at one place is none.
                if e.anchor == Some(to) {
                    e.anchor = None;
                }
                self.typing.set_edit(e);
                true
            }
            // Home and End let a selection go and put the caret at the
            // field's start or end.
            KeyIn::Home | KeyIn::End => {
                let to = if key == KeyIn::Home { 0 } else { n };
                e.anchor = if self.typing.shift() {
                    e.anchor.or(Some(caret)).filter(|&a| a != to)
                } else {
                    None
                };
                e.caret = (to < n).then_some(to);
                self.typing.set_edit(e);
                true
            }
            KeyIn::Backspace | KeyIn::Delete | KeyIn::Char(_) => {
                let insert = match key {
                    KeyIn::Char(c) if !c.is_control() => Some(c),
                    KeyIn::Char(_) => return false,
                    _ => None,
                };
                let (lo, hi) = match (selected, key) {
                    (Some(r), _) => r,
                    (None, KeyIn::Backspace) if caret == n => {
                        self.typing.caret_to_end();
                        return false;
                    }
                    (None, KeyIn::Backspace) if caret == 0 => return true,
                    (None, KeyIn::Backspace) => (caret - 1, caret),
                    (None, KeyIn::Delete) if caret == n => return true,
                    (None, KeyIn::Delete) => (caret, caret + 1),
                    (None, _) if caret == n => {
                        // Typed at the end, as it always is.
                        self.typing.set_edit(Edit {
                            field: Some(target),
                            ..Edit::default()
                        });
                        return false;
                    }
                    (None, _) => (caret, caret),
                };
                self.splice(target, &text, lo, hi, insert);
                true
            }
            _ => false,
        }
    }

    /// Replaces characters `lo` to `hi` of the field typing goes to,
    /// which holds `text`, with `insert`, through the field's own keys.
    /// The caret is left after what was typed.
    fn splice(&mut self, target: Action, text: &str, lo: usize, hi: usize, insert: Option<char>) {
        let n = text.chars().count();
        let byte = |i: usize| text.char_indices().nth(i).map_or(text.len(), |(b, _)| b);
        if let Action::TypeHere(h @ (Here::Words | Here::CheckWords)) = target {
            return self.words_splice(h, (byte(lo), byte(hi)), lo, insert);
        }
        let tail = SecretText::of(&text[byte(hi)..]);
        let len = |app: &Faraday| app.typing_text().map(|t| t.chars().count());
        self.typing.set_editing(true);
        // A paperkey line ends by itself only once the edit is done.
        let gpg = target == GPG_LINE;
        if gpg {
            self.gpg_restore_hold(true);
        }
        // What is after `lo` comes off, one character at a time.
        let mut have = n;
        while have > lo {
            self.key(KeyIn::Backspace);
            match len(self) {
                Some(l) if l + 1 == have && self.typing_target() == Some(target) => have = l,
                _ => break,
            }
        }
        if have > lo {
            // The field would not give it up (a stick is in, say): what
            // came off goes back, and nothing is changed.
            for c in text[byte(have)..byte(n)].chars() {
                self.type_back(c);
            }
            self.typing.set_editing(false);
            if gpg {
                self.gpg_restore_hold(false);
            }
            return;
        }
        if let Some(c) = insert {
            self.key(KeyIn::Char(c));
        }
        let caret = len(self).unwrap_or(lo);
        for c in tail.chars() {
            self.type_back(c);
        }
        self.typing.set_editing(false);
        if gpg {
            self.gpg_restore_hold(false);
        }
        let end = len(self).unwrap_or(caret);
        self.typing.set_edit(Edit {
            field: self.typing_target(),
            caret: (caret < end).then_some(caret),
            anchor: None,
        });
    }

    /// An edit inside a grid of words typed in Latin letters: bytes `lo`
    /// to `hi` replaced with `insert`, a letter (lower case) or a space,
    /// made in place in the words held, with no word completed part way
    /// through as typing at the end completes one. What follows is held
    /// in a [`SecretText`] meanwhile and the words keep theirs, both wiped.
    /// The caret is left after what was typed, character `at`.
    fn words_splice(&mut self, h: Here, (lo, hi): (usize, usize), at: usize, insert: Option<char>) {
        let insert = match insert {
            Some(c) if c.is_ascii_alphabetic() => Some(c.to_ascii_lowercase()),
            Some(' ') => Some(' '),
            Some(_) => return,
            None => None,
        };
        let done = match h {
            Here::Words => {
                let e = self.entry.state_mut();
                splice_text(&mut e.typed, (lo, hi), insert);
                e.error = None;
                true
            }
            _ => self
                .backup
                .check_edit(|c| {
                    splice_text(&mut c.typed, (lo, hi), insert);
                    c.error = None;
                })
                .is_some(),
        };
        if done {
            let at = at + usize::from(insert.is_some());
            let n = self.typing_text().map_or(at, |t| t.chars().count());
            self.typing.set_edit(Edit {
                field: Some(Action::TypeHere(h)),
                caret: (at < n).then_some(at),
                anchor: None,
            });
        }
    }

    /// A character typed back after an edit: a line's end as Enter.
    fn type_back(&mut self, c: char) {
        self.key(if c == '\n' {
            KeyIn::Enter
        } else {
            KeyIn::Char(c)
        });
    }

    /// A press on the field `a` at pixel (x, y), on its release (but for
    /// the first on a place's name, which opens all of it selected,
    /// `docs/BACKUP-FLOW.md` §19.18): the
    /// caret goes there; a second press soon after selects all of the
    /// field, or nothing if all was; a drag from `from` across an edge
    /// between characters selects what it went over; with Shift held
    /// the selection runs from where it was to there. Returns whether
    /// the press was taken, and whether it was a drag.
    pub(crate) fn field_press(
        &mut self,
        a: Action,
        from: (i32, i32),
        x: (i32, i32),
        double: bool,
    ) -> Option<bool> {
        let index = |app: &Faraday, at: (i32, i32)| app.typing.index_at(a, at);
        let dx = (x.0 - from.0).abs();
        let moved = dx + (x.1 - from.1).abs();
        let dragged = dx > 12 || (moved > 3 && index(self, x) != index(self, from));
        if self.typing_target() != Some(a) {
            if double || dragged || self.typing.shift() {
                // A drag or a second press on a field not yet typed in
                // opens it first.
                self.act(a);
            } else {
                return None;
            }
        }
        if self.typing_target() != Some(a) {
            return None;
        }
        let at = index;
        let n = self.typing_text().map_or(0, |t| t.chars().count());
        let mut e = self.edit_now();
        let to = at(self, x).map_or(n, |i| i.min(n));
        if double && !dragged {
            e = if e.all(n) {
                Edit {
                    field: Some(a),
                    caret: (to < n).then_some(to),
                    anchor: None,
                }
            } else {
                Edit {
                    field: Some(a),
                    caret: None,
                    anchor: Some(0),
                }
            };
        } else {
            let anchor = if dragged {
                Some(at(self, from).map_or(0, |i| i.min(n)))
            } else if self.typing.shift() {
                Some(e.anchor.unwrap_or(e.caret_in(n)))
            } else {
                None
            };
            e = Edit {
                field: Some(a),
                caret: (to < n).then_some(to),
                anchor: anchor.filter(|&p| p != to),
            };
        }
        self.typing.set_edit(e);
        Some(dragged)
    }

    /// Whether screen `s` still has what it shows: a flow left keeps its
    /// state only while it is not ended. Only lookups.
    pub(crate) fn can_show(&self, s: Screen) -> bool {
        match s {
            Screen::KeyGen => self.keygen.state().is_some(),
            Screen::Create => self.create.state().is_some(),
            Screen::Backup => self.backup.state().is_some(),
            Screen::Spend => self.wallets.spend().is_some(),
            Screen::Restore => self.restore.state().is_some(),
            Screen::Message => self.message.state().is_some(),
            Screen::Vanity => self.vanity.state().is_some(),
            Screen::Bip85 => self.bip85.state().is_some(),
            Screen::Silent => self.silent.state().is_some(),
            Screen::Explore => self.explore.state().is_some(),
            Screen::Lightning => self.lightning.state().is_some(),
            Screen::Tools => self.tools.state().is_some(),
            Screen::CreateVault => self.vaults.create.is_some(),
            Screen::BackupFlow => self.flow().is_some(),
            Screen::Decode => self.decode().is_some(),
            Screen::NonceCheck => self.outside_check().is_some(),
            Screen::CheckMessage => self.message.checking().is_some(),
            Screen::Visit => !self.stick.is_empty(),
            Screen::Transfer => self.online,
            Screen::Randomness => self.own_page().is_some(),
            _ => true,
        }
    }
}

/// Bytes `lo` to `hi` of `t` replaced with `insert`: what follows is held
/// in a [`SecretText`] while `t` is cut, which wipes what it drops, and
/// put back.
pub(crate) fn splice_text(t: &mut SecretText, (lo, hi): (usize, usize), insert: Option<char>) {
    let tail = SecretText::of(&t[hi.min(t.len())..]);
    t.truncate(lo);
    if let Some(c) = insert {
        t.push(c);
    }
    t.push_str(&tail);
}
