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
//! trusted, or with the code typed on a keyboard that comes with it. A
//! new touchscreen (one on USB: the machine's own panel is trusted by the
//! shell) is believed once it taps the target drawn at a place on the
//! screen it cannot see, and one that taps beside it [`TAP_TRIES`] times
//! is kept out until it is unplugged and counts toward
//! [`KEPT_OUT_LIMIT`] as a keyboard does (F153). Nothing is believed for
//! the name it gives. **Ignore this device** keeps it out until it is
//! unplugged; ignored devices are listed in Settings. The code and the
//! target's place are drawn from the session's seed, which is the
//! system's randomness: a device that arrives before the seed waits
//! without them, held back, and gets them when the seed arrives.

use crate::strings::EN;
use crate::strings::fill;
use crate::{Action, Faraday, Sheet, StorageEvent};
use std::collections::VecDeque;

/// How long the code is.
pub const CODE_LEN: usize = 6;

/// How many codes' worth of letters a keyboard may type before it is
/// kept out: a person who mistypes has room to type it again.
pub const CODE_TRIES: usize = 3;

/// How many keyboards may be kept out for the wrong code in a session
/// before new keyboards are ignored without a code.
pub const KEPT_OUT_LIMIT: usize = 3;

/// How many taps beside the target a touchscreen may make before it is
/// kept out: a person whose finger slips has room to tap again.
pub const TAP_TRIES: usize = 3;

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
    /// It names places on the screen: a touchscreen.
    pub touch: bool,
    /// The code it must type; `None` until the session's seed arrived.
    pub code: Option<String>,
    /// Where the target it must tap is, as fractions of the room the
    /// sheet leaves for it (0 to `u16::MAX` across and down); `None`
    /// until the session's seed arrived. Drawn with the code.
    pub spot: Option<(u16, u16)>,
    /// How many times it tapped beside the target.
    pub missed: usize,
    /// What it has typed so far, the last few letters.
    pub typed: String,
    /// How many letters it has typed in all.
    pub letters: usize,
}

impl NewInput {
    /// It is let in by tapping a target rather than typing a code: a
    /// touchscreen with no keys of its own.
    pub fn by_target(&self) -> bool {
        self.touch && !self.keyboard
    }
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
    /// It typed too many letters without it, or tapped beside the
    /// target too often, and is kept out;
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
    /// How many keyboards and touchscreens were kept out for the wrong
    /// code or taps beside the target this session.
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

    /// A device the shell holds back; `draw` draws its code and its
    /// target's place, and is called only when it is held.
    pub(crate) fn admit(
        &mut self,
        id: u32,
        name: String,
        (keyboard, pointer, touch): (bool, bool, bool),
        draw: impl FnOnce() -> Option<(String, (u16, u16))>,
    ) -> Admitted {
        if self.held.iter().any(|i| i.id == id) {
            return Admitted::Known;
        }
        if (keyboard || touch) && self.kept_out >= KEPT_OUT_LIMIT {
            self.decisions.push_back((id, false));
            self.ignored.push((id, name));
            return Admitted::Ignored;
        }
        let (code, spot) = draw().unzip();
        self.held.push(NewInput {
            id,
            name,
            keyboard,
            pointer,
            touch,
            code,
            spot,
            missed: 0,
            typed: String::new(),
            letters: 0,
        });
        Admitted::Held
    }

    /// The seed arrived: each device waiting without a code gets one,
    /// and its target's place, from `draw`.
    pub(crate) fn coded(&mut self, draw: impl Fn(u32) -> Option<(String, (u16, u16))>) {
        for d in &mut self.held {
            if d.code.is_none() {
                (d.code, d.spot) = draw(d.id).unzip();
            }
        }
    }

