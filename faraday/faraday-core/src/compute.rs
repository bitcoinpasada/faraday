//! Where event code gets its token (`docs/COMPUTE.md` §3).
//!
//! Costly work takes a [`Compute`]. Code that holds `&mut Faraday` (an
//! event, a press, an act, a tick, a storage event, boot, the warm step
//! before a frame) mints one here; drawing holds only `&View` or
//! `&Faraday` and cannot. The token notes its work with this crate's
//! counter ([`crate::work`]).

use faraday_crypto::work::Compute;

use crate::strings::EN;
use crate::worker::kef::{KefId, KefStart};
use crate::worker::kef_make;
use crate::worker::{Answer, Busy, Done, Job};
use crate::{Action, Command, Faraday, Later, motion};
use faraday_keys::kef::{Envelope, OpenError, OpenJob, Opened, Progress, SealError, SealJob};
use std::sync::Arc;
use zeroize::Zeroizing;

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
            || (held && matches!(self.later, Some(Later::Press(Action::PublicOut(..)))))
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
        }
        if let Later::Press(_) | Later::Key(_) = later {
            self.backup_settle();
            self.after_input();
        }
        // The working toast goes once the work is done, unless the work
        // said something of its own.
        if toast.is_some()
            && self.out.toast.as_ref().map(|(t, _)| t.clone()) == toast
            && [EN.working_files, EN.working_node_key, EN.working_aezeed]
                .iter()
                .any(|w| toast.as_deref() == Some(*w))
        {
            self.out.toast = None;
        }
        self.out.redraw();
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
        self.out.redraw();
        self.out.commands.push_back(crate::Command::Draw);
    }

    /// Applies the answers that have come. Returns whether any came.
    pub(crate) fn drain_jobs(&mut self) -> bool {
        let mut any = false;
        self.kef_progress_sync();
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
        loop {
            let answer = self.worker.wait_answer().expect("no answer came");
            let quiet = self.worker.quiet_out_for(answer.generation);
            self.take_answer(answer);
            if !quiet {
                break;
            }
        }
        self.drain_jobs();
    }

    /// Waits for the quiet timed run's answer, if one is out, and takes
    /// it: for the tests of the read card's time (`testkit::settle_time`).
    #[cfg(feature = "testkit")]
    pub(crate) fn settle_time_job(&mut self) {
        while self.worker.quiet_out() {
            let answer = self.worker.wait_answer().expect("no answer came");
            self.take_answer(answer);
        }
    }

    fn take_answer(&mut self, answer: Answer) {
        answer.absorb();
        if self.worker.take_quiet(answer.generation) {
            let Ok(done) = answer.done else {
                panic!("a job panicked on the worker");
            };
            let changed = match done {
                Done::KefTime(t) => {
                    self.worker.kef_mut().timing_done();
                    crate::seeds::kef::kef_timed(self, t.rate_ms)
                }
                Done::Measured(m) => crate::vault::stretch::measured(self, *m),
                _ => false,
            };
            if changed {
                self.out.redraw();
                self.out.commands.push_back(crate::Command::Draw);
            }
            return;
        }
        let Ok(done) = answer.done else {
            panic!("a job panicked on the worker");
        };
        if self.worker.is_live(answer.generation) {
            self.worker.finished();
            self.worker.set_busy(None);
            self.apply_done(done);
            // An answer may have loaded or dropped a key.
            self.signer_check_sync();
        } else {
            // Made stale by Back: dropped, and wiped as it drops.
            drop(done);
        }
        self.reblind_owed();
        self.out.redraw();
        self.out.commands.push_back(crate::Command::Draw);
    }

    fn apply_done(&mut self, done: Done) {
        match done {
            Done::SamplesLoad(l) => self.samples_loaded(*l),
            Done::Stretch(s) => crate::vault::stretch::stretched(self, *s),
            Done::Aezeed(r) => self.aezeed_read(*r),
            Done::Kdbx(k) => crate::vault::stretch::kdbx_written(self, *k),
            Done::Seal(d) => crate::vault::seal::sealed(self, *d),
            Done::Sign(d) => crate::signing::signed_spend(self, *d),
            Done::SpendRead(d) => crate::signing::read::spend_read(self, *d),
            Done::Files(d) => crate::public_files::files_made(self, *d),
            Done::KefSeal(d) => {
                self.worker.kef_mut().sealed_done(*d);
                crate::kef_tool::poll(self);
            }
            Done::KefTime(_) | Done::Measured(_) => {}
            Done::KefOpen(d) => {
                self.worker.kef_mut().opened_done(*d);
                crate::seeds::kef::kef_opened_here(self);
                crate::kef_tool::poll(self);
                self.seed_carried_tidy();
            }
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
            // A seal is not made stale: its job keeps the vault's slot, marked
            // sealing, and its file is made whatever the person presses.
            A::Back | A::Cancel if self.sealing() => false,
            A::Back | A::Cancel => {
                self.worker.stale();
                self.worker.kef_mut().stop();
                self.worker.set_busy(None);
                crate::vault::stretch::ended(self);
                true
            }
            _ => stays_live(&action),
        }
    }

    /// What follows a storage event, `screen` the one it came to. A stick
    /// that takes the person to another screen leaves this one as a press
    /// would; otherwise a device still waiting comes back up once no other
    /// sheet is. Then, if the event moved the screen or the sheet away
    /// from where the running job was started, its answer is made stale and
    /// dropped where it arrives, as Back does, once for every job
    /// (`docs/WORKERS.md` §4).
    pub(crate) fn storage_left(&mut self, screen: crate::Screen) {
        if self.screen != screen {
            self.after_input();
        } else {
            self.trail.forget_entered();
            self.input_sheet();
            self.device_sheet();
        }
        let moved = self.worker.busy().is_some()
            && !self.sealing()
            && self
                .worker
                .origin()
                .is_some_and(|o| o != (self.screen, self.sheet));
        if moved {
            self.worker.stale();
            self.worker.kef_mut().stop();
            self.worker.set_busy(None);
            crate::vault::stretch::ended(self);
            self.out.redraw();
            self.out.commands.push_back(crate::Command::Draw);
        }
    }

    /// Whether a key is refused while a job runs: every key but Escape,
    /// which is Back or Cancel and is taken as the press is. A key that
    /// would act (Enter in a field) is no more live than its press.
    pub(crate) fn key_refused_busy(&self, key: faraday_shell_api::Key) -> bool {
        self.worker.busy().is_some() && key != faraday_shell_api::Key::Escape
    }

    /// A Krux code's sheet is let go while its key is worked out: the job
    /// stops and its answer is dropped, not kept for the next job.
    pub(crate) fn kef_job_drop(&mut self) {
        if self.seed_read.kef().is_none() {
            return;
        }
        if self
            .worker
            .busy()
            .is_some_and(|b| b.kind == crate::worker::Kind::KefOpen)
        {
            self.worker.stale();
            self.worker.kef_mut().stop();
            self.worker.set_busy(None);
            crate::vault::stretch::ended(self);
        }
        self.worker.kef_mut().clear();
    }

    /// Stops the Krux code being made or being opened to check it, and
    /// drops its answer, as Cancel on the form does.
    pub(crate) fn kef_make_cancel(&mut self) {
        if self.worker.busy().is_some_and(|b| {
            matches!(
                b.kind,
                crate::worker::Kind::KefSeal | crate::worker::Kind::KefOpen
            )
        }) {
            self.worker.stale();
            self.worker.kef_mut().stop();
            self.worker.set_busy(None);
            crate::vault::stretch::ended(self);
        }
        self.worker.kef_mut().clear();
    }

    /// A lock: the running job's answer has nowhere to go.
    pub(crate) fn job_locked(&mut self) {
        self.worker.locked();
        self.worker.kef_mut().clear();
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
            | A::BootNoticeClose
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
        self.out.redraw();
        self.out.commands.push_back(crate::Command::Draw);
    }
}

