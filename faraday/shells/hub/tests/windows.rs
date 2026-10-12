//! The hub app on Windows, from Windows' answers as recorded
//! (`tests/windows/`): the disks a person is offered and refused, this
//! computer's facts, the elevated writer's arguments and what it reports
//! over its pipe, and what install and uninstall leave on the computer.
//! None of it calls Windows, so it runs on any system.

use std::collections::BTreeMap;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use faraday_core::Program;
use faraday_hub::api::{
    Bus, InUse, InstallItem, Mode, Os, Reason, Source, StickFault, WriteOrder, WriteStage,
};
use faraday_hub::decide::{Refusal, Target, offers};
use faraday_hub_shell::stick;
use faraday_hub_shell::windows::disks::{Found, Snapshot};
use faraday_hub_shell::windows::elevated::{Elevated, command_line, split};
use faraday_hub_shell::windows::install::{self, Data, Dirs, Left, Registry, UNINSTALL_KEY};
use faraday_hub_shell::windows::pipe::{self, Message};
use faraday_hub_shell::windows::registry::{self, Facts};
use faraday_hub_shell::windows::shortcut;

/// The stick image's size: 81 MiB.
const IMAGE: u64 = 84_955_136;

fn laptop() -> Snapshot {
    let text =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/windows/laptop.txt"))
            .expect("a recorded answer");
    Snapshot::parse(&text).expect("a well-formed recording")
}

fn found(s: &Snapshot) -> Vec<Found> {
    s.disks()
}

fn disk<'a>(list: &'a [Found], device: &str) -> &'a Found {
    list.iter()
        .find(|f| f.disk.device == device)
        .unwrap_or_else(|| panic!("{device} listed"))
}

fn offered(s: &Snapshot, target: Target) -> Vec<(String, Option<Refusal>)> {
    let disks: Vec<_> = found(s).into_iter().map(|f| f.disk).collect();
    offers(&disks, target, Some(IMAGE))
        .into_iter()
        .map(|o| (o.disk.device.clone(), o.refused))
        .collect()
}

#[test]
fn usb_sticks_are_offered_and_the_computers_own_disks_are_not() {
    let s = laptop();
    assert_eq!(
        offered(&s, Target::Stick),
        vec![
            ("Disk 2".to_string(), None),
            ("Disk 4".to_string(), Some(Refusal::TooSmall(IMAGE))),
            ("Disk 1".to_string(), None),
        ]
    );
    let list = found(&s);
    // The NVMe disk Windows runs from, and a second SATA disk: never
    // offered, and the first is the system's.
    assert_eq!(disk(&list, "Disk 0").disk.bus, Bus::Other);
    assert_eq!(disk(&list, "Disk 0").disk.in_use, Some(InUse::System));
    assert_eq!(disk(&list, "Disk 6").disk.bus, Bus::Other);
    assert_eq!(disk(&list, "Disk 6").disk.in_use, None);
    // The card reader with no card in it is not listed.
    assert!(list.iter().all(|f| f.disk.device != "Disk 5"));
    let stick = &disk(&list, "Disk 1").disk;
    assert_eq!(stick.id, r"\\.\PhysicalDrive1");
    assert_eq!(
        (
            stick.vendor.as_str(),
            stick.model.as_str(),
            stick.serial.as_str()
        ),
        ("Acme", "Stick", "AAA111")
    );
    assert_eq!(stick.size, 16_008_609_792);
    assert_eq!(stick.volumes.len(), 1);
    assert_eq!(stick.volumes[0].label.as_deref(), Some("HOLIDAY"));
    assert!(stick.volumes[0].files);
    assert!(stick.faraday.is_none());
}

#[test]
fn an_sd_card_in_the_computers_reader_is_offered_for_a_pi_card_only() {
    let s = laptop();
    let pi = offered(&s, Target::Pi);
    assert!(pi.contains(&("Disk 3".to_string(), None)));
    assert!(
        !offered(&s, Target::Stick)
            .iter()
            .any(|(d, _)| d == "Disk 3")
    );
    assert_eq!(disk(&found(&s), "Disk 3").disk.bus, Bus::SdReader);
}

