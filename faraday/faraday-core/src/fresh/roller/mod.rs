//! The dice roller of Faraday Desktop (`docs/CODEX32.md` §5.5, §5.6): a
//! die a click rolls for a person, its values this computer's randomness
//! from `Faraday::fresh`, landing on the entries of the flow that hosts
//! it (New key's dice pad, Create a vault's Generate with dice) as a
//! press of the pad would.
//!
//! A host holds one [`Roller`], draws it with [`screen::draw`], gives its
//! [`Act`]s to [`Roller::act`], and calls [`Roller::tick`] on every tick
//! with its own entries' length and the room left in them. The roller
//! never touches the entries: it hands back the values (1 to 6) and the
//! host adds them as keys. It remembers which positions it made
//! ([`Roller::sim`]), so the result can say "simulated dice". Hosts call
//! [`Roller::trim`] whenever their entries get shorter, and
//! [`Roller::reset`] when they start over.
//!
//! The roller is offered only where `Faraday::roller_offered` says so:
//! Faraday Desktop, never the stick, the Pi or the website.

use faraday_draw::tokens::{
    ROLLER_COUNT_DIGITS, ROLLER_COUNT_MAX, ROLLER_FACE_MS, ROLLER_PAUSE_MS, ROLLER_TUMBLE_MS,
};
use zeroize::Zeroizing;

pub mod five;
pub mod five_screen;
pub(crate) mod screen;

#[cfg(test)]
mod five_tests;

/// How many values one click of Roll makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PerClick {
    /// One (the default).
    #[default]
    One,
    /// Five.
    Five,
    /// Ten.
    Ten,
    /// Until the entries are full.
    ToEnd,
    /// The number typed.
    Typed,
}

/// A press on the roller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    /// Roll as many values as one click makes.
    Roll,
    /// Stop the click's rolls where they are.
    Stop,
    /// Choose how many one click makes.
    Per(PerClick),
    /// Choose the typed number, and type it.
    Count,
}

/// What the die is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Stage {
    /// Nothing to roll.
    #[default]
    Idle,
    /// Tumbling, since this time.
    Tumble(u64),
    /// A value landed at this time; the next tumble waits.
    Pause(u64),
}

/// Whether a result's values came from the roller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sim {
    /// Every value was typed or pressed.
    No,
    /// Some were rolled by this computer, some typed.
    Some,
    /// All were rolled by this computer.
    All,
}

/// The roller's state. Holds no entry: only the die, the choice and which
/// positions it made.
#[derive(Default)]
pub struct Roller {
    per: PerClick,
    /// The typed number's digits.
    typed: String,
    /// Values still to land from this click.
    left: usize,
    stage: Stage,
    /// The face of the last value landed, 1 to 6, 0 before any.
    face: u8,
    /// The face a tumbling die shows now.
    shown: u8,
    /// When `shown` changes next, ms.
    face_at: u64,
    /// Where in a tumble the die is, 0 to 1.
    progress: f32,
    /// Cycles the faces of a tumble; no part of any value.
    spin: u32,
    /// Positions in the host's entries that this computer rolled, rising.
    marks: Vec<usize>,
    /// The answers being used up.
    pool: Pool,
}

/// An answer being used up, one value to a byte.
#[derive(Default)]
struct Pool {
    bytes: Zeroizing<[u8; 32]>,
    /// How many bytes of `bytes` are not used yet.
    n: usize,
}

impl Pool {
    /// Erases what is left.
    fn clear(&mut self) {
        self.bytes.fill(0);
        self.n = 0;
    }

    /// The next value, 1 to 6, from `draw`'s answers; every byte of 252
    /// or more is skipped so that no face is likelier. `None` while
    /// `draw` has no answer.
    fn next(&mut self, draw: &mut dyn FnMut() -> Option<[u8; 32]>) -> Option<u8> {
        loop {
            if self.n == 0 {
                let mut bytes = Zeroizing::new(draw()?);
                self.bytes.copy_from_slice(&bytes[..]);
                self.n = self.bytes.len();
                bytes.fill(0);
            }
            self.n -= 1;
            let b = std::mem::take(&mut self.bytes[self.n]);
            if b < 252 {
                return Some(b % 6 + 1);
            }
        }
    }
}

impl Roller {
    /// How many values one click makes now (a lookup).
    pub fn count(&self) -> usize {
        match self.per {
            PerClick::One => 1,
            PerClick::Five => 5,
            PerClick::Ten => 10,
            PerClick::ToEnd => usize::MAX,
            PerClick::Typed => {
                self.typed
                    .parse::<u32>()
                    .map_or(1, |n| n.clamp(1, ROLLER_COUNT_MAX)) as usize
            }
        }
    }

    /// The choice.
    pub fn per(&self) -> PerClick {
        self.per
    }

    /// The typed number as typed.
    pub fn typed(&self) -> &str {
        &self.typed
    }

    /// Whether a click's rolls are under way.
    pub fn rolling(&self) -> bool {
        self.left > 0
    }

    /// Whether the roller has anything to tick: rolls under way, or the
    /// pause after the last value (a lookup).
    pub fn active(&self) -> bool {
        self.left > 0 || matches!(self.stage, Stage::Pause(_))
    }

