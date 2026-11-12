//! The core's worker (`docs/WORKERS.md` §3): one thread that runs the
//! costly part of a press, a [`Job`], while the main thread goes on
//! drawing and taking input. The result is applied by the core on the
//! main thread, on a later pass.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
#[cfg(feature = "testkit")]
use std::time::Duration;

use faraday_crypto::work::Compute;

use crate::work;

pub mod kef;
pub mod kef_make;

/// What a job is, for the busy screen and for the rules between jobs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Practice's samples loaded.
    SamplesLoad,
    /// A vault opened or made: its Argon2id runs.
    VaultStretch,
    /// An aezeed read: its scrypt runs.
    AezeedRead,
    /// Seeds loaded from an open vault: each one's PBKDF2 runs.
    KeysLoad,
    /// A KeePass database written: its Argon2id runs.
    KdbxWrite,
    /// The one Argon2id pass that times this computer; it has no busy screen.
    VaultMeasure,
    /// An open vault sealed at a lock: its Argon2id runs.
    VaultSeal,
    /// A PSBT signed.
    Signing,
    /// A PSBT read and inspected.
    PsbtRead,
    /// A file's signatures added to a PSBT and the result checked.
    Collecting,
    /// A set of a wallet's public files made.
    PublicFiles,
    /// A QR vault sealed in Tools: its two Argon2id runs.
    QrSeal,
    /// A QR vault opened in Tools: its Argon2id run.
    QrOpen,
    /// A Krux code sealed: its PBKDF2 runs, and stops on Cancel.
    KefSeal,
    /// A Krux code opened: its PBKDF2 runs, and stops on Cancel.
    KefOpen,
    /// The short timed run for a Krux code's time; it has no busy screen.
    KefTime,
    /// A large file hashed in Check a timestamp.
    OtsHash,
    /// A Bitcoin Core wallet read in Tools: its passphrase stretch runs.
    CoreWalletRead,
}

impl Kind {
    /// Whether a newer job of this kind replaces one not yet started
    /// (the jobs that follow typing, `docs/WORKERS.md` §3.2).
    fn latest_wins(self) -> bool {
        match self {
            Kind::SamplesLoad
            | Kind::VaultStretch
            | Kind::AezeedRead
            | Kind::KeysLoad
            | Kind::KdbxWrite
            | Kind::VaultMeasure
            | Kind::VaultSeal
            | Kind::Signing
            | Kind::PsbtRead
            | Kind::Collecting
            | Kind::PublicFiles
            | Kind::QrSeal
            | Kind::QrOpen
            | Kind::KefSeal
            | Kind::KefOpen
            | Kind::KefTime
            | Kind::OtsHash
            | Kind::CoreWalletRead => false,
        }
    }
}

/// What the core keeps of the job that runs, for drawing to read
/// (`docs/WORKERS.md` §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Busy {
    /// What the job is.
    pub kind: Kind,
    /// Its line of text.
    pub line: &'static str,
    /// When it began, on the app's clock, ms.
    pub began_ms: u64,
    /// How much is done, where the work has a count; `of` is 0 where it
    /// has none.
    pub done: u32,
    /// How much there is to do.
    pub of: u32,
}

/// The costly part of a press, with what it needs, owned.
pub(crate) enum Job {
    SamplesLoad(Box<crate::samples::LoadJob>),
    Stretch(Box<crate::vault::StretchWork>),
    Aezeed(Box<crate::lightning::ReadWork>),
    KeysLoad(Box<crate::vault::LoadKeysWork>),
    Kdbx(Box<crate::vault::KdbxWork>),
    Measure(Box<crate::vault::MeasureWork>),
    Seal(Box<crate::vault::SealWork>),
    Sign(Box<crate::signing::SignWork>),
    SpendRead(Box<crate::signing::read::ReadWork>),
    Collect(Box<crate::signing::collect::CollectWork>),
    Files(Box<crate::public_files::FilesWork>),
    QrSeal(Box<crate::qr_tool::SealWork>),
    QrOpen(Box<crate::qr_tool::OpenWork>),
    KefSeal(Box<kef::SealWork>),
    KefOpen(Box<kef::OpenWork>),
    KefTime(Box<kef::TimeWork>),
    OtsHash(Box<crate::ots_check::HashWork>),
    CoreWalletRead(Box<crate::core_wallet::tool::ReadWork>),
}

