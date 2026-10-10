//! The caret and the selection in the field typing goes to, and the
//! mouse's back and forward buttons (`docs/NEW-WALLET.md` §13.1, §13.2).
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

use osk_shell_api::Key as KeyIn;

use crate::secret_text::SecretText;
use crate::ui::Edit;
use crate::vaults::{self, VaultAction as V};
use crate::{Action, Faraday, Screen, Sheet, keygen};

/// The most screens kept for the forward button.
const FORWARD_MAX: usize = 8;

impl Faraday {
    /// The field typing goes to, by the action that focuses it: one
    /// that takes a caret, a selection and an edit inside it.
    pub(crate) fn typing_target(&self) -> Option<Action> {
        use vaults::Focus as F;
        if !self.typing_field() {
            return None;
        }
        if self.sheet == Some(Sheet::Import) {
            return Some(Action::Vault(V::FocusPassphrase));
        }
        match self.screen {
            Screen::Family | Screen::Restore if self.seeds_typing() => self
                .seeds()?
                .focus
                .map(|f| Action::Seeds(crate::seeds::SeedsAction::Focus(f))),
            Screen::Unlock | Screen::CreateVault | Screen::VaultContents | Screen::Family => {
                Some(Action::Vault(match self.vaults.focus? {
                    F::Passphrase => V::FocusPassphrase,
                    F::Phrase(i, second) => V::CFocus(i, second),
                    F::Field(k) => V::FocusField(k),
                    F::Dice => V::Dice(self.vaults.dice.as_ref().map_or(0, |d| d.0)),
                    F::Prompt => V::FocusPrompt,
                    F::Name => V::CName,
                }))
            }
            Screen::Entry if self.entry.on_passphrase => Some(Action::EntryPassphrase),
            Screen::Wallets => Some(Action::Rename),
            Screen::Message => Some(Action::MType),
            Screen::KeyGen => {
                let k = self.keygen.as_ref()?;
                if k.open == Some(keygen::kstep::KEY) {
                    k.focus.map(Action::KPassField)
                } else {
                    Some(Action::KTyping(true))
                }
            }
            Screen::Create => self.create.as_ref()?.pass_focus.map(Action::CPassField),
            Screen::Restore => self.restore.as_ref()?.pass_focus.map(Action::RPassField),
            _ => None,
        }
    }

