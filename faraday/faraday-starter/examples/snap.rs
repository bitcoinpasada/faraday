//! Renders each Starter page in its states to PNG, for review
//! (`just faraday-starter-snap WxH`).
//!
//! ```text
//! cargo run -p faraday-starter --example snap -- WxH[@DPI] OUT_DIR
//! ```
//!
//! The shell's reports are made up here: a laptop, sticks and disks, a
//! payload. Each picture is `OUT_DIR/<name>.png`, named for the page and
//! its state.

use std::path::Path;
use std::process::ExitCode;

use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};
use faraday_starter::api::{
    BitLocker, BitLockerStep, Bus, Computer, Concern, DevImage, Disk, FaradayStick, InUse,
    InstallItem, Installed, Kind, Mode, Os, Payload, PayloadFile, Protection, Reason, RecoveryKey,
    StarterEvent, Volume, WriteStage,
};
use faraday_starter::decide::Target;
use faraday_starter::notes::Notes;
use faraday_starter::{LearnId, Starter, StarterAction};

/// A token for the PDFs these snapshots make, counting nothing. Cheap.
#[expect(
    clippy::disallowed_methods,
    reason = "a mint: the Starter's snapshots, which draw nothing"
)]
fn compute() -> faraday_crypto::work::Compute {
    faraday_crypto::work::Compute::new(|_| {})
}

const GB: u64 = 1_000_000_000;

fn payload() -> Payload {
    let file = |name: &str, inner: &str, size: u64, build: &str, a: &str, b: &str| PayloadFile {
        name: name.into(),
        sha256: a.repeat(8),
        inner: inner.into(),
        inner_sha256: b.repeat(8),
        inner_size: size,
        build_id: build.into(),
    };
    Payload {
        version: "0.2.0".into(),
        commit: "4d0680b1a2b3c4d5e6f708192a3b4c5d6e7f8091".into(),
        files: vec![
            file(
                "faraday-stick-0.2.0-x86_64-uefi.img.xz",
                "faraday-stick-0.2.0-x86_64-uefi.img",
                3_900_000_000,
                "0.2.0",
                "3f9a0c1d",
                "8be41a07",
            ),
            file(
                "faraday-pi-0.2.0-pi3-waveshare-28dpi.img.xz",
                "faraday-pi-0.2.0-pi3-waveshare-28dpi.img",
                2_100_000_000,
                "0.2.0",
                "11d0e2a9",
                "c07b5e33",
            ),
        ],
    }
}

fn disk(id: &str, vendor: &str, model: &str, size: u64, volumes: &[(&str, bool)]) -> Disk {
    Disk {
        id: id.into(),
        device: format!("/dev/{id}"),
        vendor: vendor.into(),
        model: model.into(),
        serial: format!("4C530001{}", id.to_uppercase()),
        size,
        bus: Bus::Usb,
        volumes: volumes
            .iter()
            .map(|(l, f)| Volume {
                label: (!l.is_empty()).then(|| l.to_string()),
                files: *f,
            })
            .collect(),
        in_use: None,
        faraday: None,
    }
}

fn stick(id: &str, release: &str) -> Disk {
    Disk {
        faraday: Some(FaradayStick {
            release: Some(release.into()),
            read: true,
        }),
        ..disk(id, "SanDisk", "Ultra Fit", 32 * GB, &[])
    }
}

fn laptop(os: Os, secure_boot: bool) -> Computer {
    Computer {
        os,
        maker: "Example Computers".into(),
        model: "Example 14".into(),
        family: "Example".into(),
        uefi: true,
        arm: false,
        secure_boot: Some(secure_boot),
        tpm_unlock: false,
    }
}

/// A Mac on macOS, by its model identifier.
fn mac(model: &str) -> Computer {
    Computer {
        os: Os::MacOs,
        maker: "Apple Inc.".into(),
        model: model.into(),
        family: String::new(),
        uefi: true,
        arm: false,
        secure_boot: None,
        tpm_unlock: false,
    }
}

fn many() -> Vec<Disk> {
    let mut backup = disk("sdc", "WD", "Elements 2TB", 2_000 * GB, &[("BACKUP", true)]);
    backup.in_use = Some(InUse::Mounted("/srv/backup".into()));
    vec![
        disk(
            "sdb",
            "Kingston",
            "DataTraveler",
            64 * GB,
            &[("PHOTOS", true), ("", false)],
        ),
        backup,
        disk("sdd", "Generic", "Flash Disk", 2 * GB, &[]),
        stick("sde", "6.6.84-faraday-0.1.0+ef24784b48d8.test"),
        disk("sdf", "SanDisk", "Cruzer Blade", 16 * GB, &[]),
    ]
}

