//! BitLocker on Windows (`docs/STARTER.md` §6, F49), from WMI's answers
//! as recorded (`tests/windows/*.wmi`): what the elevated copy asks of
//! `Win32_EncryptableVolume` and in what order, what it refuses, and
//! what it tells the window over its pipe. Suspend sets a count of three
//! times Windows starts; Resume turns protection back on; the key is
//! read for the sheet and nothing on the computer changes. None of it
//! calls Windows, so it runs on any system.
//!
//! Restored from the tests removed by F33 (53a715be), with what F49
//! leaves out (the marker, the logon reminder, the key file, a recovery
//! password added) gone.

use std::fs;
use std::io::Cursor;
use std::path::Path;

use faraday_starter::api::{Protection, Reason, RecoveryKey, StickFault};
use faraday_starter_shell::windows::bitlocker::{self, Answer, Order, Recorded, Verb};

const KEY: &str = "205568-492019-108251-336446-650375-210331-584155-047520";
const ID: &str = "{6A0E1F2C-3B4D-4E5F-8A9B-0C1D2E3F4A5B}";
const C: &str = r"\\?\Volume{1b2c3d4e-0000-4000-8000-00000000000c}\";
const E: &str = r"\\?\Volume{1b2c3d4e-0000-4000-8000-00000000000e}\";
const PIPE: &str = r"\\.\pipe\faraday-starter-1f2e-3d4c";

fn recorded(name: &str) -> Recorded {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/windows")
        .join(name);
    let text = fs::read_to_string(&path).expect("a recorded answer");
    Recorded::parse(&text).expect("a well-formed recording")
}

fn key() -> RecoveryKey {
    RecoveryKey::parse("C:", KEY).expect("a key")
}

