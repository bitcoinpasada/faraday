//! Fresh randomness for key material. The shell answers each
//! `RequestEntropy` with 32 bytes from the system's generator; the first
//! answer seeds the session, and the next ones wait in a pool. A key
//! made here (a vault's keys, a GPG key, Secure Boot keys) takes one
//! answer of its own, hashed with what it is for, and the pool is topped
//! up; what needs more than 32 bytes (a SLIP-39 or codex32 split, Seed
//! XOR's parts, a KeePass file) takes an answer for every 32 of them
//! ([`Fresh::take_bytes`]), never one answer stretched. A signing pass that opens a MuSig2 round takes
//! [`Faraday::sign_seed`]: the session's seed with a count of the passes,
//! so that no two passes of a
//! process draw the same nonce. A vault's seal nonce takes a pooled
//! answer when one waits and is drawn from the seed when none does
//! (`vault/mod.rs`); a new keyboard's code is drawn from the seed
//! (`inputs.rs`), and so is what the session's keys are blinded from
//! (`keys_reblind`). The seed
//! is a [`Seed`] held as an `Option`: until the shell's first answer there
//! is none, and nothing drawn from it is made.
//!
//! What a person adds of their own (dice, coins, cards, camera noise,
//! pointer and keys;
//! `docs/OWN-RANDOMNESS.md` §2.3) is hashed into [`Own`]'s accumulator,
//! which is mixed into the seed and into every pooled answer taken; it
//! never stands in for the system's bytes, and alone it makes nothing.

use faraday_bip::bitcoin::hashes::{Hash, HashEngine, sha256};
use faraday_draw::tokens::PRESS_EMPHASIS_MS;
use faraday_shell_api::Command;
use zeroize::Zeroizing;

use crate::{Faraday, out::Out};

pub mod page;
pub(crate) mod roller;

/// Answers kept waiting for the next key: as many as the most one press
/// takes ([`Fresh::take_bytes`]), a codex32 set any nine of which recover
/// a 64-byte seed: eight random shares of 103 symbols, 824 bytes, 26
/// answers.
pub const POOL: usize = 26;

/// How long after the first display a missing seed is said on screen.
/// The shell answers the first `RequestEntropy` within the first frame;
/// two seconds is many frames on the slowest panel, so a seed still
/// missing then is a system that gives no randomness, not a slow one.
pub const SEED_GRACE_MS: u64 = 2_000;

/// The session's seed: the shell's first 32 bytes, hashed with what the
/// person added of their own when they have, and again at each addition
/// after; wiped when the app goes. It exists only once the shell's bytes
/// arrived.
pub(crate) struct Seed(Zeroizing<[u8; 32]>);

/// SHA-256("faraday reseed" ‖ `base` ‖ `acc`): the seed from the system's
/// bytes (or the seed before) and the person's accumulator.
fn reseed(base: &[u8; 32], acc: &[u8; 32]) -> Zeroizing<[u8; 32]> {
    let mut e = sha256::Hash::engine();
    e.input(b"faraday reseed");
    e.input(base);
    e.input(acc);
    Zeroizing::new(sha256::Hash::from_engine(e).to_byte_array())
}

/// A source a person adds randomness from (`docs/OWN-RANDOMNESS.md`
/// §2.1). Each batch arrives as its commitment, as New key's sources
/// make it (`faraday-entropy`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnSource {
    /// Camera frames; the count is frames.
    Camera,
    /// Dice rolls; the count is rolls.
    Dice,
    /// Coin flips; the count is flips.
    Coins,
    /// Cards drawn; the count is cards.
    Cards,
    /// Pointer moves over its area and keys typed while it is the source
    /// chosen, where the shell has a pointer and keys; the count is
    /// events.
    PointerKeys,
}

/// What a person added in this session, counted as it was taken. Nothing
/// is credited: a count says what was taken, not how strong it is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OwnCounts {
    /// Camera frames.
    pub camera_frames: u32,
    /// Dice rolls.
    pub dice_rolls: u32,
    /// Coin flips.
    pub coin_flips: u32,
    /// Cards drawn.
    pub cards: u32,
    /// Pointer moves and keys.
    pub pointer_keys: u32,
}

impl OwnCounts {
    /// Whether anything was added.
    pub fn any(&self) -> bool {
        *self != Self::default()
    }
}

