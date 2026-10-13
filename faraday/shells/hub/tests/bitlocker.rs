//! The BitLocker guard on Windows (`docs/HUB.md` §6, §11), from WMI's
//! answers as recorded (`tests/windows/*.wmi`): what the elevated copy
//! asks of `Win32_EncryptableVolume` and in what order, what it refuses,
//! what it tells the window over its pipe, and what the window leaves on
//! the computer (the marker, the logon reminder, and the key only where
//! Save puts it). None of it calls Windows, so it runs on any system.

use std::collections::BTreeMap;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use faraday_core::Program;
use faraday_hub::api::{InstallItem, KeyShown, Marker, Protection, Reason, StickFault};
use faraday_hub_shell::windows::bitlocker::{
    self, Answer, Found, MarkerFile, Order, Recorded, Verb,
};
use faraday_hub_shell::windows::install::{self, Data, Dirs, RUN_KEY, RUN_VALUE, Registry};
use faraday_qr::bitlocker::{KeyText, RecoveryKey};

const KEY: &str = "205568-492019-108251-336446-650375-210331-584155-047520";
const ID: &str = "{6A0E1F2C-3B4D-4E5F-8A9B-0C1D2E3F4A5B}";
const C: &str = r"\\?\Volume{1b2c3d4e-0000-4000-8000-00000000000c}\";
const E: &str = r"\\?\Volume{1b2c3d4e-0000-4000-8000-00000000000e}\";
const PIPE: &str = r"\\.\pipe\faraday-hub-1f2e-3d4c";

fn recorded(name: &str) -> Recorded {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/windows")
        .join(name);
    let text = fs::read_to_string(&path).expect("a recorded answer");
    Recorded::parse(&text).expect("a well-formed recording")
}

fn key() -> RecoveryKey {
    RecoveryKey::parse(KEY).expect("a key")
}

#[test]
fn show_key_reads_the_system_drives_recovery_password_and_changes_nothing() {
    let mut wmi = recorded("pro.wmi");
    let found = bitlocker::show_key(&mut wmi, "C:").expect("the key");
    assert_eq!(
        found,
        Found {
            drive: "C:".into(),
            id: ID.into(),
            key: key()
        }
    );
    assert_eq!(
        wmi.log,
        [
            "volumes".to_string(),
            format!("GetKeyProtectors {C} KeyProtectorType=u32:3"),
            format!("GetKeyProtectorNumericalPassword {C} VolumeKeyProtectorID=str:{ID}"),
        ]
    );
}

#[test]
fn a_drive_without_a_recovery_password_gets_one_and_that_one_is_shown() {
    let mut wmi = recorded("no-password.wmi");
    let found = bitlocker::show_key(&mut wmi, "C:").expect("the key");
    assert_eq!(found.key, key());
    assert_eq!(found.id, "{7B1F2A3D-4C5E-4F60-9BAC-1D2E3F4A5B6C}");
    assert_eq!(wmi.made("ProtectKeyWithNumericalPassword"), 1);
    assert_eq!(wmi.made("DisableKeyProtectors"), 0);
}

#[test]
fn suspend_has_no_restart_count_and_touches_only_operating_system_volumes() {
    let mut wmi = recorded("pro-suspend.wmi");
    assert_eq!(
        bitlocker::suspend(&mut wmi, "C:"),
        Ok(vec![(C.to_string(), "C:".to_string())])
    );
    let disables: Vec<&String> = wmi
        .log
        .iter()
        .filter(|l| l.starts_with("DisableKeyProtectors"))
        .collect();
    assert_eq!(
        disables,
        [&format!("DisableKeyProtectors {C} DisableCount=u32:0")]
    );
    // The key is not read to suspend.
    assert_eq!(wmi.made("GetKeyProtectorNumericalPassword"), 0);
    assert_eq!(bitlocker::state(&mut wmi, "C:"), Protection::Suspended);
}

