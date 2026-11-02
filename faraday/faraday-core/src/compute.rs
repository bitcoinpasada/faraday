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

    /// Starts `job`, whose line of text is `line`: on the worker, with the
    /// screen busy until its answer is applied; inline, applied now.
    pub(crate) fn start(&mut self, job: Job, line: &'static str) {
        if self.worker.inline() {
            let c = self.compute();
            let done = job.run(&c);
            self.apply_done(done);
            return;
        }
        let kind = job.kind();
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
    pub(crate) fn settle_jobs(&mut self) {
        while self.worker.busy().is_some() {
            let Some(answer) = self.worker.wait_answer() else {
                return;
            };
            self.take_answer(answer);
        }
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
        self.out.dirty = true;
        self.out.commands.push_back(crate::Command::Draw);
    }

    fn apply_done(&mut self, done: Done) {
        match done {
            Done::SamplesLoad(l) => self.samples_loaded(*l),
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
            A::Back | A::Cancel => {
                self.worker.stale();
                self.worker.set_busy(None);
                true
            }
            A::Lock | A::LockUnsealed | A::LockAsk => true,
            A::Scale(_) | A::Rotation(_) | A::Theme(_) | A::TextSize(_) => true,
            _ => false,
        }
    }

    /// A lock: the running job's answer has nowhere to go.
    pub(crate) fn job_locked(&mut self) {
        self.worker.locked();
        self.worker.set_busy(None);
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
