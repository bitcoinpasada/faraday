//! The devices tours.

use crate::tour::{Tour, laptop, type_text};
use faraday_core::testkit;
use faraday_core::{Action, Screen};
use faraday_shell_api::{App, Event, Key};

/// A wireless receiver: Logitech's Unifying receiver, or with `bolt` its
/// Bolt receiver.
pub(crate) fn receiver(bolt: bool) -> faraday_core::devices::UsbDevice {
    use faraday_core::devices::{Interface, UsbDevice};
    let hid = |protocol| Interface {
        class: 3,
        subclass: 1,
        protocol,
    };
    UsbDevice {
        port: "1-2".into(),
        vendor: 0x046d,
        product: if bolt { 0xc548 } else { 0xc52b },
        manufacturer: "Logitech".into(),
        name: "USB Receiver".into(),
        interfaces: vec![hid(1), hid(2)],
    }
}

/// An IP-KVM, PiKVM's composite device: a keyboard, a mouse and a disk.
pub(crate) fn kvm() -> faraday_core::devices::UsbDevice {
    use faraday_core::devices::{Interface, UsbDevice};
    let hid = |protocol| Interface {
        class: 3,
        subclass: 1,
        protocol,
    };
    UsbDevice {
        port: "1-1".into(),
        vendor: 0x1d6b,
        product: 0x0104,
        manufacturer: "PiKVM".into(),
        name: "Composite KVM Device".into(),
        interfaces: vec![
            hid(1),
            hid(2),
            Interface {
                class: 8,
                subclass: 6,
                protocol: 0x50,
            },
        ],
    }
}

/// The stick's device check (`docs/STARTER.md` §3.1a, F24): before the
/// first secret, the reminders with a wireless receiver attached, and
/// with a Bolt receiver; an IP-KVM named at start, and Continue anyway
/// asking; a virtual machine with an IP-KVM, which has no way on; a
/// desktop's reminder alone, and the same desktop whose firmware passed
/// a TV's EDID. Home empty ends it.
pub(crate) fn devices_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::devices::DeviceCheck;
    let kvm = kvm();
    t.go_on = false;
    t.app.devices = laptop();
    t.app.usb_devices(&[receiver(false)]);
    t.press(Action::Entry(None));
    t.shot("devices-receiver")?;
    t.press(Action::Cancel);
    t.app.usb_devices(&[receiver(true)]);
    t.press(Action::Entry(None));
    t.shot("devices-receiver-bolt")?;
    t.press(Action::Cancel);
    t.app.usb_devices(std::slice::from_ref(&kvm));
    t.pump();
    t.shot("devices-kvm")?;
    t.press(Action::DevicesContinue);
    t.shot("devices-continue-ask")?;
    t.app.usb_devices(&[]);
    t.press(Action::DevicesCheck);
    t.app.devices = DeviceCheck::machine(
        "flags\t: fpu vme hypervisor",
        "QEMU",
        "Standard PC (Q35 + ICH9, 2009)",
        "1",
    );
    t.app.usb_devices(&[kvm]);
    t.pump();
    t.shot("devices-virtual-machine")?;
    t.press(Action::Cancel);
    t.app.devices = DeviceCheck::machine("flags\t: fpu vme", "Dell Inc.", "OptiPlex 7090", "3");
    t.app.usb_devices(&[]);
    t.press(Action::Entry(None));
    t.shot("devices-before")?;
    t.press(Action::Cancel);
    // The same desktop whose firmware passed a TV's EDID: named, not asked.
    let mut edid = [0u8; 128];
    edid[..8].copy_from_slice(&[0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0]);
    edid[8..10].copy_from_slice(&[0x4c, 0x2d]);
    edid[54..59].copy_from_slice(&[0, 0, 0, 0xfc, 0]);
    edid[59..72].copy_from_slice(b"EXAMPLE TV\n  ");
    t.app.devices = DeviceCheck::machine("flags\t: fpu vme", "Dell Inc.", "OptiPlex 7090", "3")
        .with_edid(Some(&edid));
    t.app.usb_devices(&[]);
    t.pump();
    t.shot("devices-tv-edid")?;
    t.press(Action::Cancel);
    // A USB touchscreen plugged in, one tap beside its target (F153).
    let id = 9;
    t.app.storage(faraday_core::StorageEvent::NewInput {
        id,
        name: "USB Touchscreen".into(),
        keyboard: false,
        pointer: false,
        touch: true,
    });
    t.pump();
    t.app
        .storage(faraday_core::StorageEvent::InputTapped { id, x: 4, y: 4 });
    t.pump();
    t.shot("devices-new-touchscreen")?;
    t.press(Action::InputIgnore(id));
    t.app.devices = laptop();
    t.app.usb_devices(&[]);
    t.pump();
    t.go_on = true;
    t.shot("home-empty")
}