#[test]
fn a_refused_suspension_is_a_managed_computers_and_leaves_nothing_half_done() {
    let mut wmi = recorded("managed.wmi");
    assert_eq!(
        bitlocker::suspend(&mut wmi, "C:"),
        Err(Reason::Managed(2_150_694_970))
    );
    assert_eq!(bitlocker::state(&mut wmi, "C:"), Protection::On);

    // Two Windows installations: the one suspended is resumed again when
    // the other is refused.
    let mut wmi = recorded("two-systems.wmi");
    assert_eq!(
        bitlocker::suspend(&mut wmi, "C:"),
        Err(Reason::Managed(2_150_694_970))
    );
    assert!(wmi.log.contains(&format!("EnableKeyProtectors {C}")));
    assert!(!wmi.log.contains(&format!("EnableKeyProtectors {E}")));

    // Said yes, and protected again when read back: refused too.
    let mut wmi = recorded("undone.wmi");
    assert_eq!(bitlocker::suspend(&mut wmi, "C:"), Err(Reason::Managed(0)));
    assert!(wmi.log.contains(&format!("EnableKeyProtectors {C}")));
}

#[test]
fn a_control_panel_suspension_is_made_one_without_a_restart_count() {
    let mut wmi = recorded("control-panel.wmi");
    assert_eq!(
        bitlocker::suspend(&mut wmi, "C:"),
        Ok(vec![(C.to_string(), "C:".to_string())])
    );
    assert_eq!(wmi.made("DisableKeyProtectors"), 1);
}

#[test]
fn a_drive_not_encrypted_has_no_key_and_nothing_to_suspend() {
    let mut wmi = recorded("not-encrypted.wmi");
    assert_eq!(
        bitlocker::show_key(&mut wmi, "C:"),
        Err(Reason::NotEncrypted)
    );
    assert_eq!(
        bitlocker::suspend(&mut wmi, "C:"),
        Err(Reason::NotEncrypted)
    );
    assert_eq!(wmi.made("DisableKeyProtectors"), 0);
    assert_eq!(wmi.made("GetKeyProtectors"), 0);
}

#[test]
fn resume_enables_the_markers_volumes_and_reads_them_back() {
    let mut wmi = recorded("pro-resume.wmi");
    assert_eq!(bitlocker::resume(&mut wmi, &[C.to_string()]), Ok(()));
    assert_eq!(wmi.made("EnableKeyProtectors"), 1);
    assert_eq!(bitlocker::state(&mut wmi, "C:"), Protection::On);
    // A volume no longer here is passed over.
    let mut wmi = recorded("pro-resume.wmi");
    assert_eq!(bitlocker::resume(&mut wmi, &[E.to_string()]), Ok(()));
    assert_eq!(wmi.made("EnableKeyProtectors"), 0);
}

#[test]
fn the_elevated_copy_reads_back_the_step_it_was_started_with() {
    for verb in [
        Verb::ShowKey,
        Verb::Suspend,
        Verb::Resume(vec![C.to_string(), E.to_string()]),
    ] {
        let o = Order {
            verb,
            pipe: PIPE.into(),
        };
        assert_eq!(Order::parse(&o.args()), Ok(o));
    }
    let bad = |a: &[&str]| {
        let a: Vec<String> = a.iter().map(|s| s.to_string()).collect();
        Order::parse(&a).is_err()
    };
    assert!(bad(&["--bitlocker", "show-key"]));
    assert!(bad(&["--bitlocker", "format", "--pipe", PIPE]));
    assert!(bad(&[
        "--bitlocker",
        "suspend",
        "--pipe",
        r"\\.\pipe\other"
    ]));
    assert!(bad(&[
        "--bitlocker",
        "suspend",
        "--pipe",
        PIPE,
        "--pipe",
        PIPE
    ]));
    assert!(bad(&[
        "--bitlocker",
        "suspend",
        "--pipe",
        PIPE,
        "--volume",
        C
    ]));
    assert!(bad(&[
        "--bitlocker",
        "resume",
        "--pipe",
        PIPE,
        "--volume",
        "C:"
    ]));
    assert!(bad(&["--write", "suspend", "--pipe", PIPE]));
}