struct Snap<'a> {
    out: &'a Path,
    size: (u16, u16),
    dpi: u16,
}

impl Snap<'_> {
    fn starter(&self) -> Starter {
        let mut starter = Starter::new();
        starter.event(Event::Display(DisplayInfo {
            width: self.size.0,
            height: self.size.1,
            dpi: self.dpi,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: false,
            secure: SecureHardware::None,
            boot: BootState::Unknown,
            memory_mib: None,
        }));
        starter.starter_event(StarterEvent::Computer(laptop(Os::Linux, true)));
        starter.starter_event(StarterEvent::Payload(Some(payload())));
        starter.starter_event(StarterEvent::Disks(Vec::new()));
        starter
    }

    fn shot(&self, starter: &mut Starter, name: &str) -> Result<(), String> {
        while starter.poll_command().is_some() {}
        starter.event(Event::Tick { now_ms: 10_000 });
        starter.held().settle();
        while starter.poll_command().is_some() {}
        let frame = starter.frame();
        let path = self.out.join(format!("{name}.png"));
        let file = std::fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut enc = png::Encoder::new(
            std::io::BufWriter::new(file),
            u32::from(frame.width),
            u32::from(frame.height),
        );
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().map_err(|e| e.to_string())?;
        w.write_image_data(frame.rgba).map_err(|e| e.to_string())?;
        println!("{}", path.display());
        Ok(())
    }

    /// Scrolled down by `dy` wheel units, for what is below the first
    /// screen on a small panel.
    fn shot_down(&self, starter: &mut Starter, name: &str, dy: i16) -> Result<(), String> {
        let _ = starter.frame();
        starter.event(Event::Wheel { x: 10, y: 10, dy });
        let _ = starter.frame();
        self.shot(starter, name)
    }

    /// Scrolled to the end, for a page longer than the screen.
    fn shot_end(&self, starter: &mut Starter, name: &str) -> Result<(), String> {
        let _ = starter.frame();
        for _ in 0..20 {
            starter.event(Event::Wheel {
                x: 10,
                y: 10,
                dy: 2000,
            });
            let _ = starter.frame();
        }
        self.shot(starter, name)
    }
}

fn to(starter: &mut Starter, id: &str, mode: Mode) {
    starter.press(StarterAction::Make(Target::Stick));
    let n = StarterAction::Disk(starter.disk_number(id));
    starter.press(n);
    starter.press(StarterAction::Mode(mode));
    starter.press(StarterAction::Continue);
}

/// Another encryption product found on Windows (§6.4): the card asks for
/// its rescue disk or password before the firmware steps open.
fn other_encryption_shots(s: &Snap) -> Result<(), String> {
    let mut h = s.starter();
    h.starter_event(StarterEvent::Computer(laptop(Os::Windows, true)));
    h.starter_event(StarterEvent::Disks(vec![stick(
        "sde",
        "6.6.84-faraday-0.2.0",
    )]));
    h.starter_event(StarterEvent::OtherEncryption(Some("VeraCrypt".into())));
    s.shot(&mut h, "home-windows-veracrypt")
}