/// The status line (`docs/OWN-RANDOMNESS.md` §2.4): "System: {source}",
/// or the operating system when the shell named none, then "· your own:"
/// and each count taken, or "none". Nothing is credited: a count says
/// what was taken, not how strong it is.
pub(crate) fn own_status(source: Option<&str>, c: OwnCounts) -> String {
    use crate::strings::{EN, fill};
    let system = fill(EN.own_system, &[source.unwrap_or(EN.create_device_os)]);
    let counts: Vec<String> = [
        (c.camera_frames, EN.own_camera_frame, EN.own_camera_frames),
        (c.dice_rolls, EN.own_dice_roll, EN.own_dice_rolls),
        (c.coin_flips, EN.own_coin_flip, EN.own_coin_flips),
        (c.cards, EN.own_card, EN.own_cards),
        (c.pointer_keys, EN.own_pointer_key, EN.own_pointer_keys),
    ]
    .into_iter()
    .filter(|&(n, _, _)| n > 0)
    .map(|(n, one, many)| {
        fill(
            EN.own_count,
            &[&n.to_string(), if n == 1 { one } else { many }],
        )
    })
    .collect();
    let yours = if counts.is_empty() {
        EN.own_none.to_string()
    } else {
        counts.join(", ")
    };
    fill(EN.own_status, &[&system, &yours])
}

/// What Add randomness says was added: how many batches in all and the
/// last one, as its line reads ("Added to the pool 3 times · last: 6 dice
/// rolls"), made once as the batch is taken, and the clock of the brief
/// emphasis that shows the line changed.
pub(crate) struct OwnAdded {
    /// Batches taken in this session.
    times: u32,
    /// The line, kept: made again only by the next batch taken.
    line: String,
    /// When the emphasis started: `None` until the first tick after the
    /// batch.
    at: Option<u64>,
    /// Whether the emphasis still runs (ticks ask for frames).
    moving: bool,
}

/// The person's own randomness: an accumulator over every commitment
/// added, and the counts. Kept in the session only: a lock wipes it,
/// and it is never written or shown. Invariant: `acc` is `Some` exactly
/// when a count is above zero, and `added` exactly when `acc` is, its
/// line naming the last batch taken, since only a taken batch changes
/// any of them.
#[derive(Default)]
pub(crate) struct Own {
    /// SHA-256 chained over the commitments; `None` before the first.
    acc: Option<Zeroizing<[u8; 32]>>,
    /// What was added, for the status line.
    counts: OwnCounts,
    /// What Add randomness's line says was added.
    added: Option<OwnAdded>,
}

/// A count's noun for `source`, one or more than one, as the status line
/// names it.
fn own_noun(source: OwnSource, n: u32) -> &'static str {
    use crate::strings::EN;
    let (one, many) = match source {
        OwnSource::Camera => (EN.own_camera_frame, EN.own_camera_frames),
        OwnSource::Dice => (EN.own_dice_roll, EN.own_dice_rolls),
        OwnSource::Coins => (EN.own_coin_flip, EN.own_coin_flips),
        OwnSource::Cards => (EN.own_card, EN.own_cards),
        OwnSource::PointerKeys => (EN.own_pointer_key, EN.own_pointer_keys),
    };
    if n == 1 { one } else { many }
}

/// Add randomness's line after `times` batches, the last `n` from
/// `source`: "Added to the pool once: 6 dice rolls", or "Added to the
/// pool 3 times · last: 6 dice rolls".
fn own_added_line(times: u32, source: OwnSource, n: u32) -> String {
    use crate::strings::{EN, fill};
    let last = fill(EN.own_count, &[&n.to_string(), own_noun(source, n)]);
    if times == 1 {
        fill(EN.own_added_once, &[&last])
    } else {
        fill(EN.own_added_times, &[&times.to_string(), &last])
    }
}

impl Own {
    /// `acc` ← SHA-256("faraday own" ‖ `acc` ‖ `commitment`), all zero
    /// before the first. A batch of nothing is refused: it adds no
    /// randomness, so it is neither counted nor mixed.
    fn add(&mut self, source: OwnSource, commitment: &[u8; 32], n: u32) -> bool {
        if n == 0 {
            return false;
        }
        let mut e = sha256::Hash::engine();
        e.input(b"faraday own");
        e.input(self.acc.as_deref().unwrap_or(&[0u8; 32]));
        e.input(commitment);
        self.acc = Some(Zeroizing::new(sha256::Hash::from_engine(e).to_byte_array()));
        let c = &mut self.counts;
        let count = match source {
            OwnSource::Camera => &mut c.camera_frames,
            OwnSource::Dice => &mut c.dice_rolls,
            OwnSource::Coins => &mut c.coin_flips,
            OwnSource::Cards => &mut c.cards,
            OwnSource::PointerKeys => &mut c.pointer_keys,
        };
        *count = count.saturating_add(n);
        let times = self.added.as_ref().map_or(0, |a| a.times).saturating_add(1);
        self.added = Some(OwnAdded {
            times,
            line: own_added_line(times, source, n),
            at: None,
            moving: true,
        });
        true
    }
}

