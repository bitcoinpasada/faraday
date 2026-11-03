//! Opening a spend from an Inbox PSBT: the read and the inspection are a
//! job on the worker (`docs/WORKERS.md` §2.1, §6).
//!
//! The job is given the file's bytes and the public inputs of the
//! inspection (the network, the key references and the wallets' policies,
//! as [`faraday_keys::job::SignJob`] owns them): no key and no secret goes
//! with it. Its answer brings back the PSBT, the version 2 form where the
//! file was one, and the inspection, and opens the spend as the press did.
//! Back or a lock meanwhile drops the answer, and so does an Inbox that no
//! longer holds the file the job was given, and so does a person who is no
//! longer where they pressed (a stick plugged in, a sheet opened): nothing
//! opens behind their back.

use crate::strings::EN;
use crate::wallet::{self, InspectInputs, Spend, SpendState};
use crate::worker::Job;
use crate::{Faraday, Screen, Sheet};
use faraday_bip::keys::Network;
use faraday_bip::policy::WalletPolicy;
use faraday_crypto::work::Compute;
use faraday_psbt::v2::PsbtV2;
use faraday_psbt::{Inspection, KeyRef, Psbt};

/// Where the press that opened the spend goes on from the answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Then {
    /// The Spend tab.
    Spend,
    /// The Family tab, which uses the spend without leaving.
    Family,
}

/// A file to read and inspect on the worker: its bytes and the public
/// inputs of the inspection, owned.
pub(crate) struct ReadWork {
    bytes: Vec<u8>,
    network: Network,
    refs: Vec<KeyRef>,
    policies: Vec<WalletPolicy>,
    /// The session's inspection inputs when the press was made.
    inputs: InspectInputs,
    from: Origin,
}

/// The press the job belongs to.
pub(crate) struct Origin {
    /// The Inbox file the job was given, by place and name.
    index: usize,
    name: String,
    then: Then,
    /// The screen and sheet the person pressed on.
    screen: Screen,
    sheet: Option<Sheet>,
}

/// What the job read.
struct Read {
    psbt: Psbt,
    v2: Option<PsbtV2>,
    inspection: Inspection,
    /// The inputs the inspection was made with, the network being the one
    /// it was made on.
    inputs: InspectInputs,
}

/// A read's answer.
pub(crate) struct ReadDone {
    /// The file's bytes as the job was given them.
    bytes: Vec<u8>,
    /// `None` where the file holds no PSBT.
    read: Option<Read>,
    from: Origin,
}

impl ReadWork {
    /// Costs what `read_psbt_kept` and `faraday_psbt::inspect` cost. The
    /// file read as a PSBT and inspected, as the press did on the main
    /// thread. A PSBT of another kind of network than the session's is
    /// inspected on its own, as `Session::follow_psbt` leaves a session
    /// with no key and no wallet; the answer is applied only where the
    /// session took that network.
    pub(crate) fn run(self, c: &Compute) -> ReadDone {
        let Self {
            bytes,
            network,
            refs,
            policies,
            inputs,
            from,
        } = self;
        let read = wallet::read_psbt_kept(c, &bytes).map(|(psbt, v2)| {
            let on = match wallet::psbt_network(&psbt, network) {
                Some(n) if n.kind() != network.kind() => n,
                _ => network,
            };
            let ctx = wallet::context(on, &refs, &policies);
            let inspection = crate::review::name_scripts(faraday_psbt::inspect(c, &psbt, &ctx));
            Read {
                psbt,
                v2,
                inspection,
                inputs: inputs.on(on),
            }
        });
        ReadDone { bytes, read, from }
    }
}

/// Costs a key derivation per key (`key_refs`). Starts the read of
/// Files' PSBT `index` on the worker; `then` is where the answer goes
/// on. The key references are the session's public ones.
pub(crate) fn start_spend(app: &mut Faraday, index: usize, then: Then) {
    let c = app.compute();
    let Some((name, bytes)) = app
        .inbox
        .get(index)
        .map(|i| (i.name.clone(), i.bytes.clone()))
    else {
        return;
    };
    let work = ReadWork {
        bytes,
        network: app.session.network(),
        refs: app.session.key_refs(&c),
        policies: app.session.policies(),
        inputs: app.session.inspect_inputs(),
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
    let ReadDone { bytes, read, from } = done;
    let held = app
        .inbox
        .get(from.index)
        .is_some_and(|i| i.name == from.name && i.bytes == bytes);
    if !held || (app.screen, app.sheet) != (from.screen, from.sheet) {
        return;
    }
    let opened = spend_opened(app, read, &from.name);
    match from.then {
        Then::Spend if opened => app.screen = Screen::Spend,
        Then::Spend => {}
        Then::Family => app.family_used_psbt(opened),
    }
}

/// Costs what `inspect` costs where the session's inputs are no longer the
/// job's (the network it took, the keys, the wallets), nothing otherwise. Opens the spend
/// the job read. Whether it opened.
fn spend_opened(app: &mut Faraday, read: Option<Read>, name: &str) -> bool {
    let Some(Read {
        psbt,
        v2,
        inspection,
        inputs,
    }) = read
    else {
        app.toast(EN.start_spend_not_a_psbt);
        return false;
    };
    if let Err(e) = app.session.follow_psbt(&psbt) {
        app.toast(&e.text());
        return false;
    }
    let spend = Spend::new_kept(psbt, v2, name);
    let now = app.session.inspect_inputs();
    let inspection = if now == inputs {
        inspection
    } else {
        let c = app.compute();
        app.session.inspect(&c, &spend.psbt)
    };
    let mut state = SpendState::new(spend, inspection, now);
    state.done[0] = true;
    app.wallets.set_spend(state);
    app.refresh_spend();
    true
}