/// BitLocker on the Start page (§6, F49): on, the key's tick box ticked,
/// suspended for three times Windows starts, a refusal, and off.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn bitlocker_shots(s: &Snap) -> Result<(), String> {
    let read = |protection, starts_left| {
        StarterEvent::BitLocker(Some(BitLocker {
            drive: "C:".into(),
            protection,
            starts_left,
        }))
    };
    let mut h = s.starter();
    h.starter_event(StarterEvent::Computer(laptop(Os::Windows, true)));
    h.starter_event(StarterEvent::Disks(vec![stick(
        "sde",
        "6.6.84-faraday-0.2.0",
    )]));
    h.press(StarterAction::Start);
    h.starter_event(read(Protection::On, None));
    s.shot(&mut h, "start-windows-bitlocker-on")?;
    if s.size.0 < 800 {
        s.shot_down(&mut h, "start-windows-bitlocker-on-down", 520)?;
        h.event(Event::Wheel {
            x: 10,
            y: 10,
            dy: -5000,
        });
    }
    h.press(StarterAction::IncludeKey);
    s.shot(&mut h, "start-windows-bitlocker-key-ticked")?;
    if s.size.0 < 800 {
        s.shot_down(&mut h, "start-windows-bitlocker-key-ticked-down", 520)?;
        h.event(Event::Wheel {
            x: 10,
            y: 10,
            dy: -5000,
        });
    }
    h.press(StarterAction::IncludeKey);
    h.press(StarterAction::SuspendBitLocker);
    s.shot(&mut h, "start-windows-bitlocker-waiting")?;
    if s.size.0 < 800 {
        s.shot_down(&mut h, "start-windows-bitlocker-waiting-down", 520)?;
        h.event(Event::Wheel {
            x: 10,
            y: 10,
            dy: -5000,
        });
    }
    h.starter_event(StarterEvent::BitLockerDone {
        step: BitLockerStep::Suspend,
        result: Ok(()),
    });
    h.starter_event(read(Protection::Suspended, Some(3)));
    s.shot(&mut h, "start-windows-bitlocker-suspended")?;
    if s.size.0 < 800 {
        s.shot_down(&mut h, "start-windows-bitlocker-suspended-down", 520)?;
        h.event(Event::Wheel {
            x: 10,
            y: 10,
            dy: -5000,
        });
    }
    h.starter_event(read(Protection::On, None));
    h.press(StarterAction::SuspendBitLocker);
    h.starter_event(StarterEvent::BitLockerDone {
        step: BitLockerStep::Suspend,
        result: Err(Reason::Managed(0x8031_003A)),
    });
    s.shot(&mut h, "start-windows-bitlocker-managed")?;
    if s.size.0 < 800 {
        s.shot_down(&mut h, "start-windows-bitlocker-managed-down", 520)?;
        h.event(Event::Wheel {
            x: 10,
            y: 10,
            dy: -5000,
        });
    }
    let mut h = s.starter();
    h.starter_event(StarterEvent::Computer(laptop(Os::Windows, true)));
    h.starter_event(StarterEvent::Disks(vec![stick(
        "sde",
        "6.6.84-faraday-0.2.0",
    )]));
    // The sheet Print hands the viewer, with a made-up key ticked in, for
    // a maker with a page and one without (§3.4, F49).
    let key = RecoveryKey::parse(
        "C:",
        "111111-222222-333333-444444-555555-666666-777777-888888",
    )
    .ok_or("the made-up key")?;
    let mut dell = laptop(Os::Windows, true);
    dell.maker = "Dell Inc.".into();
    dell.family = "Latitude".into();
    dell.model = "Latitude 5440".into();
    for (c, name) in [
        (dell, "sheet-key-dell"),
        (laptop(Os::Windows, true), "sheet-key"),
    ] {
        let notes = Notes::for_computer(&c).ok_or("notes")?;
        let path = s.out.join(format!("{name}.pdf"));
        std::fs::write(&path, notes.pdf_with_key(&compute(), &key))
            .map_err(|e| format!("{}: {e}", path.display()))?;
        println!("{}", path.display());
    }
    h.press(StarterAction::Start);
    h.starter_event(read(Protection::None, None));
    s.shot(&mut h, "start-windows-bitlocker-off")?;
    if s.size.0 < 800 {
        s.shot_down(&mut h, "start-windows-bitlocker-off-down", 520)?;
    }
    Ok(())
}

fn concern(kind: Kind, name: &str, id: &str) -> Concern {
    Concern {
        kind,
        name: name.into(),
        id: id.into(),
    }
}

