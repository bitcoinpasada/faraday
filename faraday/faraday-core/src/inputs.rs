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
use std::collections::VecDeque;

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

/// How a device announced is taken.
pub(crate) enum Admitted {
    /// Already waiting: nothing changes.
    Known,
    /// A keyboard past the limit: ignored without a code.
    Ignored,
    /// Held back, waiting for its code or a decision.
    Held,
}

/// What a letter typed by a held-back keyboard led to.
pub(crate) enum Typed {
    /// Nothing: no such device, no code to type yet, or not yet enough.
    Nothing,
    /// It typed its code: it is to be believed.
    Matched,
    /// It typed too many letters without it and is kept out;
    /// `limit` is whether the limit of such keyboards is now reached.
    KeptOut { name: String, limit: bool },
}

/// The input devices area: the devices held back, the ones ignored, how
/// many were kept out for the wrong code, and the decisions waiting for
/// the shell. Its fields are private; the code is drawn from the
/// session's seed by the glue.
#[derive(Default)]
pub(crate) struct InputsArea {
    /// Devices held back, waiting for a code or a decision.
    held: Vec<NewInput>,
    /// Devices kept out until they are unplugged: the shell's number and
    /// the name it gave.
    ignored: Vec<(u32, String)>,
    /// How many keyboards were kept out for the wrong code this session.
    kept_out: usize,
    /// Decisions the shell has not been told: believed (`true`) or kept
    /// out (`false`).
    decisions: VecDeque<(u32, bool)>,
}

impl InputsArea {
    /// The devices held back.
    pub(crate) fn held(&self) -> &[NewInput] {
        &self.held
    }

    /// The devices kept out until unplugged.
    pub(crate) fn ignored(&self) -> &[(u32, String)] {
        &self.ignored
    }

    /// A device the shell holds back; `code` draws its code, and is
    /// called only when it is held.
    pub(crate) fn admit(
        &mut self,
        id: u32,
        name: String,
        keyboard: bool,
        pointer: bool,
        code: impl FnOnce() -> Option<String>,
    ) -> Admitted {
        if self.held.iter().any(|i| i.id == id) {
            return Admitted::Known;
        }
        if keyboard && self.kept_out >= KEPT_OUT_LIMIT {
            self.decisions.push_back((id, false));
            self.ignored.push((id, name));
            return Admitted::Ignored;
        }
        self.held.push(NewInput {
            id,
            name,
            keyboard,
            pointer,
            code: code(),
            typed: String::new(),
            letters: 0,
        });
        Admitted::Held
    }

    /// The seed arrived: each device waiting without a code gets one
    /// from `code`.
    pub(crate) fn coded(&mut self, code: impl Fn(u32) -> Option<String>) {
        for d in &mut self.held {
            if d.code.is_none() {
                d.code = code(d.id);
            }
        }
    }

    /// A key a held-back keyboard typed.
    pub(crate) fn typed(&mut self, id: u32, ch: char) -> Typed {
        let Some(i) = self.held.iter().position(|d| d.id == id) else {
            return Typed::Nothing;
        };
        let d = &mut self.held[i];
        // With no code to type, typing is held back and counts for nothing.
        if d.code.is_none() {
            return Typed::Nothing;
        }
        d.typed.push(ch.to_ascii_lowercase());
        d.letters += 1;
        let keep = d.typed.chars().count().saturating_sub(CODE_LEN);
        d.typed = d.typed.chars().skip(keep).collect();
        if d.code.as_deref() == Some(d.typed.as_str()) {
            Typed::Matched
        } else if d.letters >= CODE_LEN * CODE_TRIES {
            let name = d.name.clone();
            self.ignore(id);
            self.kept_out += 1;
            Typed::KeptOut {
                name,
                limit: self.kept_out >= KEPT_OUT_LIMIT,
            }
        } else {
            Typed::Nothing
        }
    }

