//! The pictures of `docs/guide/walk/first-start.md`.

use crate::tour::{Tour, display};
use faraday_core::devices::DeviceCheck;
use faraday_core::{Action, Faraday, Medium, StickInfo, StorageEvent};
use faraday_shell_api::{App, Event, Key};

/// A fresh stick session as a person meets it: the stick it started from
/// still in, Secure Boot off at start, a laptop named for no maker, an
/// IP-KVM attached; then Home and the power sheet.
pub(super) fn run(t: &mut Tour) -> Result<(), String> {
    let size = t.size();
    t.app = Faraday::new();
    t.app.medium = Medium::Stick;
    t.app.devices = DeviceCheck::machine("flags\t: fpu vme", "Example", "Example 14", "10");
    t.app.secure_boot = faraday_core::SecureBootAtStart::Off;
    t.app.event(display(size, t.dpi));
    t.app.storage(StorageEvent::Sticks(vec![StickInfo {
        id: "boot".into(),
        label: "FARADAY".into(),
        boot: true,
        serial: Some(0x1234_5678),
        files: Vec::new(),
    }]));
    t.pump();
    // An IP-KVM attached at start: named before anything else, then
    // unplugged and checked again.
    t.app.usb_devices(&[crate::tours::devices::kvm()]);
    t.pump();
    t.shot_marking("2-1-attached", Action::DevicesCheck)?;
    t.shot("prompts-attached")?;
    t.app.usb_devices(&[]);
    t.press(Action::DevicesCheck);
    for _ in 0..3 {
        t.tick();
    }
    t.shot_marking("2-1-home", Action::PowerAsk)?;
    t.press(Action::PowerAsk);
    t.shot("2-3-power")?;
    t.app.event(Event::Key(Key::Escape));
    Ok(())
}
