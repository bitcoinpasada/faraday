//! The session's randomness: the seed, the pool of the adapter's answers and the person's own
//! accumulator (`randomness.md`; `main`'s `fresh.rs`).

use faraday_crypto::Secret;
use faraday_kernel::{AtLock, ChangeId, Holds};
use faraday_port::bytes::Bytes;

/// Answers kept waiting for the next draw: `main`'s pool, which kept at most twice this many.
pub(crate) const POOL: usize = 26;

/// How long a draw waits for the system's answers before it gives up (F102): a kernel that
/// credits no generator can take many seconds on a slow board, so nothing is said sooner.
pub(crate) const NO_SEED_MS: u64 = 60_000;

/// The most bytes one draw gives: `POOL` answers of 32 (`main`'s `TAKE_MAX`).
pub(crate) const TAKE_MAX: usize = POOL * 32;

/// A draw that found no seed or no answer in the pool: kept, with the change it answers, until
/// the answers are in or the wait is over.
struct Waiting {
    id: ChangeId,
    what: &'static str,
    n: usize,
    since_ms: u64,
}

#[derive(Default)]
pub(crate) struct Random {
    /// The first answer of the adapter, hashed with the person's own when they have added any.
    seed: Option<Secret<[u8; 32]>>,
    /// The next answers, each used once.
    pool: Vec<Secret<[u8; 32]>>,
    /// SHA-256 chained over what the person added; `None` before the first.
    own: Option<Secret<[u8; 32]>>,
    /// The draws waiting, first come first served.
    waiting: Vec<Waiting>,
    /// Entropy asks of the session's own that are not answered yet.
    in_flight: usize,
    /// When the wait for the seed began: the build of the scope (F102).
    born_ms: u64,
}

/// The answers `n` bytes take: one for every 32.
pub(crate) fn answers(n: usize) -> usize {
    n.div_ceil(32)
}

/// SHA-256 of the parts one after the other, the bytes put together once in a buffer that wipes.
fn hash(parts: &[&[u8]]) -> Secret<[u8; 32]> {
    let mut joined = Secret::new(Vec::with_capacity(parts.iter().map(|p| p.len()).sum()));
    for part in parts {
        joined.expose_mut().extend_from_slice(part);
    }
    Secret::new(faraday_crypto::sha256(joined.expose()))
}

/// The seed from the system's bytes (or the seed before) and the person's accumulator.
fn reseed(base: &[u8; 32], acc: &[u8; 32]) -> Secret<[u8; 32]> {
    hash(&[b"faraday reseed", base, acc])
}

impl Random {
    pub(crate) fn seeded(&self) -> bool {
        self.seed.is_some()
    }

    /// The adapter answered: the session's seed first, then the pool. Whether this answer was
    /// the seed.
    pub(crate) fn arrived(&mut self, bytes: &[u8; 32]) -> bool {
        if self.seed.is_none() {
            self.seed = Some(match self.own.as_ref() {
                Some(acc) => reseed(bytes, acc.expose()),
                None => Secret::new(*bytes),
            });
            true
        } else {
            if self.pool.len() < POOL * 2 {
                self.pool.push(Secret::new(*bytes));
            }
            false
        }
    }

    /// The person added a batch of their own: hashed into the accumulator, and the seed, when
    /// there is one, hashed again with it.
    pub(crate) fn own_add(&mut self, commitment: &[u8; 32]) {
        let zero = [0u8; 32];
        let before = self.own.as_ref().map_or(&zero, |acc| acc.expose());
        let acc = hash(&[b"faraday own", before, commitment]);
        if let Some(seed) = self.seed.as_ref() {
            self.seed = Some(reseed(seed.expose(), acc.expose()));
        }
        self.own = Some(acc);
    }

    /// `what` hashed with the session's seed and `count`; `None` before the seed arrived.
    pub(crate) fn draw(&self, what: &[u8], count: &[u8]) -> Option<Secret<[u8; 32]>> {
        let seed = self.seed.as_ref()?;
        Some(hash(&[what, seed.expose(), count]))
    }

