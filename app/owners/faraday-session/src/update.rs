use faraday_kernel::session::{self, Change, NoRandomness};
use faraday_kernel::shared;
use faraday_kernel::{
    AnyChange, Carried, ChangeId, Ctx, Done, Effects, FactsChanged, Feature, Notice, Owner, Public,
    Top, View,
};
use faraday_port::Failure;
use faraday_port::ask::Entropy as AskEntropy;
use faraday_port::bytes::Entropy;

use crate::store::stick_in_the_way;
use crate::{Msg, Out, Session, State};

/// Why a message is refused before `update` sees it.
#[derive(Public)]
pub enum Refusal {}

impl Feature for Session {
    type State = State;
    type Msg = Msg;
    type Out = Out;
    type Refusal = Refusal;

    fn start(_carried: &mut Carried, cx: &Ctx<'_>) -> (State, Effects<Msg, Out>) {
        let mut state = State::new(cx.view.mode.network);
        state.store.set_stick(stick_in_the_way(cx.view.facts));
        (state, Effects::none())
    }

    fn update(state: &mut State, msg: Msg, cx: &Ctx<'_>) -> Effects<Msg, Out> {
        match msg {
            Msg::Press(press) => match press {},
            Msg::Given(id, change) => given(state, id, change, cx),
            Msg::Entropy(answer) => entropy(state, answer, cx),
        }
    }

    fn notice(state: &mut State, notice: &Notice<'_>, cx: &Ctx<'_>) -> Effects<Msg, Out> {
        match notice {
            Notice::Facts(FactsChanged::Sticks) => {
                state.store.set_stick(stick_in_the_way(cx.view.facts));
                Effects::none()
            }
            Notice::Facts(FactsChanged::Time) => waited(state, cx),
            _ => Effects::none(),
        }
    }

    fn refuses(_state: &State, msg: &Msg, _view: &View<'_>) -> Option<Refusal> {
        match msg {
            Msg::Press(press) => match *press {},
            Msg::Given(..) | Msg::Entropy(..) => None,
        }
    }

    /// A key held for its backup is the only copy of itself: the next build is told.
    fn carry(state: &mut State, into: &mut Carried) {
        if state.store.any_held() {
            into.mark_only_copy();
        }
    }
}

impl Owner for Session {
    fn given(id: ChangeId, change: AnyChange) -> Msg {
        Msg::Given(id, change)
    }
}

fn said(change: session::Changed) -> Out {
    Top::Shared(shared::Changed::Session(change))
}

fn answer(id: ChangeId, done: session::Done) -> Effects<Msg, Out> {
    Effects::none().done(id, Done::Session(done))
}

/// One change given to the session, answered once on every path (A22).
fn given(state: &mut State, id: ChangeId, change: AnyChange, cx: &Ctx<'_>) -> Effects<Msg, Out> {
    let AnyChange::Session(change) = change else {
        // Another owner's change is not this owner's to answer.
        return Effects::none();
    };
    match change {
        Change::AddKey(add) => {
            let added = state.store.add(cx.compute, add.key, add.passphrase);
            let came = added.as_ref().ok().copied();
            let mut fx = answer(id, session::Done::AddKey(added));
            if let Some(key) = came {
                state.changed();
                fx = fx.out(said(session::Changed::KeyCame(key)));
            }
            fx
        }
        Change::RemoveKey(remove) => {
            let went = state.store.remove(remove.0);
            let mut fx = answer(id, session::Done::RemoveKey(Ok(())));
            if went {
                state.changed();
                fx = fx.out(said(session::Changed::KeyGone(remove.0)));
            }
            fx
        }
        Change::RemoveAll(_) => {
            let gone = state.store.remove_all();
            let mut fx = answer(id, session::Done::RemoveAll(Ok(())));
            if !gone.is_empty() {
                state.changed();
            }
            for key in gone {
                fx = fx.out(said(session::Changed::KeyGone(key)));
            }
            fx
        }
        Change::Seed(seed) => {
            let new_seed = state.random.arrived(seed.0.bytes.expose());
            state.changed();
            if new_seed {
                state.reblind(cx.compute);
            }
            answer(id, session::Done::Seed(())).and(serve(state, cx))
        }
        Change::AddOwn(own) => {
            state.random.own_add(own.0.expose());
            state.changed();
            state.reblind(cx.compute);
            answer(id, session::Done::AddOwn(()))
        }
        Change::Draw(draw) => draw_for(state, id, draw, cx),
        Change::Hold(hold) => {
            state.store.hold(hold.0);
            state.changed();
            answer(id, session::Done::Hold(()))
        }
        Change::Release(release) => {
            let released = state.store.release(release.0);
            let mut fx = answer(id, session::Done::Release(()));
            if released {
                state.changed();
                fx = fx.out(said(session::Changed::HeldReleased(release.0)));
            }
            fx
        }
        // The wallets, the PSBTs to sign, the files held and the network are not built yet:
        // their changes are not answered here until they are.
        Change::AddWallet(_)
        | Change::ConfirmWallet(_)
        | Change::RemoveWallet(_)
        | Change::RenameWallet(_)
        | Change::MarkChecked(_)
        | Change::Follow(_)
        | Change::Sign(_)
        | Change::PutOutbox(_)
        | Change::TakeFile(_)
        | Change::RemoveFile(_)
        | Change::RenameFile(_)
        | Change::MarkWritten(_) => Effects::none(),
    }
}

