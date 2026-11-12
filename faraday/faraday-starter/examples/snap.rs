//! Renders each Starter page in its states to PNG, for review
//! (`just faraday-starter-snap WxH`).
//!
//! ```text
//! cargo run -p faraday-starter --example snap -- [--tour] WxH[@DPI] OUT_DIR [THEME]
//! ```
//!
//! The shell's reports are made up here: a laptop, sticks and disks, the
//! found images. Each picture is `OUT_DIR/<name>.png`, named for the page and
//! its state. `--tour` draws the website tour's picture of the Starter instead: Home after a
//! stick is made, one stick attached throughout.
//!
//! `FARADAY_SNAP_TEXT=large|larger` draws them at that text size
//! (`docs/TEXT-SIZE.md` §7), and lists in `OUT_DIR/cut.txt` each picture
//! with text cut off at a side of its clip or drawn over other text.

use std::path::Path;
use std::process::ExitCode;

use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};
use faraday_starter::api::{
    BitLocker, BitLockerStep, BootFile, BootSigner, Bus, Computer, Concern, DevImage, Disk,
    FaradayStick, FoundImage, ImageFinding, InUse, InstallItem, Installed, Kind, Mode, Os,
    Protection, Reason, RecoveryKey, StarterEvent, Volume, WriteStage,
};
use faraday_starter::decide::Target;
use faraday_starter::notes::Notes;
use faraday_starter::{StartGuide, Starter, StarterAction};

/// Whether target `t`, `g` before any neighbour cut it back, is counted
/// as under the `floor` (`docs/M3-CHECKS.md` §2.2): it is when a side is
/// short and a neighbour trimmed that side; a side only its clip trims
/// is not counted.
fn short(t: faraday_draw::geom::Rect, g: faraday_draw::geom::Rect, floor: i32) -> bool {
    (t.w + 1 < floor && t.w < g.w) || (t.h + 1 < floor && t.h < g.h)
}

/// A token for the PDFs these snapshots make, counting nothing. Cheap.
#[expect(
    clippy::disallowed_methods,
    reason = "a mint: the Starter's snapshots, which draw nothing"
)]
fn compute() -> faraday_crypto::work::Compute {
    faraday_crypto::work::Compute::new(|_| {})
}

const GB: u64 = 1_000_000_000;

/// The stick image's size: 193 MiB and 20 KiB (docs/STARTER.md §4.2).
const STICK_IMAGE: u64 = 202_395_648;
/// The Pi card image's size: 196 MiB.
const CARD_IMAGE: u64 = 205_520_896;

/// The release images the finder found in Downloads, as the screens
/// after Home take them: the stick's and the Pi card's.
fn found_images() -> Vec<ImageFinding> {
    let file = |name: &str, size: u64, build: &str, a: &str, b: &str, board: &str| {
        ImageFinding::Found(FoundImage {
            path: format!("/Downloads/{name}"),
            name: name.into(),
            xz_sha256: a.repeat(8),
            img_sha256: b.repeat(8),
            inner_size: size,
            build_id: build.into(),
            board: board.into(),
            panel: "any".into(),
            starts: Vec::new(),
            boot_sha256: None,
        })
    };
    vec![
        file(
            "faraday-stick-0.2.0-x86_64-uefi.img.xz",
            STICK_IMAGE,
            "0.2.0",
            "3f9a0c1d",
            "8be41a07",
            "x86_64-uefi",
        ),
        file(
            "faraday-pi-0.2.0.img.xz",
            CARD_IMAGE,
            "0.2.0",
            "11d0e2a9",
            "c07b5e33",
            "pi3",
        ),
    ]
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
            boot_sha256: None,
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
        disk("sdd", "Generic", "Flash Disk", 128_000_000, &[]),
        stick("sde", "6.6.84-faraday-0.1.0+ef24784b48d8.test"),
        disk("sdf", "SanDisk", "Cruzer Blade", 16 * GB, &[]),
    ]
}

