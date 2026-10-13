//! What a person sees and can do while something that can capture the
//! screen or the keys is attached (`docs/HUB.md` §3.1a), and the Learn
//! links (§3.1b, §3.1c) and the boot entry's removal (§3.1c).

use faraday_hub::api::{
    BitLocker, Bus, Computer, Concern, Disk, FaradayStick, HubCommand, HubEvent, Kind, Os,
    Protection, Reason,
};
use faraday_hub::{Hub, HubAction, LearnId, Page, RestartTo, Sheet, reason_text};
use osk_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};

fn display() -> Event {
    Event::Display(DisplayInfo {
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
    })
}

fn computer(os: Os, secure_boot: bool) -> Computer {
    Computer {
        os,
        maker: "Example Computers".into(),
        model: "Example 14".into(),
        family: String::new(),
        uefi: true,
        arm: false,
        secure_boot: Some(secure_boot),
        tpm_unlock: false,
    }
}

fn stick() -> Disk {
    Disk {
        id: "sdb".into(),
        device: "/dev/sdb".into(),
        vendor: "Maker".into(),
        model: "Stick".into(),
        serial: "SN-1".into(),
        size: 32_000_000_000,
        bus: Bus::Usb,
        volumes: Vec::new(),
        in_use: None,
        faraday: Some(FaradayStick {
            release: Some("6.6.84-faraday-0.2.0".into()),
        }),
    }
}

/// A Linux laptop with a Faraday stick attached and Secure Boot off:
/// Home's card offers Restart from the stick.
fn linux() -> Hub {
    let mut hub = Hub::new();
    hub.event(display());
    hub.hub_event(HubEvent::Computer(computer(Os::Linux, false)));
    hub.hub_event(HubEvent::Disks(vec![stick()]));
    hub
}

/// A Windows laptop with BitLocker on, on the guard's page.
fn guard() -> Hub {
    let mut hub = Hub::new();
    hub.event(display());
    hub.hub_event(HubEvent::Computer(computer(Os::Windows, true)));
    hub.hub_event(HubEvent::BitLocker(Some(BitLocker {
        drive: "C:".into(),
        protection: Protection::On,
        marker: None,
    })));
    hub.press(HubAction::BitLocker);
    hub
}

fn item(kind: Kind, name: &str, id: &str) -> Concern {
    Concern {
        kind,
        name: name.into(),
        id: id.into(),
    }
}

fn offers(hub: &mut Hub, action: HubAction) -> bool {
    let _ = hub.drawn_texts();
    hub.offers_action(action)
}

fn shows(hub: &mut Hub, s: &str) -> bool {
    hub.drawn_texts().iter().any(|t| t.contains(s))
}

fn tap(hub: &mut Hub, action: HubAction) {
    let _ = hub.drawn_texts();
    let (x, y) = hub.where_offered(action).expect("offered");
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        hub.event(Event::Touch { x, y, phase });
    }
}

fn commands(hub: &mut Hub) -> Vec<HubCommand> {
    std::iter::from_fn(|| hub.poll_hub()).collect()
}

/// Whether the person can restart from Home or the Start page: the
/// card's button, then the Start page's.
fn can_restart(hub: &mut Hub) -> bool {
    hub.press(HubAction::Home);
    let card = offers(hub, HubAction::Next);
    hub.press(HubAction::Start);
    let start = offers(hub, HubAction::Restart);
    hub.press(HubAction::Home);
    assert_eq!(card, start);
    card
}

const CAPTURING: [(Kind, &str, &str); 2] = [
    (Kind::Tv, "EXAMPLE TV", "is a TV"),
    (Kind::Kvm, "Example KVM", "is a KVM switch"),
];

#[test]
fn a_tv_or_a_kvm_holds_the_restarts_and_is_named() {
    for (kind, name, says) in CAPTURING {
        let mut hub = linux();
        assert!(can_restart(&mut hub));
        hub.hub_event(HubEvent::Concerns(vec![item(kind, name, "x")]));
        assert!(!can_restart(&mut hub), "{name}");
        assert!(shows(&mut hub, &format!("{name} {says}")), "{name}");
        assert!(offers(&mut hub, HubAction::CheckAgain));
        // Pressed anyway, the card asks nothing and restarts nothing.
        hub.press(HubAction::Next);
        assert_eq!(hub.sheet, None);
        assert!(commands(&mut hub).is_empty());
        // Named on the Start page too.
        hub.press(HubAction::Start);
        assert!(shows(&mut hub, name));
    }
}

#[test]
fn a_tv_or_a_kvm_holds_show_the_recovery_key() {
    for (kind, name, says) in CAPTURING {
        let mut hub = guard();
        assert!(offers(&mut hub, HubAction::ShowKey));
        hub.hub_event(HubEvent::Concerns(vec![item(kind, name, "x")]));
        assert!(!offers(&mut hub, HubAction::ShowKey), "{name}");
        assert!(shows(&mut hub, &format!("{name} {says}")));
        hub.press(HubAction::ShowKey);
        assert!(commands(&mut hub).is_empty());
    }
}

