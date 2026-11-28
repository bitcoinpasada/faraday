//! Input devices held back until a person is shown to be at the screen (`main`'s `inputs.rs`
//! 53-613; `docs/SECURITY-MODEL.md`, layer 3). A new keyboard is believed once it types the code
//! shown on screen, which a device that cannot see the screen cannot do; one that types
//! [`CODE_TRIES`] codes' worth of letters without it is kept out until it is unplugged. Once
//! [`KEPT_OUT_LIMIT`] have been kept out, new keyboards are ignored until Faraday starts again.
//! A touchscreen, and a pointer where no input is trusted, press several targets in a row
//! ([`target_count`]), and every press beside a target counts for the session until
//! [`BESIDE_LIMIT`] closes the door.

use faraday_kernel::{AtLock, Holds, Public};
use faraday_text::{Text, secret::SecretText};

/// How long the code is.
pub const CODE_LEN: usize = 6;

/// How many codes' worth of letters a keyboard may type before it is kept out.
pub const CODE_TRIES: usize = 3;

/// How many keyboards may be kept out for the wrong code in a session before new keyboards are
/// ignored without a code.
pub const KEPT_OUT_LIMIT: usize = 3;

/// How many taps beside the target a touchscreen may make before it is kept out.
pub const TAP_TRIES: usize = 3;

/// How many presses beside any target, from any device, close the door for the session.
pub const BESIDE_LIMIT: usize = KEPT_OUT_LIMIT * TAP_TRIES;

/// How many letters from keyboards that never typed their code close the door for the session.
pub const WRONG_LETTERS_LIMIT: usize = KEPT_OUT_LIMIT * CODE_LEN * CODE_TRIES;

/// The most targets in a row a sheet draws; a screen needing more shows none.
pub const TARGETS_MAX: usize = 16;

/// The code's letters: no two that look alike.
const LETTERS: &[u8] = b"acdefhjkmnprtvwxy";

/// The bytes one device's draw takes: the code's letters, and four for each target's place.
pub(crate) const DRAW_LEN: usize = CODE_LEN + 4 * TARGETS_MAX;

/// What `what` the draw is for.
pub(crate) const DRAW_WHAT: &str = "faraday input code";

/// A device whose input is held back. Never `Clone`, never printed: the code, the targets'
/// places and what it typed are secret until it is decided.
pub(crate) struct NewInput {
    /// The adapter's number for it.
    pub(crate) id: u32,
    /// What it calls itself: a claim, not a fact.
    pub(crate) name: Text,
    pub(crate) keyboard: bool,
    pub(crate) pointer: bool,
    /// It names places on the screen: a touchscreen.
    pub(crate) touch: bool,
    /// The code it must type; `None` until the session's randomness arrived.
    code: Option<SecretText>,
    /// Where each target it must press is, as fractions of the room the sheet leaves for it (0
    /// to `u16::MAX` across and down); `None` until the randomness arrived.
    spots: Option<Vec<(u16, u16)>>,
    /// The target showing: how many it has pressed in this run.
    pub(crate) reached: usize,
    /// How many times it pressed beside a target.
    pub(crate) missed: usize,
    /// What it has typed so far, the last few letters.
    typed: SecretText,
    /// How many letters it has typed in all.
    letters: usize,
    /// It has acted since it was held: only then does its sheet show.
    pub(crate) used: bool,
}

impl NewInput {
    /// The code it must type, while it is shown.
    pub(crate) fn code(&self) -> Option<&str> {
        self.code.as_ref().map(SecretText::as_str)
    }

    /// Whether it has a code and targets.
    pub(crate) fn drawn(&self) -> bool {
        self.code.is_some() && self.spots.is_some()
    }

