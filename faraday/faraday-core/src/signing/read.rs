//! Opening a spend from an Inbox PSBT: the read and the inspection are a
//! job on the worker (`docs/WORKERS.md` §2.1, §6).
//!
//! The job is given the file's bytes and a copy of the session
//! ([`crate::wallet::Session::job_copy`]): the keys' masters shared the way
//! a signing job shares them, the wallets with what they kept, and no
//! words or seeds. It reads the file, follows its network on the copy and
//! works the spend out as a refresh does; the copy is dropped on the worker
//! before the answer is sent. Its answer brings back the spend, what was
//! worked out of it and the wallets' kept scripts, and opens the spend as
//! the press did.
//! Back or a lock meanwhile drops the answer, and so does an Inbox that no
//! longer holds the file the job was given, and so does a person who is no
//! longer where they pressed (a stick plugged in, a sheet opened): nothing
//! opens behind their back.

use super::fresh::{Fresh, worked_out};
use crate::strings::EN;
use crate::wallet::{self, Session, Spend, SpendState, Wallet};
use crate::worker::Job;
use crate::{Faraday, Screen, Sheet};
use faraday_crypto::work::Compute;
use faraday_text::Text;
use zeroize::Zeroizing;

/// Where the press that opened the spend goes on from the answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Then {
    /// The Spend tab.
    Spend,
    /// The Spend tab, which uses the spend without leaving.
    SpendBackup,
}

/// A file to read and work out on the worker: its bytes and a copy of the
/// session, owned.
pub(crate) struct ReadWork {
    bytes: Zeroizing<Vec<u8>>,
    session: Session,
    from: Origin,
}

/// The press the job belongs to.
pub(crate) struct Origin {
    /// The Inbox file the job was given, by place and name.
    index: usize,
    name: Text,
    then: Then,
    /// The screen and sheet the person pressed on.
    screen: Screen,
    sheet: Option<Sheet>,
}

/// What the job read: the spend, worked out for the session as the copy
/// was.
struct Read {
    spend: Spend,
    fresh: Fresh,
    /// The wallets as the job left them, for what they kept.
    wallets: Vec<Wallet>,
}

/// A read's answer.
pub(crate) struct ReadDone {
    /// The file's bytes as the job was given them.
    bytes: Zeroizing<Vec<u8>>,
    /// `None` where the file holds no PSBT.
    read: Option<Read>,
    /// The file is a PSBT with silent payment fields, which is not read.
    silent: bool,
    from: Origin,
}

impl ReadWork {
    /// Costs what `read_psbt_kept` and `worked_out` cost, and a second parse
    /// of the file in `is_silent_payment_psbt` when it is not read. The file
    /// read as a PSBT and worked out as a refresh does, on the copy of the session
    /// after it took the PSBT's network as `Session::follow_psbt` does; where
    /// the session does not take it, or is not as the copy was at the apply,
    /// the apply works the spend out again.
    pub(crate) fn run(self, c: &Compute) -> ReadDone {
        let Self {
            bytes,
            mut session,
            from,
        } = self;
        let read = match wallet::read_psbt_kept(c, &bytes) {
            Some((psbt, v2)) => {
                // The refusal, if any, is the apply's to say.
                let _ = session.follow_psbt(&psbt);
                let mut spend = Spend::new_kept(psbt, v2, &from.name);
                let fresh = worked_out(c, &session, &mut spend, None, false);
                // No key's `Arc` rides in the answer: the keys go before it does.
                let wallets = std::mem::take(&mut session.wallets);
                drop(session);
                Some(Read {
                    spend,
                    fresh,
                    wallets,
                })
            }
            None => None,
        };
        let silent = read.is_none() && wallet::is_silent_payment_psbt(c, &bytes);
        ReadDone {
            bytes,
            read,
            silent,
            from,
        }
    }
}

/// Starts the read of Files' PSBT `index` on the worker; `then` is where
/// the answer goes on.
pub(crate) fn start_spend(app: &mut Faraday, index: usize, then: Then) {
    let Some((name, bytes)) = app
        .inbox
        .get(index)
        .map(|i| (Text::of(&i.name), i.bytes.clone()))
    else {
        return;
    };
    let work = ReadWork {
        bytes,
        session: app.session.job_copy(),
        from: Origin {
            index,
            name,
            then,
            screen: app.screen,
            sheet: app.sheet,
        },
    };
    app.start(Job::SpendRead(Box::new(work)), EN.working_reading_psbt);
}

/// Costs what [`spend_opened`] costs. A read's answer: opens the spend,
/// or says why not. Dropped where
/// the Inbox no longer holds the file the job was given or the person is
/// no longer on the screen and sheet they pressed on.
pub(crate) fn spend_read(app: &mut Faraday, done: ReadDone) {
    let ReadDone {
        bytes,
        read,
        silent,
        from,
    } = done;
    let held = app
        .inbox
        .get(from.index)
        .is_some_and(|i| i.name == *from.name && i.bytes == bytes);
    if !held || (app.screen, app.sheet) != (from.screen, from.sheet) {
        return;
    }
    let opened = spend_opened(app, read, silent);
    match from.then {
        Then::Spend if opened => app.screen = Screen::Spend,
        Then::Spend => {}
        Then::SpendBackup => app.spend_backup_used_psbt(opened),
    }
}

/// Costs what `worked_out` costs where the session's inputs are no longer
/// the job's (the network it took, the keys, the wallets). Opens the spend
/// the job read, a MuSig2 file holding every partial signature aggregated at
/// once. Whether it opened.
fn spend_opened(app: &mut Faraday, read: Option<Read>, silent: bool) -> bool {
    let Some(Read {
        spend,
        fresh,
        wallets,
    }) = read
    else {
        app.toast(if silent {
            EN.start_spend_silent_payments
        } else {
            EN.start_spend_not_a_psbt
        });
        return false;
    };
    if let Err(e) = app.session.follow_psbt(&spend.psbt) {
        app.toast(&e.text());
        return false;
    }
    for (kept, theirs) in app.session.wallets.iter().zip(wallets) {
        kept.take_kept(theirs);
    }
    // The opening is the spend's first read: a file with every partial in
    // it is aggregated, the signature kept only if it verifies.
    let mut state = if app.session.inspect_inputs() == fresh.inputs {
        SpendState::opened_with(spend, fresh)
    } else {
        let c = app.compute();
        let mut spend = spend;
        let fresh = worked_out(&c, &app.session, &mut spend, None, false);
        SpendState::opened_with(spend, fresh)
    };
    state.done[0] = true;
    app.wallets.set_spend(state);
    app.refresh_spend();
    true
}
