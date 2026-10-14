//! The stick's device check (`docs/STARTER.md` §3.1a): what can capture the
//! screen or the keys is named at start, before anything is unlocked;
//! while it is attached and not let through, nothing that shows a secret
//! is offered. Check again after it is pulled releases; Continue anyway
//! asks once; a virtual machine has no Continue anyway; a desktop's
//! screen is asked about once; Home always carries the TV line. A
//! wireless receiver holds nothing: it is a line on Home, and a Bolt
//! receiver's line says its typing is encrypted.

use faraday_core::devices::{
    DeviceCheck, Interface, Kind, UsbDevice, screen_external, usb_kind, usb_name, virtual_machine,
};
use faraday_core::{Action, Faraday, Screen, Sheet};
use osk_shell_api::{App, DisplayInfo, Event};

const CPU: &str = "processor\t: 0\nflags\t\t: fpu vme de pse tsc msr\n";
const CPU_VM: &str = "processor\t: 0\nflags\t\t: fpu vme de pse tsc msr hypervisor\n";

fn hid(protocol: u8) -> Interface {
    Interface {
        class: 3,
        subclass: 1,
        protocol,
    }
}

fn storage() -> Interface {
    Interface {
        class: 8,
        subclass: 6,
        protocol: 0x50,
    }
}

fn device(vendor: u16, product: u16, m: &str, p: &str, interfaces: Vec<Interface>) -> UsbDevice {
    UsbDevice {
        port: "1-2".into(),
        vendor,
        product,
        manufacturer: m.into(),
        name: p.into(),
        interfaces,
    }
}

fn receiver() -> UsbDevice {
    device(
        0x046d,
        0xc52b,
        "Logitech",
        "USB Receiver",
        vec![hid(1), hid(2)],
    )
}

fn kvm() -> UsbDevice {
    device(
        0x1d6b,
        0x0104,
        "PiKVM",
        "Composite KVM Device",
        vec![hid(1), hid(2), storage()],
    )
}

// ----- The rules -----

#[test]
fn a_linux_gadget_with_a_keyboard_and_a_disk_is_a_kvm() {
    let pikvm = device(0x1d6b, 0x0104, "", "", vec![hid(1), hid(2), storage()]);
    assert_eq!(usb_kind(&pikvm), Some(Kind::Kvm));
    // A root hub has the same vendor and is nothing.
    let hub = device(
        0x1d6b,
        0x0002,
        "Linux 6.12 xhci-hcd",
        "xHCI Host Controller",
        vec![Interface {
            class: 9,
            subclass: 0,
            protocol: 0,
        }],
    );
    assert_eq!(usb_kind(&hub), None);
    // A KVM switch that says so, under any vendor.
    let switch = device(0x0557, 0x2419, "ATEN", "CS1924 KVM Switch", vec![hid(1)]);
    assert_eq!(usb_kind(&switch), Some(Kind::Kvm));
}

#[test]
fn receivers_are_known_by_id_or_by_what_they_call_themselves() {
    // Logitech's Bolt receiver, by its ID alone, told apart.
    let bolt = device(0x046d, 0xc548, "", "", vec![hid(1)]);
    assert_eq!(usb_kind(&bolt), Some(Kind::BoltReceiver));
    // Its Unifying receiver, by its ID alone.
    assert_eq!(usb_kind(&receiver()), Some(Kind::Receiver));
    // An unknown maker's receiver, by its strings.
    let dongle = device(0x1234, 0x5678, "Acme", "2.4G Wireless Dongle", vec![hid(1)]);
    assert_eq!(usb_kind(&dongle), Some(Kind::Receiver));
    // A wired keyboard and a stick are nothing.
    let wired = device(
        0x413c,
        0x2113,
        "Dell",
        "Dell KB216 Wired Keyboard",
        vec![hid(1)],
    );
    assert_eq!(usb_kind(&wired), None);
    let stick = device(0x0781, 0x5581, "SanDisk", "Ultra", vec![storage()]);
    assert_eq!(usb_kind(&stick), None);
    // "Wireless" on something with no HID interface does not type.
    let radio = device(
        0x0bda,
        0x8179,
        "Realtek",
        "802.11n Wireless NIC",
        vec![Interface {
            class: 0xff,
            subclass: 0xff,
            protocol: 0xff,
        }],
    );
    assert_eq!(usb_kind(&radio), None);
}

#[test]
fn a_device_is_named_by_its_maker_and_product_or_its_ids() {
    assert_eq!(usb_name(&receiver()), "Logitech USB Receiver");
    let inc = device(0x046d, 0xc548, "Logitech, Inc.", "USB Receiver", vec![]);
    assert_eq!(usb_name(&inc), "Logitech USB Receiver");
    let doubled = device(0x046d, 0xc548, "Logitech", "Logitech Bolt Receiver", vec![]);
    assert_eq!(usb_name(&doubled), "Logitech Bolt Receiver");
    let bare = device(0x046d, 0xc548, "", "", vec![]);
    assert_eq!(usb_name(&bare), "USB device 046d:c548");
}

