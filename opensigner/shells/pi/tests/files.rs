//! What the card gives the device and what the device leaves on the card.
//!
//! The module is compiled into this test directly: the shell is a binary,
//! so there is no library to import, and the behaviour under test is the
//! choice of file, not the loop around it. Each test gets a directory of
//! its own under the target directory and plays the part of a person who
//! put files on the card from a computer, or who plugged a stick in after
//! the device was up.

#[path = "../src/files.rs"]
mod files;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use files::Files;
use osk_shell_api::FileKind;

/// An empty directory of this test's own, standing in for a mounted card,
/// and the channel that reads and writes it.
fn card(name: &str) -> (Files, PathBuf) {
    let dir: PathBuf = [env!("CARGO_TARGET_TMPDIR"), "files", name]
        .iter()
        .collect();
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the directory");
    }
    fs::create_dir_all(&dir).expect("make the directory");
    // A directory given by --files is not a mount point and is not
    // checked for being one.
    (Files::new(dir.clone(), false), dir)
}

/// Puts a file on the card, modified this many seconds ago.
fn put(dir: &Path, name: &str, bytes: &[u8], age_secs: u64) {
    let path = dir.join(name);
    fs::write(&path, bytes).expect("write the file");
    let when = SystemTime::now() - Duration::from_secs(age_secs);
    let handle = fs::File::options()
        .write(true)
        .open(&path)
        .expect("open the file");
    handle
        .set_times(fs::FileTimes::new().set_modified(when))
        .expect("set the time");
}

/// The names one listing carries, in the order the core will show them.
fn names(card: &Files, kind: FileKind) -> Vec<String> {
    card.list(kind)
        .expect("a listing")
        .into_iter()
        .map(|e| e.name)
        .collect()
}

#[test]
fn every_psbt_on_the_card_is_listed_newest_first() {
    let (card, dir) = card("newest-psbt");
    put(&dir, "old.psbt", b"older transaction", 600);
    put(&dir, "today.psbt", b"newer transaction", 5);
    put(&dir, "ancient.psbt", b"oldest transaction", 6000);

    assert_eq!(
        names(&card, FileKind::Psbt),
        ["today.psbt", "old.psbt", "ancient.psbt"]
    );
    assert_eq!(
        card.read(FileKind::Psbt, "old.psbt").unwrap(),
        b"older transaction",
        "the file the person chose, not the newest"
    );
}

#[test]
fn a_listed_file_carries_its_size_and_its_date() {
    let (card, dir) = card("size-and-date");
    put(&dir, "today.psbt", b"newer transaction", 5);

    let entries = card.list(FileKind::Psbt).expect("a listing");
    assert_eq!(entries[0].size, "newer transaction".len() as u64);
    let modified = entries[0].modified.expect("a date from the card");
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert!((now - 10..=now).contains(&modified), "{modified} vs {now}");
}

#[test]
fn a_card_with_no_psbt_on_it_lists_nothing() {
    let (card, dir) = card("no-psbt");
    put(&dir, "README.txt", b"the note the image wrote", 60);
    put(&dir, "holiday.jpg", b"not a transaction", 60);

    assert!(names(&card, FileKind::Psbt).is_empty());
}

#[test]
fn read_a_file_lists_anything_but_the_cards_note() {
    let (card, dir) = card("any-file");
    put(&dir, "README.txt", b"the note the image wrote", 1);
    put(&dir, ".Trashes", b"what a computer left behind", 1);
    put(&dir, "wallet.txt", b"a descriptor", 30);
    put(&dir, "older.txt", b"an older descriptor", 900);

    assert_eq!(names(&card, FileKind::Any), ["wallet.txt", "older.txt"]);
    assert_eq!(
        card.read(FileKind::Any, "wallet.txt").unwrap(),
        b"a descriptor"
    );
}

