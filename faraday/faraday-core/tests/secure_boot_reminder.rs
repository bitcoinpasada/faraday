//! At power-off a stick that started with Secure Boot off says so; with it
//! on or unread, and on the Pi or the desktop app, it says nothing.

use faraday_core::{Action, Faraday, Medium, SecureBootAtStart};
use osk_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

fn shown(mut app: Faraday) -> Vec<String> {
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
    let _ = app.frame();
    app.drawn_texts()
}

const LINE: &str = "Secure Boot is off. Turn it back on before starting Windows.";

fn power_sheet_texts(medium: Medium, secure_boot: SecureBootAtStart) -> Vec<String> {
    let mut app = Faraday::new();
    app.medium = medium;
    app.secure_boot = secure_boot;
    app.press(Action::PowerAsk);
    shown(app)
}

#[test]
fn the_power_off_sheet_reminds_when_secure_boot_was_off() {
    let t = power_sheet_texts(Medium::Stick, SecureBootAtStart::Off);
    assert!(t.iter().any(|s| s == LINE), "{t:?}");
}

#[test]
fn it_says_nothing_when_secure_boot_was_on_or_unknown() {
    for state in [SecureBootAtStart::On, SecureBootAtStart::Unknown] {
        let t = power_sheet_texts(Medium::Stick, state);
        assert!(!t.iter().any(|s| s == LINE), "{state:?}");
    }
}

#[test]
fn the_pi_never_shows_it() {
    let t = power_sheet_texts(Medium::SdCard, SecureBootAtStart::Off);
    assert!(!t.iter().any(|s| s == LINE));
}

#[test]
fn the_desktop_app_never_shows_it() {
    // The desktop shell never reads the state: the app keeps its default.
    let mut app = Faraday::new();
    app.press(Action::PowerAsk);
    assert!(!shown(app).iter().any(|s| s == LINE));
}

#[test]
fn the_byte_is_read_as_2_off_and_3_on() {
    let with = |b: u8| {
        let mut d = vec![0u8; 0x200];
        d[0x1ec] = b;
        SecureBootAtStart::from_boot_params(&d)
    };
    assert_eq!(with(2), SecureBootAtStart::Off);
    assert_eq!(with(3), SecureBootAtStart::On);
    for other in [0, 1, 4, 255] {
        assert_eq!(with(other), SecureBootAtStart::Unknown);
    }
    assert_eq!(
        SecureBootAtStart::from_boot_params(&[2; 0x1ec]),
        SecureBootAtStart::Unknown
    );
    assert_eq!(
        SecureBootAtStart::from_boot_params(&[]),
        SecureBootAtStart::Unknown
    );
}

fn display(app: &mut Faraday, width: u16, height: u16) {
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
}

fn in_view(app: &mut Faraday) -> bool {
    let _ = app.frame();
    let t = app.drawn_texts();
    t.iter().any(|s| s == LINE)
}

#[test]
fn the_sheet_opens_at_its_top_every_time() {
    for (w, h) in [(1280, 800), (900, 300), (480, 640), (480, 200)] {
        let mut app = Faraday::new();
        app.medium = Medium::Stick;
        app.secure_boot = SecureBootAtStart::Off;
        display(&mut app, w, h);
        let _ = app.frame();
        // Keys in memory make the sheet long enough to scroll.
        let mut seeds: Vec<String> = faraday_core::testkit::TEST_SEEDS
            .iter()
            .map(|(w, _)| faraday_core::testkit::test_words(w))
            .collect();
        for i in 1..=20u8 {
            let m =
                osk_bip::bip39::Mnemonic::from_entropy(osk_bip::bip39::Language::English, &[i; 16])
                    .unwrap();
            let list = osk_bip::bip39::Language::English.words();
            let words: Vec<&str> = m.indices().iter().map(|&w| list[w as usize]).collect();
            seeds.push(words.join(" "));
        }
        for words in seeds {
            app.press(Action::Entry(None));
            for c in words.chars() {
                app.event(Event::Key(osk_shell_api::Key::Char(c)));
            }
            app.press(Action::EntryAdd);
        }
        app.press(Action::Nav(faraday_core::Screen::Settings));
        let _ = app.frame();
        // The page beneath is scrolled to its foot first.
        for _ in 0..30 {
            app.event(Event::Key(osk_shell_api::Key::Down));
            app.settle();
            let _ = app.frame();
        }
        app.press(Action::PowerAsk);
        assert!(in_view(&mut app), "first open at {w}x{h}");
        // Scrolled down inside the sheet, then closed and opened again.
        for _ in 0..30 {
            app.event(Event::Key(osk_shell_api::Key::Down));
            app.settle();
            let _ = app.frame();
        }
        app.press(Action::Cancel);
        let _ = app.frame();
        app.press(Action::PowerAsk);
        assert!(in_view(&mut app), "reopened at {w}x{h}");
    }
}
