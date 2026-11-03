//! Where event code gets its token (`docs/COMPUTE.md` §3).
//!
//! Costly work takes a [`Compute`]. Code that holds `&mut Faraday` (an
//! event, a press, an act, a tick, a storage event, boot, the warm step
//! before a frame) mints one here; drawing holds only `&View` or
//! `&Faraday` and cannot. The token notes its work with this crate's
//! counter ([`crate::work`]).

use faraday_crypto::work::Compute;

use crate::strings::EN;
use crate::worker::{Answer, Busy, Done, Job};
use crate::{Action, Command, Faraday, Later, motion};

impl Faraday {
    /// A token for costly work, for code that holds the app to change
    /// it. Cheap.
    #[expect(
        clippy::disallowed_methods,
        reason = "a mint: event code holding &mut Faraday (docs/COMPUTE.md §3)"
    )]
    pub fn compute(&mut self) -> Compute {
        Compute::new(crate::work::core_count)
    }
}

/// The press left to wait for a frame, and the job on the worker
/// (`docs/SPEED.md` §9, `docs/WORKERS.md`).
impl Faraday {
    /// Says what a press left for the tick is doing, at once: the toast
    /// shows in full on the frame drawn before the work.
    pub(crate) fn working(&mut self, text: &str) {
        self.toast(text);
        self.out.toast_at = Some(self.now_ms.saturating_sub(motion::TOAST_MS));
    }

    /// Does the press waiting, once a frame has been drawn since it was
    /// pressed. Public files a slider asked for wait until it is let go.
    /// Returns whether it did.
    pub(crate) fn later_tick(&mut self) -> bool {
        let held = matches!(self.pointer.pressed(), Some((Action::Slide(..), _)));
        if !self.worker.drawn()
            || self.later.is_none()
            || (held && matches!(self.later, Some(Later::PublicOut(..))))
        {
            return false;
        }
        self.later_now();
        true
    }

    /// Does the press waiting now, and for a script (`inline`) what that
    /// leaves in turn: a vault made, then unlocked.
    pub(crate) fn later_all(&mut self, inline: bool) {
        self.later_now();
        while inline && self.later.is_some() {
            self.later_now();
        }
    }

    /// Does the press waiting now, whatever has been drawn.
    pub(crate) fn later_now(&mut self) {
        let Some(later) = self.later.take() else {
            return;
        };
        let toast = self.out.toast.as_ref().map(|(t, _)| t.clone());
        if let Later::Press(_) | Later::Key(_) = later {
            self.note_here();
        }
        match later {
            Later::Vault(w, inline) => {
                let was = self.worker.set_inline(inline || self.worker.inline());
                crate::vault::stretch::work(self, w);
                self.worker.set_inline(was);
            }
            Later::Press(a) => self.act(a),
            Later::Key(k) => self.key(k),
            Later::PublicOut(w, what) => self.public_out(w, what),
        }
        if let Later::Press(_) | Later::Key(_) = later {
            self.backup_settle();
            self.after_input();
        }
        // The working toast goes once the work is done, unless the work
        // said something of its own.
        if toast.is_some()
            && self.out.toast.as_ref().map(|(t, _)| t.clone()) == toast
            && [EN.working_files, EN.working_node_key]
                .iter()
                .any(|w| toast.as_deref() == Some(*w))
        {
            self.out.toast = None;
        }
        self.out.dirty = true;
        self.out.commands.push_back(Command::Draw);
    }
    /// Runs jobs on the calling thread and applies them at once, as the
    /// snapshot tool and the tests do (`docs/WORKERS.md` §3.2).
    pub fn run_jobs_inline(&mut self, on: bool) {
        self.worker.set_inline(on);
    }

    /// Gives the shell's wake: called from the worker's thread each time
    /// an answer is ready, so the shell makes its next pass at once and the
    /// answer is applied there (`docs/WORKERS.md` §7). It must only wake
    /// the shell's loop.
    pub fn set_wake(&mut self, wake: Box<dyn Fn() + Send>) {
        self.worker.set_wake(wake);
    }

    /// Starts `job`, whose line of text is `line`: on the worker, with the
    /// screen busy until its answer is applied; inline, applied now.
    pub(crate) fn start(&mut self, job: Job, line: &'static str) {
        if self.worker.inline() {
            let c = self.compute();
            let done = job.run(&c);
            self.apply_done(done);
            self.reblind_owed();
            return;
        }
        let kind = job.kind();
        self.worker.set_origin(Some((self.screen, self.sheet)));
        self.worker.send(job);
        self.worker.set_busy(Some(Busy {
            kind,
            line,
            began_ms: self.now_ms,
            done: 0,
            of: 0,
        }));
        self.out.dirty = true;
        self.out.commands.push_back(crate::Command::Draw);
    }

    /// Applies the answers that have come. Returns whether any came.
    pub(crate) fn drain_jobs(&mut self) -> bool {
        let mut any = false;
        while let Some(answer) = self.worker.try_answer() {
            self.take_answer(answer);
            any = true;
        }
        any
    }

    /// Waits for the running job and applies its answer: for the tests of
    /// the busy screen (`testkit::settle`).
    #[cfg(feature = "testkit")]
    pub(crate) fn settle_jobs(&mut self) {
        while self.worker.busy().is_some() {
            let Some(answer) = self.worker.wait_answer() else {
                return;
            };
            self.take_answer(answer);
        }
        self.drain_jobs();
    }

