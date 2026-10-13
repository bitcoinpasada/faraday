//! What a person sees and can do in the hub app (`docs/HUB.md` §11):
//! the shell's reports in, the frame's words and the hub's commands out.

use faraday_hub::api::{
    BitLocker, Bus, Computer, DevImage, Disk, FaradayStick, HubCommand, HubEvent, InUse, Marker,
    Mode, Os, Payload, PayloadFile, Protection, Reason, Remind, Source, Volume, remind,
};
use faraday_hub::decide::{Item, Target};
use faraday_hub::{Hub, HubAction, LearnId, Page, RestartTo, Sheet};
use osk_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};

const GB: u64 = 1_000_000_000;

fn display(width: u16, height: u16, dpi: u16) -> Event {
    Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    })
}

fn payload(stick_build: &str) -> Payload {
    let file = |name: &str, inner: &str, size: u64, build: &str, n: char| PayloadFile {
        name: name.into(),
        sha256: n.to_string().repeat(64),
        inner: inner.into(),
        inner_sha256: n.to_ascii_uppercase().to_string().repeat(64),
        inner_size: size,
        build_id: build.into(),
    };
    Payload {
        version: "0.2.0".into(),
        commit: "4d0680b1a2b34d0680b1a2b34d0680b1a2b34d06".into(),
        files: vec![
            file(
                "faraday-stick-0.2.0-x86_64-uefi.img.xz",
                "faraday-stick-0.2.0-x86_64-uefi.img",
                4 * GB,
                stick_build,
                'a',
            ),
            file(
                "faraday-pi-0.2.0-pi3-waveshare-28dpi.img.xz",
                "faraday-pi-0.2.0-pi3-waveshare-28dpi.img",
                3 * GB,
                "0.2.0",
                'b',
            ),
            file(
                "faraday-desktop-0.2.0-linux-x86_64.xz",
                "faraday-desktop-0.2.0-linux-x86_64",
                4_000_000,
                "0.2.0",
                'c',
            ),
        ],
    }
}

fn disk(id: &str, model: &str, size: u64) -> Disk {
    Disk {
        id: id.into(),
        device: format!("/dev/{id}"),
        vendor: "Maker".into(),
        model: model.into(),
        serial: format!("SN-{id}"),
        size,
        bus: Bus::Usb,
        volumes: vec![Volume {
            label: Some(format!("{model}VOL")),
            files: true,
        }],
        in_use: None,
        faraday: None,
    }
}

fn faraday_stick(id: &str, model: &str, size: u64, release: Option<&str>) -> Disk {
    Disk {
        faraday: Some(FaradayStick {
            release: release.map(str::to_string),
        }),
        volumes: Vec::new(),
        ..disk(id, model, size)
    }
}

fn linux(secure_boot: bool) -> Computer {
    Computer {
        os: Os::Linux,
        maker: "Example Computers".into(),
        model: "Example 14".into(),
        family: String::new(),
        uefi: true,
        arm: false,
        secure_boot: Some(secure_boot),
        tpm_unlock: false,
    }
}

fn windows(secure_boot: bool) -> Computer {
    Computer {
        os: Os::Windows,
        ..linux(secure_boot)
    }
}

fn hub_with(disks: Vec<Disk>) -> Hub {
    let mut hub = Hub::new();
    hub.event(display(1280, 800, 160));
    hub.hub_event(HubEvent::Computer(linux(true)));
    hub.hub_event(HubEvent::Payload(Some(payload("0.2.0"))));
    hub.hub_event(HubEvent::Disks(disks));
    hub
}

fn texts(hub: &mut Hub) -> Vec<String> {
    hub.drawn_texts()
}

fn shows(hub: &mut Hub, s: &str) -> bool {
    texts(hub).iter().any(|t| t.contains(s))
}

fn position(hub: &mut Hub, s: &str) -> Option<usize> {
    texts(hub).iter().position(|t| t.contains(s))
}

fn number(hub: &Hub, id: &str) -> HubAction {
    HubAction::Disk(hub.disk_number(id))
}

fn tap(hub: &mut Hub, action: HubAction) {
    let _ = hub.drawn_texts();
    let (x, y) = hub.where_offered(action).expect("offered");
    hub.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    hub.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
}

fn commands(hub: &mut Hub) -> Vec<HubCommand> {
    std::iter::from_fn(|| hub.poll_hub()).collect()
}

/// To Confirm for disk `id`, with `mode`.
fn to_confirm(hub: &mut Hub, id: &str, mode: Mode) {
    hub.press(HubAction::Make(Target::Stick));
    let n = number(hub, id);
    hub.press(n);
    hub.press(HubAction::Mode(mode));
    hub.press(HubAction::Continue);
    assert_eq!(hub.page, Page::Confirm);
}