#[test]
fn the_pipe_message_carrying_the_key_round_trips_and_is_wiped() {
    let answers = [
        Answer::Key(Found {
            drive: "C:".into(),
            id: ID.into(),
            key: key(),
        }),
        Answer::Suspended {
            device: C.into(),
            letter: "C:".into(),
        },
        Answer::State(Protection::Suspended),
        Answer::Done(Ok(())),
        Answer::Done(Err(Reason::Managed(2_150_694_970))),
        Answer::Done(Err(Reason::NoAdmin)),
        Answer::Done(Err(Reason::BitLocker {
            call: "GetKeyProtectors".into(),
            code: 0x8004_1003,
        })),
        Answer::Done(Err(Reason::NotStick {
            device: "Disk 2".into(),
            fault: StickFault::Partitions(3),
        })),
    ];
    let mut wire = Vec::new();
    let mut scratch = Vec::new();
    for a in &answers {
        bitlocker::send(&mut wire, a, &mut scratch).expect("sent");
        // What was sent is not left in the sender's buffer.
        assert!(scratch.iter().all(|&b| b == 0), "{a:?}");
    }
    let mut r = Cursor::new(wire);
    let mut buf = vec![0u8; bitlocker::MAX_LINE];
    for a in &answers {
        let got = bitlocker::next(&mut r, &mut buf).expect("a line");
        assert_eq!(got.as_ref(), Ok(a));
        // Nor in the reader's.
        assert!(buf.iter().all(|&b| b == 0), "{a:?}");
    }
    assert!(bitlocker::next(&mut r, &mut buf).is_none());
    // A key line that is not a key is refused.
    for bad in [
        format!("key\tC:\t{ID}\t205568-492019\n"),
        format!("key\tC\t{ID}\t{KEY}\n"),
        format!("key\tC:\tnot-an-id\t{KEY}\n"),
        String::from("key\n"),
    ] {
        let mut r = Cursor::new(bad.into_bytes());
        assert!(matches!(bitlocker::next(&mut r, &mut buf), Some(Err(_))));
        assert!(buf.iter().all(|&b| b == 0));
    }
    let long = format!("state\t{}\n", "x".repeat(bitlocker::MAX_LINE));
    let mut r = Cursor::new(long.into_bytes());
    assert!(matches!(bitlocker::next(&mut r, &mut buf), Some(Err(_))));
}

#[test]
fn the_elevated_copys_whole_step_reaches_the_window_as_it_was_answered() {
    let mut wire = Vec::new();
    assert!(bitlocker::run(
        &mut recorded("pro.wmi"),
        &Verb::ShowKey,
        "C:",
        &mut wire
    ));
    let reply = bitlocker::reply(&mut Cursor::new(wire)).expect("a reply");
    assert_eq!(reply.key.map(|f| f.key), Some(key()));
    assert_eq!(reply.state, Some(Protection::On));
    assert_eq!(reply.done, Some(Ok(())));

    let mut wire = Vec::new();
    assert!(bitlocker::run(
        &mut recorded("pro-suspend.wmi"),
        &Verb::Suspend,
        "C:",
        &mut wire
    ));
    let reply = bitlocker::reply(&mut Cursor::new(wire)).expect("a reply");
    assert_eq!(reply.suspended, [(C.to_string(), "C:".to_string())]);
    assert_eq!(reply.state, Some(Protection::Suspended));
    assert!(reply.key.is_none());

    let mut wire = Vec::new();
    assert!(!bitlocker::run(
        &mut recorded("managed.wmi"),
        &Verb::Suspend,
        "C:",
        &mut wire
    ));
    let reply = bitlocker::reply(&mut Cursor::new(wire)).expect("a reply");
    assert_eq!(reply.done, Some(Err(Reason::Managed(2_150_694_970))));
    assert_eq!(reply.state, Some(Protection::On));
    assert!(reply.suspended.is_empty());
}

