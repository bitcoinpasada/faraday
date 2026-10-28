//! Rotation on a panel that can turn (`docs/ORIENTATION.md` §6 to §9):
//! offered only where the shell says it can turn its frame, kept on the
//! stick and asked for again at the next start, and laid out at the
//! class the turned width gives, with the session kept.

use faraday_core::nav::Nav;
use faraday_core::stick::settings::FILE;
use faraday_core::{Action, Faraday, Screen, StickInfo, StorageCommand, StorageEvent, testkit};
use faraday_shell_api::{
    App, BootState, Command, DisplayInfo, Event, Key, Orientation, SecureHardware,
};

/// A panel `width × height` pixels at `dpi`, as the stick reports it.
fn display(width: u16, height: u16, dpi: u16) -> Event {
    Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    })
}

/// The stick shell's answer to the app's commands on a panel `native`
/// pixels across and down, landscape natively: a rotation asked for is
/// turned and the turned size told. Returns the rotations asked for.
fn shell(app: &mut Faraday, native: (u16, u16), dpi: u16) -> Vec<Orientation> {
    let mut asked = Vec::new();
    while let Some(c) = app.poll_command() {
        if let Command::Orientation(o) = c {
            asked.push(o);
            let (w, h) = match o {
                Orientation::Landscape => native,
                Orientation::Portrait => (native.1, native.0),
            };
            app.event(display(w, h, dpi));
        }
    }
    let _ = app.frame();
    asked
}

/// A fresh app on a panel `native` in size that can turn, the panel
/// handed over landscape.
fn on_panel(native: (u16, u16), dpi: u16) -> Faraday {
    let mut app = Faraday::new();
    app.event(display(native.0, native.1, dpi));
    app.event(Event::RotationOffered {
        native: Orientation::Landscape,
    });
    assert_eq!(
        shell(&mut app, native, dpi),
        vec![Orientation::Landscape],
        "the kept choice is asked for once, after the offer"
    );
    app
}

fn boot_stick(file: Option<&[u8]>) -> StickInfo {
    StickInfo {
        serial: None,
        id: "boot".into(),
        label: "BOOT".into(),
        boot: true,
        files: file
            .map(|f| vec![(FILE.to_string(), f.len() as u64)])
            .unwrap_or_default(),
    }
}

/// Answers the app's storage requests: reads of the settings file with
/// `file`, writes kept. Returns the settings file written, if one was.
fn pump(app: &mut Faraday, file: Option<&[u8]>) -> Option<Vec<u8>> {
    let mut wrote = None;
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
            StorageCommand::Write {
                stick, name, bytes, ..
            } => {
                app.storage(StorageEvent::Written {
                    stick,
                    name: name.clone(),
                    wrote_as: name.clone(),
                });
                if name == FILE {
                    wrote = Some(bytes);
                }
            }
            _ => {}
        }
    }
    wrote
}

/// A fresh process on the 7-inch panel booting from a stick holding
/// `file`: the rotations it asks the shell for as the boot stick is
/// read, before any input.
fn booted(file: &[u8]) -> (Faraday, Vec<Orientation>) {
    let mut app = on_panel((1024, 600), 170);
    app.storage(StorageEvent::Sticks(vec![boot_stick(Some(file))]));
    pump(&mut app, Some(file));
    let asked = shell(&mut app, (1024, 600), 170);
    (app, asked)
}

#[test]
fn the_rotation_survives_a_restart() {
    let mut app = on_panel((1024, 600), 170);
    app.storage(StorageEvent::Sticks(vec![boot_stick(None)]));
    pump(&mut app, None);
    app.press(Action::Nav(Screen::Settings));
    app.press(Action::Rotation(Orientation::Portrait));
    assert_eq!(
        shell(&mut app, (1024, 600), 170),
        vec![Orientation::Portrait]
    );
    assert_eq!(app.orientation(), Orientation::Portrait);
    // The next stick visit offers the changed settings, and writes them.
    app.press(Action::Nav(Screen::Visit));
    app.press(Action::VisitWrite);
    let file = pump(&mut app, None).expect("the settings file is written");
    let text = String::from_utf8(file.clone()).expect("text");
    assert!(text.contains("\nrotation=portrait\n"), "{text}");
    // A new process given that file asks for Portrait as the boot stick
    // is read, before anything is touched.
    let (back, asked) = booted(&file);
    assert_eq!(asked, vec![Orientation::Portrait]);
    assert_eq!(back.orientation(), Orientation::Portrait);
    // A value Settings does not offer leaves Landscape, and asks nothing.
    for file in [
        b"faraday-settings 1\nrotation=sideways\n".as_slice(),
        b"faraday-settings 1\nrotation=\n".as_slice(),
        b"faraday-settings 1\nrotation=Portrait\n".as_slice(),
    ] {
        let (app, asked) = booted(file);
        assert_eq!(app.orientation(), Orientation::Landscape);
        assert!(asked.is_empty(), "{asked:?}");
    }
}

#[test]
fn the_layout_follows_the_turned_width() {
    // The 7-inch panel at its own density: the rail in landscape, the
    // navigation bar at the foot in portrait.
    let mut app = on_panel((1024, 600), 170);
    assert_eq!(app.nav(), Nav::Rail, "landscape");
    app.press(Action::Rotation(Orientation::Portrait));
    shell(&mut app, (1024, 600), 170);
    assert_eq!(app.nav(), Nav::Bar, "portrait");
    // The 10-inch panel: a rail at the left either way.
    let mut app = on_panel((1280, 800), 150);
    assert!(
        matches!(app.nav(), Nav::Rail | Nav::ExpandedRail),
        "landscape"
    );
    app.press(Action::Rotation(Orientation::Portrait));
    shell(&mut app, (1280, 800), 150);
    assert_eq!(app.nav(), Nav::Rail, "portrait");
}

#[test]
fn rotation_is_on_settings_only_where_the_shell_offers_it() {
    let rotation = Action::Rotation(Orientation::Portrait);
    let offered = |app: &mut Faraday| {
        app.press(Action::Nav(Screen::Settings));
        let _ = app.frame();
        app.where_offered(rotation).is_some()
    };
    // The desktop app, and the Pi's panel: no offer, no row.
    let mut desktop = testkit::started();
    assert!(!offered(&mut desktop));
    let mut pi = Faraday::new();
    pi.event(display(480, 640, 286));
    assert!(!offered(&mut pi));
    // Pressed anyway, nothing is asked of a shell that cannot turn.
    pi.press(rotation);
    assert!(shell(&mut pi, (480, 640), 286).is_empty());
    // The 10-inch panel offers it.
    let mut panel = on_panel((1280, 800), 150);
    assert!(offered(&mut panel));
    assert!(panel.rotation_offered());
}

#[test]
fn a_turn_keeps_the_session_and_settings() {
    let mut app = on_panel((1024, 600), 170);
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    assert!(app.holds_secret(), "the key loaded");
    app.press(Action::Nav(Screen::Settings));
    for o in [Orientation::Portrait, Orientation::Landscape] {
        app.press(Action::Rotation(o));
        assert_eq!(shell(&mut app, (1024, 600), 170), vec![o]);
        assert!(app.holds_secret(), "{o:?}: the key is still loaded");
        assert_eq!(app.screen, Screen::Settings, "{o:?}: Settings on screen");
    }
    // In portrait, a compact window, Display opens as its own page.
    app.press(Action::Rotation(Orientation::Portrait));
    shell(&mut app, (1024, 600), 170);
    assert!(
        app.where_offered(Action::Rotation(Orientation::Landscape))
            .is_some()
    );
}
