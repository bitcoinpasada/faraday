//! A stick that has been in a Windows computer or a Mac holds their own
//! files beside the PSBT a person put there (`docs/STARTER.md` §4.4,
//! D11): Faraday lists, counts and reads the PSBT alone, at the boot
//! stick's import and on a visit, and a file Faraday writes
//! leaves the PSBT and their files as they were.

use std::fs;
use std::path::{Path, PathBuf};

use crate::compute;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageCommand, StorageEvent, testkit};
use faraday_storage::{read, stick_info, write};

const PSBT: &str = "savings-unsigned.psbt";

/// What Windows and macOS leave on a FAT volume they mount, as files and
/// folders at its top level.
const FOREIGN_FILES: [&str; 6] = [
    "desktop.ini",
    "Thumbs.db",
    ".DS_Store",
    "._savings-unsigned.psbt",
    "System Volume Information/IndexerVolumeGuid",
    "System Volume Information/WPSettings.dat",
];
const FOREIGN_DIRS: [&str; 4] = ["$RECYCLE.BIN", ".Spotlight-V100", ".fseventsd", ".Trashes"];

fn psbt() -> Vec<u8> {
    testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == PSBT)
        .unwrap()
        .1
}

/// A folder standing for a stick that has been in a Windows computer and
/// a Mac, with one PSBT a person put there.
fn visited_stick(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("faraday-foreign-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    for d in FOREIGN_DIRS {
        fs::create_dir_all(dir.join(d)).unwrap();
    }
    fs::create_dir_all(dir.join("System Volume Information")).unwrap();
    fs::write(
        dir.join("$RECYCLE.BIN/desktop.ini"),
        b"[.ShellClassInfo]\r\n",
    )
    .unwrap();
    for f in FOREIGN_FILES {
        fs::write(dir.join(f), b"an operating system's own bytes").unwrap();
    }
    fs::write(dir.join(PSBT), psbt()).unwrap();
    dir
}

/// Answers the app's reads from the stick folder, as the shell does.
fn pump(app: &mut Faraday, dir: &Path) -> Vec<String> {
    let mut asked = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Read { stick, name } = c {
            asked.push(name.clone());
            let bytes = read(&dir.display().to_string(), &name).unwrap();
            app.storage(StorageEvent::Read { stick, name, bytes });
        }
    }
    asked
}

fn inbox(app: &Faraday) -> Vec<&str> {
    app.inbox().iter().map(|i| i.name.as_str()).collect()
}

#[test]
fn the_boot_stick_reads_and_counts_the_psbt_alone() {
    let dir = visited_stick("boot");
    let mut app = testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Sticks(vec![stick_info(
        &compute(),
        &dir,
        "FARADAYDATA",
        true,
    )]));
    // Listed, nothing read before Import (F130).
    assert!(pump(&mut app, &dir).is_empty(), "a file read before Import");
    assert_eq!(app.sheet, Some(Sheet::Import));
    let texts = app.drawn_texts().join("\n");
    assert!(texts.contains("1 PSBT"), "{texts}");
    assert!(!texts.contains("other file"), "{texts}");
    assert!(inbox(&app).is_empty());
    // Import reads the PSBT alone and brings it in (F60).
    app.press(Action::Import(faraday_core::boot_import::ImportAction::Go));
    let asked = pump(&mut app, &dir);
    assert_eq!(asked, [PSBT], "only the PSBT is read");
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert_eq!(inbox(&app), [PSBT]);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_visit_lists_and_copies_the_psbt_alone() {
    let dir = visited_stick("visit");
    let mut app = testkit::started();
    app.storage(StorageEvent::Sticks(vec![stick_info(
        &compute(),
        &dir,
        "KINGSTON",
        false,
    )]));
    app.press(Action::Nav(Screen::Visit));
    assert_eq!(
        app.sticks()[0].files,
        [(PSBT.to_string(), psbt().len() as u64)],
        "the visit counts the PSBT alone"
    );
    let texts = app.drawn_texts().join("\n");
    for f in FOREIGN_FILES.iter().chain(&FOREIGN_DIRS) {
        let top = f.split('/').next().unwrap();
        assert!(!texts.contains(top), "{top} is shown:\n{texts}");
    }
    // Everything on the stick is ticked to come in: the PSBT alone.
    assert_eq!(app.visit().inn.iter().collect::<Vec<_>>(), [PSBT]);
    app.press(Action::VisitCopy);
    assert_eq!(pump(&mut app, &dir), [PSBT]);
    assert_eq!(inbox(&app), [PSBT]);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_file_faraday_writes_leaves_the_psbt_and_their_files_as_they_were() {
    let dir = visited_stick("write");
    let id = dir.display().to_string();
    let wrote = write(&id, "savings-signed.psbt", b"psbt\xffsigned").unwrap();
    assert_eq!(wrote, "savings-signed.psbt");
    assert_eq!(read(&id, PSBT).unwrap(), psbt());
    for f in FOREIGN_FILES {
        assert_eq!(
            fs::read(dir.join(f)).unwrap(),
            b"an operating system's own bytes",
            "{f}"
        );
    }
    for d in FOREIGN_DIRS {
        assert!(dir.join(d).is_dir(), "{d}");
    }
    assert!(
        read(&id, "desktop.ini").is_err(),
        "Windows' own file is never read"
    );
    let names: Vec<String> = stick_info(&compute(), &dir, "KINGSTON", false)
        .files
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    assert_eq!(names, ["savings-signed.psbt", PSBT]);
    let _ = fs::remove_dir_all(&dir);
}