#[test]
fn a_virtual_machine_is_known_by_its_firmware_or_the_hypervisor_flag() {
    let vm = |v: &str, p: &str| virtual_machine(CPU, v, p);
    assert_eq!(
        vm("QEMU", "Standard PC (Q35 + ICH9, 2009)").as_deref(),
        Some("QEMU")
    );
    assert_eq!(
        vm("VMware, Inc.", "VMware Virtual Platform").as_deref(),
        Some("VMware")
    );
    assert_eq!(
        vm("innotek GmbH", "VirtualBox").as_deref(),
        Some("VirtualBox")
    );
    assert_eq!(
        vm("Microsoft Corporation", "Virtual Machine").as_deref(),
        Some("Hyper-V")
    );
    assert_eq!(vm("Xen", "HVM domU").as_deref(), Some("Xen"));
    assert_eq!(
        vm(
            "Parallels Software International Inc.",
            "Parallels Virtual Platform"
        )
        .as_deref(),
        Some("Parallels")
    );
    // A Surface is Microsoft's and real.
    assert_eq!(vm("Microsoft Corporation", "Surface Pro 9"), None);
    assert_eq!(vm("LENOVO", "20XW"), None);
    // Any hypervisor, its maker unknown.
    assert_eq!(
        virtual_machine(CPU_VM, "Acme", "Cloud").as_deref(),
        Some("This computer")
    );
}

#[test]
fn a_desktop_has_a_separate_screen_and_a_laptop_does_not() {
    for desktop in ["3", "4", "6", "7", "35", "36\n"] {
        assert!(screen_external(desktop), "{desktop}");
    }
    // Laptop, notebook, all-in-one, tablet, convertible, unknown.
    for own in ["9", "10", "13", "30", "31", "2", "", "x"] {
        assert!(!screen_external(own), "{own}");
    }
}

// ----- The stick -----

fn shown(machine: DeviceCheck) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: osk_shell_api::SecureHardware::None,
        boot: osk_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
    app.devices = machine;
    let _ = app.frame();
    app
}

fn laptop() -> DeviceCheck {
    DeviceCheck::machine(CPU, "LENOVO", "20XW", "10")
}

fn says(app: &mut Faraday, text: &str) -> bool {
    app.drawn_texts().iter().any(|t| t.contains(text))
}

#[test]
fn a_kvm_attached_at_start_is_named_on_a_sheet() {
    let mut app = shown(laptop());
    app.usb_devices(&[kvm()]);
    assert_eq!(app.sheet, Some(Sheet::Devices));
    assert!(says(
        &mut app,
        "PiKVM Composite KVM Device: a KVM can show this screen"
    ));
    assert!(app.where_offered(Action::DevicesCheck).is_some());
    assert!(app.where_offered(Action::DevicesContinue).is_some());
}

#[test]
fn a_receiver_is_a_line_on_home_and_holds_nothing() {
    let mut app = shown(laptop());
    app.usb_devices(&[receiver()]);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::Home);
    assert!(says(&mut app, "Wireless receiver attached."));
    assert!(!says(&mut app, "Bolt"));
    // Typing a key goes on.
    app.press(Action::Entry(None));
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::Entry);
    // Pulled, the line goes.
    app.usb_devices(&[]);
    app.press(Action::Nav(Screen::Home));
    assert!(!says(&mut app, "Wireless receiver attached."));
}

#[test]
fn a_bolt_receiver_says_its_typing_is_encrypted() {
    let mut app = shown(laptop());
    let bolt = device(
        0x046d,
        0xc548,
        "Logitech",
        "USB Receiver",
        vec![hid(1), hid(2)],
    );
    app.usb_devices(std::slice::from_ref(&bolt));
    assert_eq!(app.sheet, None);
    assert!(says(&mut app, "Logitech Bolt receiver attached."));
    assert!(!says(&mut app, "Wireless receiver attached."));
    // An unknown receiver beside it: the warning.
    let dongle = device(0x1234, 0x5678, "Acme", "2.4G Wireless Dongle", vec![hid(1)]);
    app.usb_devices(&[bolt, dongle]);
    assert!(says(&mut app, "Wireless receiver attached."));
}

#[test]
fn nothing_attached_brings_no_sheet() {
    let mut app = shown(laptop());
    app.usb_devices(&[device(0x0781, 0x5581, "SanDisk", "Ultra", vec![storage()])]);
    assert_eq!(app.sheet, None);
    app.press(Action::Entry(None));
    assert_eq!(app.screen, Screen::Entry);
}

