//! On the Pi every removable medium is an SD card, and the app says so:
//! the stick shell starts it with `Medium::SdCard`. The PC keeps "stick".

use faraday_core::{Action, Faraday, Medium, StickInfo, StorageEvent};
use osk_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

/// The app as a shell starts it on the Pi's 480 × 640 panel, drawn once.
fn panel(medium: Medium) -> Faraday {
    let mut app = Faraday::new();
    app.medium = medium;
    app.event(Event::Display(DisplayInfo {
        width: 480,
        height: 640,
        dpi: 286,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
    app
}

fn attach(app: &mut Faraday, on: bool) {
    let sticks = if on {
        vec![StickInfo {
            id: "a".into(),
            label: "PHOTOS".into(),
            boot: false,
            files: Vec::new(),
        }]
    } else {
        Vec::new()
    };
    app.storage(StorageEvent::Sticks(sticks));
    let _ = app.frame();
}

#[test]
fn the_pi_s_home_says_no_sd_card() {
    let app = panel(Medium::SdCard);
    assert_eq!(app.home_files_line(), "0 in · 0 out · no SD card");
    let pc = panel(Medium::Stick);
    assert_eq!(pc.home_files_line(), "0 in · 0 out · no stick");
}

#[test]
fn the_pi_names_an_sd_card_with_its_article_and_at_the_start_of_a_line() {
    let mut app = panel(Medium::SdCard);
    // Write to an SD card with none attached.
    app.press(Action::WriteAsk);
    assert_eq!(
        app.toast_text(),
        Some("Plug in an SD card: the visit writes the Outbox")
    );
    attach(&mut app, true);
    attach(&mut app, false);
    assert_eq!(app.toast_text(), Some("SD card removed"));

    let mut pc = panel(Medium::Stick);
    pc.press(Action::WriteAsk);
    assert_eq!(
        pc.toast_text(),
        Some("Plug in a stick: the visit writes the Outbox")
    );
    attach(&mut pc, true);
    attach(&mut pc, false);
    assert_eq!(pc.toast_text(), Some("Stick removed"));
}

#[test]
fn the_pi_calls_the_card_it_started_from_the_boot_sd_card() {
    let mut app = panel(Medium::SdCard);
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        id: "boot".into(),
        label: "OSKDATA".into(),
        boot: true,
        files: Vec::new(),
    }]));
    let _ = app.frame();
    assert_eq!(app.home_files_line(), "0 in · 0 out · Boot SD card");
}