#[test]
fn faraday_sticks_come_first_then_the_others_by_size() {
    let mut hub = hub_with(vec![
        disk("sdb", "Big", 64 * GB),
        disk("sdc", "Small", 8 * GB),
        faraday_stick("sdd", "Old", 32 * GB, Some("6.6.84-faraday-0.1.0")),
        disk("sde", "Middle", 16 * GB),
    ]);
    hub.press(HubAction::Make(Target::Stick));
    let order: Vec<usize> = ["Old", "Small", "Middle", "Big"]
        .iter()
        .map(|m| position(&mut hub, &format!("Maker {m}")).expect("listed"))
        .collect();
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
    assert!(shows(&mut hub, "Faraday stick · 0.1.0"));
    assert!(shows(&mut hub, "SmallVOL, files on it"));
}

#[test]
fn disks_the_computer_uses_and_small_ones_are_shown_dimmed_with_the_reason() {
    let mut home = disk("sdb", "Home", 64 * GB);
    home.in_use = Some(InUse::Mounted("/home".into()));
    let mut swap = disk("sdc", "Swap", 64 * GB);
    swap.in_use = Some(InUse::Swap);
    let tiny = disk("sdd", "Tiny", 2 * GB);
    let mut nvme = disk("nvme0n1", "Internal", 512 * GB);
    nvme.bus = Bus::Other;
    let mut hub = hub_with(vec![home, swap, tiny, nvme, disk("sde", "Fine", 16 * GB)]);
    hub.press(HubAction::Make(Target::Stick));
    assert!(shows(&mut hub, "Used by this computer: mounted at /home"));
    assert!(shows(&mut hub, "Used by this computer: swap"));
    assert!(shows(&mut hub, "Smaller than the image: needs 4.0 GB"));
    assert!(!shows(&mut hub, "Maker Internal"));
    for id in ["sdb", "sdc", "sdd"] {
        let n = number(&hub, id);
        assert!(!hub.offers_action(n), "{id} offered");
        hub.press(n);
        assert_eq!(hub.page, Page::Disks, "{id} could be chosen");
    }
    let fine = number(&hub, "sde");
    assert!(hub.offers_action(fine));
}

#[test]
fn no_disk_is_chosen_until_one_is_tapped() {
    let mut hub = hub_with(vec![disk("sdb", "Only", 16 * GB)]);
    tap(&mut hub, HubAction::Next);
    assert_eq!(hub.page, Page::Disks);
    // One disk attached, and still nothing chosen: Which stick? stays,
    // with nothing to go on with.
    assert!(!hub.offers_action(HubAction::Continue));
    assert!(!hub.offers_action(HubAction::Write));
    hub.hub_event(HubEvent::Disks(vec![disk("sdb", "Only", 16 * GB)]));
    assert_eq!(hub.page, Page::Disks);
    assert!(commands(&mut hub).is_empty());
    let n = number(&hub, "sdb");
    tap(&mut hub, n);
    assert_eq!(hub.page, Page::Mode);
    // What to write has nothing ticked: Continue does nothing yet.
    hub.press(HubAction::Continue);
    assert_eq!(hub.page, Page::Mode);
}

#[test]
fn the_list_follows_disks_plugged_in_and_pulled_out() {
    let mut hub = hub_with(Vec::new());
    hub.press(HubAction::Make(Target::Stick));
    assert!(shows(&mut hub, "No USB stick attached"));
    hub.hub_event(HubEvent::Disks(vec![disk("sdb", "New", 16 * GB)]));
    assert!(shows(&mut hub, "Maker New"));
    hub.hub_event(HubEvent::Disks(Vec::new()));
    assert!(!shows(&mut hub, "Maker New"));
    assert!(shows(&mut hub, "No USB stick attached"));
}

#[test]
fn confirm_writes_the_disk_confirmed_with_its_serial_and_size() {
    let mut hub = hub_with(vec![disk("sdb", "Stick", 16 * GB)]);
    to_confirm(&mut hub, "sdb", Mode::Whole);
    for s in [
        "Maker Stick",
        "/dev/sdb",
        "SN-sdb",
        "16 GB",
        "Erase and write",
    ] {
        assert!(shows(&mut hub, s), "{s}");
    }
    hub.press(HubAction::Write);
    let c = commands(&mut hub);
    let [HubCommand::Write(order)] = c.as_slice() else {
        panic!("{c:?}");
    };
    assert_eq!(order.disk, "sdb");
    assert_eq!(order.serial, "SN-sdb");
    assert_eq!(order.size, 16 * GB);
    assert_eq!(order.mode, Mode::Whole);
    assert!(
        matches!(&order.source, Source::Payload { name, .. } if name.starts_with("faraday-stick-"))
    );
    assert_eq!(hub.page, Page::Writing);
}