impl Seed {
    /// `what` hashed with the seed and `count`: 32 bytes no other
    /// `what` or count draws.
    pub(crate) fn draw(&self, what: &[u8], count: &[u8]) -> [u8; 32] {
        let mut e = sha256::Hash::engine();
        e.input(what);
        e.input(&self.0[..]);
        e.input(count);
        sha256::Hash::from_engine(e).to_byte_array()
    }
}

/// Fresh randomness's state: the session's seed, the pool of answers
/// waiting for the next key, and the wait for a seed that does not come.
/// Its fields are private; it is given `out` to ask the shell for more.
#[derive(Default)]
pub(crate) struct Fresh {
    /// The session's seed, once the shell's first answer arrived.
    seed: Option<Seed>,
    /// When the wait for the seed started: the first tick after the
    /// display.
    seed_wait_from: Option<u64>,
    /// Whether the last frame said the seed is missing.
    seed_missing_drawn: bool,
    /// Signing passes that drew a seed of their own.
    sign_draws: u64,
    /// Fresh answers waiting for the next key.
    pool: Vec<Zeroizing<[u8; 32]>>,
    /// The shell offers the dice roller (Faraday Demo only).
    roller: bool,
    /// The shell has a pointer and keys, and Add randomness offers
    /// Pointer and keys (the desktop app).
    pointer_keys: bool,
    /// What the person added of their own.
    own: Own,
    /// Add randomness, while it is on show (`page.rs`).
    page: Option<page::OwnPage>,
    /// The camera is on for Add randomness.
    page_camera: bool,
}

impl Fresh {
    /// The shell answered: the session's seed first, then the pool.
    /// Returns whether the answer was the session's seed (a keyboard
    /// that arrived first then gets its code).
    pub(crate) fn arrived(&mut self, bytes: [u8; 32]) -> bool {
        if self.seed.is_none() {
            let seed = match self.own.acc.as_deref() {
                Some(acc) => reseed(&bytes, acc),
                None => Zeroizing::new(bytes),
            };
            self.seed = Some(Seed(seed));
            true
        } else {
            if self.pool.len() < POOL * 2 {
                self.pool.push(Zeroizing::new(bytes));
            }
            false
        }
    }

    /// 32 bytes of their own for `what`, or `None` while the system has
    /// not answered yet; either way another answer is asked for. With
    /// the person's own randomness added, it is hashed in after the
    /// system's answer.
    pub(crate) fn take(&mut self, what: &[u8], out: &mut Out) -> Option<[u8; 32]> {
        out.command(Command::RequestEntropy);
        let bytes = self.pool.pop()?;
        Some(self.mixed(what, &bytes))
    }

    /// `n` bytes of their own for `what`, each 32 from a pooled answer
    /// of its own, as [`Fresh::take`] makes them: `ceil(n / 32)` answers,
    /// the last one cut to what is left. `None`, with nothing taken,
    /// while fewer answers wait than it needs, or when `n` is more than
    /// [`POOL`] answers hold; either way as many answers are asked for
    /// as it takes.
    pub(crate) fn take_bytes(
        &mut self,
        what: &[u8],
        n: usize,
        out: &mut Out,
    ) -> Option<Zeroizing<Vec<u8>>> {
        let answers = n.div_ceil(32);
        for _ in 0..answers {
            out.command(Command::RequestEntropy);
        }
        if answers > POOL || self.pool.len() < answers {
            return None;
        }
        let mut bytes = Zeroizing::new(Vec::with_capacity(answers * 32));
        for _ in 0..answers {
            let answer = self.pool.pop()?;
            bytes.extend_from_slice(&Zeroizing::new(self.mixed(what, &answer))[..]);
        }
        bytes.truncate(n);
        Some(bytes)
    }

