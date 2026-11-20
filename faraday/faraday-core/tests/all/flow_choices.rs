//! Your choices on Make a new vault: the bar's button on a one-column
//! window and the sheet it opens, the pane on the wide one, and the guide
//! kept out of the pane's place on an extra-large window.

use faraday_core::testkit::Plain;
use faraday_core::vault::{VaultAction as V, vstep};
use faraday_core::{Action, ChoicesPress, Faraday, Screen, Sheet};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

/// Make a new vault on a window `width` × `height` px at 160 dpi.
fn on_create_vault(width: u16, height: u16) -> Faraday {
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
    app.press(Action::Nav(Screen::Vaults));
    if app.screen != Screen::CreateVault {
        app.press(Action::Vault(V::Create));
    }
    assert_eq!(app.screen, Screen::CreateVault);
    let _ = app.frame();
    app
}

fn open_step(app: &Faraday) -> Option<u8> {
    app.vaults().create.as_ref().and_then(|c| c.open)
}

/// Where "Your choices" is drawn, top edges in pixels.
fn heads(app: &mut Faraday) -> Vec<i32> {
    let _ = app.frame();
    app.drawn_text_boxes()
        .plain()
        .into_iter()
        .filter(|(t, _)| t == "Your choices")
        .map(|(_, b)| b.1)
        .collect()
}

const OPEN: Action = Action::Choices(ChoicesPress::Open);

#[test]
fn a_half_screen_window_has_the_button_in_the_bar_and_no_pane() {
    let mut app = on_create_vault(683, 768);
    assert!(
        app.where_offered(OPEN).is_some(),
        "the bar offers the button"
    );
    let at = heads(&mut app);
    assert_eq!(at.len(), 1, "only the button's label, no pane: {at:?}");
    assert!(at[0] < 64, "the label stands in the bar");
}

#[test]
fn a_wide_window_has_the_pane_and_no_button() {
    let mut app = on_create_vault(1366, 768);
    assert!(app.where_offered(OPEN).is_none(), "no button");
    assert_eq!(heads(&mut app).len(), 1, "the pane's heading");
}

#[test]
fn escape_and_the_way_back_close_the_sheet_and_keep_the_step() {
    let mut app = on_create_vault(683, 768);
    app.press(Action::Vault(V::CCustomise));
    app.press(Action::Vault(V::CStep(vstep::COST)));
    let step = open_step(&app);
    for escape in [true, false] {
        app.press(OPEN);
        assert_eq!(app.sheet, Some(Sheet::Choices));
        if escape {
            app.event(Event::Key(Key::Escape));
        } else {
            app.press(Action::Back);
        }
        assert_eq!(app.sheet, None, "the sheet closes");
        assert_eq!(app.screen, Screen::CreateVault, "the flow stays");
        assert_eq!(open_step(&app), step, "the step stays as it was");
    }
}

#[test]
fn a_row_of_the_sheet_closes_it_and_opens_its_step() {
    let mut app = on_create_vault(683, 768);
    app.press(Action::Vault(V::CStep(vstep::PHRASES)));
    app.press(OPEN);
    app.press(Action::Choices(ChoicesPress::Row(vstep::PRESET)));
    assert_eq!(app.sheet, None);
    assert_eq!(open_step(&app), Some(vstep::PRESET));
}

#[test]
fn on_an_extra_large_window_the_guide_is_a_sheet_beside_the_pane() {
    let mut app = on_create_vault(2560, 1440);
    app.press(Action::Guide);
    assert_eq!(app.sheet, Some(Sheet::Guide));
    // Put away, the page under it still has the pane, not the guide.
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    assert!(!heads(&mut app).is_empty(), "Your choices stays drawn");
}