#[test]
fn a_disk_that_is_not_the_confirmed_one_is_refused() {
    // Another stick in its place: same handle, another serial.
    let mut hub = hub_with(vec![disk("sdb", "Stick", 16 * GB)]);
    to_confirm(&mut hub, "sdb", Mode::Whole);
    let mut other = disk("sdb", "Stick", 16 * GB);
    other.serial = "SN-OTHER".into();
    hub.hub_event(HubEvent::Disks(vec![other]));
    assert!(shows(&mut hub, "Not the stick confirmed"));
    hub.press(HubAction::Write);
    assert!(commands(&mut hub).is_empty());
    assert_eq!(hub.page, Page::Confirm);

    // The same serial, another size.
    let mut hub = hub_with(vec![disk("sdb", "Stick", 16 * GB)]);
    to_confirm(&mut hub, "sdb", Mode::Whole);
    hub.hub_event(HubEvent::Disks(vec![disk("sdb", "Stick", 32 * GB)]));
    hub.press(HubAction::Write);
    assert!(commands(&mut hub).is_empty());

    // Pulled out.
    let mut hub = hub_with(vec![disk("sdb", "Stick", 16 * GB)]);
    to_confirm(&mut hub, "sdb", Mode::Whole);
    hub.hub_event(HubEvent::Disks(Vec::new()));
    hub.press(HubAction::Write);
    assert!(commands(&mut hub).is_empty());
    assert!(shows(&mut hub, "Not the stick confirmed"));
}

#[test]
fn keep_data_is_offered_only_on_a_faraday_stick() {
    let mut hub = hub_with(vec![disk("sdb", "Plain", 16 * GB)]);
    hub.press(HubAction::Make(Target::Stick));
    let n = number(&hub, "sdb");
    hub.press(n);
    assert!(shows(&mut hub, "New Faraday stick"));
    assert!(!shows(&mut hub, "Replace Faraday, keep data"));
    hub.press(HubAction::Mode(Mode::KeepData));
    hub.press(HubAction::Continue);
    assert_eq!(hub.page, Page::Mode);

    let mut hub = hub_with(vec![faraday_stick(
        "sdb",
        "Mine",
        16 * GB,
        Some("6.6.84-faraday-0.1.0"),
    )]);
    to_confirm(&mut hub, "sdb", Mode::KeepData);
    assert!(shows(&mut hub, "Write"));
    hub.press(HubAction::Write);
    let c = commands(&mut hub);
    assert!(
        matches!(c.as_slice(), [HubCommand::Write(o)] if o.mode == Mode::KeepData),
        "{c:?}"
    );
}

#[test]
fn keep_data_refuses_the_same_build() {
    let mut hub = hub_with(vec![faraday_stick(
        "sdb",
        "Mine",
        16 * GB,
        Some("6.6.84-faraday-0.2.0"),
    )]);
    hub.press(HubAction::Make(Target::Stick));
    let n = number(&hub, "sdb");
    hub.press(n);
    assert!(shows(&mut hub, "Already this build"));
    assert!(!hub.offers_action(HubAction::Mode(Mode::KeepData)));
    hub.press(HubAction::Mode(Mode::KeepData));
    hub.press(HubAction::Continue);
    assert_eq!(hub.page, Page::Mode);
    // The whole stick can still be written.
    assert!(hub.offers_action(HubAction::Mode(Mode::Whole)));
}

#[test]
fn keep_data_warns_about_an_older_faraday_over_a_newer_stick_and_writes_it() {
    let mut hub = hub_with(vec![faraday_stick(
        "sdb",
        "Mine",
        16 * GB,
        Some("6.6.84-faraday-0.3.0"),
    )]);
    to_confirm(&mut hub, "sdb", Mode::KeepData);
    assert!(shows(&mut hub, "This stick holds a newer Faraday: 0.3.0"));
    hub.press(HubAction::Write);
    assert_eq!(commands(&mut hub).len(), 1);

    // An older stick: no warning.
    let mut hub = hub_with(vec![faraday_stick(
        "sdb",
        "Mine",
        16 * GB,
        Some("6.6.84-faraday-0.1.0"),
    )]);
    to_confirm(&mut hub, "sdb", Mode::KeepData);
    assert!(!shows(&mut hub, "newer Faraday"));
}

#[test]
fn a_developers_image_says_it_is_not_the_apps_own_and_shows_its_sum() {
    let mut hub = hub_with(vec![disk("sdb", "Stick", 16 * GB)]);
    hub.hub_event(HubEvent::DevImage(DevImage {
        path: "out/stick/faraday-x86_64-uefi-dev.img".into(),
        sha256: "d".repeat(64),
        size: 4 * GB,
        boot_only: false,
        build_id: Some("0.2.0+4d0680b1a2b3.dev".into()),
    }));
    to_confirm(&mut hub, "sdb", Mode::Whole);
    assert!(shows(&mut hub, "Not Faraday Starter's own image"));
    assert!(shows(&mut hub, "dddd dddd"));
    assert!(shows(&mut hub, "0.2.0 dev (4d0680b1a2b3)"));
    hub.press(HubAction::Write);
    let c = commands(&mut hub);
    assert!(
        matches!(c.as_slice(), [HubCommand::Write(o)] if matches!(&o.source, Source::Image { path, .. } if path.ends_with("-dev.img"))),
        "{c:?}"
    );
}