#[test]
fn a_concern_arriving_while_the_restart_is_asked_closes_the_sheet() {
    let mut hub = linux();
    tap(&mut hub, HubAction::Next);
    assert_eq!(hub.sheet, Some(Sheet::Restart(RestartTo::Stick)));
    hub.hub_event(HubEvent::Concerns(vec![item(Kind::Tv, "EXAMPLE TV", "t")]));
    assert_eq!(hub.sheet, None);
    hub.press(HubAction::RestartNow);
    assert!(commands(&mut hub).is_empty());
}

#[test]
fn check_again_reads_again_and_a_removed_item_releases() {
    let mut hub = linux();
    hub.hub_event(HubEvent::Concerns(vec![item(Kind::Tv, "EXAMPLE TV", "t")]));
    tap(&mut hub, HubAction::CheckAgain);
    assert_eq!(commands(&mut hub), vec![HubCommand::CheckAgain]);
    // The shell's answer, the TV unplugged.
    hub.hub_event(HubEvent::Concerns(Vec::new()));
    assert!(!shows(&mut hub, "EXAMPLE TV"));
    assert!(can_restart(&mut hub));
    tap(&mut hub, HubAction::Next);
    tap(&mut hub, HubAction::RestartNow);
    assert_eq!(
        commands(&mut hub),
        vec![HubCommand::RestartFromStick {
            device: "/dev/sdb".into()
        }]
    );
}

#[test]
fn continue_anyway_asks_once_and_lets_that_item_through_for_the_session() {
    let mut hub = linux();
    let tv = item(Kind::Tv, "EXAMPLE TV", "drm:card0-HDMI-A-1");
    let kvm = item(Kind::Kvm, "Example KVM", "usb:1-2");
    hub.hub_event(HubEvent::Concerns(vec![tv.clone(), kvm.clone()]));
    let n = HubAction::ContinueAnyway(hub.concern_number(&tv.id));
    tap(&mut hub, n);
    assert_eq!(hub.sheet, Some(Sheet::LetThrough));
    assert!(shows(&mut hub, "Continue with EXAMPLE TV attached?"));
    // Cancelled, nothing is let through.
    tap(&mut hub, HubAction::Cancel);
    assert!(!can_restart(&mut hub));
    tap(&mut hub, n);
    tap(&mut hub, HubAction::LetThrough);
    assert_eq!(hub.sheet, None);
    // The KVM still holds; the TV is not named any more.
    assert!(!can_restart(&mut hub));
    assert!(!shows(&mut hub, "EXAMPLE TV is a TV"));
    hub.hub_event(HubEvent::Concerns(vec![tv.clone()]));
    assert!(can_restart(&mut hub));
    // Read again, the same TV stays let through, and nothing is asked.
    hub.hub_event(HubEvent::Concerns(vec![tv.clone()]));
    assert!(can_restart(&mut hub));
    assert!(!offers(&mut hub, n));
    // Another TV is another item.
    hub.hub_event(HubEvent::Concerns(vec![
        tv,
        item(Kind::Tv, "OTHER TV", "drm:card0-HDMI-A-2"),
    ]));
    assert!(!can_restart(&mut hub));
}

#[test]
fn a_receiver_holds_nothing_and_is_one_line_about_keyboards() {
    let receiver = item(Kind::Receiver, "Example Receiver", "usb:1-2");
    let mut hub = linux();
    hub.hub_event(HubEvent::Concerns(vec![receiver.clone()]));
    assert!(can_restart(&mut hub));
    assert!(shows(&mut hub, "Wireless receiver attached."));
    let n = HubAction::ContinueAnyway(hub.concern_number(&receiver.id));
    assert!(!offers(&mut hub, n));
    assert!(!offers(&mut hub, HubAction::CheckAgain));
    // Nor the recovery key.
    let mut hub = guard();
    hub.hub_event(HubEvent::Concerns(vec![receiver]));
    assert!(offers(&mut hub, HubAction::ShowKey));
}

#[test]
fn a_bolt_receiver_says_its_typing_is_encrypted() {
    let bolt = item(Kind::BoltReceiver, "Logitech USB Receiver", "usb:1-2");
    let mut hub = linux();
    hub.hub_event(HubEvent::Concerns(vec![bolt.clone()]));
    assert!(can_restart(&mut hub));
    assert!(shows(&mut hub, "Logitech Bolt receiver attached."));
    assert!(!shows(&mut hub, "Wireless receiver attached."));
    // An unknown receiver beside it: the warning.
    hub.hub_event(HubEvent::Concerns(vec![
        bolt,
        item(Kind::Receiver, "Example Dongle", "usb:1-3"),
    ]));
    assert!(shows(&mut hub, "Wireless receiver attached."));
}

