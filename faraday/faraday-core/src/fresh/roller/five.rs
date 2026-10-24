//! The five-dice roller of Faraday Desktop (`docs/CODEX32.md` §5.5): the
//! booklet's dice de-biasing worksheet filled by a click. Its values are
//! this computer's randomness from `Faraday::fresh`, handed to the host
//! as faces 1 to N of the dice the person chose (six faces unless told,
//! [`FiveRoller::set_faces`]; up to 20) in the booklet's order (die 1 to die 5 for the first
//! roll, then for the second; an equal pair's die rolled twice again), to
//! be entered as the pad's keys are, so a click fills the sheet exactly
//! as the same faces typed would.
//!
//! The host holds one [`FiveRoller`], draws it with
//! [`super::five_screen::pairs`], gives its [`Act`]s to [`FiveRoller::act`]
//! and calls [`FiveRoller::tick`] on every tick, describing where its
//! entry stands with a [`Where`]. The roller never touches the entry: it
//! works the sheet out on a copy of the faces ([`Sheet`]) to know where
//! a pass or a character ends, and remembers which faces it made, by
//! character and place in it ([`FiveRoller::is_rolled`]), so the sheet
//! can mark them and the result can say "simulated dice". Hosts call
//! [`FiveRoller::trim`] whenever their entry gets shorter and
//! [`FiveRoller::reset`] when they start over.
//!
//! Offered only where `Faraday::roller_offered` says so.

use super::{Pool, Sim, Stage};
use faraday_draw::tokens::{ROLLER_FACE_MS, ROLLER_FIVE_BATCH, ROLLER_PAUSE_MS, ROLLER_TUMBLE_MS};
use faraday_entropy::booklet::{BOOKLET_DICE, BOOKLET_FACES, BookletChar, faces_ok};
use zeroize::Zeroizing;

/// How much one click rolls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Reach {
    /// The five dice once: the first values or the second.
    Pass,
    /// Both passes, an equal pair's die again: one character (the
    /// default).
    #[default]
    Character,
    /// Every character of the share being entered.
    Share,
    /// Every character of every initial share.
    End,
}

/// The reaches, in the order they are offered.
pub const REACHES: [Reach; 4] = [Reach::Pass, Reach::Character, Reach::Share, Reach::End];

/// A press on the roller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    /// Roll as much as one click does.
    Roll,
    /// Stop the click's rolls where they are.
    Stop,
    /// Choose how much one click rolls.
    Reach(Reach),
}

/// Where the host's entry stands, read at a press and at every tick.
#[derive(Debug, Clone, Copy)]
pub struct Where<'a> {
    /// The faces of the character in progress, in order, struck pairs
    /// included.
    pub seq: &'a [u8],
    /// Characters entered so far in all, of every share (a character
    /// typed counts).
    pub done: usize,
    /// Characters still to enter in the share in progress.
    pub share_left: usize,
    /// Characters still to enter in every share.
    pub all_left: usize,
}

/// The first empty place of `dice`, in the pad's order.
fn first_empty(dice: &BookletChar) -> Option<usize> {
    (0..2 * BOOKLET_DICE).find(|&p| {
        if p < BOOKLET_DICE {
            dice.first(p).is_none()
        } else {
            dice.second(p - BOOKLET_DICE).is_none()
        }
    })
}

/// The booklet's sheet worked out from a character's faces, as the host
/// works it out (`codex32/mod.rs`, `replay`): the dice, and the rows
/// where an equal pair struck its die.
struct Sheet {
    dice: BookletChar,
}

impl Sheet {
    /// The sheet the faces of dice of `faces` faces give.
    fn of(seq: &[u8], faces: u8) -> Sheet {
        let mut dice = BookletChar::with_faces(faces).unwrap_or_default();
        for &face in seq {
            let place = first_empty(&dice);
            if dice.push(face).is_err() {
                continue;
            }
            if let Some(p) = place.filter(|&p| p >= BOOKLET_DICE) {
                let die = p - BOOKLET_DICE;
                if dice.die_bit(die) == Some(None) {
                    dice.redo(die);
                }
            }
        }
        Sheet { dice }
    }

    /// Whether the faces make a whole character.
    fn complete(&self) -> bool {
        self.dice.character().is_ok()
    }

