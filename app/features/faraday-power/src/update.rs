use faraday_kernel::{
    Carried, Ctx, Effects, End, Feature, Lock, Notice, Public, Started, Top, View,
};

use crate::{Msg, Out, Power, Press, State, Which};

/// Why a message is refused before `update` sees it. The feature refuses nothing: every press
/// on its sheets is an answer to what the sheet says.
#[derive(Public)]
pub enum Refusal {}

type Fx = Effects<Msg, Out>;

/// The sheet comes up.
fn show(state: &mut State, which: Which) -> Fx {
    state.show(Some(which));
    Effects::none().out(Out::Front(Some(which.frame())))
}

/// The sheet goes away, when one is up.
fn away(state: &mut State) -> Fx {
    match state.sheet() {
        Some(_) => {
            state.show(None);
            Effects::none().out(Out::Front(None))
        }
        None => Effects::none(),
    }
}

/// The lock begins; the runtime hides, seals when it is told to and ends the scope. Nothing is
/// asked here: the Lock sheet came before it where something would be lost.
fn lock(state: &mut State, seal: bool) -> Fx {
    state.show(None);
    Effects::none().out(Out::Top(Top::Lock(Lock { seal, idle: false })))
}

/// The end of the app: Quit in the app of someone's own system, else power off or restart. The
/// Power sheet asks first where something would be lost; with nothing held it does not.
fn end(state: &mut State, end: End) -> Fx {
    state.show(None);
    Effects::none().out(Out::Top(Top::End(end)))
}

impl Feature for Power {
    type State = State;
    type Msg = Msg;
    type Out = Out;
    type Refusal = Refusal;

    /// After a lock the next app opens on the Locked sheet; after a power-on, on nothing.
    fn start(_carried: &mut Carried, cx: &Ctx<'_>) -> (State, Fx) {
        match cx.view.started {
            Started::Lock => (
                State::new(Some(Which::Locked)),
                Effects::none().out(Out::Front(Some(Which::Locked.frame()))),
            ),
            Started::Power | Started::Rebuilt => (State::new(None), Effects::none()),
        }
    }

    fn update(state: &mut State, msg: Msg, cx: &Ctx<'_>) -> Fx {
        match msg {
            Msg::Show(which) => show(state, which),
            Msg::Lock { asks_first: true } => show(state, Which::LockAsk),
            Msg::ModeLock => {
                state.set_mode_change(true);
                show(state, Which::LockAsk)
            }
            Msg::Lock { asks_first: false } | Msg::Press(Press::Lock) => lock(state, true),
            Msg::Press(Press::LockWithoutSealing) => lock(state, false),
            Msg::Press(Press::Away) => away(state),
            Msg::Press(Press::Restart) => end(state, End::RestartComputer),
            Msg::Press(Press::PowerOff) => end(
                state,
                if cx.view.mode.online {
                    End::Quit
                } else {
                    End::PowerOff
                },
            ),
        }
    }

    /// The feature holds no secret and no screen: every notice leaves its sheet as it is.
    fn notice(_state: &mut State, _notice: &Notice<'_>, _cx: &Ctx<'_>) -> Fx {
        Effects::none()
    }

    fn refuses(_state: &State, _msg: &Msg, _view: &View<'_>) -> Option<Refusal> {
        None
    }
}

#[cfg(test)]
mod tests {
    use faraday_app_test::{TestView, seen};
    use faraday_kernel::route;

    use super::*;

    fn run(view: &TestView, state: &mut State, msg: Msg) -> faraday_app_test::Seen<Msg, Out> {
        seen(route::<Power>(state, msg, &view.ctx()))
    }

    fn ended(out: &[Out]) -> Option<End> {
        match out {
            [Out::Top(Top::End(end))] => Some(*end),
            _ => None,
        }
    }

    /// Invariant: Power off on the device ends the app by powering off, and Restart by restarting the computer, each with nothing asked first while nothing is held.
    /// Regression: Restart powers off, or either waits for a confirmation nothing needs.
    /// Owner: app
    #[test]
    fn power_off_and_restart_end_the_app_with_their_own_status() {
        let view = TestView::device();
        let mut state = State::new(Some(Which::Power));
        let off = run(&view, &mut state, Msg::Press(Press::PowerOff)).outs;
        assert_eq!(ended(&off), Some(End::PowerOff));

        let mut state = State::new(Some(Which::Power));
        let restart = run(&view, &mut state, Msg::Press(Press::Restart)).outs;
        assert_eq!(ended(&restart), Some(End::RestartComputer));
    }

