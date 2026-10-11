//! Copying files from one stick to another, as a backup: a stick visit's
//! write list offers the Inbox's files under "From the Inbox". A public
//! or sealed file is written as it came, a picture's bytes too; one that
//! may be a secret is written only once the secret sheet's line is
//! ticked; a file larger than the disk process reads is not read.

use std::collections::BTreeMap;
use std::path::PathBuf;

use faraday_core::secrets::Ack;
use faraday_core::wallet::FileKind;
use faraday_core::{
    Action, Faraday, Screen, Sheet, StickInfo, StorageCommand, StorageEvent, testkit,
};
use osk_shell_api::App;

/// A stick: its id, and its files by name.
struct Stick {
    id: &'static str,
    files: BTreeMap<String, Vec<u8>>,
    /// Where the stick's files are on disk, read as the disk process
    /// reads them; `None` reads them from `files`.
    dir: Option<PathBuf>,
}

impl Stick {
    fn new(id: &'static str, files: &[(String, Vec<u8>)]) -> Stick {
        Stick {
            id,
            files: files.iter().cloned().collect(),
            dir: None,
        }
    }

    /// The stick as the shell lists it.
    fn info(&self) -> StickInfo {
        StickInfo {
            id: self.id.to_string(),
            label: self.id.to_uppercase(),
            boot: false,
            files: self
                .files
                .iter()
                .map(|(n, b)| (n.clone(), b.len() as u64))
                .collect(),
        }
    }
}

/// The test kit's file `name`.
fn kit(name: &str) -> (String, Vec<u8>) {
    testkit::files()
        .expect("the test kit")
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no {name} in the test kit"))
}

/// The test kit's first file whose name ends with `end`.
fn kit_ending(end: &str) -> (String, Vec<u8>) {
    testkit::files()
        .expect("the test kit")
        .into_iter()
        .find(|(n, _)| n.ends_with(end))
        .unwrap_or_else(|| panic!("no *{end} in the test kit"))
}

/// The sticks `in_now` attached, the visit open on the first.
fn attach(app: &mut Faraday, in_now: &[&Stick]) {
    app.storage(StorageEvent::Sticks(
        in_now.iter().map(|s| s.info()).collect(),
    ));
    app.press(Action::Nav(Screen::Visit));
    let _ = app.frame();
}

/// Answers what the app asks of `sticks` as a shell does; returns the
/// writes, as (stick, name, bytes).
fn pump(app: &mut Faraday, sticks: &[&Stick]) -> Vec<(String, String, Vec<u8>)> {
    let mut wrote = Vec::new();
    while let Some(c) = app.poll_storage() {
        match c {
            StorageCommand::Read { stick, name } => {
                let s = sticks.iter().find(|s| s.id == stick).expect("a stick");
                let bytes = s.files[&name].clone();
                app.storage(StorageEvent::Read { stick, name, bytes });
            }
            StorageCommand::ReadQr { stick, name } => {
                let s = sticks.iter().find(|s| s.id == stick).expect("a stick");
                // The disk process's own reader, on the stick's files.
                let (bytes, payloads) = match &s.dir {
                    Some(dir) => {
                        faraday_files::read_qr_png(&mut faraday_files::Dir(dir.clone()), &name)
                            .expect("read")
                    }
                    None => {
                        let bytes = s.files[&name].clone();
                        let codes = faraday_files::qr_in_png(&bytes).unwrap_or_default();
                        (bytes, codes)
                    }
                };
                app.storage(StorageEvent::QrRead {
                    name,
                    bytes,
                    payloads,
                });
            }
            StorageCommand::Write { stick, name, bytes } => {
                wrote.push((stick.clone(), name.clone(), bytes));
                app.storage(StorageEvent::Written {
                    stick,
                    wrote_as: name.clone(),
                    name,
                });
            }
            _ => {}
        }
    }
    let _ = app.frame();
    wrote
}

