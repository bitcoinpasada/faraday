use faraday_kernel::settings::{self, Change, Set};
use faraday_kernel::shared::{self, Changed};
use faraday_kernel::{AnyChange, ChangeId, FactsChanged, Lock, Owner};
use faraday_kernel::{Carried, Ctx, Effects, Feature, Notice, Public, View};
use faraday_port::storage::{Read, StickId};

use crate::file;
use crate::kept::{Kept, MODE, SETTINGS};
use crate::state::Look;
use crate::{Msg, Out, Press, Settings, State};

/// Why a message is refused before `update` sees it.
#[derive(Public)]
pub enum Refusal {}

/// Asks for the settings file when the boot stick is listed and holds one: once per process, and
/// only from the boot stick (what is on any other stick is not settings). A boot stick without
/// the file is what the settings would have given had it held one.
fn look(state: &mut State, cx: &Ctx<'_>) -> Effects<Msg, Out> {
    if state.look() != Look::Waiting {
        return Effects::none();
    }
    let Some(boot) = cx.view.facts.sticks.listed.iter().find(|s| s.boot) else {
        return Effects::none();
    };
    match boot
        .files
        .iter()
        .find(|(name, _)| file::is_file(name.as_str()))
    {
        Some((name, _)) => {
            state.set_look(Look::Asked);
            Effects::none().ask(
                Read {
                    stick: boot.id.clone(),
                    name: name.clone(),
                },
                boot.id.clone(),
                read_back,
            )
        }
        None => {
            state.on_stick_known();
            Effects::none()
        }
    }
}

/// The answer to the read: its bytes, or `None` when it failed.
fn read_back(
    stick: StickId,
    answer: Result<faraday_port::bytes::Bytes, faraday_port::Failure>,
) -> Msg {
    Msg::Read(stick, answer.ok().map(|bytes| bytes.expose().clone()))
}

impl Feature for Settings {
    type State = State;
    type Msg = Msg;
    type Out = Out;
    type Refusal = Refusal;

    fn start(carried: &mut Carried, cx: &Ctx<'_>) -> (State, Effects<Msg, Out>) {
        // What the last build carried is what the person chose: the file is not read over it.
        let mut state = match carried.take::<Kept>(SETTINGS) {
            Some(kept) => {
                let (settings, on_stick) = kept.settings();
                State::new(settings, Look::Known, on_stick)
            }
            None => State::new(Default::default(), Look::Waiting, None),
        };
        let fx = look(&mut state, cx);
        (state, fx)
    }

    fn update(state: &mut State, msg: Msg, _cx: &Ctx<'_>) -> Effects<Msg, Out> {
        match msg {
            Msg::Press(press) => match press {
                Press::Mode(m) => {
                    state.ask_mode(Some(m));
                    Effects::none()
                }
                Press::ModeCancel => {
                    state.ask_mode(None);
                    Effects::none()
                }
                Press::ModeLock => match state.mode_asked() {
                    Some(_) => Effects::none().out(Out::Lock(Lock {
                        seal: false,
                        idle: false,
                    })),
                    None => Effects::none(),
                },
            },
            Msg::Read(_, bytes) => {
                if state.look() != Look::Asked {
                    return Effects::none();
                }
                let before = file::body(state.settings());
                if let Some(bytes) = bytes {
                    file::read(state.settings_mut(), &bytes);
                }
                // A failed read, like a file ignored whole, leaves what the settings are.
                state.on_stick_known();
                if file::body(state.settings()) != before {
                    Effects::none().out(Out::Shared(Changed::Settings(settings::Changed::Choices)))
                } else {
                    Effects::none()
                }
            }
            Msg::Given(id, change) => given(state, id, change),
        }
    }

    fn notice(state: &mut State, notice: &Notice<'_>, cx: &Ctx<'_>) -> Effects<Msg, Out> {
        match notice {
            Notice::Facts(FactsChanged::Sticks) => look(state, cx),
            _ => Effects::none(),
        }
    }

    fn refuses(_state: &State, _msg: &Msg, _view: &View<'_>) -> Option<Refusal> {
        None
    }

    fn carry(state: &mut State, into: &mut Carried) {
        into.put(SETTINGS, &Kept::of(state.settings(), state.on_stick()));
        if let Some(m) = state.mode_asked() {
            into.put(MODE, &m);
        }
    }
}

/// One choice given by another feature: applied with the offered-value checks, answered once.
fn given(state: &mut State, id: ChangeId, change: AnyChange) -> Effects<Msg, Out> {
    match change {
        AnyChange::Settings(Change::Set(Set(choice))) => {
            let changed = file::choose(state.settings_mut(), choice);
            let fx = Effects::none().done(id, shared::Done::Settings(settings::Done::Set(())));
            if changed {
                fx.out(Out::Shared(Changed::Settings(settings::Changed::Choices)))
            } else {
                fx
            }
        }
        // Another owner's change is not this one's.
        _ => Effects::none(),
    }
}

impl Owner for Settings {
    fn given(id: ChangeId, change: AnyChange) -> Msg {
        Msg::Given(id, change)
    }
}

#[cfg(test)]
mod tests {
    use faraday_app_test::{Seen, TestView, seen};
    use faraday_kernel::route;
    use faraday_kernel::settings::Choice;

    use super::*;
    use crate::ModeAsked;
    use crate::kept::MODE;

    fn started() -> State {
        let view = TestView::device();
        Settings::start(&mut Carried::empty(), &view.ctx()).0
    }

