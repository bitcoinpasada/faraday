//! A held device asks when it is used, not when it is plugged in (F202):
//! at plug-in a notice names it and nothing blocks; its first motion,
//! press, touch or key brings up its sheet, and that press or touch counts
//! for nothing, as nothing was drawn to aim at, while a letter counts as
//! one. One sheet shows at a time; a device that acts while another's is
//! up waits its turn in the order they acted, its proof and counts
//! unchanged. A device that never acts never asks, stays held and is never
//! believed, so nothing waits for the held list to be empty.
//!
//! New input devices must show a person is at the screen before they are
//! believed (`docs/SECURITY-MODEL.md`, layer 3).
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
//! [`KEPT_OUT_LIMIT`] as a keyboard does (F153). A pointer on a machine
//! where no input is trusted (`alone`, which the shell says) is let in the
//! same way, since it is the only thing that can; and a touchscreen and
//! such a pointer press several targets in a row, as many as make a blind
//! device's chance no better than a keyboard's ([`target_count`]), and
//! every press beside a target counts for the session, whatever happens to
//! the device, until [`BESIDE_LIMIT`] closes the door (F202). Nothing is
//! believed for the name it gives. **Ignore this device** keeps it out until it is
//! unplugged; ignored devices are listed in Settings. The code and the
//! target's place are drawn from the session's seed, which is the
//! system's randomness: a device that arrives before the seed waits
//! without them, held back, and gets them when the seed arrives.

use crate::strings::EN;
use crate::strings::fill;
use crate::{Action, Faraday, Sheet, StorageEvent};
use faraday_draw::tokens::TOUCH;
use std::collections::VecDeque;

/// How long the code is.
pub const CODE_LEN: usize = 6;

/// How many codes' worth of letters a keyboard may type before it is
/// kept out: a person who mistypes has room to type it again.
pub const CODE_TRIES: usize = 3;

/// How many keyboards may be kept out for the wrong code in a session
/// before new keyboards are ignored without a code.
pub(crate) const KEPT_OUT_LIMIT: usize = 3;

/// How many taps beside the target a touchscreen may make before it is
/// kept out: a person whose finger slips has room to tap again.
pub(crate) const TAP_TRIES: usize = 3;

/// How many presses beside any target, from any device, close the door for
/// the session: each of [`KEPT_OUT_LIMIT`] devices' [`TAP_TRIES`].
pub(crate) const BESIDE_LIMIT: usize = KEPT_OUT_LIMIT * TAP_TRIES;

/// The most targets in a row a sheet draws; a screen needing more shows
/// none.
pub(crate) const TARGETS_MAX: usize = 16;

/// How long a pointer's targets must have been drawn, with no key pressed
/// on the trusted keyboard, before Enter lets the pointer in, in ms: an
/// Enter a person meant for what they were typing lets nothing in (F202).
pub const ENTER_QUIET_MS: u64 = 1000;

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
    /// Where each target it must press is, as fractions of the room the
    /// sheet leaves for it (0 to `u16::MAX` across and down), each drawn
    /// apart; `None` until the session's seed arrived.
    pub spots: Option<[(u16, u16); TARGETS_MAX]>,
    /// The target showing: how many it has pressed in this run.
    pub reached: usize,
    /// How many times it pressed beside a target.
    pub missed: usize,
    /// What it has typed so far, the last few letters.
    pub typed: String,
    /// How many letters it has typed in all.
    pub letters: usize,
    /// It has acted (moved, pressed, typed or touched) since it was held:
    /// only then does its sheet show (F202).
    pub used: bool,
}

/// How many targets in a row a sheet draws, from the room it leaves for
/// one (`room_w` and `room_h`, which the target's place is drawn in): the
/// fewest that make a blind device's chance in a session no better than a
/// keyboard's. One press lands with the share `TOUCH` is of the room on
/// each side (all of it where there is none); a session has at most
/// [`BESIDE_LIMIT`] runs and a keyboard's code is guessed in its windows
/// of the letters' arrangements. `None` when no count up to
/// [`TARGETS_MAX`] does (F202). Arithmetic.
pub(crate) fn target_count(room_w: f32, room_h: f32) -> Option<usize> {
    let side = |room: f32| {
        if room <= 0.0 {
            1.0
        } else {
            f64::from((TOUCH / room).min(1.0))
        }
    };
    let one = side(room_w) * side(room_h);
    let windows = (KEPT_OUT_LIMIT * (CODE_LEN * CODE_TRIES - CODE_LEN + 1)) as f64;
    let keyboard = windows / (LETTERS.len() as f64).powi(CODE_LEN as i32);
    (1..=TARGETS_MAX).find(|n| BESIDE_LIMIT as f64 * one.powi(*n as i32) <= keyboard)
}