#[test]
fn writing_shows_progress_then_done_or_the_writers_reason() {
    let mut hub = hub_with(vec![disk("sdb", "Stick", 16 * GB)]);
    to_confirm(&mut hub, "sdb", Mode::Whole);
    hub.press(HubAction::Write);
    hub.hub_event(HubEvent::WriteProgress {
        stage: faraday_hub::api::WriteStage::Writing,
        done: GB,
        total: 4 * GB,
    });
    assert!(shows(&mut hub, "Written 1.0 GB of 4.0 GB"));
    hub.hub_event(HubEvent::WriteProgress {
        stage: faraday_hub::api::WriteStage::ReadingBack,
        done: 2 * GB,
        total: 4 * GB,
    });
    assert!(shows(&mut hub, "Reading back"));
    hub.hub_event(HubEvent::Written(Err(Reason::BackDiffers {
        device: "/dev/sdb".into(),
        at: 12,
    })));
    assert!(shows(&mut hub, "differs from what was written at byte 12"));
    assert!(!shows(&mut hub, "can be pulled out"));

    let mut hub = hub_with(vec![disk("sdb", "Stick", 16 * GB)]);
    to_confirm(&mut hub, "sdb", Mode::Whole);
    hub.press(HubAction::Write);
    hub.hub_event(HubEvent::Written(Ok(())));
    assert!(shows(&mut hub, "The stick can be pulled out."));
    hub.press(HubAction::Start);
    assert_eq!(hub.page, Page::Start);
}

#[test]
fn a_pi_card_is_offered_sd_card_readers_and_no_keep_data() {
    let mut reader = disk("mmcblk0", "Card", 32 * GB);
    reader.bus = Bus::SdReader;
    let mut hub = hub_with(vec![reader.clone()]);
    hub.press(HubAction::Make(Target::Stick));
    assert!(!shows(&mut hub, "Maker Card"));
    hub.press(HubAction::Home);
    hub.press(HubAction::Make(Target::Pi));
    assert!(shows(&mut hub, "Raspberry Pi 3B"));
    assert!(shows(&mut hub, "Maker Card"));
    let mut fs = faraday_stick("mmcblk0", "Card", 32 * GB, Some("6.6.84-faraday-0.1.0"));
    fs.bus = Bus::SdReader;
    hub.hub_event(HubEvent::Disks(vec![fs]));
    let n = number(&hub, "mmcblk0");
    hub.press(n);
    assert!(shows(&mut hub, "New Faraday card"));
    assert!(!shows(&mut hub, "keep data"));
}

/// Home's card button: whether it can be pressed, and the page and
/// sheet a press on it opens (the sheet is closed again with Cancel).
fn next_step(hub: &mut Hub) -> (bool, Page, Option<Sheet>) {
    let _ = hub.drawn_texts();
    let offered = hub.offers_action(HubAction::Next);
    hub.press(HubAction::Next);
    let opened = (offered, hub.page, hub.sheet);
    hub.press(HubAction::Cancel);
    hub.press(HubAction::Home);
    opened
}

const FIRMWARE: Option<Sheet> = Some(Sheet::Restart(RestartTo::Firmware));
const STICK: Option<Sheet> = Some(Sheet::Restart(RestartTo::Stick));

fn suspended(secure_boot_was_on: bool) -> BitLocker {
    BitLocker {
        drive: "C:".into(),
        protection: Protection::Suspended,
        marker: Some(Marker {
            volumes: vec!["C:".into()],
            secure_boot_was_on,
            this_boot: false,
        }),
    }
}

#[test]
fn home_leads_with_the_next_step_for_each_state() {
    // No stick.
    let mut hub = hub_with(Vec::new());
    assert!(position(&mut hub, "Make a Faraday stick") < position(&mut hub, "Checklist"));
    assert!(shows(&mut hub, "No Faraday stick attached"));
    assert_eq!(next_step(&mut hub), (true, Page::Disks, None));

    // A stick attached.
    let mut hub = hub_with(vec![faraday_stick(
        "sdb",
        "Mine",
        16 * GB,
        Some("6.6.84-faraday-0.2.0"),
    )]);
    assert!(shows(&mut hub, "Faraday stick attached · 0.2.0"));
    assert_eq!(next_step(&mut hub), (true, Page::Home, FIRMWARE));

    // Windows, protected: the BitLocker step.
    hub.hub_event(HubEvent::Computer(windows(true)));
    hub.hub_event(HubEvent::BitLocker(Some(BitLocker {
        drive: "C:".into(),
        protection: Protection::On,
        marker: None,
    })));
    assert!(shows(&mut hub, "Protected"));
    assert_eq!(next_step(&mut hub), (true, Page::BitLocker, None));

    // Suspended by Faraday, Secure Boot off: turn it back on, even with
    // no stick attached.
    hub.hub_event(HubEvent::Disks(Vec::new()));
    hub.hub_event(HubEvent::Computer(windows(false)));
    hub.hub_event(HubEvent::BitLocker(Some(suspended(true))));
    assert!(card_says(&mut hub, "Restart into firmware settings"));
    assert!(shows(&mut hub, "Turn Secure Boot back on"));
    assert!(shows(&mut hub, "Needs Secure Boot on"));
    assert_eq!(next_step(&mut hub), (true, Page::Home, FIRMWARE));

    // Suspended, Secure Boot on again: resume.
    hub.hub_event(HubEvent::Computer(windows(true)));
    assert!(position(&mut hub, "Resume BitLocker") < position(&mut hub, "Checklist"));
    assert!(!shows(&mut hub, "Turn Secure Boot back on"));
    assert_eq!(next_step(&mut hub), (true, Page::BitLocker, None));
}

