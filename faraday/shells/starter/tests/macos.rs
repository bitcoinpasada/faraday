//! The Starter on macOS, from macOS's answers as written by hand from
//! documentation (`tests/macos/`; batch A3c replaces them with answers
//! recorded on a Mac): the disks a person is offered and refused, a
//! property list that cannot be read, this computer's facts, and what
//! install and uninstall leave in a home folder. None of it calls macOS,
//! so it runs on any system.

use std::fs;
use std::path::{Path, PathBuf};

use faraday_starter::api::{Bus, InUse, InstallItem, Kind, Os, Reason};
use faraday_starter::decide::{Refusal, Target, offers};
use faraday_starter_shell::macos::diskutil::{Answers, Found};
use faraday_starter_shell::macos::facts;
use faraday_starter_shell::macos::install::{self, Dirs};
use faraday_starter_shell::macos::plist;

/// The stick image's size: 81 MiB.
const IMAGE: u64 = 84_955_136;

/// A file of answers: each after a `=== <command>` line.
fn answers(name: &str) -> Answers {
    let text = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/macos")
            .join(name),
    )
    .expect("written answers");
    let mut parts: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        if let Some(cmd) = line.strip_prefix("=== ") {
            parts.push((cmd.to_string(), String::new()));
        } else if let Some((_, body)) = parts.last_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    let (first, list) = &parts[0];
    assert_eq!(first, "diskutil list -plist");
    let mut a = Answers::from_list(list).expect("a well-formed list");
    for (cmd, body) in &parts[1..] {
        let id = cmd.strip_prefix("diskutil info -plist ").expect("an info");
        a.add_info(id, body).expect("a well-formed info");
    }
    a
}

fn disk<'a>(list: &'a [Found], id: &str) -> &'a Found {
    list.iter()
        .find(|f| f.disk.id == id)
        .unwrap_or_else(|| panic!("{id} listed"))
}

fn offered(a: &Answers, target: Target) -> Vec<(String, Option<Refusal>)> {
    let disks: Vec<_> = a.disks().into_iter().map(|f| f.disk).collect();
    offers(&disks, target, Some(IMAGE))
        .into_iter()
        .map(|o| (o.disk.device.clone(), o.refused))
        .collect()
}

#[test]
fn usb_sticks_are_offered_first_a_faraday_stick_and_a_disk_mounted_elsewhere_is_refused() {
    let a = answers("intel-mac.txt");
    assert_eq!(
        offered(&a, Target::Stick),
        vec![
            ("/dev/disk3".to_string(), None),
            ("/dev/disk2".to_string(), None),
            (
                "/dev/disk6".to_string(),
                Some(Refusal::InUse(InUse::Mounted(String::from(
                    "/Library/Backup"
                ))))
            ),
        ]
    );
    let list = a.disks();
    // The internal disk, its APFS container and the disk image are not
    // listed at all.
    for id in ["disk0", "disk1", "disk5"] {
        assert!(list.iter().all(|f| f.disk.id != id), "{id} listed");
    }
    let stick = &disk(&list, "disk2").disk;
    assert_eq!(stick.bus, Bus::Usb);
    assert_eq!(
        (
            stick.vendor.as_str(),
            stick.model.as_str(),
            stick.serial.as_str()
        ),
        ("Acme", "Stick", "")
    );
    assert_eq!(stick.size, 16_008_609_792);
    assert_eq!(stick.volumes.len(), 1);
    assert_eq!(stick.volumes[0].label.as_deref(), Some("HOLIDAY"));
    assert!(stick.volumes[0].files);
    assert!(stick.faraday.is_none());
    assert_eq!(stick.in_use, None);
    // A size diskutil prints as a real reads as the same bytes.
    assert_eq!(disk(&list, "disk6").disk.size, 1_000_204_886_016);
}