/// What a press at a held device's sheet hit.
pub(crate) enum Hit {
    /// Not a target: a wrong try.
    Beside,
    /// The target drawn as number `step` of `of`.
    Target { step: u8, of: u8 },
}

/// How a device announced is taken.
pub(crate) enum Admitted {
    /// Already waiting: nothing changes.
    Known,
    /// A device past the limit: ignored without a code or a target.
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
    /// It typed too many letters without it, or pressed beside the
    /// target too often, and is kept out; `touch` is whether it is a
    /// touchscreen, and `limit` whether the door is now closed.
    KeptOut {
        name: String,
        touch: bool,
        limit: bool,
    },
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
    /// How many presses beside a target any device made this session:
    /// unplugging and Ignore do not clear it.
    beside: usize,
    /// Nothing trusted can point, as the shell says: a pointer may approve
    /// itself. False until it says so.
    alone: bool,
    /// A trusted device types, as the shell says: Enter on it lets in the
    /// pointer whose targets show.
    typist: bool,
    /// The pointer whose Enter line was drawn, and when it was drawn or a
    /// key last pressed since, whichever is later.
    enter_quiet: Option<(u32, u64)>,
    /// Decisions the shell has not been told: believed (`true`) or kept
    /// out (`false`).
    decisions: VecDeque<(u32, bool)>,
    /// What was plugged in at start, and when the first frame drew it.
    boot_notice: Option<(String, Option<u64>)>,
    /// The lock notice waits under a held device's sheet, and comes back
    /// when the device is decided.
    locked_under: bool,
}

/// How long the list of what was plugged in at start shows, from the
/// first frame that draws it, in milliseconds, unless its × or Escape
/// takes it away first.
pub(crate) const BOOT_NOTICE_MS: u64 = 10_000;

impl InputsArea {
    /// The list of what was plugged in at start, while it shows.
    pub(crate) fn boot_notice(&self) -> Option<&str> {
        self.boot_notice.as_ref().map(|(t, _)| t.as_str())
    }

    /// The lock notice is set aside for a held device's sheet, or back.
    pub(crate) fn set_locked_under(&mut self, on: bool) {
        self.locked_under = on;
    }

    /// Whether the lock notice waits under a held device's sheet; clears
    /// it.
    pub(crate) fn take_locked_under(&mut self) -> bool {
        std::mem::take(&mut self.locked_under)
    }

    /// The devices held back.
    pub(crate) fn held(&self) -> &[NewInput] {
        &self.held
    }

    /// The devices kept out until unplugged.
    pub(crate) fn ignored(&self) -> &[(u32, String)] {
        &self.ignored
    }

    /// Whether the device is let in by pressing targets rather than
    /// typing a code: a touchscreen with no keys of its own, or a pointer
    /// with none while no input is trusted.
    pub(crate) fn by_target(&self, d: &NewInput) -> bool {
        !d.keyboard && (d.touch || (d.pointer && self.alone))
    }

    /// The shell says whether anything trusted points, and whether
    /// anything trusted types. With nothing that points, the pointers
    /// waiting are let in by targets, unless the door is closed: they are
    /// swept as the door's closing sweeps. Whether anything changed that
    /// the sheet shows.
    pub(crate) fn set_trust(&mut self, pointer: bool, keyboard: bool) -> bool {
        let was = (self.alone, self.typist);
        self.alone = !pointer;
        self.typist = keyboard;
        if self.alone && self.closed() {
            let before = self.held.len();
            self.sweep();
            if self.held.len() != before {
                return true;
            }
        }
        was != (self.alone, self.typist)
    }

    /// Whether nothing trusted can point.
    pub(crate) fn alone(&self) -> bool {
        self.alone
    }

    /// Whether a trusted device types.
    pub(crate) fn typist(&self) -> bool {
        self.typist
    }

    /// The Enter line of pointer `id` was drawn at `now`: its wait starts,
    /// unless it already has.
    fn enter_drawn(&mut self, id: u32, now: u64) {
        if !self.enter_quiet.is_some_and(|(at, _)| at == id) {
            self.enter_quiet = Some((id, now));
        }
    }

