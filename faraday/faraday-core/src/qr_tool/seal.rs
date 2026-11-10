//! The QR vault tool's seal on the worker (`docs/VAULT.md` §13 Make,
//! `docs/WORKERS.md` §2.1): the job is made by `make` on the main thread
//! and runs under the busy screen; its answer makes the code or says why
//! there is none. Back, Cancel, a lock or the idle lock make the answer
//! stale and it is dropped where it arrives, and wiped as it drops.

use faraday_crypto::work::Compute;
use faraday_keys::{QrError, QrSealJob};

use super::{Made, qstep, refusal_line, st};
use crate::strings::EN;
use crate::worker::Job;
use crate::{Faraday, Screen};

/// A code to seal, to run on the worker, with the name it goes by.
pub(crate) struct SealWork {
    job: QrSealJob,
    stem: String,
}

/// A seal's answer: the code made or why there is none.
pub(crate) struct SealDone {
    sealed: Result<Vec<u8>, QrError>,
    stem: String,
}

impl SealWork {
    /// Costs an Argon2id run at the memory step chosen, and one to read
    /// the seal back: the job's own run.
    pub(crate) fn run(self, c: &Compute) -> SealDone {
        SealDone {
            sealed: self.job.run(c),
            stem: self.stem,
        }
    }
}

/// Starts sealing `job`, the code to go by `stem`, under the busy screen.
pub(super) fn start(app: &mut Faraday, job: QrSealJob, stem: String) {
    app.start(
        Job::QrSeal(Box::new(SealWork { job, stem })),
        EN.qr_tool_sealing,
    );
}

/// A seal's answer: the code is made and what was typed goes, or the
/// refusal is told and what was typed stays. Dropped where the tool is no
/// longer open.
pub(crate) fn sealed(app: &mut Faraday, done: SealDone) {
    let SealDone { sealed, stem } = done;
    if app.screen != Screen::QrTool {
        return;
    }
    let date = app.clock_now().map(crate::gpg::date);
    let Some(s) = st(app) else { return };
    match sealed {
        Ok(bytes) => {
            s.wipe_typed();
            s.made = Some(Made { bytes, stem, date });
            s.done[usize::from(qstep::PASSPHRASE)] = true;
            s.done[usize::from(qstep::CODE)] = true;
            s.open = Some(qstep::CODE);
            s.focus = None;
            s.scroll.follow = true;
        }
        Err(e) => s.error = Some(refusal_line(&e)),
    }
}

/// Back, Cancel or a move away while the seal runs: what was typed is
/// wiped, and the tool is at the Contents card with no note for a note,
/// at the passphrase card for a loaded key, with no code. The
/// answer is dropped where it arrives.
pub(crate) fn seal_stopped(app: &mut Faraday) {
    if app
        .busy()
        .is_none_or(|b| b.kind != crate::worker::Kind::QrSeal)
    {
        return;
    }
    if let Some(s) = st(app) {
        s.wipe_typed();
        s.made = None;
        if s.pick == Some(super::Pick::Note) {
            // The note is gone with the rest: it is typed again.
            s.done = [false; qstep::COUNT];
            s.open = Some(qstep::CONTENTS);
            s.focus = Some(super::Field::Note);
        } else {
            s.done[usize::from(qstep::PASSPHRASE)] = false;
            s.done[usize::from(qstep::CODE)] = false;
            s.open = Some(qstep::PASSPHRASE);
            s.focus = Some(super::Field::Pass);
        }
    }
}
