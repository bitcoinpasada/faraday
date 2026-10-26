//! A Practice passphrase field draws its caret as any field does: **Use
//! faraday** stands in the host's strip, not inside the field or
//! anywhere in the app (`docs/DEMO.md` §5.4, `docs/DESKTOP-HOST.md` §3).

use crate::common::demo_app;
use faraday_core::demo::PassField;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StorageEvent};
use faraday_shell_api::{App, Event};

fn pixels_at(app: &mut Faraday, now_ms: u64) -> Vec<u8> {
    app.event(Event::Tick { now_ms });
    app.frame().rgba.to_vec()
}

/// The sample vault's unlock, its field in focus and empty.
fn unlock() -> Faraday {
    let mut app = demo_app();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::UseSample);
    app.press(Action::Vault(V::Open(0)));
    let _ = pixels_at(&mut app, 1_000);
    app
}

/// How many pixels differ between the caret's on and off frames of
/// `app`: what blinks on the screen, the caret and any clock beside it.
fn blinking(app: &mut Faraday) -> usize {
    let on = pixels_at(app, 1_000);
    let off = pixels_at(app, 1_540);
    on.iter().zip(&off).filter(|(a, b)| a != b).count()
}

#[test]
fn the_caret_blinks_in_an_empty_demo_passphrase_field_and_use_faraday_is_in_the_strip() {
    let mut app = unlock();
    let chip = Action::PassChip(PassField::Unlock);
    assert!(
        crate::common::strip_offers(&mut app, chip),
        "Use faraday is in the strip"
    );
    let _ = app.frame();
    assert!(!app.offers(chip), "the app draws no Use faraday");
    assert!(
        !app.drawn_texts().iter().any(|t| t == "faraday"),
        "nothing is drawn inside the field"
    );
    let before = blinking(&mut app);
    assert!(before > 0, "the caret blinks with nothing pressed");
    let mut pressed = unlock();
    pressed.press(Action::Vault(V::FocusPassphrase));
    assert_eq!(
        before,
        blinking(&mut pressed),
        "pressing in the field changes nothing the caret draws"
    );
}
