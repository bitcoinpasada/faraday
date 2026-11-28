use faraday_kernel::{
    Carried, Ctx, Effects, End, Feature, Lock, Notice, Public, Started, Top, View,
};

use crate::{Msg, Out, Power, Press, State, Which};

/// Why a message is refused before `update` sees it. The feature refuses nothing: every press
/// on its sheets is an answer to what the sheet says.
#[derive(Public)]
pub enum Refusal {}

/// The highest number a mode asked for has (`Top::ModeLock`).
const MODES: u8 = 4;

type Fx = Effects<Msg, Out>;

/// The sheet comes up.
fn show(state: &mut State, which: Which) -> Fx {
    // Another sheet over a mode change's Lock sheet gives the asked mode up.
    let cancel = give_up(state, which != Which::LockAsk);
    state.show(Some(which));
    cancel.out(Out::Front(Some(which.frame())))
}

/// The mode change's mark is cleared, and the settings owner told, when `leaving`: every way out
/// of the mode change's Lock sheet but its Lock presses sends it.
fn give_up(state: &mut State, leaving: bool) -> Fx {
    if leaving && state.mode_change().is_some() {
        state.set_mode_change(None);
        Effects::none().out(Out::Top(Top::ModeCancel))
    } else {
        Effects::none()
    }
}

