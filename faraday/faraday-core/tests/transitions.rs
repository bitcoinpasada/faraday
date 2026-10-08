//! How changes show (`docs/MOTION.md` §5): a new screen is there at once,
//! with no fade, and a step card grows open and comes to rest where it
//! would have jumped to.

use faraday_core::{Action, Faraday, Screen, StorageEvent, testkit};
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
    app.event(Event::Tick { now_ms: 1_000 });
    let _ = app.frame();
    app
}

/// Ticks every 16 ms from `from` to `to`, drawing each frame as a shell
/// does, and returns the last.
fn frames(app: &mut Faraday, from: u64, to: u64) -> Vec<u8> {
    let mut now = from;
    loop {
        app.event(Event::Tick { now_ms: now });
        while app.poll_command().is_some() {}
        let _ = app.frame();
        if now >= to {
            return app.frame().rgba.to_vec();
        }
        now = (now + 16).min(to);
    }
}

#[test]
fn a_new_screen_is_there_at_once() {
    let mut app = shown();
    app.press(Action::Nav(Screen::Settings));
    let first = frames(&mut app, 1_016, 1_016);
    let rest = frames(&mut app, 1_032, 2_000);
    assert!(first == rest, "the first frame is the screen at rest");
}

#[test]
fn a_step_card_grows_open_and_comes_to_rest() {
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let mut app = shown();
    app.storage(StorageEvent::Restored {
        inbox: vec![("spending-wallet.txt".into(), kit.descriptor.into_bytes())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
    app.press(Action::Backup(0));
    let _ = frames(&mut app, 1_016, 2_000);
    app.press(Action::BStep(2));
    let early = frames(&mut app, 2_016, 2_048);
    let rest = frames(&mut app, 2_064, 2_600);
    assert!(early != rest, "part of the way open");
    assert!(frames(&mut app, 2_616, 3_200) == rest, "and then still");
    assert!(
        app.where_offered(Action::BStep(2)).is_some(),
        "its header is still there to close it"
    );
}