#[test]
fn resume_follows_secure_boot_as_the_marker_recorded_it() {
    // The marker recorded Secure Boot off, and it is off now: Resume is
    // offered, and nobody is told to turn Secure Boot on.
    let mut hub = hub_with(Vec::new());
    hub.hub_event(HubEvent::Computer(windows(false)));
    hub.hub_event(HubEvent::BitLocker(Some(suspended(false))));
    assert!(!shows(&mut hub, "Turn Secure Boot back on"));
    assert!(!shows(&mut hub, "Needs Secure Boot"));
    assert!(position(&mut hub, "Resume BitLocker") < position(&mut hub, "Checklist"));
    assert_eq!(next_step(&mut hub), (true, Page::BitLocker, None));

    // Recorded off, on now: not offered.
    hub.hub_event(HubEvent::Computer(windows(true)));
    assert!(shows(&mut hub, "Needs Secure Boot off"));
    assert!(!next_step(&mut hub).0);
}

#[test]
fn remind_shows_a_window_in_one_state_of_three() {
    let on = Marker {
        volumes: vec!["C:".into()],
        secure_boot_was_on: true,
        this_boot: false,
    };
    let off = Marker {
        secure_boot_was_on: false,
        ..on.clone()
    };
    let now = Marker {
        this_boot: true,
        ..on.clone()
    };
    assert_eq!(
        remind(None, Protection::Suspended, Some(true)),
        Remind::RemoveAndExit
    );
    assert_eq!(
        remind(Some(&on), Protection::On, Some(true)),
        Remind::RemoveAndExit
    );
    assert_eq!(
        remind(Some(&on), Protection::Suspended, Some(false)),
        Remind::Exit
    );
    assert_eq!(remind(Some(&on), Protection::Suspended, None), Remind::Exit);
    assert_eq!(
        remind(Some(&on), Protection::Suspended, Some(true)),
        Remind::Open
    );
    assert_eq!(
        remind(Some(&off), Protection::Suspended, Some(false)),
        Remind::Open
    );
    assert_eq!(
        remind(Some(&off), Protection::Suspended, Some(true)),
        Remind::Exit
    );
    // In the boot that suspended, no window.
    assert_eq!(
        remind(Some(&now), Protection::Suspended, Some(true)),
        Remind::Exit
    );
}

#[test]
fn start_holds_the_firmware_buttons_until_the_bitlocker_step_is_done() {
    let mut hub = hub_with(Vec::new());
    hub.hub_event(HubEvent::Computer(windows(true)));
    hub.hub_event(HubEvent::BitLocker(Some(BitLocker {
        drive: "C:".into(),
        protection: Protection::On,
        marker: None,
    })));
    hub.press(HubAction::Start);
    assert!(shows(&mut hub, "Restart into firmware settings"));
    assert!(!hub.offers_action(HubAction::Restart));
    hub.press(HubAction::Restart);
    assert!(hub.sheet.is_none());

    hub.hub_event(HubEvent::BitLocker(Some(suspended(true))));
    tap(&mut hub, HubAction::Restart);
    assert!(shows(
        &mut hub,
        "This restarts the computer now. Save your work first."
    ));
    // The guide's address is on the confirmation as a code.
    assert!(shows(&mut hub, "https://faradayoffline.app/start/windows"));
    tap(&mut hub, HubAction::RestartNow);
    assert_eq!(commands(&mut hub), vec![HubCommand::RestartIntoFirmware]);
}

// The guide chosen for a maker string (§3.4, §7.6): the maker's page
// where there is one, by the maker and, for a maker with families, the
// family or model; the general page for the operating system otherwise.
// The Start page links it and the restart confirmation carries its
// address.

fn guide_for(maker: &str, family: &str, model: &str, os: Os) -> &'static str {
    let c = Computer {
        os,
        maker: maker.into(),
        family: family.into(),
        model: model.into(),
        ..linux(true)
    };
    LearnId::for_computer(&c).id()
}

#[test]
fn the_guide_is_the_makers_page_where_there_is_one() {
    assert_eq!(
        guide_for("Dell Inc.", "Example", "Example 14", Os::Windows),
        "maker-dell"
    );
    assert_eq!(
        guide_for("LENOVO", "ThinkPad Example 14", "21XX0000", Os::Linux),
        "maker-lenovo-thinkpad"
    );
    assert_eq!(
        guide_for("LENOVO", "IdeaPad Example 14", "82XX", Os::Windows),
        "maker-lenovo-ideapad"
    );
    // A Lenovo of a family without a page gets the general page.
    assert_eq!(
        guide_for("LENOVO", "Example 14", "Example 14", Os::Windows),
        "windows"
    );
    assert_eq!(
        guide_for("Example Computers", "Example", "Example 14", Os::Windows),
        "windows"
    );
    assert_eq!(guide_for("", "", "", Os::Linux), "linux");
    assert_eq!(guide_for("", "", "", Os::Windows), "windows");
    // Every maker page is there to open.
    for m in faraday_core::learn::MAKERS {
        assert!(LearnId::Maker(m.id).page().is_some(), "no page {}", m.id);
    }
}