/// The holds of §3.1a, the Continue anyway sheet, and the boot entry's
/// row (§3.1c).
fn concern_shots(s: &Snap) -> Result<(), String> {
    let mut h = s.starter();
    h.starter_event(StarterEvent::Computer(laptop(Os::Linux, false)));
    h.starter_event(StarterEvent::Disks(vec![stick(
        "sde",
        "6.6.84-faraday-0.2.0",
    )]));
    h.starter_event(StarterEvent::Concerns(vec![
        concern(Kind::Tv, "EXAMPLE TV", "drm:card1-HDMI-A-1"),
        concern(Kind::Receiver, "Example USB Receiver", "usb:1-2"),
    ]));
    s.shot(&mut h, "concern-home-tv-receiver")?;
    let n = StarterAction::ContinueAnyway(h.concern_number("drm:card1-HDMI-A-1"));
    h.press(n);
    s.shot(&mut h, "concern-continue-anyway-sheet")?;
    h.press(StarterAction::LetThrough);
    s.shot(&mut h, "concern-home-tv-let-through")?;
    h.press(StarterAction::Start);
    s.shot(&mut h, "concern-start-receiver")?;
    let mut h = s.starter();
    h.starter_event(StarterEvent::Concerns(vec![concern(
        Kind::VirtualMachine,
        "QEMU",
        "vm",
    )]));
    s.shot(&mut h, "concern-home-virtual-machine")?;
    let mut h = s.starter();
    h.starter_event(StarterEvent::Computer(laptop(Os::Windows, true)));
    h.starter_event(StarterEvent::Concerns(vec![concern(
        Kind::Kvm,
        "PiKVM V4",
        "drm:card0-HDMI-A-1",
    )]));
    s.shot(&mut h, "concern-windows-kvm-held")?;
    let mut h = s.starter();
    h.starter_event(StarterEvent::BootEntry(true));
    s.shot_end(&mut h, "home-remove-boot-entry")?;
    h.press(StarterAction::RemoveEntry);
    h.starter_event(StarterEvent::BootEntryRemoved(Err(Reason::Declined)));
    s.shot_end(&mut h, "home-remove-boot-entry-declined")?;
    Ok(())
}

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn run(s: &Snap) -> Result<(), String> {
    // Home in each state.
    let mut h = s.starter();
    s.shot(&mut h, "home-no-stick")?;
    s.shot_end(&mut h, "home-no-stick-end")?;
    let mut h = s.starter();
    h.starter_event(StarterEvent::Disks(vec![stick(
        "sde",
        "6.6.84-faraday-0.2.0",
    )]));
    s.shot(&mut h, "home-stick")?;
    h.press(StarterAction::Next);
    s.shot(&mut h, "home-restart-firmware-sheet")?;
    h.press(StarterAction::Cancel);
    let mut c = laptop(Os::Linux, true);
    c.secure_boot = None;
    h.starter_event(StarterEvent::Computer(c));
    s.shot(&mut h, "home-secure-boot-unknown")?;
    h.starter_event(StarterEvent::Computer(laptop(Os::Linux, false)));
    s.shot(&mut h, "home-stick-secure-boot-off")?;
    h.press(StarterAction::Next);
    s.shot(&mut h, "home-restart-stick-sheet")?;
    h.press(StarterAction::Cancel);
    h.starter_event(StarterEvent::StickRestartFailed(Reason::Missing(
        "efibootmgr".into(),
    )));
    s.shot(&mut h, "home-stick-restart-failed")?;
    let mut h = s.starter();
    h.starter_event(StarterEvent::Computer(laptop(Os::Windows, false)));
    h.starter_event(StarterEvent::Disks(vec![stick(
        "sde",
        "6.6.84-faraday-0.2.0",
    )]));
    s.shot(&mut h, "home-windows-stick-secure-boot-off")?;
    h.press(StarterAction::Next);
    s.shot(&mut h, "home-windows-restart-stick-sheet")?;
    let mut h = s.starter();
    h.starter_event(StarterEvent::Computer(laptop(Os::Windows, true)));
    h.starter_event(StarterEvent::Disks(vec![stick(
        "sde",
        "6.6.84-faraday-0.2.0",
    )]));
    s.shot(&mut h, "home-windows-stick-secure-boot-on")?;
    let mut c = laptop(Os::Windows, true);
    c.secure_boot = None;
    h.starter_event(StarterEvent::Computer(c));
    s.shot(&mut h, "home-windows-stick-secure-boot-unread")?;
    let mut h = s.starter();
    let mut arm = laptop(Os::Windows, true);
    arm.arm = true;
    arm.maker = "Microsoft Corporation".into();
    arm.model = "Surface Example".into();
    h.starter_event(StarterEvent::Computer(arm));
    s.shot(&mut h, "home-arm")?;

    // On a Mac (§3.1's "On a Mac", §3.4): an Intel Mac and a T2 Mac,
    // Home with a stick attached and the Start page.
    for (model, name) in [
        ("MacBookPro11,1", "mac-intel"),
        ("MacBookPro15,1", "mac-t2"),
        ("MacBookPro99,9", "mac-unknown"),
    ] {
        let mut h = s.starter();
        h.starter_event(StarterEvent::Computer(mac(model)));
        s.shot(&mut h, &format!("home-{name}-no-stick"))?;
        h.starter_event(StarterEvent::Disks(vec![stick(
            "disk4",
            "6.6.84-faraday-0.2.0",
        )]));
        s.shot(&mut h, &format!("home-{name}"))?;
        h.press(StarterAction::Next);
        s.shot(&mut h, &format!("start-{name}"))?;
    }
    let mut h = s.starter();
    h.starter_event(StarterEvent::Computer(Computer {
        arm: true,
        uefi: false,
        ..mac("Mac14,2")
    }));
    s.shot(&mut h, "home-mac-m2")?;
    // For which computer?, with a stand-in Apple silicon image.
    let mut p = payload();
    let mut apple = p.files[0].clone();
    apple.name = "faraday-stick-0.2.0-arm64-apple.img.xz".into();
    apple.inner = "faraday-stick-0.2.0-arm64-apple.img".into();
    p.files.push(apple);
    h.starter_event(StarterEvent::Payload(Some(p)));
    h.press(StarterAction::Make(Target::Stick));
    s.shot(&mut h, "stick-0-which-mac-m2")?;

    // What is attached that can capture the screen or the keys (§3.1a).
    concern_shots(s)?;

    // Make a Faraday stick.
    let mut h = s.starter();
    h.press(StarterAction::Make(Target::Stick));
    s.shot(&mut h, "stick-1-which-none")?;
    h.starter_event(StarterEvent::Disks(many()));
    s.shot(&mut h, "stick-1-which")?;
    s.shot_end(&mut h, "stick-1-which-end")?;
    let n = StarterAction::Disk(h.disk_number("sdb"));
    h.press(n);
    s.shot(&mut h, "stick-2-what-plain")?;
    h.press(StarterAction::Back);
    let n = StarterAction::Disk(h.disk_number("sde"));
    h.press(n);
    h.press(StarterAction::Mode(Mode::KeepData));
    s.shot(&mut h, "stick-2-what-faraday")?;
    let mut h = s.starter();
    h.starter_event(StarterEvent::Disks(vec![stick(
        "sde",
        "6.6.84-faraday-0.2.0",
    )]));
    h.press(StarterAction::Make(Target::Stick));
    let n = StarterAction::Disk(h.disk_number("sde"));
    h.press(n);
    s.shot(&mut h, "stick-2-what-same-build")?;

    // Windows, a stick whose version it did not let the Starter read:
    // Home, Which stick, then keep data reading it and
    // the prompt declined.
    let unread = || {
        let mut d = stick("sde", "");
        d.faraday = Some(FaradayStick {
            release: None,
            read: false,
        });
        d
    };
    let mut h = s.starter();
    h.starter_event(StarterEvent::Computer(laptop(Os::Windows, true)));
    h.starter_event(StarterEvent::Disks(vec![unread()]));
    s.shot(&mut h, "home-windows-stick-version-not-read")?;
    h.press(StarterAction::Make(Target::Stick));
    s.shot(&mut h, "stick-1-which-windows-version-not-read")?;
    let n = StarterAction::Disk(h.disk_number("sde"));
    h.press(n);
    h.press(StarterAction::Mode(Mode::KeepData));
    s.shot(&mut h, "stick-2-what-windows-reading")?;
    let d = unread();
    h.starter_event(StarterEvent::VersionRead {
        disk: d.id,
        serial: d.serial,
        size: d.size,
        result: Err(Reason::Declined),
    });
    s.shot(&mut h, "stick-2-what-windows-declined")?;

    let mut h = s.starter();
    h.starter_event(StarterEvent::Disks(many()));
    to(&mut h, "sdf", Mode::Whole);
    s.shot(&mut h, "stick-3-confirm")?;
    s.shot_end(&mut h, "stick-3-confirm-end")?;
    let mut other = many();
    other[4].serial = "OTHER".into();
    h.starter_event(StarterEvent::Disks(other));
    s.shot_end(&mut h, "stick-3-confirm-not-the-one")?;
    let mut h = s.starter();
    h.starter_event(StarterEvent::Disks(vec![stick(
        "sde",
        "6.6.84-faraday-0.3.0",
    )]));
    to(&mut h, "sde", Mode::KeepData);
    s.shot_end(&mut h, "stick-3-confirm-keep-newer")?;
    let mut h = s.starter();
    h.starter_event(StarterEvent::Disks(many()));
    h.starter_event(StarterEvent::DevImage(DevImage {
        path: "out/stick/faraday-x86_64-uefi-dev.img".into(),
        sha256: "9c1e44d0".repeat(8),
        size: 3_900_000_000,
        boot_only: false,
        build_id: Some("0.2.0+4d0680b1a2b3.dev".into()),
    }));
    to(&mut h, "sdf", Mode::Whole);
    s.shot_end(&mut h, "stick-3-confirm-dev-image")?;

    let mut h = s.starter();
    h.starter_event(StarterEvent::Disks(many()));
    to(&mut h, "sdf", Mode::Whole);
    h.press(StarterAction::Write);
    h.starter_event(StarterEvent::WriteProgress {
        stage: WriteStage::Writing,
        done: 1_400_000_000,
        total: 3_900_000_000,
    });
    s.shot(&mut h, "stick-4-writing")?;
    h.starter_event(StarterEvent::WriteProgress {
        stage: WriteStage::ReadingBack,
        done: 3_100_000_000,
        total: 3_900_000_000,
    });
    s.shot(&mut h, "stick-4-reading-back")?;
    h.starter_event(StarterEvent::Written(Ok(())));
    s.shot(&mut h, "stick-5-done")?;
    let mut h = s.starter();
    h.starter_event(StarterEvent::Disks(many()));
    to(&mut h, "sdf", Mode::Whole);
    h.press(StarterAction::Write);
    h.starter_event(StarterEvent::Written(Err(Reason::BackDiffers {
        device: "/dev/sdf".into(),
        at: 1_200_000_000,
    })));
    s.shot(&mut h, "stick-5-failed")?;

    // Make a Pi card.
    let mut h = s.starter();
    let mut reader = disk(
        "mmcblk0",
        "",
        "SD card reader",
        32 * GB,
        &[("boot", true), ("rootfs", true)],
    );
    reader.bus = Bus::SdReader;
    let mut disks = many();
    disks.push(reader);
    h.starter_event(StarterEvent::Disks(disks));
    h.press(StarterAction::Make(Target::Pi));
    s.shot(&mut h, "pi-1-which")?;
    let n = StarterAction::Disk(h.disk_number("mmcblk0"));
    h.press(n);
    s.shot(&mut h, "pi-2-what")?;

    // Start this computer from the stick.
    let mut h = s.starter();
    h.press(StarterAction::Start);
    s.shot(&mut h, "start-linux")?;
    s.shot_end(&mut h, "start-linux-end")?;
    let mut h = s.starter();
    let mut c = laptop(Os::Linux, false);
    c.tpm_unlock = true;
    h.starter_event(StarterEvent::Computer(c));
    h.press(StarterAction::Start);
    s.shot(&mut h, "start-linux-secure-boot-off-tpm")?;
    let mut h = s.starter();
    h.starter_event(StarterEvent::Computer(laptop(Os::Windows, true)));
    h.press(StarterAction::Start);
    s.shot(&mut h, "start-windows-secure-boot-on")?;
    s.shot_end(&mut h, "start-windows-secure-boot-on-end")?;
    h.starter_event(StarterEvent::Computer(laptop(Os::Windows, false)));
    s.shot_end(&mut h, "start-windows-secure-boot-off-end")?;
    h.starter_event(StarterEvent::Computer(laptop(Os::Windows, true)));
    h.press(StarterAction::Restart);
    s.shot(&mut h, "start-restart-sheet")?;
    // A maker with a page of its own (§7.6).
    let mut h = s.starter();
    let mut c = laptop(Os::Windows, true);
    c.maker = "Dell Inc.".into();
    h.starter_event(StarterEvent::Computer(c));
    h.press(StarterAction::Start);
    s.shot(&mut h, "start-maker")?;
    s.shot_end(&mut h, "start-maker-end")?;
    h.press(StarterAction::Restart);
    s.shot(&mut h, "start-maker-restart-sheet")?;
    // What to write down, as the sheet Print opens (§3.4, F47).
    let mut dell = laptop(Os::Windows, true);
    dell.maker = "Dell Inc.".into();
    for (c, name) in [
        (dell, "notes-dell"),
        (laptop(Os::Windows, true), "notes-unknown"),
        (mac("MacBookPro15,1"), "notes-mac-t2"),
        (mac("MacBookPro99,9"), "notes-mac-unknown"),
    ] {
        let notes = faraday_starter::notes::Notes::for_computer(&c).ok_or("no notes")?;
        let path = s.out.join(format!("{name}.pdf"));
        std::fs::write(&path, notes.pdf(&compute()))
            .map_err(|e| format!("{}: {e}", path.display()))?;
        println!("{}", path.display());
    }

    // Install, Installed, About.
    let mut h = s.starter();
    h.press(StarterAction::Install);
    s.shot(&mut h, "install")?;
    h.press(StarterAction::InstallNow);
    s.shot(&mut h, "install-installing")?;
    h.starter_event(StarterEvent::InstallDone(Ok(
        "~/.local/bin/faraday-starter".into(),
    )));
    s.shot(&mut h, "install-done")?;
    let mut h = s.starter();
    h.starter_event(StarterEvent::Installed(Some(Installed {
        version: "0.2.0".into(),
        items: vec![
            (InstallItem::App, "~/.local/bin/faraday-starter".into()),
            (
                InstallItem::Menu,
                "~/.local/share/applications/app.faradayoffline.FaradayStarter.desktop".into(),
            ),
            (
                InstallItem::Icon,
                "~/.local/share/icons/hicolor/256x256/apps/app.faradayoffline.FaradayStarter.png"
                    .into(),
            ),
        ],
    })));
    s.shot(&mut h, "home-installed")?;
    h.press(StarterAction::Installed);
    s.shot(&mut h, "installed")?;
    h.press(StarterAction::Uninstall);
    s.shot(&mut h, "installed-uninstall-sheet")?;
    h.press(StarterAction::UninstallNow);
    h.starter_event(StarterEvent::BootEntryKept(Reason::Declined));
    h.starter_event(StarterEvent::UninstallDone(Ok(())));
    s.shot(&mut h, "installed-entry-kept")?;
    let mut h = s.starter();
    h.press(StarterAction::About);
    s.shot(&mut h, "about")?;
    s.shot_end(&mut h, "about-end")?;
    other_encryption_shots(s)?;
    bitlocker_shots(s)?;

    // Blank templates (§3.8): the menu, a Choice, and after a save.
    let mut h = s.starter();
    h.press(StarterAction::Templates);
    s.shot(&mut h, "templates")?;
    h.press(StarterAction::TemplateChoice(
        faraday_starter::TemplateChoice::Copy,
    ));
    s.shot(&mut h, "templates-copy")?;
    h.press(StarterAction::Back);
    h.press(StarterAction::SaveTemplate);
    h.starter_event(StarterEvent::TemplateSaved(Ok(
        "Downloads/faraday-blank-template-24-words-words.pdf".into(),
    )));
    s.shot(&mut h, "templates-saved")?;

    // The desktop app's own Transfer and Learn.
    let mut h = s.starter();
    h.press(StarterAction::Transfer);
    s.shot(&mut h, "transfer")?;
    let mut h = s.starter();
    h.press(StarterAction::Learn);
    s.shot(&mut h, "learn")?;
    sketch_shots(s)?;
    step_shots(s)?;
    guide_shots(s)?;
    guide_start_shots(s)
}

