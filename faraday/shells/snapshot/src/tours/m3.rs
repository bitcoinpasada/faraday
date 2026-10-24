//! The Material 3 tour (`docs/decisions/F108.md`): Home, Settings with
//! its segmented buttons and its Appearance, the power-off dialog, a
//! flow's title row with **Steps only · Guided** and **Cancel**, the
//! dialog Cancel brings up, in Cobalt and then in High contrast dark and
//! light, where nothing see-through lowers the contrast, Set the time
//! with its Set button not yet available and then the toast its Set
//! leaves, and last the keyboard's place ringed on Home.

use crate::tour::{Tour, scroll_to, type_text};
use faraday_core::set_time::Focus;
use faraday_core::testkit;
use faraday_core::{Action, Screen};
use faraday_shell_api::{App, Event, Key};

pub(crate) fn m3_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::cstep;
    for n in 0..3 {
        t.press(Action::Entry(None));
        type_text(t, &testkit::test_words(testkit::TEST_SEEDS[n].0));
        t.press(Action::EntryAdd);
    }
    t.press(Action::Nav(Screen::Home));
    t.shot("m3-1-home")?;
    t.press(Action::Nav(Screen::Settings));
    t.shot("m3-2-settings")?;
    scroll_to(t, Action::ReduceMotion(false));
    t.shot("m3-3-settings-motion")?;
    t.press(Action::PowerAsk);
    t.shot("m3-4-power-dialog")?;
    t.press(Action::Cancel);
    let multi = faraday_core::create::NewKind::ALL
        .iter()
        .position(|k| *k == faraday_core::create::NewKind::Multi)
        .ok_or("no multisig")? as u8;
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::CreateWallet);
    t.press(Action::CKind(multi));
    t.press(Action::CNext(cstep::KIND));
    t.press(Action::CVaults(false));
    t.press(Action::CNext(cstep::WHERE));
    t.shot("m3-5-flow")?;
    t.press(Action::CancelFlow);
    t.shot("m3-6-cancel-dialog")?;
    for (theme, name) in [
        (
            faraday_core::ui::Preset::HighContrastDark,
            "m3-7-hc-dark-dialog",
        ),
        (
            faraday_core::ui::Preset::HighContrastLight,
            "m3-8-hc-light-dialog",
        ),
    ] {
        t.press(Action::Theme(theme));
        t.shot(name)?;
    }
    t.press(Action::Cancel);
    t.shot("m3-9-hc-light-flow")?;
    t.press(Action::Theme(faraday_core::ui::Preset::Cobalt));
    t.press(Action::Nav(Screen::Settings));
    t.press(Action::OpenSetTime);
    t.shot("m3-10-set-time-off")?;
    t.press(Action::SetTimeFocus(Focus::Date));
    for _ in 0..12 {
        t.app.event(Event::Key(Key::Backspace));
    }
    type_text(t, "2026-10-23");
    t.press(Action::SetTimeFocus(Focus::Time));
    type_text(t, "14:30");
    t.press(Action::SetTimeSet);
    // Inside its 4 s on screen.
    t.shot_after("m3-11-toast", 1000)?;
    // The keyboard's place, moved twice from none, last: a scripted
    // press does not take it off as a pointer's does.
    t.press(Action::Nav(Screen::Home));
    t.app.event(Event::Key(Key::Tab));
    t.app.event(Event::Key(Key::Tab));
    t.pump();
    t.shot("m3-12-home-focus")
}