#[test]
fn start_opens_the_makers_page_and_its_address_is_on_the_restart() {
    let mut hub = hub_with(Vec::new());
    let mut c = windows(true);
    c.maker = "Dell Inc.".into();
    hub.hub_event(HubEvent::Computer(c));
    hub.hub_event(HubEvent::BitLocker(Some(suspended(true))));
    hub.press(HubAction::Start);
    assert!(shows(&mut hub, "Starting a Dell computer from the stick"));
    assert!(hub.offers_action(HubAction::LearnPage(LearnId::Maker("maker-dell"))));
    tap(&mut hub, HubAction::Restart);
    assert!(shows(
        &mut hub,
        "https://faradayoffline.app/start/maker-dell"
    ));
}

#[test]
fn about_lists_the_payload() {
    let mut hub = hub_with(Vec::new());
    hub.press(HubAction::About);
    for s in [
        "faraday-stick-0.2.0-x86_64-uefi.img.xz",
        "faraday-pi-0.2.0-pi3-waveshare-28dpi.img.xz",
        "faraday-desktop-0.2.0-linux-x86_64.xz",
        "aaaa aaaa",
        "AAAA AAAA",
        "SHA-256 of faraday-stick-0.2.0-x86_64-uefi.img",
        "D1AF A712 ABEF 9010 9A1B  587F F153 6BE0 7E23 4CFD",
    ] {
        assert!(seen_scrolling(&mut hub, s), "{s}");
    }
}

/// Whether `s` is drawn anywhere on the page, scrolled from its top to
/// its end.
fn seen_scrolling(hub: &mut Hub, s: &str) -> bool {
    for _ in 0..12 {
        hub.event(Event::Wheel {
            x: 0,
            y: 0,
            dy: -2000,
        });
    }
    for _ in 0..40 {
        if shows(hub, s) {
            return true;
        }
        hub.event(Event::Wheel {
            x: 0,
            y: 0,
            dy: 200,
        });
    }
    false
}

#[test]
fn install_offers_the_desktop_app_only_where_the_payload_carries_it() {
    let mut hub = hub_with(Vec::new());
    hub.press(HubAction::Install);
    assert!(shows(&mut hub, "Also install the desktop app"));
    hub.press(HubAction::InstallDesktop);
    hub.press(HubAction::InstallNow);
    assert_eq!(
        commands(&mut hub),
        vec![HubCommand::Install { desktop: true }]
    );

    let mut p = payload("0.2.0");
    p.files.retain(|f| !f.name.starts_with("faraday-desktop-"));
    hub.hub_event(HubEvent::Payload(Some(p)));
    hub.hub_event(HubEvent::InstallDone(Err(Reason::NoHome)));
    hub.press(HubAction::Install);
    assert!(!shows(&mut hub, "Also install the desktop app"));
    hub.press(HubAction::Home);
    assert!(!shows(&mut hub, "Try the desktop app"));
}

#[test]
fn transfer_is_the_desktop_apps_and_back_returns_to_home() {
    let mut hub = hub_with(Vec::new());
    hub.press(HubAction::Transfer);
    assert!(hub.showing_held());
    assert!(shows(&mut hub, "Transfer"));
    assert!(!shows(&mut hub, "Wallets"));
    hub.event(Event::Back);
    assert!(!hub.showing_held());
    assert_eq!(hub.page, Page::Home);
    assert!(shows(&mut hub, "Checklist"));

    // Learn: the sheet with the guides, closed back to Home.
    hub.press(HubAction::Learn);
    assert!(hub.showing_held());
    assert!(shows(
        &mut hub,
        "Starting a Windows computer from the stick"
    ));
    hub.event(Event::Key(osk_shell_api::Key::Escape));
    assert!(!hub.showing_held());
}

#[test]
fn checklist_items_that_do_not_apply_are_not_shown() {
    let mut hub = hub_with(Vec::new());
    // Linux: no BitLocker items (the Learn link's title names it).
    let t = texts(&mut hub);
    assert!(
        !t.iter()
            .any(|t| t == "BitLocker" || t.contains("Resume BitLocker"))
    );
    let _ = Item::Resume;
    // An ARM computer: only Make a stick, and the line that says why.
    let mut arm = windows(true);
    arm.arm = true;
    hub.hub_event(HubEvent::Computer(arm));
    assert!(shows(
        &mut hub,
        "This computer cannot start a Faraday stick"
    ));
    assert!(!shows(&mut hub, "Start this computer from the stick"));
}