    fn given(state: &mut State, choice: Choice) -> Seen<Msg, Out> {
        let view = TestView::device();
        let change = AnyChange::Settings(Change::Set(Set(choice)));
        let id = ChangeId {
            generation: 1,
            n: 1,
        };
        seen(route::<Settings>(
            state,
            Msg::Given(id, change),
            &view.ctx(),
        ))
    }

    /// Invariant: A choice another feature gives is answered once, and the settings say they changed only when it changed them.
    /// Regression: A choice is answered twice, never, or announced as a change when it was already the setting or not offered.
    /// Owner: app
    #[test]
    fn a_given_choice_is_answered_once_and_announced_only_when_it_changes() {
        let mut state = started();
        let first = given(&mut state, Choice::Scale(150));
        assert_eq!(first.dones.len(), 1);
        assert_eq!(first.outs.len(), 1);
        assert_eq!(state.settings_mut().scale_pct, 150);
        for choice in [Choice::Scale(150), Choice::Scale(7)] {
            let again = given(&mut state, choice);
            assert_eq!(again.dones.len(), 1);
            assert!(again.outs.is_empty());
        }
        assert_eq!(state.settings_mut().scale_pct, 150);
    }

    /// Invariant: What the settings owner carries through a lock starts the next build as it was, with an idle time of never and the Remembered choices kept, and an entry that cannot be read starts the defaults.
    /// Regression: The carried settings are read by the stick's rules, drop a field, or an unreadable entry is half applied.
    /// Owner: app
    #[test]
    fn the_carry_starts_the_next_build_as_the_last_one_ended() {
        let view = TestView::device();
        let mut state = started();
        let _ = given(&mut state, Choice::IdleLock(0));
        let _ = given(&mut state, Choice::Zone(-210));
        let _ = given(&mut state, Choice::SealAmounts(true));
        state.settings_mut().offset_secs = -42;
        state.settings_mut().checked = true;
        let mut carried = Carried::empty();
        Settings::carry(&mut state, &mut carried);
        let mut carried = Carried::from_carry(carried.into_carry(), false);
        let mut next = Settings::start(&mut carried, &view.ctx()).0;
        let kept = next.settings_mut();
        assert_eq!(kept.idle_lock_min, 0);
        assert_eq!(kept.zone_min, Some(-210));
        assert!(kept.seal_amounts && kept.checked);
        assert_eq!(kept.offset_secs, -42);

        // An entry that cannot be read starts the defaults.
        let junk = faraday_port::storage::Carry {
            inbox: Vec::new(),
            outbox: Vec::new(),
            kept: vec![(
                faraday_text::Text::of("settings.kept"),
                faraday_port::bytes::Bytes::new(vec![9, 9, 9]),
            )],
        };
        let mut junk = Carried::from_carry(junk, false);
        let mut fresh = Settings::start(&mut junk, &view.ctx()).0;
        assert_eq!(fresh.settings_mut().idle_lock_min, 30);
    }

    /// Invariant: The mode a person asks for is carried through the lock that applies it, and asking for none carries none.
    /// Regression: The asked mode is dropped at the lock, or an unasked mode is carried so a plain lock changes the mode.
    /// Owner: app
    #[test]
    fn the_mode_asked_is_carried_and_a_cancel_carries_none() {
        let view = TestView::device();
        let mut state = started();
        let mut carried = Carried::empty();
        Settings::carry(&mut state, &mut carried);
        assert!(carried.take::<ModeAsked>(MODE).is_none());

        let picked = route::<Settings>(
            &mut state,
            Msg::Press(Press::Mode(ModeAsked::Signet)),
            &view.ctx(),
        );
        assert!(seen(picked).outs.is_empty());
        let lock = seen(route::<Settings>(
            &mut state,
            Msg::Press(Press::ModeLock),
            &view.ctx(),
        ));
        assert!(matches!(
            lock.outs.as_slice(),
            [Out::Lock(Lock {
                seal: false,
                idle: false
            })]
        ));
        let mut carried = Carried::empty();
        Settings::carry(&mut state, &mut carried);
        assert_eq!(carried.take::<ModeAsked>(MODE), Some(ModeAsked::Signet));

        let _ = route::<Settings>(&mut state, Msg::Press(Press::ModeCancel), &view.ctx());
        let none = seen(route::<Settings>(
            &mut state,
            Msg::Press(Press::ModeLock),
            &view.ctx(),
        ));
        assert!(none.outs.is_empty());
        let mut carried = Carried::empty();
        Settings::carry(&mut state, &mut carried);
        assert!(carried.take::<ModeAsked>(MODE).is_none());
    }

    /// Invariant: A file answer nobody asked for changes no setting, and a lock leaves the person's choices as they were.
    /// Regression: A stray answer overwrites the choices the carry restored, or a lock wipes the settings.
    /// Owner: app
    #[test]
    fn a_stray_file_answer_and_a_lock_leave_the_choices() {
        use faraday_kernel::AtLock;
        let view = TestView::device();
        let mut state = started();
        let _ = given(&mut state, Choice::Scale(200));
        let file = format!("{}\nscale=75\n", crate::HEADER).into_bytes();
        let stick = view
            .ctx()
            .view
            .facts
            .sticks
            .listed
            .first()
            .map(|s| s.id.clone());
        if let Some(stick) = stick {
            let fx = seen(route::<Settings>(
                &mut state,
                Msg::Read(stick, Some(file)),
                &view.ctx(),
            ));
            assert!(fx.outs.is_empty());
        }
        assert_eq!(state.settings_mut().scale_pct, 200);
        state.at_lock();
        assert_eq!(state.settings_mut().scale_pct, 200);
    }
}