/// The sheet goes away, when one is up.
fn away(state: &mut State) -> Fx {
    match state.sheet() {
        Some(_) => {
            // A Lock sheet that asked for a mode change closes without it: the asked mode is
            // given up.
            let cancel = state.mode_change().is_some();
            state.show(None);
            let fx = Effects::none().out(Out::Front(None));
            if cancel {
                fx.out(Out::Top(Top::ModeCancel))
            } else {
                fx
            }
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
    let cancel = give_up(state, true);
    state.show(None);
    cancel.out(Out::Top(Top::End(end)))
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
            Msg::Lock { asks_first: true } => {
                // A plain Lock sheet replaces the mode change's.
                let cancel = give_up(state, true);
                cancel.and(show(state, Which::LockAsk))
            }
            // A mode the device does not number is no mode asked: the settings owner is told it
            // is given up.
            Msg::ModeLock(mode) if mode > MODES => Effects::none().out(Out::Top(Top::ModeCancel)),
            Msg::ModeLock(mode) => {
                let fx = show(state, Which::LockAsk);
                // `show` clears the mark of any other sheet; the asked mode is set after it.
                state.set_mode_change(Some(mode));
                fx
            }
            // A lock asked for anywhere is no mode change: the sheet that asked for one ends.
            Msg::Lock { asks_first: false } => give_up(state, true).and(lock(state, true)),
            Msg::Press(Press::Lock) => lock(state, true),
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
    fn notice(state: &mut State, notice: &Notice<'_>, _cx: &Ctx<'_>) -> Fx {
        match notice {
            // The Lock sheet of a mode change closed any other way than its Lock (Back, a press
            // elsewhere): the asked mode is given up. A pressed Lock has already cleared the
            // mark.
            Notice::Left if state.mode_change().is_some() => away(state),
            _ => Effects::none(),
        }
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
        let shown = run(&view, &mut state, Msg::ModeLock(3)).outs;
        assert!(matches!(shown.as_slice(), [Out::Front(Some(_))]));
        assert_eq!(state.sheet(), Some(Which::LockAsk));
        assert_eq!(state.mode_change(), Some(3));
        let locked = run(&view, &mut state, Msg::Press(Press::Lock)).outs;
        assert!(matches!(
            locked.as_slice(),
            [Out::Top(Top::Lock(Lock {
                seal: true,
                idle: false
            }))]
        ));
        assert_eq!(state.mode_change(), None);
    }

    /// Invariant: A mode change's Lock sheet that closes without its Lock gives the asked mode up, by Not now or by leaving it, and its Lock does not.
    /// Regression: Not now or Back leaves the asked mode standing, so a later lock changes the mode.
    /// Owner: app
    #[test]
    fn a_mode_sheet_closed_without_locking_cancels_the_mode() {
        let view = TestView::device();
        let mut state = State::new(None);
        let _ = run(&view, &mut state, Msg::ModeLock(3));
        let away = run(&view, &mut state, Msg::Press(Press::Away)).outs;
        assert!(away.iter().any(|o| matches!(o, Out::Top(Top::ModeCancel))));

        let _ = run(&view, &mut state, Msg::ModeLock(3));
        let left = seen(Power::notice(&mut state, &Notice::Left, &view.ctx())).outs;
        assert!(left.iter().any(|o| matches!(o, Out::Top(Top::ModeCancel))));

        let _ = run(&view, &mut state, Msg::ModeLock(3));
        let locked = run(&view, &mut state, Msg::Press(Press::Lock)).outs;
        assert!(
            !locked
                .iter()
                .any(|o| matches!(o, Out::Top(Top::ModeCancel)))
        );
        let plain = run(
            &view,
            &mut State::new(Some(Which::LockAsk)),
            Msg::Press(Press::Away),
        )
        .outs;
        assert!(!plain.iter().any(|o| matches!(o, Out::Top(Top::ModeCancel))));
    }

    /// Invariant: Every way out of a mode change's Lock sheet but its Lock presses gives the asked mode up: another sheet over it, a plain lock, a Lock sheet of the ordinary kind.
    /// Regression: The mark clears without telling the settings owner, so a later lock applies a mode nobody confirmed.
    /// Owner: app
    #[test]
    fn every_way_out_of_the_mode_sheet_but_its_lock_gives_the_mode_up() {
        let view = TestView::device();
        let cancels = |outs: &[Out]| {
            outs.iter()
                .filter(|o| matches!(o, Out::Top(Top::ModeCancel)))
                .count()
        };
        for msg in [
            Msg::Show(Which::Power),
            Msg::Show(Which::IdleWarn),
            Msg::Lock { asks_first: false },
            Msg::Lock { asks_first: true },
            Msg::Press(Press::PowerOff),
        ] {
            let mut state = State::new(None);
            let _ = run(&view, &mut state, Msg::ModeLock(3));
            let outs = run(&view, &mut state, msg).outs;
            assert_eq!(cancels(&outs), 1);
            assert_eq!(state.mode_change(), None);
            // Nothing more to cancel when the sheet is put away.
            let away = run(&view, &mut state, Msg::Press(Press::Away)).outs;
            assert_eq!(cancels(&away), 0);
        }
        for press in [Press::Lock, Press::LockWithoutSealing] {
            let mut state = State::new(None);
            let _ = run(&view, &mut state, Msg::ModeLock(3));
            let outs = run(&view, &mut state, Msg::Press(press)).outs;
            assert_eq!(cancels(&outs), 0);
        }
    }

    /// Invariant: The mode asked is kept only while its Lock sheet is open: it is gone once the lock begins, and the next build of the scope starts without it; a number that is no mode is given up at once.
    /// Regression: A mode asked before a lock stands in the next build's sheet, so a later Lock sheet says "Change to X?" for a change nobody asked for.
    /// Owner: app
    #[test]
    fn the_mode_asked_is_wiped_at_lock_and_a_number_that_is_no_mode_is_refused() {
        let view = TestView::device();
        let mut state = State::new(None);
        let _ = run(&view, &mut state, Msg::ModeLock(2));
        assert_eq!(state.mode_change(), Some(2));
        let _ = run(&view, &mut state, Msg::Press(Press::Lock));
        assert_eq!(state.mode_change(), None);

        let after = view.with_started(Started::Lock);
        let (next, _) = Power::start(&mut Carried::empty(), &after.ctx());
        assert_eq!(next.mode_change(), None);
        assert_eq!(next.sheet(), Some(Which::Locked));

        let mut state = State::new(None);
        let outs = run(&after, &mut state, Msg::ModeLock(9)).outs;
        assert!(matches!(outs.as_slice(), [Out::Top(Top::ModeCancel)]));
        assert_eq!(state.sheet(), None);
    }
}
