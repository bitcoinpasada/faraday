//! New input devices must show a person is at the screen before they are
//! believed (`docs/history/PLAN.md` §4.6, layer 3).
//!
//! The shell trusts the machine's own keyboard, touchpad and touch panel.
//! Any other device's input is held back by the shell and reported here
//! instead: a new keyboard is believed once it types the code shown on
//! screen, which a device that cannot see the screen cannot do, and one
//! that types [`CODE_TRIES`] codes' worth of letters without it is kept
//! out until it is unplugged, so it cannot try codes one after another.
//! Once [`KEPT_OUT_LIMIT`] keyboards have been kept out that way, new
//! keyboards are ignored until Faraday starts again, so a device that
//! unplugs and plugs itself back in gets no fresh codes to try. A new
//! pointer is believed once a person says so with input already
//! trusted, or with the code typed on a keyboard that comes with it.
//! **Ignore this device** keeps it out until it is unplugged; ignored
//! devices are listed in Settings. The code is drawn from the session's
//! seed: a keyboard that arrives before the seed waits without one, held
//! back, and gets one when the seed arrives.

use crate::strings::EN;
use crate::strings::fill;
use crate::{Faraday, Sheet};

/// How long the code is.
pub const CODE_LEN: usize = 6;

/// How many codes' worth of letters a keyboard may type before it is
/// kept out: a person who mistypes has room to type it again.
pub const CODE_TRIES: usize = 3;

/// How many keyboards may be kept out for the wrong code in a session
/// before new keyboards are ignored without a code.
pub const KEPT_OUT_LIMIT: usize = 3;

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
    /// The code it must type; `None` until the session's seed arrived.
    pub code: Option<String>,
    /// What it has typed so far, the last few letters.
    pub typed: String,
    /// How many letters it has typed in all.
    pub letters: usize,
}

impl Faraday {
    /// A device the shell holds back.
    pub(crate) fn input_new(&mut self, id: u32, name: String, keyboard: bool, pointer: bool) {
        if self.inputs.iter().any(|i| i.id == id) {
            return;
        }
        if keyboard && self.inputs_kept_out >= KEPT_OUT_LIMIT {
            self.input_decisions.push_back((id, false));
            self.ignored_inputs.push((id, name));
            self.toast(EN.inputs_new_keyboards_ignored);
            return;
        }
        let code = self.input_code(id);
        self.inputs.push(NewInput {
            id,
            name,
            keyboard,
            pointer,
            code,
            typed: String::new(),
            letters: 0,
        });
        // The boot import waits for it: which pointer and keyboard are
        // believed is the first thing settled (`docs/history/PLAN.md` §4.6).
        if self.sheet == Some(Sheet::Import) {
            self.import_under_input = true;
            self.sheet = None;
        }
        if self.sheet.is_none() {
            self.sheet = Some(Sheet::NewInput);
        }
    }

    /// A code from the session's seed, one per device; `None` before the
    /// seed arrived.
    fn input_code(&self, id: u32) -> Option<String> {
        let count = [id.to_le_bytes().as_slice(), &self.now_ms.to_le_bytes()].concat();
        let draw = self.seed.as_ref()?.draw(b"faraday input code", &count);
        Some(
            draw.iter()
                .take(CODE_LEN)
                .map(|b| char::from(LETTERS[usize::from(*b) % LETTERS.len()]))
                .collect(),
        )
    }

    /// The seed arrived: each device waiting without a code gets one.
    pub(crate) fn inputs_coded(&mut self) {
        for i in 0..self.inputs.len() {
            if self.inputs[i].code.is_none() {
                let code = self.input_code(self.inputs[i].id);
                self.inputs[i].code = code;
            }
        }
    }

    /// A key a held-back keyboard typed.
    pub(crate) fn input_typed(&mut self, id: u32, ch: char) {
        let Some(i) = self.inputs.iter().position(|d| d.id == id) else {
            return;
        };
        let d = &mut self.inputs[i];
        // With no code to type, typing is held back and counts for nothing.
        if d.code.is_none() {
            return;
        }
        d.typed.push(ch.to_ascii_lowercase());
        d.letters += 1;
        let keep = d.typed.chars().count().saturating_sub(CODE_LEN);
        d.typed = d.typed.chars().skip(keep).collect();
        if d.code.as_deref() == Some(d.typed.as_str()) {
            self.input_believe(id);
        } else if d.letters >= CODE_LEN * CODE_TRIES {
            let name = d.name.clone();
            self.input_ignore(id);
            self.inputs_kept_out += 1;
            if self.inputs_kept_out >= KEPT_OUT_LIMIT {
                self.toast(EN.inputs_new_keyboards_ignored);
            } else {
                self.toast(&fill(EN.inputs_wrong_code, &[&name]));
            }
        }
    }

    /// The device is believed from now on.
    pub(crate) fn input_believe(&mut self, id: u32) {
        let Some(i) = self.inputs.iter().position(|d| d.id == id) else {
            return;
        };
        let d = self.inputs.remove(i);
        self.input_decisions.push_back((id, true));
        self.toast(&fill(EN.inputs_is_in_use, &[&d.name.to_string()]));
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

    /// The sheet shows while a device waits, and goes when none does,
    /// back to the boot import's sheet if that waited under it.
    pub(crate) fn input_sheet(&mut self) {
        if self.inputs.is_empty() {
            if self.sheet == Some(Sheet::NewInput) {
                let import = std::mem::take(&mut self.import_under_input);
                self.sheet = (import && self.import.is_some()).then_some(Sheet::Import);
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