    /// What the field typing goes to holds, as a copy wiped when it is
    /// dropped.
    fn typing_text(&self) -> Option<SecretText> {
        use crate::seeds::{Focus as SF, SeedsAction as S};
        use vaults::Focus as F;
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
                    F::Prompt => &v.prompt.as_ref()?.text.text,
                    F::Name => &v.create.as_ref()?.name.text,
                }
            }
            Action::EntryPassphrase => &self.entry.passphrase,
            Action::Rename => self.renaming.as_deref()?,
            Action::MType => &self.message.as_ref()?.text,
            Action::KPassField(0) => &self.keygen.as_ref()?.passphrase,
            Action::KPassField(_) => &self.keygen.as_ref()?.passphrase2,
            Action::KTyping(_) => &self.keygen.as_ref()?.entered,
            Action::CPassField(0) => &self.create.as_ref()?.pass,
            Action::CPassField(_) => &self.create.as_ref()?.pass2,
            Action::RPassField(0) => &self.restore.as_ref()?.pass,
            Action::RPassField(_) => &self.restore.as_ref()?.pass2,
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
                    .is_some_and(|field| vaults::multiline(f.kind, field.0))
            }),
            _ => true,
        }
    }

    /// The caret and the selection now: those of the field typing goes
    /// to, or the caret at the end of any other.
    pub(crate) fn edit_now(&self) -> Edit {
        match self.typing_target() {
            Some(t) if self.edit.field == Some(t) => self.edit,
            t => Edit {
                field: t,
                ..Edit::default()
            },
        }
    }

    /// Where the last frame drew the edge before character `i` of the
    /// field pressed with `action`, in pixels, at the middle of the
    /// field's height: for tests that press or drag there.
    pub fn where_typed(&self, action: Action, i: usize) -> Option<(u16, u16)> {
        let f = self.fields.iter().rev().find(|f| f.action == action)?;
        let x = *f.edges.get(i.checked_sub(f.first)?)?;
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
        if self.editing {
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
                    Some((lo, hi)) if !self.shift => {
                        if left {
                            lo
                        } else {
                            hi
                        }
                    }
                    _ if left => caret.saturating_sub(1),
                    _ => (caret + 1).min(n),
                };
                e.anchor = if self.shift {
                    e.anchor.or(Some(caret))
                } else {
                    None
                };
                e.caret = (to < n).then_some(to);
                // A selection with both ends at one place is none.
                if e.anchor == Some(to) {
                    e.anchor = None;
                }
                self.edit = e;
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
                        self.edit.caret = None;
                        return false;
                    }
                    (None, KeyIn::Backspace) if caret == 0 => return true,
                    (None, KeyIn::Backspace) => (caret - 1, caret),
                    (None, KeyIn::Delete) if caret == n => return true,
                    (None, KeyIn::Delete) => (caret, caret + 1),
                    (None, _) if caret == n => {
                        // Typed at the end, as it always is.
                        self.edit = Edit {
                            field: Some(target),
                            ..Edit::default()
                        };
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
        self.editing = true;
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
            self.editing = false;
            return;
        }
        if let Some(c) = insert {
            self.key(KeyIn::Char(c));
        }
        let caret = len(self).unwrap_or(lo);
        for c in tail.chars() {
            self.type_back(c);
        }
        self.editing = false;
        let end = len(self).unwrap_or(caret);
        self.edit = Edit {
            field: self.typing_target(),
            caret: (caret < end).then_some(caret),
            anchor: None,
        };
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
        let index = |app: &Faraday, x: i32| {
            app.fields
                .iter()
                .rev()
                .find(|f| f.action == a)
                .map(|f| f.index_at(x))
        };
        let dx = (x - from).abs();
        let dragged = dx > 12 || (dx > 3 && index(self, x) != index(self, from));
        if self.typing_target() != Some(a) {
            if double || dragged || self.shift {
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
            } else if self.shift {
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
        self.edit = e;
        Some(dragged)
    }

    /// The mouse's back button: the screen's back link, or Escape; a
    /// screen it leaves can be opened again with the forward button.
    pub(crate) fn mouse_back(&mut self) {
        let before = self.screen;
        if self.landed != Some(before) {
            self.forward.clear();
        }
        match (self.sheet, self.back_link) {
            // A sheet's own way out, as Escape takes it.
            (Some(Sheet::Lock), _) => {}
            (Some(_), _) => self.act(Action::Cancel),
            (None, Some(a)) => self.act(a),
            (None, None) => self.key(KeyIn::Escape),
        }
        if self.screen != before {
            if self.forward.len() == FORWARD_MAX {
                self.forward.remove(0);
            }
            self.forward.push(before);
            self.landed = Some(self.screen);
        }
    }

    /// The mouse's forward button: the screen the back button last left,
    /// while nothing else has been opened since and it can still be
    /// shown as it was left.
    pub(crate) fn mouse_forward(&mut self) {
        if self.landed != Some(self.screen) || self.sheet.is_some() {
            self.forward.clear();
            return;
        }
        let Some(s) = self.forward.pop() else {
            return;
        };
        if !self.can_show(s) {
            self.forward.clear();
            return;
        }
        self.act(Action::Nav(s));
        self.landed = Some(self.screen);
    }

    /// Whether screen `s` still has what it shows: a flow left keeps its
    /// state only while it is not ended.
    fn can_show(&self, s: Screen) -> bool {
        match s {
            Screen::KeyGen => self.keygen.is_some(),
            Screen::Create => self.create.is_some(),
            Screen::Backup => self.backup.is_some(),
            Screen::Spend => self.spend.is_some(),
            Screen::Restore => self.restore.is_some(),
            Screen::Message => self.message.is_some(),
            Screen::Vanity => self.vanity.is_some(),
            Screen::Bip85 => self.bip85.is_some(),
            Screen::Silent => self.silent.is_some(),
            Screen::Explore => self.explore.is_some(),
            Screen::Lightning => self.lightning.is_some(),
            Screen::Tools => self.tools.is_some(),
            Screen::Upgrade => self.upgrade.is_some(),
            Screen::CreateVault => self.vaults.create.is_some(),
            _ => true,
        }
    }
}
