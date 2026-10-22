//! A Demo passphrase field that offers the `faraday` chip draws no caret
//! until the person presses in it or types: the caret would stand over
//! the chip's edge.

use crate::common::demo_app;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StorageEvent};
use faraday_shell_api::{App, Event, Key};

fn pixels_at(app: &mut Faraday, now_ms: u64) -> Vec<u8> {
    app.event(Event::Tick { now_ms });
    app.frame().rgba.to_vec()
}

/// The sample vault's unlock, its field in focus with the chip showing.
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
fn the_caret_is_not_drawn_beside_the_chip_until_the_person_is_in_the_field() {
    let mut chip = unlock();
    assert!(
        chip.drawn_texts().iter().any(|t| t == "faraday"),
        "the chip"
    );
    let with_chip = blinking(&mut chip);
    let mut pressed = unlock();
    pressed.press(Action::Vault(V::FocusPassphrase));
    let in_field = blinking(&mut pressed);
    assert!(
        with_chip < in_field,
        "the caret blinks beside the chip: {with_chip} against {in_field}"
    );
    let mut typed = unlock();
    typed.event(Event::Key(Key::Char('x')));
    assert!(with_chip < blinking(&mut typed), "typing brings the caret");
}