    /// `n` bytes for `what`, one pooled answer for every 32 (the last cut to length), each hashed
    /// with `what` and with the person's accumulator when they added any. Nothing is taken, and
    /// `None` given, while the seed is not in or fewer answers wait than it needs.
    pub(crate) fn take_bytes(&mut self, what: &str, n: usize) -> Option<Bytes> {
        self.seed.as_ref()?;
        let answers = answers(n);
        if n > TAKE_MAX || self.pool.len() < answers {
            return None;
        }
        let mut out = Secret::new(Vec::with_capacity(answers * 32));
        for _ in 0..answers {
            let answer = self.pool.pop()?;
            let mixed = match self.own.as_ref() {
                Some(acc) => hash(&[
                    b"faraday fresh",
                    what.as_bytes(),
                    answer.expose(),
                    acc.expose(),
                ]),
                None => hash(&[b"faraday fresh", what.as_bytes(), answer.expose()]),
            };
            out.expose_mut().extend_from_slice(mixed.expose());
        }
        out.expose_mut().truncate(n);
        Some(out)
    }

    pub(crate) fn wait(&mut self, id: ChangeId, what: &'static str, n: usize, now_ms: u64) {
        self.waiting.push(Waiting {
            id,
            what,
            n,
            since_ms: now_ms,
        });
    }

    /// The first draw waiting that can be served now, with its bytes.
    pub(crate) fn serve(&mut self) -> Option<(ChangeId, Bytes, usize)> {
        let first = self.waiting.first()?;
        let (what, n) = (first.what, first.n);
        let bytes = self.take_bytes(what, n)?;
        Some((self.waiting.remove(0).id, bytes, answers(n)))
    }

    /// The build of the scope began: the wait for the seed counts from here.
    pub(crate) fn begin(&mut self, now_ms: u64) {
        self.born_ms = now_ms;
    }

    /// The seed is still missing a minute after the build began: the system gives none.
    pub(crate) fn gives_none(&self, now_ms: u64) -> bool {
        self.seed.is_none() && now_ms.saturating_sub(self.born_ms) >= NO_SEED_MS
    }

    /// How many more answers the adapter is to be asked for: what the draws waiting need, and
    /// the seed's own, less what the pool holds and what is on its way.
    pub(crate) fn wanted(&self) -> usize {
        let need: usize = self.waiting.iter().map(|w| answers(w.n)).sum::<usize>()
            + usize::from(self.seed.is_none());
        need.saturating_sub(self.pool.len() + self.in_flight)
    }

    /// The draws that have waited for the system's answers as long as they do.
    pub(crate) fn expired(&mut self, now_ms: u64) -> Vec<ChangeId> {
        let mut gone = Vec::new();
        let none = self.gives_none(now_ms);
        self.waiting.retain(|w| {
            let over = none || now_ms.saturating_sub(w.since_ms) >= NO_SEED_MS;
            if over {
                gone.push(w.id);
            }
            !over
        });
        gone
    }

    pub(crate) fn is_waiting(&self) -> bool {
        !self.waiting.is_empty()
    }

    pub(crate) fn in_flight(&self) -> usize {
        self.in_flight
    }

    pub(crate) fn asked(&mut self, n: usize) {
        self.in_flight += n;
    }

    pub(crate) fn answered(&mut self) {
        self.in_flight = self.in_flight.saturating_sub(1);
    }
}

impl AtLock for Random {
    /// The seed and the pool fill in ordinary use: counting them would keep every process
    /// unclean. What the person added is the session's: mixed with the system's, it is not a
    /// secret alone.
    fn holds(&self) -> Holds {
        Holds {
            kept: false,
            typed: false,
            session: self.own.is_some(),
            asks_first: None,
        }
    }

    fn hide(&mut self) {}

    /// Everything goes, the pool too: `main` kept the pool across a lock inside a process that
    /// then restarted, and a restart is what a lock is here.
    fn at_lock(&mut self) {
        *self = Random::default();
    }
}
