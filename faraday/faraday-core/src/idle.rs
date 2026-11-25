//! What the idle lock keeps: when input last came, the idle time carried
//! over a lock, and what the warning has shown.

use crate::{Faraday, Sheet};
use faraday_crypto::work::Compute;

/// The idle lock's state. One
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
    /// This process follows a lock for idleness.
    followed: bool,
    /// The lock asked for last is a step of a backup flow (bringing in
    /// the setup's sticks, Lock and write), not the person's own Lock:
    /// it keeps Make a new wallet. Set by
    /// every way a lock is asked for.
    lock_for_flow: bool,
    /// The idle countdown's whole seconds last drawn, so a tick redraws
    /// only when the number on screen changes.
    shown: Option<u64>,
    /// Whole minutes without input when the idle warning opened: what
    /// its text says, fixed while it is up.
    warn_min: u64,
    /// While Make a new wallet holds the lock: the words on show were put
    /// away at the warning time, once; input clears it.
    masked_held: bool,
    /// The sheet the warning came over, put back when the warning is put
    /// away.
    under: Option<crate::Sheet>,
}

/// Holds nothing a lock wipes: the idle timers and the sheet the warning came over.
impl crate::at_lock::Public for Idle {}

impl Idle {
    pub(crate) fn new() -> Self {
        Self {
            last_input: None,
            carry: 0,
            locking: None,
            followed: false,
            lock_for_flow: false,
            shown: None,
            warn_min: 0,
            masked_held: false,
            under: None,
        }
    }

    /// Input arrived at `now_ms`: the idle timers start again.
    pub(crate) fn input(&mut self, now_ms: u64) {
        self.last_input = Some(now_ms);
        self.carry = 0;
        self.masked_held = false;
    }

    /// Notes that the held flow's words were put away; false when they
    /// already were since the last input.
    pub(crate) fn mask_held_once(&mut self) -> bool {
        !std::mem::replace(&mut self.masked_held, true)
    }

    /// This process follows a lock for idleness after `ms` without
    /// input: the time counts toward power-off.
    pub(crate) fn followed_lock(&mut self, ms: u64) {
        self.carry = ms;
        self.last_input = None;
        self.followed = true;
    }

    /// Whether this process follows a lock for idleness.
    pub(crate) fn follows_lock(&self) -> bool {
        self.followed
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

    /// Notes the sheet the warning came over.
    pub(crate) fn set_under(&mut self, sheet: Option<crate::Sheet>) {
        self.under = sheet;
    }

    /// The sheet the warning came over.
    pub(crate) fn under(&self) -> Option<crate::Sheet> {
        self.under
    }

    /// The sheet the warning came over, taken: it is remembered once.
    pub(crate) fn take_under(&mut self) -> Option<crate::Sheet> {
        self.under.take()
    }

    pub(crate) fn warn_min(&self) -> u64 {
        self.warn_min
    }

    pub(crate) fn set_warn_min(&mut self, minutes: u64) {
        self.warn_min = minutes;
    }
}

/// When the idle warning comes up and when the lock follows, ms without
/// input, for a lock after `lock_min` minutes; `None` when the session
/// never locks for idleness.
pub(crate) fn lock_at(lock_min: u16) -> Option<(u64, u64)> {
    (lock_min > 0).then(|| {
        let lock = u64::from(lock_min) * 60_000;
        (lock - crate::IDLE_WARN_MS.min(lock / 2), lock)
    })
}

impl Faraday {
    pub(crate) fn note_secrets(&mut self) {
        let h = self.holds();
        if h.kept || h.typed {
            self.tainted = true;
        }
    }

    /// The clean state (`docs/SECURITY-MODEL.md`): this process has never held or
    /// typed a secret. Only then may a stick be handed to the disk
    /// process; the shell publishes it for `faraday-grant`, and a lock is
    /// the way back to it.
    ///
    /// While the camera is on the process is not clean either, since a
    /// frame may picture a seed: no stick is handed out then, and one
    /// plugged in waits until the camera is off.
    pub fn clean(&mut self) -> bool {
        self.note_secrets();
        !self.tainted && !self.camera_on()
    }

    /// [`Faraday::clean`] without noting anything: whether this process has
    /// held or typed no secret, as the screen draws it.
    pub(crate) fn clean_now(&self) -> bool {
        let h = self.holds();
        !self.tainted && !self.camera_on() && !h.kept && !h.typed
    }