/// A USB stick named for no maker, as `docs/GUIDE.md` shows one, as
/// Windows names its disks: `n` is its number there.
fn plain(n: u8, size: u64, volumes: &[(&str, bool)]) -> Disk {
    Disk {
        device: format!(r"\\.\PhysicalDrive{n}"),
        ..disk(&format!("disk{n}"), "USB", "Flash Drive", size, volumes)
    }
}

/// The pictures in `docs/GUIDE.md` §1, making the stick: a Windows laptop
/// with Secure Boot on and two sticks named for no maker; Which stick?,
/// What to write?, Confirm, the write, Done, and a write that failed.
fn guide_shots(s: &Snap) -> Result<(), String> {
    let windows = || StarterEvent::Computer(laptop(Os::Windows, true));
    let sticks = || {
        StarterEvent::Disks(vec![
            plain(1, 32 * GB, &[]),
            plain(2, 64 * GB, &[("PHOTOS", true)]),
        ])
    };
    let mut h = s.starter();
    h.starter_event(windows());
    s.shot(&mut h, "guide-home")?;
    h.press(StarterAction::Make(Target::Stick));
    h.starter_event(sticks());
    s.shot(&mut h, "guide-which-stick")?;
    let n = StarterAction::Disk(h.disk_number("disk1"));
    h.press(n);
    s.shot(&mut h, "guide-what-to-write")?;
    h.press(StarterAction::Mode(Mode::Whole));
    h.press(StarterAction::Continue);
    s.shot(&mut h, "guide-confirm")?;
    s.shot_end(&mut h, "guide-confirm-end")?;
    h.press(StarterAction::Write);
    h.starter_event(StarterEvent::WriteProgress {
        stage: WriteStage::Writing,
        done: 1_400_000_000,
        total: 3_900_000_000,
    });
    s.shot(&mut h, "guide-writing")?;
    h.starter_event(StarterEvent::Written(Ok(())));
    s.shot(&mut h, "guide-done")?;
    let mut h = s.starter();
    h.starter_event(windows());
    h.starter_event(sticks());
    to(&mut h, "disk1", Mode::Whole);
    h.press(StarterAction::Write);
    h.starter_event(StarterEvent::Written(Err(Reason::BackDiffers {
        device: r"\\.\PhysicalDrive1".into(),
        at: 1_200_000_000,
    })));
    s.shot(&mut h, "guide-write-failed")
}