#[test]
fn the_marker_holds_the_volumes_secure_boot_and_the_boot_and_nothing_else() {
    let m = MarkerFile {
        volumes: vec![(C.to_string(), "C:".to_string())],
        secure_boot_was_on: true,
        boot: Some(1_000_000),
    };
    let text = m.text();
    assert_eq!(
        text,
        format!("faraday bitlocker-suspended 1\nvolume {C} C:\nsecure-boot on\nboot 1000000\n")
    );
    assert_eq!(MarkerFile::parse(&text), Some(m.clone()));
    assert_eq!(
        m.marker(Some(2_000_000)),
        Marker {
            volumes: vec!["C:".into()],
            secure_boot_was_on: true,
            this_boot: false
        }
    );
    for bad in [
        "",
        "C:\non",
        "faraday bitlocker-suspended 1\nsecure-boot on\n",
        &format!("faraday bitlocker-suspended 1\nvolume {C} C:\n"),
        &format!("faraday bitlocker-suspended 1\nvolume {C} C:\nsecure-boot maybe\n"),
        "faraday bitlocker-suspended 1\nvolume C: C:\nsecure-boot on\n",
    ] {
        assert_eq!(MarkerFile::parse(bad), None, "{bad:?}");
    }
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

    fn get(&mut self, key: &str, name: &str) -> Option<Data> {
        self.0.get(key)?.get(name).cloned()
    }

    fn delete_value(&mut self, key: &str, name: &str) -> Result<(), String> {
        if let Some(k) = self.0.get_mut(key) {
            k.remove(name);
        }
        Ok(())
    }
}

/// A file that answers `--version` as the hub app when it starts so.
fn ask(path: &Path) -> Option<(Program, String)> {
    let text = fs::read_to_string(path).ok()?;
    let build = text.strip_prefix("FARADAY HUB ")?;
    Some((Program::HubApp, build.trim().to_string()))
}

/// A new `%LOCALAPPDATA%`, Start menu and USB stick, and the running
/// copy in Downloads.
fn computer(name: &str) -> (PathBuf, Dirs, PathBuf, PathBuf) {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("hub-bitlocker-{name}"));
    let _ = fs::remove_dir_all(&root);
    let dirs = Dirs {
        local: root.join("AppData/Local"),
        programs: root.join("AppData/Roaming/Microsoft/Windows/Start Menu/Programs"),
    };
    let stick = root.join("E");
    for p in [&dirs.local, &dirs.programs, &stick, &root.join("Downloads")] {
        fs::create_dir_all(p).expect("folder");
    }
    let running = root.join("Downloads/Faraday-Starter-0.2.0-Windows.exe");
    fs::write(&running, "FARADAY HUB 0.2.0").expect("file");
    (root, dirs, running, stick)
}

/// Every file under `root`, and whether any holds any group of the key.
fn files_with_the_key(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in fs::read_dir(dir).expect("readable") {
            let p = e.expect("entry").path();
            if p.is_dir() {
                walk(&p, out);
            } else {
                out.push(p);
            }
        }
    }
    let mut all = Vec::new();
    walk(root, &mut all);
    let groups: Vec<&str> = KEY.split('-').collect();
    all.into_iter()
        .filter(|p| {
            let bytes = fs::read(p).expect("readable");
            let text = String::from_utf8_lossy(&bytes);
            groups.iter().any(|g| text.contains(g))
        })
        .map(|p| p.strip_prefix(root).expect("under").display().to_string())
        .collect()
}

fn shown(found: Found) -> KeyShown {
    KeyShown {
        text: KeyText {
            computer: "DESKTOP-4K2Q".into(),
            drive: found.drive,
            key: found.key,
        },
        id: found.id,
    }
}