/// Krux's encrypted code on the worker (`docs/KEF.md` §7.2, §10): what a
/// screen calls. The work is `faraday_keys::kef`'s, the state
/// `worker::kef`'s.
impl Faraday {
    /// Starts sealing `plaintext` under `passphrase` as a Krux code of
    /// `version` and `count` rounds, on the worker, `line` its working
    /// line. The IV (12 or 16 bytes, none for ECB) and a random id are
    /// drawn from the session's fresh bytes; with a public id the count
    /// takes Krux's random change. Without enough fresh bytes nothing
    /// starts: [`KefStart::NoFreshBytes`].
    pub fn kef_seal(
        &mut self,
        passphrase: Zeroizing<Vec<u8>>,
        plaintext: Zeroizing<Vec<u8>>,
        version: u8,
        id: KefId,
        count: u32,
        line: &'static str,
    ) -> Result<(), KefStart> {
        let refused = |r| KefStart::Refused(SealError::Refused(r));
        let iv_len = faraday_keys::kef::iv_len(version)
            .ok_or_else(|| refused(faraday_keys::kef::Refusal::Version))?;
        let random_len = match &id {
            KefId::Random(n) => *n,
            KefId::Public(_) => 0,
        };
        if random_len > faraday_keys::kef::ID_MAX {
            return Err(refused(faraday_keys::kef::Refusal::IdLength));
        }
        // One draw for the IV, the id and the number the count's change is
        // taken from: the bytes are spent only if all of them are here.
        let drawn = self
            .fresh_bytes(b"kef code", iv_len + random_len + 4)
            .ok_or(KefStart::NoFreshBytes)?;
        let (iv, rest) = drawn.split_at(iv_len);
        let (fresh_id, number) = rest.split_at(random_len);
        let number = u32::from_be_bytes(number.try_into().expect("four bytes"));
        let (id, count) = match id {
            KefId::Random(_) => (fresh_id.to_vec(), count),
            KefId::Public(bytes) => (bytes, faraday_keys::kef::count_with_change(count, number)),
        };
        let progress = Arc::new(Progress::default());
        let how = faraday_keys::kef::Sealing {
            version,
            id: &id,
            count,
            iv,
        };
        let job = SealJob::new(passphrase, plaintext, &how, Arc::clone(&progress));
        let job = Job::KefSeal(Box::new(crate::worker::kef::SealWork { job }));
        self.kef_started(progress, count, line, job);
        Ok(())
    }