/// The pictures in `docs/GUIDE.md` §1, starting from the stick: Home with
/// the stick written, Secure Boot on, its restart sheet and the Start
/// page; then Secure Boot off, Home's button and its sheet.
fn guide_start_shots(s: &Snap) -> Result<(), String> {
    let written = || {
        let mut d = plain(1, 32 * GB, &[]);
        d.faraday = Some(FaradayStick {
            release: None,
            read: false,
        });
        StarterEvent::Disks(vec![d])
    };
    let mut h = s.starter();
    h.starter_event(StarterEvent::Computer(laptop(Os::Windows, true)));
    h.starter_event(written());
    s.shot(&mut h, "guide-home-stick")?;
    h.press(StarterAction::Next);
    s.shot(&mut h, "guide-restart-bios-sheet")?;
    h.press(StarterAction::Cancel);
    h.press(StarterAction::Start);
    s.shot(&mut h, "guide-start")?;
    s.shot_end(&mut h, "guide-start-end")?;
    let mut h = s.starter();
    h.starter_event(StarterEvent::Computer(laptop(Os::Windows, false)));
    h.starter_event(written());
    s.shot(&mut h, "guide-home-secure-boot-off")?;
    h.press(StarterAction::Next);
    s.shot(&mut h, "guide-restart-stick-sheet")
}