    /// How many of the code's boxes show as typed: the length of the longest ending of what was
    /// typed that is a start of the code, so a wrong key takes the marks back to what still
    /// matches.
    pub(crate) fn marked(&self) -> usize {
        let typed = self.typed.as_str().as_bytes();
        let code = self.code().unwrap_or("").as_bytes();
        (1..=typed.len().min(code.len()))
            .rev()
            .find(|k| {
                typed
                    .get(typed.len() - k..)
                    .is_some_and(|t| code.get(..*k) == Some(t))
            })
            .unwrap_or(0)
    }

    /// Takes the code and the places from a draw's bytes; `false` when it is too short.
    fn take(&mut self, bytes: &[u8]) -> bool {
        let Some(letters) = bytes.get(..CODE_LEN) else {
            return false;
        };
        let mut code = SecretText::new();
        for b in letters {
            let at = usize::from(*b) % LETTERS.len();
            if let Some(l) = LETTERS.get(at) {
                code.push(char::from(*l));
            }
        }
        let Some(rest) = bytes.get(CODE_LEN..CODE_LEN + 4 * TARGETS_MAX) else {
            return false;
        };
        let spots = rest
            .chunks_exact(4)
            .map(|c| match *c {
                [a, b, x, y] => (u16::from_le_bytes([a, b]), u16::from_le_bytes([x, y])),
                _ => (0, 0),
            })
            .collect();
        self.code = Some(code);
        self.spots = Some(spots);
        true
    }

    /// What it holds that a lock wipes.
    fn wipe(&mut self) {
        self.code = None;
        self.spots = None;
        self.typed.clear();
        self.letters = 0;
        self.reached = 0;
    }
}

/// How many targets in a row a sheet draws, from the room it leaves for one (`room_w` and
/// `room_h`, which the target's place is drawn in) and the size of a touch (`touch`): the fewest
/// that make a blind device's chance in a session no better than a keyboard's. `None` when no
/// count up to [`TARGETS_MAX`] does.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    reason = "small counts, in arithmetic over a probability"
)]
pub fn target_count(room_w: f32, room_h: f32, touch: f32) -> Option<usize> {
    let side = |room: f32| {
        if room <= 0.0 {
            1.0
        } else {
            f64::from((touch / room).min(1.0))
        }
    };
    let one = side(room_w) * side(room_h);
    let windows = (KEPT_OUT_LIMIT * (CODE_LEN * CODE_TRIES - CODE_LEN + 1)) as f64;
    let keyboard = windows / (LETTERS.len() as f64).powi(CODE_LEN as i32);
    (1..=TARGETS_MAX).find(|n| BESIDE_LIMIT as f64 * one.powi(*n as i32) <= keyboard)
}

/// What a press at a held device's sheet hit.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Hit {
    /// Not a target: a wrong try.
    Beside,
    /// The target drawn as number `step` of `of`.
    Target { step: u8, of: u8 },
}

/// How a device announced is taken.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Admitted {
    /// Already waiting: nothing changes.
    Known,
    /// A device past the limit: ignored without a code or a target.
    Ignored,
    /// Held back, waiting for its code or a decision.
    Held,
}

/// What a letter typed or a press made by a held-back device led to.
pub(crate) enum Tried {
    /// Nothing: no such device, no code to type yet, or not yet enough.
    Nothing,
    /// It typed its code or pressed its targets: it is to be believed.
    Matched,
    /// It typed too many letters without the code, or pressed beside the target too often, and
    /// is kept out; `touch` is whether it is a touchscreen, and `limit` whether the door is now
    /// closed.
    KeptOut {
        name: Text,
        touch: bool,
        limit: bool,
    },
}

/// What a lock carries to the next process for the session's limits on new devices: devices
/// kept out, presses beside a target, wrong letters.
#[derive(Public, Copy, PartialEq, Eq)]
pub struct Counts {
    pub kept_out: u8,
    pub beside: u8,
    pub wrong: u8,
}

impl faraday_kernel::CarryValue for Counts {
    fn encode(&self) -> faraday_port::bytes::Bytes {
        faraday_port::bytes::Bytes::new(vec![self.kept_out, self.beside, self.wrong])
    }