    /// The face the die shows: the tumbling one, else the last landed
    /// (0: none yet).
    pub fn shown(&self) -> u8 {
        if matches!(self.stage, Stage::Tumble(_)) {
            self.shown
        } else {
            self.face
        }
    }

    /// Where in its tumble the die is, 0 to 1; 0 at rest.
    pub fn progress(&self) -> f32 {
        if matches!(self.stage, Stage::Tumble(_)) {
            self.progress
        } else {
            0.0
        }
    }

    /// A press. `room` is how many more values the host's entries take.
    pub fn act(&mut self, act: Act, room: usize, now_ms: u64) {
        match act {
            Act::Roll => {
                if room == 0 {
                    return;
                }
                self.left = self.count().min(room);
                self.stage = Stage::Tumble(now_ms);
                self.progress = 0.0;
                self.face_at = now_ms;
            }
            Act::Stop => self.stop(),
            Act::Per(p) => self.per = p,
            Act::Count => self.per = PerClick::Typed,
        }
    }

    /// A digit typed into the number's field.
    pub fn digit(&mut self, c: char) {
        if c.is_ascii_digit() && self.typed.len() < ROLLER_COUNT_DIGITS {
            self.typed.push(c);
        }
    }

    /// Backspace in the number's field.
    pub fn backspace(&mut self) {
        self.typed.pop();
    }

    /// Ends a click's rolls.
    pub fn stop(&mut self) {
        self.left = 0;
        self.stage = Stage::Idle;
    }

    /// Forgets which positions it made and ends any rolls: the host's
    /// entries started over.
    pub fn reset(&mut self) {
        self.stop();
        self.marks.clear();
        self.face = 0;
        self.pool.clear();
    }

    /// The host's entries are now `len` long: positions past it are not
    /// the roller's any more.
    pub fn trim(&mut self, len: usize) {
        while self.marks.last().is_some_and(|&m| m >= len) {
            self.marks.pop();
        }
    }

    /// Whether the value at `pos` was rolled by this computer.
    pub fn is_rolled(&self, pos: usize) -> bool {
        self.marks.binary_search(&pos).is_ok()
    }

    /// Whether the `len` entries came from the roller, in part or all.
    pub fn sim(&self, len: usize) -> Sim {
        let n = self.marks.iter().filter(|&&m| m < len).count();
        if n == 0 {
            Sim::No
        } else if n == len {
            Sim::All
        } else {
            Sim::Some
        }
    }

    /// The next value, 1 to 6 (see [`Pool::next`]).
    fn next(&mut self, draw: &mut dyn FnMut() -> Option<[u8; 32]>) -> Option<u8> {
        self.pool.next(draw)
    }

    /// One tick. `len` is how many entries the host has, `room` how many
    /// more it takes, `reduce` whether Reduce motion is on, and `draw`
    /// asks for 32 fresh bytes (`Faraday::fresh`). Returns the values to
    /// add, in order, and whether the screen needs a frame. Costs a
    /// fresh answer (a hash) for every 31 values at most, and nothing at
    /// rest.
    pub fn tick(
        &mut self,
        now_ms: u64,
        reduce: bool,
        len: usize,
        room: usize,
        draw: &mut dyn FnMut() -> Option<[u8; 32]>,
    ) -> (Vec<u8>, bool) {
        let mut landed = Vec::new();
        if self.left == 0 && !matches!(self.stage, Stage::Pause(_)) {
            return (landed, false);
        }
        if room == 0 && self.left > 0 {
            self.stop();
            return (landed, true);
        }
        if reduce {
            // No tumble and no pause: the values land at once.
            while self.left > 0 && landed.len() < room {
                let Some(v) = self.next(draw) else { break };
                self.land(v, len + landed.len());
                landed.push(v);
            }
            if landed.len() >= room || self.left == 0 {
                self.stop();
            }
            return (landed, true);
        }
        match self.stage {
            Stage::Idle => self.stage = Stage::Tumble(now_ms),
            Stage::Tumble(since) => {
                let t = now_ms.saturating_sub(since);
                self.progress = (t as f32 / ROLLER_TUMBLE_MS as f32).min(1.0);
                if t >= ROLLER_TUMBLE_MS {
                    if let Some(v) = self.next(draw) {
                        self.land(v, len);
                        landed.push(v);
                        self.stage = Stage::Pause(now_ms);
                    }
                } else if now_ms >= self.face_at {
                    self.spin = self.spin.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                    let f = ((self.spin >> 16) % 6) as u8 + 1;
                    self.shown = if f == self.shown { f % 6 + 1 } else { f };
                    self.face_at = now_ms + ROLLER_FACE_MS;
                }
            }
            Stage::Pause(since) => {
                if now_ms.saturating_sub(since) >= ROLLER_PAUSE_MS {
                    self.stage = if self.left == 0 {
                        Stage::Idle
                    } else {
                        self.progress = 0.0;
                        Stage::Tumble(now_ms)
                    };
                }
            }
        }
        (landed, true)
    }

    /// A value landed as the host's entry `pos`.
    fn land(&mut self, v: u8, pos: usize) {
        self.face = v;
        self.left -= 1;
        self.marks.push(pos);
    }
}