    /// SHA-256("faraday fresh" ‖ `what` ‖ `answer` ‖ the person's
    /// accumulator, when there is one): one answer, for `what`
    /// (`docs/OWN-RANDOMNESS.md` §2.3).
    fn mixed(&self, what: &[u8], answer: &[u8; 32]) -> [u8; 32] {
        let mut e = sha256::Hash::engine();
        e.input(b"faraday fresh");
        e.input(what);
        e.input(&answer[..]);
        if let Some(acc) = self.own.acc.as_deref() {
            e.input(acc);
        }
        sha256::Hash::from_engine(e).to_byte_array()
    }

    /// The person added a batch of their own: hashed into the
    /// accumulator, and the seed, when there is one, hashed again with
    /// it. The draws' counts go on, so no draw repeats one made before.
    /// Returns whether it was taken (a batch of nothing is not).
    pub(crate) fn own_add(&mut self, source: OwnSource, commitment: &[u8; 32], n: u32) -> bool {
        if !self.own.add(source, commitment, n) {
            return false;
        }
        if let (Some(seed), Some(acc)) = (self.seed.as_mut(), self.own.acc.as_deref()) {
            seed.0 = reseed(&seed.0, acc);
        }
        true
    }

    /// What the person added in this session.
    pub(crate) fn own_counts(&self) -> OwnCounts {
        self.own.counts
    }

    /// The accumulator, while anything was added (a lookup).
    pub(crate) fn own_acc(&self) -> Option<&[u8; 32]> {
        self.own.acc.as_deref()
    }

    /// Add randomness's line, when its emphasis started and whether it
    /// still runs, once a batch was added (a lookup).
    pub(crate) fn own_added(&self) -> Option<(&str, Option<u64>, bool)> {
        self.own
            .added
            .as_ref()
            .map(|a| (a.line.as_str(), a.at, a.moving))
    }

    /// One tick of the line's emphasis: starts its clock on the first
    /// tick after a batch, ends it once `ms` are up, or at once under
    /// `still`. Returns whether it still moves. A lookup.
    pub(crate) fn own_added_tick(&mut self, now_ms: u64, ms: u64, still: bool) -> bool {
        let Some(a) = self.own.added.as_mut().filter(|a| a.moving) else {
            return false;
        };
        let at = *a.at.get_or_insert(now_ms);
        a.moving = !still && now_ms < at + ms + crate::motion::FIRST_STEP_MS;
        a.moving
    }

    /// The added line's emphasis brought to rest at once, as everything
    /// moving is when a driver settles the screen.
    pub(crate) fn own_settle(&mut self) {
        if let Some(a) = self.own.added.as_mut() {
            a.moving = false;
        }
    }

    /// The lock: what the person added is wiped, and the next session
    /// starts with none.
    pub(crate) fn own_wipe(&mut self) {
        self.own = Own::default();
        if let Some(p) = self.page.as_mut() {
            p.wipe();
        }
    }

    /// The seed a signing pass draws a MuSig2 round's secret
    /// nonces from: the session's seed hashed with a count of the
    /// passes, so a nonce shared again is never the one shared before.
    /// `None` until the session is seeded, and the shell is asked: a
    /// round is never opened on bytes the system did not give.
    pub(crate) fn sign_seed(&mut self, out: &mut Out) -> Option<Zeroizing<[u8; 32]>> {
        let Some(seed) = self.seed.as_ref() else {
            out.command(Command::RequestEntropy);
            return None;
        };
        self.sign_draws += 1;
        Some(Zeroizing::new(
            seed.draw(b"faraday sign seed", &self.sign_draws.to_le_bytes()),
        ))
    }

    /// `what` hashed with the session's seed and `count`, or `None`
    /// before the seed arrived.
    pub(crate) fn draw(&self, what: &[u8], count: &[u8]) -> Option<[u8; 32]> {
        Some(self.seed.as_ref()?.draw(what, count))
    }

    /// Whether the session's seed has arrived.
    pub(crate) fn seeded(&self) -> bool {
        self.seed.is_some()
    }

    /// Whether no answer waits in the pool.
    pub(crate) fn pool_is_empty(&self) -> bool {
        self.pool.is_empty()
    }

    /// The seed is still missing well after the first display.
    pub(crate) fn missing(&self, now_ms: u64) -> bool {
        self.seed.is_none()
            && self
                .seed_wait_from
                .is_some_and(|from| now_ms.saturating_sub(from) >= SEED_GRACE_MS)
    }