    /// A count past its limit is the limit, so a file that was edited cannot do more than close
    /// the door.
    fn decode(bytes: &[u8]) -> Option<Self> {
        let [a, b, c] = *bytes else {
            return None;
        };
        let clamp =
            |n: u8, limit: usize| u8::try_from(usize::from(n).min(limit)).unwrap_or(u8::MAX);
        Some(Counts {
            kept_out: clamp(a, KEPT_OUT_LIMIT),
            beside: clamp(b, BESIDE_LIMIT),
            wrong: clamp(c, WRONG_LETTERS_LIMIT),
        })
    }
}

/// The input devices area: the devices held back, the ones ignored, how many were kept out for
/// the wrong code. Its fields are private; the code is drawn from the session's randomness.
#[derive(Default)]
pub struct Inputs {
    /// Devices held back, waiting for a code or a decision.
    held: Vec<NewInput>,
    /// Devices kept out until they are unplugged: the adapter's number and the name it gave.
    ignored: Vec<(u32, Text)>,
    /// How many keyboards and touchscreens were kept out this session.
    kept_out: usize,
    /// How many presses beside a target any device made this session.
    beside: usize,
    /// How many letters keyboards that did not type their code made and that were then kept out
    /// or unplugged this session.
    wrong: usize,
    /// Nothing trusted can point, as the adapter says: a pointer may approve itself.
    alone: bool,
    /// A trusted device types, as the adapter says.
    typist: bool,
    /// Decisions the adapter has not been told: believed (`true`) or kept out (`false`).
    decisions: Vec<(u32, bool)>,
}

/// Holds the codes and what was typed of them: gone after a lock.
impl AtLock for Inputs {
    fn holds(&self) -> Holds {
        Holds {
            kept: false,
            typed: self.held.iter().any(|d| d.code.is_some()),
            session: false,
            asks_first: None,
        }
    }

    fn hide(&mut self) {}

    fn at_lock(&mut self) {
        for d in &mut self.held {
            d.wipe();
        }
        self.held.clear();
        self.ignored.clear();
    }
}

impl Inputs {
    /// The held device whose sheet shows: the first, if it has acted. A device that stays
    /// silent is never it.
    pub(crate) fn shown(&self) -> Option<&NewInput> {
        self.held.first().filter(|d| d.used)
    }

    /// The devices kept out until they are unplugged.
    #[must_use]
    pub fn ignored(&self) -> &[(u32, Text)] {
        &self.ignored
    }

    /// Whether Enter on a trusted keyboard could let in a pointer: such a keyboard exists.
    #[must_use]
    pub fn typist(&self) -> bool {
        self.typist
    }

    /// Whether nothing trusted can point.
    #[must_use]
    pub fn alone(&self) -> bool {
        self.alone
    }

    /// Whether the device is let in by pressing targets rather than typing a code: a
    /// touchscreen with no keys of its own, or a pointer with none while no input is trusted.
    pub(crate) fn by_target(&self, d: &NewInput) -> bool {
        !d.keyboard && (d.touch || (d.pointer && self.alone))
    }

