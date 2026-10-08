//! The settings file goes back over the settings file on a stick; any
//! other file of the same name is still written beside it, never over it.

use std::fs;
use std::path::PathBuf;

use faraday_storage::{Dirs, Sticks};

fn stick(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("faraday-settings-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_settings_file_replaces_the_one_on_the_stick() {
    let dir = stick("over");
    fs::write(
        dir.join("faraday-settings.txt"),
        "faraday-settings 1\ntheme=nord\n",
    )
    .unwrap();
    let id = dir.to_str().unwrap();
    let new = b"faraday-settings 1\ntheme=light\n";
    let wrote = Dirs.write(id, "faraday-settings.txt", new).unwrap();
    assert_eq!(wrote, "faraday-settings.txt");
    assert_eq!(fs::read(dir.join("faraday-settings.txt")).unwrap(), new);
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
}

#[test]
fn a_file_that_is_not_settings_is_not_written_over_it() {
    let dir = stick("beside");
    fs::write(
        dir.join("faraday-settings.txt"),
        "faraday-settings 1\ntheme=nord\n",
    )
    .unwrap();
    let id = dir.to_str().unwrap();
    let wrote = Dirs
        .write(id, "faraday-settings.txt", b"a note, not settings")
        .unwrap();
    assert_eq!(wrote, "faraday-settings-2.txt");
    assert_eq!(
        fs::read(dir.join("faraday-settings.txt")).unwrap(),
        b"faraday-settings 1\ntheme=nord\n"
    );
}
