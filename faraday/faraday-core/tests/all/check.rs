//! Check: a list of what to hold up against a hardware wallet. A row
//! opens its tool and Back returns to Check; a row's tool not built is
//! not on the list.

use faraday_core::check::{Press, Row};
use faraday_core::{Action, Faraday, Screen, Sheet};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

fn shown() -> Faraday {
    shown_at(768)
}

fn shown_at(height: u16) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
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
    let _ = app.frame();
    app
}

fn back(app: &mut Faraday) {
    app.event(Event::Back);
    let _ = app.frame();
}

/// Home offers Check, and a press opens it.
#[test]
fn home_opens_check() {
    let mut app = shown();
    assert!(app.offers(Action::Nav(Screen::Check)));
    app.press(Action::Nav(Screen::Check));
    assert_eq!(app.screen, Screen::Check);
}

/// A row opens its tool, and Back lands on Check.
#[test]
fn a_row_opens_its_tool_and_back_returns_to_check() {
    for (row, screen) in [
        (Row::Signed, Screen::SignerCheck),
        (Row::Child, Screen::Check),
        (Row::Dice, Screen::KeyGen),
    ] {
        let mut app = shown();
        app.press(Action::Nav(Screen::Check));
        app.press(Action::Check(Press::Row(row)));
        let _ = app.frame();
        // Child seeds needs a key first: it opens nothing and stays.
        assert_eq!(app.screen, screen, "{row:?}");
        if screen != Screen::Check {
            // A flow of cards steps back through them first.
            for _ in 0..6 {
                back(&mut app);
                if app.screen != screen {
                    break;
                }
            }
            assert_eq!(app.screen, Screen::Check, "back from {row:?}");
        }
    }
}

/// A backup in another form offers its forms; one opens its tool and
/// Cancel leaves the list as it was.
#[test]
fn a_backup_in_another_form_offers_its_forms() {
    let mut app = shown();
    app.press(Action::Nav(Screen::Check));
    app.press(Action::Check(Press::Row(Row::Backup)));
    assert_eq!(app.sheet, Some(Sheet::CheckBackups));
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::Check);
}

/// The list shows the rows whose tools are built, and none for a tool
/// that is not.
#[test]
fn the_list_draws_built_rows_only() {
    let mut app = shown_at(1400);
    app.press(Action::Nav(Screen::Check));
    let _ = app.frame();
    let texts = app.drawn_texts();
    for want in [
        "An address on its screen",
        "A backup in another form",
        "An xpub's other spellings",
    ] {
        assert!(texts.iter().any(|t| t == want), "{want}: {texts:?}");
    }
    for not in ["A multisig setup", "My seed makes my wallet"] {
        assert!(!texts.iter().any(|t| t == not), "{not} is not built");
    }
}