/// A draw of `what`: one answer of 32 bytes, from the pool once the seed is in (the seed comes
/// first), else kept until the system's answers are. A draw of more than one answer is not
/// given: the answer type is one answer, so a caller needing more draws again.
fn draw_for(
    state: &mut State,
    id: ChangeId,
    draw: session::Draw,
    cx: &Ctx<'_>,
) -> Effects<Msg, Out> {
    if draw.n == 0 || draw.n > 32 {
        return answer(id, session::Done::Draw(Err(NoRandomness)));
    }
    if !state.random.is_waiting()
        && let Some(bytes) = state.random.take(draw.what)
    {
        return answer(id, session::Done::Draw(Ok(bytes))).and(ask_more(state));
    }
    state.random.wait(id, draw.what, cx.view.clock.now_ms);
    ask_more(state)
}

/// The adapter is asked for one more answer, when none is on its way.
fn ask_more(state: &mut State) -> Effects<Msg, Out> {
    if state.random.in_flight() > 0 {
        return Effects::none();
    }
    state.random.asked();
    Effects::none().ask(AskEntropy, false, came)
}

fn came(_context: bool, answer: Result<Entropy, Failure>) -> Msg {
    Msg::Entropy(answer)
}

/// The draws waiting that the pool can serve now, each answered; and one more answer asked for
/// while some still wait or one was taken.
fn serve(state: &mut State, cx: &Ctx<'_>) -> Effects<Msg, Out> {
    let _ = cx;
    let mut fx = Effects::none();
    let mut taken = false;
    while let Some((id, bytes)) = state.random.serve() {
        taken = true;
        fx = fx.and(answer(id, session::Done::Draw(Ok(bytes))));
    }
    if taken || state.random.is_waiting() {
        fx = fx.and(ask_more(state));
    }
    fx
}

/// The adapter answered the session's own ask.
fn entropy(
    state: &mut State,
    answered: Result<Entropy, Failure>,
    cx: &Ctx<'_>,
) -> Effects<Msg, Out> {
    state.random.answered();
    // A failed ask gives nothing; the waiting draws try again at the next tick and give up when
    // their wait is over.
    let Ok(entropy) = answered else {
        return Effects::none();
    };
    if state.random.arrived(entropy.bytes.expose()) {
        state.reblind(cx.compute);
    }
    state.changed();
    serve(state, cx)
}

/// A tick: the draws that waited as long as they do are answered with `NoRandomness`, and the
/// ones left ask again when nothing is on its way.
fn waited(state: &mut State, cx: &Ctx<'_>) -> Effects<Msg, Out> {
    let mut fx = Effects::none();
    for id in state.random.expired(cx.view.clock.now_ms) {
        fx = fx.and(answer(id, session::Done::Draw(Err(NoRandomness))));
    }
    if state.random.is_waiting() {
        fx = fx.and(ask_more(state));
    }
    fx
}