#[test]
fn a_virtual_machine_holds_the_restarts_with_no_continue_anyway() {
    let mut hub = linux();
    let vm = item(Kind::VirtualMachine, "QEMU", "vm");
    hub.hub_event(HubEvent::Concerns(vec![vm.clone()]));
    assert!(!can_restart(&mut hub));
    assert!(shows(&mut hub, "QEMU is a virtual machine"));
    assert!(shows(&mut hub, "must start on the real computer"));
    let n = HubAction::ContinueAnyway(hub.concern_number(&vm.id));
    assert!(!offers(&mut hub, n));
    hub.press(n);
    assert_eq!(hub.sheet, None);
    assert!(!can_restart(&mut hub));
}

#[test]
fn a_remote_session_holds_only_the_recovery_key() {
    let session = item(Kind::RemoteSession, "", "session");
    let mut hub = linux();
    hub.hub_event(HubEvent::Concerns(vec![session.clone()]));
    assert!(can_restart(&mut hub));
    assert!(!shows(&mut hub, "remote session"));
    let mut hub = guard();
    hub.hub_event(HubEvent::Concerns(vec![
        session,
        item(Kind::WirelessDisplay, "Example Cast", "display:1"),
    ]));
    assert!(!offers(&mut hub, HubAction::ShowKey));
    assert!(shows(&mut hub, "This session is a remote session"));
    assert!(shows(&mut hub, "Example Cast is a wireless display"));
}

#[test]
fn nothing_attached_holds_nothing_and_says_nothing() {
    let mut hub = linux();
    hub.hub_event(HubEvent::Concerns(Vec::new()));
    assert!(can_restart(&mut hub));
    assert!(!offers(&mut hub, HubAction::CheckAgain));
    let mut hub = guard();
    hub.hub_event(HubEvent::Concerns(Vec::new()));
    assert!(offers(&mut hub, HubAction::ShowKey));
}

#[test]
fn the_learn_links_are_drawn_only_where_their_pages_exist() {
    let link = |id: LearnId| HubAction::LearnPage(id);
    let exists = |id: LearnId| id.page().is_some();
    // Secure Boot and BitLocker: Home's card, the Start page, the guard.
    let mut hub = linux();
    let sb = LearnId::SecureBootBitLocker;
    assert_eq!(offers(&mut hub, link(sb)), exists(sb));
    hub.press(HubAction::Start);
    assert_eq!(offers(&mut hub, link(sb)), exists(sb));
    let mut g = guard();
    assert_eq!(offers(&mut g, link(sb)), exists(sb));
    // Screens that record: on the warning only.
    let mut hub = linux();
    let sr = LearnId::ScreensThatRecord;
    assert!(!offers(&mut hub, link(sr)));
    hub.hub_event(HubEvent::Concerns(vec![item(Kind::Tv, "EXAMPLE TV", "t")]));
    assert_eq!(offers(&mut hub, link(sr)), exists(sr));
    // The boot entry: on Linux's Restart from the stick, and it opens
    // from the sheet.
    let be = LearnId::BootEntry;
    let mut hub = linux();
    tap(&mut hub, HubAction::Next);
    assert_eq!(offers(&mut hub, link(be)), exists(be));
    if exists(be) {
        tap(&mut hub, link(be));
        assert!(hub.showing_held());
    }
    let mut hub = linux();
    hub.hub_event(HubEvent::Computer(computer(Os::Windows, false)));
    hub.hub_event(HubEvent::BitLocker(Some(BitLocker {
        drive: "C:".into(),
        protection: Protection::None,
        marker: None,
    })));
    tap(&mut hub, HubAction::Next);
    assert_eq!(hub.sheet, Some(Sheet::Restart(RestartTo::Stick)));
    assert!(!offers(&mut hub, link(be)));
}

#[test]
fn remove_the_boot_entry_is_offered_while_it_exists_and_no_stick_is_attached() {
    let mut hub = linux();
    hub.hub_event(HubEvent::BootEntry(true));
    // A stick attached: about to be started from, so not offered.
    assert!(!offers(&mut hub, HubAction::RemoveEntry));
    hub.hub_event(HubEvent::Disks(Vec::new()));
    assert!(shows(&mut hub, "Remove the Faraday boot entry"));
    tap(&mut hub, HubAction::RemoveEntry);
    assert_eq!(commands(&mut hub), vec![HubCommand::RemoveBootEntry]);
    // Declined: kept, and said.
    hub.hub_event(HubEvent::BootEntryRemoved(Err(Reason::Declined)));
    assert!(offers(&mut hub, HubAction::RemoveEntry));
    assert!(shows(&mut hub, &reason_text(&Reason::Declined)));
    tap(&mut hub, HubAction::RemoveEntry);
    hub.hub_event(HubEvent::BootEntryRemoved(Ok(())));
    assert!(!offers(&mut hub, HubAction::RemoveEntry));
    assert_eq!(hub.page, Page::Home);
    // No entry, or Windows: never.
    let mut hub = linux();
    hub.hub_event(HubEvent::Disks(Vec::new()));
    assert!(!offers(&mut hub, HubAction::RemoveEntry));
    hub.hub_event(HubEvent::Computer(computer(Os::Windows, true)));
    hub.hub_event(HubEvent::BootEntry(true));
    assert!(!offers(&mut hub, HubAction::RemoveEntry));
}