/// A press target a frame offers under the touch floor, with where it was
/// found (`docs/M3-CHECKS.md` §2).
#[derive(Debug, Clone)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "read by the touch-target test that includes this file"
    )
)]
pub(crate) struct Small {
    /// The picture's name.
    pub(crate) step: String,
    /// The page, and the open sheet.
    pub(crate) screen: String,
    pub(crate) sheet: Option<String>,
    pub(crate) action: faraday_core::Action,
    /// Where the press is taken, and the control as drawn: (w, h) and
    /// (x, y, w, h), pixels.
    pub(crate) target: (i32, i32),
    pub(crate) drawn: (i32, i32, i32, i32),
    /// The floor at this frame's scale, pixels.
    pub(crate) floor: i32,
}

pub(crate) struct Snap<'a> {
    pub(crate) out: &'a Path,
    pub(crate) size: (u16, u16),
    pub(crate) dpi: u16,
    pub(crate) theme: faraday_ui::Theme,
    /// The text size, and whether `cut.txt` is written.
    pub(crate) text: Option<faraday_ui::TextSize>,
    /// Whether the pictures are written; the touch-target test draws
    /// frames and writes none.
    pub(crate) pictures: bool,
    /// The targets under the floor each frame offers, collected when set
    /// (the test in `tests/all/targets.rs`).
    pub(crate) targets: Option<std::cell::RefCell<Vec<Small>>>,
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
        starter.starter_event(StarterEvent::ImagesFound(found_images()));
        starter.starter_event(StarterEvent::Disks(Vec::new()));
        starter.set_theme(self.theme);
        if let Some(text) = self.text {
            starter.set_text_size(text);
        }
        starter
    }

    /// Adds to `cut.txt` each line of text the frame, picture `name`,
    /// cuts off at a side of its clip or draws over other text, as
    /// Faraday's snapshots do (`docs/TEXT-SIZE.md` §7).
    fn note_cut(&self, starter: &mut Starter, name: &str) -> Result<(), String> {
        use faraday_draw::canvas::InkKind;
        use std::io::Write;
        let mut texts = Vec::new();
        let mut cut = Vec::new();
        for i in starter.drawn_ink() {
            if let InkKind::Text { text, cut: c, .. } = i.kind {
                if c {
                    cut.push(format!("text {text:?} cut at a side"));
                }
                texts.push((text, (i.rect.x, i.rect.y, i.rect.w, i.rect.h)));
            }
        }
        for (i, (a, ra)) in texts.iter().enumerate() {
            for (b, rb) in &texts[i + 1..] {
                let marks = a.chars().count() < 3 && b.chars().count() < 3;
                if (a == b && ra == rb) || marks {
                    continue;
                }
                let w = (ra.0 + ra.2).min(rb.0 + rb.2) - ra.0.max(rb.0);
                let h = (ra.1 + ra.3).min(rb.1 + rb.3) - ra.1.max(rb.1);
                if w > 1 && h > 1 {
                    cut.push(format!("text {a:?} drawn over {b:?} by {w} × {h} px"));
                }
            }
        }
        if cut.is_empty() {
            return Ok(());
        }
        let path = self.out.join("cut.txt");
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        for c in cut {
            writeln!(f, "{name}: {c}").map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// The targets of the frame on screen, picture `name`, under the touch
    /// floor and not cut by a clip (`docs/M3-CHECKS.md` §2.2), at the
    /// frame's density alone, Standard text.
    fn small(&self, starter: &Starter, name: &str) -> Vec<Small> {
        let f = (f32::from(self.dpi.max(1)) / 160.0).max(0.5);
        let floor = (faraday_draw::tokens::TOUCH * f).round() as i32;
        starter
            .drawn_targets()
            .into_iter()
            .filter(|(_, t, _, cut, g)| !cut && short(*t, *g, floor))
            .map(|(action, t, d, _, _)| Small {
                step: name.to_string(),
                screen: format!("{:?}", starter.page),
                sheet: starter.sheet.map(|s| format!("{s:?}")),
                action,
                target: (t.w, t.h),
                drawn: (d.x, d.y, d.w, d.h),
                floor,
            })
            .collect()
    }

    fn shot(&self, starter: &mut Starter, name: &str) -> Result<(), String> {
        while starter.poll_command().is_some() {}
        starter.event(Event::Tick { now_ms: 10_000 });
        starter.held().settle();
        if let Some(demo) = starter.demo() {
            demo.app.settle();
        }
        while starter.poll_command().is_some() {}
        while starter.poll_demo_command().is_some() {}
        let _ = starter.frame();
        if let Some(all) = &self.targets {
            all.borrow_mut().extend(self.small(starter, name));
        }
        let frame = starter.frame();
        if !self.pictures {
            return Ok(());
        }
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
        if self.text.is_some() {
            self.note_cut(starter, name)?;
        }
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

/// What to write's Boot file, "One I signed" (§4.5): the files in
/// Downloads, one refused, one taken, then Confirm.
fn boot_file_shots(s: &Snap, h: &mut Starter) -> Result<(), String> {
    const DOWNLOADS: &str = "/media/stick/Downloads";
    h.press(StarterAction::BootChoice(true));
    let files = vec![
        format!("{DOWNLOADS}/BOOTX64-signed.EFI"),
        format!("{DOWNLOADS}/BOOTX64.EFI"),
    ];
    h.starter_event(StarterEvent::BootFiles {
        folder: DOWNLOADS.into(),
        files: files.clone(),
    });
    s.shot_end(h, "stick-2-what-boot-file-list")?;
    h.press(StarterAction::BootFile(1));
    s.shot_end(h, "stick-2-what-boot-file-checking")?;
    h.starter_event(StarterEvent::BootFileChecked {
        path: files[1].clone(),
        result: Err(Reason::BootFileUnsigned),
    });
    s.shot_end(h, "stick-2-what-boot-file-refused")?;
    h.press(StarterAction::BootFile(0));
    h.starter_event(StarterEvent::BootFileChecked {
        path: files[0].clone(),
        result: Ok((
            BootFile {
                path: files[0].clone(),
                sha256: "3f0a9c2e".repeat(8),
            },
            BootSigner {
                subject: "Faraday db".into(),
                fingerprint: "5a1c0e7b2d9f4c3a".repeat(4),
            },
        )),
    });
    s.shot_end(h, "stick-2-what-boot-file-signed")?;
    h.press(StarterAction::Continue);
    s.shot_end(h, "stick-3-confirm-boot-file-signed")?;
    h.press(StarterAction::Back);
    Ok(())
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
pub(crate) fn run(s: &Snap) -> Result<(), String> {
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
    // The Start page's own button, with a stick attached.
    h.press(StarterAction::Cancel);
    h.press(StarterAction::Start);
    s.shot_end(&mut h, "start-windows-stick-secure-boot-off-end")?;
    h.press(StarterAction::RestartStick);
    s.shot(&mut h, "start-windows-restart-stick-sheet")?;
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
    // For which computer?, with a stand-in Apple silicon image found too.
    let mut images = found_images();
    if let Some(ImageFinding::Found(stick)) = images.first().cloned() {
        images.push(ImageFinding::Found(FoundImage {
            path: "/Downloads/faraday-stick-0.2.0-arm64-apple.img.xz".into(),
            name: "faraday-stick-0.2.0-arm64-apple.img.xz".into(),
            ..stick
        }));
    }
    h.starter_event(StarterEvent::ImagesFound(images));
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
    // The Name field, a name typed in lower case (docs/STICK-IDENTITY.md
    // §6), then taken away again.
    h.press(StarterAction::Name);
    for c in "my stick".chars() {
        h.event(Event::Key(Key::Char(c)));
    }
    s.shot_end(&mut h, "stick-2-what-name")?;
    h.press(StarterAction::Name);
    for _ in 0.."my stick".len() {
        h.event(Event::Key(Key::Backspace));
    }
    h.press(StarterAction::Back);
    let n = StarterAction::Disk(h.disk_number("sde"));
    h.press(n);
    h.press(StarterAction::Mode(Mode::KeepData));
    s.shot(&mut h, "stick-2-what-faraday")?;
    boot_file_shots(s, &mut h)?;
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
            boot_sha256: None,
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
        size: STICK_IMAGE,
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
        done: 70_000_000,
        total: STICK_IMAGE,
    });
    s.shot(&mut h, "stick-4-writing")?;
    h.starter_event(StarterEvent::WriteProgress {
        stage: WriteStage::ReadingBack,
        done: 160_000_000,
        total: STICK_IMAGE,
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

    // Clear the whole stick first, ticked: What to write, Confirm and
    // Writing's clearing pass (docs/STICK-IDENTITY.md §4.2).
    let mut h = s.starter();
    h.starter_event(StarterEvent::Disks(many()));
    h.press(StarterAction::Make(Target::Stick));
    let n = StarterAction::Disk(h.disk_number("sdf"));
    h.press(n);
    h.press(StarterAction::Mode(Mode::Whole));
    h.press(StarterAction::ClearFirst);
    s.shot_end(&mut h, "stick-2-what-clear")?;
    h.press(StarterAction::Continue);
    s.shot_end(&mut h, "stick-3-confirm-clear")?;
    h.press(StarterAction::Write);
    h.starter_event(StarterEvent::WriteProgress {
        stage: WriteStage::Clearing,
        done: 3_000_000_000,
        total: 16_000_000_000,
    });
    s.shot(&mut h, "stick-4-clearing")?;

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

    // Found images (§3.2, §3.3): two for one card, and one for the other.
    let found = |name: &str, img: &str, board: &str| {
        faraday_starter::api::ImageFinding::Found(faraday_starter::api::FoundImage {
            path: format!("/Downloads/{name}"),
            name: name.into(),
            xz_sha256: "aa".repeat(32),
            img_sha256: img.into(),
            inner_size: 1 << 30,
            build_id: "0.3.0".into(),
            board: board.into(),
            panel: "any".into(),
            starts: Vec::new(),
            boot_sha256: None,
        })
    };
    let wanted = vec![
        "faraday-stick-0.3.0-x86_64-uefi.img.xz".to_string(),
        "faraday-pi-0.3.0.img.xz".to_string(),
    ];
    let mut h = s.starter();
    h.starter_event(StarterEvent::Disks(many()));
    h.starter_event(StarterEvent::ImagesWanted(wanted.clone()));
    h.starter_event(StarterEvent::ImagesFound(vec![
        found("faraday-pi-0.3.0.img.xz", &"bb".repeat(32), "pi3"),
        found("faraday-pi-0.3.0-copy.img.xz", &"cc".repeat(32), "pi3"),
        faraday_starter::api::ImageFinding::OtherRelease {
            path: "/Downloads/faraday-stick-0.2.0-x86_64-uefi.img.xz".into(),
            name: "faraday-stick-0.2.0-x86_64-uefi.img.xz".into(),
            needs: "0.3.0".into(),
        },
    ]));
    h.press(StarterAction::Make(Target::Pi));
    s.shot(&mut h, "images-1-pi-two-found")?;
    h.press(StarterAction::Image(1));
    s.shot(&mut h, "images-2-pi-found-chosen")?;
    let mut h = s.starter();
    h.starter_event(StarterEvent::Disks(many()));
    h.starter_event(StarterEvent::ImagesWanted(wanted));
    h.starter_event(StarterEvent::ImagesFound(vec![found(
        "faraday-pi-0.3.0.img.xz",
        &"bb".repeat(32),
        "pi3",
    )]));
    h.press(StarterAction::Make(Target::Stick));
    s.shot(&mut h, "images-3-stick-to-download")?;

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
    // The ⋮ menu, and its Appearance page (`docs/THEMES.md` §14) in the
    // scheme the run is in, then with a choice made on it.
    let mut h = s.starter();
    h.press(StarterAction::Menu);
    s.shot(&mut h, "menu")?;
    h.press(StarterAction::Appearance);
    s.shot(&mut h, "appearance")?;
    for (words, name) in [
        ("frost,dark", "appearance-frost-dark"),
        (
            "bitcoin-orange,light,medium",
            "appearance-bitcoin-orange-medium",
        ),
        ("cobalt,dark,high", "appearance-cobalt-dark-high"),
    ] {
        let theme = faraday_ui::Theme::from_words(words).ok_or(words)?;
        h.press(StarterAction::Theme(theme));
        s.shot(&mut h, name)?;
    }
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
    codex32_template_shots(s)?;

    // The desktop app's own Transfer and Learn.
    let mut h = s.starter();
    h.press(StarterAction::Transfer);
    s.shot(&mut h, "transfer")?;
    let mut h = s.starter();
    h.press(StarterAction::Learn);
    s.shot(&mut h, "learn")?;
    destination_shots(s)?;
    demo_shots(s)?;
    step_shots(s)
}

/// The website tour's picture: a Faraday stick made on a Linux laptop
/// with one stick attached throughout (Which stick?, Confirm, the write,
/// Written), then Home with that stick, Faraday's now.
fn run_tour(s: &Snap) -> Result<(), String> {
    let blank = disk("sdb", "USB", "Flash Drive", 32 * GB, &[]);
    let mut h = s.starter();
    h.starter_event(StarterEvent::Disks(vec![blank.clone()]));
    to(&mut h, "sdb", Mode::Whole);
    h.press(StarterAction::Write);
    h.starter_event(StarterEvent::Written(Ok(())));
    let written = Disk {
        faraday: Some(FaradayStick {
            release: Some(format!("6.6.84-faraday-{}", env!("CARGO_PKG_VERSION"))),
            read: true,
            boot_sha256: None,
        }),
        ..blank
    };
    h.starter_event(StarterEvent::Disks(vec![written]));
    h.press(StarterAction::Home);
    s.shot(&mut h, "starter-stick")
}

/// The destinations (`docs/STARTER.md` §3.1d): Make and Tools, the ⋮
/// menu, Update in the bar, and a task page's bar with the way back.
fn destination_shots(s: &Snap) -> Result<(), String> {
    let mut h = s.starter();
    h.press(StarterAction::MakePage);
    s.shot(&mut h, "make")?;
    h.press(StarterAction::ToolsPage);
    s.shot(&mut h, "tools")?;
    h.starter_event(StarterEvent::Installed(Some(Installed {
        version: "0.1.0".into(),
        items: vec![(InstallItem::App, "~/.local/bin/faraday-starter".into())],
    })));
    h.starter_event(StarterEvent::BootEntry(true));
    h.press(StarterAction::Menu);
    s.shot(&mut h, "tools-menu")?;
    h.press(StarterAction::Home);
    s.shot(&mut h, "home-update-and-boot-entry")?;
    h.press(StarterAction::MakePage);
    h.starter_event(StarterEvent::Disks(vec![stick(
        "sde",
        "6.6.84-faraday-0.2.0",
    )]));
    s.shot(&mut h, "make-stick-attached")?;
    Ok(())
}

/// Faraday Demo inside the Starter's window (F126): opened from Home
/// under the top app bar with its host's strip, and Home again after the
/// way back.
fn demo_shots(s: &Snap) -> Result<(), String> {
    let mut h = s.starter();
    h.press(StarterAction::OpenDesktop);
    let app = faraday_desktop::desktop_app_in(faraday_core::Mode::Demo);
    h.show_demo(faraday_core::Host::new(app));
    s.shot(&mut h, "demo")?;
    h.press(StarterAction::Back);
    s.shot(&mut h, "demo-left")?;
    Ok(())
}

/// The codex32 blank templates (`docs/CODEX32.md` §11): the menu and its
/// choices, and each template's PDF written beside the pictures, for
/// `pdftoppm` to turn into the pages that are looked at.
fn codex32_template_shots(s: &Snap) -> Result<(), String> {
    use faraday_starter::TemplateChoice;
    use faraday_starter::template::{CodexCopy, Form};
    let mut h = s.starter();
    h.press(StarterAction::Templates);
    h.press(StarterAction::TemplateChoice(TemplateChoice::Form));
    s.shot(&mut h, "templates-codex32-form")?;
    h.press(StarterAction::TemplateForm(Form::Codex32));
    h.press(StarterAction::Back);
    s.shot(&mut h, "templates-codex32")?;
    h.press(StarterAction::TemplateChoice(TemplateChoice::Worksheets));
    h.press(StarterAction::TemplateWorksheets(true));
    s.shot(&mut h, "templates-codex32-worksheets")?;
    h.press(StarterAction::Back);
    h.press(StarterAction::TemplateChoice(TemplateChoice::Copy));
    s.shot(&mut h, "templates-codex32-copy")?;
    h.press(StarterAction::TemplateCodexCopy(CodexCopy::Both));
    h.press(StarterAction::Back);
    for (name, chars, worksheets) in [("48", 48, true), ("74", 74, true), ("127", 127, false)] {
        h.template.set_chars(chars);
        h.template.set_worksheets(worksheets);
        let pdf = h.template.pdf(&compute())?;
        let path = s.out.join(format!("template-codex32-{name}.pdf"));
        std::fs::write(&path, pdf).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("{}", path.display());
    }
    Ok(())
}

/// A start guide page at its top, its first steps in circles
/// (`docs/STARTER.md` §7).
fn step_shots(s: &Snap) -> Result<(), String> {
    let mut h = s.starter();
    h.press(StarterAction::StartGuide(StartGuide::Maker("maker-hp")));
    s.shot(&mut h, "steps-hp")
}

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().collect();
    let tour = match args.iter().position(|a| a == "--tour") {
        Some(i) => {
            args.remove(i);
            true
        }
        None => false,
    };
    if args.len() != 3 && args.len() != 4 {
        eprintln!("usage: snap [--tour] WxH[@DPI] OUT_DIR [THEME]");
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
    // THEME is a scheme's words, `<theme>[,<mode>[,<contrast>]]`
    // (`frost,dark,high`; a stored id alone, `nord`, as a settings file
    // of it opens); the default is Cobalt, Light, Standard.
    let theme = match args.get(3) {
        Some(id) => match faraday_ui::Theme::from_words(id) {
            Some(theme) => theme,
            None => {
                eprintln!("{id}: not a theme id");
                return ExitCode::from(2);
            }
        },
        None => faraday_ui::Theme::default(),
    };
    let text = match std::env::var("FARADAY_SNAP_TEXT") {
        Ok(words) => match faraday_ui::TextSize::from_id(&words) {
            Some(t) => Some(t),
            None => {
                eprintln!("FARADAY_SNAP_TEXT={words}: standard, large or larger");
                return ExitCode::from(2);
            }
        },
        Err(_) => None,
    };
    if text.is_some()
        && let Err(e) = std::fs::write(out.join("cut.txt"), "")
    {
        eprintln!("{}: {e}", out.display());
        return ExitCode::FAILURE;
    }
    let snap = Snap {
        out,
        size,
        dpi,
        theme,
        text,
        pictures: true,
        targets: None,
    };
    let done = if tour { run_tour(&snap) } else { run(&snap) };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
