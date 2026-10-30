//! What to write's Name field (`docs/STICK-IDENTITY.md` §6): an optional
//! name for the data partition of a stick or card written whole, empty at
//! first. It is a FAT label: at most 11 of FAT's characters, kept in
//! capitals ([`faraday_fat::label_bytes`]); a name typed in lower case is
//! written in capitals and the field says so as it is typed, and a boot
//! partition's name is refused (`faraday_boot::layout`). Keep data does
//! not touch the data partition, so the field is not offered there.
//!
//! The caret is `docs/TEXT-CURSOR.md`'s: Left and Right move it, Home and
//! End to the ends, Backspace and Delete take the character before and
//! after it, a character typed goes in at it, and a press puts it between
//! the two characters nearest the pointer, from where the last frame drew
//! them.

use faraday_shell_api::Key;
use faraday_strings::EN;
use faraday_ui::{Edit, FieldAt};

use crate::api::Mode;
use crate::{Page, Starter, StarterAction};

/// The most characters the field holds: past FAT's 11, so a name too
/// long is shown and said so rather than cut.
const MAX_TYPED: usize = 32;

/// The Name field as typed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NameField {
    /// What was typed.
    pub typed: String,
    /// Whether typing goes to it.
    pub focused: bool,
    /// The caret, in characters; at the end when `None`.
    pub caret: Option<usize>,
}

/// The Name field's text as FAT writes it, in capitals, empty for none;
/// or why it cannot be written, as its line. A lookup.
pub fn checked(typed: &str) -> Result<String, &'static str> {
    let bytes = match faraday_fat::label_bytes(typed) {
        Ok(b) => b,
        Err(faraday_fat::Error::Name(faraday_fat::BadName::Long)) => {
            return Err(EN.starter_name_long);
        }
        Err(_) => return Err(EN.starter_name_character),
    };
    if bytes.is_some_and(|b| faraday_boot::layout::is_boot_label(&b)) {
        return Err(EN.starter_name_boot);
    }
    Ok(bytes.map_or_else(String::new, |b| {
        String::from_utf8_lossy(b.trim_ascii_end()).into_owned()
    }))
}

impl Starter {
    /// What to write's Name field, while it is offered: on a stick or
    /// card written whole. A lookup.
    pub fn name_field(&self) -> Option<&NameField> {
        self.making
            .as_ref()
            .filter(|m| m.mode == Some(Mode::Whole))
            .map(|m| &m.name)
    }

    /// Whether the Name field lets Continue go on: not offered, empty, or
    /// a name FAT can hold that is no boot partition's. A lookup.
    pub(crate) fn name_ready(&self) -> bool {
        self.name_field().is_none_or(|f| checked(&f.typed).is_ok())
    }

    /// The name the write carries, as FAT writes it: `None` with no name
    /// or where the field is not offered.
    pub(crate) fn name_order(&self) -> Option<String> {
        let f = self.name_field()?;
        checked(&f.typed).ok().filter(|n| !n.is_empty())
    }

    /// Where typing goes, as the frame draws it: the Name field while it
    /// is focused on What to write.
    pub(crate) fn name_edit(&self) -> Edit<faraday_core::Action> {
        match self.name_field() {
            Some(f) if f.focused && self.page == Page::Mode => Edit {
                field: Some(StarterAction::Name.host()),
                caret: f.caret,
                anchor: None,
            },
            _ => Edit::default(),
        }
    }

    /// Any press: the Name field takes typing when it is pressed, and
    /// lets it go when anything else is.
    pub(crate) fn name_pressed(&mut self, action: StarterAction, x: Option<i32>) {
        let caret = x.and_then(|x| {
            self.fields
                .iter()
                .find(|f| f.action == StarterAction::Name.host())
                .map(|f: &FieldAt<_>| f.index_at(x))
        });
        let offered = self.page == Page::Mode && self.name_field().is_some();
        if let Some(m) = self.making.as_mut() {
            m.name.focused = action == StarterAction::Name && offered;
            if m.name.focused && caret.is_some() {
                m.name.caret = caret;
            }
        }
    }

    /// A key, while the Name field takes typing: whether it took it.
    pub(crate) fn name_key(&mut self, key: Key) -> bool {
        if self.page != Page::Mode || !self.name_field().is_some_and(|f| f.focused) {
            return false;
        }
        let Some(f) = self.making.as_mut().map(|m| &mut m.name) else {
            return false;
        };
        let n = f.typed.chars().count();
        let at = f.caret.map_or(n, |c| c.min(n));
        let byte = |s: &str, i: usize| s.char_indices().nth(i).map_or(s.len(), |(b, _)| b);
        let to = match key {
            Key::Char(c) if !c.is_control() && n < MAX_TYPED => {
                f.typed.insert(byte(&f.typed, at), c);
                at + 1
            }
            Key::Backspace if at > 0 => {
                f.typed.remove(byte(&f.typed, at - 1));
                at - 1
            }
            Key::Delete if at < n => {
                f.typed.remove(byte(&f.typed, at));
                at
            }
            Key::Left => at.saturating_sub(1),
            Key::Right => (at + 1).min(n),
            Key::Home => 0,
            Key::End => n,
            Key::Enter | Key::Tab => {
                f.focused = false;
                at
            }
            Key::Char(_) | Key::Backspace | Key::Delete => at,
            _ => return false,
        };
        f.caret = Some(to);
        true
    }
}