    /// The first empty place, in the pad's order: 0 to 4 the first
    /// roll's dice, 5 to 9 the second's.
    fn place(&self) -> Option<usize> {
        first_empty(&self.dice)
    }

    /// The dice, from 0, that the next faces go to: those of the row the
    /// first empty place is in, whose place in that row is empty.
    fn next_dice(&self) -> Vec<usize> {
        let first_row = (0..BOOKLET_DICE).any(|d| self.dice.first(d).is_none());
        (0..BOOKLET_DICE)
            .filter(|&d| {
                if first_row {
                    self.dice.first(d).is_none()
                } else {
                    self.dice.second(d).is_none()
                }
            })
            .collect()
    }
}

/// The five-dice roller's state. Holds no entry: only the dice, the
/// choice and which faces it made.
pub struct FiveRoller {
    reach: Reach,
    /// The dice's faces, 2 to 20: six unless the host says otherwise.
    faces: u8,
    /// Passes still to roll (a Pass click: one).
    passes: usize,
    /// Characters still to complete.
    chars: usize,
    stage: Stage,
    /// The face each die last landed with, 1 to `faces`, 0 before any.
    face: [u8; BOOKLET_DICE],
    /// The face each die shows while tumbling.
    shown: [u8; BOOKLET_DICE],
    /// The dice that tumble now, die 1 as bit 0.
    moving: u8,
    /// When the faces shown change next, ms.
    face_at: u64,
    /// Where in a tumble the dice are, 0 to 1.
    progress: f32,
    /// Cycles the faces of a tumble; no part of any value.
    spin: u32,
    /// Faces this computer made: (character, place in it), rising.
    marks: Vec<(usize, usize)>,
    pool: Pool,
}

impl Default for FiveRoller {
    fn default() -> Self {
        FiveRoller {
            reach: Reach::default(),
            faces: BOOKLET_FACES,
            passes: 0,
            chars: 0,
            stage: Stage::default(),
            face: [0; BOOKLET_DICE],
            shown: [0; BOOKLET_DICE],
            moving: 0,
            face_at: 0,
            progress: 0.0,
            spin: 0,
            marks: Vec::new(),
            pool: Pool::default(),
        }
    }
}

impl FiveRoller {
    /// The choice.
    pub fn reach(&self) -> Reach {
        self.reach
    }

    /// The dice's faces.
    pub fn faces(&self) -> u8 {
        self.faces
    }

    /// The dice the person chose: `faces` faces each, 2 to 20 (anything
    /// else is ignored). Faces already landed stay as they are.
    pub fn set_faces(&mut self, faces: u8) {
        if faces_ok(faces) {
            self.faces = faces;
        }
    }

    /// Whether a click's rolls are under way.
    pub fn rolling(&self) -> bool {
        self.passes > 0 || self.chars > 0
    }

    /// Whether the roller has anything to tick: rolls under way, or the
    /// pause after the last landing (a lookup).
    pub fn active(&self) -> bool {
        self.rolling() || matches!(self.stage, Stage::Pause(_))
    }

    /// The face die `die` (from 0) shows: the tumbling one while it
    /// moves, else the last landed (0: none yet).
    pub fn shown(&self, die: usize) -> u8 {
        if matches!(self.stage, Stage::Tumble(_)) && self.moving >> die & 1 == 1 {
            self.shown[die]
        } else {
            self.face[die]
        }
    }

    /// Whether die `die` (from 0) is tumbling now.
    pub fn tumbling(&self, die: usize) -> bool {
        matches!(self.stage, Stage::Tumble(_)) && self.moving >> die & 1 == 1
    }

    /// Where in its tumble the dice are, 0 to 1; 0 at rest.
    pub fn progress(&self) -> f32 {
        if matches!(self.stage, Stage::Tumble(_)) {
            self.progress
        } else {
            0.0
        }
    }

    /// A press. Nothing rolls where nothing is left to enter.
    pub fn act(&mut self, act: Act, at: &Where, now_ms: u64) {
        match act {
            Act::Roll => {
                if at.all_left == 0 {
                    return;
                }
                (self.passes, self.chars) = match self.reach {
                    Reach::Pass => (1, 0),
                    Reach::Character => (0, 1),
                    Reach::Share => (0, at.share_left.min(at.all_left)),
                    Reach::End => (0, at.all_left),
                };
                if !self.rolling() {
                    return;
                }
                self.begin(Sheet::of(at.seq, self.faces).next_dice(), now_ms);
            }
            Act::Stop => self.stop(),
            Act::Reach(r) => self.reach = r,
        }
    }