#[test]
fn a_name_the_card_never_listed_is_never_read() {
    let (card, dir) = card("not-listed");
    put(&dir, "wallet.txt", b"a descriptor", 30);
    put(&dir, "README.txt", b"the note the image wrote", 30);
    put(&dir, "notes.txt", b"not a transaction", 30);
    fs::write(dir.join("..").join("outside.txt"), b"another card's file").unwrap();

    // A file the rule does not allow, one from another directory, a
    // path, and a name that is not there at all.
    for name in [
        "README.txt",
        "../outside.txt",
        "sub/wallet.txt",
        "gone.txt",
        "notes.txt",
    ] {
        assert!(
            card.read(FileKind::Psbt, name).is_err(),
            "{name} was read for a transaction"
        );
    }
    assert!(card.read(FileKind::Any, "README.txt").is_err());
    assert!(card.read(FileKind::Any, "../outside.txt").is_err());
}

#[test]
fn a_saved_file_never_replaces_one_already_there() {
    let (card, _dir) = card("never-replace");
    let first = card.write("signed.psbt", b"one").unwrap();
    let second = card.write("signed.psbt", b"two").unwrap();
    let third = card.write("signed.psbt", b"three").unwrap();

    assert_eq!(first.file_name().unwrap(), "signed.psbt");
    assert_eq!(second.file_name().unwrap(), "signed-2.psbt");
    assert_eq!(third.file_name().unwrap(), "signed-3.psbt");
    assert_eq!(fs::read(&first).unwrap(), b"one");
}

#[test]
fn a_saved_file_holds_exactly_the_bytes_it_was_given() {
    let (card, _dir) = card("exact-bytes");
    // A PSBT is binary: a zero byte, a high byte and no trailing newline.
    let bytes: Vec<u8> = vec![0x70, 0x73, 0x62, 0x74, 0xff, 0x00, 0x01, 0xfe];

    let path = card.write("signed.psbt", &bytes).unwrap();

    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[test]
fn a_name_the_card_cannot_hold_is_still_saved() {
    let (card, dir) = card("awkward-name");
    let path = card.write("../signed tx?.psbt", b"one").unwrap();

    assert_eq!(path.parent().unwrap(), dir);
    assert!(
        path.file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    );
    assert_eq!(fs::read(&path).unwrap(), b"one");
}

#[test]
fn the_settings_the_device_keeps_are_never_offered_as_a_file() {
    let (card, dir) = card("settings");
    put(&dir, "wallet.txt", b"a descriptor", 900);
    let path = card
        .write_settings(b"opensigner-settings 1\nunit=btc\n")
        .unwrap();

    // Written just now, so it would win "the newest file" if it were a
    // file anyone had put there to read.
    assert_eq!(path.file_name().unwrap(), "opensigner-settings.txt");
    assert_eq!(names(&card, FileKind::Any), ["wallet.txt"]);
    assert!(card.read(FileKind::Any, "opensigner-settings.txt").is_err());
    // And the device reads them back as it left them.
    assert_eq!(
        card.read_settings().unwrap(),
        b"opensigner-settings 1\nunit=btc\n"
    );
}

#[test]
fn settings_a_second_time_replace_the_first() {
    let (card, dir) = card("settings-replace");
    card.write_settings(b"opensigner-settings 1\nunit=sat\n")
        .unwrap();
    card.write_settings(b"opensigner-settings 1\nunit=btc\n")
        .unwrap();

    assert_eq!(
        card.read_settings().unwrap(),
        b"opensigner-settings 1\nunit=btc\n"
    );
    let names: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_str().unwrap().to_string())
        .collect();
    assert_eq!(names, ["opensigner-settings.txt"]);
}

#[test]
fn the_timings_the_dev_card_records_are_never_offered_as_a_file() {
    let (card, dir) = card("timings");
    put(&dir, "wallet.txt", b"a descriptor", 900);
    let path = card
        .append_timings("opensigner-pi 0.1.0 panel 480x640\n")
        .unwrap();
    // A later block, and a later run, add to what is there.
    card.append_timings("     5s passes      97 touches      12\n")
        .unwrap();

    assert_eq!(path.file_name().unwrap(), "opensigner-timings.txt");
    assert_eq!(names(&card, FileKind::Any), ["wallet.txt"]);
    assert!(card.read(FileKind::Any, "opensigner-timings.txt").is_err());
    let kept = fs::read_to_string(&path).unwrap();
    assert!(
        kept.starts_with("opensigner-pi 0.1.0 panel 480x640"),
        "{kept}"
    );
    assert!(kept.contains("passes      97"), "{kept}");
}