    /// Invariant: In the desktop app the way out is Quit, not a power-off.
    /// Regression: Faraday Demo powers off the person's own computer.
    /// Owner: app
    #[test]
    fn the_desktop_app_quits_instead_of_powering_off() {
        let view = TestView::demo();
        let mut state = State::new(Some(Which::Power));
        let out = run(&view, &mut state, Msg::Press(Press::PowerOff)).outs;
        assert_eq!(ended(&out), Some(End::Quit));
    }

    /// Invariant: A lock with nothing to lose begins at once, sealing the open vaults, with no sheet between; one that would lose something shows the Lock sheet first and begins only when the person presses Lock there.
    /// Regression: A lock goes ahead over unsaved changes without asking, or asks when nothing is held.
    /// Owner: app
    #[test]
    fn a_lock_asks_first_only_where_something_would_be_lost() {
        let view = TestView::device();
        let mut state = State::new(None);
        let at_once = run(&view, &mut state, Msg::Lock { asks_first: false }).outs;
        assert!(matches!(
            at_once.as_slice(),
            [Out::Top(Top::Lock(Lock {
                seal: true,
                idle: false
            }))]
        ));

        let mut state = State::new(None);
        let asked = run(&view, &mut state, Msg::Lock { asks_first: true }).outs;
        assert!(matches!(
            asked.as_slice(),
            [Out::Front(Some(faraday_kernel::FrameSheet::LockAsk))]
        ));
        assert_eq!(state.sheet(), Some(Which::LockAsk));
    }

    /// Invariant: Lock without saving begins a lock that does not seal; Lock on the same sheet seals.
    /// Regression: Both buttons of the Lock sheet send the same lock.
    /// Owner: app
    #[test]
    fn lock_without_saving_does_not_seal_and_lock_does() {
        let view = TestView::device();
        let mut state = State::new(Some(Which::LockAsk));
        let without = run(&view, &mut state, Msg::Press(Press::LockWithoutSealing)).outs;
        assert!(matches!(
            without.as_slice(),
            [Out::Top(Top::Lock(Lock { seal: false, .. }))]
        ));
        let mut state = State::new(Some(Which::LockAsk));
        let sealed = run(&view, &mut state, Msg::Press(Press::Lock)).outs;
        assert!(matches!(
            sealed.as_slice(),
            [Out::Top(Top::Lock(Lock { seal: true, .. }))]
        ));
    }

    /// Invariant: Cancel puts the sheet away and nothing else; with no sheet up it does nothing.
    /// Regression: Cancel on the Power sheet locks or powers off, or puts away a sheet that is not this feature's.
    /// Owner: app
    #[test]
    fn cancel_puts_the_sheet_away_and_does_nothing_else() {
        let view = TestView::device();
        let mut state = State::new(Some(Which::Power));
        let out = run(&view, &mut state, Msg::Press(Press::Away)).outs;
        assert!(matches!(out.as_slice(), [Out::Front(None)]));
        assert_eq!(state.sheet(), None);
        let again = run(&view, &mut state, Msg::Press(Press::Away)).outs;
        assert!(again.is_empty());
    }

    /// Invariant: The app that follows a lock opens on the Locked sheet, and one that follows a power-on opens on none.
    /// Regression: The next app after a lock shows no word that it was locked.
    /// Owner: app
    #[test]
    fn the_app_after_a_lock_opens_on_the_locked_sheet() {
        let view = TestView::device().with_started(Started::Lock);
        let (state, fx) = Power::start(&mut Carried::empty(), &view.ctx());
        assert_eq!(state.sheet(), Some(Which::Locked));
        assert!(matches!(
            seen(fx).outs.as_slice(),
            [Out::Front(Some(faraday_kernel::FrameSheet::Lock))]
        ));

        let view = TestView::device();
        let (state, fx) = Power::start(&mut Carried::empty(), &view.ctx());
        assert_eq!(state.sheet(), None);
        assert!(seen(fx).outs.is_empty());
    }

    /// Invariant: A mode change shows the Lock sheet even where nothing would be lost, and the Lock it offers seals the open vaults.
    /// Regression: A mode change locks at once, or its Lock leaves the vaults unsealed.
    /// Owner: app
    #[test]
    fn a_mode_change_asks_first_and_its_lock_seals() {
        let view = TestView::device();
        let mut state = State::new(None);
        let shown = run(&view, &mut state, Msg::ModeLock).outs;
        assert!(matches!(shown.as_slice(), [Out::Front(Some(_))]));
        assert_eq!(state.sheet(), Some(Which::LockAsk));
        assert!(state.mode_change());
        let locked = run(&view, &mut state, Msg::Press(Press::Lock)).outs;
        assert!(matches!(
            locked.as_slice(),
            [Out::Top(Top::Lock(Lock {
                seal: true,
                idle: false
            }))]
        ));
        assert!(!state.mode_change());
    }
}