/// Maker pages scrolled to one of their firmware sketches (§7.8): the
/// page, a line of the sketch, how far above that line the shot starts
/// (units), and the picture's name.
fn sketch_shots(s: &Snap) -> Result<(), String> {
    let shots: [(&str, &str, f32, &str); 10] = [
        ("maker-hp", "USB Boot", 90.0, "sketch-hp-secure-boot"),
        (
            "maker-hp",
            "Ignore Changes and Exit",
            90.0,
            "sketch-hp-save",
        ),
        ("maker-hp", "7294 + ENTER", 160.0, "sketch-hp-code"),
        (
            "maker-lenovo-thinkpad",
            "Secure Boot Mode",
            110.0,
            "sketch-thinkpad-secure-boot",
        ),
        (
            "maker-lenovo-ideapad",
            "USB ZIP : USB Flash Drive",
            110.0,
            "sketch-ideapad-boot-menu",
        ),
        (
            "maker-surface",
            "Microsoft only",
            170.0,
            "sketch-surface-secure-boot",
        ),
        ("maker-asus", "Enter Setup", 130.0, "sketch-asus-boot-menu"),
        (
            "maker-dell",
            "Enable Microsoft UEFI CA",
            110.0,
            "sketch-dell-secure-boot",
        ),
        (
            "maker-framework",
            "Options:",
            150.0,
            "sketch-framework-secure-boot",
        ),
        ("maker-msi", "System Mode", 110.0, "sketch-msi-secure-boot"),
    ];
    for (page, line, above, name) in shots {
        let mut h = s.starter();
        h.press(StarterAction::LearnPage(LearnId::Maker(page)));
        let _ = h.drawn_texts();
        let max = h.held().learn().max;
        // The last offset the line is still in view at: it is at the
        // top then.
        let (mut seen, mut at, mut y) = (false, 0.0, 0.0);
        while y <= max {
            faraday_core::testkit::learn_scroll_to(h.held(), y);
            if h.drawn_texts().iter().any(|t| t == line) {
                (seen, at) = (true, y);
            } else if seen {
                break;
            }
            y += 10.0;
        }
        if !seen {
            eprintln!("{name}: \"{line}\" not drawn");
        }
        faraday_core::testkit::learn_scroll_to(h.held(), (at - above).max(0.0));
        s.shot(&mut h, name)?;
    }
    Ok(())
}