#[test]
fn a_faraday_stick_is_known_by_its_partition_names_and_its_data_partition_is_never_read() {
    let a = answers("intel-mac.txt");
    let list = a.disks();
    let f = disk(&list, "disk3");
    assert!(f.disk.faraday.is_some());
    let boot = f.boot.as_ref().expect("its boot partition");
    // macOS does not mount an EFI system partition, so the boot
    // partition is mounted read-only to read its version.
    assert_eq!(boot.id, "disk3s1");
    assert_eq!(boot.mounted, None);
    let labels: Vec<_> = f.disk.volumes.iter().map(|v| v.label.as_deref()).collect();
    assert_eq!(labels, [Some("OSKBOOT"), None]);
    // A stick whose volumes carry those names as labels alone is not one.
    assert!(disk(&list, "disk2").disk.faraday.is_none());
}

#[test]
fn the_built_in_sd_reader_is_offered_for_a_pi_card_only() {
    let a = answers("intel-mac.txt");
    assert!(offered(&a, Target::Pi).contains(&("/dev/disk4".to_string(), None)));
    assert!(
        !offered(&a, Target::Stick)
            .iter()
            .any(|(d, _)| d == "/dev/disk4")
    );
    assert_eq!(disk(&a.disks(), "disk4").disk.bus, Bus::SdReader);
}

#[test]
fn the_usb_disk_macos_started_from_is_refused_as_the_systems() {
    let a = answers("usb-start.txt");
    assert_eq!(
        offered(&a, Target::Stick),
        vec![(
            "/dev/disk2".to_string(),
            Some(Refusal::InUse(InUse::System))
        )]
    );
    // Its APFS container says USB too, and is never listed.
    assert!(a.disks().iter().all(|f| f.disk.id != "disk3"));
}

#[test]
fn the_writer_checks_the_same_disk_it_was_shown_with_its_blocks() {
    let a = answers("intel-mac.txt");
    let (id, block) = a.identity("disk2").expect("disk2");
    assert_eq!(id.device, "/dev/disk2");
    assert_eq!(id.size, 16_008_609_792);
    assert_eq!(id.in_use, None);
    assert_eq!(block, 512);
    assert!(a.identity("disk5").is_none());
    assert!(a.identity("disk9").is_none());
    let started = answers("usb-start.txt");
    assert_eq!(
        started.identity("disk2").expect("disk2").0.in_use,
        Some(InUse::System)
    );
}

#[test]
fn a_disk_whose_answers_lack_what_the_rule_needs_is_not_listed() {
    let full = answers("intel-mac.txt");
    // Without a partition's own answer, or without the key saying it is
    // not a disk image, or the bus, the disk is not listed.
    let mut a = full.clone();
    a.info.remove("disk2s1");
    assert!(a.disks().iter().all(|f| f.disk.id != "disk2"));
    for key in ["VirtualOrPhysical", "BusProtocol", "DeviceBlockSize"] {
        let mut a = full.clone();
        if let Some(plist::Value::Dict(d)) = a.info.get_mut("disk2") {
            d.remove(key);
        }
        assert!(
            a.disks().iter().all(|f| f.disk.id != "disk2"),
            "listed without {key}"
        );
    }
    // An answer about another disk than the one asked is refused.
    let mut a = full;
    let other =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/macos/intel-mac.txt"))
            .expect("answers");
    let disk3 = other
        .split("=== diskutil info -plist disk3\n")
        .nth(1)
        .and_then(|t| t.split("=== ").next())
        .expect("disk3's answer");
    assert!(a.add_info("disk2", disk3).is_err());
}

#[test]
fn a_property_list_cut_short_or_with_an_element_it_does_not_have_is_refused() {
    let whole = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<plist version=\"1.0\">\n<dict>\n\t<key>Size</key>\n\t<integer>512</integer>\n\t<key>Name</key>\n\t<string>A &amp; B</string>\n</dict>\n</plist>\n";
    let v = plist::parse(whole).expect("read");
    assert_eq!(v.get("Size").and_then(plist::Value::as_u64), Some(512));
    assert_eq!(v.get("Name").and_then(plist::Value::as_str), Some("A & B"));
    // Cut anywhere, it is refused, never read in part.
    let end = whole.find("</plist>").expect("closed");
    for at in 0..end {
        assert!(plist::parse(&whole[..at]).is_err(), "read when cut at {at}");
    }
    for bad in [
        whole.replace("<integer>512</integer>", "<uid>512</uid>"),
        whole.replace("<integer>512</integer>", "<integer>5x2</integer>"),
        whole.replace("A &amp; B", "A &nbsp; B"),
        whole.replace("<key>Name</key>", "<key>Size</key>"),
        whole.replace("<key>Name</key>\n", ""),
        whole.replace("</plist>\n", "</plist>\n<plist/>"),
        whole.replace("<string>", "<string kind=\"x\">"),
        String::from("bplist00"),
    ] {
        assert!(plist::parse(&bad).is_err(), "read: {bad}");
    }
}