    /// Whether locking would wipe anything: a key, a vault, a wallet, a
    /// spend, or a secret on its way in any area (`docs/AT-LOCK.md`).
    pub(crate) fn lockable(&self) -> bool {
        self.holds().any()
    }

    /// Puts the idle warning away: the sheet it came over comes back,
    /// unless the warning took its content away (a code on the QR sheet).
    pub(crate) fn put_away_idle_warning(&mut self) {
        self.sheet = self
            .idle
            .take_under()
            .filter(|s| *s != Sheet::Qr || self.qr().is_some());
        // The vault step comes back with its field ready.
        if self.sheet == Some(Sheet::Import) {
            self.import_vault_step();
        }
    }

    /// Lock asked for: by the person (`for_flow` false), or by a step of
    /// a backup flow. Asks first when a vault is open with something
    /// not in it, a seal waits, the lock would end Make a new wallet, or it
    /// is a mode change and a file waits in From or For the stick (the
    /// change empties both), or it is a mode change (the sheet is the
    /// question, with nothing listed when nothing is held); else locks.
    pub(crate) fn lock_ask(&mut self, for_flow: bool) {
        let c = self.compute();
        self.idle.set_lock_for_flow(for_flow);
        let (keys, wallets) = self.unsaved_now(&c);
        let sealed = self.vaults.open.iter().any(|v| v.changes > 0);
        let files_go =
            self.mode_changing() && (!self.inbox().is_empty() || !self.outbox.is_empty());
        if self.lock_ends_create(&c)
            || self.lock_would_lose(&c)
            || files_go
            || !self.vaults.open.is_empty() && (sealed || !keys.is_empty() || !wallets.is_empty())
            || self.mode_changing()
        {
            self.vaults.name_lock(&c);
            self.sheet = Some(Sheet::LockAsk);
        } else {
            self.lock();
        }
    }

    /// Costs what `lock_held` costs. A lock now would end Make a new
    /// wallet: its keys are kept nowhere yet and the lock is the person's
    /// own, not a flow's step.
    pub(crate) fn lock_ends_create(&self, c: &Compute) -> bool {
        self.lock_held(c) && !self.idle.lock_for_flow()
    }

    /// Lock, unless it would lose an open vault's changes: then nothing
    /// is sealed or wiped, the vaults stay open with their changes, and
    /// the Lock sheet says what a lock without sealing loses.
    pub(crate) fn lock(&mut self) {
        if self.sealing() {
            self.lock_after_seal(false);
            return;
        }
        let c = self.compute();
        if self.lock_would_lose(&c) {
            if self.seal_waits() {
                self.lock_waits_for_seal();
            }
            self.vaults.name_lock(&c);
            self.sheet = Some(Sheet::LockAsk);
            return;
        }
        self.lock_now(false);
    }

    /// The lock a seal of one vault held back: the seal is over, so the
    /// lock goes ahead as it would have.
    pub(crate) fn lock_after_seal_ended(&mut self, may_lose: bool) {
        if may_lose {
            self.lock_now(true);
        } else {
            self.lock();
        }
    }

    /// The state could not be kept. A lock that would lose the keys that
    /// wait for their backup does not restart the process: the shell
    /// builds a fresh app in it over what this one carries
    /// ([`Faraday::carry_unkept_lock`]).
    pub(crate) fn not_kept(&mut self) {
        if self.restart && self.flows.iter().any(|f| f.waiting()) {
            self.restart = false;
            self.next_mode.clear();
        }
    }

    /// Lock: what can be sealed is, and the session goes. With `may_lose`
    /// a vault that cannot be sealed is let go, its changes lost (an idle
    /// lock, Lock without sealing); without it the lock stops there with
    /// that vault open and the Lock sheet shows. Only what the seals need
    /// is done here; what ends flows waits for the last seal
    /// (`vault::seal`).
    pub(crate) fn lock_now(&mut self, may_lose: bool) {
        if self.sealing() {
            self.lock_after_seal(may_lose);
            return;
        }
        self.job_locked();
        // What is on show goes first, its text wiped.
        self.lock_secrets_off_screen();
        self.idle.set_under(None);
        // The vaults seal on the worker; the lock goes on in
        // `lock_finish` when the last is done.
        self.vault_seal_all(may_lose);
    }
}
