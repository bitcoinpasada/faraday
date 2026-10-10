//! Settings on the stick: the boot stick's `faraday-settings.txt` is read
//! at boot, before the passphrase, taking only what Settings itself
//! offers; a stick visit writes the settings back to the boot stick when
//! they changed; and a lock does not read the file again over settings
//! changed since.

use faraday_core::stick_settings::FILE;
use faraday_core::ui::Theme;
use faraday_core::{Action, Faraday, Screen, StickInfo, StorageCommand, StorageEvent};
use osk_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

fn display() -> Event {
    Event::Display(DisplayInfo {
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
    })
}

fn stick(id: &str, boot: bool, file: Option<&[u8]>) -> StickInfo {
    StickInfo {
        id: id.into(),
        label: id.to_uppercase(),
        boot,
        files: file
            .map(|f| vec![(FILE.to_string(), f.len() as u64)])
            .unwrap_or_default(),
    }
}

/// Answers the app's storage requests: reads of the settings file with
/// `file`, writes by keeping them. Returns what was written, by name.
fn pump(app: &mut Faraday, file: Option<&[u8]>) -> Vec<(String, String, Vec<u8>)> {
    let mut wrote = Vec::new();
    while let Some(c) = app.poll_storage() {
        match c {
            StorageCommand::Read { stick, name } if name == FILE => {
                if let Some(bytes) = file {
                    app.storage(StorageEvent::Read {
                        stick,
                        name,
                        bytes: bytes.to_vec(),
                    });
                }
            }
            StorageCommand::Write { stick, name, bytes } => {
                app.storage(StorageEvent::Written {
                    stick: stick.clone(),
                    name: name.clone(),
                    wrote_as: name.clone(),
                });
                wrote.push((stick, name, bytes));
            }
            _ => {}
        }
    }
    wrote
}

/// A fresh process that boots from a stick holding `file`.
fn booted(file: Option<&[u8]>) -> Faraday {
    let mut app = Faraday::new();
    app.event(display());
    app.storage(StorageEvent::Sticks(vec![stick("boot", true, file)]));
    pump(&mut app, file);
    app
}

/// The settings a fresh process starts with.
fn defaults() -> Faraday {
    let mut app = Faraday::new();
    app.event(display());
    app
}

fn same_settings(a: &Faraday, b: &Faraday) -> bool {
    a.theme == b.theme
        && a.scale_pct == b.scale_pct
        && a.reduce_motion == b.reduce_motion
        && a.guided == b.guided
        && a.qr_frame_ms == b.qr_frame_ms
        && a.idle_lock_min == b.idle_lock_min
        && a.idle_off_min == b.idle_off_min
        && a.seal_amounts == b.seal_amounts
}

#[test]
fn the_boot_sticks_settings_apply_before_the_passphrase() {
    let file = b"faraday-settings 1\nscale=125\ntheme=gruvbox\nmotion=reduced\nidle-lock=30\nidle-off=60\nqr-ms=200\n";
    let mut app = booted(Some(file));
    assert_eq!(app.theme, Theme::Gruvbox);
    assert_eq!(app.scale_pct, 125);
    assert!(app.reduce_motion);
    assert_eq!((app.idle_lock_min, app.idle_off_min), (30, 60));
    assert_eq!(app.qr_frame_ms, 200);
    // Pulling the stick asks for nothing the settings did not already set.
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert_eq!(app.theme, Theme::Gruvbox);
}

#[test]
fn a_file_cannot_turn_a_protection_off_or_set_what_settings_does_not_offer() {
    let file = b"faraday-settings 1\r\nidle-lock=0\r\nidle-off=0\r\nseal-amounts=0\r\nnetwork=testnet\r\nscale=999\r\ntheme=hotdog\r\nqr-ms=1\r\nidle-lock=7\r\nmotion=sometimes\r\n";
    let app = booted(Some(file));
    assert!(same_settings(&app, &defaults()));
}

#[test]
fn a_file_too_large_or_of_another_format_is_ignored_whole() {
    let mut large = b"faraday-settings 1\ntheme=nord\n".to_vec();
    large.resize(5000, b'\n');
    let mut wrong = b"faraday-settings 2\ntheme=light\n".to_vec();
    let not_text = b"faraday-settings 1\ntheme=light\n\xff\xfe\n".to_vec();
    for file in [&large, &wrong, &not_text] {
        assert!(same_settings(&booted(Some(file)), &defaults()));
    }
    wrong.clear();
    assert!(same_settings(&booted(Some(&wrong)), &defaults()));
}

