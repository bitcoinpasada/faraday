//! Restart from the stick on Linux (`docs/STARTER.md` §3.1): `efibootmgr`'s
//! listing as recorded (`tests/efibootmgr/`), and the command lines the
//! Starter runs, through a fake `pkexec`, `efibootmgr` and `systemctl`
//! in a temporary directory that is the whole `PATH`. Each fake writes
//! its arguments, one `[…]` each, to a log beside it; no real one is ever
//! run.

#![cfg(target_os = "linux")]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use faraday_starter::api::Reason;
use faraday_starter_shell::linux::boot::{
    self, Entry, Tools, entries, entry_exists, faraday_entries, free_number, remove_entry,
};

const SEVERAL: &str = include_str!("../efibootmgr/several.txt");
const FARADAY: &str = include_str!("../efibootmgr/faraday-verbose.txt");
const ONE: &str = include_str!("../efibootmgr/one.txt");

/// Writes its arguments to `<name>.log`, then does what the files
/// beside it say.
const LOG: &str = r#"d=$(dirname "$0"); n=$(basename "$0")
for a in "$@"; do printf '[%s]' "$a"; done >> "$d/$n.log"; echo >> "$d/$n.log"
"#;

const PKEXEC: &str = r#"[ -f "$d/pkexec-exit" ] && exit "$(cat "$d/pkexec-exit")"
exec "$@"
"#;

const EFIBOOTMGR: &str = r#"if [ $# -eq 0 ]; then
  [ -f "$d/list-fails" ] && { echo "EFI variables are not supported on this system." >&2; exit 2; }
  cat "$d/listing.txt"; exit 0
fi
[ -f "$d/write-fails" ] && { echo "Could not set BootNext: Invalid argument" >&2; exit 5; }
exit 0
"#;

