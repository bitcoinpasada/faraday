//! The sheet's state: a secret waiting for the person to say where it goes, and the hold that
//! sends one out as a plain file (`main`'s `SecretsArea`).

use faraday_kernel::{AtLock, Holds, Public};
use faraday_text::Text;

use crate::sheet::{Keep, Plain, SecretOut, Then};

/// Why the last press changed nothing, for the owner to say.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Answer {
    /// No vault is open to put the secret in.
    NoVaultOpen,
    /// The vault refused the record (it does not fit, or holds it): the sheet stays as it was.
    VaultRefused,
}

/// What the last press finished, for the owner to act on: it ends the sheet, and the part tells
/// its owner nothing by an out.
#[derive(Public, PartialEq, Eq)]
pub enum Finished {
    /// The secret went into the open vault as `what` (a file's name, or a key's label).
    SavedToVault { what: Text },
    /// The secret went to the Outbox unprotected: this many files.
    Unprotected { names: Vec<Text> },
    /// An Inbox file on its way to another stick: ticked on the visit, written from the Inbox.
    ToVisit { name: Text },
    /// The hold was held for something the owner does.
    Held(Then),
    /// Nothing chosen: the secret was dropped; a plan's file asked about leaves the plan, which
    /// is as it was before; a received file is not saved.
    Cancelled(Option<Then>),
    /// The sheet closes with no message (a key that changed, a record that cannot be made, a
    /// secret no vault takes).
    Closed,
}

/// The secret sheet's state.
#[derive(Default)]
pub struct State {
    pub(crate) out: Option<SecretOut>,
    pub(crate) plain: Option<Plain>,
    pub(crate) answer: Option<Answer>,
    pub(crate) finished: Option<Finished>,
}

impl State {
    /// An empty sheet.
    pub fn new() -> State {
        State::default()
    }

    /// What the last press finished, once.
    pub fn take_finished(&mut self) -> Option<Finished> {
        self.finished.take()
    }

    /// Why the last press changed nothing.
    pub fn answer(&self) -> Option<Answer> {
        self.answer
    }

    /// The secret waiting, if one is.
    pub fn out(&self) -> Option<&SecretOut> {
        self.out.as_ref()
    }

    /// The secret about to be saved as a plain file, waiting for its hold.
    pub fn plain(&self) -> Option<&Plain> {
        self.plain.as_ref()
    }

    /// Takes the secret away, to be put somewhere or dropped.
    pub fn take(&mut self) -> Option<SecretOut> {
        self.out.take()
    }

    /// Unticks field `number` of the waiting key's record, or ticks it again.
    pub(crate) fn toggle_left(&mut self, number: u8) {
        if let Some(out) = self.out.as_mut()
            && let Keep::Key { left, .. } = &mut out.keep
        {
            if left.contains(&number) {
                left.retain(|n| *n != number);
            } else {
                left.push(number);
            }
        }
    }

    /// The file's form `i`, picked on the sheet.
    pub(crate) fn pick_form(&mut self, i: usize) {
        if let Some(out) = self.out.as_mut()
            && let Some(f) = out.forms.get(i)
        {
            out.name = f.name.clone();
            out.bytes = faraday_port::bytes::Bytes::new(f.bytes.expose().clone());
            out.form = i;
        }
    }

    /// Drops the secret, its bytes wiped.
    pub(crate) fn clear(&mut self) {
        self.out = None;
        self.plain = None;
        self.answer = None;
        self.finished = None;
    }
}

/// A secret on its way out. A sheet for an Inbox file on its way to another stick holds a copy of
/// what the Inbox already holds, and the stick it goes to must stay open (`docs/FLOWS.md`).
impl AtLock for State {
    fn holds(&self) -> Holds {
        Holds {
            kept: false,
            typed: self.out.as_ref().is_some_and(|o| !o.to_visit),
            session: false,
            asks_first: None,
        }
    }

    fn hide(&mut self) {}

    fn at_lock(&mut self) {
        self.clear();
    }
}

/// The read view the owner's `present` maps to a kit model.
pub struct Read<'s>(&'s State);

impl State {
    /// The view of the sheet.
    pub fn read(&self) -> Read<'_> {
        Read(self)
    }
}

impl<'s> Read<'s> {
    /// The secret waiting, if one is.
    pub fn out(&self) -> Option<&'s SecretOut> {
        self.0.out.as_ref()
    }

    /// The plain-save ask, if one is up.
    pub fn plain(&self) -> Option<&'s Plain> {
        self.0.plain.as_ref()
    }

    /// Why the last press changed nothing.
    pub fn answer(&self) -> Option<Answer> {
        self.0.answer
    }
}