    /// Ends a click's rolls.
    pub fn stop(&mut self) {
        self.passes = 0;
        self.chars = 0;
        self.stage = Stage::Idle;
        self.moving = 0;
    }

    /// Forgets which faces it made and ends any rolls: the host's entry
    /// started over.
    pub fn reset(&mut self) {
        self.stop();
        self.marks.clear();
        self.face = [0; BOOKLET_DICE];
        self.pool.clear();
    }

    /// The host's entry is now `done` characters entered and `seq_len`
    /// faces into the next: faces past that are not the roller's any
    /// more (a face or a character taken back).
    pub fn trim(&mut self, done: usize, seq_len: usize) {
        while self.marks.last().is_some_and(|&m| m >= (done, seq_len)) {
            self.marks.pop();
        }
    }

    /// Whether face `place` (from 0) of character `ch` (from 0, over all
    /// the shares) was rolled by this computer.
    pub fn is_rolled(&self, ch: usize, place: usize) -> bool {
        self.marks.binary_search(&(ch, place)).is_ok()
    }

    /// Whether any face of character `ch` was rolled by this computer.
    pub fn char_rolled(&self, ch: usize) -> bool {
        let at = self.marks.partition_point(|&m| m < (ch, 0));
        self.marks.get(at).is_some_and(|m| m.0 == ch)
    }

    /// Whether the `faces` entered in all came from the roller, in part
    /// or all. A host that took characters typed counts one face for
    /// each, as they were not rolled either.
    pub fn sim(&self, faces: usize) -> Sim {
        let n = self.marks.len();
        if n == 0 {
            Sim::No
        } else if n >= faces {
            Sim::All
        } else {
            Sim::Some
        }
    }

    /// The faces that complete the character whose faces so far are
    /// `seq`, rolled as a click of One character rolls them (each from
    /// `draw`'s answers as [`Pool::next`] takes them, in the pad's order,
    /// an equal pair's die rolled twice again), for the step-through of
    /// `docs/CODEX32.md` §5.5 to enter one at a time. Nothing is marked:
    /// the host marks each face as it enters it ([`FiveRoller::mark`]).
    /// `None` while `draw` has no answer; the faces drawn so far are then
    /// dropped. Costs a fresh answer (a hash) for every 31 faces at most.
    pub fn roll_character(&mut self, seq: &[u8], draw: &mut Draw) -> Option<Zeroizing<Vec<u8>>> {
        let mut sim = Zeroizing::new(seq.to_vec());
        let mut out = Zeroizing::new(Vec::new());
        loop {
            let sheet = Sheet::of(&sim, self.faces);
            if sheet.complete() {
                return Some(out);
            }
            sheet.place()?;
            let v = self.pool.next(self.faces, draw)?;
            sim.push(v);
            out.push(v);
        }
    }

    /// Face `place` (from 0) of character `ch` (from 0, over all the
    /// shares) was rolled by this computer: the host entered it.
    pub fn mark(&mut self, ch: usize, place: usize) {
        let m = (ch, place);
        if let Err(at) = self.marks.binary_search(&m) {
            self.marks.insert(at, m);
        }
    }

    /// Starts a tumble of `dice`.
    fn begin(&mut self, dice: Vec<usize>, now_ms: u64) {
        self.moving = dice.iter().fold(0, |m, &d| m | 1 << d);
        self.stage = Stage::Tumble(now_ms);
        self.progress = 0.0;
        self.face_at = now_ms;
    }

