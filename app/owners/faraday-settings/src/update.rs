use faraday_kernel::settings::{self, Change, Set};
use faraday_kernel::shared::{self, Changed};
use faraday_kernel::{AnyChange, ChangeId, FactsChanged, Owner};
use faraday_kernel::{Carried, Ctx, Effects, Feature, Notice, Public, View};
use faraday_port::storage::{Read, StickId};

use crate::file;
use crate::kept::{Kept, MODE, ModeAsked, SETTINGS};
use crate::state::Look;
use crate::{Msg, Out, Press, Settings, State};

/// Why a message is refused before `update` sees it.
#[derive(Public)]
pub enum Refusal {
    /// The mode pressed is the one in use: the sheet closes and nothing else happens.
    ModeInUse,
}

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
        let in_use = ModeAsked::of_index(cx.view.mode.index());
        // What the last build carried is what the person chose: the file is not read over it.
        let mut state = match carried.take::<Kept>(SETTINGS) {
            Some(kept) => {
                let (settings, on_stick) = kept.settings();
                // What the boot stick held is carried only once known: a lock before that
                // reads the file again and applies it over the kept choices, as `main` did.
                let look = if on_stick.is_some() {
                    Look::Known
                } else {
                    Look::Waiting
                };
                State::new(settings, look, on_stick, in_use)
            }
            None => State::new(Default::default(), Look::Waiting, None, in_use),
        };
        let fx = look(&mut state, cx);
        (state, fx)
    }

    fn update(state: &mut State, msg: Msg, cx: &Ctx<'_>) -> Effects<Msg, Out> {
        state.refresh(cx.view.mode.index());
        match msg {
            Msg::Press(press) => match press {
                Press::Mode(m) => {
                    state.ask_mode(Some(m));
                    Effects::none()
                }
                Press::ModeCancel => {
                    state.ask_mode(None);
                    state.set_pending(false);
                    Effects::none()
                }
                Press::ModeLock => match state.mode_asked() {
                    // The Lock sheet asks first and its Lock seals: the power feature's, reached
                    // through the root.
                    Some(_) => {
                        state.set_pending(true);
                        Effects::none().out(Out::ModeLock)
                    }
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
        state.refresh(cx.view.mode.index());
        match notice {
            Notice::Facts(FactsChanged::Sticks) => look(state, cx),
            // A pick left on the mode sheet, by Back or a press elsewhere, is given up; one the
            // Lock sheet is asking about waits for its answer.
            Notice::Left => {
                if !state.pending() {
                    state.ask_mode(None);
                }
                Effects::none()
            }
            _ => Effects::none(),
        }
    }

    fn refuses(_state: &State, msg: &Msg, view: &View<'_>) -> Option<Refusal> {
        match msg {
            Msg::Press(Press::Mode(m)) if *m == ModeAsked::of_index(view.mode.index()) => {
                Some(Refusal::ModeInUse)
            }
            _ => None,
        }
    }

    fn carry(state: &mut State, into: &mut Carried) {
        into.put(SETTINGS, &Kept::of(state.settings(), state.on_stick()));
        into.put(MODE, &state.mode_after_lock());
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
    use faraday_port::storage::StickId;

    use super::*;
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

    fn stick(id: &str, boot: bool, files: &[&str]) -> faraday_port::storage::StickInfo {
        faraday_port::storage::StickInfo {
            id: StickId(faraday_text::Text::of(id)),
            label: faraday_text::Text::of(id),
            boot,
            serial: None,
            files: files
                .iter()
                .map(|f| (faraday_text::Text::of(f), 10))
                .collect(),
            vaults: Vec::new(),
            make: Default::default(),
        }
    }

    fn lock_carry(state: &mut State) -> Carried {
        let mut carried = Carried::empty();
        Settings::carry(state, &mut carried);
        carried
    }

    /// Invariant: A lock with no mode change begun starts the next build in the mode in use, and only the Lock sheet's own mode change starts it in another.
    /// Regression: A plain lock carries no mode and the root gives its own, or a mode picked and abandoned is applied at the next lock.
    /// Owner: app
    #[test]
    fn a_lock_keeps_the_mode_in_use_unless_a_change_was_begun() {
        let view = TestView::device();
        let mut state = started();
        let mut carried = lock_carry(&mut state);
        assert_eq!(carried.take::<ModeAsked>(MODE), Some(ModeAsked::Mainnet));

        // The Lock sheet asked, Not now, then an idle lock: the mode in use.
        let _ = route::<Settings>(
            &mut state,
            Msg::Press(Press::Mode(ModeAsked::Signet)),
            &view.ctx(),
        );
        let _ = route::<Settings>(&mut state, Msg::Press(Press::ModeLock), &view.ctx());
        let _ = route::<Settings>(&mut state, Msg::Press(Press::ModeCancel), &view.ctx());
        let mut idle = lock_carry(&mut state);
        assert_eq!(idle.take::<ModeAsked>(MODE), Some(ModeAsked::Mainnet));

        // Picked, then left by Back or a press elsewhere: given up.
        let _ = route::<Settings>(
            &mut state,
            Msg::Press(Press::Mode(ModeAsked::Signet)),
            &view.ctx(),
        );
        let _ = route::<Settings>(&mut state, Msg::Press(Press::ModeLock), &view.ctx());
        let _ = route::<Settings>(&mut state, Msg::Press(Press::ModeCancel), &view.ctx());
        let mut carried = lock_carry(&mut state);
        assert_eq!(carried.take::<ModeAsked>(MODE), Some(ModeAsked::Mainnet));
        let _ = route::<Settings>(
            &mut state,
            Msg::Press(Press::Mode(ModeAsked::Signet)),
            &view.ctx(),
        );
        let _ = route::<Settings>(
            &mut state,
            Msg::Press(Press::Mode(ModeAsked::Regtest)),
            &view.ctx(),
        );
        let _ = Settings::notice(&mut state, &Notice::Left, &view.ctx());
        let mut carried = lock_carry(&mut state);
        assert_eq!(carried.take::<ModeAsked>(MODE), Some(ModeAsked::Mainnet));

        // The Lock sheet asked, and it is the lock that applies it.
        let _ = route::<Settings>(
            &mut state,
            Msg::Press(Press::Mode(ModeAsked::Signet)),
            &view.ctx(),
        );
        let asked = seen(route::<Settings>(
            &mut state,
            Msg::Press(Press::ModeLock),
            &view.ctx(),
        ));
        assert!(matches!(asked.outs.as_slice(), [Out::ModeLock]));
        let _ = Settings::notice(&mut state, &Notice::Left, &view.ctx());
        let mut carried = lock_carry(&mut state);
        assert_eq!(carried.take::<ModeAsked>(MODE), Some(ModeAsked::Signet));
    }

    /// Invariant: Picking the mode already in use does nothing, and no lock follows.
    /// Regression: The mode in use is accepted and its Lock sheet locks the session for no change.
    /// Owner: app
    #[test]
    fn the_mode_in_use_is_refused() {
        let view = TestView::device();
        let state = started();
        let refused = Settings::refuses(
            &state,
            &Msg::Press(Press::Mode(ModeAsked::Mainnet)),
            &view.ctx().view,
        );
        assert!(matches!(refused, Some(Refusal::ModeInUse)));
        let other = Settings::refuses(
            &state,
            &Msg::Press(Press::Mode(ModeAsked::Testnet)),
            &view.ctx().view,
        );
        assert!(other.is_none());
    }

    /// Invariant: A lock before the boot stick's file was read starts the next build looking for it again, and a carry that knows the stick does not.
    /// Regression: A carried start marks the file read, so a stick's idle times and scale are never applied that power-on.
    /// Owner: app
    #[test]
    fn a_carry_before_the_file_was_read_reads_it_again() {
        let view = TestView::device();
        let mut state = started();
        assert_eq!(state.look(), Look::Waiting);
        let mut carried = lock_carry(&mut state);
        let next = Settings::start(&mut carried, &view.ctx()).0;
        assert_eq!(next.look(), Look::Waiting);

        let mut known = started();
        known.on_stick_known();
        let mut carried = lock_carry(&mut known);
        let next = Settings::start(&mut carried, &view.ctx()).0;
        assert_eq!(next.look(), Look::Known);
    }

    fn file_bytes(lines: &str) -> Vec<u8> {
        format!("{}\n{lines}", crate::HEADER).into_bytes()
    }

    /// Invariant: The settings file is asked for once, from the boot stick alone, and what comes back is applied; a failed read leaves the settings and counts as known.
    /// Regression: The file is read from a stick that is not the boot stick, read again at each listing, or a failed read leaves the owner asking.
    /// Owner: app
    #[test]
    fn the_file_is_read_once_from_the_boot_stick_only() {
        // A boot stick without the file and another stick with it: nothing is asked.
        let view = TestView::device().with_sticks(vec![
            stick("boot", true, &[]),
            stick("other", false, &["faraday-settings.txt"]),
        ]);
        let (state, fx) = Settings::start(&mut Carried::empty(), &view.ctx());
        assert!(seen(fx).asks.is_empty());
        assert_eq!(state.look(), Look::Known);

        // The boot stick holds it (any case): one ask, and a second listing asks nothing.
        let view = TestView::device().with_sticks(vec![
            stick("other", false, &["faraday-settings.txt"]),
            stick("boot", true, &["Faraday-Settings.TXT"]),
        ]);
        let (mut state2, fx) = Settings::start(&mut Carried::empty(), &view.ctx());
        let mut asks = seen(fx).asks;
        assert_eq!(asks.len(), 1);
        let (request, _reply) = faraday_app_test::asked(asks.remove(0));
        assert!(matches!(
            request,
            faraday_port::Ask::Storage(faraday_port::storage::StorageAsk::Read { stick, .. })
                if stick == StickId(faraday_text::Text::of("boot"))
        ));
        let again = Settings::notice(
            &mut state2,
            &Notice::Facts(FactsChanged::Sticks),
            &view.ctx(),
        );
        assert!(seen(again).asks.is_empty());

        // The answer applies the file and tells the others.
        let boot = StickId(faraday_text::Text::of("boot"));
        let read = route::<Settings>(
            &mut state2,
            Msg::Read(boot.clone(), Some(file_bytes("scale=150\n"))),
            &view.ctx(),
        );
        assert_eq!(seen(read).outs.len(), 1);
        assert_eq!(state2.settings().scale_pct, 150);
        assert_eq!(state2.look(), Look::Known);

        // A failed read leaves the settings and is known.
        let (mut state3, fx) = Settings::start(&mut Carried::empty(), &view.ctx());
        let _ = seen(fx);
        let failed = route::<Settings>(&mut state3, Msg::Read(boot.clone(), None), &view.ctx());
        assert!(seen(failed).outs.is_empty());
        assert_eq!(state3.settings().scale_pct, 100);
        assert_eq!(state3.look(), Look::Known);
        // An answer nobody asked for changes nothing.
        let stray = route::<Settings>(
            &mut state3,
            Msg::Read(boot, Some(file_bytes("scale=75\n"))),
            &view.ctx(),
        );
        assert!(seen(stray).outs.is_empty());
        assert_eq!(state3.settings().scale_pct, 100);
    }

    /// Invariant: An owner that outlives the scope (the Starter's) refuses the mode now in use, not the one it started in, and a mode change that took effect leaves no asked mode behind.
    /// Regression: The owner keeps the mode of its start, so after a change it refuses the wrong mode and its next plain lock returns to the old one.
    /// Owner: app
    #[test]
    fn a_mode_change_that_took_effect_leaves_the_owner_in_the_new_mode() {
        let device = TestView::device();
        let demo = TestView::demo();
        let mut state = started();
        let _ = route::<Settings>(
            &mut state,
            Msg::Press(Press::Mode(ModeAsked::Signet)),
            &device.ctx(),
        );
        let _ = route::<Settings>(&mut state, Msg::Press(Press::ModeLock), &device.ctx());
        let mut carried = lock_carry(&mut state);
        assert_eq!(carried.take::<ModeAsked>(MODE), Some(ModeAsked::Signet));

        // The scope is built again in another mode and the owner is not started again.
        let _ = route::<Settings>(
            &mut state,
            Msg::Press(Press::Mode(ModeAsked::Testnet)),
            &demo.ctx(),
        );
        let refused = Settings::refuses(
            &state,
            &Msg::Press(Press::Mode(ModeAsked::Practice)),
            &demo.ctx().view,
        );
        assert!(matches!(refused, Some(Refusal::ModeInUse)));
        let mut carried = lock_carry(&mut state);
        assert_eq!(carried.take::<ModeAsked>(MODE), Some(ModeAsked::Practice));
        let open = Settings::refuses(
            &state,
            &Msg::Press(Press::Mode(ModeAsked::Mainnet)),
            &demo.ctx().view,
        );
        assert!(open.is_none());
    }
}
