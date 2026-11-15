//! Restart from the stick on Linux (`docs/STARTER.md` §3.1): `efibootmgr`'s
//! listing as recorded (`tests/efibootmgr/`), and the command lines the
//! Starter runs, through a fake `pkexec`, `efibootmgr` and `systemctl`
//! in a temporary directory that is the whole `PATH`. Each fake writes
//! its arguments, one `[…]` each, to a log beside it; no real one is ever
//! run.

#![cfg(target_os = "linux")]

use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use faraday_starter::api::Reason;
use faraday_starter_shell::linux::boot::{
    self, Entry, Tools, Trusted, entries, entry_exists, faraday_entries, free_number, plan,
    remove_entry,
};

const SEVERAL: &str = include_str!("../efibootmgr/several.txt");
const FARADAY: &str = include_str!("../efibootmgr/faraday-verbose.txt");
const ONE: &str = include_str!("../efibootmgr/one.txt");
const TWO_STALE: &str = include_str!("../efibootmgr/faraday-two-stale.txt");

/// The boot partition GUID `FARADAY`'s entry names, and one it does not.
const GUID: &str = "46617261-6461-792d-5374-69636b000001";
const OTHER_GUID: &str = "46617261-6461-792d-5374-69636b0000ff";

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
[ -f "$d/delete-fails" ] && [ "$1" = "--delete-bootnum" ] && { echo "Could not delete" >&2; exit 5; }
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

/// Only `dir`, and the user who made it (the fakes' owner).
fn trusted(dir: &Path) -> Trusted {
    Trusted {
        dirs: vec![dir.to_path_buf()],
        uid: fs::metadata(dir).expect("folder").uid(),
    }
}

fn restart_with(dir: &Path, device: &str, guid: &str) -> Result<(), Reason> {
    boot::restart_from_stick(&Tools::find(&trusted(dir))?, device, guid)
}