/// Copies `names` in from the stick on show.
fn copy_in(app: &mut Faraday, sticks: &[&Stick], names: &[&str]) {
    let shown = &app.sticks[app.visit.stick];
    let ks: Vec<usize> = names
        .iter()
        .map(|n| {
            shown
                .files
                .iter()
                .position(|(f, _)| f == n)
                .unwrap_or_else(|| panic!("{n} is not on the stick"))
        })
        .collect();
    // Those alone: Unselect all, then each ticked.
    app.press(Action::VisitInAll);
    for k in ks {
        app.press(Action::VisitIn(k));
    }
    app.press(Action::VisitCopy);
    pump(app, sticks);
}

/// The visit's write-list action for the Inbox file `name`, if it offers
/// it.
fn row(app: &Faraday, name: &str) -> Option<Action> {
    app.visit_inbox_rows()
        .into_iter()
        .find(|&k| app.inbox[k].name == name)
        .map(Action::VisitInbox)
}

/// Write pressed on the visit; the files written.
fn write(app: &mut Faraday, sticks: &[&Stick]) -> Vec<(String, String, Vec<u8>)> {
    app.press(Action::VisitWrite);
    pump(app, sticks)
        .into_iter()
        .filter(|(_, n, _)| !faraday_core::stick_settings::is_file(n))
        .collect()
}

