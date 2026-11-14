//! The Linux Starter's own close button (F266, `docs/STARTER.md` §3.1d):
//! a press is what the window's close is.

use faraday_core::Mode;
use faraday_host::Host;
use faraday_shell_api::{App, Event, TouchPhase};
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

fn commands(s: &mut Starter) -> Vec<StarterCommand> {
    std::iter::from_fn(|| s.next_command()).collect()
}

/// A press on the bar's close button, as a finger or the mouse gives it.
fn press_close(s: &mut Starter) {
    let _ = s.frame();
    let (x, y) = s
        .where_offered(StarterAction::Close)
        .expect("the bar offers Close");
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        s.event(Event::Touch { x, y, phase });
    }
}

#[test]
fn the_linux_close_button_closes_the_window_and_windows_has_none() {
    // F266: on Linux a press is what the window's close is.
    let mut s = starter(Os::Linux);
    press_close(&mut s);
    assert_eq!(commands(&mut s), vec![StarterCommand::Close]);

    // It stays pressable under the menu and under a sheet.
    let mut s = starter(Os::Linux);
    s.press(StarterAction::Menu);
    press_close(&mut s);
    assert_eq!(commands(&mut s), vec![StarterCommand::Close]);
    let mut s = starter(Os::Linux);
    s.page = Page::Start;
    s.press(StarterAction::Guide);
    assert!(s.sheet.is_some());
    press_close(&mut s);
    assert_eq!(commands(&mut s), vec![StarterCommand::Close]);

    // Windows draws its own title bar and the bar has no button.
    let mut s = starter(Os::Windows);
    let _ = s.frame();
    assert_eq!(s.where_offered(StarterAction::Close), None);
}

#[test]
fn the_close_button_in_faraday_demo_returns_to_home() {
    // F266: in Faraday Demo the window's close goes back to Home (F126).
    let mut s = starter(Os::Linux);
    s.press(StarterAction::OpenDesktop);
    assert_eq!(commands(&mut s), vec![StarterCommand::OpenDemo]);
    s.show_demo(Host::new(faraday_desktop::desktop_app_in(Mode::Demo)));
    press_close(&mut s);
    assert!(!s.showing_demo());
    assert_eq!(s.page, Page::Home);
    assert_eq!(commands(&mut s), vec![StarterCommand::DemoClosed]);
}
