//! Send text: any text a person brings to the online app, shown as codes
//! for the device's camera, except a secret (F236). The text is never
//! kept past Transfer: it is wiped when the screen is left.
//!
//! The refusal is [`crate::text_check`]'s, the device's own classifier
//! reused. It is worked out when the text changes and once more at **Show
//! as QR**; drawing reads the result.

use crate::text_check::{MAX_TEXT, TextCheck, check};
use faraday_crypto::work::Compute;
use faraday_text::Text;

/// The most text the field holds: what is pasted past it is dropped, so
/// the check never reads more than this.
const HOLD: usize = 4 * MAX_TEXT;

/// A press on the Send text card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextPress {
    /// Put the clipboard's text in the field.
    Paste,
    /// Empty the field.
    Clear,
    /// Show the text as codes.
    Show,
}

/// The Send text card's field, and what its check said.
pub struct SendText {
    text: Text,
    check: TextCheck,
    /// A paste was longer than the field holds, so it is cut: refused
    /// until the field is cleared.
    over: bool,
}

/// Prints the length of the text, never the text.
impl std::fmt::Debug for SendText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(SendText))
            .field("len", &self.text.len())
            .field("check", &self.check)
            .field("over", &self.over)
            .finish()
    }
}

impl Default for SendText {
    fn default() -> Self {
        Self {
            text: Text::new(),
            check: TextCheck::Fine,
            over: false,
        }
    }
}

impl SendText {
    /// The text in the field.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// What the check said of it.
    pub fn check(&self) -> TextCheck {
        self.check
    }

    /// The text as it is sent: its ends trimmed. `None` while there is
    /// nothing to send or the check refused it.
    pub fn sendable(&self) -> Option<&str> {
        let t = self.text.trim();
        (!t.is_empty() && self.check == TextCheck::Fine).then_some(t)
    }

    /// Whether the field holds anything.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Empties the field, wiping what it held.
    pub(crate) fn clear(&mut self) {
        self.text.clear();
        self.check = TextCheck::Fine;
        self.over = false;
    }

    /// Costs what [`check`] costs. A key typed into the field: whether it
    /// was taken. A character, Enter as a new line and Backspace.
    pub(crate) fn key(&mut self, c: &Compute, key: faraday_shell_api::Key) -> bool {
        use faraday_shell_api::Key as K;
        match key {
            K::Char(ch) if !ch.is_control() => {
                if self.text.len() + ch.len_utf8() <= HOLD {
                    self.text.push(ch);
                }
            }
            K::Enter => {
                if self.text.len() < HOLD {
                    self.text.push('\n');
                }
            }
            K::Backspace => {
                self.text.pop();
            }
            _ => return false,
        }
        self.recheck(c);
        true
    }

    /// Costs what [`check`] costs. The clipboard's text in place of
    /// what the field held.
    pub(crate) fn put(&mut self, c: &Compute, text: &str) {
        let mut cut = text.len().min(HOLD);
        while !text.is_char_boundary(cut) {
            cut -= 1;
        }
        self.text = Text::of(&text[..cut]);
        self.over = cut < text.len();
        self.recheck(c);
    }

    /// Costs what [`check`] costs. The check made again, on the text now
    /// in the field.
    pub(crate) fn recheck(&mut self, c: &Compute) {
        self.check = if self.over {
            TextCheck::TooLong
        } else {
            check(c, &self.text)
        };
    }
}