    /// One pass lands, a face at a time in the pad's order, on the copy
    /// `sim` of the host's entry: the faces for the row the first empty
    /// place is in, ending when that row is full, when an equal pair
    /// struck its die (the die's two rolls are the next pass), or when a
    /// character is complete. Returns whether it completed a character.
    fn land(&mut self, sim: &mut Sim5, faces: &mut Vec<u8>, draw: &mut Draw) -> bool {
        let row = Sheet::of(&sim.seq, self.faces)
            .place()
            .map(|p| p / BOOKLET_DICE);
        let mut completed = false;
        let mut starved = false;
        while let Some(place) = Sheet::of(&sim.seq, self.faces).place() {
            if Some(place / BOOKLET_DICE) != row {
                break;
            }
            let Some(v) = self.pool.next(self.faces, draw) else {
                starved = true;
                break;
            };
            self.face[place % BOOKLET_DICE] = v;
            self.marks.push((sim.done, sim.seq.len()));
            sim.seq.push(v);
            faces.push(v);
            if Sheet::of(&sim.seq, self.faces).complete() {
                sim.seq.clear();
                sim.done += 1;
                sim.share_left = sim.share_left.saturating_sub(1);
                sim.all_left = sim.all_left.saturating_sub(1);
                self.chars = self.chars.saturating_sub(1);
                completed = true;
                break;
            }
        }
        if starved {
            // No answer yet: the rest lands on a later tick.
            return completed;
        }
        if self.passes > 0 {
            self.passes -= 1;
        }
        if sim.all_left == 0 {
            self.passes = 0;
            self.chars = 0;
        }
        completed
    }

    /// One tick. `reduce` is whether Reduce motion is on and `draw` asks
    /// for 32 fresh bytes (`Faraday::fresh`). Returns the faces to enter,
    /// in order, and whether the screen needs a frame. Costs a fresh
    /// answer (a hash) for every 31 faces at most, and nothing at rest.
    pub fn tick(
        &mut self,
        now_ms: u64,
        reduce: bool,
        at: &Where,
        draw: &mut Draw,
    ) -> (Vec<u8>, bool) {
        let mut faces = Vec::new();
        if !self.active() {
            return (faces, false);
        }
        if at.all_left == 0 && self.rolling() {
            self.stop();
            return (faces, true);
        }
        let mut sim = Sim5 {
            seq: Zeroizing::new(at.seq.to_vec()),
            done: at.done,
            share_left: at.share_left,
            all_left: at.all_left,
        };
        if reduce {
            // No tumble and no pause: the faces land at once, a batch at
            // a time so that no frame is long.
            while self.rolling() && faces.len() < ROLLER_FIVE_BATCH {
                let before = faces.len();
                self.land(&mut sim, &mut faces, draw);
                if faces.len() == before {
                    break;
                }
            }
            if !self.rolling() {
                self.stop();
            }
            return (faces, true);
        }
        match self.stage {
            Stage::Idle => self.begin(Sheet::of(&sim.seq, self.faces).next_dice(), now_ms),
            Stage::Tumble(since) => {
                let t = now_ms.saturating_sub(since);
                self.progress = (t as f32 / ROLLER_TUMBLE_MS as f32).min(1.0);
                if t >= ROLLER_TUMBLE_MS {
                    let completed = self.land(&mut sim, &mut faces, draw);
                    if faces.is_empty() {
                        // No answer yet: tumble on until there is one.
                    } else if completed || !self.rolling() {
                        self.stage = Stage::Pause(now_ms);
                        self.moving = 0;
                    } else {
                        self.begin(Sheet::of(&sim.seq, self.faces).next_dice(), now_ms);
                    }
                } else if now_ms >= self.face_at {
                    for die in 0..BOOKLET_DICE {
                        self.spin = self.spin.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                        let n = u32::from(self.faces);
                        let f = ((self.spin >> 16) % n) as u8 + 1;
                        self.shown[die] = if f == self.shown[die] {
                            f % self.faces + 1
                        } else {
                            f
                        };
                    }
                    self.face_at = now_ms + ROLLER_FACE_MS;
                }
            }
            Stage::Pause(since) => {
                if now_ms.saturating_sub(since) >= ROLLER_PAUSE_MS {
                    if self.rolling() {
                        self.begin(Sheet::of(&sim.seq, self.faces).next_dice(), now_ms);
                    } else {
                        self.stage = Stage::Idle;
                    }
                }
            }
        }
        (faces, true)
    }
}

/// What asks for 32 fresh bytes.
pub type Draw<'a> = dyn FnMut() -> Option<[u8; 32]> + 'a;

/// The host's entry as the roller carries it through one tick.
struct Sim5 {
    seq: Zeroizing<Vec<u8>>,
    done: usize,
    share_left: usize,
    all_left: usize,
}