/// A folder standing for a stick, holding `files`.
fn folder(tag: &str, files: &[(String, Vec<u8>)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("faraday-stick-copy-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    for (n, b) in files {
        std::fs::write(dir.join(n), b).expect("written");
    }
    dir
}

#[test]
fn a_public_file_copied_from_one_stick_is_written_to_the_next_as_it_came() {
    let mut app = testkit::started();
    let share = kit("savings-share-1-of-3.txt");
    let backup = kit_ending(".oskb");
    let a = Stick::new("a", &[share.clone(), backup.clone()]);
    attach(&mut app, &[&a]);
    copy_in(&mut app, &[&a], &[&share.0, &backup.0]);
    // On the stick it came from it is not offered again.
    assert_eq!(row(&app, &share.0), None);

    app.storage(StorageEvent::Sticks(Vec::new()));
    let b = Stick::new("b", &[]);
    attach(&mut app, &[&b]);
    let share_row = row(&app, &share.0).expect("not offered under From the Inbox");
    assert!(app.offers(share_row), "not on screen");
    assert!(
        app.visit.from_inbox.is_empty(),
        "an Inbox file is ticked before anyone chose it"
    );
    app.press(share_row);
    app.press(row(&app, &backup.0).unwrap());
    let wrote = write(&mut app, &[&b]);
    assert_eq!(
        wrote.len(),
        2,
        "{:?}",
        wrote.iter().map(|w| &w.1).collect::<Vec<_>>()
    );
    for (stick, name, bytes) in &wrote {
        assert_eq!(stick, "b");
        let want = if *name == share.0 {
            &share.1
        } else {
            &backup.1
        };
        assert_eq!(bytes, want, "{name} changed on its way");
    }
    // The copy stays in the Inbox, for the next stick; the Outbox is
    // untouched.
    assert!(app.inbox.iter().any(|i| i.name == share.0));
    assert!(app.outbox.is_empty());
}

#[test]
fn a_picture_copied_in_keeps_its_bytes_and_its_codes_are_still_read() {
    let mut app = testkit::started();
    let png = kit("savings-wallet-qr.png");
    let mut a = Stick::new("a", std::slice::from_ref(&png));
    a.dir = Some(folder("a", std::slice::from_ref(&png)));
    attach(&mut app, &[&a]);
    copy_in(&mut app, &[&a], &[&png.0]);
    assert!(
        app.inbox.iter().any(|i| i.kind == FileKind::Wallet),
        "the wallet in its code was not read: {:?}",
        app.visit.log
    );
    let picture = app
        .inbox
        .iter()
        .find(|i| i.name == png.0)
        .expect("the picture itself is not in the Inbox");
    assert_eq!(picture.bytes, png.1);

    app.storage(StorageEvent::Sticks(Vec::new()));
    let b = Stick::new("b", &[]);
    attach(&mut app, &[&b]);
    // What its code holds is public: written with no sheet.
    app.press(row(&app, &png.0).expect("the picture is not offered"));
    assert_eq!(app.sheet, None);
    let wrote = write(&mut app, &[&b]);
    let (_, name, bytes) = wrote
        .iter()
        .find(|(_, n, _)| *n == png.0)
        .expect("the picture was not written");
    assert_eq!(name, &png.0, "written under another name");
    assert_eq!(bytes, &png.1, "the picture changed on its way");
    if let Some(dir) = &a.dir {
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[test]
fn a_seed_in_the_inbox_is_written_only_past_the_secret_sheet() {
    let mut app = testkit::started();
    let words = kit_ending("-words.txt");
    let seedqr = kit_ending("-seedqr.png");
    // Both in at once: pulling the first would load the SeedQR's key.
    let a = Stick::new("a", &[words.clone(), seedqr.clone()]);
    let b = Stick::new("b", &[]);
    attach(&mut app, &[&a, &b]);
    copy_in(&mut app, &[&a, &b], &[&words.0, &seedqr.0]);
    app.press(Action::VisitStick(1));
    let _ = app.frame();

    // Select all leaves both alone.
    app.press(Action::VisitOutAll);
    assert!(!app.visit.from_inbox.contains(&words.0));
    assert!(!app.visit.from_inbox.contains(&seedqr.0));

    for (name, bytes) in [&words, &seedqr] {
        let r = row(&app, name).expect("not offered");
        app.press(r);
        assert_eq!(app.sheet, Some(Sheet::SecretOut), "{name}: no secret sheet");
        assert_eq!(app.secret_out.as_ref().map(|o| o.ack), Some(Ack::Seed));
        // Nothing until the line is ticked.
        app.press(Action::SecretUnprotected);
        assert!(
            !app.visit.from_inbox.contains(name),
            "{name} ticked unasked"
        );
        app.press(Action::SecretAck);
        app.press(Action::SecretUnprotected);
        assert_eq!(app.sheet, None);
        assert!(app.visit.from_inbox.contains(name), "{name} not ticked");
        assert!(app.outbox.is_empty(), "{name} went to the Outbox");
        let wrote = write(&mut app, &[&a, &b]);
        assert_eq!(wrote.len(), 1);
        assert_eq!(wrote[0].0, "b");
        assert_eq!(&wrote[0].1, name);
        assert_eq!(&wrote[0].2, bytes, "{name} changed on its way");
    }

    // Cancel ticks nothing.
    app.press(row(&app, &words.0).unwrap());
    app.press(Action::Cancel);
    assert!(app.visit.from_inbox.is_empty());
    assert!(write(&mut app, &[&a, &b]).is_empty());
}

#[test]
fn text_is_offered_as_something_faraday_cannot_tell_about() {
    let mut app = testkit::started();
    let notes = (
        "recovery-codes.txt".to_string(),
        b"github 1234-5678\nmail 8765-4321\n".to_vec(),
    );
    let a = Stick::new("a", std::slice::from_ref(&notes));
    attach(&mut app, &[&a]);
    copy_in(&mut app, &[&a], &[&notes.0]);
    assert_eq!(app.inbox[0].kind, FileKind::Text);
    app.storage(StorageEvent::Sticks(Vec::new()));
    let b = Stick::new("b", &[]);
    attach(&mut app, &[&b]);
    app.press(Action::VisitOutAll);
    assert!(app.visit.from_inbox.is_empty(), "Select all ticked text");
    app.press(row(&app, &notes.0).unwrap());
    let out = app.secret_out.as_ref().expect("no secret sheet");
    assert_eq!(out.ack, Ack::Unknown);
    assert_eq!(
        out.ack.text(app.medium),
        "Faraday cannot tell whether this is a secret: anyone who copies the stick can read it"
    );
    app.press(Action::SecretAck);
    app.press(Action::SecretUnprotected);
    let wrote = write(&mut app, &[&b]);
    assert_eq!(wrote.len(), 1);
    assert_eq!(wrote[0].2, notes.1);
}

#[test]
fn a_file_larger_than_18_mib_is_not_read() {
    let mut app = testkit::started();
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        id: "a".into(),
        label: "A".into(),
        boot: false,
        files: vec![
            ("film.mp4".into(), 19 * 1024 * 1024),
            ("savings-share-1-of-3.txt".into(), 900),
        ],
    }]));
    app.press(Action::Nav(Screen::Visit));
    let _ = app.frame();
    assert!(
        !app.offers(Action::VisitIn(0)),
        "the large file can be ticked"
    );
    app.press(Action::VisitIn(0));
    // It is not ticked with everything else, nor by its own press.
    assert!(!app.visit.inn.contains("film.mp4"));
    assert!(app.visit.inn.contains("savings-share-1-of-3.txt"));
    app.press(Action::VisitCopy);
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Read { name, .. } | StorageCommand::ReadQr { name, .. } = c {
            assert_ne!(name, "film.mp4", "the large file was read");
        }
    }
}