    /// The tick: the wait for the seed starts at the first one after
    /// the display (`displayed`); returns whether the missing seed's
    /// line changed.
    pub(crate) fn tick(&mut self, displayed: bool, now_ms: u64) -> bool {
        if self.seed.is_none() && displayed && self.seed_wait_from.is_none() {
            self.seed_wait_from = Some(now_ms);
        }
        let missing = self.missing(now_ms);
        missing != std::mem::replace(&mut self.seed_missing_drawn, missing)
    }

    /// The first display asks for the seed and fills the pool.
    pub(crate) fn ask(&self, out: &mut Out) {
        if self.seed.is_some() {
            return;
        }
        for _ in 0..=POOL {
            out.command(Command::RequestEntropy);
        }
    }
}

impl Faraday {
    /// The shell answered: the session's seed first, then the pool.
    pub(crate) fn fresh_arrived(&mut self, bytes: [u8; 32]) {
        if self.fresh.arrived(bytes) {
            // A keyboard that arrived first gets its code now.
            self.inputs_coded();
            self.keys_reblind();
        }
    }

    /// Costs an elliptic-curve multiplication per key held. The session's
    /// keys blinded again from its seed (`docs/KEYS.md` §2), when the seed
    /// arrives and whenever the person's randomness changes it; a key added
    /// later is blinded from it as it is added.
    fn keys_reblind(&mut self) {
        if let Some(blind) = self.fresh.draw(b"faraday key blinding", b"") {
            let blind = Zeroizing::new(blind);
            let c = self.compute();
            self.session.keys.reblind(&c, &blind);
        }
    }

    /// 32 bytes of their own for `what`, or `None` while the system has
    /// not answered yet; either way another answer is asked for.
    pub(crate) fn fresh(&mut self, what: &[u8]) -> Option<[u8; 32]> {
        self.fresh.take(what, &mut self.out)
    }

    /// `n` bytes of their own for `what`, an answer for every 32
    /// ([`Fresh::take_bytes`]), or `None` while fewer answers wait than
    /// that; either way as many are asked for.
    pub(crate) fn fresh_bytes(&mut self, what: &[u8], n: usize) -> Option<Zeroizing<Vec<u8>>> {
        self.fresh.take_bytes(what, n, &mut self.out)
    }

    /// The shell is Faraday Demo and offers the dice roller
    /// (`docs/CODEX32.md` §5.6): its values are this computer's
    /// randomness. Never set on the stick, the Pi or the website.
    pub fn set_roller_offered(&mut self, on: bool) {
        self.fresh.roller = on;
    }

    /// Whether the dice roller is offered now: Faraday Demo in Practice,
    /// with keys allowed to load. It stands in the host's strip, and
    /// in every other mode it is not offered at all (owner, 2026-10-24;
    /// `docs/DEMO.md` §14).
    pub fn roller_offered(&self) -> bool {
        self.fresh.roller && self.may_load_keys() && self.demo_passphrase()
    }

    /// The shell has a pointer and a keyboard (the desktop app): Add
    /// randomness offers Pointer and keys
    /// (`docs/OWN-RANDOMNESS.md` §2.1). Never set on the stick, the Pi or
    /// the website.
    pub fn set_pointer_keys_offered(&mut self, on: bool) {
        self.fresh.pointer_keys = on;
    }

    /// Whether Add randomness offers Pointer and keys.
    pub fn pointer_keys_offered(&self) -> bool {
        self.fresh.pointer_keys
    }

    /// A batch of the person's own randomness (Add randomness,
    /// `docs/OWN-RANDOMNESS.md` §2.3): `commitment` is the batch's, as
    /// New key's source makes it, and `n` how many it holds. Mixed into
    /// the seed and every fresh draw from now on, never used alone.
    /// Returns whether it was taken.
    pub fn own_add(&mut self, source: OwnSource, commitment: &[u8; 32], n: u32) -> bool {
        let taken = self.fresh.own_add(source, commitment, n);
        self.keygen.own_changed(self.fresh.own_acc().is_some());
        if taken {
            self.keys_reblind();
        }
        taken
    }

    /// What the person added of their own in this session, for the
    /// status line.
    pub fn own_counts(&self) -> OwnCounts {
        self.fresh.own_counts()
    }

    /// The lock wipes what the person added.
    pub(crate) fn own_wipe(&mut self) {
        self.fresh.own_wipe();
        self.keygen.own_changed(false);
    }