fn stick_attached() -> Vec<Disk> {
    vec![faraday_stick(
        "sdb",
        "Mine",
        16 * GB,
        Some("6.6.84-faraday-0.2.0"),
    )]
}

/// Whether the frame reads `s`, across the lines it wraps into.
fn reads(hub: &mut Hub, s: &str) -> bool {
    texts(hub).join(" ").contains(s)
}

/// Whether the card (above the checklist) carries `label`.
fn card_says(hub: &mut Hub, label: &str) -> bool {
    matches!(
        (position(hub, label), position(hub, "Checklist")),
        (Some(a), Some(b)) if a < b
    )
}

#[test]
fn the_cards_button_follows_this_computers_state() {
    // No stick: Make a Faraday stick, no sheet.
    let mut hub = hub_with(Vec::new());
    hub.hub_event(HubEvent::Computer(linux(false)));
    assert!(card_says(&mut hub, "Make a Faraday stick"));
    assert_eq!(next_step(&mut hub), (true, Page::Disks, None));

    // Windows, protected, the BitLocker step not done.
    let mut hub = hub_with(stick_attached());
    hub.hub_event(HubEvent::Computer(windows(false)));
    hub.hub_event(HubEvent::BitLocker(Some(BitLocker {
        drive: "C:".into(),
        protection: Protection::On,
        marker: None,
    })));
    assert!(card_says(&mut hub, "Do the BitLocker step"));
    assert_eq!(next_step(&mut hub), (true, Page::BitLocker, None));

    // Secure Boot on: the firmware's settings, after the sheet.
    let mut hub = hub_with(stick_attached());
    assert!(card_says(&mut hub, "Restart into firmware settings"));
    assert_eq!(next_step(&mut hub), (true, Page::Home, FIRMWARE));

    // Secure Boot not known: the same.
    let mut c = linux(true);
    c.secure_boot = None;
    hub.hub_event(HubEvent::Computer(c));
    assert!(card_says(&mut hub, "Restart into firmware settings"));
    assert_eq!(next_step(&mut hub), (true, Page::Home, FIRMWARE));

    // Secure Boot off, a stick attached, Linux: the stick.
    hub.hub_event(HubEvent::Computer(linux(false)));
    assert!(card_says(&mut hub, "Restart from the stick"));
    assert_eq!(next_step(&mut hub), (true, Page::Home, STICK));

    // And Windows with no encrypted drive.
    hub.hub_event(HubEvent::Computer(windows(false)));
    hub.hub_event(HubEvent::BitLocker(Some(BitLocker {
        drive: "C:".into(),
        protection: Protection::None,
        marker: None,
    })));
    assert!(card_says(&mut hub, "Restart from the stick"));
    assert_eq!(next_step(&mut hub), (true, Page::Home, STICK));
}

#[test]
fn homes_next_step_moves_on_once_the_recovery_key_is_in_hand() {
    let mut hub = hub_with(stick_attached());
    hub.hub_event(HubEvent::Computer(windows(true)));
    hub.hub_event(HubEvent::BitLocker(Some(BitLocker {
        drive: "C:".into(),
        protection: Protection::On,
        marker: None,
    })));
    assert!(shows(&mut hub, "Protected"));
    assert!(card_says(&mut hub, "Do the BitLocker step"));
    hub.press(HubAction::Next);
    assert_eq!(hub.page, Page::BitLocker);
    hub.press(HubAction::KeyInHand);
    hub.press(HubAction::Home);
    // The item reads as done, and the card leads to the firmware.
    assert!(shows(&mut hub, "Recovery key kept"));
    assert!(card_says(&mut hub, "Restart into firmware settings"));
    assert_eq!(next_step(&mut hub), (true, Page::Home, FIRMWARE));
    // The Start page offers its restart too.
    hub.press(HubAction::Start);
    let _ = hub.drawn_texts();
    assert!(hub.offers_action(HubAction::Restart));
    // Nothing was sent to the computer for it.
    assert!(commands(&mut hub).is_empty());
}

#[test]
fn a_restart_asks_first_and_cancel_sends_nothing() {
    for (computer, label, sent) in [
        (
            linux(false),
            "Restart from the stick",
            HubCommand::RestartFromStick {
                device: "/dev/sdb".into(),
            },
        ),
        (
            linux(true),
            "Restart into firmware settings",
            HubCommand::RestartIntoFirmware,
        ),
    ] {
        let mut hub = hub_with(stick_attached());
        hub.hub_event(HubEvent::Computer(computer));
        tap(&mut hub, HubAction::Next);
        // The sheet, and nothing sent yet.
        assert!(shows(
            &mut hub,
            "This restarts the computer now. Save your work first."
        ));
        assert!(shows(&mut hub, label));
        assert!(shows(&mut hub, "https://faradayoffline.app/start/linux"));
        assert!(commands(&mut hub).is_empty());
        tap(&mut hub, HubAction::Cancel);
        assert!(hub.sheet.is_none());
        assert!(commands(&mut hub).is_empty());
        // Restart sends exactly one.
        tap(&mut hub, HubAction::Next);
        tap(&mut hub, HubAction::RestartNow);
        assert_eq!(commands(&mut hub), vec![sent]);
        assert!(hub.sheet.is_none());
    }
}

