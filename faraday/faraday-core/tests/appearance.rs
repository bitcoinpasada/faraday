//! Appearance on Settings (`docs/MOTION.md` §3.6): Light draws the
//! screens dark on light and Dark turns them back; the choice is kept
//! across a lock with the other settings.

use faraday_core::ui::Theme;
use faraday_core::{Action, Faraday, Screen, StorageCommand, StorageEvent};
use osk_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

fn shown() -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1280,
        height: 800,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.press(Action::Nav(Screen::Settings));
    app
}

/// How light the page is in the middle of the screen's empty right
/// margin, 0 to 255, once the change has faded in: ticked for half a
/// second as a shell ticks.
fn page_light(app: &mut Faraday, clock: &mut u64) -> u8 {
    for _ in 0..30 {
        *clock += 16;
        app.event(Event::Tick { now_ms: *clock });
        while app.poll_command().is_some() {}
        let _ = app.frame();
    }
    let frame = app.frame();
    let i = (790 * usize::from(frame.width) + 1270) * 4;
    let px = &frame.rgba[i..i + 3];
    ((u16::from(px[0]) + u16::from(px[1]) + u16::from(px[2])) / 3) as u8
}

#[test]
fn light_draws_the_page_light_and_dark_turns_it_back() {
    let mut app = shown();
    let mut clock = 1_000;
    assert!(page_light(&mut app, &mut clock) < 64, "dark at first");
    assert!(app.offers(Action::Theme(Theme::Light)));
    app.press(Action::Theme(Theme::Light));
    assert!(page_light(&mut app, &mut clock) > 192, "light");
    app.press(Action::Theme(Theme::Dark));
    assert!(page_light(&mut app, &mut clock) < 64, "dark again");
}

#[test]
fn the_choice_is_kept_across_a_lock() {
    let mut app = shown();
    app.press(Action::Theme(Theme::Light));
    let mut kept = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { kept: k, .. } = c {
            kept = k;
        }
    }
    let mut next = Faraday::new();
    next.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: Vec::new(),
        kept,
    });
    assert_eq!(next.theme, Theme::Light);
}

#[test]
fn reduce_motion_is_kept_across_a_lock() {
    let mut app = shown();
    let _ = app.frame();
    assert!(app.offers(Action::ReduceMotion(true)));
    app.press(Action::ReduceMotion(true));
    let mut kept = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { kept: k, .. } = c {
            kept = k;
        }
    }
    let mut next = Faraday::new();
    next.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: Vec::new(),
        kept,
    });
    assert!(next.reduce_motion);
}
