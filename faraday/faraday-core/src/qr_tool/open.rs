//! The QR vault tool's open on the worker (`docs/VAULT.md` §13 Open,
//! `docs/WORKERS.md` §2.1): the job is made by `press` on the main thread
//! after the code's stated cost is checked, and runs under the busy
//! screen; its answer is held in `faraday-keys` or says why there is no
//! item. Back, Cancel, a lock or the idle lock make the answer stale and it
//! is dropped where it arrives, and wiped as it drops.

use faraday_crypto::work::Compute;
use faraday_keys::{QrError, QrOpenJob, QrOpened, QrUnsupported, qr_cost};

use super::{Field, refusal_line, say, st};
use crate::strings::EN;
use crate::worker::Job;
use crate::{Faraday, Screen};

/// A code to open, to run on the worker.
pub(crate) struct OpenWork {
    job: QrOpenJob,
}

/// An open's answer: the item or why there is none.
pub(crate) struct OpenDone {
    opened: Result<QrOpened, QrError>,
}

impl OpenWork {
    /// Costs one Argon2id run at the code's stated cost: the job's own run.
    pub(crate) fn run(self, c: &Compute) -> OpenDone {
        OpenDone {
            opened: self.job.run(c),
        }
    }
}

/// The line an open's refusal is told in.
fn open_line(e: &QrError) -> String {
    match e {
        QrError::WrongPassphrase => EN.qr_tool_wrong_pass.to_string(),
        QrError::NotQrVault(QrUnsupported::Other) | QrError::NotAMessage => {
            EN.qr_tool_not_message.to_string()
        }
        QrError::NotQrVault(_) => EN.qr_tool_not_vault.to_string(),
        QrError::Cost => EN.qr_tool_cost.to_string(),
        QrError::Damaged => EN.qr_tool_damaged.to_string(),
        QrError::Contents(_) | QrError::Unreadable | QrError::Memory(_) => {
            EN.qr_tool_not_opened.to_string()
        }
        other => refusal_line(other),
    }
}

/// The Open press: the code's stated cost is checked against this computer
/// at once, with no Progress, and a code Faraday does not open is named;
/// then the job runs under the busy screen.
pub(super) fn press(app: &mut Faraday) {
    let Some(s) = st(app) else { return };
    let Some(code) = s.code.clone() else { return };
    if s.pass.text.is_empty() || s.holds.is_some() {
        return;
    }
    let cost = match qr_cost(&code) {
        Ok(cost) => cost,
        Err(e) => return say(app, open_line(&e)),
    };
    // Held to what is free now, as an unlock is.
    app.vaults.read_free();
    if let Some(line) = app
        .vaults
        .open_refusal(cost.memory_kib() / 1024, code.len())
    {
        return say(app, line);
    }
    let Some(s) = st(app) else { return };
    s.error = None;
    let job = QrOpenJob::new(&code, s.pass.text.as_str());
    app.start(Job::QrOpen(Box::new(OpenWork { job })), EN.qr_tool_opening);
}

/// An open's answer: the item is held and what was typed goes, or the
/// refusal is told and the passphrase is typed again. Dropped where the
/// tool is no longer open.
pub(crate) fn opened(app: &mut Faraday, done: OpenDone) {
    let OpenDone { opened } = done;
    if app.screen != Screen::QrTool || st(app).is_none() {
        return;
    }
    match opened {
        Ok(item) => {
            let holds = app.session.keys.qr_hold(item);
            if let Some(s) = st(app) {
                s.wipe_typed();
                s.holds = Some(holds);
                s.shown = false;
                s.error = None;
                s.focus = None;
            }
        }
        Err(e) => {
            if let Some(s) = st(app) {
                s.wipe_typed();
                s.focus = Some(Field::Pass);
                s.error = Some(open_line(&e));
            }
        }
    }
}

/// Back, Cancel or a move away while the open runs: the passphrase is
/// wiped and nothing is opened. The answer is dropped where it arrives.
pub(crate) fn open_stopped(app: &mut Faraday) {
    if app
        .busy()
        .is_none_or(|b| b.kind != crate::worker::Kind::QrOpen)
    {
        return;
    }
    app.session.keys.qr_forget();
    if let Some(s) = st(app) {
        s.wipe_typed();
        s.holds = None;
        s.focus = Some(Field::Pass);
    }
}