    /// Starts opening `envelope` under `passphrase` on the worker, `line`
    /// its working line. The envelope's count is the work (`docs/KEF.md`
    /// §7.2): nothing runs before this press.
    pub fn kef_open(
        &mut self,
        passphrase: Zeroizing<Vec<u8>>,
        envelope: Envelope,
        line: &'static str,
    ) {
        let progress = Arc::new(Progress::default());
        let count = envelope.count;
        let job = OpenJob::new(passphrase, envelope, Arc::clone(&progress));
        let job = Job::KefOpen(Box::new(crate::worker::kef::OpenWork { job }));
        self.kef_started(progress, count, line, job);
    }

    /// Costs nothing here: sends the short timed run to the worker, for
    /// the read card's time. It has no busy screen, and a seal or an open
    /// stops it as it starts. Inline there is no time to measure. Returns
    /// whether a run is out.
    pub(crate) fn kef_time_start(&mut self) -> bool {
        if self.worker.inline() {
            return false;
        }
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        self.worker.kef_mut().timing_started(Arc::clone(&stop));
        let job = Job::KefTime(Box::new(crate::worker::kef::TimeWork { stop }));
        self.worker.send_quiet(job);
        true
    }

    fn kef_started(&mut self, progress: Arc<Progress>, count: u32, line: &'static str, job: Job) {
        self.worker.kef_mut().stop_timing();
        self.worker.kef_mut().started(progress);
        self.start(job, line);
        if let Some(b) = self.worker.busy().copied() {
            self.worker.set_busy(Some(Busy { of: count, ..b }));
        }
    }

    /// The last seal's answer, if one has come: the envelope's bytes or
    /// why there are none.
    pub fn kef_sealed(&self) -> Option<&Result<Vec<u8>, SealError>> {
        self.worker.kef().sealed()
    }

    /// The last open's answer, if one has come.
    pub fn kef_opened(&self) -> Option<&Result<Opened, OpenError>> {
        self.worker.kef().opened()
    }

    /// Takes the last seal's answer, leaving none.
    pub fn kef_take_sealed(&mut self) -> Option<Result<Vec<u8>, SealError>> {
        self.worker.kef_mut().take_sealed()
    }