fn computer(model: &str, machine: &str, translated: Option<&str>, vmm: &str) -> facts::Answers {
    facts::Answers {
        model: Some(format!("{model}\n")),
        machine: Some(format!("{machine}\n")),
        translated: translated.map(|t| format!("{t}\n")),
        vmm: Some(format!("{vmm}\n")),
    }
}

#[test]
fn this_macs_facts_are_its_model_identifier_and_whether_it_is_arm() {
    // An Intel Mac: sysctl has no sysctl.proc_translated there.
    let intel = facts::computer(&computer("MacBookPro14,1", "x86_64", None, "0"));
    assert_eq!(intel.os, Os::MacOs);
    assert_eq!(intel.maker, "Apple Inc.");
    assert_eq!(intel.model, "MacBookPro14,1");
    assert_eq!(intel.family, "");
    assert!(intel.uefi && !intel.arm);
    assert_eq!(intel.secure_boot, None);
    assert!(!intel.tpm_unlock);
    // An Apple Silicon Mac running the arm64 Starter.
    let m1 = facts::computer(&computer("MacBookAir10,1", "arm64", Some("0"), "0"));
    assert!(m1.arm && !m1.uefi);
    assert_eq!(m1.model, "MacBookAir10,1");
    // The x86_64 Starter under Rosetta on the same Mac: still ARM.
    let rosetta = facts::computer(&computer("MacBookAir10,1", "x86_64", Some("1"), "0"));
    assert!(rosetta.arm && !rosetta.uefi);
    // None of them is a virtual machine.
    for a in [
        computer("MacBookPro14,1", "x86_64", None, "0"),
        computer("MacBookAir10,1", "x86_64", Some("1"), "0"),
    ] {
        assert!(facts::concerns(&a).is_empty());
    }
}

#[test]
fn a_mac_that_is_a_virtual_machine_says_so() {
    let vm = computer("VMware7,1", "x86_64", None, "1");
    let c = facts::concerns(&vm);
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].kind, Kind::VirtualMachine);
    assert_eq!(c[0].id, "vm");
    assert!(!c[0].name.is_empty());
    // Apple's own virtual machines are named by their model.
    let apple = facts::concerns(&computer("VirtualMac2,1", "arm64", Some("0"), "1"));
    assert_eq!(apple[0].name, "VirtualMac2,1");
}

/// A new home folder with what a person already has in it, and the
/// running bundle in Downloads.
fn home(name: &str) -> (PathBuf, Dirs, PathBuf) {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("starter-macos-{name}"));
    let _ = fs::remove_dir_all(&root);
    for (file, text) in [
        ("Downloads/received.txt", "a received file"),
        (
            "Applications/Other.app/Contents/Info.plist",
            "another program",
        ),
        ("Library/Caches/other/cache.db", "another program's cache"),
        (
            "Library/Caches/faraday/0.1.1/faraday-desktop",
            "a cached desktop app",
        ),
        ("Library/Caches/faraday/notes.txt", "kept"),
    ] {
        let p = root.join(file);
        fs::create_dir_all(p.parent().expect("a folder")).expect("folder");
        fs::write(p, text).expect("file");
    }
    let bundle = root.join("Downloads/Faraday Starter.app");
    write_bundle(&bundle, install::IDENTIFIER, "0.2.0");
    fs::write(bundle.join("Contents/Resources/icon.icns"), "an icon").expect("file");
    let running = bundle.join("Contents/MacOS/faraday-starter");
    (root.clone(), Dirs { home: root }, running)
}

