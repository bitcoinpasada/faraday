//! The on-screen keyboard over a field on a small panel: with the
//! keyboard up, the field typing goes to stands above the keyboard's top
//! edge with its label, the page scrolled as far as that takes; focus
//! moved to the next field, that one does too; and on a panel where the
//! field is clear of the keyboard, nothing scrolls.

use faraday_core::set_time::Focus as T;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, OskPress, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

/// The Pi's panel, and the smallest panel the design supports.
const PANELS: [(u16, u16, u16); 2] = [(480, 640, 286), (240, 320, 143)];

/// The app on a panel `w` × `h` pixels at `dpi`, drawn once.
fn panel((w, h, dpi): (u16, u16, u16)) -> Faraday {
    let mut app = Faraday::new();
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
    let _ = app.frame();
    app
}

/// `action` pressed and the frame after it drawn.
fn press(app: &mut Faraday, action: Action) {
    app.press(action);
    let _ = app.frame();
    let _ = app.frame();
}

/// The field typing goes to, with its label, is wholly on the panel and
/// above the keyboard's top edge.
fn above_keyboard(app: &mut Faraday, what: &str) {
    let _ = app.frame();
    let top = app
        .keyboard_top()
        .unwrap_or_else(|| panic!("{what}: the keyboard is up"));
    let (_, y, _, h) = app
        .focused_field()
        .unwrap_or_else(|| panic!("{what}: a field has focus"));
    assert!(y >= 0, "{what}: the field's top ({y}) is on the panel");
    assert!(
        y + h <= i32::from(top),
        "{what}: the field ({y} to {}) is above the keyboard's top ({top})",
        y + h
    );
}

/// Create a vault opened on its name and passphrases.
fn create_vault(app: &mut Faraday) {
    press(app, Action::Nav(Screen::Vaults));
    press(app, Action::Vault(V::Create));
    assert_eq!(app.screen, Screen::CreateVault);
}

#[test]
fn a_page_opens_at_its_top_with_no_field_focused_and_a_tap_brings_the_keyboard() {
    for size in PANELS {
        let mut app = panel(size);
        create_vault(&mut app);
        let _ = app.frame();
        assert!(
            app.keyboard_top().is_none(),
            "{size:?}: the keyboard is down"
        );
        assert!(
            app.focused_field().is_none(),
            "{size:?}: nothing is focused"
        );
        assert!(
            app.drawn_texts()
                .iter()
                .any(|t| t.contains("Name and") || t.contains("passphrases")),
            "{size:?}: the step's heading is in view"
        );
        press(&mut app, Action::Vault(V::CFocus(0, false)));
        above_keyboard(&mut app, &format!("{size:?} the passphrase, tapped"));
    }
}

#[test]
fn a_field_s_supporting_text_stands_above_the_keyboard_with_it() {
    for size in PANELS {
        let mut app = panel(size);
        create_vault(&mut app);
        press(&mut app, Action::Vault(V::CFocus(0, true)));
        above_keyboard(&mut app, &format!("{size:?} the passphrase again"));
        let top = i32::from(app.keyboard_top().expect("the keyboard is up"));
        let (_, (_, y, _, h)) = app
            .drawn_text_boxes()
            .into_iter()
            .find(|(t, _)| t == "Type it, then type it again")
            .unwrap_or_else(|| panic!("{size:?}: the field's supporting text is drawn"));
        assert!(
            y >= 0 && y + h <= top,
            "{size:?}: the supporting text ({y} to {}) is above the keyboard ({top})",
            y + h
        );
    }
}

#[test]
fn create_a_vault_s_fields_stand_above_the_keyboard_as_focus_moves() {
    for size in PANELS {
        let mut app = panel(size);
        create_vault(&mut app);
        for (field, what) in [
            (V::CName, "the name"),
            (V::CFocus(0, false), "the passphrase"),
            (V::CFocus(0, true), "the passphrase again"),
        ] {
            press(&mut app, Action::Vault(field));
            above_keyboard(&mut app, &format!("{size:?} Create a vault, {what}"));
        }
        // Back up to the name, above where the page has scrolled to.
        press(&mut app, Action::Vault(V::CName));
        above_keyboard(
            &mut app,
            &format!("{size:?} Create a vault, the name again"),
        );
    }
}