    /// Takes the last open's answer, leaving none; what it holds wipes
    /// when dropped.
    pub fn kef_take_opened(&mut self) -> Option<Result<Opened, OpenError>> {
        self.worker.kef_mut().take_opened()
    }

    /// Notes the rounds done on the running job's `Busy`, for a
    /// determinate bar.
    pub(crate) fn kef_progress_sync(&mut self) {
        let Some(done) = self.worker.kef().rounds() else {
            return;
        };
        if let Some(b) = self.worker.busy().copied() {
            self.worker.set_busy(Some(Busy { done, ..b }));
        }
    }
}

/// Making a Krux encrypted code (`docs/KEF.md` §15): what the seed's form
/// and the tool in Tools call. The options and the code are
/// `worker::kef_make`'s.
impl Faraday {
    /// Starts making a code of seed `seed`'s entropy under `passphrase`
    /// (`docs/KEF.md` §6). A seed of 15, 18 or 21 words, another list's,
    /// or none gives [`kef_make::MakeStart::NotOffered`].
    pub fn kef_make_seed(
        &mut self,
        seed: usize,
        passphrase: Zeroizing<Vec<u8>>,
        options: &kef_make::Options,
    ) -> Result<(), kef_make::MakeStart> {
        let c = self.compute();
        let entropy = faraday_keys::kef::seed_plaintext(&self.session.keys, &c, seed)
            .ok_or(kef_make::MakeStart::NotOffered)?;
        self.kef_make(passphrase, entropy, options)
    }

    /// Starts making a code of `text`, as UTF-8 and nothing around it
    /// (a passphrase, a codex32 share, a note, a wallet's text).
    pub fn kef_make_text(
        &mut self,
        passphrase: Zeroizing<Vec<u8>>,
        text: Zeroizing<String>,
        options: &kef_make::Options,
    ) -> Result<(), kef_make::MakeStart> {
        self.kef_make(
            passphrase,
            Zeroizing::new(text.as_bytes().to_vec()),
            options,
        )
    }

    fn kef_make(
        &mut self,
        passphrase: Zeroizing<Vec<u8>>,
        plaintext: Zeroizing<Vec<u8>>,
        options: &kef_make::Options,
    ) -> Result<(), kef_make::MakeStart> {
        if let Some(p) = options.problem() {
            return Err(kef_make::MakeStart::Refused(p));
        }
        let writable =
            std::str::from_utf8(&passphrase).is_ok_and(faraday_keys::kef::writable_passphrase);
        if !writable {
            return Err(kef_make::MakeStart::Refused(EN.kef_refuse_passphrase));
        }
        self.kef_seal(
            passphrase,
            plaintext,
            options.version,
            options.kef_id(),
            options.count,
            EN.kef_making,
        )
        .map_err(|e| match e {
            KefStart::Refused(r) => kef_make::MakeStart::Refused(kef_make::refusal_line(r)),
            other => kef_make::MakeStart::Not(other),
        })
    }

    /// Costs a QR encoding. Takes the
    /// seal's answer: the code made, or what was refused, in words.
    pub fn kef_take_made(
        &mut self,
        options: &kef_make::Options,
    ) -> Option<Result<kef_make::Made, String>> {
        let answer = self.kef_take_sealed()?;
        let bytes = match answer {
            Ok(b) => b,
            Err(e) => return Some(Err(kef_make::refusal_text(e))),
        };
        let c = self.compute();
        Some(Ok(kef_make::made(&c, bytes, options)))
    }

    /// Puts the code's picture (when it fits one code) and its file in
    /// the Outbox, beside any other of the same name. Neither is a secret
    /// (`docs/QR.md` §7.1). Gives the name the toast reads: the picture's,
    /// or the file's when there is no picture.
    pub fn kef_save_made(&mut self, made: &kef_make::Made) -> String {
        let c = self.compute();
        let png = match &made.view {
            Ok(view) => view
                .png(&c)
                .map(|(name, png)| self.put_outbox_beside(&name, png)),
            Err(_) => None,
        };
        let file = self.put_outbox_beside(&made.file_name, made.bytes.clone());
        png.unwrap_or(file)
    }
}