#[test]
fn the_key_is_read_for_the_sheet_and_nothing_changes() {
    let mut wmi = recorded("pro.wmi");
    assert_eq!(bitlocker::read_key(&mut wmi, "C:"), Ok(key()));
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
fn a_drive_without_a_recovery_password_is_not_given_one() {
    let mut wmi = recorded("no-password.wmi");
    assert_eq!(
        bitlocker::read_key(&mut wmi, "C:"),
        Err(Reason::NoRecoveryKey)
    );
    assert_eq!(wmi.made("ProtectKeyWithNumericalPassword"), 0);
    assert_eq!(wmi.made("GetKeyProtectorNumericalPassword"), 0);
    assert_eq!(wmi.made("DisableKeyProtectors"), 0);
}

#[test]
fn suspend_is_for_three_times_windows_starts_on_operating_system_volumes_only() {
    let mut wmi = recorded("pro-suspend.wmi");
    assert_eq!(bitlocker::suspend(&mut wmi, "C:"), Ok(()));
    let disables: Vec<&String> = wmi
        .log
        .iter()
        .filter(|l| l.starts_with("DisableKeyProtectors"))
        .collect();
    assert_eq!(
        disables,
        [&format!("DisableKeyProtectors {C} DisableCount=u32:3")]
    );
    // The key is not read to suspend.
    assert_eq!(wmi.made("GetKeyProtectorNumericalPassword"), 0);
    assert_eq!(
        bitlocker::state(&mut wmi, "C:"),
        (Protection::Suspended, Some(3))
    );
}

#[test]
fn a_refused_suspension_is_a_managed_computers_and_leaves_nothing_half_done() {
    let mut wmi = recorded("managed.wmi");
    assert_eq!(
        bitlocker::suspend(&mut wmi, "C:"),
        Err(Reason::Managed(2_150_694_970))
    );
    assert_eq!(bitlocker::state(&mut wmi, "C:"), (Protection::On, None));

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
fn a_drive_already_suspended_is_left_as_it_is_and_resume_turns_it_on() {
    let mut wmi = recorded("control-panel.wmi");
    assert_eq!(bitlocker::suspend(&mut wmi, "C:"), Ok(()));
    assert_eq!(wmi.made("DisableKeyProtectors"), 0);
    let mut wmi = recorded("control-panel.wmi");
    assert_eq!(bitlocker::resume(&mut wmi, "C:"), Ok(()));
    assert_eq!(wmi.made("EnableKeyProtectors"), 1);
}

#[test]
fn a_drive_not_encrypted_has_no_key_and_nothing_to_suspend() {
    let mut wmi = recorded("not-encrypted.wmi");
    assert_eq!(
        bitlocker::read_key(&mut wmi, "C:"),
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
fn resume_enables_the_suspended_system_volume_and_reads_it_back() {
    let mut wmi = recorded("pro-resume.wmi");
    assert_eq!(bitlocker::resume(&mut wmi, "C:"), Ok(()));
    assert_eq!(wmi.made("EnableKeyProtectors"), 1);
    assert!(wmi.log.contains(&format!("EnableKeyProtectors {C}")));
    assert_eq!(bitlocker::state(&mut wmi, "C:"), (Protection::On, None));
}

#[test]
fn the_elevated_copy_reads_back_the_step_it_was_started_with() {
    for verb in [Verb::Key, Verb::Suspend, Verb::Resume] {
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
    assert!(bad(&["--bitlocker", "key"]));
    assert!(bad(&["--bitlocker", "show-key", "--pipe", PIPE]));
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
        "resume",
        "--pipe",
        PIPE,
        "--volume",
        C
    ]));
    assert!(bad(&["--write", "suspend", "--pipe", PIPE]));
}

#[test]
fn the_pipe_message_carrying_the_key_round_trips_and_is_wiped() {
    let answers = [
        Answer::Key(key()),
        Answer::State(Protection::Suspended, Some(3)),
        Answer::State(Protection::On, None),
        Answer::Done(Ok(())),
        Answer::Done(Err(Reason::Managed(2_150_694_970))),
        Answer::Done(Err(Reason::NoAdmin)),
        Answer::Done(Err(Reason::NoRecoveryKey)),
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
        String::from("key\tC:\t205568-492019\n"),
        format!("key\tC\t{KEY}\n"),
        format!("key\tC:\t{ID}\t{KEY}\n"),
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
        Verb::Key,
        "C:",
        &mut wire
    ));
    let reply = bitlocker::reply(&mut Cursor::new(wire)).expect("a reply");
    assert_eq!(reply.key, Some(key()));
    assert_eq!(reply.state, Some((Protection::On, None)));
    assert_eq!(reply.done, Some(Ok(())));

    let mut wire = Vec::new();
    assert!(bitlocker::run(
        &mut recorded("pro-suspend.wmi"),
        Verb::Suspend,
        "C:",
        &mut wire
    ));
    let reply = bitlocker::reply(&mut Cursor::new(wire)).expect("a reply");
    assert_eq!(reply.state, Some((Protection::Suspended, Some(3))));
    assert!(reply.key.is_none());

    let mut wire = Vec::new();
    assert!(!bitlocker::run(
        &mut recorded("managed.wmi"),
        Verb::Suspend,
        "C:",
        &mut wire
    ));
    let reply = bitlocker::reply(&mut Cursor::new(wire)).expect("a reply");
    assert_eq!(reply.done, Some(Err(Reason::Managed(2_150_694_970))));
    assert_eq!(reply.state, Some((Protection::On, None)));
}

#[test]
fn an_unreadable_protection_is_not_known() {
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
    assert_eq!(bitlocker::state(&mut wmi, "C:"), (Protection::None, None));
    // Not listed: not known, and no key to read.
    let mut wmi = recorded("not-encrypted.wmi");
    assert_eq!(
        bitlocker::state(&mut wmi, "Z:"),
        (Protection::Unknown, None)
    );
    assert_eq!(bitlocker::read_key(&mut wmi, "Z:"), Err(Reason::NotRead));
    // Protection off with conversion not at 0: suspended.
    let mut wmi = recorded("suspended-converted.wmi");
    assert_eq!(bitlocker::state(&mut wmi, "C:").0, Protection::Suspended);
    assert_eq!(wmi.made("GetKeyProtectors"), 0);
    // No system volume listed: nothing to suspend is not "not encrypted".
    let mut wmi = recorded("data-only.wmi");
    assert_eq!(bitlocker::suspend(&mut wmi, "C:"), Err(Reason::NotRead));
}

// The following three tests are against `tests/windows/vm-on.wmi` and
// `vm-suspended.wmi`: real `Win32_EncryptableVolume` answers from the
// Windows 11 Pro test virtual machine, not hand-made ones. On a real
// machine, `GetSuspendCount` on a protected (not suspended) volume
// answers with a failing `ReturnValue` (2147942450, 0x80070032) rather
// than the hand-made files' 0, and the removable FAT volume D: (not
// encrypted) answers it with another failing `ReturnValue` (2150694952,
// 0x80310028) too. `vm-suspended.wmi` was recorded after the 0.1.2
// Starter's suspension, which had no count.

#[test]
fn a_real_protected_drive_reads_as_protected() {
    // `ProtectionStatus` 1 is read as protected without `GetSuspendCount`
    // even being asked, so its failing `ReturnValue` changes nothing.
    let mut wmi = recorded("vm-on.wmi");
    assert_eq!(bitlocker::state(&mut wmi, "C:"), (Protection::On, None));
    assert_eq!(wmi.made("GetSuspendCount"), 0);
}

#[test]
fn a_real_suspended_drive_reads_as_suspended_until_resumed_and_is_left_as_it_is() {
    // `ProtectionStatus` 0 with `ConversionStatus` 1 is suspended, and
    // `SuspendCount` 0 is until resumed.
    let mut wmi = recorded("vm-suspended.wmi");
    assert_eq!(
        bitlocker::state(&mut wmi, "C:"),
        (Protection::Suspended, Some(0))
    );
    // Suspending it again changes nothing.
    let mut wmi = recorded("vm-suspended.wmi");
    assert_eq!(bitlocker::suspend(&mut wmi, "C:"), Ok(()));
    assert_eq!(wmi.made("DisableKeyProtectors"), 0);
}

#[test]
fn a_real_unencrypted_removable_drive_reads_as_not_encrypted() {
    let mut wmi = recorded("vm-on.wmi");
    assert_eq!(bitlocker::state(&mut wmi, "D:").0, Protection::None);
    let mut wmi = recorded("vm-suspended.wmi");
    assert_eq!(bitlocker::state(&mut wmi, "D:").0, Protection::None);
}
