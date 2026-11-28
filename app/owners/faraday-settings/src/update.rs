use faraday_kernel::settings::{self, Change, Set};
use faraday_kernel::shared::{self, Changed};
use faraday_kernel::{AnyChange, ChangeId, FactsChanged, Owner};
use faraday_kernel::{Carried, Ctx, Effects, Feature, Notice, Public, View};
use faraday_port::storage::{Read, StickId};

use crate::clock::{self, Key, Typed};
use crate::file;
use crate::kept::{Kept, MODE, ModeAsked, SETTINGS};
use crate::state::Look;
use crate::{Msg, Out, Press, Settings, State};

/// Why a message is refused before `update` sees it.
#[derive(Public)]
pub enum Refusal {
    /// The mode pressed is the one in use: the sheet closes and nothing else happens.
    ModeInUse,
    /// Set needs a zone chosen and a real date and a real 24-hour time typed.
    NotReady,
    /// Keep this time needs a clock reading to keep.
    NoClock,
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
            Msg::Press(press) => pressed(state, press, cx),
            Msg::Typed(c) => key(state, Key::Char(c), cx),
            Msg::Erase => key(state, Key::Erase, cx),
            Msg::Tab => key(state, Key::Tab, cx),
            Msg::Enter => key(state, Key::Enter, cx),
            Msg::Escape => key(state, Key::Escape, cx),
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
            // The page left by the bar or a menu: Set the time is given up (what was typed goes
            // with it, the form opens afresh), and a pick left on the mode sheet is given up; one
            // the Lock sheet is asking about waits for its answer.
            Notice::Left => {
                state.show(settings::Page::Settings);
                if !state.pending() {
                    state.ask_mode(None);
                }
                Effects::none()
            }
            _ => Effects::none(),
        }
    }

    fn refuses(state: &State, msg: &Msg, view: &View<'_>) -> Option<Refusal> {
        match msg {
            Msg::Press(Press::Mode(m) | Press::ModeAsk(m))
                if *m == ModeAsked::of_index(view.mode.index()) =>
            {
                Some(Refusal::ModeInUse)
            }
            Msg::Press(Press::Set) if !state.set_ready(view) => Some(Refusal::NotReady),
            Msg::Press(Press::Keep) if clock::now(&view.clock).is_none() => Some(Refusal::NoClock),
            _ => None,
        }
    }

    fn carry(state: &mut State, into: &mut Carried) {
        into.put(SETTINGS, &Kept::of(state.settings(), state.on_stick()));
        into.put(MODE, &state.mode_after_lock());
        state.settle();
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

/// A press: the mode sheet's, a choice of Settings, or a step of Set the time.
fn pressed(state: &mut State, press: Press, cx: &Ctx<'_>) -> Effects<Msg, Out> {
    match press {
        Press::Mode(m) => {
            state.ask_mode(Some(m));
            Effects::none()
        }
        Press::ModeAsk(m) => {
            state.ask_mode(Some(m));
            state.set_pending(true);
            Effects::none().out(Out::ModeLock(m.index()))
        }
        Press::ModeCancel => {
            state.ask_mode(None);
            state.set_pending(false);
            Effects::none()
        }
        Press::ModeLock => match state.mode_asked() {
            // The Lock sheet asks first and its Lock seals: the power feature's, reached
            // through the root.
            Some(m) => {
                state.set_pending(true);
                Effects::none().out(Out::ModeLock(m.index()))
            }
            None => Effects::none(),
        },
        Press::Choose(choice) => {
            if file::choose(state.settings_mut(), choice) {
                changed(settings::Changed::Choices)
            } else {
                Effects::none()
            }
        }
        Press::OpenSetTime => {
            let date = state.today(&cx.view);
            state.form_mut().open(date.as_deref());
            state.show(settings::Page::SetTime);
            Effects::none()
        }
        Press::SetTimeBack => {
            state.show(settings::Page::Settings);
            Effects::none()
        }
        Press::Focus(focus) => {
            state.form_mut().set_focus(focus);
            Effects::none()
        }
        Press::ZoneList(on) => {
            // Opening the list from Settings opens Set the time on it.
            if on && state.page() != settings::Page::SetTime {
                let date = state.today(&cx.view);
                state.form_mut().open(date.as_deref());
                state.show(settings::Page::SetTime);
            }
            state.form_mut().set_zones(on);
            Effects::none()
        }
        Press::ZoneStep(delta) => {
            let at = state.settings().zone_min.unwrap_or(0);
            zone(state, at.saturating_add(delta), &cx.view)
        }
        Press::Zone(min) => {
            state.form_mut().set_zones(false);
            zone(state, min, &cx.view)
        }
        Press::Set => set(state, &cx.view),
        Press::Keep => {
            if clock::now(&cx.view.clock).is_some() {
                state.settings_mut().checked = true;
                state.show(settings::Page::Settings);
                changed(settings::Changed::Clock)
            } else {
                Effects::none()
            }
        }
        Press::SelfTest => Effects::none().out(Out::SelfTest),
        Press::Find => {
            state.focus_find(true);
            Effects::none()
        }
        Press::FindClear => {
            state.clear_find();
            Effects::none()
        }
        // The power feature's: the product routes these presses there.
        Press::Lock | Press::PowerAsk => Effects::none(),
    }
}

fn changed(what: settings::Changed) -> Effects<Msg, Out> {
    Effects::none().out(Out::Shared(Changed::Settings(what)))
}

/// A zone chosen: kept (held to what is offered) and, while Date is empty and the clock is
/// known, Date filled from the clock.
fn zone(state: &mut State, min: i16, view: &View<'_>) -> Effects<Msg, Out> {
    let moved = file::choose(state.settings_mut(), settings::Choice::Zone(min));
    if state.form().date_empty()
        && let Some(date) = state.today(view)
    {
        state.form_mut().fill_date(&date);
    }
    if moved {
        changed(settings::Changed::Choices)
    } else {
        Effects::none()
    }
}

/// Set: the local time typed, at :00 seconds, becomes the offset against the computer's own
/// clock, and the clock counts as checked. Never writes the computer's clock.
fn set(state: &mut State, view: &View<'_>) -> Effects<Msg, Out> {
    let Some((local, _, _)) = state.form().typed_local() else {
        return Effects::none();
    };
    let tz = state.settings().zone_min.unwrap_or(0);
    let s = state.settings_mut();
    s.offset_secs = clock::offset_for(local, tz, &view.clock);
    s.checked = true;
    state.show(settings::Page::Settings);
    changed(settings::Changed::Clock)
}

/// A key typed on Set the time.
fn key(state: &mut State, key: Key, cx: &Ctx<'_>) -> Effects<Msg, Out> {
    if state.page() == settings::Page::Settings {
        // The search takes what is typed while it is focused.
        if state.finding() {
            match key {
                Key::Char(c) => state.type_find(c),
                Key::Erase => state.erase_find(),
                Key::Escape => state.focus_find(false),
                _ => {}
            }
        }
        return Effects::none();
    }
    match state.form_mut().key(key) {
        Typed::Ignored | Typed::Taken => Effects::none(),
        // Enter sets whenever the date and time are real, with no zone chosen (read as UTC), as
        // `main` did; the Set button needs a zone.
        Typed::Set => set(state, &cx.view),
        Typed::Back if state.form().zones() => {
            state.form_mut().set_zones(false);
            Effects::none()
        }
        Typed::Back => {
            state.show(settings::Page::Settings);
            Effects::none()
        }
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
        assert!(matches!(asked.outs.as_slice(), [Out::ModeLock(_)]));
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

    /// Invariant: A lock settles the mode it carried at once, so an owner that outlives the scope knows its mode and holds no pick when the next lock comes, with no notice between.
    /// Regression: The Starter's owner keeps the mode it started in until a once-a-second notice, so a lock before it carries the old mode and drops the session's entries.
    /// Owner: app
    #[test]
    fn a_lock_settles_the_mode_it_carried_without_a_notice() {
        let device = TestView::device();
        let mut state = started();
        let _ = route::<Settings>(
            &mut state,
            Msg::Press(Press::Mode(ModeAsked::Signet)),
            &device.ctx(),
        );
        let _ = route::<Settings>(&mut state, Msg::Press(Press::ModeLock), &device.ctx());
        let mut first = lock_carry(&mut state);
        assert_eq!(first.take::<ModeAsked>(MODE), Some(ModeAsked::Signet));
        assert_eq!(state.mode_asked(), None);
        assert!(!state.pending());
        let mut second = lock_carry(&mut state);
        assert_eq!(second.take::<ModeAsked>(MODE), Some(ModeAsked::Signet));
    }

    fn on_set_time(view: &TestView) -> State {
        let mut state = started();
        let _ = route::<Settings>(&mut state, Msg::Press(Press::OpenSetTime), &view.ctx());
        assert_eq!(state.page(), settings::Page::SetTime);
        state
    }

    fn typed(state: &mut State, view: &TestView, text: &str) {
        for c in text.chars() {
            let _ = route::<Settings>(state, Msg::Typed(c), &view.ctx());
        }
    }

    /// Invariant: Set is offered only once a zone is chosen and a real date and a real 24-hour time are typed, and then it makes the offset, marks the clock checked, announces it and returns to Settings.
    /// Regression: Set works with no zone, with a date outside 2020 to 2099 or with a half-typed time, or leaves the clock unchecked, or stays on the screen.
    /// Owner: app
    #[test]
    fn set_needs_a_zone_and_a_real_time_and_then_checks_the_clock() {
        let view = TestView::device();
        let mut state = on_set_time(&view);
        let set = Msg::Press(Press::Set);
        let offered = |state: &State| Settings::refuses(state, &set, &view.ctx().view);
        typed(&mut state, &view, "12:00");
        // The date is empty: no zone, so no date was filled from the clock either.
        assert!(matches!(offered(&state), Some(Refusal::NotReady)));
        let _ = route::<Settings>(&mut state, Msg::Press(Press::Zone(60)), &view.ctx());
        // The zone fills Date from the clock, so a zone and a real time make Set offered.
        assert!(offered(&state).is_none());
        let _ = route::<Settings>(&mut state, Msg::Tab, &view.ctx());
        for _ in 0..10 {
            let _ = route::<Settings>(&mut state, Msg::Erase, &view.ctx());
        }
        typed(&mut state, &view, "2019-12-31");
        assert!(matches!(offered(&state), Some(Refusal::NotReady)));
        for _ in 0..10 {
            let _ = route::<Settings>(&mut state, Msg::Erase, &view.ctx());
        }
        typed(&mut state, &view, "2025-06-15");
        assert!(offered(&state).is_none());
        let out = seen(route::<Settings>(
            &mut state,
            Msg::Press(Press::Set),
            &view.ctx(),
        ));
        assert_eq!(out.outs.len(), 1);
        assert!(state.settings().checked);
        assert_ne!(state.settings().offset_secs, 0);
        assert_eq!(state.page(), settings::Page::Settings);
    }

    /// Invariant: Enter in a field sets whenever a real date and time are typed, with no zone chosen, as `main` did; a half-typed time does nothing.
    /// Regression: Enter is held to the zone and a typed time cannot be set from the keyboard, or Enter sets a time that is not real.
    /// Owner: app
    #[test]
    fn enter_sets_a_real_time_with_no_zone_chosen() {
        let view = TestView::device();
        let mut state = on_set_time(&view);
        typed(&mut state, &view, "12:00");
        let _ = route::<Settings>(&mut state, Msg::Tab, &view.ctx());
        typed(&mut state, &view, "2025-06");
        let _ = route::<Settings>(&mut state, Msg::Enter, &view.ctx());
        assert!(!state.settings().checked);
        assert_eq!(state.page(), settings::Page::SetTime);
        typed(&mut state, &view, "-15");
        let _ = route::<Settings>(&mut state, Msg::Enter, &view.ctx());
        assert!(state.settings().checked);
        assert_eq!(state.settings().zone_min, None);
        assert_eq!(state.page(), settings::Page::Settings);
    }

    /// Invariant: Keep this time needs a clock reading and marks the clock checked with no offset, and Escape closes the zone list before it leaves Set the time.
    /// Regression: Keep marks a clock that has no reading as checked, sets an offset, or Escape leaves the screen with the zone list open.
    /// Owner: app
    #[test]
    fn keep_needs_a_reading_and_escape_closes_the_zone_list_first() {
        let none = TestView::device().with_unix_secs(0);
        let refused = Settings::refuses(&started(), &Msg::Press(Press::Keep), &none.ctx().view);
        assert!(matches!(refused, Some(Refusal::NoClock)));
        let view = TestView::device();
        let mut state = on_set_time(&view);
        assert!(Settings::refuses(&state, &Msg::Press(Press::Keep), &view.ctx().view).is_none());
        let _ = route::<Settings>(&mut state, Msg::Press(Press::ZoneList(true)), &view.ctx());
        let _ = route::<Settings>(&mut state, Msg::Escape, &view.ctx());
        assert_eq!(state.page(), settings::Page::SetTime);
        let _ = route::<Settings>(&mut state, Msg::Press(Press::SetTimeBack), &view.ctx());
        assert_eq!(state.page(), settings::Page::Settings);
        let _ = route::<Settings>(&mut state, Msg::Press(Press::OpenSetTime), &view.ctx());
        let _ = route::<Settings>(&mut state, Msg::Press(Press::Keep), &view.ctx());
        assert!(state.settings().checked);
        assert_eq!(state.settings().offset_secs, 0);
    }

    /// Invariant: The zone steps a quarter hour at a time and stops at -12:00 and +14:00; opening the zone list from Settings opens Set the time.
    /// Regression: The zone steps past either end, or the Time card's zone row opens a list with no screen to show it.
    /// Owner: app
    #[test]
    fn the_zone_steps_to_its_ends_and_its_list_opens_set_the_time() {
        let view = TestView::device();
        let mut state = started();
        let _ = route::<Settings>(&mut state, Msg::Press(Press::ZoneList(true)), &view.ctx());
        assert_eq!(state.page(), settings::Page::SetTime);
        for _ in 0..70 {
            let _ = route::<Settings>(&mut state, Msg::Press(Press::ZoneStep(-15)), &view.ctx());
        }
        assert_eq!(state.settings().zone_min, Some(-720));
        for _ in 0..120 {
            let _ = route::<Settings>(&mut state, Msg::Press(Press::ZoneStep(15)), &view.ctx());
        }
        assert_eq!(state.settings().zone_min, Some(840));
    }

    /// Invariant: The Mode card asks the Lock sheet at once for a mode that is offered and not in use; the one in use is refused.
    /// Regression: A pick on the Mode card changes the mode with no Lock sheet, or the mode in use is asked for again.
    /// Owner: app
    #[test]
    fn a_pick_on_the_mode_card_asks_the_lock_sheet() {
        let view = TestView::device();
        let mut state = started();
        let out = seen(route::<Settings>(
            &mut state,
            Msg::Press(Press::ModeAsk(ModeAsked::Signet)),
            &view.ctx(),
        ));
        assert_eq!(out.outs.len(), 1);
        assert_eq!(state.mode_asked(), Some(ModeAsked::Signet));
        let mut carried = lock_carry(&mut state);
        assert_eq!(carried.take::<ModeAsked>(MODE), Some(ModeAsked::Signet));
        let in_use = Settings::refuses(
            &started(),
            &Msg::Press(Press::ModeAsk(ModeAsked::of_index(
                view.ctx().view.mode.index(),
            ))),
            &view.ctx().view,
        );
        assert!(matches!(in_use, Some(Refusal::ModeInUse)));
    }
}