#[test]
fn nothing_that_shows_a_secret_is_offered_while_held() {
    let mut app = shown(laptop());
    app.usb_devices(&[kvm()]);
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    // Typing a key, unlocking a vault and showing a seed bring the sheet
    // back instead.
    for held in [
        Action::Entry(None),
        Action::Nav(Screen::Unlock),
        Action::BReveal,
        Action::KShow,
    ] {
        app.press(held);
        assert_eq!(app.sheet, Some(Sheet::Devices), "{held:?}");
        assert_ne!(app.screen, Screen::Entry);
        app.press(Action::Cancel);
    }
    // What shows nothing secret goes on.
    app.press(Action::Nav(Screen::Settings));
    assert_eq!(app.screen, Screen::Settings);
}

#[test]
fn check_again_after_it_is_pulled_releases_and_carries_on() {
    let mut app = shown(laptop());
    app.usb_devices(&[kvm()]);
    app.press(Action::Cancel);
    app.press(Action::Entry(None));
    // Still in: Check again keeps the sheet.
    app.press(Action::DevicesCheck);
    assert_eq!(app.sheet, Some(Sheet::Devices));
    // Pulled: the sheet still names it until Check again.
    app.usb_devices(&[]);
    assert_eq!(app.sheet, Some(Sheet::Devices));
    app.press(Action::DevicesCheck);
    assert_eq!(app.sheet, None);
    // What was pressed carries on.
    assert_eq!(app.screen, Screen::Entry);
}

#[test]
fn continue_anyway_asks_once_then_lets_it_through() {
    let mut app = shown(laptop());
    app.usb_devices(&[kvm()]);
    app.press(Action::DevicesContinue);
    assert_eq!(app.sheet, Some(Sheet::Devices));
    assert!(says(&mut app, "What is listed stays possible"));
    app.press(Action::DevicesContinue);
    assert_eq!(app.sheet, None);
    app.press(Action::Entry(None));
    assert_eq!(app.screen, Screen::Entry);
    // It stays let through; another KVM is new and named.
    app.usb_devices(&[kvm()]);
    assert_eq!(app.sheet, None);
    let mut other = kvm();
    other.port = "1-3".into();
    app.usb_devices(&[kvm(), other]);
    assert_eq!(app.sheet, Some(Sheet::Devices));
}

#[test]
fn a_virtual_machine_has_no_continue_anyway() {
    let mut app = shown(DeviceCheck::machine(
        CPU_VM,
        "QEMU",
        "Standard PC (Q35 + ICH9, 2009)",
        "1",
    ));
    app.usb_devices(&[]);
    assert_eq!(app.sheet, Some(Sheet::Devices));
    assert!(says(&mut app, "QEMU: a virtual machine's host"));
    assert!(says(&mut app, "Faraday must start on a real computer."));
    assert!(app.where_offered(Action::DevicesContinue).is_none());
    // Pressed anyway, twice, it goes nowhere; a key never loads.
    app.press(Action::DevicesContinue);
    app.press(Action::DevicesContinue);
    app.press(Action::DevicesCheck);
    assert_eq!(app.sheet, Some(Sheet::Devices));
    app.press(Action::Cancel);
    app.press(Action::Entry(None));
    assert_eq!(app.sheet, Some(Sheet::Devices));
    assert_ne!(app.screen, Screen::Entry);
}

#[test]
fn a_desktop_asks_once_whether_its_screen_is_a_tv() {
    let desktop = || DeviceCheck::machine(CPU, "Dell Inc.", "OptiPlex 7090", "3");
    let mut app = shown(desktop());
    app.usb_devices(&[]);
    assert_eq!(app.sheet, Some(Sheet::TvAsk));
    assert!(says(&mut app, "Is this screen a TV?"));
    // Unanswered, a key waits for the answer.
    app.press(Action::Cancel);
    app.press(Action::Entry(None));
    assert_eq!(app.sheet, Some(Sheet::TvAsk));
    app.press(Action::DevicesTv(false));
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::Entry);

    // A TV is held like anything else.
    let mut app = shown(desktop());
    app.usb_devices(&[]);
    app.press(Action::DevicesTv(true));
    assert_eq!(app.sheet, Some(Sheet::Devices));
    assert!(says(&mut app, "This screen: a TV can record what it shows"));
    app.press(Action::Cancel);
    app.press(Action::Entry(None));
    assert_ne!(app.screen, Screen::Entry);
    // Check again asks again: the screen may have been changed.
    app.press(Action::DevicesCheck);
    assert_eq!(app.sheet, Some(Sheet::TvAsk));
}

#[test]
fn the_stick_home_always_carries_the_tv_line() {
    let line = "On a TV? Stop: a smart TV can record its screen.";
    let mut app = shown(laptop());
    app.usb_devices(&[]);
    assert_eq!(app.screen, Screen::Home);
    assert!(says(&mut app, line));
    // Not the desktop app, which reads no machine.
    let mut desktop = shown(DeviceCheck::default());
    assert!(!says(&mut desktop, line));
}
