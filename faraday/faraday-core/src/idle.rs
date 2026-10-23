//! What the idle lock keeps: when input last came, the idle time carried
//! over a lock, and what the warning has shown.

/// The idle lock's state (`docs/STRUCTURE-PASS.md` §7.4.4, step 48). One
/// field on `Faraday`; the fields are read and written through these
/// methods. The timers themselves are `Faraday::idle_tick`.
pub(crate) struct Idle {
    /// When the last input came, on this process's clock; set at the
    /// first tick.
    last_input: Option<u64>,
    /// Idle time the previous process had when it locked for idleness,
    /// counted toward power-off here.
    carry: u64,
    /// This process locks for idleness, after this many ms without input.
    locking: Option<u64>,
    /// The lock asked for last is a step of a backup flow (bringing in
    /// the setup's sticks, Lock and write), not the person's own Lock:
    /// it keeps Make a new wallet (`docs/history/NEW-WALLET.md` §18). Set by
    /// every way a lock is asked for.
    lock_for_flow: bool,
    /// The idle countdown's whole seconds last drawn, so a tick redraws
    /// only when the number on screen changes.
    shown: Option<u64>,
    /// Whole minutes without input when the idle warning opened: what
    /// its text says, fixed while it is up.
    warn_min: u64,
}

impl Idle {
    pub(crate) fn new() -> Self {
        Self {
            last_input: None,
            carry: 0,
            locking: None,
            lock_for_flow: false,
            shown: None,
            warn_min: 0,
        }
    }

    /// Input arrived at `now_ms`: the idle timers start again.
    pub(crate) fn input(&mut self, now_ms: u64) {
        self.last_input = Some(now_ms);
        self.carry = 0;
    }

    /// This process follows a lock for idleness after `ms` without
    /// input: the time counts toward power-off.
    pub(crate) fn followed_lock(&mut self, ms: u64) {
        self.carry = ms;
        self.last_input = None;
    }

    /// Time without input so far at `now_ms`, counted across an idle
    /// lock.
    pub(crate) fn ms(&self, now_ms: u64) -> u64 {
        self.last_input
            .map_or(0, |since| now_ms.saturating_sub(since))
            + self.carry
    }

    /// As [`Idle::ms`], starting the count at `now_ms` when no input has
    /// come yet.
    pub(crate) fn ms_starting(&mut self, now_ms: u64) -> u64 {
        let since = *self.last_input.get_or_insert(now_ms);
        now_ms.saturating_sub(since) + self.carry
    }

    /// The idle time this process locked after, when it locked for
    /// idleness.
    pub(crate) fn locking(&self) -> Option<u64> {
        self.locking
    }

    pub(crate) fn set_locking(&mut self, idle: Option<u64>) {
        self.locking = idle;
    }

    pub(crate) fn lock_for_flow(&self) -> bool {
        self.lock_for_flow
    }

    pub(crate) fn set_lock_for_flow(&mut self, for_flow: bool) {
        self.lock_for_flow = for_flow;
    }

    pub(crate) fn shown(&self) -> Option<u64> {
        self.shown
    }

    pub(crate) fn set_shown(&mut self, left: Option<u64>) {
        self.shown = left;
    }

    pub(crate) fn warn_min(&self) -> u64 {
        self.warn_min
    }

    pub(crate) fn set_warn_min(&mut self, minutes: u64) {
        self.warn_min = minutes;
    }
}
