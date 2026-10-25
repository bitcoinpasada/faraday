//! Fresh randomness for key material. The shell answers each
//! `RequestEntropy` with 32 bytes from the system's generator; the first
//! answer seeds the session, and the next few wait in a small pool. A key
//! made here (a vault's keys, a GPG key, Secure Boot keys) takes one
//! answer of its own, hashed with what it is for, and the pool is topped
//! up. A signing pass that opens a MuSig2 or FROST round takes
//! [`Faraday::sign_seed`]: the session's seed with a count of the passes,
//! so that no two passes of a
//! process draw the same nonce. A vault's seal nonce and a new keyboard's
//! code are drawn from the seed too (`vault/mod.rs`, `inputs.rs`). The seed
//! is a [`Seed`] held as an `Option`: until the shell's first answer there
//! is none, and nothing drawn from it is made.
//!
//! What a person adds of their own (dice, coins, cards, camera noise;
//! `docs/OWN-RANDOMNESS.md` §2.3) is hashed into [`Own`]'s accumulator,
//! which is mixed into the seed and into every pooled answer taken; it
//! never stands in for the system's bytes, and alone it makes nothing.

use faraday_bip::bitcoin::hashes::{Hash, HashEngine, sha256};
use faraday_shell_api::Command;
use zeroize::Zeroizing;

use crate::{Faraday, out::Out};

pub mod page;
pub(crate) mod roller;

/// Answers kept waiting for the next key.
pub const POOL: usize = 3;

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
    /// Pointer moves and keys while Add randomness is open; the count is
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

/// The person's own randomness: an accumulator over every commitment
/// added, and the counts. Kept in the session only: a lock wipes it,
/// and it is never written or shown. Invariant: `acc` is `Some` exactly
/// when a count is above zero, since only a taken batch changes either.
#[derive(Default)]
pub(crate) struct Own {
    /// SHA-256 chained over the commitments; `None` before the first.
    acc: Option<Zeroizing<[u8; 32]>>,
    /// What was added, for the status line.
    counts: OwnCounts,
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
    /// The shell offers the dice roller (Faraday Desktop only).
    roller: bool,
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
        let mut e = sha256::Hash::engine();
        e.input(b"faraday fresh");
        e.input(what);
        e.input(&bytes[..]);
        if let Some(acc) = self.own.acc.as_deref() {
            e.input(acc);
        }
        Some(sha256::Hash::from_engine(e).to_byte_array())
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

    /// The lock: what the person added is wiped, and the next session
    /// starts with none.
    pub(crate) fn own_wipe(&mut self) {
        self.own = Own::default();
        if let Some(p) = self.page.as_mut() {
            p.wipe();
        }
    }

    /// The seed a signing pass draws a MuSig2 or FROST round's secret
    /// nonces from: the session's seed hashed with a count of the
    /// passes, so a nonce shared again is never the one shared before.
    /// `None` until the session is seeded, and the shell is asked: a
    /// round is never opened on bytes the system did not give.
    pub(crate) fn sign_seed(&mut self, out: &mut Out) -> Option<[u8; 32]> {
        let Some(seed) = self.seed.as_ref() else {
            out.command(Command::RequestEntropy);
            return None;
        };
        self.sign_draws += 1;
        Some(seed.draw(b"faraday sign seed", &self.sign_draws.to_le_bytes()))
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
        }
    }

    /// 32 bytes of their own for `what`, or `None` while the system has
    /// not answered yet; either way another answer is asked for.
    pub(crate) fn fresh(&mut self, what: &[u8]) -> Option<[u8; 32]> {
        self.fresh.take(what, &mut self.out)
    }

    /// The shell is Faraday Desktop and offers the dice roller
    /// (`docs/CODEX32.md` §5.6): its values are this computer's
    /// randomness. Never set on the stick, the Pi or the website.
    pub fn set_roller_offered(&mut self, on: bool) {
        self.fresh.roller = on;
    }

    /// Whether the dice roller is offered now: Faraday Desktop in Demo,
    /// with keys allowed to load. It stands in the "Demo only" card, and
    /// in every other mode it is not offered at all (owner, 2026-10-24;
    /// `docs/DEMO.md` §14).
    pub fn roller_offered(&self) -> bool {
        self.fresh.roller && self.may_load_keys() && self.demo_passphrase()
    }

    /// A batch of the person's own randomness (Add randomness,
    /// `docs/OWN-RANDOMNESS.md` §2.3): `commitment` is the batch's, as
    /// New key's source makes it, and `n` how many it holds. Mixed into
    /// the seed and every fresh draw from now on, never used alone.
    /// Returns whether it was taken.
    pub fn own_add(&mut self, source: OwnSource, commitment: &[u8; 32], n: u32) -> bool {
        let taken = self.fresh.own_add(source, commitment, n);
        self.keygen.own_changed(self.fresh.own_acc().is_some());
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

    /// Add randomness, while it is on show.
    pub(crate) fn own_page(&self) -> Option<&page::OwnPage> {
        self.fresh.page.as_ref()
    }

    /// A press on Add randomness, or one that opens it.
    pub(crate) fn own_act(&mut self, a: page::Act) {
        if a == page::Act::Open {
            if self.fresh.page.is_none() {
                self.fresh.page = Some(page::OwnPage::new(!self.cameras().is_empty()));
            }
            self.screen = crate::Screen::Randomness;
            self.list_offset = 0.0;
            return;
        }
        if self.screen != crate::Screen::Randomness {
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
        if self.screen != crate::Screen::Randomness {
            return false;
        }
        match self.fresh.page.as_ref().and_then(|p| p.key(key)) {
            Some(a) => {
                self.own_act(a);
                true
            }
            None => false,
        }
    }

    /// After an input: Add randomness goes with its batches once another
    /// screen is on show, and the camera is on while Camera is its
    /// source. Run twice around New key's own camera, `on` false first so
    /// the one that goes off says so before the other comes on. A lookup.
    pub(crate) fn own_camera(&mut self, on: bool) {
        if self.screen != crate::Screen::Randomness {
            self.fresh.page = None;
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

    /// The seed a signing pass draws a MuSig2 or FROST round's secret
    /// nonces from (see [`Fresh::sign_seed`]).
    pub(crate) fn sign_seed(&mut self) -> Option<[u8; 32]> {
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
