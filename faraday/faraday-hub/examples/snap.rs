//! Renders each hub page in its states to PNG, for review
//! (`just faraday-hub-snap WxH`).
//!
//! ```text
//! cargo run -p faraday-hub --example snap -- WxH[@DPI] OUT_DIR
//! ```
//!
//! The shell's reports are made up here: a laptop, sticks and disks, a
//! payload. Each picture is `OUT_DIR/<name>.png`, named for the page and
//! its state.

use std::path::Path;
use std::process::ExitCode;

use faraday_hub::api::{
    BitLocker, Bus, Computer, DevImage, Disk, FaradayStick, HubEvent, InUse, InstallItem,
    Installed, Marker, Mode, Os, Payload, PayloadFile, Protection, Volume, WriteStage,
};
use faraday_hub::decide::Target;
use faraday_hub::{Hub, HubAction};
use osk_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

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
        maker: "LENOVO".into(),
        model: "ThinkPad T14 Gen 3".into(),
        family: "ThinkPad".into(),
        uefi: true,
        arm: false,
        secure_boot: Some(secure_boot),
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
    fn hub(&self) -> Hub {
        let mut hub = Hub::new();
        hub.event(Event::Display(DisplayInfo {
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
        hub.hub_event(HubEvent::Computer(laptop(Os::Linux, true)));
        hub.hub_event(HubEvent::Payload(Some(payload())));
        hub.hub_event(HubEvent::Disks(Vec::new()));
        hub
    }

    fn shot(&self, hub: &mut Hub, name: &str) -> Result<(), String> {
        while hub.poll_command().is_some() {}
        hub.event(Event::Tick { now_ms: 10_000 });
        hub.held().settle();
        while hub.poll_command().is_some() {}
        let frame = hub.frame();
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
    fn shot_end(&self, hub: &mut Hub, name: &str) -> Result<(), String> {
        let _ = hub.frame();
        for _ in 0..20 {
            hub.event(Event::Wheel {
                x: 10,
                y: 10,
                dy: 2000,
            });
            let _ = hub.frame();
        }
        self.shot(hub, name)
    }
}

fn to(hub: &mut Hub, id: &str, mode: Mode) {
    hub.press(HubAction::Make(Target::Stick));
    let n = HubAction::Disk(hub.disk_number(id));
    hub.press(n);
    hub.press(HubAction::Mode(mode));
    hub.press(HubAction::Continue);
}

fn suspended(was_on: bool) -> BitLocker {
    BitLocker {
        drive: "C:".into(),
        protection: Protection::Suspended,
        marker: Some(Marker {
            volumes: vec!["C:".into()],
            secure_boot_was_on: was_on,
        }),
    }
}

fn run(s: &Snap) -> Result<(), String> {
    // Home in each state.
    let mut h = s.hub();
    s.shot(&mut h, "home-no-stick")?;
    s.shot_end(&mut h, "home-no-stick-end")?;
    let mut h = s.hub();
    h.hub_event(HubEvent::Disks(vec![stick("sde", "6.6.84-faraday-0.2.0")]));
    s.shot(&mut h, "home-stick")?;
    let mut h = s.hub();
    h.hub_event(HubEvent::Computer(laptop(Os::Windows, true)));
    h.hub_event(HubEvent::Disks(vec![stick("sde", "6.6.84-faraday-0.2.0")]));
    h.hub_event(HubEvent::BitLocker(Some(BitLocker {
        drive: "C:".into(),
        protection: Protection::On,
        marker: None,
    })));
    s.shot(&mut h, "home-windows-protected")?;
    h.hub_event(HubEvent::Computer(laptop(Os::Windows, false)));
    h.hub_event(HubEvent::Disks(Vec::new()));
    h.hub_event(HubEvent::BitLocker(Some(suspended(true))));
    s.shot(&mut h, "home-suspended-secure-boot-off")?;
    h.hub_event(HubEvent::Computer(laptop(Os::Windows, true)));
    s.shot(&mut h, "home-suspended-secure-boot-on")?;
    let mut h = s.hub();
    let mut arm = laptop(Os::Windows, true);
    arm.arm = true;
    arm.maker = "Microsoft Corporation".into();
    arm.model = "Surface Laptop 7".into();
    h.hub_event(HubEvent::Computer(arm));
    s.shot(&mut h, "home-arm")?;

    // Make a Faraday stick.
    let mut h = s.hub();
    h.press(HubAction::Make(Target::Stick));
    s.shot(&mut h, "stick-1-which-none")?;
    h.hub_event(HubEvent::Disks(many()));
    s.shot(&mut h, "stick-1-which")?;
    s.shot_end(&mut h, "stick-1-which-end")?;
    let n = HubAction::Disk(h.disk_number("sdb"));
    h.press(n);
    s.shot(&mut h, "stick-2-what-plain")?;
    h.press(HubAction::Back);
    let n = HubAction::Disk(h.disk_number("sde"));
    h.press(n);
    h.press(HubAction::Mode(Mode::KeepData));
    s.shot(&mut h, "stick-2-what-faraday")?;
    let mut h = s.hub();
    h.hub_event(HubEvent::Disks(vec![stick("sde", "6.6.84-faraday-0.2.0")]));
    h.press(HubAction::Make(Target::Stick));
    let n = HubAction::Disk(h.disk_number("sde"));
    h.press(n);
    s.shot(&mut h, "stick-2-what-same-build")?;

    let mut h = s.hub();
    h.hub_event(HubEvent::Disks(many()));
    to(&mut h, "sdf", Mode::Whole);
    s.shot(&mut h, "stick-3-confirm")?;
    s.shot_end(&mut h, "stick-3-confirm-end")?;
    let mut other = many();
    other[4].serial = "OTHER".into();
    h.hub_event(HubEvent::Disks(other));
    s.shot_end(&mut h, "stick-3-confirm-not-the-one")?;
    let mut h = s.hub();
    h.hub_event(HubEvent::Disks(vec![stick("sde", "6.6.84-faraday-0.3.0")]));
    to(&mut h, "sde", Mode::KeepData);
    s.shot_end(&mut h, "stick-3-confirm-keep-newer")?;
    let mut h = s.hub();
    h.hub_event(HubEvent::Disks(many()));
    h.hub_event(HubEvent::DevImage(DevImage {
        path: "out/stick/faraday-x86_64-uefi-dev.img".into(),
        sha256: "9c1e44d0".repeat(8),
        size: 3_900_000_000,
        boot_only: false,
        build_id: Some("0.2.0+4d0680b1a2b3.dev".into()),
    }));
    to(&mut h, "sdf", Mode::Whole);
    s.shot_end(&mut h, "stick-3-confirm-dev-image")?;

    let mut h = s.hub();
    h.hub_event(HubEvent::Disks(many()));
    to(&mut h, "sdf", Mode::Whole);
    h.press(HubAction::Write);
    h.hub_event(HubEvent::WriteProgress {
        stage: WriteStage::Writing,
        done: 1_400_000_000,
        total: 3_900_000_000,
    });
    s.shot(&mut h, "stick-4-writing")?;
    h.hub_event(HubEvent::WriteProgress {
        stage: WriteStage::ReadingBack,
        done: 3_100_000_000,
        total: 3_900_000_000,
    });
    s.shot(&mut h, "stick-4-reading-back")?;
    h.hub_event(HubEvent::Written(Ok(())));
    s.shot(&mut h, "stick-5-done")?;
    let mut h = s.hub();
    h.hub_event(HubEvent::Disks(many()));
    to(&mut h, "sdf", Mode::Whole);
    h.press(HubAction::Write);
    h.hub_event(HubEvent::Written(Err(
        "Read back did not match at 1.2 GB".into()
    )));
    s.shot(&mut h, "stick-5-failed")?;

    // Make a Pi card.
    let mut h = s.hub();
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
    h.hub_event(HubEvent::Disks(disks));
    h.press(HubAction::Make(Target::Pi));
    s.shot(&mut h, "pi-1-which")?;
    let n = HubAction::Disk(h.disk_number("mmcblk0"));
    h.press(n);
    s.shot(&mut h, "pi-2-what")?;

    // Start this computer from the stick.
    let mut h = s.hub();
    h.press(HubAction::Start);
    s.shot(&mut h, "start-linux")?;
    s.shot_end(&mut h, "start-linux-end")?;
    let mut h = s.hub();
    let mut c = laptop(Os::Linux, false);
    c.tpm_unlock = true;
    h.hub_event(HubEvent::Computer(c));
    h.press(HubAction::Start);
    s.shot(&mut h, "start-linux-secure-boot-off-tpm")?;
    let mut h = s.hub();
    h.hub_event(HubEvent::Computer(laptop(Os::Windows, true)));
    h.hub_event(HubEvent::BitLocker(Some(BitLocker {
        drive: "C:".into(),
        protection: Protection::On,
        marker: None,
    })));
    h.press(HubAction::Start);
    s.shot(&mut h, "start-windows-protected")?;
    h.hub_event(HubEvent::BitLocker(Some(suspended(true))));
    s.shot(&mut h, "start-windows-suspended")?;
    s.shot_end(&mut h, "start-windows-suspended-end")?;
    h.press(HubAction::Restart);
    s.shot(&mut h, "start-restart-sheet")?;

    // Install, Installed, About, BitLocker.
    let mut h = s.hub();
    h.press(HubAction::Install);
    s.shot(&mut h, "install")?;
    h.press(HubAction::InstallDesktop);
    h.press(HubAction::InstallNow);
    s.shot(&mut h, "install-installing")?;
    h.hub_event(HubEvent::InstallDone(Ok("~/.local/bin/faraday".into())));
    s.shot(&mut h, "install-done")?;
    let mut h = s.hub();
    h.hub_event(HubEvent::Installed(Some(Installed {
        version: "0.2.0".into(),
        items: vec![
            (InstallItem::App, "~/.local/bin/faraday".into()),
            (InstallItem::Desktop, "~/.local/bin/faraday-desktop".into()),
            (
                InstallItem::Menu,
                "~/.local/share/applications/app.faradayoffline.Faraday.desktop".into(),
            ),
            (
                InstallItem::Icon,
                "~/.local/share/icons/hicolor/256x256/apps/app.faradayoffline.Faraday.png".into(),
            ),
        ],
    })));
    s.shot(&mut h, "home-installed")?;
    h.press(HubAction::Installed);
    s.shot(&mut h, "installed")?;
    h.press(HubAction::Uninstall);
    s.shot(&mut h, "installed-uninstall-sheet")?;
    let mut h = s.hub();
    h.press(HubAction::About);
    s.shot(&mut h, "about")?;
    s.shot_end(&mut h, "about-end")?;
    let mut h = s.hub();
    h.hub_event(HubEvent::Computer(laptop(Os::Windows, false)));
    h.hub_event(HubEvent::BitLocker(Some(suspended(true))));
    h.press(HubAction::BitLocker);
    s.shot(&mut h, "bitlocker")?;

    // The desktop app's own Transfer and Learn.
    let mut h = s.hub();
    h.press(HubAction::Transfer);
    s.shot(&mut h, "transfer")?;
    let mut h = s.hub();
    h.press(HubAction::Learn);
    s.shot(&mut h, "learn")?;
    Ok(())
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