#[test]
fn a_disk_holding_windows_or_a_page_file_is_refused_as_the_systems() {
    // Windows started from the USB stick (Windows To Go), and a page
    // file on the small stick.
    let mut s = laptop();
    s.windows = String::from(r"e:\WINDOWS");
    s.page_files = vec![String::from(r"\??\H:\pagefile.sys")];
    let offers = offered(&s, Target::Stick);
    assert!(offers.contains(&("Disk 1".to_string(), Some(Refusal::InUse(InUse::System)))));
    assert!(offers.contains(&("Disk 4".to_string(), Some(Refusal::InUse(InUse::System)))));
    assert!(offers.contains(&("Disk 2".to_string(), None)));
    // The system partition on its own marks its disk too.
    let list = found(&s);
    assert_eq!(disk(&list, "Disk 0").disk.in_use, Some(InUse::System));
}

#[test]
fn every_disk_is_refused_when_the_windows_folder_cannot_be_placed() {
    let mut s = laptop();
    s.windows = String::from(r"X:\Windows");
    for (_, refused) in offered(&s, Target::Pi) {
        assert_eq!(refused, Some(Refusal::InUse(InUse::System)));
    }
}

#[test]
fn a_faraday_stick_is_known_by_its_partitions_and_read_from_its_boot_volume_alone() {
    let list = found(&laptop());
    let stick = disk(&list, "Disk 2");
    assert!(stick.disk.faraday.is_some());
    assert_eq!(
        stick.boot.as_deref(),
        Some(r"\\?\Volume{00000000-0000-0000-0000-000000000020}\")
    );
    let labels: Vec<_> = stick
        .disk
        .volumes
        .iter()
        .map(|v| v.label.as_deref())
        .collect();
    assert_eq!(labels, [Some("OSKBOOT"), Some("OSKDATA")]);
    // An MBR stick whose volumes happen to carry those labels is not one.
    assert!(disk(&list, "Disk 1").disk.faraday.is_none());
    // Its version, read as from the boot volume's root.
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("hub-windows-boot");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("EFI/BOOT")).expect("folder");
    let mut kernel = vec![0u8; 4096];
    kernel.extend(b"\x006.6.84-faraday-0.2.0+4d0680b1a2b3 (buildroot@buildroot)\x00");
    fs::write(root.join("EFI/BOOT/BOOTX64.EFI"), kernel).expect("file");
    assert_eq!(
        stick::release_at(&root).as_deref(),
        Some("6.6.84-faraday-0.2.0+4d0680b1a2b3")
    );
}

#[test]
fn the_writer_checks_the_same_disk_it_was_shown() {
    let s = laptop();
    let (id, bus) = s.identity(1).expect("disk 1");
    assert_eq!(id.device, "Disk 1");
    assert_eq!(id.serial, "AAA111");
    assert_eq!(id.size, 16_008_609_792);
    assert_eq!(id.in_use, None);
    assert_eq!(bus, Bus::Usb);
    assert!(s.identity(5).is_none());
    assert_eq!(s.identity(0).expect("disk 0").0.in_use, Some(InUse::System));
}

#[test]
fn a_recording_reads_back_as_it_was_written() {
    let s = laptop();
    assert_eq!(Snapshot::parse(&s.record()), Ok(s));
    assert!(Snapshot::parse("disk 1\n  descriptor 0g\n").is_err());
    assert!(Snapshot::parse("  label OSKBOOT\n").is_err());
}

fn sz(s: &str) -> Vec<u8> {
    s.encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect()
}