    /// The door is closed: too many devices kept out, or too many presses beside a target,
    /// this session.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.kept_out >= KEPT_OUT_LIMIT
            || self.beside >= BESIDE_LIMIT
            || self.wrong >= WRONG_LETTERS_LIMIT
    }

    /// The counts a lock carries to the next process.
    #[must_use]
    pub fn carry(&self) -> Counts {
        let n = |v: usize| u8::try_from(v).unwrap_or(u8::MAX);
        Counts {
            kept_out: n(self.kept_out),
            beside: n(self.beside),
            wrong: n(self.wrong),
        }
    }

    /// Takes the counts a lock carried (already clamped by `decode`).
    pub(crate) fn set_carry(&mut self, c: Counts) {
        self.kept_out = usize::from(c.kept_out).min(KEPT_OUT_LIMIT);
        self.beside = usize::from(c.beside).min(BESIDE_LIMIT);
        self.wrong = usize::from(c.wrong).min(WRONG_LETTERS_LIMIT);
    }

    /// The adapter says whether anything trusted points, and whether anything trusted types.
    /// With nothing that points, the pointers waiting are let in by targets, unless the door is
    /// closed: they are swept as the door's closing sweeps. Returns the ids kept out by the
    /// sweep and whether anything changed that the sheet shows.
    pub(crate) fn set_trust(&mut self, pointer: bool, keyboard: bool) -> (Vec<u32>, bool) {
        let was = (self.alone, self.typist);
        self.alone = !pointer;
        self.typist = keyboard;
        let mut swept = Vec::new();
        if self.alone && self.closed() {
            swept = self.sweep();
        }
        let changed = !swept.is_empty() || was != (self.alone, self.typist);
        (swept, changed)
    }

    /// The door has closed: every device waiting on a target is kept out. Their ids.
    fn sweep(&mut self) -> Vec<u32> {
        let ids: Vec<u32> = self
            .held
            .iter()
            .filter(|d| self.by_target(d))
            .map(|d| d.id)
            .collect();
        for id in &ids {
            self.ignore(*id);
        }
        ids
    }

    /// A device the adapter holds back. Past the limit a keyboard, touchscreen or self-approving
    /// pointer is ignored without a code or a target.
    pub(crate) fn admit(
        &mut self,
        id: u32,
        name: Text,
        (keyboard, pointer, touch): (bool, bool, bool),
    ) -> Admitted {
        if self.held.iter().any(|i| i.id == id) {
            return Admitted::Known;
        }
        if self.closed() && (keyboard || touch || (pointer && self.alone)) {
            self.decisions.push((id, false));
            self.ignored.push((id, name));
            return Admitted::Ignored;
        }
        self.held.push(NewInput {
            id,
            name,
            keyboard,
            pointer,
            touch,
            code: None,
            spots: None,
            reached: 0,
            missed: 0,
            typed: SecretText::new(),
            letters: 0,
            used: false,
        });
        Admitted::Held
    }

    /// The draw for device `id` arrived: its code and targets' places. Dropped when the device
    /// has gone or has them. Whether it took them.
    pub(crate) fn coded(&mut self, id: u32, bytes: &[u8]) -> bool {
        match self.held.iter_mut().find(|d| d.id == id && !d.drawn()) {
            Some(d) => d.take(bytes),
            None => false,
        }
    }

    /// Whether device `id` waits without a code.
    pub(crate) fn needs_code(&self, id: u32) -> bool {
        self.held.iter().any(|d| d.id == id && !d.drawn())
    }

    /// A press a held-back device made while its target was drawn: on the target it was aimed at
    /// (`hit`), or beside it. A press on a target that is no longer the one showing counts for
    /// nothing; one beside restarts the run.
    pub(crate) fn tapped(&mut self, id: u32, hit: Hit) -> Tried {
        let (alone, closed) = (self.alone, self.closed());
        let Some(d) = self.held.iter_mut().find(|d| d.id == id) else {
            return Tried::Nothing;
        };
        // With no target to press, a press is held back and counts for nothing.
        if d.keyboard || !(d.touch || (d.pointer && alone)) || d.spots.is_none() || closed {
            return Tried::Nothing;
        }
        let Hit::Target { step, of } = hit else {
            d.reached = 0;
            d.missed += 1;
            let (name, touch) = (d.name.clone(), d.touch);
            let mut out = d.missed >= TAP_TRIES;
            self.beside += 1;
            if out {
                self.ignore(id);
                self.kept_out += 1;
            }
            if self.closed() {
                self.sweep();
                out = true;
            }
            return if out {
                Tried::KeptOut {
                    name,
                    touch,
                    limit: self.closed(),
                }
            } else {
                Tried::Nothing
            };
        };
        if usize::from(step) != d.reached {
            return Tried::Nothing;
        }
        d.reached += 1;
        if d.reached >= usize::from(of) {
            Tried::Matched
        } else {
            Tried::Nothing
        }
    }

    /// A key a held-back keyboard typed.
    pub(crate) fn typed(&mut self, id: u32, ch: char) -> Tried {
        let Some(i) = self.held.iter().position(|d| d.id == id) else {
            return Tried::Nothing;
        };
        let in_hand = self.letters_in_hand();
        let Some(d) = self.held.get_mut(i) else {
            return Tried::Nothing;
        };
        // With no code to type, typing is held back and counts for nothing.
        if d.code.is_none() {
            return Tried::Nothing;
        }
        d.typed.push(ch.to_ascii_lowercase());
        d.letters += 1;
        let keep = d.typed.as_str().chars().count().saturating_sub(CODE_LEN);
        if keep > 0 {
            // Rebuilt in a wiping text; the old one wipes as it drops.
            let mut tail = SecretText::new();
            d.typed
                .as_str()
                .chars()
                .skip(keep)
                .for_each(|c| tail.push(c));
            d.typed = tail;
        }
        if d.code() == Some(d.typed.as_str()) {
            Tried::Matched
        } else if d.letters >= CODE_LEN * CODE_TRIES || in_hand + 1 >= WRONG_LETTERS_LIMIT {
            let (name, touch) = (d.name.clone(), d.touch);
            self.ignore(id);
            self.kept_out += 1;
            if self.closed() {
                self.sweep();
            }
            Tried::KeptOut {
                name,
                touch,
                limit: self.closed(),
            }
        } else {
            Tried::Nothing
        }
    }

    /// The device acted: it moves ahead of every held device not yet used and behind those
    /// already used, so the first held device is the one that asked first. Whether anything
    /// changed.
    pub(crate) fn used(&mut self, id: u32) -> bool {
        let Some(i) = self.held.iter().position(|d| d.id == id) else {
            return false;
        };
        if self.held.get(i).is_none_or(|d| d.used) {
            return false;
        }
        let mut d = self.held.remove(i);
        d.used = true;
        let at = self.held.iter().take_while(|d| d.used).count();
        self.held.insert(at, d);
        true
    }

    /// Whether the held device `id` has acted.
    pub(crate) fn is_used(&self, id: u32) -> bool {
        self.held.iter().any(|d| d.id == id && d.used)
    }

    /// The device is believed from now on; it, if it was waiting. What it does is trusted from
    /// here on, whatever the adapter last said.
    pub(crate) fn believe(&mut self, id: u32) -> Option<NewInput> {
        let i = self.held.iter().position(|d| d.id == id)?;
        let d = self.held.remove(i);
        self.decisions.push((id, true));
        if d.pointer || d.touch {
            self.alone = false;
        }
        if d.keyboard {
            self.typist = true;
        }
        Some(d)
    }

    /// The wrong letters so far: those counted and those the held keyboards have typed.
    fn letters_in_hand(&self) -> usize {
        self.wrong + self.held.iter().map(|d| d.letters).sum::<usize>()
    }

    /// The device is kept out until it is unplugged. Whether it was waiting.
    pub(crate) fn ignore(&mut self, id: u32) -> bool {
        let Some(i) = self.held.iter().position(|d| d.id == id) else {
            return false;
        };
        let mut d = self.held.remove(i);
        self.wrong += d.letters;
        d.wipe();
        self.decisions.push((id, false));
        self.ignored.push((id, d.name));
        true
    }

    /// The decisions the adapter has not been told, in order.
    pub(crate) fn decisions(&mut self) -> Vec<(u32, bool)> {
        std::mem::take(&mut self.decisions)
    }

    /// The device was unplugged.
    pub(crate) fn gone(&mut self, id: u32) {
        if let Some(d) = self.held.iter().find(|d| d.id == id) {
            self.wrong += d.letters;
        }
        self.held.retain(|d| d.id != id);
        self.ignored.retain(|(i, _)| *i != id);
    }
}