#[test]
fn the_key_is_never_written_to_a_file_unless_the_person_saves_it() {
    let (root, dirs, running, stick) = computer("guard");
    let mut reg = Map::default();
    // Installed, so the reminder is added.
    let steps = install::plan(&dirs, "0.2.0", "Faraday Starter", 1);
    install::install(&dirs, &steps, &running, &ask, &mut reg).expect("installed");

    // The whole guard: the key shown, then suspended, the marker and the
    // reminder written.
    let found = bitlocker::show_key(&mut recorded("pro.wmi"), "C:").expect("the key");
    let k = shown(found);
    let volumes = bitlocker::suspend(&mut recorded("pro-suspend.wmi"), "C:").expect("suspended");
    bitlocker::after_suspend(&dirs, &volumes, Some(true), Some(1000), &ask, &mut reg)
        .expect("marker");
    assert!(files_with_the_key(&root).is_empty());
    let marker = bitlocker::read_marker(&dirs.local).expect("the marker");
    assert_eq!(marker.volumes, volumes);
    assert!(marker.secure_boot_was_on);

    // Save, to the stick the person chose: one file there, which reads
    // back as the QR code's text.
    let saved = bitlocker::save_key_to(&stick, &k, "BitLocker recovery key {}.txt").expect("saved");
    assert_eq!(saved, stick.join("BitLocker recovery key 6A0E1F2C.txt"));
    assert_eq!(
        files_with_the_key(&root),
        ["E/BitLocker recovery key 6A0E1F2C.txt"]
    );
    let text = fs::read_to_string(&saved).expect("saved");
    assert_eq!(KeyText::read(&text), Some(k.text.clone()));
    // Saved again: a second file, never over the first.
    let again = bitlocker::save_key_to(&stick, &k, "BitLocker recovery key {}.txt").expect("saved");
    assert_eq!(again, stick.join("BitLocker recovery key 6A0E1F2C 2.txt"));
}

#[test]
fn the_reminder_is_added_for_an_installed_copy_only_and_goes_with_the_marker() {
    // Run from Downloads, nothing installed: the marker, no reminder.
    let (_root, dirs, running, _) = computer("portable");
    let mut reg = Map::default();
    let volumes = vec![(C.to_string(), "C:".to_string())];
    bitlocker::after_suspend(&dirs, &volumes, None, None, &ask, &mut reg).expect("marker");
    assert!(reg.get(RUN_KEY, RUN_VALUE).is_none());
    // Secure Boot not read counts as on: Resume never offered with it
    // off on a computer that may have had it on.
    assert!(
        bitlocker::read_marker(&dirs.local)
            .expect("marker")
            .secure_boot_was_on
    );

    // Installed: the reminder points at the installed copy, never at the
    // one in Downloads, and Installed lists it.
    let steps = install::plan(&dirs, "0.2.0", "Faraday Starter", 1);
    install::install(&dirs, &steps, &running, &ask, &mut reg).expect("installed");
    bitlocker::after_suspend(&dirs, &volumes, Some(false), Some(1000), &ask, &mut reg)
        .expect("marker");
    assert_eq!(
        reg.get(RUN_KEY, RUN_VALUE),
        Some(Data::Sz(format!("\"{}\" --remind", dirs.app().display())))
    );
    assert!(
        !bitlocker::read_marker(&dirs.local)
            .expect("marker")
            .secure_boot_was_on
    );
    let items: Vec<InstallItem> = install::installed(&dirs, &ask, &mut reg)
        .expect("installed")
        .items
        .iter()
        .map(|(i, _)| *i)
        .collect();
    assert!(items.contains(&InstallItem::Reminder));

    // Resumed: both go; another program's Run values stay.
    reg.set(RUN_KEY, "Other", &Data::Sz("other.exe".into()))
        .expect("set");
    bitlocker::after_resume(&dirs, &mut reg).expect("resumed");
    assert!(bitlocker::read_marker(&dirs.local).is_none());
    assert!(!bitlocker::marker_path(&dirs.local).exists());
    assert!(reg.get(RUN_KEY, RUN_VALUE).is_none());
    assert_eq!(
        reg.get(RUN_KEY, "Other"),
        Some(Data::Sz("other.exe".into()))
    );
}