#[test]
fn this_computers_facts_come_from_the_registry_and_the_firmware() {
    let facts = Facts {
        manufacturer: Some(sz("Acme Computers ")),
        product: Some(sz("Book 14")),
        family: Some(sz("Book")),
        secure_boot: Some(1u32.to_le_bytes().to_vec()),
        firmware: Some(registry::FIRMWARE_UEFI),
        machine: Some(0x8664),
    };
    let c = registry::computer(&facts);
    assert_eq!(c.os, Os::Windows);
    assert_eq!(
        (c.maker.as_str(), c.model.as_str(), c.family.as_str()),
        ("Acme Computers", "Book 14", "Book")
    );
    assert!(c.uefi);
    assert!(!c.arm);
    assert_eq!(c.secure_boot, Some(true));
    // Secure Boot off; an ARM laptop; a value that cannot be read.
    let off = Facts {
        secure_boot: Some(0u32.to_le_bytes().to_vec()),
        machine: Some(0xaa64),
        ..facts.clone()
    };
    let c = registry::computer(&off);
    assert_eq!(c.secure_boot, Some(false));
    assert!(c.arm);
    let unknown = Facts {
        secure_boot: None,
        ..facts.clone()
    };
    assert_eq!(registry::computer(&unknown).secure_boot, None);
    // Started in legacy BIOS mode: no UEFI, no Secure Boot.
    let bios = Facts {
        firmware: Some(1),
        secure_boot: None,
        ..facts
    };
    let c = registry::computer(&bios);
    assert!(!c.uefi);
    assert_eq!(c.secure_boot, Some(false));
    // The page files' list, as the registry holds it.
    let mut multi = sz(r"\??\C:\pagefile.sys");
    multi.extend(sz(r"\??\D:\swapfile.sys"));
    multi.extend([0, 0]);
    assert_eq!(
        registry::multi_sz(&multi),
        [r"\??\C:\pagefile.sys", r"\??\D:\swapfile.sys"]
    );
}

fn order(source: Source, serial: &str, mode: Mode) -> WriteOrder {
    WriteOrder {
        source,
        disk: String::from(r"\\.\PhysicalDrive2"),
        device: String::from("Disk 2"),
        serial: serial.to_string(),
        size: 8_004_304_896,
        mode,
    }
}

const SUM: &str = "7b89e3875dc5832ecbbfb9509cf5517200c8e48762c2d983beeaf01ed2094b56";
const PIPE: &str = r"\\.\pipe\faraday-hub-1f2c-18c4a2b9d0e1f";

#[test]
fn the_elevated_writer_reads_back_the_order_it_was_started_with() {
    let payload = Elevated {
        pipe: PIPE.into(),
        order: order(
            Source::Payload {
                name: "faraday-stick-0.2.0-x86_64-uefi.img.xz".into(),
                inner_sha256: SUM.into(),
            },
            "AAA 111 \"x\\\"",
            Mode::Whole,
        ),
        image_size: None,
    };
    let image = Elevated {
        pipe: PIPE.into(),
        order: order(
            Source::Image {
                path: r"D:\work dir\out\stick dev\faraday-x86_64-uefi-dev.img".into(),
                sha256: SUM.into(),
            },
            "",
            Mode::KeepData,
        ),
        image_size: Some(IMAGE),
    };
    for e in [payload, image] {
        let line = command_line(&e.args());
        assert_eq!(Elevated::parse(&split(&line)), Ok(e));
    }
    // Backslashes before a quote and at the end survive.
    let tricky = vec![
        r#"a\\"b"#.to_string(),
        r"c:\dir\".to_string(),
        String::new(),
    ];
    assert_eq!(split(&command_line(&tricky)), tricky);
}

#[test]
fn a_malformed_order_is_refused_before_anything_is_opened() {
    let good = Elevated {
        pipe: PIPE.into(),
        order: order(
            Source::Payload {
                name: "faraday-stick-0.2.0-x86_64-uefi.img.xz".into(),
                inner_sha256: SUM.into(),
            },
            "FFF333",
            Mode::Whole,
        ),
        image_size: None,
    }
    .args();
    assert!(Elevated::parse(&good).is_ok());
    let with = |key: &str, value: &str| {
        let mut a = good.clone();
        let i = a.iter().position(|k| k == key).expect("key");
        a[i + 1] = value.to_string();
        a
    };
    let refused = [
        with("--disk", r"\\.\C:"),
        with("--disk", r"\\.\PhysicalDrive02"),
        with("--disk", r"C:\Windows"),
        with("--device", "Disk 3"),
        with("--pipe", r"\\.\pipe\someone-else"),
        with("--pipe", r"\\server\pipe\faraday-hub-1"),
        with("--size", "0"),
        with("--size", "-1"),
        with("--size", "16 GB"),
        with("--mode", "everything"),
        with("--sha256", "abc"),
        with("--sha256", &SUM.to_uppercase()),
        with("--payload", r"..\..\Windows\System32\config\SAM"),
        with("--payload", "a/b"),
        with("--serial", "two\nlines"),
        good[1..].to_vec(),
        [
            good.clone(),
            vec!["--disk".into(), r"\\.\PhysicalDrive1".into()],
        ]
        .concat(),
        [good.clone(), vec!["--image".into(), r"C:\x.img".into()]].concat(),
        [good.clone(), vec!["--extra".into(), "1".into()]].concat(),
        good[..good.len() - 1].to_vec(),
        good.iter()
            .map(|a| {
                if a == "--image" {
                    "--payload".into()
                } else {
                    a.clone()
                }
            })
            .filter(|a| a != "--sha256")
            .collect(),
    ];
    for args in refused {
        assert!(Elevated::parse(&args).is_err(), "{args:?} was taken");
    }
    // An image needs its full path and its size.
    let image = |path: &str, size: Option<&str>| {
        let mut a = good[..good.len() - 4].to_vec();
        a.extend(["--image".into(), path.to_string()]);
        if let Some(s) = size {
            a.extend(["--image-size".into(), s.to_string()]);
        }
        a.extend(["--sha256".into(), SUM.into()]);
        a
    };
    assert!(Elevated::parse(&image(r"C:\out\a.img", Some("512"))).is_ok());
    assert!(Elevated::parse(&image(r"out\a.img", Some("512"))).is_err());
    assert!(Elevated::parse(&image(r"C:\out\a.img", None)).is_err());
}

