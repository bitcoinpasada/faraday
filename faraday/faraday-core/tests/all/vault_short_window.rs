//! Create a vault on a short window: Create vault can be reached by
//! scrolling, is drawn whole inside the window, and pressing it starts making the
//! vault; a tall window is the same.

use crate::common::{scroll_to_text, top_up, type_text};
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};

fn window(app: &mut Faraday, (width, height): (u16, u16)) {
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

fn made_from_the_button(size: (u16, u16)) {
    let mut app = testkit::started();
    window(&mut app, size);
    top_up(&mut app);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(&mut app, "test phrase");
    scroll_to_text(&mut app, "Create vault");
    let _ = app.frame();
    let (_, y, _, h) = app
        .hit_box(Action::Vault(V::CGo))
        .expect("Create vault is offered");
    assert!(
        u32::from(y) + u32::from(h) <= u32::from(size.1),
        "it is cut off"
    );
    let (x, y) = app.where_offered(Action::Vault(V::CGo)).unwrap();
    for (phase, now_ms) in [(TouchPhase::Down, 500_000), (TouchPhase::Up, 500_100)] {
        app.event(Event::Tick { now_ms });
        app.event(Event::Touch { x, y, phase });
        let _ = app.frame();
    }
    for t in 501..520 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    // Creating has begun; it takes longer than the test lets it run.
    assert!(
        app.drawn_texts().iter().any(|t| t.starts_with("Creating")),
        "creating did not start"
    );
}

#[test]
fn create_vault_is_pressable_on_a_low_window() {
    made_from_the_button((1366, 768));
}

#[test]
fn create_vault_is_pressable_on_a_tall_window() {
    made_from_the_button((1920, 1080));
}
