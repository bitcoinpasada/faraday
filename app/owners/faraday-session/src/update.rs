use faraday_kernel::session::{self, Change, NoRandomness};
use faraday_kernel::shared;
use faraday_kernel::{
    AnyChange, Carried, ChangeId, Ctx, Done, Effects, FactsChanged, Feature, Notice, Owner, Public,
    Top, View,
};
use faraday_port::Failure;
use faraday_port::ask::Entropy as AskEntropy;
use faraday_port::bytes::Entropy;

use crate::random::{POOL, TAKE_MAX, answers};
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
        if cx.view.mode.practice {
            state.store.practice(cx.view.mode.samples);
        }
        // The system's answers are asked for at the start: the first is the seed, the rest the
        // pool (F102; `main` asked for one more than the pool holds).
        state.random.begin(cx.view.clock.now_ms);
        state.random.asked(POOL + 1);
        let mut fx = Effects::none();
        for _ in 0..=POOL {
            fx = fx.ask(AskEntropy, false, came);
        }
        (state, fx)
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
            answer(id, session::Done::Seed(())).and(serve(state))
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

/// A draw of `what`: `n` bytes, one pooled answer for every 32, from the pool once the seed is in
/// (the seed comes first), else kept until the system's answers are. More than the pool's most
/// is refused, and so is any draw once the seed is a minute late with the system giving none.
fn draw_for(
    state: &mut State,
    id: ChangeId,
    draw: session::Draw,
    cx: &Ctx<'_>,
) -> Effects<Msg, Out> {
    if draw.n > TAKE_MAX {
        return answer(id, session::Done::Draw(Err(NoRandomness::TooLong)));
    }
    let now = cx.view.clock.now_ms;
    if state.random.gives_none(now) {
        return answer(id, session::Done::Draw(Err(NoRandomness::System)));
    }
    if !state.random.is_waiting()
        && let Some(bytes) = state.random.take_bytes(draw.what, draw.n)
    {
        return answer(id, session::Done::Draw(Ok(bytes))).and(ask_more(state, answers(draw.n)));
    }
    state.random.wait(id, draw.what, draw.n, now);
    ask_more(state, 0)
}

/// The adapter is asked for the answers the draws waiting still need, and `extra` to refill
/// what a draw took.
fn ask_more(state: &mut State, extra: usize) -> Effects<Msg, Out> {
    let n = state.random.wanted() + extra;
    state.random.asked(n);
    let mut fx = Effects::none();
    for _ in 0..n {
        fx = fx.ask(AskEntropy, false, came);
    }
    fx
}

fn came(_context: bool, answer: Result<Entropy, Failure>) -> Msg {
    Msg::Entropy(answer)
}

/// The draws waiting that the pool can serve now, each answered; the answers they took are asked
/// for again, and what the draws still waiting need.
fn serve(state: &mut State) -> Effects<Msg, Out> {
    let mut fx = Effects::none();
    let mut taken = 0;
    while let Some((id, bytes, used)) = state.random.serve() {
        taken += used;
        fx = fx.and(answer(id, session::Done::Draw(Ok(bytes))));
    }
    fx.and(ask_more(state, taken))
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
    serve(state)
}

/// A tick: the draws that waited as long as they do are answered with no randomness, and the
/// ones left ask again when nothing is on its way.
fn waited(state: &mut State, cx: &Ctx<'_>) -> Effects<Msg, Out> {
    let mut fx = Effects::none();
    for id in state.random.expired(cx.view.clock.now_ms) {
        fx = fx.and(answer(id, session::Done::Draw(Err(NoRandomness::System))));
    }
    if state.random.in_flight() == 0 {
        fx = fx.and(ask_more(state, 0));
    }
    fx
}
