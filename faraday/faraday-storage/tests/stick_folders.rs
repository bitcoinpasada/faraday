//! Each stick pill is a folder of its own: what was written to Stick 3
//! is still there once Stick 3 is pulled and put back, and the other
//! sticks do not hold it. The boot stick is Stick 1; a backup's sticks
//! are Stick 2 and on, in `STICK-2` and on.

use std::fs;
use std::path::PathBuf;

use faraday_core::backup_folder;
use faraday_storage::{stick_info, write};

/// A fresh folder standing for the sticks' folder of the desktop app.
fn sticks_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("faraday-stick-folders-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The sticks that are in: a folder is a stick while it is attached.
fn in_now(dir: &std::path::Path, attached: &[String]) -> Vec<faraday_core::StickInfo> {
    attached
        .iter()
        .map(|name| stick_info(&dir.join(name), name, false))
        .collect()
}

#[test]
fn a_vault_written_to_stick_3_is_there_after_it_is_pulled_and_put_back() {
    let dir = sticks_dir();
    let (one, two) = (backup_folder(2), backup_folder(3));
    assert_eq!((one.as_str(), two.as_str()), ("STICK-2", "STICK-3"));
    for name in [&one, &two] {
        fs::create_dir_all(dir.join(name)).unwrap();
    }
    let vault = vec![7u8; 4096];
    let id = dir.join(&two).display().to_string();
    let wrote = write(&id, "vault.ofv", &vault).unwrap();
    // Stick 3 pulled, Stick 2 stays in.
    let attached = vec![one.clone()];
    assert!(
        in_now(&dir, &attached)
            .iter()
            .all(|s| s.label != two && s.files.is_empty())
    );
    // Stick 3 put back.
    let attached = vec![one.clone(), two.clone()];
    let sticks = in_now(&dir, &attached);
    let back = sticks.iter().find(|s| s.label == two).unwrap();
    assert_eq!(back.files, vec![(wrote.clone(), vault.len() as u64)]);
    assert_eq!(fs::read(dir.join(&two).join(&wrote)).unwrap(), vault);
    let first = sticks.iter().find(|s| s.label == one).unwrap();
    assert!(first.files.is_empty(), "Stick 2 holds nothing of it");
    let _ = fs::remove_dir_all(&dir);
}