#[test]
fn a_save_with_nowhere_to_put_it_is_refused() {
    let (card, dir) = card("no-card");
    // The card came out between the tap and the write.
    fs::remove_dir_all(&dir).expect("take the card out");

    assert!(card.write("signed.psbt", b"one").is_err());
}

/// The boot medium's partition and a stick plugged in after boot, as two
/// directories of this test's own. The second is the directory init would
/// have mounted the stick's partition at, under the name the partition
/// carries.
fn card_and_stick(name: &str, stick: &str) -> (Files, PathBuf, PathBuf) {
    let root: PathBuf = [env!("CARGO_TARGET_TMPDIR"), "files", name]
        .iter()
        .collect();
    if root.exists() {
        fs::remove_dir_all(&root).expect("clear the directory");
    }
    let card = root.join("OSKDATA");
    let usb = root.join("usb");
    let plugged = usb.join(stick);
    fs::create_dir_all(&card).expect("make the card's directory");
    fs::create_dir_all(&plugged).expect("make the stick's directory");
    (Files::at(card.clone(), usb), card, plugged)
}

#[test]
fn a_file_is_named_by_the_partition_it_is_on_when_two_are_plugged_in() {
    let (channel, card, stick) = card_and_stick("two-places", "KINGSTON");
    put(&card, "from-the-card.psbt", b"one transaction", 600);
    put(&stick, "from-the-stick.psbt", b"another transaction", 5);
    // The same name on both media is two files a person can tell apart.
    put(&card, "signed.psbt", b"the card's", 900);
    put(&stick, "signed.psbt", b"the stick's", 60);

    assert_eq!(
        names(&channel, FileKind::Psbt),
        [
            "KINGSTON/from-the-stick.psbt",
            "KINGSTON/signed.psbt",
            "OSKDATA/from-the-card.psbt",
            "OSKDATA/signed.psbt",
        ]
    );
    assert_eq!(channel.place().as_deref(), Some("OSKDATA, KINGSTON"));
    assert_eq!(
        channel.read(FileKind::Psbt, "OSKDATA/signed.psbt").unwrap(),
        b"the card's"
    );
    assert_eq!(
        channel
            .read(FileKind::Psbt, "KINGSTON/signed.psbt")
            .unwrap(),
        b"the stick's"
    );
    // A name the listing never carried is no file, whether it is the
    // bare name of a file on one of them or a path to somewhere else.
    for name in [
        "signed.psbt",
        "KINGSTON/../OSKDATA/signed.psbt",
        "SANDISK/signed.psbt",
        "OSKDATA/gone.psbt",
    ] {
        assert!(
            channel.read(FileKind::Psbt, name).is_err(),
            "{name} was read"
        );
    }
}

#[test]
fn a_file_keeps_its_own_name_when_only_one_partition_is_plugged_in() {
    let (channel, card, stick) = card_and_stick("one-place", "KINGSTON");
    put(&card, "signed.psbt", b"the card's", 60);
    fs::remove_dir_all(&stick).expect("unplug the stick");

    assert_eq!(names(&channel, FileKind::Psbt), ["signed.psbt"]);
    assert_eq!(channel.place().as_deref(), Some("OSKDATA"));
    assert_eq!(
        channel.read(FileKind::Psbt, "signed.psbt").unwrap(),
        b"the card's"
    );
}

#[test]
fn a_saved_file_goes_to_the_boot_medium_when_it_is_there() {
    let (channel, card, stick) = card_and_stick("save-first", "KINGSTON");

    assert_eq!(
        channel.write("signed.psbt", b"one").unwrap(),
        card.join("signed.psbt")
    );

    // With no boot medium to write to, the save goes to the stick.
    fs::remove_dir_all(&card).expect("take the card out");
    assert_eq!(
        channel.write("signed.psbt", b"two").unwrap(),
        stick.join("signed.psbt")
    );
    assert_eq!(names(&channel, FileKind::Psbt), ["signed.psbt"]);
}