fn write_bundle(at: &Path, identifier: &str, version: &str) {
    fs::create_dir_all(at.join("Contents/MacOS")).expect("folder");
    fs::create_dir_all(at.join("Contents/Resources")).expect("folder");
    fs::write(
        at.join("Contents/Info.plist"),
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n\t<key>CFBundleExecutable</key>\n\t<string>faraday-starter</string>\n\t<key>CFBundleIdentifier</key>\n\t<string>{identifier}</string>\n\t<key>CFBundleShortVersionString</key>\n\t<string>{version}</string>\n</dict>\n</plist>\n"
        ),
    )
    .expect("file");
    fs::write(at.join("Contents/MacOS/faraday-starter"), "the starter app").expect("file");
}

/// What `ditto` does with a bundle folder, for the tests.
fn copy(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for e in fs::read_dir(from).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        let target = to.join(e.file_name());
        if e.path().is_dir() {
            copy(&e.path(), &target)?;
        } else {
            fs::copy(e.path(), target).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
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
fn install_copies_exactly_the_bundle_and_uninstall_takes_back_only_it() {
    let (root, dirs, running) = home("install");
    let before = tree(&root);
    assert_eq!(install::installed(&dirs), None);
    assert_eq!(
        install::install(&dirs, &running, &mut copy),
        Ok(String::from("~/Applications/Faraday Starter.app"))
    );
    let mut expected = before.clone();
    expected.extend(
        [
            "Applications/Faraday Starter.app/Contents/Info.plist",
            "Applications/Faraday Starter.app/Contents/MacOS/faraday-starter",
            "Applications/Faraday Starter.app/Contents/Resources/icon.icns",
        ]
        .map(String::from),
    );
    expected.sort();
    assert_eq!(tree(&root), expected);
    let found = install::installed(&dirs).expect("installed");
    assert_eq!(found.version, "0.2.0");
    assert_eq!(
        found.items,
        vec![(
            InstallItem::App,
            String::from("~/Applications/Faraday Starter.app")
        )]
    );
    // Installing a newer copy replaces it whole.
    let newer = root.join("Downloads/new/Faraday Starter.app");
    write_bundle(&newer, install::IDENTIFIER, "0.2.1");
    install::install(
        &dirs,
        &newer.join("Contents/MacOS/faraday-starter"),
        &mut copy,
    )
    .expect("installed");
    assert_eq!(
        install::installed(&dirs).expect("installed").version,
        "0.2.1"
    );
    assert!(!dirs.bundle().join("Contents/Resources/icon.icns").exists());
    // Uninstall: the bundle goes; everything else stays, a cache an
    // earlier Starter left included (F30).
    assert_eq!(install::uninstall(&dirs), Ok(()));
    let mut after = before;
    after.extend(
        [
            "Downloads/new/Faraday Starter.app/Contents/Info.plist",
            "Downloads/new/Faraday Starter.app/Contents/MacOS/faraday-starter",
        ]
        .map(String::from),
    );
    after.sort();
    assert_eq!(tree(&root), after);
    assert_eq!(install::installed(&dirs), None);
}

#[test]
fn another_programs_bundle_of_that_name_is_in_the_way_and_never_removed() {
    let (root, dirs, running) = home("in-the-way");
    write_bundle(&dirs.bundle(), "com.example.other", "3.0");
    let before = tree(&root);
    assert_eq!(install::installed(&dirs), None);
    assert_eq!(
        install::install(&dirs, &running, &mut copy),
        Err(Reason::InTheWay {
            path: String::from("~/Applications/Faraday Starter.app")
        })
    );
    assert_eq!(tree(&root), before);
    assert_eq!(install::uninstall(&dirs), Ok(()));
    assert_eq!(tree(&root), before);
}

#[test]
fn a_starter_not_running_from_a_bundle_cannot_install() {
    let (root, dirs, _) = home("no-bundle");
    let before = tree(&root);
    let loose = root.join("Downloads/faraday-starter");
    fs::write(&loose, "the starter app").expect("file");
    assert_eq!(
        install::install(&dirs, &loose, &mut copy),
        Err(Reason::Unsupported)
    );
    let mut expected = before;
    expected.push(String::from("Downloads/faraday-starter"));
    expected.sort();
    assert_eq!(tree(&root), expected);
}
