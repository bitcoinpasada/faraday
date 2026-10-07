//! The vanity address grinder's state (`docs/PLANNING.md` §16.117).
//!
//! The engine is `osk_bip::vanity`; this is what the screens hold
//! between frames: which dial is being turned, the prefix as it is
//! typed, how far the counter has gone, how long that took, and the
//! find that stopped it.
//!
//! The core has one thread, so a grind is not a thread: every tick
//! spends a budget of candidates and comes back, and the budget is
//! resized from how long the last one took so that a frame is still a
//! frame. The find is the same whatever the budget, because the order
//! is the counter's.

use osk_bip::keys::{Network, ScriptType};
use osk_bip::vanity::{self, Find};

/// The longest prefix a person types, which is a whole Taproot address
/// on a test network.
pub const MAX_PREFIX: usize = vanity::MAX_ADDRESS_CHARS;

/// How long one tick aims to spend grinding, in milliseconds. Short
/// enough that Stop lands promptly and the count moves.
const TICK_MS: u64 = 100;

/// The first budget of each dial: small enough that the first tick
/// returns quickly on the slowest device, since it is the tick the rate
/// is first measured over.
const FIRST_BUDGET: u64 = 8;

/// The largest budget one tick takes, whatever the measured rate.
const MAX_BUDGET: u64 = 8192;

/// Which dial the counter turns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dial {
    /// The key's passphrase, with the counter appended.
    Passphrase,
    /// The account index of the key's own passphrase.
    Account,
}

impl Dial {
    /// Both dials, in the order the Choice lists them.
    pub const ALL: [Dial; 2] = [Dial::Passphrase, Dial::Account];
}

/// Where the flow is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// §5 Choice, "Which script type?".
    Script,
    /// §5 Entry: the prefix, on the address keyboard.
    Prefix,
    /// §5 Record: the count, the rate, the expected time and Stop.
    Running,
    /// §5 Result: the address and what reaches it.
    Found,
    /// §5 Secret: the counter the find names, which is part of a
    /// passphrase and is masked like one.
    Secret,
}

/// The prefix as it is typed, in a fixed buffer. An address is public,
/// so this is not a secret; it is fixed-size because nothing in this
/// file holds heap text.
pub struct Prefix {
    buf: [u8; MAX_PREFIX],
    len: u8,
}

impl Prefix {
    /// The fixed prefix every address of `script` on `network` begins
    /// with, which is what the field starts at.
    pub fn fixed(script: ScriptType, network: Network) -> Self {
        let mut out = Prefix {
            buf: [0; MAX_PREFIX],
            len: 0,
        };
        for b in vanity::fixed_prefix(script, network).bytes() {
            out.push_byte(b);
        }
        out
    }

    /// The text.
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..usize::from(self.len)]).expect("an address is ascii")
    }

    /// How many characters the fixed part is, which is where deleting
    /// stops.
    pub fn fixed_len(&self, script: ScriptType, network: Network) -> usize {
        vanity::fixed_prefix(script, network).len()
    }

    fn push_byte(&mut self, b: u8) {
        let n = usize::from(self.len);
        if n < MAX_PREFIX {
            self.buf[n] = b;
            self.len += 1;
        }
    }

    /// Adds `c` where it can continue an address of this kind, and
    /// answers whether it did.
    pub fn push(&mut self, c: char, script: ScriptType, network: Network) -> bool {
        if !c.is_ascii() || usize::from(self.len) >= MAX_PREFIX {
            return false;
        }
        let mut next = Prefix {
            buf: self.buf,
            len: self.len,
        };
        next.push_byte(c as u8);
        if !vanity::can_begin(next.as_str(), script, network) {
            return false;
        }
        self.push_byte(c as u8);
        true
    }

    /// Deletes the last character, down to the fixed part.
    pub fn pop(&mut self, script: ScriptType, network: Network) {
        if usize::from(self.len) > self.fixed_len(script, network) {
            self.len -= 1;
        }
    }

    /// Whether anything has been asked for beyond the fixed part, which
    /// is what ✓ needs: the fixed prefix alone matches every address of
    /// its kind.
    pub fn has_free(&self, script: ScriptType, network: Network) -> bool {
        usize::from(self.len) > self.fixed_len(script, network)
    }
}

/// The grind, as the screens hold it.
pub struct Grind {
    /// Which dial the counter turns.
    pub dial: Dial,
    /// Which script type's first address is compared.
    pub script: ScriptType,
    /// Where the flow is.
    pub step: Step,
    /// The prefix being asked for.
    pub prefix: Prefix,
    /// The next counter to test.
    pub cursor: u64,
    /// How many candidates have been tested.
    pub tested: u64,
    /// How long that took, in milliseconds of wall clock: what the
    /// rate is measured over.
    pub elapsed_ms: u64,
    /// When the last tick was, so the next one can measure itself.
    pub last_ms: u64,
    /// How many candidates the next tick takes.
    pub budget: u64,
    /// Whether the counter space ran out.
    pub exhausted: bool,
    /// The candidate that matched.
    pub find: Option<Find>,
}

impl Grind {
    /// A grind of `dial` over a key, at the script type the device
    /// suggests first.
    pub fn new(dial: Dial, script: ScriptType, network: Network) -> Self {
        Grind {
            dial,
            script,
            step: Step::Script,
            prefix: Prefix::fixed(script, network),
            cursor: 0,
            tested: 0,
            elapsed_ms: 0,
            last_ms: 0,
            budget: FIRST_BUDGET,
            exhausted: false,
            find: None,
        }
    }

    /// Takes the script type, which resets the prefix to that type's
    /// own fixed characters.
    pub fn set_script(&mut self, script: ScriptType, network: Network) {
        self.script = script;
        self.prefix = Prefix::fixed(script, network);
    }

    /// Starts the run: the counter at zero, nothing measured yet.
    pub fn start(&mut self, now_ms: u64) {
        self.step = Step::Running;
        self.cursor = 0;
        self.tested = 0;
        self.elapsed_ms = 0;
        self.last_ms = now_ms;
        self.budget = FIRST_BUDGET;
        self.exhausted = false;
        self.find = None;
    }

    /// Candidates a second, measured over the ticks so far. `None`
    /// until the first tick has been timed.
    pub fn per_second(&self) -> Option<f64> {
        (self.elapsed_ms > 0 && self.tested > 0)
            .then(|| self.tested as f64 * 1000.0 / self.elapsed_ms as f64)
    }

    /// How many candidates this prefix is expected to take.
    pub fn expected(&self, network: Network) -> u64 {
        vanity::expected_candidates(self.prefix.as_str(), self.script, network)
    }

    /// How long that is at the measured rate, in seconds.
    pub fn expected_seconds(&self, network: Network) -> Option<u64> {
        vanity::expected_seconds(self.expected(network), self.per_second()?)
    }

    /// Takes what one tick did: `tested` candidates in the wall-clock
    /// time since the last tick, and resizes the next budget so that a
    /// tick stays about [`TICK_MS`].
    pub fn ticked(&mut self, tested: u64, now_ms: u64) {
        let span = now_ms.saturating_sub(self.last_ms).max(1);
        self.last_ms = now_ms;
        self.tested += tested;
        self.cursor += tested;
        self.elapsed_ms += span;
        if tested == 0 {
            self.exhausted = true;
            return;
        }
        let next = (tested as u128 * u128::from(TICK_MS) / u128::from(span)).max(1);
        self.budget = (next as u64).clamp(1, MAX_BUDGET);
    }
}