#[test]
fn a_changed_setting_is_written_to_the_boot_stick_and_reads_back_the_same() {
    let mut app = booted(None);
    app.press(Action::Theme(Theme::RosePine));
    app.press(Action::Scale(150));
    app.press(Action::Nav(Screen::Visit));
    app.press(Action::VisitWrite);
    let wrote = pump(&mut app, None);
    let (_, _, bytes) = wrote
        .iter()
        .find(|(s, n, _)| s == "boot" && n == FILE)
        .expect("the settings are written to the boot stick");
    let back = booted(Some(bytes));
    assert!(same_settings(&back, &app));
    // Written, they match the stick again: the next visit leaves them.
    app.press(Action::VisitWrite);
    assert!(pump(&mut app, None).iter().all(|(_, n, _)| n != FILE));
}

#[test]
fn every_theme_round_trips_through_the_settings_file() {
    for theme in Theme::ALL {
        let mut app = booted(None);
        app.press(Action::Theme(theme));
        app.press(Action::Nav(Screen::Visit));
        // A changed theme ticks the settings row by default; the
        // default theme itself does not, so it is ticked here.
        if !app.visit_settings_on() {
            app.press(Action::VisitSettings);
        }
        app.press(Action::VisitWrite);
        let wrote = pump(&mut app, None);
        let (_, _, bytes) = wrote
            .iter()
            .find(|(s, n, _)| s == "boot" && n == FILE)
            .unwrap_or_else(|| panic!("{} is not written to the boot stick", theme.name()));
        let back = booted(Some(bytes));
        assert_eq!(back.theme, theme, "{} did not read back", theme.name());
    }
}

#[test]
fn unchanged_settings_are_not_written_unless_ticked() {
    let mut app = booted(Some(b"faraday-settings 1\ntheme=nord\n"));
    app.press(Action::Nav(Screen::Visit));
    app.press(Action::VisitWrite);
    assert!(pump(&mut app, None).iter().all(|(_, n, _)| n != FILE));
    app.press(Action::VisitSettings);
    app.press(Action::VisitWrite);
    assert!(pump(&mut app, None).iter().any(|(_, n, _)| n == FILE));
}

#[test]
fn another_stick_gets_the_settings_only_when_ticked() {
    let mut app = defaults();
    app.storage(StorageEvent::Sticks(vec![stick("other", false, None)]));
    pump(&mut app, None);
    app.press(Action::Theme(Theme::Light));
    app.press(Action::Nav(Screen::Visit));
    app.press(Action::VisitWrite);
    assert!(pump(&mut app, None).iter().all(|(_, n, _)| n != FILE));
    app.press(Action::VisitSettings);
    app.press(Action::VisitWrite);
    assert!(
        pump(&mut app, None)
            .iter()
            .any(|(s, n, _)| s == "other" && n == FILE)
    );
}

#[test]
fn the_settings_file_is_not_copied_into_the_inbox() {
    let mut app = booted(Some(b"faraday-settings 1\ntheme=nord\n"));
    app.press(Action::Nav(Screen::Visit));
    // Every file comes in ticked but the settings, which are not offered.
    assert!(!app.visit.inn.contains(FILE));
    app.press(Action::VisitCopy);
    pump(&mut app, Some(b"faraday-settings 1\ntheme=nord\n"));
    assert!(app.inbox.iter().all(|i| i.name != FILE));
}

#[test]
fn after_a_lock_the_boot_stick_is_not_read_again() {
    let mut app = booted(Some(b"faraday-settings 1\ntheme=nord\n"));
    app.press(Action::Theme(Theme::Catppuccin));
    // What the process keeps for the next one, as a shell carries it.
    let mut kept = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { kept: k, .. } = c {
            kept = k;
        }
    }
    let mut next = Faraday::new();
    next.event(display());
    next.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: Vec::new(),
        kept,
    });
    let file = b"faraday-settings 1\ntheme=light\n";
    next.storage(StorageEvent::Sticks(vec![stick("boot", true, Some(file))]));
    pump(&mut next, Some(file));
    assert_eq!(next.theme, Theme::Catppuccin);
    // And the settings changed since boot are still offered to the stick.
    next.press(Action::Nav(Screen::Visit));
    next.press(Action::VisitWrite);
    assert!(pump(&mut next, None).iter().any(|(_, n, _)| n == FILE));
}
