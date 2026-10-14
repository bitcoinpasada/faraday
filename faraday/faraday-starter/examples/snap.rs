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

use faraday_qr::bitlocker::{KeyText, RecoveryKey};
use faraday_starter::api::{
    BitLocker, BitLockerStep, Bus, Computer, Concern, DevImage, Disk, FaradayStick, InUse,
    InstallItem, Installed, KeyShown, Kind, Marker, Mode, Os, Payload, PayloadFile, Protection,
    Reason, StarterEvent, Volume, WriteStage,
};
use faraday_starter::decide::Target;
use faraday_starter::{LearnId, Starter, StarterAction};
use osk_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};

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
            file(
                "faraday-desktop-0.2.0-linux-x86_64.xz",
                "faraday-desktop-0.2.0-linux-x86_64",
                11_000_000,
                "0.2.0",
                "5aa1f00e",
                "e2c94b10",
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

fn suspended(was_on: bool) -> BitLocker {
    BitLocker {
        drive: "C:".into(),
        protection: Protection::Suspended,
        marker: Some(Marker {
            volumes: vec!["C:".into()],
            secure_boot_was_on: was_on,
            this_boot: false,
        }),
    }
}

fn protected() -> BitLocker {
    BitLocker {
        drive: "C:".into(),
        protection: Protection::On,
        marker: None,
    }
}

/// A made-up recovery key.
fn recovery_key() -> KeyShown {
    KeyShown {
        text: KeyText {
            computer: "DESKTOP-4K2Q".into(),
            drive: "C:".into(),
            key: RecoveryKey::parse("205568-492019-108251-336446-650375-210331-584155-047520")
                .expect("a key"),
        },
        id: "{6A0E1F2C-3B4D-4E5F-8A9B-0C1D2E3F4A5B}".into(),
    }
}

/// A Windows laptop with BitLocker on, on the guard's page.
fn guard(s: &Snap, b: BitLocker) -> Starter {
    let mut h = s.starter();
    h.starter_event(StarterEvent::Computer(laptop(Os::Windows, true)));
    h.starter_event(StarterEvent::BitLocker(Some(b)));
    h.press(StarterAction::BitLocker);
    h
}

/// The guard's page in each of its states (docs/STARTER.md §6.5).
fn guard_shots(s: &Snap) -> Result<(), String> {
    let mut h = guard(s, protected());
    s.shot(&mut h, "bitlocker-1-protected")?;
    // The key shown and Done: Home reads "Key shown" (F21).
    let mut shown = guard(s, protected());
    shown.press(StarterAction::ShowKey);
    shown.starter_event(StarterEvent::KeyShown(Ok(recovery_key())));
    while shown.poll_command().is_some() {}
    shown.press(StarterAction::KeyDone);
    s.shot(&mut shown, "bitlocker-1-key-shown")?;
    h.press(StarterAction::ShowKey);
    s.shot(&mut h, "bitlocker-1-waiting")?;
    h.starter_event(StarterEvent::KeyShown(Err(Reason::Declined)));
    s.shot(&mut h, "bitlocker-1-declined")?;
    h.starter_event(StarterEvent::KeyShown(Err(Reason::NoAdmin)));
    s.shot(&mut h, "bitlocker-1-no-admin")?;
    h.starter_event(StarterEvent::Disks(vec![disk(
        "sdb",
        "Kingston",
        "DataTraveler",
        64 * GB,
        &[("PHOTOS", true)],
    )]));
    h.press(StarterAction::ShowKey);
    h.starter_event(StarterEvent::KeyShown(Ok(recovery_key())));
    while h.poll_command().is_some() {}
    let mut bytes = [0u8; 32];
    bytes[..2].copy_from_slice(&[3, 2]);
    h.event(Event::Entropy(EntropyBytes::new(bytes)));
    s.shot(&mut h, "bitlocker-2-key")?;
    s.shot_end(&mut h, "bitlocker-2-key-end")?;
    h.press(StarterAction::ExtraSafe);
    s.shot_end(&mut h, "bitlocker-2-key-extra-safe")?;
    h.press(StarterAction::KeptIt);
    s.shot(&mut h, "bitlocker-3-groups")?;
    for c in "336446".chars() {
        h.event(Event::Key(Key::Char(c)));
    }
    for c in "58415".chars() {
        h.event(Event::Key(Key::Char(c)));
    }
    s.shot_end(&mut h, "bitlocker-3-groups-typing")?;
    h.event(Event::Key(Key::Char('6')));
    h.press(StarterAction::Suspend);
    s.shot_end(&mut h, "bitlocker-3-groups-wrong")?;
    h.event(Event::Key(Key::Backspace));
    h.event(Event::Key(Key::Char('5')));
    h.press(StarterAction::Suspend);
    s.shot_end(&mut h, "bitlocker-4-suspending")?;
    h.starter_event(StarterEvent::BitLocker(Some(protected())));
    h.starter_event(StarterEvent::BitLockerDone {
        step: BitLockerStep::Suspend,
        result: Err(Reason::Managed(0x8031_003A)),
    });
    s.shot_end(&mut h, "bitlocker-4-managed")?;
    let mut now = suspended(true);
    if let Some(m) = &mut now.marker {
        m.this_boot = true;
    }
    h.starter_event(StarterEvent::BitLocker(Some(now)));
    h.starter_event(StarterEvent::BitLockerDone {
        step: BitLockerStep::Suspend,
        result: Ok(()),
    });
    s.shot(&mut h, "bitlocker-5-done")?;
    h.press(StarterAction::Home);
    s.shot(&mut h, "bitlocker-5-done-home")?;

    // Later: Resume, with Secure Boot as recorded and not.
    let mut h = guard(s, suspended(true));
    s.shot(&mut h, "bitlocker-6-resume")?;
    h.starter_event(StarterEvent::Computer(laptop(Os::Windows, false)));
    s.shot(&mut h, "bitlocker-6-resume-needs-secure-boot")?;
    let mut h = guard(s, suspended(true));
    h.press(StarterAction::Resume);
    h.starter_event(StarterEvent::BitLocker(Some(protected())));
    h.starter_event(StarterEvent::BitLockerDone {
        step: BitLockerStep::Resume,
        result: Ok(()),
    });
    s.shot(&mut h, "bitlocker-6-resumed")?;

    // Undone by a policy, another product, nothing read.
    let mut undone = suspended(true);
    undone.protection = Protection::On;
    let mut h = guard(s, undone);
    s.shot(&mut h, "bitlocker-7-undone")?;
    let mut h = guard(
        s,
        BitLocker {
            drive: "C:".into(),
            protection: Protection::None,
            marker: None,
        },
    );
    h.starter_event(StarterEvent::OtherEncryption(Some("VeraCrypt".into())));
    s.shot(&mut h, "bitlocker-7-veracrypt")?;
    let mut h = s.starter();
    h.starter_event(StarterEvent::Computer(laptop(Os::Windows, true)));
    h.starter_event(StarterEvent::BitLocker(None));
    h.press(StarterAction::BitLocker);
    s.shot(&mut h, "bitlocker-7-not-read")?;

    // Uninstall while suspended (§5.4).
    for (on, name) in [
        (true, "uninstall-suspended-resume"),
        (false, "uninstall-suspended-stays"),
    ] {
        let mut h = s.starter();
        h.starter_event(StarterEvent::Computer(laptop(Os::Windows, on)));
        h.starter_event(StarterEvent::BitLocker(Some(suspended(true))));
        h.starter_event(StarterEvent::Installed(Some(Installed {
            version: "0.2.0".into(),
            items: vec![(
                InstallItem::App,
                r"%LOCALAPPDATA%\Programs\Faraday Starter\Faraday Starter.exe".into(),
            )],
        })));
        h.press(StarterAction::Installed);
        h.press(StarterAction::Uninstall);
        s.shot(&mut h, name)?;
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
    let mut h = guard(s, protected());
    h.starter_event(StarterEvent::Concerns(vec![
        concern(Kind::RemoteSession, "", "session"),
        concern(Kind::Kvm, "PiKVM V4", "drm:card0-HDMI-A-1"),
    ]));
    s.shot(&mut h, "concern-bitlocker-held")?;
    let mut h = s.starter();
    h.starter_event(StarterEvent::BootEntry(true));
    s.shot_end(&mut h, "home-remove-boot-entry")?;
    h.press(StarterAction::RemoveEntry);
    h.starter_event(StarterEvent::BootEntryRemoved(Err(Reason::Declined)));
    s.shot_end(&mut h, "home-remove-boot-entry-declined")?;
    Ok(())
}

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
    h.starter_event(StarterEvent::BitLocker(Some(BitLocker {
        drive: "C:".into(),
        protection: Protection::None,
        marker: None,
    })));
    s.shot(&mut h, "home-windows-stick-secure-boot-off")?;
    let mut h = s.starter();
    h.starter_event(StarterEvent::Computer(laptop(Os::Windows, true)));
    h.starter_event(StarterEvent::Disks(vec![stick(
        "sde",
        "6.6.84-faraday-0.2.0",
    )]));
    h.starter_event(StarterEvent::BitLocker(Some(BitLocker {
        drive: "C:".into(),
        protection: Protection::On,
        marker: None,
    })));
    s.shot(&mut h, "home-windows-protected")?;
    h.starter_event(StarterEvent::Computer(laptop(Os::Windows, false)));
    h.starter_event(StarterEvent::Disks(Vec::new()));
    h.starter_event(StarterEvent::BitLocker(Some(suspended(true))));
    s.shot(&mut h, "home-suspended-secure-boot-off")?;
    h.starter_event(StarterEvent::Computer(laptop(Os::Windows, true)));
    s.shot(&mut h, "home-suspended-secure-boot-on")?;
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
    h.starter_event(StarterEvent::BitLocker(Some(BitLocker {
        drive: "C:".into(),
        protection: Protection::On,
        marker: None,
    })));
    h.press(StarterAction::Start);
    s.shot(&mut h, "start-windows-protected")?;
    h.starter_event(StarterEvent::BitLocker(Some(suspended(true))));
    s.shot(&mut h, "start-windows-suspended")?;
    s.shot_end(&mut h, "start-windows-suspended-end")?;
    h.press(StarterAction::Restart);
    s.shot(&mut h, "start-restart-sheet")?;
    // A maker with a page of its own (§7.6).
    let mut h = s.starter();
    let mut c = laptop(Os::Windows, true);
    c.maker = "Dell Inc.".into();
    h.starter_event(StarterEvent::Computer(c));
    h.starter_event(StarterEvent::BitLocker(Some(suspended(true))));
    h.press(StarterAction::Start);
    s.shot(&mut h, "start-maker")?;
    h.press(StarterAction::Restart);
    s.shot(&mut h, "start-maker-restart-sheet")?;

    // Install, Installed, About, BitLocker.
    let mut h = s.starter();
    h.press(StarterAction::Install);
    s.shot(&mut h, "install")?;
    h.press(StarterAction::InstallDesktop);
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
            (InstallItem::Desktop, "~/.local/bin/faraday-desktop".into()),
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
    guard_shots(s)?;

    // The desktop app's own Transfer and Learn.
    let mut h = s.starter();
    h.press(StarterAction::Transfer);
    s.shot(&mut h, "transfer")?;
    let mut h = s.starter();
    h.press(StarterAction::Learn);
    s.shot(&mut h, "learn")?;
    sketch_shots(s)?;
    step_shots(s)
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
        let max = h.held().learn.max;
        // The last offset the line is still in view at: it is at the
        // top then.
        let (mut seen, mut at, mut y) = (false, 0.0, 0.0);
        while y <= max {
            h.held().learn.scroll = y;
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
        h.held().learn.scroll = (at - above).max(0.0);
        s.shot(&mut h, name)?;
    }
    Ok(())
}

/// A guide page at its top, its first steps in circles
/// (`docs/COMPANION.md` §4.7): HP untouched, Dell with its first three
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