/// What a job gives back.
pub(crate) enum Done {
    SamplesLoad(Box<crate::samples::Loaded>),
    Stretch(Box<crate::vault::StretchDone>),
    Aezeed(Box<crate::lightning::ReadDone>),
    KeysLoad(Box<crate::vault::LoadKeysDone>),
    Kdbx(Box<crate::vault::KdbxDone>),
    Measured(Box<crate::vault::MeasureDone>),
    Seal(Box<crate::vault::SealDone>),
    Sign(Box<crate::signing::SignDone>),
    SpendRead(Box<crate::signing::read::ReadDone>),
    Collect(Box<crate::signing::collect::CollectDone>),
    Files(Box<crate::public_files::FilesDone>),
    QrSeal(Box<crate::qr_tool::SealDone>),
    QrOpen(Box<crate::qr_tool::OpenDone>),
    KefSeal(Box<kef::SealDone>),
    KefOpen(Box<kef::OpenDone>),
    KefTime(Box<kef::TimeDone>),
    OtsHash(Box<crate::ots_check::HashDone>),
    CoreWalletRead(Box<crate::core_wallet::tool::ReadDone>),
}

impl Job {
    pub(crate) fn kind(&self) -> Kind {
        match self {
            Job::SamplesLoad(_) => Kind::SamplesLoad,
            Job::Stretch(_) => Kind::VaultStretch,
            Job::Aezeed(_) => Kind::AezeedRead,
            Job::KeysLoad(_) => Kind::KeysLoad,
            Job::Kdbx(_) => Kind::KdbxWrite,
            Job::Measure(_) => Kind::VaultMeasure,
            Job::Seal(_) => Kind::VaultSeal,
            Job::Sign(_) => Kind::Signing,
            Job::SpendRead(_) => Kind::PsbtRead,
            Job::Collect(_) => Kind::Collecting,
            Job::Files(_) => Kind::PublicFiles,
            Job::QrSeal(_) => Kind::QrSeal,
            Job::QrOpen(_) => Kind::QrOpen,
            Job::KefSeal(_) => Kind::KefSeal,
            Job::KefOpen(_) => Kind::KefOpen,
            Job::KefTime(_) => Kind::KefTime,
            Job::OtsHash(_) => Kind::OtsHash,
            Job::CoreWalletRead(_) => Kind::CoreWalletRead,
        }
    }

    /// Costs what the job costs: its variant's own run.
    pub(crate) fn run(self, c: &Compute) -> Done {
        match self {
            Job::SamplesLoad(j) => Done::SamplesLoad(Box::new(j.run(c))),
            Job::Stretch(j) => Done::Stretch(Box::new(j.run(c))),
            Job::Aezeed(j) => Done::Aezeed(Box::new(j.run(c))),
            Job::KeysLoad(j) => Done::KeysLoad(Box::new(j.run(c))),
            Job::Kdbx(j) => Done::Kdbx(Box::new(j.run(c))),
            Job::Measure(j) => Done::Measured(Box::new(j.run(c))),
            Job::Seal(j) => Done::Seal(Box::new(j.run(c))),
            Job::Sign(j) => Done::Sign(Box::new(j.run(c))),
            Job::SpendRead(j) => Done::SpendRead(Box::new(j.run(c))),
            Job::Collect(j) => Done::Collect(Box::new(j.run(c))),
            Job::Files(j) => Done::Files(Box::new(j.run(c))),
            Job::QrSeal(j) => Done::QrSeal(Box::new(j.run(c))),
            Job::QrOpen(j) => Done::QrOpen(Box::new(j.run(c))),
            Job::KefSeal(j) => Done::KefSeal(Box::new(j.run(c))),
            Job::KefOpen(j) => Done::KefOpen(Box::new(j.run(c))),
            Job::KefTime(j) => Done::KefTime(Box::new(j.run(c))),
            Job::OtsHash(j) => Done::OtsHash(Box::new(j.run(c))),
            Job::CoreWalletRead(j) => Done::CoreWalletRead(Box::new(j.run(c))),
        }
    }
}

