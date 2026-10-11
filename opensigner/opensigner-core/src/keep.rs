//! Keeping keys on the device, as the app runs it: the one exchange
//! with the shell's secure element in flight, where it leaves the
//! person, and the PIN pads that start it (`docs/PLANNING.md` §6, §15
//! items 32 and 9). What the stored bytes are, and the cryptography over
//! them, is `osk-keep`.
//!
//! This file is listed in `tools/lint-secrets.sh` and may hold no heap
//! text.

use alloc::vec::Vec;

use osk_crypto::{Secret, Zeroize};
use osk_keep::{Guess, Header};

use crate::session::PinEntry;

/// What the core asked the shell for on the kept-secret channel, and
/// what it will do with the answer. Exactly one exchange is in flight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// A tag, to write a fresh blob for the key on the Hold.
    Keeping,
    /// The blob and then a tag, to open it with the PIN just typed.
    Opening,
    /// The blob and then a tag, to rewrite its duress record.
    Duress,
    /// The shell's word that the blob is kept, and then this screen.
    Storing(Landing),
    /// The shell's word that the blob is gone, and then this screen.
    Forgetting(Landing),
}

/// Where a finished exchange leaves the person.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Landing {
    /// The key's own menu, where "Keep on this device" was tapped.
    KeyMenu,
    /// Where the person already is: the pad after a wrong PIN.
    Stay,
    /// Settings, where the duress PIN and the forget row are.
    Settings,
    /// The Result that says the stored key is gone.
    Removed,
    /// Home with nothing on it: the duress PIN was entered, so the app
    /// shows no stored key and no error.
    Empty,
}

/// The state of the one exchange in flight, and of the pad that started
/// it. Everything secret is a fixed array that zeroizes on drop.
pub struct Flow {
    /// The exchange in flight, if any.
    pub step: Option<Step>,
    /// The header of the blob under work.
    pub header: Option<Header>,
    /// The PIN typed for the exchange in flight, stretched: it is made
    /// when the challenge is sent and used when the element answers, so
    /// Argon2id runs once per typed PIN.
    pub guess: Option<Guess>,
    /// The blob under work, while the shell's answer is being used.
    pub blob: Vec<u8>,
    /// The key the Hold is keeping, by its index.
    pub key: usize,
    /// The duress pad is on its repeat step.
    pub repeat: bool,
    /// The two duress entries differed.
    pub mismatch: bool,
    /// Bytes the writes draw their salts and nonces from.
    seed: Secret<[u8; 32]>,
    entry: PinEntry,
    first: PinEntry,
}

impl Default for Flow {
    fn default() -> Self {
        Self::new()
    }
}

impl Flow {
    /// Nothing in flight and nothing typed.
    pub fn new() -> Self {
        Flow {
            step: None,
            header: None,
            guess: None,
            blob: Vec::new(),
            key: 0,
            repeat: false,
            mismatch: false,
            seed: Secret::new([0; 32]),
            entry: PinEntry::new(),
            first: PinEntry::new(),
        }
    }

    /// Forgets the exchange, the blob and everything typed.
    pub fn clear(&mut self) {
        self.step = None;
        self.header = None;
        self.guess = None;
        self.blob.zeroize();
        self.blob = Vec::new();
        self.key = 0;
        self.repeat = false;
        self.mismatch = false;
        self.seed.zeroize();
        self.entry.clear();
        self.first.clear();
    }

    /// The bytes this flow's writes draw their salts and nonces from.
    pub fn seed(&self) -> &[u8; 32] {
        self.seed.expose()
    }

    /// Sets them, from the session key.
    pub fn set_seed(&mut self, seed: [u8; 32]) {
        self.seed = Secret::new(seed);
    }

    /// The digits typed on the pad.
    pub fn pin_len(&self) -> usize {
        self.entry.len()
    }

    /// Whether they are enough to be a PIN.
    pub fn pin_complete(&self) -> bool {
        self.entry.is_complete()
    }

    /// Types a digit.
    pub fn push(&mut self, c: char) {
        self.entry.push(c);
    }

    /// Deletes one.
    pub fn pop(&mut self) {
        self.entry.pop();
    }

    /// Wipes what was typed, so the next attempt starts empty.
    pub fn clear_entry(&mut self) {
        self.entry.clear();
    }

    /// Runs `f` on the digits typed.
    pub fn with_pin<R>(&self, f: impl FnOnce(&[u8]) -> R) -> R {
        f(self.entry.digits())
    }

    /// The digits typed, for the session PIN a stored key becomes.
    pub fn entry(&self) -> &PinEntry {
        &self.entry
    }

    /// Types the digits of `pin` onto this pad, replacing what was
    /// there: the lock screen hands its entry over when the stored key
    /// is asked (§16.63).
    pub fn set_entry(&mut self, pin: &PinEntry) {
        self.entry.clear();
        for &d in pin.digits() {
            self.entry.push(char::from(d));
        }
    }

    /// The duress pad's first step is done: the digits move aside and
    /// the pad asks again. `false` while too few are typed.
    pub fn repeat_duress(&mut self) -> bool {
        if !self.entry.is_complete() {
            return false;
        }
        self.mismatch = false;
        core::mem::swap(&mut self.entry, &mut self.first);
        self.entry.clear();
        self.repeat = true;
        true
    }

    /// The repeat matched: the digits stay in `entry` for the write.
    /// `false` restarts the entry with "PINs differed" on the line.
    pub fn confirm_duress(&mut self) -> bool {
        if !self.entry.is_complete() {
            return false;
        }
        if self.entry.same_as(&self.first) {
            self.first.clear();
            return true;
        }
        self.entry.clear();
        self.first.clear();
        self.repeat = false;
        self.mismatch = true;
        false
    }
}

impl Zeroize for Flow {
    fn zeroize(&mut self) {
        self.clear();
    }
}

impl Drop for Flow {
    fn drop(&mut self) {
        self.zeroize();
    }
}
