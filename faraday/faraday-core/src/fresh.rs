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

use faraday_bip::bitcoin::hashes::{Hash, HashEngine, sha256};
use faraday_shell_api::Command;
use zeroize::Zeroizing;

use crate::{Faraday, out::Out};

pub(crate) mod roller;

/// Answers kept waiting for the next key.
pub const POOL: usize = 3;

/// How long after the first display a missing seed is said on screen.
/// The shell answers the first `RequestEntropy` within the first frame;
/// two seconds is many frames on the slowest panel, so a seed still
/// missing then is a system that gives no randomness, not a slow one.
pub const SEED_GRACE_MS: u64 = 2_000;

/// The session's seed: the shell's first 32 bytes, wiped when the app
/// goes. It exists only once they arrived.
pub(crate) struct Seed(Zeroizing<[u8; 32]>);

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
}

impl Fresh {
    /// The shell answered: the session's seed first, then the pool.
    /// Returns whether the answer was the session's seed (a keyboard
    /// that arrived first then gets its code).
    pub(crate) fn arrived(&mut self, bytes: [u8; 32]) -> bool {
        if self.seed.is_none() {
            self.seed = Some(Seed(Zeroizing::new(bytes)));
            true
        } else {
            if self.pool.len() < POOL * 2 {
                self.pool.push(Zeroizing::new(bytes));
            }
            false
        }
    }

    /// 32 bytes of their own for `what`, or `None` while the system has
    /// not answered yet; either way another answer is asked for.
    pub(crate) fn take(&mut self, what: &[u8], out: &mut Out) -> Option<[u8; 32]> {
        out.command(Command::RequestEntropy);
        let bytes = self.pool.pop()?;
        let mut e = sha256::Hash::engine();
        e.input(b"faraday fresh");
        e.input(what);
        e.input(&bytes[..]);
        Some(sha256::Hash::from_engine(e).to_byte_array())
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

    /// Whether the dice roller is offered now: Faraday Desktop, with keys
    /// allowed to load.
    pub fn roller_offered(&self) -> bool {
        self.fresh.roller && self.may_load_keys()
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
