//! `Feature`, `Notice`, `route`, and what the top states tell the runtime (03 §2; 05 §1.1).

use crate::cx::{Ctx, FactsChanged, View};
use crate::effects::Effects;
use crate::lock::{AtLock, Carried};
use crate::message::Message;
use crate::shared;
use crate::words::Refused;

pub trait Feature {
    /// Never Clone (L1). Its fields are private to the feature's crate (A7).
    type State: AtLock;
    type Msg: Message;
    /// What the feature tells its parent (A6).
    type Out: Message;
    /// Why a message is refused before `update` sees it.
    type Refusal: Refused;

    /// The state at a build of the lock scope, from what the last build carried for this
    /// feature.
    fn start(carried: &mut Carried, cx: &Ctx<'_>) -> (Self::State, Effects<Self::Msg, Self::Out>);

    /// One message: the state changes, effects come back.
    fn update(
        state: &mut Self::State,
        msg: Self::Msg,
        cx: &Ctx<'_>,
    ) -> Effects<Self::Msg, Self::Out>;

    /// What every feature is told. No default: each feature says what it does with each notice.
    fn notice(
        state: &mut Self::State,
        notice: &Notice<'_>,
        cx: &Ctx<'_>,
    ) -> Effects<Self::Msg, Self::Out>;

    /// Whether `msg` is refused now. Called by `route` before `update`, and by `present` to draw
    /// the control that would send it.
    fn refuses(state: &Self::State, msg: &Self::Msg, view: &View<'_>) -> Option<Self::Refusal>;

    /// At a lock, after the seal and before `at_lock`: put what the next build's `start` needs
    /// into the carry. Most features carry nothing.
    fn carry(_state: &mut Self::State, _into: &mut Carried) {}
}

pub enum Notice<'a> {
    /// None of this feature's pages is shown any more: a press elsewhere, Back, Home, a sheet's
    /// Cancel. Drop what only a screen needed.
    Left,
    /// Take secrets off the screen now (`AtLock::hide` is called after).
    Hide,
    /// A key, wallet, file or vault came, went or changed.
    Shared(&'a shared::Changed),
    /// A fact in `View::facts` changed: sticks, display, cameras, time.
    Facts(FactsChanged),
}

/// How a composite or root passes a message to a child.
///
/// A refused message gives one note, the refusal's words, and nothing else (A9); otherwise it
/// reaches `update`.
pub fn route<F: Feature>(
    state: &mut F::State,
    msg: F::Msg,
    cx: &Ctx<'_>,
) -> Effects<F::Msg, F::Out> {
    match F::refuses(state, &msg, &cx.view) {
        Some(refusal) => Effects::none().note(refusal),
        None => F::update(state, msg, cx),
    }
}

/// In the kernel's `feature` module: what the top states tell the runtime.
#[derive(Message)]
pub enum Top {
    /// Every secret on show off the screen now: the idle warning.
    Hide,
    /// What an owner holds came, went or changed; broadcast as `Notice::Shared`.
    Shared(shared::Changed),
    /// Begin a lock of the scope.
    Lock(Lock),
    /// The settings owner asks for a mode change: the root shows the Lock sheet, which always
    /// asks for it, and the lock it then begins seals. Carries the mode asked, as the number the
    /// settings owner carries it by (`Mode::index`: 0 Practice, 1 Mainnet, 2 Testnet, 3 Signet,
    /// 4 Regtest).
    ModeLock(u8),
    /// The Lock sheet that asked for a mode change closed without its Lock: the settings owner
    /// gives the asked mode up.
    ModeCancel,
    /// The app's navigation left the settings owner's page (06 §4.4): the settings owner is given
    /// `Notice::Left`, as a feature whose page was left is by the navigation (A13).
    SettingsLeft,
    /// End the process, or close the Starter's scope.
    End(End),
    /// Run the self-test again (Settings' Run again).
    SelfTest,
    /// The Starter only: build the lock scope, after `PrepareDemoSticks` answers `Ok`.
    Open,
}

/// `seal: false` is "lock without saving".
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Lock {
    pub seal: bool,
    pub idle: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum End {
    PowerOff,
    RestartComputer,
    Quit,
}
