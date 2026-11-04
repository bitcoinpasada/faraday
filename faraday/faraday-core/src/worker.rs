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

/// What a job is, for the busy screen and for the rules between jobs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Practice's samples loaded.
    SamplesLoad,
    /// A vault opened or made: its Argon2id runs.
    VaultStretch,
    /// An aezeed read: its scrypt runs.
    AezeedRead,
    /// A KeePass database written: its Argon2id runs.
    KdbxWrite,
    /// An open vault sealed at a lock: its Argon2id runs.
    VaultSeal,
    /// A PSBT signed.
    Signing,
    /// A PSBT read and inspected.
    PsbtRead,
    /// A set of a wallet's public files made.
    PublicFiles,
}

impl Kind {
    /// Whether a newer job of this kind replaces one not yet started
    /// (the jobs that follow typing, `docs/WORKERS.md` §3.2).
    fn latest_wins(self) -> bool {
        match self {
            Kind::SamplesLoad
            | Kind::VaultStretch
            | Kind::AezeedRead
            | Kind::KdbxWrite
            | Kind::VaultSeal
            | Kind::Signing
            | Kind::PsbtRead
            | Kind::PublicFiles => false,
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
    Kdbx(Box<crate::vault::KdbxWork>),
    Seal(Box<crate::vault::SealWork>),
    Sign(Box<crate::signing::SignWork>),
    SpendRead(Box<crate::signing::read::ReadWork>),
    Files(Box<crate::public_files::FilesWork>),
}

/// What a job gives back.
pub(crate) enum Done {
    SamplesLoad(Box<crate::samples::Loaded>),
    Stretch(Box<crate::vault::StretchDone>),
    Aezeed(Box<crate::lightning::ReadDone>),
    Kdbx(Box<crate::vault::KdbxDone>),
    Seal(Box<crate::vault::SealDone>),
    Sign(Box<crate::signing::SignDone>),
    SpendRead(Box<crate::signing::read::ReadDone>),
    Files(Box<crate::public_files::FilesDone>),
}

impl Job {
    pub(crate) fn kind(&self) -> Kind {
        match self {
            Job::SamplesLoad(_) => Kind::SamplesLoad,
            Job::Stretch(_) => Kind::VaultStretch,
            Job::Aezeed(_) => Kind::AezeedRead,
            Job::Kdbx(_) => Kind::KdbxWrite,
            Job::Seal(_) => Kind::VaultSeal,
            Job::Sign(_) => Kind::Signing,
            Job::SpendRead(_) => Kind::PsbtRead,
            Job::Files(_) => Kind::PublicFiles,
        }
    }

    /// Costs what the job costs: its variant's own run.
    pub(crate) fn run(self, c: &Compute) -> Done {
        match self {
            Job::SamplesLoad(j) => Done::SamplesLoad(Box::new(j.run(c))),
            Job::Stretch(j) => Done::Stretch(Box::new(j.run(c))),
            Job::Aezeed(j) => Done::Aezeed(Box::new(j.run(c))),
            Job::Kdbx(j) => Done::Kdbx(Box::new(j.run(c))),
            Job::Seal(j) => Done::Seal(Box::new(j.run(c))),
            Job::Sign(j) => Done::Sign(Box::new(j.run(c))),
            Job::SpendRead(j) => Done::SpendRead(Box::new(j.run(c))),
            Job::Files(j) => Done::Files(Box::new(j.run(c))),
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
        }
    }

    /// Sets what is called, from the worker's thread, when an answer
    /// has been sent.
    pub(crate) fn set_wake(&mut self, wake: Box<dyn Fn() + Send>) {
        if let Ok(mut slot) = self.wake.lock() {
            *slot = Some(wake);
        }
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

fn spawn(wake: Waker) -> Link {
    let (ask, rx) = channel();
    let (tx, back) = channel();
    std::thread::Builder::new()
        .name("worker".into())
        .spawn(move || worker_loop(&rx, &tx, &wake))
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
