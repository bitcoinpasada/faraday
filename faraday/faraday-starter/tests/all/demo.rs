//! Faraday Demo inside the Starter (F126, `docs/STARTER.md` §5.3): Open
//! Faraday Demo on Home shows the Demo, its host's strip and its app, in
//! the Starter's own window under the top app bar; the way back returns
//! to Home; and a theme chosen in the Demo is the Starter's.

use faraday_core::ui::{Contrast, Mode, Theme, ThemeName};
use faraday_core::{Action, Host, Screen};
use faraday_shell_api::{App, Event, Key, TouchPhase};
use faraday_starter::api::{Computer, Os, StarterCommand, StarterEvent};
use faraday_starter::{Page, Starter, StarterAction};

use crate::starter::display;

fn starter(os: Os) -> Starter {
    let mut s = Starter::new();
    s.event(display(1280, 800, 160));
    s.starter_event(StarterEvent::Computer(Computer {
        os,
        ..Computer::default()
    }));
    s
}

fn shows(s: &mut Starter, text: &str) -> bool {
    s.drawn_texts().iter().any(|t| t.contains(text))
}

fn commands(s: &mut Starter) -> Vec<StarterCommand> {
    std::iter::from_fn(|| s.next_command()).collect()
}

/// Open Faraday Demo pressed, and the shell's answer: the Demo's app as
/// Faraday Demo starts it, held by its host.
fn opened(s: &mut Starter) {
    s.press(StarterAction::OpenDesktop);
    assert_eq!(commands(s), vec![StarterCommand::OpenDemo]);
    let app = faraday_desktop::desktop_app_in(faraday_core::Mode::Demo);
    s.show_demo(Host::new(app));
}

fn touch(s: &mut Starter, (x, y): (u16, u16)) {
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        s.event(Event::Touch { x, y, phase });
    }
}

#[test]
fn open_faraday_demo_shows_the_demo_in_the_starters_window_and_back_returns_home() {
    let mut s = starter(Os::Linux);
    assert!(shows(&mut s, "Open Faraday Demo"));
    opened(&mut s);
    assert!(s.showing_demo());
    // The Starter's bar names it, and under it are the Demo's strip and
    // its app in Practice.
    let texts = s.drawn_texts();
    assert!(texts.iter().any(|t| t == "Faraday Demo"), "{texts:?}");
    assert!(texts.iter().any(|t| t.contains("Practice")), "{texts:?}");
    assert!(!texts.iter().any(|t| t.contains("Open Faraday Demo")));

    // A press on the Demo's frame is the Demo's.
    let _ = s.frame();
    let at = s
        .where_demo_offers(Action::Nav(Screen::Settings))
        .expect("the Demo offers Settings");
    touch(&mut s, at);
    assert_eq!(s.demo().expect("open").app.screen, Screen::Settings);
    // Its keys are its own: Escape takes the Demo back, not the window.
    s.event(Event::Key(Key::Escape));
    assert!(s.showing_demo());

    // The bar's way back returns to Home, and the shell lets the Demo go.
    let _ = s.frame();
    let back = s.where_offered(StarterAction::Back).expect("a way back");
    touch(&mut s, back);
    assert!(!s.showing_demo());
    assert_eq!(s.page, Page::Home);
    assert_eq!(commands(&mut s), vec![StarterCommand::DemoClosed]);
    assert!(shows(&mut s, "Open Faraday Demo"));

    // Opened again, it is a fresh session; closing the window or the
    // Demo powering off also returns to Home.
    opened(&mut s);
    assert_eq!(s.demo().expect("open").app.screen, Screen::Home);
    s.close_demo();
    assert!(!s.showing_demo());
    assert_eq!(commands(&mut s), vec![StarterCommand::DemoClosed]);
}

#[test]
fn faraday_demo_is_offered_on_linux_and_windows_and_not_on_macos() {
    for (os, offered) in [(Os::Linux, true), (Os::Windows, true), (Os::MacOs, false)] {
        let mut s = starter(os);
        assert_eq!(s.offers_desktop(), offered, "{os:?}");
        s.press(StarterAction::OpenDesktop);
        assert_eq!(
            commands(&mut s).contains(&StarterCommand::OpenDemo),
            offered,
            "{os:?}"
        );
    }
}

#[test]
fn a_theme_chosen_in_the_demo_is_the_starters_and_stays_after_leaving_it() {
    let mut s = starter(Os::Linux);
    let dark = Theme::new(ThemeName::Rose, Mode::Dark, Contrast::Medium);
    let start = s.theme();
    assert_ne!(start, dark);
    // The Demo opens in the Starter's theme.
    opened(&mut s);
    assert_eq!(s.demo().expect("open").app.theme(), start);

    // Chosen in the Demo's Settings, it is the Starter's at the next
    // event.
    s.demo().expect("open").app.press(Action::Theme(dark));
    s.event(Event::Tick { now_ms: 1_000 });
    assert_eq!(s.theme(), dark);

    // Left, the Starter keeps it, and the next Demo opens in it.
    s.press(StarterAction::Back);
    assert!(!s.showing_demo());
    assert_eq!(commands(&mut s), vec![StarterCommand::DemoClosed]);
    assert_eq!(s.theme(), dark);
    opened(&mut s);
    assert_eq!(s.demo().expect("open").app.theme(), dark);
}