#[test]
fn the_keyboard_brought_back_brings_its_field_above_it() {
    for size in PANELS {
        let mut app = panel(size);
        create_vault(&mut app);
        press(&mut app, Action::Vault(V::CFocus(0, true)));
        press(&mut app, Action::Osk(OskPress::Hide));
        assert!(app.keyboard_top().is_none(), "{size:?}: put away");
        press(&mut app, Action::Vault(V::CFocus(0, false)));
        above_keyboard(&mut app, &format!("{size:?} the passphrase, keyboard back"));
    }
}

#[test]
fn set_the_time_s_date_and_time_stand_above_the_keyboard() {
    for size in PANELS {
        let mut app = panel(size);
        press(&mut app, Action::OpenSetTime);
        assert_eq!(app.screen, Screen::SetTime);
        for (focus, what) in [(T::Date, "the date"), (T::Time, "the time")] {
            press(&mut app, Action::SetTimeFocus(focus));
            above_keyboard(&mut app, &format!("{size:?} Set the time, {what}"));
        }
    }
}

#[test]
fn a_field_clear_of_the_keyboard_scrolls_nothing() {
    // A phone's panel, 5 inches at 9:16: Create a vault's name stands
    // well above where the keyboard comes up.
    let mut app = panel((678, 1206, 277));
    create_vault(&mut app);
    press(&mut app, Action::Vault(V::CFocus(0, true)));
    press(&mut app, Action::Osk(OskPress::Hide));
    let field = Action::Vault(V::CName);
    let before = app.hit_box(field).expect("the name");
    press(&mut app, field);
    assert!(app.keyboard_top().is_some(), "the keyboard is up");
    above_keyboard(&mut app, "a phone, the name");
    let after = app.hit_box(field).expect("the name");
    assert_eq!(before.1, after.1, "the page did not move");
}

#[test]
fn the_keyboard_put_away_leaves_the_page_where_it_was() {
    let mut app = panel(PANELS[0]);
    create_vault(&mut app);
    press(&mut app, Action::Vault(V::CFocus(0, true)));
    above_keyboard(&mut app, "the passphrase again");
    let field = Action::Vault(V::CFocus(0, true));
    let up = app.hit_box(field).expect("the field is on the panel").1;
    press(&mut app, Action::Osk(OskPress::Hide));
    let down = app
        .hit_box(field)
        .expect("the field is still on the panel")
        .1;
    assert!(
        down >= up,
        "the field did not jump up as the keyboard went down"
    );
}

/// Answers the app's reads from the test vault, the one file on the
/// boot stick.
fn pump(app: &mut Faraday, vault: &[u8]) {
    use faraday_core::{StorageCommand, StorageEvent};
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Read { stick, name } = c {
            app.storage(StorageEvent::Read {
                stick,
                name,
                bytes: vault.to_vec(),
            });
        }
    }
}

#[test]
fn unlock_s_passphrase_stands_above_the_keyboard() {
    use faraday_core::boot_import::ImportAction as I;
    use faraday_core::{StickInfo, StorageEvent, testkit};
    for size in PANELS {
        let vault = testkit::test_vault().unwrap();
        let mut app = panel(size);
        testkit::move_to(&mut app.session, testkit::NET).unwrap();
        app.storage(StorageEvent::Memory {
            available_mib: 15_000,
        });
        testkit::vault_speed(&mut app, Some(100));
        app.storage(StorageEvent::Sticks(vec![StickInfo {
            serial: None,
            id: "boot".into(),
            label: "FARADAY".into(),
            boot: true,
            files: vec![("vault.fvlt".into(), vault.len() as u64)],
            vaults: Vec::new(),
        }]));
        pump(&mut app, &vault);
        app.press(Action::Import(I::Go));
        pump(&mut app, &vault);
        app.press(Action::Import(I::Later));
        app.storage(StorageEvent::Sticks(Vec::new()));
        press(&mut app, Action::Vault(V::Open(0)));
        assert_eq!(app.screen, Screen::Unlock, "{size:?}");
        press(&mut app, Action::Vault(V::FocusPassphrase));
        above_keyboard(&mut app, &format!("{size:?} Unlock, the passphrase"));
    }
}

#[test]
fn add_a_key_s_passphrase_stands_above_the_keyboard() {
    for size in PANELS {
        let mut app = panel(size);
        // The words are typed on the page's own word keys; the
        // passphrase on the docked keyboard.
        press(&mut app, Action::Entry(None));
        assert_eq!(app.screen, Screen::Entry, "{size:?}");
        press(&mut app, Action::EntryPassphrase);
        above_keyboard(&mut app, &format!("{size:?} Add a key, the passphrase"));
    }
}