    /// A tap a held-back touchscreen made while its target was drawn:
    /// on the target, or beside it.
    pub(crate) fn tapped(&mut self, id: u32, on_target: bool) -> Typed {
        let Some(d) = self.held.iter_mut().find(|d| d.id == id) else {
            return Typed::Nothing;
        };
        // With no target to tap, a tap is held back and counts for nothing.
        if !d.by_target() || d.spot.is_none() {
            return Typed::Nothing;
        }
        if on_target {
            return Typed::Matched;
        }
        d.missed += 1;
        if d.missed < TAP_TRIES {
            return Typed::Nothing;
        }
        let name = d.name.clone();
        self.ignore(id);
        self.kept_out += 1;
        Typed::KeptOut {
            name,
            limit: self.kept_out >= KEPT_OUT_LIMIT,
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

    /// What the shell said of a held-back device: one announced, a key it
    /// typed, a tap it made, or one unplugged.
    pub(crate) fn input_event(&mut self, event: StorageEvent) {
        match event {
            StorageEvent::NewInput {
                id,
                name,
                keyboard,
                pointer,
                touch,
            } => self.input_new(id, name, (keyboard, pointer, touch)),
            StorageEvent::InputTyped { id, ch } => self.input_typed(id, ch),
            StorageEvent::InputTapped { id, x, y } => self.input_tapped(id, x, y),
            StorageEvent::InputGone { id } => self.input_gone(id),
            // `storage` passes only the events matched above.
            _ => {}
        }
    }

    /// A device the shell holds back, with what it says it is: keyboard,
    /// pointer, touchscreen.
    pub(crate) fn input_new(&mut self, id: u32, name: String, kinds: (bool, bool, bool)) {
        let (fresh, now) = (&self.fresh, self.now_ms);
        match self
            .inputs
            .admit(id, name, kinds, || input_code(fresh, id, now))
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
            self.import.set_under_input(true);
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
        let typed = self.inputs.typed(id, ch);
        self.input_tried(id, typed, EN.inputs_wrong_code);
    }

    /// A tap a held-back touchscreen made at (x, y), in pixels. It counts
    /// only while the sheet shows that device's target: on the target it
    /// lets the device in, on the sheet's Ignore it keeps it out, and
    /// anywhere else it is a tap beside the target. A lookup.
    pub(crate) fn input_tapped(&mut self, id: u32, x: u16, y: u16) {
        let showing = self.sheet == Some(Sheet::NewInput)
            && self.inputs.held().first().is_some_and(|d| d.id == id);
        if !showing {
            return;
        }
        let (x, y) = (i32::from(x), i32::from(y));
        // Only the control's own box: no room around it.
        let hit = self
            .pointer
            .hit_at(x, y)
            .filter(|(_, r)| r.contains(x, y))
            .map(|(a, _)| a);
        if hit == Some(Action::InputIgnore(id)) {
            self.input_ignore(id);
            return;
        }
        let tapped = self.inputs.tapped(id, hit == Some(Action::InputTarget(id)));
        self.input_tried(id, tapped, EN.inputs_wrong_taps);
    }

    /// What a held-back device's try led to; `wrong` is the toast for one
    /// kept out for it.
    fn input_tried(&mut self, id: u32, tried: Typed, wrong: &str) {
        match tried {
            Typed::Nothing => {}
            Typed::Matched => self.input_believe(id),
            Typed::KeptOut { name, limit } => {
                self.input_sheet();
                if limit {
                    self.toast(EN.inputs_new_keyboards_ignored);
                } else {
                    self.toast(&fill(wrong, &[&name]));
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
                let import = self.import.take_under_input();
                self.sheet = (import && self.import.state().is_some()).then_some(Sheet::Import);
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

/// A code and a target's place from the session's seed, one per device;
/// `None` before the seed arrived. The code is the draw's first bytes,
/// the place four of the bytes after it.
fn input_code(fresh: &crate::fresh::Fresh, id: u32, now_ms: u64) -> Option<(String, (u16, u16))> {
    let count = [id.to_le_bytes().as_slice(), &now_ms.to_le_bytes()].concat();
    let draw = fresh.draw(b"faraday input code", &count)?;
    let code = draw
        .iter()
        .take(CODE_LEN)
        .map(|b| char::from(LETTERS[usize::from(*b) % LETTERS.len()]))
        .collect();
    let spot = (
        u16::from_le_bytes([draw[8], draw[9]]),
        u16::from_le_bytes([draw[10], draw[11]]),
    );
    Some((code, spot))
}
