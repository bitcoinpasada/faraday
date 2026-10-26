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

    /// The last drawn edge before character `i` of the field pressed
    /// with `action`, in pixels.
    fn edge(&self, action: Action, i: usize) -> Option<i32> {
        let f = self.fields.iter().rev().find(|f| f.action == action)?;
        f.edges.get(i.checked_sub(f.first)?).copied()
    }

    /// The character index nearest pixel column `x` in the field pressed
    /// with `action`, as the last frame drew it.
    fn index_at(&self, action: Action, x: i32) -> Option<usize> {
        self.fields
            .iter()
            .rev()
            .find(|f| f.action == action)
            .map(|f| f.index_at(x))
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

impl Faraday {
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
        if self.sheet == Some(Sheet::PassphraseSeeds) {
            return self.vaults.focus == Some(vault::Focus::Prompt);
        }
        if self.sheet == Some(Sheet::Rename) {
            return self.visit.files_menu().renaming.is_some();
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
                        && self.backup.state().is_some_and(|b| b.naming.is_some()))
            }
            Screen::Entry => true,
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
        if !self.typing_field() {
            return None;
        }
        if self.sheet == Some(Sheet::Import) {
            return Some(Action::Vault(V::FocusPassphrase));
        }
        if self.sheet == Some(Sheet::Rename) {
            return Some(crate::file_menu::FIELD);
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
            Screen::Entry if self.entry.state().on_passphrase => Some(Action::EntryPassphrase),
            Screen::Wallets => Some(Action::Rename),
            Screen::Message => Some(Action::MType),
            Screen::KeyGen => {
                let k = self.keygen.state()?;
                match k.open {
                    Some(keygen::kstep::KEY) => k.focus.map(Action::KPassField),
                    // A key typed: Add a key's form, which has no caret of
                    // its own to move.
                    Some(keygen::kstep::TYPED) => None,
                    _ => Some(Action::KTyping(true)),
                }
            }
            Screen::Create => self.create.state()?.pass_focus.map(Action::CPassField),
            _ => None,
        }
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
            Action::BName(p) => self.backup.state()?.names.get(usize::from(p))?,
            Action::MType => &self.message.state()?.text,
            Action::KPassField(0) => &self.keygen.state()?.passphrase,
            Action::KPassField(_) => &self.keygen.state()?.passphrase2,
            Action::KTyping(_) => &self.keygen.state()?.entered,
            Action::CPassField(0) => &self.create.state()?.pass,
            Action::CPassField(_) => &self.create.state()?.pass2,
            _ => return None,
        };
        Some(SecretText::of(text))
    }

    /// Whether the caret moves in the field typing goes to: not in one
    /// of several lines, whose caret stays at its end.
    fn caret_moves(&self, target: Action) -> bool {
        match target {
            Action::MType => false,
            Action::Vault(V::FocusField(k)) => !self.vaults.form.as_ref().is_some_and(|f| {
                f.fields
                    .get(k)
                    .is_some_and(|field| vault::multiline(f.kind, field.0))
            }),
            _ => true,
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
        let x = self.typing.edge(action, i)?;
        let (_, y) = self.where_offered(action)?;
        Some((u16::try_from(x).ok()?, y))
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
            KeyIn::Left | KeyIn::Right if self.caret_moves(target) => {
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
            KeyIn::Home | KeyIn::End if self.caret_moves(target) => {
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
        let tail = SecretText::of(&text[byte(hi)..]);
        let len = |app: &Faraday| app.typing_text().map(|t| t.chars().count());
        self.typing.set_editing(true);
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
        let end = len(self).unwrap_or(caret);
        self.typing.set_edit(Edit {
            field: self.typing_target(),
            caret: (caret < end).then_some(caret),
            anchor: None,
        });
    }

    /// A character typed back after an edit: a line's end as Enter.
    fn type_back(&mut self, c: char) {
        self.key(if c == '\n' {
            KeyIn::Enter
        } else {
            KeyIn::Char(c)
        });
    }

    /// A press on the field `a` at pixel column `x`, on its release:
    /// the caret goes there; a second press soon after selects all of
    /// the field, or nothing if all was; a drag from `from` across an
    /// edge between characters selects what it went over; with Shift
    /// held the selection runs from where it was to there. Returns
    /// whether the press was taken, and whether it was a drag.
    pub(crate) fn field_press(
        &mut self,
        a: Action,
        from: i32,
        x: i32,
        double: bool,
    ) -> Option<bool> {
        let index = |app: &Faraday, x: i32| app.typing.index_at(a, x);
        let dx = (x - from).abs();
        let dragged = dx > 12 || (dx > 3 && index(self, x) != index(self, from));
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
            Screen::Upgrade => self.upgrade.state().is_some(),
            Screen::CreateVault => self.vaults.create.is_some(),
            Screen::BackupFlow => self.flow().is_some(),
            Screen::Decode => self.decode().is_some(),
            Screen::CheckMessage => self.message.checking().is_some(),
            Screen::Visit => !self.stick.is_empty(),
            Screen::Transfer => self.online,
            Screen::Randomness => self.own_page().is_some(),
            _ => true,
        }
    }
}