fn restart(dir: &Path, device: &str) -> Result<(), Reason> {
    restart_with(dir, device, GUID)
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
            label: "Windows Boot Manager".into(),
            guid: None,
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
fn an_entry_names_its_partition_guid_in_lower_case() {
    assert_eq!(entries(FARADAY)[2].guid.as_deref(), Some(GUID));
    // The plain listing has no device paths.
    assert!(entries(SEVERAL).iter().all(|e| e.guid.is_none()));
    let upper = "Boot0004  Faraday stick\tHD(1,GPT,46617261-6461-792D-5374-69636B000001,0x800,0x1)/File(x)\n";
    assert_eq!(entries(upper)[0].guid.as_deref(), Some(GUID));
}

#[test]
fn an_entry_for_another_stick_is_replaced_and_one_for_this_stick_is_kept() {
    let kept = plan(FARADAY, "/dev/sdc", GUID).expect("plan");
    assert_eq!(kept.set, vec!["--bootnext", "0004"]);
    assert!(kept.stale.is_empty());
    // Any case of the same GUID is the same partition.
    let upper = plan(FARADAY, "/dev/sdc", &GUID.to_ascii_uppercase()).expect("plan");
    assert_eq!(upper.set, vec!["--bootnext", "0004"]);

    let other = plan(FARADAY, "/dev/sdc", OTHER_GUID).expect("plan");
    assert_eq!(other.set[..3], ["--create-only", "--bootnum", "0002"]);
    assert_eq!(other.stale, vec![4]);

    // The one that matches is used; the other is left alone.
    let two = plan(
        TWO_STALE,
        "/dev/sdc",
        "46617261-6461-792d-5374-69636b000002",
    )
    .expect("plan");
    assert_eq!(two.set, vec!["--bootnext", "0005"]);

    assert_eq!(plan(FARADAY, "/dev/sdc", ""), Err(Reason::NoStick));
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
fn an_entry_naming_another_stick_is_replaced_in_the_same_restart() {
    let dir = fakes("stale", TWO_STALE, &ALL);
    assert_eq!(restart_with(&dir, "/dev/sdc", OTHER_GUID), Ok(()));
    let e = efibootmgr(&dir);
    assert_eq!(
        log(&dir, "pkexec"),
        vec![
            format!(
                "[{e}][--create-only][--bootnum][0002][--disk][/dev/sdc][--part][1]\
                 [--loader][\\EFI\\BOOT\\BOOTX64.EFI][--label][Faraday stick][--bootnext][0002]"
            ),
            format!("[{e}][--delete-bootnum][--bootnum][0004]"),
            format!("[{e}][--delete-bootnum][--bootnum][0005]"),
        ]
    );
    assert_eq!(log(&dir, "systemctl"), vec!["[reboot]"]);
}

#[test]
fn a_stale_entry_that_cannot_be_deleted_does_not_stop_the_restart() {
    let dir = fakes("stale-delete-fails", FARADAY, &ALL);
    fs::write(dir.join("delete-fails"), "").expect("flag");
    assert_eq!(restart_with(&dir, "/dev/sdc", OTHER_GUID), Ok(()));
    assert_eq!(log(&dir, "systemctl"), vec!["[reboot]"]);
}

#[test]
fn a_program_not_owned_by_the_trusted_owner_or_writable_by_others_is_not_run() {
    let dir = fakes("untrusted", SEVERAL, &ALL);
    let mine = trusted(&dir);
    assert!(Tools::find(&mine).is_ok());
    let not_mine = Trusted {
        uid: mine.uid + 1,
        ..mine.clone()
    };
    assert_eq!(
        Tools::find(&not_mine),
        Err(Reason::Missing("efibootmgr".into()))
    );
    for mode in [0o775, 0o757] {
        fs::set_permissions(dir.join("efibootmgr"), fs::Permissions::from_mode(mode))
            .expect("mode");
        assert_eq!(
            Tools::find(&mine),
            Err(Reason::Missing("efibootmgr".into()))
        );
    }
}

#[test]
fn the_system_lookup_is_root_owned_files_in_the_system_folders_only() {
    let system = Trusted::system();
    assert_eq!(system.uid, 0);
    assert_eq!(
        system.dirs,
        ["/usr/sbin", "/usr/bin", "/sbin", "/bin"].map(PathBuf::from)
    );
    // A fake in a folder of the person's is in none of them.
    let dir = fakes("home-folder", SEVERAL, &ALL);
    assert!(system.dirs.iter().all(|d| !d.starts_with(&dir)));
    assert!(
        system
            .which("efibootmgr")
            .is_none_or(|p| !p.starts_with(&dir))
    );
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
    assert_eq!(remove_entry(&trusted(&dir)), Ok(()));
    let e = efibootmgr(&dir);
    assert_eq!(
        log(&dir, "pkexec"),
        vec![format!("[{e}][--delete-bootnum][--bootnum][0004]")]
    );

    // No entry: no password asked.
    let dir = fakes("uninstall-none", SEVERAL, &ALL);
    assert_eq!(remove_entry(&trusted(&dir)), Ok(()));
    assert!(log(&dir, "pkexec").is_empty());

    // No efibootmgr: there can be no entry.
    let dir = fakes("uninstall-no-efibootmgr", FARADAY, &["pkexec"]);
    assert_eq!(remove_entry(&trusted(&dir)), Ok(()));
    assert!(log(&dir, "pkexec").is_empty());
}

#[test]
fn uninstall_declined_leaves_the_entry_and_says_why() {
    let dir = fakes("uninstall-declined", FARADAY, &ALL);
    fs::write(dir.join("pkexec-exit"), "126").expect("exit code");
    assert_eq!(remove_entry(&trusted(&dir)), Err(Reason::Declined));
    assert_eq!(log(&dir, "efibootmgr"), vec![""]);
}

#[test]
fn home_knows_the_entry_from_the_listing_read_as_the_person() {
    let dir = fakes("exists", FARADAY, &ALL);
    assert!(entry_exists(&trusted(&dir)));
    // Read without a password: pkexec never ran.
    assert!(log(&dir, "pkexec").is_empty());
    assert!(!entry_exists(&trusted(&fakes(
        "exists-none",
        SEVERAL,
        &ALL
    ))));
    assert!(!entry_exists(&trusted(&fakes(
        "exists-no-efibootmgr",
        FARADAY,
        &["pkexec"]
    ))));
    let dir = fakes("exists-list-fails", FARADAY, &ALL);
    fs::write(dir.join("list-fails"), "").expect("flag");
    assert!(!entry_exists(&trusted(&dir)));
}

#[test]
fn removing_the_entry_from_home_asks_once_and_declined_keeps_it() {
    let dir = fakes("home-remove", FARADAY, &ALL);
    assert_eq!(remove_entry(&trusted(&dir)), Ok(()));
    assert_eq!(log(&dir, "pkexec").len(), 1);
    let dir = fakes("home-remove-declined", FARADAY, &ALL);
    fs::write(dir.join("pkexec-exit"), "127").expect("exit code");
    assert_eq!(remove_entry(&trusted(&dir)), Err(Reason::Declined));
    assert!(entry_exists(&trusted(&dir)));
}
