//! What a person sees and can do in the Starter (`docs/STARTER.md` §11):
//! the shell's reports in, the frame's words and the Starter's commands out.

use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};
use faraday_starter::api::{
    Bus, Computer, DevImage, Disk, FaradayStick, InUse, Mode, Os, Payload, PayloadFile, Reason,
    Source, StarterCommand, StarterEvent, Volume,
};
use faraday_starter::decide::Target;
use faraday_starter::{LearnId, Page, RestartTo, Sheet, Starter, StarterAction};

const GB: u64 = 1_000_000_000;

pub(super) fn display(width: u16, height: u16, dpi: u16) -> Event {
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

pub(super) fn payload(stick_build: &str) -> Payload {
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
                "faraday-stick-0.2.0-x86_64-uefi-strict.img.xz",
                "faraday-stick-0.2.0-x86_64-uefi-strict.img",
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
        ],
    }
}

pub(super) fn disk(id: &str, model: &str, size: u64) -> Disk {
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

pub(super) fn faraday_stick(id: &str, model: &str, size: u64, release: Option<&str>) -> Disk {
    Disk {
        faraday: Some(FaradayStick {
            release: release.map(str::to_string),
            read: true,
        }),
        volumes: Vec::new(),
        ..disk(id, model, size)
    }
}

/// A disk with its own vendor and model strings, as an INQUIRY or USB
/// descriptor answers them (space-padded).
fn padded_disk(id: &str, vendor: &str, model: &str, size: u64) -> Disk {
    Disk {
        vendor: vendor.into(),
        ..disk(id, model, size)
    }
}

/// A Faraday stick with its own vendor and model strings, space-padded.
fn padded_faraday_stick(id: &str, vendor: &str, model: &str, size: u64) -> Disk {
    Disk {
        vendor: vendor.into(),
        ..faraday_stick(id, model, size, Some("6.6.84-faraday-0.1.0"))
    }
}

pub(super) fn linux(secure_boot: bool) -> Computer {
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

pub(super) fn windows(secure_boot: bool) -> Computer {
    Computer {
        os: Os::Windows,
        ..linux(secure_boot)
    }
}

/// A Dell computer: its boot menu follows the default pattern.
fn dell() -> Computer {
    Computer {
        maker: "Dell Inc.".into(),
        model: "XPS 13 9310".into(),
        family: "XPS".into(),
        ..linux(true)
    }
}

/// A Lenovo IdeaPad computer: its boot menu follows its own pattern.
fn lenovo_ideapad() -> Computer {
    Computer {
        maker: "LENOVO".into(),
        model: "81CT".into(),
        family: "IdeaPad".into(),
        ..linux(true)
    }
}

fn starter_with(disks: Vec<Disk>) -> Starter {
    let mut starter = Starter::new();
    starter.event(display(1280, 800, 160));
    starter.starter_event(StarterEvent::Computer(linux(true)));
    starter.starter_event(StarterEvent::Payload(Some(payload("0.2.0"))));
    starter.starter_event(StarterEvent::Disks(disks));
    starter
}

fn texts(starter: &mut Starter) -> Vec<String> {
    starter.drawn_texts()
}

fn shows(starter: &mut Starter, s: &str) -> bool {
    texts(starter).iter().any(|t| t.contains(s))
}

fn position(starter: &mut Starter, s: &str) -> Option<usize> {
    texts(starter).iter().position(|t| t.contains(s))
}

fn number(starter: &Starter, id: &str) -> StarterAction {
    StarterAction::Disk(starter.disk_number(id))
}

fn tap(starter: &mut Starter, action: StarterAction) {
    let _ = starter.drawn_texts();
    let (x, y) = starter.where_offered(action).expect("offered");
    starter.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    starter.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
}

/// A hold on screen, `action`, held past its time and let go (F136).
pub(super) fn hold(starter: &mut Starter, action: StarterAction) {
    let _ = starter.drawn_texts();
    let (x, y) = starter.where_offered(action).expect("offered");
    let t0 = 1_000_000;
    starter.event(Event::Tick { now_ms: t0 });
    starter.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    starter.event(Event::Tick {
        now_ms: t0 + faraday_draw::tokens::HOLD_MS + 100,
    });
    starter.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
}

fn commands(starter: &mut Starter) -> Vec<StarterCommand> {
    std::iter::from_fn(|| starter.next_command()).collect()
}

/// To Confirm for disk `id`, with `mode`.
fn to_confirm(starter: &mut Starter, id: &str, mode: Mode) {
    starter.press(StarterAction::Make(Target::Stick));
    let n = number(starter, id);
    starter.press(n);
    starter.press(StarterAction::Mode(mode));
    starter.press(StarterAction::Continue);
    assert_eq!(starter.page, Page::Confirm);
}

#[test]
fn faraday_sticks_come_first_then_the_others_by_size() {
    let mut starter = starter_with(vec![
        disk("sdb", "Big", 64 * GB),
        disk("sdc", "Small", 8 * GB),
        faraday_stick("sdd", "Old", 32 * GB, Some("6.6.84-faraday-0.1.0")),
        disk("sde", "Middle", 16 * GB),
    ]);
    starter.press(StarterAction::Make(Target::Stick));
    let order: Vec<usize> = ["Old", "Small", "Middle", "Big"]
        .iter()
        .map(|m| position(&mut starter, &format!("Maker {m}")).expect("listed"))
        .collect();
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
    assert!(shows(&mut starter, "Faraday stick · 0.1.0"));
    assert!(shows(&mut starter, "SmallVOL, files on it"));
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
    let mut starter = starter_with(vec![home, swap, tiny, nvme, disk("sde", "Fine", 16 * GB)]);
    starter.press(StarterAction::Make(Target::Stick));
    assert!(shows(
        &mut starter,
        "Used by this computer: mounted at /home"
    ));
    assert!(shows(&mut starter, "Used by this computer: swap"));
    assert!(shows(&mut starter, "Smaller than the image: needs 4.0 GB"));
    assert!(!shows(&mut starter, "Maker Internal"));
    for id in ["sdb", "sdc", "sdd"] {
        let n = number(&starter, id);
        assert!(!starter.offers_action(n), "{id} offered");
        starter.press(n);
        assert_eq!(starter.page, Page::Disks, "{id} could be chosen");
    }
    let fine = number(&starter, "sde");
    assert!(starter.offers_action(fine));
}

#[test]
fn no_disk_is_chosen_until_one_is_tapped() {
    let mut starter = starter_with(vec![disk("sdb", "Only", 16 * GB)]);
    tap(&mut starter, StarterAction::Next);
    assert_eq!(starter.page, Page::Disks);
    // One disk attached, and still nothing chosen: Which stick? stays,
    // with nothing to go on with.
    assert!(!starter.offers_action(StarterAction::Continue));
    assert!(!starter.offers_action(StarterAction::Write));
    starter.starter_event(StarterEvent::Disks(vec![disk("sdb", "Only", 16 * GB)]));
    assert_eq!(starter.page, Page::Disks);
    assert!(commands(&mut starter).is_empty());
    let n = number(&starter, "sdb");
    tap(&mut starter, n);
    assert_eq!(starter.page, Page::Mode);
    // Its only choice, to write the whole stick, starts ticked.
    starter.press(StarterAction::Continue);
    assert_eq!(starter.page, Page::Confirm);
}

#[test]
fn the_list_follows_disks_plugged_in_and_pulled_out() {
    let mut starter = starter_with(Vec::new());
    starter.press(StarterAction::Make(Target::Stick));
    assert!(shows(&mut starter, "No USB stick attached"));
    starter.starter_event(StarterEvent::Disks(vec![disk("sdb", "New", 16 * GB)]));
    assert!(shows(&mut starter, "Maker New"));
    starter.starter_event(StarterEvent::Disks(Vec::new()));
    assert!(!shows(&mut starter, "Maker New"));
    assert!(shows(&mut starter, "No USB stick attached"));
}

#[test]
fn confirm_writes_the_disk_confirmed_with_its_serial_and_size() {
    let mut starter = starter_with(vec![disk("sdb", "Stick", 16 * GB)]);
    to_confirm(&mut starter, "sdb", Mode::Whole);
    for s in [
        "Maker Stick",
        "/dev/sdb",
        "SN-sdb",
        "16 GB",
        "Erase and write",
    ] {
        assert!(shows(&mut starter, s), "{s}");
    }
    starter.press(StarterAction::Write);
    let c = commands(&mut starter);
    let [StarterCommand::Write(order)] = c.as_slice() else {
        panic!("{c:?}");
    };
    assert_eq!(order.disk, "sdb");
    assert_eq!(order.serial, "SN-sdb");
    assert_eq!(order.size, 16 * GB);
    assert_eq!(order.mode, Mode::Whole);
    assert!(
        matches!(&order.source, Source::Payload { name, .. } if name.starts_with("faraday-stick-"))
    );
    assert_eq!(starter.page, Page::Writing);
}

#[test]
fn a_disk_that_is_not_the_confirmed_one_is_refused() {
    // Another stick in its place: same handle, another serial.
    let mut starter = starter_with(vec![disk("sdb", "Stick", 16 * GB)]);
    to_confirm(&mut starter, "sdb", Mode::Whole);
    let mut other = disk("sdb", "Stick", 16 * GB);
    other.serial = "SN-OTHER".into();
    starter.starter_event(StarterEvent::Disks(vec![other]));
    assert!(shows(&mut starter, "Not the stick confirmed"));
    starter.press(StarterAction::Write);
    assert!(commands(&mut starter).is_empty());
    assert_eq!(starter.page, Page::Confirm);

    // The same serial, another size.
    let mut starter = starter_with(vec![disk("sdb", "Stick", 16 * GB)]);
    to_confirm(&mut starter, "sdb", Mode::Whole);
    starter.starter_event(StarterEvent::Disks(vec![disk("sdb", "Stick", 32 * GB)]));
    starter.press(StarterAction::Write);
    assert!(commands(&mut starter).is_empty());

    // Pulled out.
    let mut starter = starter_with(vec![disk("sdb", "Stick", 16 * GB)]);
    to_confirm(&mut starter, "sdb", Mode::Whole);
    starter.starter_event(StarterEvent::Disks(Vec::new()));
    starter.press(StarterAction::Write);
    assert!(commands(&mut starter).is_empty());
    assert!(shows(&mut starter, "Not the stick confirmed"));
}

#[test]
fn choose_again_leaves_a_confirm_whose_disk_changed() {
    let mut starter = starter_with(vec![disk("sdb", "Stick", 16 * GB)]);
    to_confirm(&mut starter, "sdb", Mode::Whole);
    starter.starter_event(StarterEvent::Disks(Vec::new()));
    assert!(shows(&mut starter, "Choose again"));
    starter.press(StarterAction::Again);
    assert_eq!(starter.page, Page::Disks);
}

#[test]
fn keep_data_is_offered_only_on_a_faraday_stick() {
    let mut starter = starter_with(vec![disk("sdb", "Plain", 16 * GB)]);
    starter.press(StarterAction::Make(Target::Stick));
    let n = number(&starter, "sdb");
    starter.press(n);
    assert!(shows(&mut starter, "New Faraday stick"));
    assert!(!shows(&mut starter, "Replace Faraday, keep data"));
    // Keep data cannot be ticked; the whole stick stays the choice.
    starter.press(StarterAction::Mode(Mode::KeepData));
    starter.press(StarterAction::Continue);
    assert_eq!(starter.page, Page::Confirm);
    starter.press(StarterAction::Write);
    let c = commands(&mut starter);
    assert!(
        matches!(c.as_slice(), [StarterCommand::Write(o)] if o.mode == Mode::Whole),
        "{c:?}"
    );

    let mut starter = starter_with(vec![faraday_stick(
        "sdb",
        "Mine",
        16 * GB,
        Some("6.6.84-faraday-0.1.0"),
    )]);
    to_confirm(&mut starter, "sdb", Mode::KeepData);
    assert!(shows(&mut starter, "Write"));
    starter.press(StarterAction::Write);
    let c = commands(&mut starter);
    assert!(
        matches!(c.as_slice(), [StarterCommand::Write(o)] if o.mode == Mode::KeepData),
        "{c:?}"
    );
}

#[test]
fn keep_data_refuses_the_same_build() {
    let mut starter = starter_with(vec![faraday_stick(
        "sdb",
        "Mine",
        16 * GB,
        Some("6.6.84-faraday-0.2.0"),
    )]);
    starter.press(StarterAction::Make(Target::Stick));
    let n = number(&starter, "sdb");
    starter.press(n);
    assert!(shows(&mut starter, "Already this build"));
    assert!(!starter.offers_action(StarterAction::Mode(Mode::KeepData)));
    starter.press(StarterAction::Mode(Mode::KeepData));
    starter.press(StarterAction::Continue);
    assert_eq!(starter.page, Page::Mode);
    // The whole stick can still be written.
    assert!(starter.offers_action(StarterAction::Mode(Mode::Whole)));
}

/// Writing an older release over a newer stick is a hold that names
/// going back to an older release (F136): a tap writes nothing.
#[test]
fn keep_data_writes_an_older_faraday_over_a_newer_stick_only_after_the_hold() {
    let mut starter = starter_with(vec![faraday_stick(
        "sdb",
        "Mine",
        16 * GB,
        Some("6.6.84-faraday-0.3.0"),
    )]);
    to_confirm(&mut starter, "sdb", Mode::KeepData);
    assert!(shows(
        &mut starter,
        "This stick holds a newer Faraday: 0.3.0"
    ));
    assert!(shows(&mut starter, "Only to go back to an older release."));
    assert!(shows(&mut starter, "Hold to write the older release"));
    let _ = commands(&mut starter);
    tap(&mut starter, StarterAction::Write);
    assert!(commands(&mut starter).is_empty());
    assert_eq!(starter.page, Page::Confirm);
    hold(&mut starter, StarterAction::Write);
    assert_eq!(commands(&mut starter).len(), 1);

    // An older stick: no warning.
    let mut starter = starter_with(vec![faraday_stick(
        "sdb",
        "Mine",
        16 * GB,
        Some("6.6.84-faraday-0.1.0"),
    )]);
    to_confirm(&mut starter, "sdb", Mode::KeepData);
    assert!(!shows(&mut starter, "newer Faraday"));
}

#[test]
fn a_developers_image_says_it_is_not_the_apps_own_and_shows_its_sum() {
    let mut starter = starter_with(vec![disk("sdb", "Stick", 16 * GB)]);
    starter.starter_event(StarterEvent::DevImage(DevImage {
        path: "out/stick/faraday-x86_64-uefi-strict-dev.img".into(),
        sha256: "d".repeat(64),
        size: 4 * GB,
        boot_only: false,
        build_id: Some("0.2.0+4d0680b1a2b3.dev".into()),
    }));
    to_confirm(&mut starter, "sdb", Mode::Whole);
    assert!(shows(&mut starter, "Not Faraday Starter's own image"));
    assert!(shows(&mut starter, "dddd dddd"));
    assert!(shows(&mut starter, "0.2.0 dev (4d0680b1a2b3)"));
    starter.press(StarterAction::Write);
    let c = commands(&mut starter);
    assert!(
        matches!(c.as_slice(), [StarterCommand::Write(o)] if matches!(&o.source, Source::Image { path, .. } if path.ends_with("-dev.img"))),
        "{c:?}"
    );
}

#[test]
fn writing_shows_progress_then_done_or_the_writers_reason() {
    let mut starter = starter_with(vec![disk("sdb", "Stick", 16 * GB)]);
    to_confirm(&mut starter, "sdb", Mode::Whole);
    starter.press(StarterAction::Write);
    starter.starter_event(StarterEvent::WriteProgress {
        stage: faraday_starter::api::WriteStage::Writing,
        done: GB,
        total: 4 * GB,
    });
    assert!(shows(&mut starter, "Written 1.0 GB of 4.0 GB"));
    assert_eq!(starter.drawn_progress(), vec![Some(0.25)]);
    starter.starter_event(StarterEvent::WriteProgress {
        stage: faraday_starter::api::WriteStage::ReadingBack,
        done: 2 * GB,
        total: 4 * GB,
    });
    assert!(shows(&mut starter, "Reading back"));
    assert_eq!(starter.drawn_progress(), vec![Some(0.5)]);
    starter.starter_event(StarterEvent::Written(Err(Reason::BackDiffers {
        device: "/dev/sdb".into(),
        at: 12,
    })));
    assert!(shows(
        &mut starter,
        "differs from what was written at byte 12"
    ));
    assert!(!shows(&mut starter, "pull it out"));

    let mut starter = starter_with(vec![disk("sdb", "Stick", 16 * GB)]);
    to_confirm(&mut starter, "sdb", Mode::Whole);
    starter.press(StarterAction::Write);
    starter.starter_event(StarterEvent::Written(Ok(())));
    assert!(shows(
        &mut starter,
        "Keep the stick in to start this computer from it."
    ));
    starter.press(StarterAction::Start);
    assert_eq!(starter.page, Page::Start);
}

#[test]
fn a_write_result_or_progress_with_no_write_asked_changes_nothing() {
    let mut starter = starter_with(vec![disk("sdb", "Stick", 16 * GB)]);
    to_confirm(&mut starter, "sdb", Mode::Whole);
    // Confirm is shown and no write has been asked.
    starter.starter_event(StarterEvent::Written(Ok(())));
    starter.starter_event(StarterEvent::WriteProgress {
        stage: faraday_starter::api::WriteStage::Writing,
        done: GB,
        total: 4 * GB,
    });
    assert_eq!(starter.page, Page::Confirm);
    assert!(starter.drawn_progress().is_empty());
    // The write asked ends; a late progress does not bring it back.
    starter.press(StarterAction::Write);
    starter.starter_event(StarterEvent::Written(Ok(())));
    assert_eq!(starter.page, Page::Done);
    starter.starter_event(StarterEvent::WriteProgress {
        stage: faraday_starter::api::WriteStage::ReadingBack,
        done: GB,
        total: 4 * GB,
    });
    assert!(starter.drawn_progress().is_empty());
    assert!(shows(
        &mut starter,
        "Keep the stick in to start this computer from it."
    ));
}

#[test]
fn done_shows_the_expected_boot_menu_entry_by_maker() {
    // Dell: the default pattern.
    let mut starter = starter_with(vec![padded_disk(
        "sdb",
        "SanDisk  ",
        "  Cruzer Blade ",
        16 * GB,
    )]);
    starter.starter_event(StarterEvent::Computer(dell()));
    to_confirm(&mut starter, "sdb", Mode::Whole);
    starter.press(StarterAction::Write);
    starter.starter_event(StarterEvent::Written(Ok(())));
    assert!(shows(
        &mut starter,
        "something like UEFI: SanDisk Cruzer Blade, Partition 1"
    ));

    // Lenovo IdeaPad: its own pattern.
    let mut starter = starter_with(vec![padded_disk(
        "sdb",
        "SanDisk  ",
        "  Cruzer Blade ",
        16 * GB,
    )]);
    starter.starter_event(StarterEvent::Computer(lenovo_ideapad()));
    to_confirm(&mut starter, "sdb", Mode::Whole);
    starter.press(StarterAction::Write);
    starter.starter_event(StarterEvent::Written(Ok(())));
    assert!(shows(
        &mut starter,
        "something like EFI USB Device (SanDisk Cruzer Blade)"
    ));
}

#[test]
fn done_shows_no_boot_menu_row_when_the_stick_reports_neither() {
    let mut starter = starter_with(vec![padded_disk("sdb", "", "", 16 * GB)]);
    starter.starter_event(StarterEvent::Computer(dell()));
    to_confirm(&mut starter, "sdb", Mode::Whole);
    starter.press(StarterAction::Write);
    starter.starter_event(StarterEvent::Written(Ok(())));
    assert!(!shows(&mut starter, "In the boot menu"));
}

#[test]
fn start_shows_the_boot_menu_entry_with_a_stick_attached() {
    let mut starter = starter_with(vec![padded_faraday_stick(
        "sdb",
        "SanDisk  ",
        "  Cruzer Blade ",
        16 * GB,
    )]);
    starter.starter_event(StarterEvent::Computer(dell()));
    starter.press(StarterAction::Start);
    assert!(shows(
        &mut starter,
        "something like UEFI: SanDisk Cruzer Blade, Partition 1"
    ));
}

#[test]
fn a_pi_card_is_offered_sd_card_readers_and_no_keep_data() {
    let mut reader = disk("mmcblk0", "Card", 32 * GB);
    reader.bus = Bus::SdReader;
    let mut starter = starter_with(vec![reader.clone()]);
    starter.press(StarterAction::Make(Target::Stick));
    assert!(!shows(&mut starter, "Maker Card"));
    starter.press(StarterAction::Home);
    starter.press(StarterAction::Make(Target::Pi));
    assert!(shows(&mut starter, "Raspberry Pi 3B"));
    assert!(shows(&mut starter, "Maker Card"));
    let mut fs = faraday_stick("mmcblk0", "Card", 32 * GB, Some("6.6.84-faraday-0.1.0"));
    fs.bus = Bus::SdReader;
    starter.starter_event(StarterEvent::Disks(vec![fs]));
    let n = number(&starter, "mmcblk0");
    starter.press(n);
    assert!(shows(&mut starter, "New Faraday card"));
    assert!(!shows(&mut starter, "keep data"));
}

/// Home's card button: whether it can be pressed, and the page and
/// sheet a press on it opens (the sheet is closed again with Cancel).
fn next_step(starter: &mut Starter) -> (bool, Page, Option<Sheet>) {
    let _ = starter.drawn_texts();
    let offered = starter.offers_action(StarterAction::Next);
    starter.press(StarterAction::Next);
    let opened = (offered, starter.page, starter.sheet);
    starter.press(StarterAction::Cancel);
    starter.press(StarterAction::Home);
    opened
}

const FIRMWARE: Option<Sheet> = Some(Sheet::Restart(RestartTo::Firmware));
const STICK: Option<Sheet> = Some(Sheet::Restart(RestartTo::Stick));

#[test]
fn home_leads_with_the_next_step_for_each_state() {
    // No stick.
    let mut starter = starter_with(Vec::new());
    assert!(card_says(&mut starter, "Make a Faraday stick"));
    assert!(shows(&mut starter, "No Faraday stick attached"));
    assert_eq!(next_step(&mut starter), (true, Page::Disks, None));

    // A stick attached.
    let mut starter = starter_with(vec![faraday_stick(
        "sdb",
        "Mine",
        16 * GB,
        Some("6.6.84-faraday-0.2.0"),
    )]);
    assert!(shows(&mut starter, "Faraday stick attached · 0.2.0"));
    assert_eq!(next_step(&mut starter), (true, Page::Home, FIRMWARE));

    // Windows with Secure Boot on: the firmware.
    starter.starter_event(StarterEvent::Computer(windows(true)));
    assert!(!shows(&mut starter, "Turn Secure Boot back on"));
    assert_eq!(next_step(&mut starter), (true, Page::Home, FIRMWARE));

    // Windows with Secure Boot off and no stick: Turn Secure Boot back on
    // is listed, and never the next step.
    starter.starter_event(StarterEvent::Disks(Vec::new()));
    starter.starter_event(StarterEvent::Computer(windows(false)));
    assert!(shows(&mut starter, "Turn Secure Boot back on"));
    assert_eq!(next_step(&mut starter), (true, Page::Disks, None));
}

#[test]
fn start_offers_the_firmware_buttons_on_windows() {
    let mut starter = starter_with(Vec::new());
    starter.starter_event(StarterEvent::Computer(windows(true)));
    starter.press(StarterAction::Start);
    // Shown, the page reads BitLocker (F49), which changes nothing.
    assert_eq!(commands(&mut starter), [StarterCommand::ReadBitLocker]);
    assert!(shows(&mut starter, "Restart into the BIOS"));
    let _ = starter.drawn_texts();
    assert!(starter.offers_action(StarterAction::Restart));
    tap(&mut starter, StarterAction::Restart);
    assert!(shows(
        &mut starter,
        "This restarts the computer now. Save your work first."
    ));
    // The guide's address is on the confirmation as a code.
    assert!(shows(
        &mut starter,
        "https://faradayoffline.app/start/windows"
    ));
    tap(&mut starter, StarterAction::RestartNow);
    assert_eq!(
        commands(&mut starter),
        vec![StarterCommand::RestartIntoFirmware]
    );
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
    let mut starter = starter_with(Vec::new());
    let mut c = windows(true);
    c.maker = "Dell Inc.".into();
    starter.starter_event(StarterEvent::Computer(c));
    starter.press(StarterAction::Start);
    assert!(shows(
        &mut starter,
        "Starting a Dell computer from the stick"
    ));
    assert!(starter.offers_action(StarterAction::LearnPage(LearnId::Maker("maker-dell"))));
    tap(&mut starter, StarterAction::Restart);
    assert!(shows(
        &mut starter,
        "https://faradayoffline.app/start/maker-dell"
    ));
}

#[test]
fn about_lists_the_payload() {
    let mut starter = starter_with(Vec::new());
    starter.press(StarterAction::About);
    for s in [
        "faraday-stick-0.2.0-x86_64-uefi-strict.img.xz",
        "faraday-pi-0.2.0-pi3-waveshare-28dpi.img.xz",
        "aaaa aaaa",
        "AAAA AAAA",
        "SHA-256 of faraday-stick-0.2.0-x86_64-uefi-strict.img",
        "D1AF A712 ABEF 9010 9A1B  587F F153 6BE0 7E23 4CFD",
    ] {
        assert!(seen_scrolling(&mut starter, s), "{s}");
    }
}

/// Whether `s` is drawn anywhere on the page, scrolled from its top to
/// its end.
fn seen_scrolling(starter: &mut Starter, s: &str) -> bool {
    for _ in 0..12 {
        starter.event(Event::Wheel {
            x: 0,
            y: 0,
            dy: -2000,
        });
    }
    for _ in 0..40 {
        if shows(starter, s) {
            return true;
        }
        starter.event(Event::Wheel {
            x: 0,
            y: 0,
            dy: 200,
        });
    }
    false
}

#[test]
fn home_offers_faraday_demo_with_this_starters_version_and_says_why_it_did_not_open() {
    let mut starter = starter_with(Vec::new());
    assert!(seen_scrolling(&mut starter, "Open Faraday Demo"));
    assert!(seen_scrolling(
        &mut starter,
        &format!("Version {}", faraday_starter::VERSION)
    ));
    starter.press(StarterAction::OpenDesktop);
    assert_eq!(commands(&mut starter), vec![StarterCommand::OpenDemo]);
    // An open that failed says why on the card.
    starter.starter_event(StarterEvent::DesktopStarted(Err(Reason::File {
        path: "~/faraday-sticks".into(),
        detail: "Permission denied".into(),
    })));
    assert!(seen_scrolling(&mut starter, "Permission denied"));
    starter.starter_event(StarterEvent::DesktopStarted(Ok(())));
    assert!(!seen_scrolling(&mut starter, "Permission denied"));
}

#[test]
fn install_is_one_press_with_no_check_box() {
    let mut starter = starter_with(Vec::new());
    starter.press(StarterAction::Install);
    assert!(!shows(&mut starter, "Also install"));
    starter.press(StarterAction::InstallNow);
    assert_eq!(commands(&mut starter), vec![StarterCommand::Install]);
    starter.starter_event(StarterEvent::InstallDone(Ok(
        "~/.local/bin/faraday-starter".into(),
    )));
    assert!(shows(
        &mut starter,
        "Installed in ~/.local/bin/faraday-starter"
    ));
}

#[test]
fn transfer_is_the_desktop_apps_and_back_returns_to_home() {
    let mut starter = starter_with(Vec::new());
    starter.press(StarterAction::Transfer);
    assert!(starter.showing_held());
    assert!(shows(&mut starter, "Transfer"));
    assert!(!shows(&mut starter, "Wallets"));
    starter.event(Event::Back);
    assert!(!starter.showing_held());
    assert_eq!(starter.page, Page::Home);
    assert!(shows(&mut starter, "Checklist"));

    // Learn: the sheet with the guides, closed back to Home.
    starter.press(StarterAction::Learn);
    assert!(starter.showing_held());
    assert!(shows(
        &mut starter,
        "Starting a Windows computer from the stick"
    ));
    starter.event(Event::Key(faraday_shell_api::Key::Escape));
    assert!(!starter.showing_held());
}

#[test]
fn checklist_items_that_do_not_apply_are_not_shown() {
    let mut starter = starter_with(Vec::new());
    // Linux: no Turn Secure Boot back on with Secure Boot on.
    assert!(!shows(&mut starter, "Turn Secure Boot back on"));
    // An ARM computer: only Make a stick, and the line that says why.
    let mut arm = windows(true);
    arm.arm = true;
    starter.starter_event(StarterEvent::Computer(arm));
    assert!(shows(
        &mut starter,
        "This computer cannot start a Faraday stick"
    ));
    assert!(!shows(&mut starter, "Start this computer from the stick"));
}

fn stick_attached() -> Vec<Disk> {
    vec![faraday_stick(
        "sdb",
        "Mine",
        16 * GB,
        Some("6.6.84-faraday-0.2.0"),
    )]
}

fn reads(starter: &mut Starter, s: &str) -> bool {
    texts(starter).join(" ").contains(s)
}

/// Whether the card's button carries `label`: the card draws its
/// checklist, then its button, so a label that is also a checklist item
/// is drawn twice below the checklist's label.
fn card_says(starter: &mut Starter, label: &str) -> bool {
    let t = texts(starter);
    let Some(from) = t.iter().position(|s| s.contains("Checklist")) else {
        return false;
    };
    let items = ["Make a Faraday stick", "Start this computer from the stick"];
    let need = 1 + usize::from(items.contains(&label));
    t[from..].iter().filter(|s| s.as_str() == label).count() >= need
}

#[test]
fn the_cards_button_follows_this_computers_state() {
    // No stick: Make a Faraday stick, no sheet.
    let mut starter = starter_with(Vec::new());
    starter.starter_event(StarterEvent::Computer(linux(false)));
    assert!(card_says(&mut starter, "Make a Faraday stick"));
    assert_eq!(next_step(&mut starter), (true, Page::Disks, None));

    // Windows with Secure Boot off.
    let mut starter = starter_with(stick_attached());
    starter.starter_event(StarterEvent::Computer(windows(false)));
    assert!(card_says(&mut starter, "Restart from the stick"));
    assert_eq!(next_step(&mut starter), (true, Page::Home, STICK));

    // Secure Boot on: the firmware's settings, after the sheet.
    let mut starter = starter_with(stick_attached());
    assert!(card_says(&mut starter, "Restart into the BIOS"));
    assert_eq!(next_step(&mut starter), (true, Page::Home, FIRMWARE));

    // Secure Boot not known: the same.
    let mut c = linux(true);
    c.secure_boot = None;
    starter.starter_event(StarterEvent::Computer(c));
    assert!(card_says(&mut starter, "Restart into the BIOS"));
    assert_eq!(next_step(&mut starter), (true, Page::Home, FIRMWARE));

    // Secure Boot off, a stick attached, Linux: the stick.
    starter.starter_event(StarterEvent::Computer(linux(false)));
    assert!(card_says(&mut starter, "Restart from the stick"));
    assert_eq!(next_step(&mut starter), (true, Page::Home, STICK));

    // And Windows with Secure Boot off.
    starter.starter_event(StarterEvent::Computer(windows(false)));
    assert!(card_says(&mut starter, "Restart from the stick"));
    assert_eq!(next_step(&mut starter), (true, Page::Home, STICK));
}

#[test]
fn homes_next_step_on_windows_with_secure_boot_on_is_the_firmware() {
    let mut starter = starter_with(stick_attached());
    starter.starter_event(StarterEvent::Computer(windows(true)));
    assert!(card_says(&mut starter, "Restart into the BIOS"));
    assert_eq!(next_step(&mut starter), (true, Page::Home, FIRMWARE));
    // The Start page offers its restart too.
    starter.press(StarterAction::Start);
    let _ = starter.drawn_texts();
    assert!(starter.offers_action(StarterAction::Restart));
    // Nothing was sent to the computer for it: the page's reading of
    // BitLocker (F49) changes nothing.
    assert_eq!(commands(&mut starter), [StarterCommand::ReadBitLocker]);
}

#[test]
fn a_restart_asks_first_and_cancel_sends_nothing() {
    for (computer, label, sent) in [
        (
            linux(false),
            "Restart from the stick",
            StarterCommand::RestartFromStick {
                device: "/dev/sdb".into(),
            },
        ),
        (
            linux(true),
            "Restart into the BIOS",
            StarterCommand::RestartIntoFirmware,
        ),
    ] {
        let mut starter = starter_with(stick_attached());
        starter.starter_event(StarterEvent::Computer(computer));
        tap(&mut starter, StarterAction::Next);
        // The sheet, and nothing sent yet.
        assert!(shows(
            &mut starter,
            "This restarts the computer now. Save your work first."
        ));
        assert!(shows(&mut starter, label));
        assert!(shows(
            &mut starter,
            "https://faradayoffline.app/start/linux"
        ));
        assert!(commands(&mut starter).is_empty());
        tap(&mut starter, StarterAction::Cancel);
        assert!(starter.sheet.is_none());
        assert!(commands(&mut starter).is_empty());
        // Restart sends exactly one.
        tap(&mut starter, StarterAction::Next);
        tap(&mut starter, StarterAction::RestartNow);
        assert_eq!(commands(&mut starter), vec![sent]);
        assert!(starter.sheet.is_none());
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
        let mut starter = starter_with(stick_attached());
        starter.starter_event(StarterEvent::Computer(linux(false)));
        tap(&mut starter, StarterAction::Next);
        tap(&mut starter, StarterAction::RestartNow);
        let _ = commands(&mut starter);
        starter.starter_event(StarterEvent::StickRestartFailed(why));
        assert!(card_says(&mut starter, "Restart into the BIOS"));
        assert!(reads(
            &mut starter,
            &format!("Restart from the stick did not happen: {says}")
        ));
        tap(&mut starter, StarterAction::Next);
        tap(&mut starter, StarterAction::RestartNow);
        assert_eq!(
            commands(&mut starter),
            vec![StarterCommand::RestartIntoFirmware]
        );
    }
}

#[test]
fn uninstall_says_when_the_firmwares_entry_remains() {
    let installed = || {
        Some(faraday_starter::api::Installed {
            version: "0.2.0".into(),
            items: vec![(
                faraday_starter::api::InstallItem::App,
                "~/.local/bin/faraday-starter".into(),
            )],
        })
    };
    // Removed, or there was none: Home.
    let mut starter = starter_with(Vec::new());
    starter.starter_event(StarterEvent::Installed(installed()));
    starter.press(StarterAction::Installed);
    tap(&mut starter, StarterAction::Uninstall);
    tap(&mut starter, StarterAction::UninstallNow);
    assert_eq!(commands(&mut starter), vec![StarterCommand::Uninstall]);
    starter.starter_event(StarterEvent::UninstallDone(Ok(())));
    assert_eq!(starter.page, Page::Home);

    // Declined: uninstalled, and the entry remains, said on Installed.
    let mut starter = starter_with(Vec::new());
    starter.starter_event(StarterEvent::Installed(installed()));
    starter.press(StarterAction::Installed);
    tap(&mut starter, StarterAction::Uninstall);
    tap(&mut starter, StarterAction::UninstallNow);
    starter.starter_event(StarterEvent::BootEntryKept(Reason::Declined));
    starter.starter_event(StarterEvent::UninstallDone(Ok(())));
    assert_eq!(starter.page, Page::Installed);
    assert!(starter.installed.is_none());
    assert!(reads(
        &mut starter,
        "The BIOS's Faraday stick entry remains: Administrator permission was declined."
    ));
}

#[test]
fn the_stick_and_the_starter_show_the_same_address_for_a_computer() {
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

fn sized(width: u16, height: u16, dpi: u16) -> Starter {
    let mut starter = starter_with(Vec::new());
    starter.event(display(width, height, dpi));
    starter
}

#[test]
fn the_starter_draws_at_the_desktops_density_in_any_window() {
    let half = sized(1600, 1800, 320);
    let full = sized(3200, 1800, 320);
    let small = sized(1100, 760, 160);
    assert_eq!(half.scale(), 2.0);
    assert_eq!(full.scale(), 2.0);
    assert_eq!(small.scale(), 1.0);
}

#[test]
fn a_narrow_window_is_one_column_and_still_shows_home() {
    let mut starter = sized(640, 400, 160);
    assert_eq!(starter.scale(), 1.0);
    assert!(shows(&mut starter, "Checklist"));
    let mut half = sized(1600, 1800, 320);
    assert!(shows(&mut half, "Checklist"));
}

#[test]
fn transfer_in_a_half_screen_window_is_one_column() {
    let mut starter = sized(1600, 1800, 320);
    starter.press(StarterAction::Transfer);
    assert!(starter.showing_held());
    assert!(starter.held().is_compact());
    assert!(shows(&mut starter, "Transfer"));
    starter.event(display(3200, 1800, 320));
    assert!(!starter.held().is_compact());
}