fn every_reason() -> Vec<Reason> {
    let s = || String::from("words\twith a tab\\ and\nlines");
    vec![
        Reason::NoWriter,
        Reason::NotCarried,
        Reason::Changed {
            device: "Disk 2".into(),
            serial: String::new(),
            size: 7,
        },
        Reason::InUse(InUse::Mounted(s())),
        Reason::InUse(InUse::Swap),
        Reason::InUse(InUse::System),
        Reason::BootOnly,
        Reason::TooSmall { disk: 1, image: 2 },
        Reason::PayloadSum,
        Reason::ImageChanged {
            path: s(),
            sha256: SUM.into(),
        },
        Reason::Gone,
        Reason::Bus(s()),
        Reason::Unmounting(s()),
        Reason::OpenWrite(s()),
        Reason::OpenRead(s()),
        Reason::Device {
            device: s(),
            detail: s(),
        },
        Reason::Flush {
            device: s(),
            detail: s(),
        },
        Reason::NotStick {
            device: "Disk 2".into(),
            fault: StickFault::BootSmall { boot: 3, image: 4 },
        },
        Reason::NotStick {
            device: "Disk 2".into(),
            fault: StickFault::Reading(s()),
        },
        Reason::NotStickImage(StickFault::Partitions(3)),
        Reason::NotStickImage(StickFault::DiskUuid),
        Reason::BootLarger { image: 5, stick: 6 },
        Reason::BootLabel,
        Reason::BootShort,
        Reason::TableChanged(s()),
        Reason::ReadingTable(s()),
        Reason::BackEnds { device: s(), at: 8 },
        Reason::BackDiffers { device: s(), at: 9 },
        Reason::BackFailed {
            device: s(),
            detail: s(),
        },
        Reason::SourceRead(s()),
        Reason::WriteFailed(s()),
        Reason::PayloadChanged,
        Reason::Damaged(s()),
        Reason::Longer(10),
        Reason::Shorter {
            expected: 11,
            actual: 12,
        },
        Reason::WrongSum(SUM.into()),
        Reason::NoHome,
        Reason::NoDesktopApp,
        Reason::CacheSum(SUM.into()),
        Reason::File {
            path: s(),
            detail: s(),
        },
        Reason::InTheWay { path: s() },
        Reason::Start {
            path: s(),
            detail: s(),
        },
        Reason::Restart(s()),
        Reason::Missing(s()),
        Reason::BootEntry(s()),
        Reason::NoStick,
        Reason::Declined,
        Reason::Unsupported,
    ]
}