    /// The status line (`docs/OWN-RANDOMNESS.md` §2.4) of Settings'
    /// Randomness row and Add randomness: the system's source, then what
    /// the person added. A lookup and a format.
    pub fn own_status(&self) -> String {
        own_status(self.keygen.entropy_source(), self.own_counts())
    }

    /// One tick of Add randomness's added line: its emphasis eases away
    /// over Material's medium 4 after each batch, none under reduce
    /// motion. Returns whether it still moves, so a frame is asked for.
    /// A lookup.
    pub(crate) fn own_tick(&mut self, now_ms: u64) -> bool {
        let still = self.reduce_motion();
        self.fresh.own_added_tick(now_ms, PRESS_EMPHASIS_MS, still)
    }

    /// Add randomness's added line, once anything was added, and how
    /// strongly it is emphasised now: 1 from the first tick after a
    /// batch is taken, easing to 0; 0 before that tick, so a frame no
    /// tick follows (a still picture) shows the line at rest. A lookup.
    pub(crate) fn own_added(&self) -> Option<(&str, f32)> {
        use crate::motion::{ease_move, progress};
        let (line, at, moving) = self.fresh.own_added()?;
        let shown = match at {
            Some(at) if moving && !self.reduce_motion() => {
                1.0 - ease_move(progress(Some(at), self.now_ms, PRESS_EMPHASIS_MS))
            }
            _ => 0.0,
        };
        Some((line, shown))
    }

    /// Add randomness, while it is on show.
    pub(crate) fn own_page(&self) -> Option<&page::OwnPage> {
        self.fresh.page.as_ref()
    }

    /// Whether Add randomness's sources are on show: its own page, or New
    /// key's Generator card, which offers them on itself (owner,
    /// 2026-10-25). A lookup.
    fn own_on_show(&self) -> bool {
        self.screen == crate::Screen::Randomness
            || (self.screen == crate::Screen::KeyGen && self.keygen.own_offered())
    }

    /// A press on Add randomness, or one that opens it.
    pub(crate) fn own_act(&mut self, a: page::Act) {
        if a == page::Act::Open {
            if self.fresh.page.is_none() {
                // Nothing is chosen or started until a source is pressed
                // (owner, 2026-10-26).
                self.fresh.page = Some(page::OwnPage::unchosen());
            }
            self.screen = crate::Screen::Randomness;
            self.list_offset = 0.0;
            return;
        }
        if !self.own_on_show()
            || (a == page::Act::Source(OwnSource::PointerKeys) && !self.fresh.pointer_keys)
        {
            return;
        }
        // The Generator card's Skip: on to the words, nothing added.
        if a == page::Act::Skip {
            if self.screen == crate::Screen::KeyGen {
                self.act(crate::Action::KNext);
            }
            return;
        }
        let batch = self.fresh.page.as_mut().and_then(|p| p.act(a));
        if let Some((source, commitment, n)) = batch {
            self.own_add(source, &commitment, n);
        }
    }

    /// A key typed on Add randomness; `false` when it is not on show or
    /// the key means nothing there.
    pub(crate) fn own_key(&mut self, key: faraday_shell_api::Key) -> bool {
        if !self.own_on_show() {
            return false;
        }
        if self
            .own_page()
            .is_some_and(page::OwnPage::takes_pointer_keys)
        {
            return self.own_pointer_key(key);
        }
        match self.fresh.page.as_ref().and_then(|p| p.key(key)) {
            Some(a) => {
                self.own_act(a);
                true
            }
            None => false,
        }
    }

    /// A key while Pointer and keys is the source chosen: a character,
    /// Backspace or Delete is taken, unless Space presses the control the
    /// keyboard is on; any other key is the page's, and what was taken
    /// is added first.
    fn own_pointer_key(&mut self, key: faraday_shell_api::Key) -> bool {
        use faraday_shell_api::Key as K;
        let presses = key == K::Char(' ') && self.pointer.focused().is_some();
        let now = self.now_ms;
        let Some(p) = self.fresh.page.as_mut() else {
            return false;
        };
        if presses || !p.takes_key(key) {
            self.own_flush();
            return false;
        }
        if let Some((source, c, n)) = p.key_typed(key, now) {
            self.own_add(source, &c, n);
        }
        true
    }

