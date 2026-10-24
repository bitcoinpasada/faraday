//! What a person sees and can do while something that can capture the
//! screen or the keys is attached (`docs/STARTER.md` §3.1a), and the Learn
//! links (§3.1b, §3.1c) and the boot entry's removal (§3.1c).

use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};
use faraday_starter::api::{
    Bus, Computer, Concern, Disk, FaradayStick, Kind, Os, Reason, StarterCommand, StarterEvent,
};
use faraday_starter::{LearnId, Page, RestartTo, Sheet, Starter, StarterAction, reason_text};

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
            read: true,
        }),
    }
}

/// A Linux laptop with a Faraday stick attached and Secure Boot off:
/// Home's card offers Restart from the stick.
fn linux() -> Starter {
    let mut starter = Starter::new();
    starter.event(display());
    starter.starter_event(StarterEvent::Computer(computer(Os::Linux, false)));
    starter.starter_event(StarterEvent::Disks(vec![stick()]));
    starter
}

fn item(kind: Kind, name: &str, id: &str) -> Concern {
    Concern {
        kind,
        name: name.into(),
        id: id.into(),
    }
}

fn offers(starter: &mut Starter, action: StarterAction) -> bool {
    let _ = starter.drawn_texts();
    starter.offers_action(action)
}

fn shows(starter: &mut Starter, s: &str) -> bool {
    starter.drawn_texts().iter().any(|t| t.contains(s))
}

fn tap(starter: &mut Starter, action: StarterAction) {
    let _ = starter.drawn_texts();
    let (x, y) = starter.where_offered(action).expect("offered");
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        starter.event(Event::Touch { x, y, phase });
    }
}

fn commands(starter: &mut Starter) -> Vec<StarterCommand> {
    std::iter::from_fn(|| starter.next_command()).collect()
}

/// Whether the person can restart from Home or the Start page: the
/// card's button, then the Start page's.
fn can_restart(starter: &mut Starter) -> bool {
    starter.press(StarterAction::Home);
    let card = offers(starter, StarterAction::Next);
    starter.press(StarterAction::Start);
    let start = offers(starter, StarterAction::Restart);
    starter.press(StarterAction::Home);
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
        let mut starter = linux();
        assert!(can_restart(&mut starter));
        starter.starter_event(StarterEvent::Concerns(vec![item(kind, name, "x")]));
        assert!(!can_restart(&mut starter), "{name}");
        assert!(shows(&mut starter, &format!("{name} {says}")), "{name}");
        assert!(offers(&mut starter, StarterAction::CheckAgain));
        // Pressed anyway, the card asks nothing and restarts nothing.
        starter.press(StarterAction::Next);
        assert_eq!(starter.sheet, None);
        assert!(commands(&mut starter).is_empty());
        // Named on the Start page too.
        starter.press(StarterAction::Start);
        assert!(shows(&mut starter, name));
    }
}

#[test]
fn a_concern_arriving_while_the_restart_is_asked_closes_the_sheet() {
    let mut starter = linux();
    tap(&mut starter, StarterAction::Next);
    assert_eq!(starter.sheet, Some(Sheet::Restart(RestartTo::Stick)));
    starter.starter_event(StarterEvent::Concerns(vec![item(
        Kind::Tv,
        "EXAMPLE TV",
        "t",
    )]));
    assert_eq!(starter.sheet, None);
    starter.press(StarterAction::RestartNow);
    assert!(commands(&mut starter).is_empty());
}

#[test]
fn check_again_reads_again_and_a_removed_item_releases() {
    let mut starter = linux();
    starter.starter_event(StarterEvent::Concerns(vec![item(
        Kind::Tv,
        "EXAMPLE TV",
        "t",
    )]));
    tap(&mut starter, StarterAction::CheckAgain);
    assert_eq!(commands(&mut starter), vec![StarterCommand::CheckAgain]);
    // The shell's answer, the TV unplugged.
    starter.starter_event(StarterEvent::Concerns(Vec::new()));
    assert!(!shows(&mut starter, "EXAMPLE TV"));
    assert!(can_restart(&mut starter));
    tap(&mut starter, StarterAction::Next);
    tap(&mut starter, StarterAction::RestartNow);
    assert_eq!(
        commands(&mut starter),
        vec![StarterCommand::RestartFromStick {
            device: "/dev/sdb".into()
        }]
    );
}