struct Asked {
    job: Job,
    generation: u64,
}

pub(crate) struct Answer {
    /// `Err` when the job panicked (a test build; a release build aborts).
    pub(crate) done: Result<Done, ()>,
    pub(crate) generation: u64,
    counts: work::Counts,
    /// The vault stretches the job noted on the worker's thread.
    notes: Vec<work::VaultWork>,
}

/// What the shell gave to be called when an answer is ready
/// (`docs/WORKERS.md` §7): shared with the worker's thread.
type Waker = Arc<Mutex<Option<Box<dyn Fn() + Send>>>>;

struct Link {
    ask: Sender<Asked>,
    back: Receiver<Answer>,
}

/// The worker's state on the core's side. Private to this module;
/// reached through its methods.
pub(crate) struct Worker {
    /// Jobs run on the calling thread and are applied at once.
    inline: bool,
    /// The last generation given.
    generation: u64,
    /// The generation whose answer is applied; any other is stale.
    live: Option<u64>,
    link: Option<Link>,
    /// Called from the worker's thread after each answer is sent.
    wake: Waker,
    /// What the core keeps of the job that runs.
    busy: Option<Busy>,
    /// The screen and sheet the running job was started from: the only
    /// place a Back or Cancel makes its answer stale.
    origin: Option<(crate::Screen, Option<crate::Sheet>)>,
    /// Whether a frame has been drawn since a press was left to wait.
    drawn: bool,
    /// Krux's encrypted code: the running job's progress and the answers.
    kef: kef::KefArea,
    /// The generations of the quiet runs that are out (the Krux code's timed
    /// run, the vault measurement), which are no live job: they have no
    /// busy screen and no other job makes them stale.
    quiet: Vec<u64>,
}

impl Worker {
    pub(crate) fn new() -> Self {
        Self {
            inline: false,
            generation: 0,
            live: None,
            link: None,
            wake: Waker::default(),
            busy: None,
            origin: None,
            drawn: false,
            kef: kef::KefArea::default(),
            quiet: Vec::new(),
        }
    }

    /// Sets what is called, from the worker's thread, when an answer
    /// has been sent.
    pub(crate) fn set_wake(&mut self, wake: Box<dyn Fn() + Send>) {
        if let Ok(mut slot) = self.wake.lock() {
            *slot = Some(wake);
        }
    }

    pub(crate) fn kef(&self) -> &kef::KefArea {
        &self.kef
    }

    /// A timed Key Encryption Format run goes out: the flag that stops it.
    pub(crate) fn kef_timing_started(&mut self, stop: Arc<std::sync::atomic::AtomicBool>) {
        self.kef.timing_started(stop);
    }

    /// The timed run is told to stop.
    pub(crate) fn kef_stop_timing(&mut self) {
        self.kef.stop_timing();
    }

    /// The timed run's answer is in.
    pub(crate) fn kef_timing_done(&mut self) {
        self.kef.timing_done();
    }

    /// A seal or open goes out with its progress.
    pub(crate) fn kef_started(&mut self, progress: Arc<faraday_keys::kef::Progress>) {
        self.kef.started(progress);
    }

    /// The seal or open under way is told to stop.
    pub(crate) fn kef_stop(&mut self) {
        self.kef.stop();
    }

    /// The seal, open and timing are let go.
    pub(crate) fn kef_clear(&mut self) {
        self.kef.clear();
    }