/// A guide page at its top, its first steps in circles
/// (`docs/STARTER.md` §7): HP untouched, Dell with its first three
/// ticked.
fn step_shots(s: &Snap) -> Result<(), String> {
    let mut h = s.starter();
    h.press(StarterAction::LearnPage(LearnId::Maker("maker-hp")));
    s.shot(&mut h, "steps-hp")?;

    let mut h = s.starter();
    h.press(StarterAction::LearnPage(LearnId::Maker("maker-dell")));
    for n in 1..=3 {
        h.held().press(faraday_core::Action::LearnStep(n));
    }
    s.shot(&mut h, "steps-dell-ticked")
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: snap WxH[@DPI] OUT_DIR");
        return ExitCode::from(2);
    }
    let (size, dpi) = match args[1].split_once('@') {
        Some((s, d)) => (s, d.parse().unwrap_or(160)),
        None => (args[1].as_str(), 160),
    };
    let Some(size) = size
        .split_once('x')
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
    else {
        eprintln!("size must be WxH[@DPI]");
        return ExitCode::from(2);
    };
    let out = Path::new(&args[2]);
    if let Err(e) = std::fs::create_dir_all(out) {
        eprintln!("{}: {e}", out.display());
        return ExitCode::FAILURE;
    }
    match run(&Snap { out, size, dpi }) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