#[test]
fn what_the_elevated_writer_reports_reaches_the_window_as_it_was_sent() {
    let mut sent = vec![
        Message::Progress {
            stage: WriteStage::Writing,
            done: 1 << 20,
            total: IMAGE,
        },
        Message::Progress {
            stage: WriteStage::ReadingBack,
            done: IMAGE,
            total: IMAGE,
        },
        Message::Done(Ok(())),
    ];
    sent.extend(every_reason().into_iter().map(|r| Message::Done(Err(r))));
    let bytes: String = sent.iter().map(Message::encode).collect();
    let mut r = Cursor::new(bytes.into_bytes());
    let mut got = Vec::new();
    while let Some(m) = pipe::next(&mut r) {
        got.push(m.expect("a message"));
    }
    assert_eq!(got, sent);
    // Anything else ends the conversation with an error, not a guess.
    for bad in [
        "progress\twriting\t1\n",
        "progress\tsideways\t1\t2\n",
        "failed\tNoSuchReason\n",
        "failed\tTooSmall\t1\t2\t3\n",
        "failed\tWriteFailed\tbad \\x escape\n",
        "ok\textra\n",
        "ok",
    ] {
        let mut r = Cursor::new(bad.as_bytes().to_vec());
        assert!(
            matches!(pipe::next(&mut r), Some(Err(_))),
            "{bad:?} was taken"
        );
    }
    let long = format!("failed WriteFailed\t{}\n", "x".repeat(pipe::MAX_LINE));
    assert!(matches!(
        pipe::next(&mut Cursor::new(long.into_bytes())),
        Some(Err(_))
    ));
}

/// The person's registry, kept in a map.
#[derive(Default)]
struct Map(BTreeMap<String, BTreeMap<String, Data>>);

impl Registry for Map {
    fn set(&mut self, key: &str, name: &str, data: &Data) -> Result<(), String> {
        self.0
            .entry(key.to_string())
            .or_default()
            .insert(name.to_string(), data.clone());
        Ok(())
    }

    fn delete(&mut self, key: &str) -> Result<(), String> {
        self.0.remove(key);
        Ok(())
    }

    fn exists(&mut self, key: &str) -> bool {
        self.0.contains_key(key)
    }
}

/// A file that answers `--version` as the hub app when it starts so.
fn ask(path: &Path) -> Option<(Program, String)> {
    let text = fs::read_to_string(path).ok()?;
    let build = text.strip_prefix("FARADAY HUB ")?;
    Some((Program::HubApp, build.trim().to_string()))
}

/// A new `%LOCALAPPDATA%` and Start menu, with what a person already
/// has there, and the running copy in Downloads.
fn computer(name: &str) -> (PathBuf, Dirs, PathBuf) {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("hub-windows-{name}"));
    let _ = fs::remove_dir_all(&root);
    let dirs = Dirs {
        local: root.join("AppData/Local"),
        programs: root.join("AppData/Roaming/Microsoft/Windows/Start Menu/Programs"),
    };
    for (file, text) in [
        ("Downloads/received.txt", "a received file"),
        ("AppData/Local/faraday/bitlocker-suspended", "C:\non"),
        ("AppData/Local/Programs/Other/other.exe", "another program"),
        (
            "AppData/Roaming/Microsoft/Windows/Start Menu/Programs/Other.lnk",
            "another shortcut",
        ),
    ] {
        let p = root.join(file);
        fs::create_dir_all(p.parent().expect("a folder")).expect("folder");
        fs::write(p, text).expect("file");
    }
    let running = root.join("Downloads/Faraday-0.2.0-Windows.exe");
    fs::write(&running, "FARADAY HUB 0.2.0").expect("file");
    (root, dirs, running)
}