    /// A seal's answer is in.
    pub(crate) fn kef_sealed_done(&mut self, done: kef::SealDone) {
        self.kef.sealed_done(done);
    }

    /// An open's answer is in.
    pub(crate) fn kef_opened_done(&mut self, done: kef::OpenDone) {
        self.kef.opened_done(done);
    }

    /// Takes the seal's answer, once.
    pub(crate) fn kef_take_sealed(
        &mut self,
    ) -> Option<Result<zeroize::Zeroizing<Vec<u8>>, faraday_keys::kef::SealError>> {
        self.kef.take_sealed()
    }

    /// Takes the open's answer, once.
    pub(crate) fn kef_take_opened(
        &mut self,
    ) -> Option<Result<faraday_keys::kef::Opened, faraday_keys::kef::OpenError>> {
        self.kef.take_opened()
    }

    pub(crate) fn busy(&self) -> Option<&Busy> {
        self.busy.as_ref()
    }

    pub(crate) fn set_busy(&mut self, busy: Option<Busy>) {
        self.busy = busy;
    }

    /// The screen and sheet the running job was started from.
    pub(crate) fn origin(&self) -> Option<(crate::Screen, Option<crate::Sheet>)> {
        self.origin
    }

    pub(crate) fn set_origin(&mut self, origin: Option<(crate::Screen, Option<crate::Sheet>)>) {
        self.origin = origin;
    }

    pub(crate) fn drawn(&self) -> bool {
        self.drawn
    }

    pub(crate) fn set_drawn(&mut self, drawn: bool) {
        self.drawn = drawn;
    }

    pub(crate) fn inline(&self) -> bool {
        self.inline
    }

    /// Sets inline mode, returning the mode it was in.
    pub(crate) fn set_inline(&mut self, on: bool) -> bool {
        std::mem::replace(&mut self.inline, on)
    }

    /// Sends `job` to the worker, starting the thread the first time, and
    /// makes it the job whose answer is applied.
    pub(crate) fn send(&mut self, job: Job) {
        self.generation += 1;
        let generation = self.generation;
        self.live = Some(generation);
        let wake = Arc::clone(&self.wake);
        let link = self.link.get_or_insert_with(|| spawn(wake));
        // The thread lives as long as this end does.
        let _ = link.ask.send(Asked { job, generation });
    }

    /// Sends `job` to the worker as a quiet one: no job is made stale, and
    /// its answer is told apart by [`Worker::take_quiet`].
    pub(crate) fn send_quiet(&mut self, job: Job) {
        self.generation += 1;
        let generation = self.generation;
        self.quiet.push(generation);
        let wake = Arc::clone(&self.wake);
        let link = self.link.get_or_insert_with(|| spawn(wake));
        let _ = link.ask.send(Asked { job, generation });
    }

    /// Whether the quiet job's answer is still to come.
    #[cfg(feature = "testkit")]
    pub(crate) fn quiet_out(&self) -> bool {
        !self.quiet.is_empty()
    }

    /// Whether `generation` is the quiet job's, leaving it.
    #[cfg(feature = "testkit")]
    pub(crate) fn quiet_out_for(&self, generation: u64) -> bool {
        self.quiet.contains(&generation)
    }

    /// Whether `generation` is the quiet job's; taking it ends it.
    pub(crate) fn take_quiet(&mut self, generation: u64) -> bool {
        let before = self.quiet.len();
        self.quiet.retain(|g| *g != generation);
        self.quiet.len() != before
    }

    /// How many jobs have been asked of the worker.
    #[cfg(feature = "testkit")]
    pub(crate) fn asked(&self) -> u64 {
        self.generation
    }

    /// Makes the running job's answer stale.
    pub(crate) fn stale(&mut self) {
        self.live = None;
    }

    /// A lock: every answer to come is dropped where it arrives.
    pub(crate) fn locked(&mut self) {
        self.live = None;
    }