    /// Waits for the answer of a job made stale and takes it, as the next
    /// tick does: for the tests of a dropped answer (`testkit::settle_stale`).
    #[cfg(feature = "testkit")]
    pub(crate) fn settle_stale_job(&mut self) {
        let answer = self.worker.wait_answer().expect("no answer came");
        self.take_answer(answer);
        self.drain_jobs();
    }

    fn take_answer(&mut self, answer: Answer) {
        answer.absorb();
        let Ok(done) = answer.done else {
            panic!("a job panicked on the worker");
        };
        if self.worker.is_live(answer.generation) {
            self.worker.finished();
            self.worker.set_busy(None);
            self.apply_done(done);
        } else {
            // Made stale by Back: dropped, and wiped as it drops.
            drop(done);
        }
        self.reblind_owed();
        self.out.dirty = true;
        self.out.commands.push_back(crate::Command::Draw);
    }

    fn apply_done(&mut self, done: Done) {
        match done {
            Done::SamplesLoad(l) => self.samples_loaded(*l),
            Done::Stretch(s) => crate::vault::stretch::stretched(self, *s),
            Done::Aezeed(r) => self.aezeed_read(*r),
            Done::Kdbx(k) => crate::vault::stretch::kdbx_written(self, *k),
            Done::Sign(d) => crate::signing::signed_spend(self, *d),
        }
    }

    /// Costs an elliptic-curve multiplication per key owing one. A signing
    /// job's answer was applied or dropped, so its keys are alone again and
    /// the reblind they were skipped for is done (`docs/WORKERS.md` §6).
    fn reblind_owed(&mut self) {
        if self.session.keys.owes_reblind() {
            let c = self.compute();
            self.session.keys.reblind_owed(&c);
        }
    }

    /// Whether `action` is taken while a job runs: only Back, Cancel and
    /// Lock, and what changes how the screen is drawn (`docs/WORKERS.md`
    /// §4). Back and Cancel make the job's answer stale.
    pub(crate) fn taken_while_busy(&mut self, action: crate::Action) -> bool {
        use crate::Action as A;
        if self.worker.busy().is_none() {
            return true;
        }
        match action {
            // Back or Cancel anywhere else (a Lock sheet dismissed, say)
            // is taken as it always is and leaves the job running.
            A::Back | A::Cancel
                if self
                    .worker
                    .origin()
                    .is_some_and(|o| o != (self.screen, self.sheet)) =>
            {
                true
            }
            A::Back | A::Cancel => {
                self.worker.stale();
                self.worker.set_busy(None);
                crate::vault::stretch::ended(self);
                true
            }
            _ => stays_live(&action),
        }
    }

    /// Whether a key is refused while a job runs: every key but Escape,
    /// which is Back or Cancel and is taken as the press is. A key that
    /// would act (Enter in a field) is no more live than its press.
    pub(crate) fn key_refused_busy(&self, key: faraday_shell_api::Key) -> bool {
        self.worker.busy().is_some() && key != faraday_shell_api::Key::Escape
    }

    /// A lock: the running job's answer has nowhere to go.
    pub(crate) fn job_locked(&mut self) {
        self.worker.locked();
        self.worker.set_busy(None);
        crate::vault::stretch::ended(self);
    }

    /// The idle clock does not run while a job does and starts again from
    /// its end (`docs/WORKERS.md` §4). Returns whether it is held.
    pub(crate) fn hold_idle(&mut self, now_ms: u64) -> bool {
        let held = self.worker.busy().is_some();
        if held {
            self.idle.input(now_ms);
        }
        held
    }

    /// The job that runs, for drawing to read.
    pub fn busy(&self) -> Option<&Busy> {
        self.worker.busy()
    }
}

/// Whether `action` stays live, drawn enabled, while a job runs: Back and
/// Cancel, Lock, and what changes how the screen is drawn
/// (`docs/WORKERS.md` §4).
pub(crate) fn stays_live(action: &crate::Action) -> bool {
    use crate::Action as A;
    matches!(
        action,
        A::Back
            | A::Cancel
            | A::Lock
            | A::LockUnsealed
            | A::LockAsk
            | A::Scale(_)
            | A::Rotation(_)
            | A::Theme(_)
            | A::TextSize(_)
    )
}

impl Faraday {
    /// What the frame draws dimmed while a job runs.
    pub(crate) fn keep_live(&self) -> Option<fn(&crate::Action) -> bool> {
        self.busy()
            .map(|_| stays_live as fn(&crate::Action) -> bool)
    }

    /// Where the job's progress indicator stands, for drawing: the share
    /// done where the work has a count, and where its moving bar is along
    /// the track where it has none (still under reduce motion).
    pub(crate) fn busy_marks(&self, b: &Busy) -> (Option<f32>, Option<f32>) {
        let done = (b.of > 0).then(|| b.done as f32 / b.of as f32);
        let phase =
            (!self.reduce_motion()).then(|| crate::motion::busy_phase(b.began_ms, self.now_ms));
        (done, phase)
    }

    /// Sets the busy state with no job behind it (`testkit::show_busy`).
    #[cfg(feature = "testkit")]
    pub(crate) fn show_busy(&mut self, busy: Busy) {
        self.worker.set_busy(Some(busy));
        self.out.dirty = true;
        self.out.commands.push_back(crate::Command::Draw);
    }
}