/// A directory holding the fakes `programs` and the listing.
fn fakes(name: &str, listing: &str, programs: &[&str]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("starter-boot-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("folder");
    fs::write(dir.join("listing.txt"), listing).expect("listing");
    for p in programs {
        let body = match *p {
            "pkexec" => PKEXEC,
            "efibootmgr" => EFIBOOTMGR,
            _ => "exit 0\n",
        };
        let file = dir.join(p);
        fs::write(&file, format!("#!/bin/sh\n{LOG}{body}")).expect("fake");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("mode");
    }
    dir
}

const ALL: [&str; 3] = ["pkexec", "efibootmgr", "systemctl"];

/// What `program` was run with, one line per run.
fn log(dir: &Path, program: &str) -> Vec<String> {
    fs::read_to_string(dir.join(format!("{program}.log")))
        .map(|s| s.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

fn restart(dir: &Path, device: &str) -> Result<(), Reason> {
    boot::restart_from_stick(&Tools::find(dir.as_os_str())?, device)
}

fn efibootmgr(dir: &Path) -> String {
    dir.join("efibootmgr").display().to_string()
}

#[test]
fn the_listing_gives_each_entry_and_ignores_the_other_lines() {
    let found = entries(SEVERAL);
    assert_eq!(
        found.iter().map(|e| e.num).collect::<Vec<_>>(),
        vec![0x0000, 0x0001, 0x0002, 0x2001, 0x2002]
    );
    assert_eq!(
        found[0],
        Entry {
            num: 0,
            label: "Windows Boot Manager".into()
        }
    );
    // BootNext names 0003, which is not an entry: it is free.
    assert!(faraday_entries(SEVERAL).is_empty());
    assert_eq!(free_number(SEVERAL), Some(3));
    assert_eq!(free_number(ONE), Some(1));
    assert!(faraday_entries(ONE).is_empty());
}

#[test]
fn an_existing_faraday_stick_entry_is_found_by_its_label() {
    assert_eq!(faraday_entries(FARADAY), vec![0x0004]);
    // `-v`'s device path after the tab is not part of the label.
    assert_eq!(entries(FARADAY)[2].label, "Faraday stick");
}

#[test]
fn with_no_entry_one_pkexec_creates_it_sets_bootnext_and_the_computer_restarts() {
    let dir = fakes("create", SEVERAL, &ALL);
    assert_eq!(restart(&dir, "/dev/sdb"), Ok(()));
    let e = efibootmgr(&dir);
    assert_eq!(
        log(&dir, "pkexec"),
        vec![format!(
            "[{e}][--create-only][--bootnum][0003][--disk][/dev/sdb][--part][1]\
             [--loader][\\EFI\\BOOT\\BOOTX64.EFI][--label][Faraday stick][--bootnext][0003]"
        )]
    );
    // Read as the person, then written through pkexec.
    assert_eq!(log(&dir, "efibootmgr").len(), 2);
    assert_eq!(log(&dir, "systemctl"), vec!["[reboot]"]);
}

#[test]
fn an_existing_entry_is_reused_and_only_bootnext_is_set() {
    let dir = fakes("reuse", FARADAY, &ALL);
    assert_eq!(restart(&dir, "/dev/sdc"), Ok(()));
    let e = efibootmgr(&dir);
    assert_eq!(
        log(&dir, "pkexec"),
        vec![format!("[{e}][--bootnext][0004]")]
    );
    assert_eq!(log(&dir, "systemctl"), vec!["[reboot]"]);
}

#[test]
fn a_missing_program_restarts_nothing_and_says_which() {
    for (missing, programs) in [
        ("efibootmgr", ["pkexec", "systemctl"]),
        ("pkexec", ["efibootmgr", "systemctl"]),
    ] {
        let dir = fakes(&format!("missing-{missing}"), SEVERAL, &programs);
        assert_eq!(
            restart(&dir, "/dev/sdb"),
            Err(Reason::Missing(missing.into()))
        );
        assert!(log(&dir, "efibootmgr").is_empty());
        assert!(log(&dir, "systemctl").is_empty());
    }
}

#[test]
fn a_declined_password_restarts_nothing() {
    for code in ["126", "127"] {
        let dir = fakes(&format!("declined-{code}"), SEVERAL, &ALL);
        fs::write(dir.join("pkexec-exit"), code).expect("exit code");
        assert_eq!(restart(&dir, "/dev/sdb"), Err(Reason::Declined));
        assert_eq!(log(&dir, "pkexec").len(), 1);
        // Only the listing ran; nothing was written.
        assert_eq!(log(&dir, "efibootmgr"), vec![""]);
        assert!(log(&dir, "systemctl").is_empty());
    }
}

#[test]
fn efibootmgr_failing_restarts_nothing_and_says_its_words() {
    let dir = fakes("write-fails", SEVERAL, &ALL);
    fs::write(dir.join("write-fails"), "").expect("flag");
    assert_eq!(
        restart(&dir, "/dev/sdb"),
        Err(Reason::BootEntry(
            "Could not set BootNext: Invalid argument".into()
        ))
    );
    assert!(log(&dir, "systemctl").is_empty());

    let dir = fakes("list-fails", SEVERAL, &ALL);
    fs::write(dir.join("list-fails"), "").expect("flag");
    assert_eq!(
        restart(&dir, "/dev/sdb"),
        Err(Reason::BootEntry(
            "EFI variables are not supported on this system.".into()
        ))
    );
    assert!(log(&dir, "pkexec").is_empty());
    assert!(log(&dir, "systemctl").is_empty());
}

#[test]
fn only_a_plain_device_name_reaches_the_command_line() {
    let dir = fakes("device", SEVERAL, &ALL);
    for device in ["/dev/sdb --bootnext 0000", "/dev/../sdb", "sdb", "/dev/"] {
        assert_eq!(restart(&dir, device), Err(Reason::NoStick));
    }
    assert!(log(&dir, "efibootmgr").is_empty());
    assert!(log(&dir, "pkexec").is_empty());
}

#[test]
fn uninstall_removes_the_faraday_stick_entry_and_nothing_else() {
    let dir = fakes("uninstall", FARADAY, &ALL);
    assert_eq!(remove_entry(dir.as_os_str()), Ok(()));
    let e = efibootmgr(&dir);
    assert_eq!(
        log(&dir, "pkexec"),
        vec![format!("[{e}][--delete-bootnum][--bootnum][0004]")]
    );

    // No entry: no password asked.
    let dir = fakes("uninstall-none", SEVERAL, &ALL);
    assert_eq!(remove_entry(dir.as_os_str()), Ok(()));
    assert!(log(&dir, "pkexec").is_empty());

    // No efibootmgr: there can be no entry.
    let dir = fakes("uninstall-no-efibootmgr", FARADAY, &["pkexec"]);
    assert_eq!(remove_entry(dir.as_os_str()), Ok(()));
    assert!(log(&dir, "pkexec").is_empty());
}

#[test]
fn uninstall_declined_leaves_the_entry_and_says_why() {
    let dir = fakes("uninstall-declined", FARADAY, &ALL);
    fs::write(dir.join("pkexec-exit"), "126").expect("exit code");
    assert_eq!(remove_entry(dir.as_os_str()), Err(Reason::Declined));
    assert_eq!(log(&dir, "efibootmgr"), vec![""]);
}

#[test]
fn home_knows_the_entry_from_the_listing_read_as_the_person() {
    let dir = fakes("exists", FARADAY, &ALL);
    assert!(entry_exists(dir.as_os_str()));
    // Read without a password: pkexec never ran.
    assert!(log(&dir, "pkexec").is_empty());
    assert!(!entry_exists(
        fakes("exists-none", SEVERAL, &ALL).as_os_str()
    ));
    assert!(!entry_exists(
        fakes("exists-no-efibootmgr", FARADAY, &["pkexec"]).as_os_str()
    ));
    let dir = fakes("exists-list-fails", FARADAY, &ALL);
    fs::write(dir.join("list-fails"), "").expect("flag");
    assert!(!entry_exists(dir.as_os_str()));
}

#[test]
fn removing_the_entry_from_home_asks_once_and_declined_keeps_it() {
    let dir = fakes("home-remove", FARADAY, &ALL);
    assert_eq!(remove_entry(dir.as_os_str()), Ok(()));
    assert_eq!(log(&dir, "pkexec").len(), 1);
    let dir = fakes("home-remove-declined", FARADAY, &ALL);
    fs::write(dir.join("pkexec-exit"), "127").expect("exit code");
    assert_eq!(remove_entry(dir.as_os_str()), Err(Reason::Declined));
    assert!(entry_exists(dir.as_os_str()));
}
