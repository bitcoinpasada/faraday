//! Transfer in Faraday Demo: what Receive says once the shell has saved
//! a file.

use faraday_core::ui::{Role, Theme};
use faraday_core::wallet::Session;
use faraday_core::{Action, Faraday, Screen, StorageEvent, testkit};
use faraday_draw::canvas::InkKind;
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

/// Faraday Demo at 1280 × 800 on Transfer, drawn in `theme`.
fn on_transfer(theme: Theme) -> Faraday {
    let mut app = Faraday::new();
    app.online = true;
    app.session = Session::on(testkit::NET);
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
    app.draw_in(theme);
    app.press(Action::Nav(Screen::Transfer));
    app.settle();
    app
}

/// The colour `text` is drawn in, if it is drawn.
fn color_of(app: &mut Faraday, text: &str) -> Option<faraday_draw::Color> {
    app.drawn_ink().iter().find_map(|i| match &i.kind {
        InkKind::Text { text: t, color, .. } if t == text => Some(*color),
        _ => None,
    })
}

#[test]
fn a_file_saved_is_said_in_the_ok_tone() {
    let theme = Theme::all().next().expect("a theme");
    let mut app = on_transfer(theme);
    assert_eq!(app.screen, Screen::Transfer);
    app.storage(StorageEvent::Saved {
        path: "~/Downloads/payment-signed.psbt".into(),
    });
    let line = "Saved ~/Downloads/payment-signed.psbt";
    assert_eq!(color_of(&mut app, line), Some(theme.role(Role::Ok)));
}