#[test]
fn continue_anyway_asks_once_and_lets_that_item_through_for_the_session() {
    let mut starter = linux();
    let tv = item(Kind::Tv, "EXAMPLE TV", "drm:card0-HDMI-A-1");
    let kvm = item(Kind::Kvm, "Example KVM", "usb:1-2");
    starter.starter_event(StarterEvent::Concerns(vec![tv.clone(), kvm.clone()]));
    let n = StarterAction::ContinueAnyway(starter.concern_number(&tv.id));
    tap(&mut starter, n);
    assert_eq!(starter.sheet, Some(Sheet::LetThrough));
    assert!(shows(&mut starter, "Continue with EXAMPLE TV attached?"));
    // Cancelled, nothing is let through.
    tap(&mut starter, StarterAction::Cancel);
    assert!(!can_restart(&mut starter));
    tap(&mut starter, n);
    tap(&mut starter, StarterAction::LetThrough);
    assert_eq!(starter.sheet, None);
    // The KVM still holds; the TV is not named any more.
    assert!(!can_restart(&mut starter));
    assert!(!shows(&mut starter, "EXAMPLE TV is a TV"));
    starter.starter_event(StarterEvent::Concerns(vec![tv.clone()]));
    assert!(can_restart(&mut starter));
    // Read again, the same TV stays let through, and nothing is asked.
    starter.starter_event(StarterEvent::Concerns(vec![tv.clone()]));
    assert!(can_restart(&mut starter));
    assert!(!offers(&mut starter, n));
    // Another TV is another item.
    starter.starter_event(StarterEvent::Concerns(vec![
        tv,
        item(Kind::Tv, "OTHER TV", "drm:card0-HDMI-A-2"),
    ]));
    assert!(!can_restart(&mut starter));
}

#[test]
fn a_receiver_holds_nothing_and_is_one_line_about_keyboards() {
    let receiver = item(Kind::Receiver, "Example Receiver", "usb:1-2");
    let mut starter = linux();
    starter.starter_event(StarterEvent::Concerns(vec![receiver.clone()]));
    assert!(can_restart(&mut starter));
    assert!(shows(&mut starter, "Wireless receiver attached."));
    let n = StarterAction::ContinueAnyway(starter.concern_number(&receiver.id));
    assert!(!offers(&mut starter, n));
    assert!(!offers(&mut starter, StarterAction::CheckAgain));
}

#[test]
fn a_bolt_receiver_says_its_typing_is_encrypted() {
    let bolt = item(Kind::BoltReceiver, "Logitech USB Receiver", "usb:1-2");
    let mut starter = linux();
    starter.starter_event(StarterEvent::Concerns(vec![bolt.clone()]));
    assert!(can_restart(&mut starter));
    assert!(shows(&mut starter, "Logitech Bolt receiver attached."));
    assert!(!shows(&mut starter, "Wireless receiver attached."));
    // An unknown receiver beside it: the warning.
    starter.starter_event(StarterEvent::Concerns(vec![
        bolt,
        item(Kind::Receiver, "Example Dongle", "usb:1-3"),
    ]));
    assert!(shows(&mut starter, "Wireless receiver attached."));
}

#[test]
fn a_virtual_machine_holds_the_restarts_with_no_continue_anyway() {
    let mut starter = linux();
    let vm = item(Kind::VirtualMachine, "QEMU", "vm");
    starter.starter_event(StarterEvent::Concerns(vec![vm.clone()]));
    assert!(!can_restart(&mut starter));
    assert!(shows(&mut starter, "QEMU is a virtual machine"));
    assert!(shows(&mut starter, "must start on the real computer"));
    let n = StarterAction::ContinueAnyway(starter.concern_number(&vm.id));
    assert!(!offers(&mut starter, n));
    starter.press(n);
    assert_eq!(starter.sheet, None);
    assert!(!can_restart(&mut starter));
}

