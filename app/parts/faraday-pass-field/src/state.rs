//! The part's state: the one holder of a typed passphrase (`main`'s `pass_field::PassField`).

use core::fmt;

use faraday_kernel::{AtLock, Holds, Public};
use faraday_text::secret::SecretText;

use crate::eye::Eye;
use crate::kind::{Kind, Refused};
use crate::pair;

/// Which of a pair's two fields.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Which {
    /// The passphrase.
    First,
    /// The one typed again.
    Again,
}

/// What the last key did to the field (`main`'s `PassKey`): the owner reads it after routing a
/// key, since a part tells its owner nothing by an out.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Last {
    /// No key yet, or a press, a fill or a clear since.
    Nothing,
    /// A character went in or came out, or the other field of a pair took the typing.
    Typed,
    /// The key was refused, and why.
    Refused(Refused),
    /// Enter: the screen decides what it means.
    Enter,
    /// Escape: the screen decides what it means.
    Escape,
    /// Not a key of a field.
    Ignored,
}

/// One passphrase entry: the text, its repeat where it is being made, and which of the two typing
/// goes to. It holds no plain text and has no `Clone` and no `PartialEq`; its `Debug` names
/// lengths only.
pub struct State {
    pub(crate) kind: Kind,
    pub(crate) text: SecretText,
    /// Some for a passphrase being made, typed twice.
    pub(crate) again: Option<SecretText>,
    /// Typing goes to the repeat.
    pub(crate) on_again: bool,
    pub(crate) eye: Eye,
    /// A hardware key's character makes the step wait for the eye (Add a key, a Create slot's key,
    /// the seed check).
    pub(crate) waits_for_eye: bool,
    pub(crate) last: Last,
}

impl State {
    /// A passphrase typed once.
    pub fn new(kind: Kind) -> State {
        State {
            kind,
            text: SecretText::new(),
            again: None,
            on_again: false,
            eye: Eye::default(),
            waits_for_eye: false,
            last: Last::Nothing,
        }
    }

    /// A passphrase being made, typed twice.
    pub fn pair(kind: Kind) -> State {
        State {
            again: Some(SecretText::new()),
            ..State::new(kind)
        }
    }

    /// A passphrase typed with hardware keys waits for the eye to be opened once.
    #[must_use]
    pub fn waiting_for_eye(mut self) -> State {
        self.waits_for_eye = true;
        self
    }

    /// What the field takes.
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// What the last key did.
    pub fn last(&self) -> Last {
        self.last
    }

    /// The field is a pair.
    pub fn is_pair(&self) -> bool {
        self.again.is_some()
    }

    /// Typing goes to the repeat (of a pair).
    pub fn on_again(&self) -> bool {
        self.on_again && self.again.is_some()
    }

    /// The characters are drawn, not dots.
    pub fn shown(&self) -> bool {
        self.eye.shown()
    }

    /// The step waits for the eye to be opened once: a hardware key typed here and the eye has
    /// not been opened since.
    pub fn waits(&self) -> bool {
        self.waits_for_eye && self.eye.waits()
    }

    /// The passphrase is empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Either text holds something.
    pub fn holds_any(&self) -> bool {
        !self.text.is_empty() || self.again.as_ref().is_some_and(|a| !a.is_empty())
    }

    /// The field other than `which` holds text (a single field has no other).
    pub(crate) fn other_typed(&self, which: Which) -> bool {
        match (which, self.again.as_ref()) {
            (Which::First, Some(again)) => !again.is_empty(),
            (Which::Again, _) => !self.text.is_empty(),
            (Which::First, None) => false,
        }
    }

    /// Both texts' bytes.
    pub fn bytes(&self) -> usize {
        self.text.len() + self.again.as_ref().map_or(0, |a| a.len())
    }

    /// A pair is typed alike and not empty; a single one is not empty.
    pub fn ready(&self) -> bool {
        match &self.again {
            Some(again) => pair::ready(&self.text, again),
            None => !self.is_empty(),
        }
    }

    /// A pair differs (never for a single one).
    pub fn differ(&self) -> bool {
        self.again
            .as_ref()
            .is_some_and(|again| pair::differ(&self.text, again))
    }

    /// The repeat is begun while the two differ: the line "The two passphrases differ" shows (an
    /// empty second field is still to be typed, not a mistake).
    pub fn differs_now(&self) -> bool {
        self.again
            .as_ref()
            .is_some_and(|again| !again.is_empty() && self.differ())
    }

    /// Moves the passphrase out and leaves the field empty: where the screen ends with the
    /// hand-off.
    pub fn take(&mut self) -> SecretText {
        core::mem::take(&mut self.text)
    }

    /// One named copy for a job, where the screen keeps its field for a Back or an error; it wipes
    /// with the job.
    pub fn job_copy(&self) -> SecretText {
        SecretText::of(self.text.as_str())
    }

    /// The passphrase, for a call of `faraday-keys` that takes one.
    pub fn secret(&self) -> &SecretText {
        &self.text
    }

    /// Wipes both texts, puts typing back on the first and closes the eye.
    pub(crate) fn clear(&mut self) {
        self.text.clear();
        if let Some(again) = self.again.as_mut() {
            again.clear();
        }
        self.on_again = false;
        self.eye.close();
        self.eye.reset();
        self.last = Last::Nothing;
    }
}

/// Lengths only, never the text.
impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "pass_field::State({:?}, {} bytes, {} again)",
            self.kind,
            self.text.len(),
            self.again.as_ref().map_or(0, |a| a.len())
        )
    }
}

impl AtLock for State {
    fn holds(&self) -> Holds {
        Holds {
            kept: false,
            typed: self.holds_any(),
            session: false,
            asks_first: None,
        }
    }

    /// The eye closes; what was typed stays.
    fn hide(&mut self) {
        self.eye.close();
    }

    fn at_lock(&mut self) {
        self.clear();
    }
}