#[test]
fn uninstall_while_suspended_removes_the_marker_and_the_reminder() {
    let (_root, dirs, running, _) = computer("uninstall");
    let mut reg = Map::default();
    let steps = install::plan(&dirs, "0.2.0", "Faraday Starter", 1);
    install::install(&dirs, &steps, &running, &ask, &mut reg).expect("installed");
    let volumes = vec![(C.to_string(), "C:".to_string())];
    bitlocker::after_suspend(&dirs, &volumes, Some(true), Some(1000), &ask, &mut reg)
        .expect("marker");
    install::uninstall(&dirs, &running, &ask, &mut reg).expect("uninstalled");
    assert!(!bitlocker::marker_path(&dirs.local).exists());
    assert!(!dirs.local.join("faraday").exists());
    assert!(reg.get(RUN_KEY, RUN_VALUE).is_none());

    // Another program's Run value of the same name is not the hub app's.
    let (_root, dirs, running, _) = computer("uninstall-other");
    let mut reg = Map::default();
    reg.set(
        RUN_KEY,
        RUN_VALUE,
        &Data::Sz(r"C:\Tools\faraday.exe".into()),
    )
    .expect("set");
    install::uninstall(&dirs, &running, &ask, &mut reg).expect("uninstalled");
    assert_eq!(
        reg.get(RUN_KEY, RUN_VALUE),
        Some(Data::Sz(r"C:\Tools\faraday.exe".into()))
    );
}

#[test]
fn an_unreadable_protection_is_not_known_and_treated_as_protected() {
    assert_eq!(bitlocker::shell_protection(None), Protection::Unknown);
    assert_eq!(bitlocker::shell_protection(Some(0)), Protection::Unknown);
    // 2 is not trusted as "not encrypted".
    assert_eq!(bitlocker::shell_protection(Some(2)), Protection::Unknown);
    assert_eq!(bitlocker::shell_protection(Some(1)), Protection::On);
    assert_eq!(bitlocker::shell_protection(Some(5)), Protection::Suspended);
    assert_eq!(bitlocker::drive_of(r"C:\Windows"), "C:");
    assert_eq!(bitlocker::drive_of(r"d:\WINDOWS"), "D:");
    assert_eq!(bitlocker::drive_of(""), "C:");
}

#[test]
fn only_a_fully_decrypted_unprotected_volume_is_not_encrypted() {
    let mut wmi = recorded("not-encrypted.wmi");
    assert_eq!(bitlocker::state(&mut wmi, "C:"), Protection::None);
    // Not listed: not known, and no key to show.
    let mut wmi = recorded("not-encrypted.wmi");
    assert_eq!(bitlocker::state(&mut wmi, "Z:"), Protection::Unknown);
    assert_eq!(bitlocker::show_key(&mut wmi, "Z:"), Err(Reason::NotRead));
    // Protection off with conversion not at 0: suspended.
    let mut wmi = recorded("suspended-converted.wmi");
    assert_eq!(bitlocker::state(&mut wmi, "C:"), Protection::Suspended);
    assert_eq!(wmi.made("GetKeyProtectors"), 0);
    // No system volume listed: nothing to suspend is not "not encrypted".
    let mut wmi = recorded("data-only.wmi");
    assert_eq!(bitlocker::suspend(&mut wmi, "C:"), Err(Reason::NotRead));
}

#[test]
fn resume_waits_for_a_later_boot_than_the_one_that_suspended() {
    use bitlocker::{MarkerFile, same_boot};
    let m = |boot| MarkerFile {
        volumes: vec![(C.to_string(), "C:".to_string())],
        secure_boot_was_on: true,
        boot,
    };
    // The same boot read twice, a second apart.
    assert!(same_boot(Some(1_000_000), Some(1_000_001)));
    assert!(m(Some(1_000_000)).marker(Some(1_000_002)).this_boot);
    // Windows started again.
    assert!(!m(Some(1_000_000)).marker(Some(1_086_400)).this_boot);
    // A marker without a boot is from an earlier boot.
    assert!(!m(None).marker(Some(1_000_000)).this_boot);
    // The text round-trips, and an old marker without the line reads.
    let text = m(Some(1_000_000)).text();
    assert_eq!(MarkerFile::parse(&text), Some(m(Some(1_000_000))));
    assert_eq!(MarkerFile::parse(&m(None).text()), Some(m(None)));
}