#[test]
fn nothing_attached_holds_nothing_and_says_nothing() {
    let mut starter = linux();
    starter.starter_event(StarterEvent::Concerns(Vec::new()));
    assert!(can_restart(&mut starter));
    assert!(!offers(&mut starter, StarterAction::CheckAgain));
}

#[test]
fn the_learn_links_are_drawn_only_where_their_pages_exist() {
    let link = |id: LearnId| StarterAction::LearnPage(id);
    let exists = |id: LearnId| id.page().is_some();
    // Secure Boot and BitLocker: Home's Turn Secure Boot back on, and the
    // Start page.
    let mut starter = linux();
    let sb = LearnId::SecureBootBitLocker;
    assert_eq!(offers(&mut starter, link(sb)), exists(sb));
    starter.press(StarterAction::Start);
    assert_eq!(offers(&mut starter, link(sb)), exists(sb));
    // Screens that record: on the warning only.
    let mut starter = linux();
    let sr = LearnId::ScreensThatRecord;
    assert!(!offers(&mut starter, link(sr)));
    starter.starter_event(StarterEvent::Concerns(vec![item(
        Kind::Tv,
        "EXAMPLE TV",
        "t",
    )]));
    assert_eq!(offers(&mut starter, link(sr)), exists(sr));
    // The boot entry: on Linux's Restart from the stick, and it opens
    // from the sheet.
    let be = LearnId::BootEntry;
    let mut starter = linux();
    tap(&mut starter, StarterAction::Next);
    assert_eq!(offers(&mut starter, link(be)), exists(be));
    if exists(be) {
        tap(&mut starter, link(be));
        assert!(starter.showing_held());
    }
    let mut starter = linux();
    starter.starter_event(StarterEvent::Computer(computer(Os::Windows, false)));
    tap(&mut starter, StarterAction::Next);
    assert_eq!(starter.sheet, Some(Sheet::Restart(RestartTo::Stick)));
    assert!(!offers(&mut starter, link(be)));
}

#[test]
fn remove_the_boot_entry_is_offered_while_it_exists_and_no_stick_is_attached() {
    let mut starter = linux();
    starter.starter_event(StarterEvent::BootEntry(true));
    // A stick attached: about to be started from, so not offered.
    assert!(!offers(&mut starter, StarterAction::RemoveEntry));
    starter.starter_event(StarterEvent::Disks(Vec::new()));
    assert!(shows(&mut starter, "Remove the Faraday boot entry"));
    tap(&mut starter, StarterAction::RemoveEntry);
    assert_eq!(
        commands(&mut starter),
        vec![StarterCommand::RemoveBootEntry]
    );
    // Declined: kept, and said.
    starter.starter_event(StarterEvent::BootEntryRemoved(Err(Reason::Declined)));
    assert!(offers(&mut starter, StarterAction::RemoveEntry));
    // The card wraps the reason over lines.
    assert!(
        starter
            .drawn_texts()
            .join(" ")
            .contains(&reason_text(&Reason::Declined))
    );
    tap(&mut starter, StarterAction::RemoveEntry);
    starter.starter_event(StarterEvent::BootEntryRemoved(Ok(())));
    assert!(!offers(&mut starter, StarterAction::RemoveEntry));
    assert_eq!(starter.page, Page::Home);
    // No entry, or Windows: never.
    let mut starter = linux();
    starter.starter_event(StarterEvent::Disks(Vec::new()));
    assert!(!offers(&mut starter, StarterAction::RemoveEntry));
    starter.starter_event(StarterEvent::Computer(computer(Os::Windows, true)));
    starter.starter_event(StarterEvent::BootEntry(true));
    assert!(!offers(&mut starter, StarterAction::RemoveEntry));
}