    /// A key was pressed at `now` while pointer `id` waits: whether its
    /// line had been up and the keyboard quiet long enough, and the wait
    /// starts again.
    fn enter_key(&mut self, id: u32, now: u64) -> bool {
        match self.enter_quiet.as_mut() {
            Some((at, since)) if *at == id => {
                let quiet = now.saturating_sub(*since) >= ENTER_QUIET_MS;
                *since = now;
                quiet
            }
            _ => false,
        }
    }

    /// The door is closed: too many devices kept out, or too many presses
    /// beside a target, this session.
    fn closed(&self) -> bool {
        self.kept_out >= KEPT_OUT_LIMIT || self.beside >= BESIDE_LIMIT
    }

    /// The door has closed: every device waiting on a target is kept out.
    fn sweep(&mut self) {
        let ids: Vec<u32> = self
            .held
            .iter()
            .filter(|d| self.by_target(d))
            .map(|d| d.id)
            .collect();
        for id in ids {
            self.ignore(id);
        }
    }

    /// A device the shell holds back; `draw` draws its code and its
    /// targets' places, and is called only when it is held.
    pub(crate) fn admit(
        &mut self,
        id: u32,
        name: String,
        (keyboard, pointer, touch): (bool, bool, bool),
        draw: impl FnOnce() -> Option<(String, [(u16, u16); TARGETS_MAX])>,
    ) -> Admitted {
        if self.held.iter().any(|i| i.id == id) {
            return Admitted::Known;
        }
        if self.closed() && (keyboard || touch || (pointer && self.alone)) {
            self.decisions.push_back((id, false));
            self.ignored.push((id, name));
            return Admitted::Ignored;
        }
        let (code, spots) = draw().unzip();
        self.held.push(NewInput {
            id,
            name,
            keyboard,
            pointer,
            touch,
            code,
            spots,
            reached: 0,
            missed: 0,
            typed: String::new(),
            letters: 0,
            used: false,
        });
        Admitted::Held
    }

    /// The seed arrived: each device waiting without a code gets one,
    /// and its targets' places, from `draw`.
    pub(crate) fn coded(
        &mut self,
        draw: impl Fn(u32) -> Option<(String, [(u16, u16); TARGETS_MAX])>,
    ) {
        for d in &mut self.held {
            if d.code.is_none() {
                (d.code, d.spots) = draw(d.id).unzip();
            }
        }
    }