/// Toasts on Home (Material's snackbar): a wireless receiver let in as
/// a pointer, its one-line toast; a keyboard that typed the wrong code,
/// a toast long enough to wrap at the narrow sizes.
pub(crate) fn toasts_tour(t: &mut Tour) -> Result<(), String> {
    t.app.devices = laptop();
    t.app.usb_devices(&[]);
    t.press(Action::Nav(Screen::Home));
    let held = |id, name: &str, keyboard| faraday_core::StorageEvent::NewInput {
        id,
        name: name.into(),
        keyboard,
        pointer: !keyboard,
        touch: false,
    };
    t.app.storage(held(11, "USB Receiver", false));
    t.pump();
    t.press(Action::InputUse(11));
    t.shot_after("toast-in-use", 1000)?;
    t.app.storage(held(12, "USB Keyboard", true));
    t.pump();
    let letters = faraday_core::inputs::CODE_LEN * faraday_core::inputs::CODE_TRIES;
    for _ in 0..letters {
        t.app
            .storage(faraday_core::StorageEvent::InputTyped { id: 12, ch: '#' });
    }
    t.shot_after("toast-wrong-code", 1000)
}

/// The wide layout's on-screen keyboard: a receiver's line on Home, the
/// keyboard brought up from its button on a passphrase entry, and on
/// Tools.
/// The power sheet on a PC stick that started with Secure Boot on, and
/// with it off (the reminder and this computer's BIOS key).
pub(crate) fn power_tour(t: &mut Tour) -> Result<(), String> {
    t.app.secure_boot = faraday_core::SecureBootAtStart::On;
    t.press(Action::PowerAsk);
    t.shot("power-sheet")?;
    t.app.event(Event::Key(Key::Escape));
    t.app.secure_boot = faraday_core::SecureBootAtStart::Off;
    t.app.maker = "LENOVO".into();
    t.app.maker_family = "ThinkPad X1 Carbon".into();
    t.app.maker_model = "Example 14".into();
    t.press(Action::PowerAsk);
    t.shot("power-sheet-secure-boot-off")
}

pub(crate) fn osk_tour(t: &mut Tour) -> Result<(), String> {
    t.app.devices = laptop();
    t.app.usb_devices(&[receiver(false)]);
    t.pump();
    t.shot("osk-home-receiver")?;
    t.press(Action::Entry(None));
    type_text(t, &testkit::test_words(testkit::TEST_SEEDS[2].0));
    t.press(Action::EntryPassphrase);
    type_text(t, "TREZOR");
    t.shot("osk-passphrase-button")?;
    t.press(Action::Osk(faraday_core::OskPress::Show));
    for c in ['a', 'b'] {
        t.press(Action::Osk(faraday_core::OskPress::Char(c)));
    }
    t.shot("osk-passphrase-keyboard")?;
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Osk(faraday_core::OskPress::Show));
    t.shot("osk-tools-keyboard")
}

/// Home's Scan with a stick in: the sheet asks for it to be pulled, and
/// the camera opens when it is.
pub(crate) fn scan_tour(t: &mut Tour) -> Result<(), String> {
    t.sticks(true);
    t.press(Action::Nav(Screen::Home));
    t.press(Action::Scan);
    t.shot("home-pull-scan")?;
    t.sticks(false);
    if t.app.sheet != Some(faraday_core::Sheet::Scan) {
        return Err("pulling the stick did not open the camera".into());
    }
    t.shot("scan-after-pull")?;
    // A code in the camera's picture, found and then read: outlined
    // where it is (`docs/QR.md` §8 item 4).
    use faraday_codec::qr::{self, Ecc, Payload, QUIET_ZONE};
    let m = qr::encode(
        &testkit::compute(),
        Payload::Bytes(b"faraday viewfinder"),
        Ecc::Low,
    )
    .map_err(|e| format!("{e:?}"))?;
    let scale = 6;
    let (cw, ch, code) = m.to_luma(scale, QUIET_ZONE);
    let (w, h) = (640usize, 480usize);
    let mut luma: Vec<u8> = (0..w * h)
        .map(|i| (90 + (i % w) / 8 + (i / w) / 6) as u8)
        .collect();
    let (x0, y0) = (w / 2 - cw / 2 + 40, h / 2 - ch / 2);
    for y in 0..ch {
        luma[(y0 + y) * w + x0..(y0 + y) * w + x0 + cw]
            .copy_from_slice(&code[y * cw..(y + 1) * cw]);
    }
    t.app.event(Event::CameraFrame {
        width: w as u16,
        height: h as u16,
        luma,
        chroma: None,
    });
    let q = QUIET_ZONE * scale;
    let (l, tp) = ((x0 + q) as u16, (y0 + q) as u16);
    let (r, b) = ((x0 + cw - q) as u16, (y0 + ch - q) as u16);
    for (read, name) in [(false, "scan-found"), (true, "scan-read")] {
        t.app.storage(faraday_core::StorageEvent::QrSeen {
            width: w as u16,
            height: h as u16,
            corners: [(l, tp), (r, tp), (r, b), (l, b)],
            module_tenths: 60,
            read,
        });
        t.shot(name)?;
    }
    Ok(())
}