    /// The device is believed from now on; its name, if it was waiting.
    pub(crate) fn believe(&mut self, id: u32) -> Option<String> {
        let i = self.held.iter().position(|d| d.id == id)?;
        let d = self.held.remove(i);
        self.decisions.push_back((id, true));
        Some(d.name)
    }

    /// The device is kept out until it is unplugged.
    pub(crate) fn ignore(&mut self, id: u32) {
        let Some(i) = self.held.iter().position(|d| d.id == id) else {
            return;
        };
        let d = self.held.remove(i);
        self.decisions.push_back((id, false));
        self.ignored.push((id, d.name));
    }

    /// The device was unplugged.
    pub(crate) fn gone(&mut self, id: u32) {
        self.held.retain(|d| d.id != id);
        self.ignored.retain(|(i, _)| *i != id);
    }

    /// The next decision for the shell.
    pub(crate) fn next_decision(&mut self) -> Option<(u32, bool)> {
        self.decisions.pop_front()
    }
}

impl Faraday {
    /// The devices the shell holds back, waiting for their code.
    pub fn new_inputs(&self) -> &[NewInput] {
        self.inputs.held()
    }

    /// The devices kept out until they are unplugged: the shell's number
    /// and the name each gave.
    pub fn ignored_inputs(&self) -> &[(u32, String)] {
        self.inputs.ignored()
    }

    /// A device the shell holds back.
    pub(crate) fn input_new(&mut self, id: u32, name: String, keyboard: bool, pointer: bool) {
        let (fresh, now) = (&self.fresh, self.now_ms);
        match self
            .inputs
            .admit(id, name, keyboard, pointer, || input_code(fresh, id, now))
        {
            Admitted::Known => return,
            Admitted::Ignored => {
                self.toast(EN.inputs_new_keyboards_ignored);
                return;
            }
            Admitted::Held => {}
        }
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

    /// The seed arrived: each device waiting without a code gets one.
    pub(crate) fn inputs_coded(&mut self) {
        let (fresh, now) = (&self.fresh, self.now_ms);
        self.inputs.coded(|id| input_code(fresh, id, now));
    }

    /// A key a held-back keyboard typed.
    pub(crate) fn input_typed(&mut self, id: u32, ch: char) {
        match self.inputs.typed(id, ch) {
            Typed::Nothing => {}
            Typed::Matched => self.input_believe(id),
            Typed::KeptOut { name, limit } => {
                self.input_sheet();
                if limit {
                    self.toast(EN.inputs_new_keyboards_ignored);
                } else {
                    self.toast(&fill(EN.inputs_wrong_code, &[&name]));
                }
            }
        }
    }

    /// The device is believed from now on.
    pub(crate) fn input_believe(&mut self, id: u32) {
        let Some(name) = self.inputs.believe(id) else {
            return;
        };
        self.toast(&fill(EN.inputs_is_in_use, &[&name.to_string()]));
        self.input_sheet();
    }

    /// The device is kept out until it is unplugged.
    pub(crate) fn input_ignore(&mut self, id: u32) {
        self.inputs.ignore(id);
        self.input_sheet();
    }

    /// The device was unplugged.
    pub(crate) fn input_gone(&mut self, id: u32) {
        self.inputs.gone(id);
        self.input_sheet();
    }

    /// The sheet shows while a device waits, and goes when none does,
    /// back to the boot import's sheet if that waited under it.
    pub(crate) fn input_sheet(&mut self) {
        if self.inputs.held().is_empty() {
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
        self.inputs.next_decision()
    }
}

/// A code from the session's seed, one per device; `None` before the
/// seed arrived.
fn input_code(fresh: &crate::fresh::Fresh, id: u32, now_ms: u64) -> Option<String> {
    let count = [id.to_le_bytes().as_slice(), &now_ms.to_le_bytes()].concat();
    let draw = fresh.draw(b"faraday input code", &count)?;
    Some(
        draw.iter()
            .take(CODE_LEN)
            .map(|b| char::from(LETTERS[usize::from(*b) % LETTERS.len()]))
            .collect(),
    )
}