#[test]
fn with_two_sticks_in_at_once_a_file_goes_from_one_to_the_other() {
    let mut app = testkit::started();
    let share = kit("savings-share-2-of-3.txt");
    let a = Stick::new("a", std::slice::from_ref(&share));
    let b = Stick::new("b", &[]);
    attach(&mut app, &[&a, &b]);
    app.press(Action::VisitStick(0));
    copy_in(&mut app, &[&a, &b], &[&share.0]);
    assert_eq!(row(&app, &share.0), None, "offered back to its own stick");
    app.press(Action::VisitStick(1));
    let _ = app.frame();
    let r = row(&app, &share.0).expect("not offered for the other stick");
    assert!(app.offers(r));
    app.press(r);
    let wrote = write(&mut app, &[&a, &b]);
    assert_eq!(wrote.len(), 1);
    assert_eq!(wrote[0].0, "b", "written to the wrong stick");
    assert_eq!(wrote[0].2, share.1);
}

#[test]
fn a_picture_keeps_what_its_codes_hold_across_a_lock_and_a_seedqr_does_not_outlive_it() {
    let mut app = testkit::started();
    let wallet = kit("savings-wallet-qr.png");
    let seedqr = kit_ending("-seedqr.png");
    let a = Stick::new("a", &[wallet.clone(), seedqr.clone()]);
    attach(&mut app, &[&a]);
    copy_in(&mut app, &[&a], &[&wallet.0, &seedqr.0]);
    app.press(Action::Lock);
    let mut saved = None;
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes {
            inbox,
            outbox,
            kept,
        } = c
        {
            saved = Some((inbox, outbox, kept));
        }
    }
    let (inbox, outbox, kept) = saved.expect("the boxes were not saved");
    let mut next = testkit::started();
    next.storage(StorageEvent::Restored {
        inbox,
        outbox,
        kept,
    });
    assert!(
        !next.inbox.iter().any(|i| i.name == seedqr.0),
        "a SeedQR outlived the lock"
    );
    let b = Stick::new("b", &[]);
    attach(&mut next, &[&b]);
    next.press(row(&next, &wallet.0).expect("the wallet's picture is gone"));
    assert_eq!(
        next.sheet, None,
        "a public picture asks for the secret sheet"
    );
    let wrote = write(&mut next, &[&b]);
    assert_eq!(wrote.len(), 1);
    assert_eq!(wrote[0].2, wallet.1);
}
