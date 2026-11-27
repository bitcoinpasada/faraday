//! The product's trait and what it is lent (05 §1).

use core::borrow::Borrow;

use faraday_atoms::{Pressed, Ui};
use faraday_kernel::{
    AtLock, Carried, Ctx, Effects, Facts, Feature, Holds, Mode, Never, Notice, Public, Refused,
    Show, Top, View, Worded, session, shared, vaults,
};
use faraday_port as port;
use faraday_port::event::{CameraFact, DevicesFact, HostFact, SystemFact};

use crate::queue::Part;
use crate::runtime::Built;

/// What a product is made of, as the runtime sees it. Its value carries the product's Options.
pub trait Root: Sized + 'static {
    type Settings: shared::Owner<Out = Top, State: Borrow<faraday_kernel::shared::Settings>>;
    type Session: shared::Owner<Out = Top, State: session::Read>;
    type Vaults: shared::Owner<Out = Top, State: vaults::Read>;
    /// The frame and the product's composite (06 §4).
    type App: Feature<Out = Top>;
    /// The Starter's own composite, outside the lock scope (§7.9); `Nothing` elsewhere.
    type Outside: Feature<Out = Top>;
    /// One variant per top state's press (kit atoms, 04 §2.1).
    type Press: Pressed;
    type Options: Public;

    /// The mode the scope is built in: the carried one, or the root's own (§7.5).
    fn mode(&self, carried: &mut Carried, facts: &Facts) -> Mode;
    /// A press of the last frame, or a laid fact's message, addressed to its top state.
    fn press(press: Self::Press) -> Addr<Self>;
    /// Input the last frame did not take (Back, Forward, a key with no focus).
    fn input(&self, tops: &Tops<'_, Self>, input: &port::event::Input) -> Option<Addr<Self>>;
    /// A stream fact (devices, camera frames and codes, a host's drop, a write's progress).
    fn fact(&self, tops: &Tops<'_, Self>, fact: Stream) -> Option<Addr<Self>>;
    /// The window's close button, as a message (desktop roots).
    fn close_asked(&self) -> Option<Addr<Self>>;
    /// Present then draw (04 §1, V5): the app's frame and the page its navigation shows,
    /// presented from the top state that holds it (an owner's page from `tops`).
    fn frame(
        &self,
        tops: &Tops<'_, Self>,
        view: &View<'_>,
        show: &Show<'_>,
        ui: &mut Ui<'_, Self::Press>,
    );
}

/// The top states, lent read-only. `None`: the Starter's lock scope is not built.
pub struct Tops<'a, R: Root> {
    settings: &'a OwnerState<R::Settings>,
    session: Option<&'a OwnerState<R::Session>>,
    vaults: Option<&'a OwnerState<R::Vaults>>,
    app: Option<&'a <R::App as Feature>::State>,
    outside: &'a <R::Outside as Feature>::State,
}

impl<'a, R: Root> Tops<'a, R> {
    /// The runtime's: the top states as built, `None` before the first `Start` has built the
    /// part outside the scope.
    pub(crate) fn lend(built: &'a Built<R>) -> Option<Tops<'a, R>> {
        Some(Tops {
            settings: built.settings.as_ref()?,
            session: built.session.as_ref(),
            vaults: built.vaults.as_ref(),
            app: built.app.as_ref(),
            outside: built.outside.as_ref()?,
        })
    }

    pub fn settings(&self) -> &'a OwnerState<R::Settings> {
        self.settings
    }

    pub fn session(&self) -> Option<&'a OwnerState<R::Session>> {
        self.session
    }

    pub fn vaults(&self) -> Option<&'a OwnerState<R::Vaults>> {
        self.vaults
    }

    pub fn app(&self) -> Option<&'a <R::App as Feature>::State> {
        self.app
    }

    pub fn outside(&self) -> &'a <R::Outside as Feature>::State {
        self.outside
    }
}

/// A message addressed to one top state. The variant names the part it is for.
pub enum Addr<R: Root> {
    Settings(<R::Settings as Feature>::Msg),
    Session(<R::Session as Feature>::Msg),
    Vaults(<R::Vaults as Feature>::Msg),
    App(<R::App as Feature>::Msg),
    Outside(<R::Outside as Feature>::Msg),
}

impl<R: Root> Addr<R> {
    /// The top state this message is for.
    pub(crate) fn part(&self) -> Part {
        match self {
            Addr::Settings(_) => Part::Settings,
            Addr::Session(_) => Part::Session,
            Addr::Vaults(_) => Part::Vaults,
            Addr::App(_) => Part::App,
            Addr::Outside(_) => Part::Outside,
        }
    }
}

/// A port fact that goes to one feature, not into `Facts` (§2.2).
pub enum Stream {
    Devices(DevicesFact),
    Camera(CameraFact),
    Host(HostFact),
    System(SystemFact),
}

/// Whether the root has a part outside the lock scope: its `Outside` is not `Nothing` (the
/// Starter, §7.9).
pub(crate) fn has_outside<R: Root>() -> bool {
    core::any::type_name::<R::Outside>() != core::any::type_name::<Nothing>()
}

/// A top state's state type, by its feature.
pub type OwnerState<F> = <F as Feature>::State;

/// The runtime's feature with no state, no message and no page: the `Outside` of every root but
/// the Starter.
pub struct Nothing;

/// `Nothing`'s state.
pub struct Empty;

/// `Nothing`'s refusal: it refuses nothing, as it has no message.
#[derive(Public)]
pub enum NoRefusal {}

impl AtLock for Empty {
    fn holds(&self) -> Holds {
        Holds {
            kept: false,
            typed: false,
            session: false,
            asks_first: None,
        }
    }

    fn hide(&mut self) {}

    fn at_lock(&mut self) {}
}

impl Worded for NoRefusal {
    fn words(&self, _show: &Show<'_>) -> faraday_kernel::Line {
        match *self {}
    }
}

impl Refused for NoRefusal {}

impl Feature for Nothing {
    type State = Empty;
    type Msg = Never;
    type Out = Top;
    type Refusal = NoRefusal;

    fn start(_carried: &mut Carried, _cx: &Ctx<'_>) -> (Empty, Effects<Never, Top>) {
        (Empty, Effects::none())
    }

    fn update(_state: &mut Empty, msg: Never, _cx: &Ctx<'_>) -> Effects<Never, Top> {
        match msg {}
    }

    fn notice(_state: &mut Empty, _notice: &Notice<'_>, _cx: &Ctx<'_>) -> Effects<Never, Top> {
        Effects::none()
    }

    fn refuses(_state: &Empty, msg: &Never, _view: &View<'_>) -> Option<NoRefusal> {
        match *msg {}
    }
}
