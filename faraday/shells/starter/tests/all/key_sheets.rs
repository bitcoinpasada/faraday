//! The sweep of key sheets an earlier run left behind (`docs/STARTER.md`
//! §6, F49), and where a key sheet is written: only into a folder made
//! just then that only this user can open; the sweep zeroes and removes
//! the sheets in such folders and never follows a link out of them.

#![cfg(unix)]

use faraday_starter_shell::pdf::{key_base_from, key_sheet_in, sweep_key_sheets};
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

fn base(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("faraday-sweep-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn mode(path: &Path, bits: u32) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(bits)).unwrap();
}

fn key_folder(base: &Path, name: &str) -> PathBuf {
    let folder = base.join(name);
    std::fs::create_dir(&folder).unwrap();
    mode(&folder, 0o700);
    folder
}

#[test]
fn a_link_in_a_key_folder_survives_and_its_target_is_unchanged() {
    let base = base("link");
    let target = base.join("precious");
    std::fs::write(&target, b"not a key sheet").unwrap();
    let folder = key_folder(&base, "faraday-key-1-2");
    std::fs::write(folder.join("sheet.pdf"), b"key").unwrap();
    symlink(&target, folder.join("link")).unwrap();
    sweep_key_sheets(&base);
    assert_eq!(std::fs::read(&target).unwrap(), b"not a key sheet");
    assert!(folder.join("link").symlink_metadata().is_ok());
    assert!(!folder.join("sheet.pdf").exists(), "the sheet itself goes");
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn a_linked_key_folder_is_left_alone() {
    let base = base("folder");
    let elsewhere = base.join("elsewhere");
    std::fs::create_dir(&elsewhere).unwrap();
    std::fs::write(elsewhere.join("mine"), b"mine").unwrap();
    symlink(&elsewhere, base.join("faraday-key-9-9")).unwrap();
    sweep_key_sheets(&base);
    assert_eq!(std::fs::read(elsewhere.join("mine")).unwrap(), b"mine");
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn a_key_folder_others_can_open_is_left_alone() {
    let base = base("open");
    let folder = base.join("faraday-key-3-3");
    std::fs::create_dir(&folder).unwrap();
    mode(&folder, 0o755);
    std::fs::write(folder.join("sheet.pdf"), b"key").unwrap();
    sweep_key_sheets(&base);
    assert_eq!(std::fs::read(folder.join("sheet.pdf")).unwrap(), b"key");
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn a_sheet_is_written_into_a_fresh_folder_only_this_user_can_open() {
    let base = base("fresh");
    mode(&base, 0o700);
    let path = key_sheet_in(&base, "recovery.pdf", b"%PDF key").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"%PDF key");
    let folder = path.parent().unwrap();
    assert_eq!(folder.parent().unwrap(), base);
    let meta = folder.symlink_metadata().unwrap();
    assert!(meta.is_dir());
    assert_eq!(meta.permissions().mode() & 0o077, 0);
    // Another sheet goes into another folder.
    let other = key_sheet_in(&base, "recovery.pdf", b"%PDF key").unwrap();
    assert_ne!(other.parent(), path.parent());
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn a_folder_made_beforehand_under_a_fixed_name_is_never_written_into() {
    let base = base("fixed");
    for name in ["faraday-keys", "faraday-key-", "faraday-key"] {
        let planted = base.join(name);
        std::fs::create_dir(&planted).unwrap();
        mode(&planted, 0o777);
    }
    let path = key_sheet_in(&base, "recovery.pdf", b"%PDF key").unwrap();
    for name in ["faraday-keys", "faraday-key-", "faraday-key"] {
        let planted = base.join(name);
        assert_eq!(std::fs::read_dir(&planted).unwrap().count(), 0, "{name}");
        assert_ne!(path.parent().unwrap(), planted);
    }
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn a_runtime_folder_others_can_open_or_that_is_not_this_users_is_not_used() {
    let open = base("runtime-open");
    mode(&open, 0o777);
    assert_eq!(key_base_from(Some(open.clone())), std::env::temp_dir());
    // Not this user's, whoever runs the test: the root folder is
    // root's, and no one else's to use.
    assert_eq!(
        key_base_from(Some(PathBuf::from("/"))),
        std::env::temp_dir()
    );
    // A link to a good folder is not the folder itself.
    let own = base("runtime-own");
    mode(&own, 0o700);
    assert_eq!(key_base_from(Some(own.clone())), own);
    let link = open.join("link");
    symlink(&own, &link).unwrap();
    assert_eq!(key_base_from(Some(link)), std::env::temp_dir());
    let _ = std::fs::remove_dir_all(&open);
    let _ = std::fs::remove_dir_all(&own);
}