fn tree(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) {
        for e in fs::read_dir(dir).expect("readable") {
            let p = e.expect("entry").path();
            if p.is_dir() {
                walk(&p, root, out);
            } else {
                out.push(p.strip_prefix(root).expect("under").display().to_string());
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

#[test]
fn install_puts_the_app_the_shortcut_and_the_uninstall_entry_and_uninstall_takes_them_back() {
    let (root, dirs, running) = computer("install");
    let before = tree(&root);
    let mut reg = Map::default();
    let steps = install::plan(&dirs, "0.2.0", "Faraday", 10_138_112);
    let app = dirs.app();
    let shown = app.display().to_string();
    assert_eq!(
        install::install(&dirs, &steps, &running, &ask, &mut reg),
        Ok(shown.clone())
    );
    let mut after = before.clone();
    after.extend([
        "AppData/Local/Programs/Faraday/Faraday.exe".to_string(),
        "AppData/Roaming/Microsoft/Windows/Start Menu/Programs/Faraday.lnk".to_string(),
    ]);
    after.sort();
    assert_eq!(tree(&root), after);
    assert_eq!(
        fs::read(&app).expect("installed"),
        fs::read(&running).expect("running")
    );
    let lnk = fs::read(dirs.shortcut()).expect("shortcut");
    assert_eq!(shortcut::target(&lnk).as_deref(), Some(shown.as_str()));
    let values = &reg.0[UNINSTALL_KEY];
    let text = |k: &str| match values.get(k) {
        Some(Data::Sz(s)) => s.clone(),
        other => panic!("{k}: {other:?}"),
    };
    assert_eq!(text("DisplayName"), "Faraday");
    assert_eq!(text("Publisher"), "Faraday");
    assert_eq!(text("DisplayVersion"), "0.2.0");
    assert_eq!(text("UninstallString"), format!("\"{shown}\" --uninstall"));
    assert_eq!(text("DisplayIcon"), format!("{shown},0"));
    assert_eq!(values.get("EstimatedSize"), Some(&Data::Dword(9901)));
    assert_eq!(reg.0.len(), 1);
    let installed = install::installed(&dirs, &ask, &mut reg).expect("installed");
    assert_eq!(installed.version, "0.2.0");
    let items: Vec<_> = installed.items.iter().map(|(i, _)| *i).collect();
    assert_eq!(
        items,
        [
            InstallItem::App,
            InstallItem::Menu,
            InstallItem::UninstallEntry
        ]
    );
    // Run from Downloads: everything goes at once.
    assert_eq!(
        install::uninstall(&dirs, &running, &ask, &mut reg),
        Ok(Left::Nothing)
    );
    assert_eq!(tree(&root), before);
    assert!(reg.0.is_empty());
    assert!(install::installed(&dirs, &ask, &mut reg).is_none());
}

#[test]
fn installing_again_replaces_faradays_own_file_and_never_another_programs() {
    let (_root, dirs, running) = computer("again");
    let mut reg = Map::default();
    let steps = install::plan(&dirs, "0.2.0", "Faraday", 1);
    fs::create_dir_all(dirs.folder()).expect("folder");
    fs::write(dirs.app(), "FARADAY HUB 0.1.1").expect("older");
    assert!(install::install(&dirs, &steps, &running, &ask, &mut reg).is_ok());
    assert_eq!(
        fs::read_to_string(dirs.app()).expect("app"),
        "FARADAY HUB 0.2.0"
    );
    assert_eq!(
        fs::read_dir(dirs.folder()).expect("folder").count(),
        1,
        "nothing set aside is left"
    );
    // Someone else's Faraday.exe is in the way, and stays.
    let (_root, dirs, running) = computer("in-the-way");
    fs::create_dir_all(dirs.folder()).expect("folder");
    fs::write(dirs.app(), "another program").expect("other");
    let mut reg = Map::default();
    assert_eq!(
        install::install(&dirs, &steps, &running, &ask, &mut reg),
        Err(Reason::InTheWay {
            path: dirs.app().display().to_string()
        })
    );
    assert!(reg.0.is_empty());
    assert!(!dirs.shortcut().exists());
    assert_eq!(
        install::uninstall(&dirs, &running, &ask, &mut reg),
        Ok(Left::Nothing)
    );
    assert_eq!(
        fs::read_to_string(dirs.app()).expect("kept"),
        "another program"
    );
}

#[test]
fn uninstall_from_the_installed_copy_leaves_its_last_step_for_after_it_exits() {
    let (_root, dirs, running) = computer("self");
    let mut reg = Map::default();
    let steps = install::plan(&dirs, "0.2.0", "Faraday", 1);
    install::install(&dirs, &steps, &running, &ask, &mut reg).expect("installed");
    let left = install::uninstall(&dirs, &dirs.app(), &ask, &mut reg).expect("uninstalled");
    let folder = dirs.folder().display().to_string();
    let Left::AfterExit(command) = left else {
        panic!("nothing left for after exit");
    };
    assert!(command.contains(&format!("del /f /q \"{folder}\\Faraday.exe\"")));
    assert!(command.contains(&format!("rmdir \"{folder}\"")));
    assert!(!command.contains("/s /q \""), "never a recursive remove");
    // The rest is gone already; the running file waits for its exit.
    assert!(dirs.app().exists());
    assert!(!dirs.shortcut().exists());
    assert!(reg.0.is_empty());
}
