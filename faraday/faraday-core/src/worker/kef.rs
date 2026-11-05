//! Krux's encrypted code on the worker (`docs/KEF.md` §7.2, §10,
//! `docs/WORKERS.md` §4): a seal and an open, each one job, each stopped
//! by Cancel while its key is worked out, with the rounds done readable
//! by the main thread. No screen reaches these yet; a screen starts a job
//! with `Faraday::kef_seal` or `Faraday::kef_open` and reads the answer
//! with the accessors beside them. The work itself is `faraday_keys::kef`'s.

use std::sync::Arc;

use faraday_crypto::work::Compute;
use faraday_keys::kef::{OpenError, OpenJob, Opened, Progress, SealError, SealJob};

/// A code to seal, to run on the worker.
pub(crate) struct SealWork {
    pub(crate) job: SealJob,
}

/// A seal's answer: the envelope's bytes or why there are none.
pub(crate) struct SealDone {
    sealed: Result<Vec<u8>, SealError>,
}

impl SealWork {
    /// Costs a PBKDF2 of the count's rounds, stopped by Cancel.
    pub(crate) fn run(self, c: &Compute) -> SealDone {
        SealDone {
            sealed: self.job.run(c),
        }
    }
}

/// A code to open, to run on the worker.
pub(crate) struct OpenWork {
    pub(crate) job: OpenJob,
}

/// An open's answer: the contents or why there are none.
pub(crate) struct OpenDone {
    opened: Result<Opened, OpenError>,
}

impl OpenWork {
    /// Costs a PBKDF2 of the envelope's count of rounds, stopped by Cancel.
    pub(crate) fn run(self, c: &Compute) -> OpenDone {
        OpenDone {
            opened: self.job.run(c),
        }
    }
}

/// What a code is sealed under for its id.
pub enum KefId {
    /// This many fresh random bytes (`docs/KEF.md` §10: 8 by default).
    Random(usize),
    /// Bytes the person chose or the wallet's fingerprint, which are
    /// public: the count then gets Krux's random change on top
    /// (`faraday_keys::kef::count_with_change`).
    Public(Vec<u8>),
}

/// Why a seal did not start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KefStart {
    /// The session's fresh bytes are not enough yet; more are asked for,
    /// and the press is made again once they come.
    NoFreshBytes,
    /// The code cannot be written as asked.
    Refused(SealError),
}

/// The Krux code area's state: the running job's progress and the last
/// answers, wiped when taken, replaced or at a lock.
#[derive(Default)]
pub(crate) struct KefArea {
    progress: Option<Arc<Progress>>,
    sealed: Option<Result<Vec<u8>, SealError>>,
    opened: Option<Result<Opened, OpenError>>,
}

impl KefArea {
    /// A job starts: the last answers go and `progress` is the running
    /// job's.
    pub(crate) fn started(&mut self, progress: Arc<Progress>) {
        self.sealed = None;
        self.opened = None;
        self.progress = Some(progress);
    }

    /// The rounds the running job has done, if one of this area's runs.
    pub(crate) fn rounds(&self) -> Option<u32> {
        self.progress.as_ref().map(|p| p.rounds())
    }

    /// Tells a running job to stop; its answer is dropped.
    pub(crate) fn stop(&mut self) {
        if let Some(p) = self.progress.take() {
            p.cancel();
        }
    }

    /// A lock: the running job stops and the answers go.
    pub(crate) fn clear(&mut self) {
        self.stop();
        self.sealed = None;
        self.opened = None;
    }

    pub(crate) fn sealed_done(&mut self, done: SealDone) {
        self.progress = None;
        self.sealed = Some(done.sealed);
    }

    pub(crate) fn opened_done(&mut self, done: OpenDone) {
        self.progress = None;
        self.opened = Some(done.opened);
    }

    pub(crate) fn sealed(&self) -> Option<&Result<Vec<u8>, SealError>> {
        self.sealed.as_ref()
    }

    pub(crate) fn opened(&self) -> Option<&Result<Opened, OpenError>> {
        self.opened.as_ref()
    }

    pub(crate) fn take_sealed(&mut self) -> Option<Result<Vec<u8>, SealError>> {
        self.sealed.take()
    }

    pub(crate) fn take_opened(&mut self) -> Option<Result<Opened, OpenError>> {
        self.opened.take()
    }
}
