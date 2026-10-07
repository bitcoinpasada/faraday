//! New input devices must show a person is at the screen before they are
//! believed (`PLAN.md` §4.6, layer 3).
//!
//! The shell trusts the machine's own keyboard, touchpad and touch panel.
//! Any other device's input is held back by the shell and reported here
//! instead: a new keyboard is believed once it types the code shown on
//! screen, which a device that cannot see the screen cannot do; a new
//! pointer is believed once a person says so with input already trusted,
//! or with the code typed on a keyboard that comes with it. **Ignore this
//! device** keeps it out until it is unplugged; ignored devices are listed
//! in Settings.

use crate::{Faraday, Sheet};

/// How long the code is.
pub const CODE_LEN: usize = 6;

/// The code's letters: no two that look alike.
const LETTERS: &[u8] = b"acdefhjkmnprtvwxy";

/// A device whose input is held back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewInput {
    /// The shell's number for it.
    pub id: u32,
    /// What it calls itself: a claim, not a fact.
    pub name: String,
    /// It types.
    pub keyboard: bool,
    /// It points.
    pub pointer: bool,
    /// The code it must type.
    pub code: String,
    /// What it has typed so far, the last few letters.
    pub typed: String,
}

impl Faraday {
    /// A device the shell holds back.
    pub(crate) fn input_new(&mut self, id: u32, name: String, keyboard: bool, pointer: bool) {
        if self.inputs.iter().any(|i| i.id == id) {
            return;
        }
        let draw = self.input_code_bytes(id);
        let code = draw
            .iter()
            .take(CODE_LEN)
            .map(|b| char::from(LETTERS[usize::from(*b) % LETTERS.len()]))
            .collect();
        self.inputs.push(NewInput {
            id,
            name,
            keyboard,
            pointer,
            code,
            typed: String::new(),
        });
        if self.sheet.is_none() {
            self.sheet = Some(Sheet::NewInput);
        }
    }

    /// A code from the session's entropy, one per device.
    fn input_code_bytes(&self, id: u32) -> [u8; 32] {
        use osk_bip::bitcoin::hashes::{Hash, HashEngine, sha256};
        let mut e = sha256::Hash::engine();
        e.input(b"faraday input code");
        e.input(&self.seed);
        e.input(&id.to_le_bytes());
        e.input(&self.now_ms.to_le_bytes());
        sha256::Hash::from_engine(e).to_byte_array()
    }

    /// A key a held-back keyboard typed.
    pub(crate) fn input_typed(&mut self, id: u32, ch: char) {
        let Some(i) = self.inputs.iter().position(|d| d.id == id) else {
            return;
        };
        let d = &mut self.inputs[i];
        d.typed.push(ch.to_ascii_lowercase());
        let keep = d.typed.chars().count().saturating_sub(CODE_LEN);
        d.typed = d.typed.chars().skip(keep).collect();
        if d.typed == d.code {
            self.input_believe(id);
        }
    }

    /// The device is believed from now on.
    pub(crate) fn input_believe(&mut self, id: u32) {
        let Some(i) = self.inputs.iter().position(|d| d.id == id) else {
            return;
        };
        let d = self.inputs.remove(i);
        self.input_decisions.push_back((id, true));
        self.toast(&format!("{} is in use", d.name));
        self.input_sheet();
    }

    /// The device is kept out until it is unplugged.
    pub(crate) fn input_ignore(&mut self, id: u32) {
        let Some(i) = self.inputs.iter().position(|d| d.id == id) else {
            return;
        };
        let d = self.inputs.remove(i);
        self.input_decisions.push_back((id, false));
        self.ignored_inputs.push((id, d.name));
        self.input_sheet();
    }

    /// The device was unplugged.
    pub(crate) fn input_gone(&mut self, id: u32) {
        self.inputs.retain(|d| d.id != id);
        self.ignored_inputs.retain(|(i, _)| *i != id);
        self.input_sheet();
    }

    /// The sheet shows while a device waits, and goes when none does.
    pub(crate) fn input_sheet(&mut self) {
        if self.inputs.is_empty() {
            if self.sheet == Some(Sheet::NewInput) {
                self.sheet = None;
            }
        } else if self.sheet.is_none() {
            self.sheet = Some(Sheet::NewInput);
        }
    }

    /// The next decision for the shell: a device believed (`true`) or kept
    /// out (`false`).
    pub fn poll_input_decision(&mut self) -> Option<(u32, bool)> {
        self.input_decisions.pop_front()
    }
}