    /// A press a held-back device made while its target was drawn: on
    /// the target it was aimed at (`hit`), or beside it. A press on a
    /// target that is no longer the one showing counts for nothing, as the
    /// hit is the last frame's; one beside restarts the run.
    pub(crate) fn tapped(&mut self, id: u32, hit: Hit) -> Typed {
        let (alone, closed) = (self.alone, self.closed());
        let Some(d) = self.held.iter_mut().find(|d| d.id == id) else {
            return Typed::Nothing;
        };
        // With no target to press, a press is held back and counts for
        // nothing.
        if d.keyboard || !(d.touch || (d.pointer && alone)) || d.spots.is_none() {
            return Typed::Nothing;
        }
        if closed {
            return Typed::Nothing;
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
                Typed::KeptOut {
                    name,
                    touch,
                    limit: self.closed(),
                }
            } else {
                Typed::Nothing
            };
        };
        if usize::from(step) != d.reached {
            return Typed::Nothing;
        }
        d.reached += 1;
        if d.reached >= usize::from(of) {
            Typed::Matched
        } else {
            Typed::Nothing
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
            let (name, touch) = (d.name.clone(), d.touch);
            self.ignore(id);
            self.kept_out += 1;
            if self.closed() {
                self.sweep();
            }
            Typed::KeptOut {
                name,
                touch,
                limit: self.closed(),
            }
        } else {
            Typed::Nothing
        }
    }

    /// The device acted: it moves ahead of every held device not yet used
    /// and behind those already used, so the first held device is the one
    /// that asked first. Whether anything changed.
    pub(crate) fn used(&mut self, id: u32) -> bool {
        let Some(i) = self.held.iter().position(|d| d.id == id) else {
            return false;
        };
        if self.held[i].used {
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

    /// The device is believed from now on; it, if it was waiting. What it
    /// does is trusted from here on, whatever the shell last said.
    pub(crate) fn believe(&mut self, id: u32) -> Option<NewInput> {
        let i = self.held.iter().position(|d| d.id == id)?;
        let d = self.held.remove(i);
        self.decisions.push_back((id, true));
        if d.pointer || d.touch {
            self.alone = false;
        }
        if d.keyboard {
            self.typist = true;
        }
        Some(d)
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

    /// The held device whose sheet shows: the first, if it has acted
    /// (F202). A device that stays silent is never it.
    pub fn shown_input(&self) -> Option<&NewInput> {
        self.inputs.held().first().filter(|d| d.used)
    }

    /// Whether the held device `id` has acted; a device already used
    /// needs no telling again.
    pub fn input_is_used(&self, id: u32) -> bool {
        self.inputs.is_used(id)
    }

    /// The devices kept out until they are unplugged: the shell's number
    /// and the name each gave.
    pub fn ignored_inputs(&self) -> &[(u32, String)] {
        self.inputs.ignored()
    }

    /// Whether the device is let in by pressing targets (F202).
    pub(crate) fn input_by_target(&self, d: &NewInput) -> bool {
        self.inputs.by_target(d)
    }

    /// Whether Enter on a trusted keyboard lets in the pointer whose
    /// targets show: there is one, and such a keyboard exists.
    pub(crate) fn input_enter_lets_in(&self) -> bool {
        self.inputs.typist()
            && self.sheet == Some(Sheet::NewInput)
            && self
                .shown_input()
                .is_some_and(|d| d.pointer && !d.touch && self.inputs.by_target(d))
    }

    /// A trusted keyboard's key while a device's sheet is up stays on the
    /// sheet: it moves through the sheet's controls, presses one or scrolls,
    /// and never reaches the page beneath. Escape does nothing, since the
    /// sheet stays until the device is decided. Whether the sheet was up.
    pub(crate) fn input_sheet_key(&mut self, key: faraday_shell_api::Key) -> bool {
        use faraday_shell_api::Key;
        if self.sheet != Some(Sheet::NewInput) {
            return false;
        }
        if self.input_enter(key) {
            return true;
        }
        match key {
            Key::Down => self.glide(crate::motion::WHEEL_NOTCH),
            Key::Up => self.glide(-crate::motion::WHEEL_NOTCH),
            _ => self
                .pointer
                .focus_key(key)
                .into_iter()
                .for_each(|a| self.press_in(a)),
        }
        true
    }

    /// Enter from a trusted keyboard lets in the pointer whose targets show,
    /// as **Use this pointer** does, once its line has been drawn and the
    /// keyboard quiet for [`ENTER_QUIET_MS`]. An earlier Enter does nothing,
    /// and a control the keyboard is on keeps its Enter. Whether the key was
    /// taken.
    fn input_enter(&mut self, key: faraday_shell_api::Key) -> bool {
        if !self.input_enter_lets_in() {
            return false;
        }
        let Some(id) = self.shown_input().map(|d| d.id) else {
            return false;
        };
        let quiet = self.inputs.enter_key(id, self.now_ms);
        if key != faraday_shell_api::Key::Enter || self.pointer.focused().is_some() {
            return false;
        }
        if quiet {
            self.input_believe(id);
        }
        true
    }

    /// What a frame drew: the wait for Enter starts with the first frame
    /// showing pointer `line`'s Enter line, and ends with the sheet. Cheap.
    pub(crate) fn input_drawn(&mut self, line: Option<u32>) {
        self.boot_notice_drawn();
        if self.sheet != Some(Sheet::NewInput) {
            self.inputs.enter_quiet = None;
        } else if let Some(id) = line {
            self.inputs.enter_drawn(id, self.now_ms);
        }
    }

    /// Whether the held pointer `id` is the one whose targets show, so the
    /// shell moves its cursor and arrow for it: the sheet is the new
    /// input's, it is first, and it is let in by pressing. True before the
    /// seed arrived and where no count fits, so the arrow reaches Ignore.
    /// Never for a touchscreen.
    pub fn input_aims(&self, id: u32) -> bool {
        self.sheet == Some(Sheet::NewInput)
            && self
                .shown_input()
                .is_some_and(|d| d.id == id && d.pointer && !d.touch && self.inputs.by_target(d))
    }

    /// What was plugged in when the shell's start window closed: one toast
    /// lists it, which only its × or Escape dismisses and nothing replaces.
    /// It comes once.
    fn input_boot_usb(&mut self, names: &[String]) {
        if names.is_empty() || self.inputs.boot_notice.is_some() {
            return;
        }
        let named: Vec<&str> = names
            .iter()
            .map(|n| {
                if n.is_empty() {
                    EN.inputs_usb_device_unnamed
                } else {
                    n
                }
            })
            .collect();
        let text = fill(EN.inputs_plugged_in_at_start, &[&named.join(", ")]);
        self.inputs.boot_notice = Some((text, None));
        self.out.redraw();
    }

    /// The list of what was plugged in at start, while it shows.
    pub(crate) fn boot_notice_text(&self) -> Option<&str> {
        self.inputs.boot_notice()
    }

    /// Notes the first frame that drew the list: its time runs from there.
    fn boot_notice_drawn(&mut self) {
        if let Some((_, at @ None)) = self.inputs.boot_notice.as_mut() {
            *at = Some(self.now_ms);
        }
    }

    /// The × on the list (F218), once a frame has drawn it: it goes for
    /// this power-on. A press before that counts nothing. The press is a
    /// person's: a held-back device's presses and keys arrive as
    /// `StorageEvent`s (`input_typed`, `input_tapped`) and reach neither
    /// this nor [`Faraday::input_key`].
    pub(crate) fn boot_notice_close(&mut self) {
        if matches!(self.inputs.boot_notice, Some((_, Some(_)))) {
            self.inputs.boot_notice = None;
        }
    }

    /// Escape, before the screens see a key: it takes a tooltip off first,
    /// then the list while it shows (drawn), and does nothing else.
    /// Whether the key was taken.
    pub(crate) fn input_key(&mut self, key: faraday_shell_api::Key) -> bool {
        if key != faraday_shell_api::Key::Escape {
            return false;
        }
        if self.pointer.tip_dismiss() {
            return true;
        }
        let shows = matches!(self.inputs.boot_notice, Some((_, Some(_))));
        if shows {
            self.inputs.boot_notice = None;
        }
        shows
    }

    /// Takes the list away once its time is up. Whether it went.
    pub(crate) fn boot_notice_tick(&mut self, now_ms: u64) -> bool {
        let over = self
            .inputs
            .boot_notice
            .as_ref()
            .is_some_and(|(_, at)| at.is_some_and(|at| now_ms >= at + BOOT_NOTICE_MS));
        if over {
            self.inputs.boot_notice = None;
        }
        over
    }

    /// What the shell said of a held-back device: one announced, a key it
    /// typed, a press it made, one unplugged, or whether input is trusted.
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
            StorageEvent::InputUsed { id } => self.input_used(id),
            StorageEvent::InputGone { id } => self.input_gone(id),
            StorageEvent::InputTrust { pointer, keyboard }
                if self.inputs.set_trust(pointer, keyboard) =>
            {
                self.input_sheet();
            }
            StorageEvent::InputTrust { .. } => {}
            StorageEvent::BootUsb { names } => self.input_boot_usb(&names),
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
                self.toast_ignored();
                return;
            }
            Admitted::Held => {}
        }
        // A notice only: the sheet comes when the device is used.
        if let Some(d) = self.inputs.held().iter().find(|d| d.id == id) {
            let text = fill(EN.inputs_new_device_held, &[&d.name]);
            self.toast(&text);
        }
    }

    /// A held device acted. The first to act takes the screen; the boot
    /// import waits under its sheet (`docs/SECURITY-MODEL.md`); one
    /// that acts while a sheet is up waits its turn.
    pub(crate) fn input_used(&mut self, id: u32) {
        if !self.inputs.used(id) {
            return;
        }
        if self.sheet == Some(Sheet::Import) {
            self.import.set_under_input(true);
            self.sheet = None;
        } else if self.sheet == Some(Sheet::Devices) {
            self.devices.over_inputs = true;
            self.sheet = None;
        } else if self.sheet == Some(Sheet::Locked) {
            // The lock notice has no way to be answered by a held device:
            // the device's sheet takes the screen and the notice returns.
            self.inputs.set_locked_under(true);
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
        self.input_used(id);
        let typed = self.inputs.typed(id, ch);
        self.input_tried(id, typed, [EN.inputs_wrong_code; 2]);
    }

    /// A press a held-back device made at (x, y), in pixels. It counts
    /// only while the sheet shows that device's target: on the target it
    /// moves it on a step, on the sheet's Ignore it keeps the device out
    /// and counts nothing, and anywhere else it is a press beside the
    /// target. A lookup.
    pub(crate) fn input_tapped(&mut self, id: u32, x: u16, y: u16) {
        // A device's first touch or press only brings up its sheet: it
        // could not aim at what was not drawn.
        if self.inputs.held().iter().any(|d| d.id == id && !d.used) {
            self.input_used(id);
            return;
        }
        let showing =
            self.sheet == Some(Sheet::NewInput) && self.shown_input().is_some_and(|d| d.id == id);
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
        let hit = match hit {
            Some(Action::InputIgnore(i)) if i == id => {
                self.input_ignore(id);
                return;
            }
            Some(Action::InputTarget(i, step, of)) if i == id => Hit::Target { step, of },
            _ => Hit::Beside,
        };
        // A press beside counts only when the last frame offered a target
        // of this device's to aim at.
        let drawn = self
            .pointer
            .hits()
            .iter()
            .any(|(_, a)| matches!(a, Action::InputTarget(i, ..) if *i == id));
        if matches!(hit, Hit::Beside) && !drawn {
            return;
        }
        let tapped = self.inputs.tapped(id, hit);
        self.input_tried(id, tapped, [EN.inputs_wrong_presses, EN.inputs_wrong_taps]);
    }

    /// What a held-back device's try led to; `wrong` is the toast for one
    /// kept out for it, for a pointer or keyboard and for a touchscreen.
    fn input_tried(&mut self, id: u32, tried: Typed, wrong: [&str; 2]) {
        match tried {
            Typed::Nothing => {}
            Typed::Matched => self.input_believe(id),
            Typed::KeptOut { name, touch, limit } => {
                self.input_sheet();
                if limit {
                    self.toast_ignored();
                } else {
                    self.toast(&fill(wrong[usize::from(touch)], &[&name]));
                }
            }
        }
    }

    /// The door is closed: new devices of the kinds the session no longer
    /// asks about are ignored.
    fn toast_ignored(&mut self) {
        let text = if self.inputs.alone() {
            EN.inputs_new_devices_ignored
        } else {
            EN.inputs_new_keyboards_ignored
        };
        self.toast(text);
    }

    /// The device is believed from now on, and the toast says what kind.
    pub(crate) fn input_believe(&mut self, id: u32) {
        let Some(d) = self.inputs.believe(id) else {
            return;
        };
        let text = match (d.keyboard, d.pointer, d.touch) {
            (_, _, true) => EN.inputs_touchscreen_in_use,
            (true, true, _) => EN.inputs_keyboard_pointer_in_use,
            (true, false, _) => EN.inputs_keyboard_in_use,
            _ => EN.inputs_pointer_in_use,
        };
        self.toast(&fill(text, &[&d.name]));
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

    /// The sheet shows while a device has acted and waits, and goes when
    /// none has, back to the boot import's sheet if that waited under it.
    /// A device that stays silent keeps nothing up.
    pub(crate) fn input_sheet(&mut self) {
        if self.shown_input().is_none() {
            if self.sheet == Some(Sheet::NewInput) {
                self.sheet = None;
                if self.devices.take_over_inputs() {
                    // An import that came under the new device's sheet
                    // comes back after this one.
                    let import = self.import.take_under_input();
                    self.devices.over_import |= import && self.import.state().is_some();
                    self.sheet = Some(Sheet::Devices);
                } else {
                    let import = self.import.take_under_input();
                    self.sheet = (import && self.import.state().is_some()).then_some(Sheet::Import);
                }
                if self.inputs.take_locked_under() && self.sheet.is_none() {
                    self.sheet = Some(Sheet::Locked);
                }
                // A device that came due while the sheet was up.
                self.device_sheet();
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

/// A code and each target's place from the session's seed, one set per
/// device; `None` before the seed arrived. The code is the draw's first
/// bytes; each place is its own draw, labelled and counted apart.
fn input_code(
    fresh: &crate::fresh::Fresh,
    id: u32,
    now_ms: u64,
) -> Option<(String, [(u16, u16); TARGETS_MAX])> {
    let count = [id.to_le_bytes().as_slice(), &now_ms.to_le_bytes()].concat();
    let draw = fresh.draw(b"faraday input code", &count)?;
    let code = draw
        .iter()
        .take(CODE_LEN)
        .map(|b| char::from(LETTERS[usize::from(*b) % LETTERS.len()]))
        .collect();
    let mut spots = [(0, 0); TARGETS_MAX];
    for (i, spot) in spots.iter_mut().enumerate() {
        let n = [count.as_slice(), &[i as u8]].concat();
        let d = fresh.draw(b"faraday input target", &n)?;
        *spot = (
            u16::from_le_bytes([d[0], d[1]]),
            u16::from_le_bytes([d[2], d[3]]),
        );
    }
    Some((code, spots))
}