#[test]
fn a_restart_from_the_stick_that_did_not_happen_offers_the_firmware_with_why() {
    for (why, says) in [
        (
            Reason::Missing("efibootmgr".into()),
            "efibootmgr is not installed",
        ),
        (Reason::Missing("pkexec".into()), "pkexec is not installed"),
        (
            Reason::Declined,
            "Administrator permission was declined. Nothing was changed.",
        ),
        (
            Reason::BootEntry("Could not set BootNext".into()),
            "efibootmgr failed: Could not set BootNext",
        ),
    ] {
        let mut hub = hub_with(stick_attached());
        hub.hub_event(HubEvent::Computer(linux(false)));
        tap(&mut hub, HubAction::Next);
        tap(&mut hub, HubAction::RestartNow);
        let _ = commands(&mut hub);
        hub.hub_event(HubEvent::StickRestartFailed(why));
        assert!(card_says(&mut hub, "Restart into firmware settings"));
        assert!(reads(
            &mut hub,
            &format!("Restart from the stick did not happen: {says}")
        ));
        tap(&mut hub, HubAction::Next);
        tap(&mut hub, HubAction::RestartNow);
        assert_eq!(commands(&mut hub), vec![HubCommand::RestartIntoFirmware]);
    }
}

#[test]
fn uninstall_says_when_the_firmwares_entry_remains() {
    let installed = || {
        Some(faraday_hub::api::Installed {
            version: "0.2.0".into(),
            items: vec![(
                faraday_hub::api::InstallItem::App,
                "~/.local/bin/faraday-hub".into(),
            )],
        })
    };
    // Removed, or there was none: Home.
    let mut hub = hub_with(Vec::new());
    hub.hub_event(HubEvent::Installed(installed()));
    hub.press(HubAction::Installed);
    tap(&mut hub, HubAction::Uninstall);
    tap(&mut hub, HubAction::UninstallNow);
    assert_eq!(commands(&mut hub), vec![HubCommand::Uninstall]);
    hub.hub_event(HubEvent::UninstallDone(Ok(())));
    assert_eq!(hub.page, Page::Home);

    // Declined: uninstalled, and the entry remains, said on Installed.
    let mut hub = hub_with(Vec::new());
    hub.hub_event(HubEvent::Installed(installed()));
    hub.press(HubAction::Installed);
    tap(&mut hub, HubAction::Uninstall);
    tap(&mut hub, HubAction::UninstallNow);
    hub.hub_event(HubEvent::BootEntryKept(Reason::Declined));
    hub.hub_event(HubEvent::UninstallDone(Ok(())));
    assert_eq!(hub.page, Page::Installed);
    assert!(hub.installed.is_none());
    assert!(reads(
        &mut hub,
        "The firmware's Faraday stick entry remains: Administrator permission was declined."
    ));
}

#[test]
fn the_stick_and_the_hub_show_the_same_address_for_a_computer() {
    for (maker, family, model) in [
        ("Dell Inc.", "Example", "Example 14"),
        ("LENOVO", "ThinkPad Example 14", "21XX0000"),
        ("Example Computers", "Example", "Example 14"),
    ] {
        let c = Computer {
            os: Os::Windows,
            maker: maker.into(),
            family: family.into(),
            model: model.into(),
            ..windows(false)
        };
        let mut app = faraday_core::Faraday::new();
        app.secure_boot = faraday_core::SecureBootAtStart::Off;
        app.maker = maker.into();
        app.maker_family = family.into();
        app.maker_model = model.into();
        assert_eq!(
            app.secure_boot_guide(),
            Some(LearnId::for_computer(&c).address()),
            "{maker}"
        );
    }
}

fn sized(width: u16, height: u16, dpi: u16) -> Hub {
    let mut hub = hub_with(Vec::new());
    hub.event(display(width, height, dpi));
    hub
}

#[test]
fn the_hub_draws_at_the_desktops_density_in_any_window() {
    let half = sized(1600, 1800, 320);
    let full = sized(3200, 1800, 320);
    let small = sized(1100, 760, 160);
    assert_eq!(half.scale(), 2.0);
    assert_eq!(full.scale(), 2.0);
    assert_eq!(small.scale(), 1.0);
}

#[test]
fn a_narrow_window_is_one_column_and_still_shows_home() {
    let mut hub = sized(640, 400, 160);
    assert_eq!(hub.scale(), 1.0);
    assert!(shows(&mut hub, "Checklist"));
    let mut half = sized(1600, 1800, 320);
    assert!(shows(&mut half, "Checklist"));
}

#[test]
fn transfer_in_a_half_screen_window_is_one_column() {
    let mut hub = sized(1600, 1800, 320);
    hub.press(HubAction::Transfer);
    assert!(hub.showing_held());
    assert!(hub.held().is_compact());
    assert!(shows(&mut hub, "Transfer"));
    hub.event(display(3200, 1800, 320));
    assert!(!hub.held().is_compact());
}
