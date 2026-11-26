//! Dialogs (`docs/DESIGN.md` §4.13, Dialog): the one Material 3 dialog
//! every question over a page is drawn with. Its actions are pressed by
//! a finger or the pointer on every layout, and the keyboard reaches
//! them: Tab goes to the first, Enter presses it.

use faraday_bip::bip39::Language;
use faraday_core::ui::TextSize;
use faraday_core::{Action, Faraday, Sheet, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware, TouchPhase};

fn on(width: u16, height: u16) -> Faraday {
    let mut app = Faraday::new();
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
    app.press(Action::ReduceMotion(true));
    app
}

/// The power dialog asked for, a frame drawn.
fn power(width: u16, height: u16) -> Faraday {
    let mut app = on(width, height);
    app.press(Action::PowerAsk);
    assert_eq!(app.sheet, Some(Sheet::Power));
    let _ = app.frame();
    app
}

/// A finger down and up where `action` is drawn.
fn tap(app: &mut Faraday, action: Action) {
    let _ = app.frame();
    let (x, y) = app
        .where_offered(action)
        .unwrap_or_else(|| panic!("{action:?} is not on screen"));
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    let _ = app.frame();
}

#[test]
fn a_dialogs_cancel_takes_a_press_on_every_layout() {
    for (w, h) in [(1280, 800), (1920, 1080), (480, 640)] {
        let mut app = power(w, h);
        assert!(
            app.offers(Action::PowerOff),
            "{w}x{h}: Power off is offered"
        );
        tap(&mut app, Action::Cancel);
        assert_eq!(app.sheet, None, "{w}x{h}: Cancel closed it");
    }
}

#[test]
fn the_keyboard_reaches_a_dialogs_first_action_and_enter_presses_it() {
    for (w, h) in [(1280, 800), (480, 640)] {
        let mut app = power(w, h);
        app.event(Event::Key(Key::Tab));
        let _ = app.frame();
        assert_eq!(
            app.sheet,
            Some(Sheet::Power),
            "{w}x{h}: Tab presses nothing"
        );
        app.event(Event::Key(Key::Enter));
        let _ = app.frame();
        assert_eq!(app.sheet, None, "{w}x{h}: Enter on Cancel closed it");
    }
}

#[test]
fn nothing_under_a_dialog_takes_a_press() {
    let mut app = power(1280, 800);
    let _ = app.frame();
    assert!(
        !app.offers(Action::PowerAsk),
        "the page's power button is out of reach"
    );
}

/// The sheet "A stick is attached" with eleven keys loaded, the widest
/// its button's words run: every line of them is inside the button.
#[test]
fn the_stick_sheets_lock_button_holds_all_its_words() {
    use faraday_text::Text;
    for (w, h, dpi) in [
        (240u16, 320u16, 160u16),
        (320, 480, 160),
        (480, 640, 286),
        (1024, 600, 160),
        (1366, 768, 160),
    ] {
        for size in [TextSize::Standard, TextSize::Large] {
            let mut app = on(1366, 768);
            let _ = faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::net());
            app.press(Action::TextSize(size));
            for i in 0..11 {
                let m = faraday_keys::words::from_entropy(Language::English, &[i + 1; 16]).unwrap();
                let words: Vec<&str> = m
                    .indices()
                    .iter()
                    .map(|&w| Language::English.words()[usize::from(w)])
                    .collect();
                app.session
                    .keys
                    .add_words(
                        &faraday_core::testkit::compute(),
                        &words.join(" "),
                        &format!("Seed {i}"),
                        None,
                    )
                    .unwrap();
            }
            assert_eq!(app.session.keys.all().count(), 11);
            app.event(Event::Display(DisplayInfo {
                width: w,
                height: h,
                dpi,
                inset_bottom: 0,
                inset_top: 0,
                buttons: 0,
                camera_fixed: false,
                secure: SecureHardware::None,
                boot: BootState::Unknown,
                memory_mib: None,
            }));
            app.storage(StorageEvent::Sticks(vec![faraday_core::StickInfo {
                make: Default::default(),
                serial: None,
                id: Text::of("stick"),
                label: Text::of("STICK"),
                boot: false,
                files: Vec::new(),
                vaults: Vec::new(),
            }]));
            let _ = app.frame();
            assert_eq!(app.sheet, Some(Sheet::Lock));
            let _ = app.frame();
            let button = app
                .drawn_targets()
                .into_iter()
                .find(|t| t.0 == Action::Lock)
                .expect("the button")
                .2;
            let texts = app.drawn_text_boxes();
            let words = "Lock and use stick · 11 keys leave memory";
            let lines = (0..texts.len())
                .find_map(|from| {
                    (from + 1..=texts.len()).find_map(|to| {
                        let run = &texts[from..to];
                        let joined: Vec<&str> = run.iter().map(|t| t.0.as_str()).collect();
                        (joined.join(" ") == words).then_some(run)
                    })
                })
                .unwrap_or_else(|| panic!("{w}x{h} {size:?}: the words are not drawn whole"));
            for (line, (x, y, lw, lh)) in lines {
                assert!(
                    *x >= button.x
                        && x + lw <= button.right()
                        && *y >= button.y
                        && y + lh <= button.y + button.h,
                    "{w}x{h} {size:?}: {:?} at {:?} runs out of {button:?}",
                    line.as_str(),
                    (x, y, lw, lh)
                );
            }
        }
    }
}