    pub(crate) fn is_live(&self, generation: u64) -> bool {
        self.live == Some(generation)
    }

    pub(crate) fn finished(&mut self) {
        self.live = None;
    }

    /// The next answer, if one has come.
    pub(crate) fn try_answer(&mut self) -> Option<Answer> {
        self.link.as_ref()?.back.try_recv().ok()
    }

    /// Waits for the next answer, for tests: `None` when none comes.
    #[cfg(feature = "testkit")]
    pub(crate) fn wait_answer(&mut self) -> Option<Answer> {
        self.link
            .as_ref()?
            .back
            .recv_timeout(Duration::from_secs(120))
            .ok()
    }
}

impl Answer {
    /// Brings the work the job counted home to this thread's counts
    /// (`docs/WORKERS.md` §5).
    pub(crate) fn absorb(&self) {
        work::absorb(&self.counts);
        work::absorb_vault(self.notes.clone());
    }
}

/// Whether the worker waits before its next job, for the tests that put
/// something in between a job's start and its answer.
#[cfg(feature = "testkit")]
static HELD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(feature = "testkit")]
pub(crate) fn hold(on: bool) {
    HELD.store(on, std::sync::atomic::Ordering::SeqCst);
}

#[cfg(feature = "testkit")]
fn hold_gate() {
    while HELD.load(std::sync::atomic::Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// What a test gives to run first on the worker's thread, made on the
/// thread that starts the worker (`testkit::on_worker_start`).
#[cfg(feature = "testkit")]
pub type OnStart = Box<dyn Fn() -> Box<dyn FnOnce() + Send>>;

#[cfg(feature = "testkit")]
thread_local! {
    static ON_START: std::cell::RefCell<Option<OnStart>> = const { std::cell::RefCell::new(None) };
}

/// Sets what the worker this thread starts runs first on its own thread.
#[cfg(feature = "testkit")]
pub(crate) fn set_on_start(hook: Option<OnStart>) {
    ON_START.with(|h| *h.borrow_mut() = hook);
}

fn spawn(wake: Waker) -> Link {
    let (ask, rx) = channel();
    let (tx, back) = channel();
    #[cfg(feature = "testkit")]
    let start = ON_START.with(|h| h.borrow().as_ref().map(|f| f()));
    std::thread::Builder::new()
        .name("worker".into())
        .spawn(move || {
            #[cfg(feature = "testkit")]
            if let Some(start) = start {
                start();
            }
            worker_loop(&rx, &tx, &wake);
        })
        .expect("the worker thread starts");
    Link { ask, back }
}

/// The worker's loop: one job at a time, in the order asked, those
/// replaced by a newer of their kind left out.
fn worker_loop(rx: &Receiver<Asked>, tx: &Sender<Answer>, wake: &Waker) {
    #[expect(
        clippy::disallowed_methods,
        reason = "a mint: the worker, which draws nothing"
    )]
    let c = Compute::new(work::core_count);
    while let Ok(first) = rx.recv() {
        let mut batch = vec![first];
        batch.extend(rx.try_iter());
        let kinds: Vec<Kind> = batch.iter().map(|a| a.job.kind()).collect();
        for (i, asked) in batch.into_iter().enumerate() {
            let kind = kinds[i];
            if kind.latest_wins() && kinds[i + 1..].contains(&kind) {
                continue;
            }
            #[cfg(feature = "testkit")]
            hold_gate();
            let before = work::snapshot();
            let generation = asked.generation;
            let done = catch_unwind(AssertUnwindSafe(|| asked.job.run(&c))).map_err(|_| ());
            let counts = work::since(&before);
            let notes = work::take_vault_work();
            if tx
                .send(Answer {
                    done,
                    generation,
                    counts,
                    notes,
                })
                .is_err()
            {
                return;
            }
            if let Ok(slot) = wake.lock()
                && let Some(w) = slot.as_ref()
            {
                w();
            }
        }
    }
}