    /// A pointer move with no button down, in the frame's pixels: taken
    /// when Pointer and keys is the source chosen and the pointer is over
    /// its area, the topmost thing there; moved off it, what was taken
    /// is added. Returns whether the frame changes. A lookup.
    pub(crate) fn own_pointer(&mut self, x: u16, y: u16) -> bool {
        if !self.own_on_show()
            || !self
                .own_page()
                .is_some_and(page::OwnPage::takes_pointer_keys)
        {
            return false;
        }
        let area = page::Act::Area;
        let over = self
            .pointer
            .hit_at(i32::from(x), i32::from(y))
            .is_some_and(|(a, r)| a == crate::Action::Own(area) && r.contains(x.into(), y.into()));
        if !over {
            return self.own_flush();
        }
        let now = self.now_ms;
        let batch = self
            .fresh
            .page
            .as_mut()
            .and_then(|p| p.pointer_moved(x, y, now));
        if let Some((source, c, n)) = batch {
            self.own_add(source, &c, n);
        }
        true
    }

    /// Pointer and keys' batch added now, however few it holds: the
    /// pointer left its area or the window, a key was not its own, or
    /// the source or the page is left. Returns whether anything was added.
    pub(crate) fn own_flush(&mut self) -> bool {
        match self.fresh.page.as_mut().and_then(page::OwnPage::flush) {
            Some((source, c, n)) => self.own_add(source, &c, n),
            None => false,
        }
    }

    /// After an input: Add randomness goes with its batches once neither
    /// its page nor New key's Generator card is on show, and comes
    /// with the card, no source chosen, so that nothing runs unasked; the camera
    /// is on while Camera is its source. Run twice around New key's own
    /// camera, `on` false first so the one that goes off says so before
    /// the other comes on. A lookup.
    pub(crate) fn own_camera(&mut self, on: bool) {
        if !self.own_on_show() {
            // Pointer and keys' last events are added as the page goes.
            self.own_flush();
            self.fresh.page = None;
        } else if self.fresh.page.is_none() && self.screen == crate::Screen::KeyGen {
            self.fresh.page = Some(page::OwnPage::unchosen());
        }
        let want = self
            .fresh
            .page
            .as_ref()
            .is_some_and(page::OwnPage::wants_camera);
        if want == self.fresh.page_camera || want != on {
            return;
        }
        self.fresh.page_camera = want;
        self.out.command(if want {
            Command::CameraOn
        } else {
            Command::CameraOff
        });
        if let (false, Some(p)) = (want, self.fresh.page.as_mut()) {
            p.camera_off();
        }
    }

    /// After an input: the camera for noise, New key's and Add
    /// randomness's, on while one of them takes pictures; the one that
    /// goes off says so before the other comes on.
    pub(crate) fn cameras_noise(&mut self) {
        self.own_camera(false);
        self.keygen_camera();
        self.own_camera(true);
    }

    /// Whether the camera is on for noise, New key's or Add randomness's.
    pub(crate) fn noise_camera_on(&self) -> bool {
        self.keygen.camera_on() || self.fresh.page_camera
    }

    /// A camera frame for noise, in place of the one before.
    pub(crate) fn noise_frame(&mut self, luma: Vec<u8>) {
        if self.keygen.camera_on() {
            self.keygen.camera_frame(luma);
        } else if let Some(p) = self.fresh.page.as_mut() {
            p.camera_frame(luma);
        }
    }

    /// The seed a signing pass draws a MuSig2 round's secret
    /// nonces from (see [`Fresh::sign_seed`]).
    pub(crate) fn sign_seed(&mut self) -> Option<Zeroizing<[u8; 32]>> {
        self.fresh.sign_seed(&mut self.out)
    }

    /// Whether the session's seed has arrived: a vault seals, and a new
    /// keyboard gets its code, only once it has.
    pub fn seeded(&self) -> bool {
        self.fresh.seeded()
    }

    /// The seed is still missing well after the first display: the
    /// system gives no randomness, and the screen says Faraday has to
    /// restart.
    pub fn seed_missing(&self) -> bool {
        self.fresh.missing(self.now_ms)
    }

    /// The tick: see [`Fresh::tick`].
    pub(crate) fn seed_tick(&mut self) -> bool {
        self.fresh.tick(self.canvas.is_some(), self.now_ms)
    }

    /// The first display asks for the seed and fills the pool.
    pub(crate) fn fresh_ask(&mut self) {
        self.fresh.ask(&mut self.out);
    }
}
