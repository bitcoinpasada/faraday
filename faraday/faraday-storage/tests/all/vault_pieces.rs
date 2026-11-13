//! A vault longer than a frame crosses to and from the disk process in
//! pieces, and the shell's thread is not held while it does (F206
//! point 7): its answer comes on a later pass, byte for byte.

use std::collections::HashMap;
use std::path::PathBuf;

use faraday_files::OnTaken;
use faraday_files::proto::{PIECE, Request, Response, receive, send};
use faraday_storage::{DiskProcess, Sticks, Transfer, Transferred};
use faraday_text::Text;

/// A named pipe in a folder of its own.
fn fifo(dir: &std::path::Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    let made = std::process::Command::new("mkfifo").arg(&path).status();
    assert!(made.unwrap().success());
    path
}

/// A disk process that keeps files in memory and answers what a vault's
/// transfer asks.
fn serve(requests: PathBuf, responses: PathBuf, files: HashMap<String, Vec<u8>>) {
    let open = |p: &PathBuf| {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(p)
            .unwrap()
    };
    let mut rx = open(&requests);
    let mut tx = open(&responses);
    let mut files = files;
    let mut held: Vec<u8> = Vec::new();
    while let Ok(Some((seq, frame))) = receive(&mut rx) {
        let answer = match Request::decode(&frame).unwrap() {
            Request::Piece { at, bytes, .. } => {
                assert_eq!(at as usize, held.len(), "pieces come in order");
                held.extend_from_slice(&bytes);
                Response::Held(held.len() as u64)
            }
            Request::Write { name, bytes, .. } => {
                held.extend_from_slice(&bytes);
                files.insert(String::from(&*name), std::mem::take(&mut held));
                Response::Written(name)
            }
            Request::ReadPart {
                name, from, len, ..
            } => {
                let file = &files[&*name];
                let from = from as usize;
                let to = (from + len as usize).min(file.len());
                Response::Part {
                    total: file.len() as u64,
                    bytes: file[from..to].to_vec(),
                }
            }
            _ => Response::Failed(faraday_text::Text::of("not asked")),
        };
        send(&mut tx, seq, &answer.encode()).unwrap();
    }
}

/// The transfers done by the time the disk process has answered.
fn done(disk: &mut DiskProcess) -> Transferred {
    for _ in 0..2000 {
        if let Some(t) = disk.transferred().pop() {
            return t;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("the transfer did not end");
}

#[test]
fn a_vault_longer_than_a_frame_is_written_and_read_back_whole() {
    let dir = std::env::temp_dir().join(format!("faraday-pieces-disk-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let requests = fifo(&dir, "requests");
    let responses = fifo(&dir, "responses");
    let server = {
        let (a, b) = (requests.clone(), responses.clone());
        std::thread::spawn(move || serve(a, b, HashMap::new()))
    };
    let mut disk = DiskProcess::open(&requests, &responses).unwrap();

    let vault: Vec<u8> = (0..2 * PIECE + 12_345).map(|i| (i % 251) as u8).collect();
    let asked = disk.transfer_later(Transfer::Write {
        stick: Text::of("s"),
        name: Text::of("Passwords.kdbx"),
        bytes: zeroize::Zeroizing::new(vault.clone()),
        copies: Vec::new(),
        replace: false,
        renamed: false,
        on_taken: OnTaken::Beside,
    });
    assert!(asked.is_ok(), "a vault of this length goes in pieces");
    match done(&mut disk) {
        Transferred::Write { result, .. } => {
            assert_eq!(result.ok().as_deref(), Some("Passwords.kdbx"))
        }
        Transferred::Read { .. } => panic!("a read answered a write"),
    }

    let asked = disk.transfer_later(Transfer::Read {
        stick: Text::of("s"),
        name: Text::of("Passwords.kdbx"),
    });
    assert!(asked.is_ok());
    match done(&mut disk) {
        Transferred::Read { result, .. } => {
            assert!(result.unwrap() == vault, "the file comes back whole")
        }
        Transferred::Write { .. } => panic!("a write answered a read"),
    }

    // A vault read for its SHA-256 is read the same way, off the thread,
    // and a file that is not a vault is read at once.
    assert!(disk.hash_later("s", "Passwords.kdbx"));
    match done(&mut disk) {
        Transferred::Read { stick, name, .. } => {
            assert!(disk.hash_taken(&stick, &name), "asked for its hash");
            assert!(!disk.hash_taken(&stick, &name), "answered once");
        }
        Transferred::Write { .. } => panic!("a write answered a read"),
    }
    assert!(!disk.hash_later("s", "notes.txt"));

    // A file that is not a vault is not carried in pieces.
    let other = disk.transfer_later(Transfer::Read {
        stick: Text::of("s"),
        name: Text::of("notes.txt"),
    });
    assert!(other.is_err());

    drop(disk);
    drop(server);
    let _ = std::fs::remove_dir_all(dir);
}

/// The app served over the disk process, a pass at a time as a shell
/// does, until `until` holds.
fn serve_until(
    app: &mut faraday_core::Faraday,
    boxes: &mut faraday_storage::Boxes,
    disk: &mut DiskProcess,
    until: impl Fn(&faraday_core::Faraday) -> bool,
) {
    use faraday_shell_api::{App, Event};
    for t in 0..4000u64 {
        let _ = faraday_storage::serve_with(app, boxes, None, disk);
        let _ = app.frame();
        app.event(Event::Tick { now_ms: 1000 + t });
        std::thread::sleep(std::time::Duration::from_millis(2));
        if until(app) {
            return;
        }
    }
    panic!("did not settle: {:?}", app.drawn_texts());
}

#[test]
fn a_vault_over_one_frame_is_opened_changed_and_written_back_through_the_app() {
    use faraday_core::{Action, Screen, StorageEvent, testkit};
    use faraday_shell_api::{App, EntropyBytes, Event};
    use faraday_storage::Boxes;
    use faraday_storage::Sticks as _;

    // A vault whose file is longer than the disk process's frame.
    let file = big_vault();
    assert!(
        file.len() as u64 > faraday_files::MAX_READ,
        "the vault is {} bytes",
        file.len()
    );

    let dir = std::env::temp_dir().join(format!("faraday-pieces-app-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let requests = fifo(&dir, "requests");
    let responses = fifo(&dir, "responses");
    let on_stick: HashMap<String, Vec<u8>> =
        HashMap::from([("Big.kdbx".to_string(), file.clone())]);
    let server = {
        let (a, b) = (requests.clone(), responses.clone());
        std::thread::spawn(move || serve(a, b, on_stick))
    };
    let mut disk = DiskProcess::open(&requests, &responses).unwrap();
    let mut boxes = Boxes::Memory {
        inbox: Vec::new(),
        outbox: Vec::new(),
        kept: Vec::new(),
    };

    let mut app = testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut app, Some(100));
    for i in 0..6u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x40 + i; 32])));
    }
    app.storage(StorageEvent::Sticks(vec![stick_with(file.len() as u64)]));

    // The vault comes into the Inbox from the stick, whole.
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    app.press(Action::VisitInAll);
    app.press(Action::VisitIn(0));
    app.press(Action::VisitCopy);
    serve_until(&mut app, &mut boxes, &mut disk, |a| a.inbox().len() == 1);
    assert_eq!(app.inbox().len(), 1, "the vault is in the Inbox");

    // The stick comes out, as an unlock asks.
    app.storage(StorageEvent::Sticks(Vec::new()));

    let sealed_bytes = open_change_and_lock(&mut app);
    assert!(sealed_bytes != file, "the change is in the seal");

    // The lock restarts the app with the seal waiting in the Outbox; the
    // stick goes back in and the seal is written over the same file.
    let waiting = (app.outbox[0].name.clone(), sealed_bytes.clone());
    let mut app = testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: faraday_core::files_from(vec![waiting]),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Sticks(vec![stick_with(file.len() as u64)]));
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    app.press(Action::VisitWrite);
    // The write ends, in pieces, and the seal leaves the Outbox.
    serve_until(&mut app, &mut boxes, &mut disk, |a| {
        a.sheet.is_none() && a.outbox.is_empty()
    });
    if app.sheet.is_some() {
        app.press(Action::VisitReplace);
        serve_until(&mut app, &mut boxes, &mut disk, |a| a.outbox.is_empty());
    }
    assert!(app.outbox.is_empty(), "the seal was not written");

    // What the stick now holds is the sealed file, byte for byte.
    let read = Transfer::Read {
        stick: Text::of("s"),
        name: Text::of("Big.kdbx"),
    };
    assert!(disk.transfer_later(read).is_ok());
    match done(&mut disk) {
        Transferred::Read { result, .. } => {
            assert!(result.unwrap() == sealed_bytes, "the stick holds the seal")
        }
        Transferred::Write { .. } => panic!("a write answered a read"),
    }
    drop(disk);
    drop(server);
    let _ = std::fs::remove_dir_all(dir);
}

/// A vault of one entry holding an attachment, in a file of 24 MiB.
fn big_vault() -> Vec<u8> {
    // One attachment of about 20 MiB, sealed as the vault layer seals one.
    let mut noise = 0x9E37_79B9_7F4A_7C15_u64;
    let attachment: Vec<u8> = (0..20 * 1024 * 1024)
        .map(|_| {
            noise ^= noise << 13;
            noise ^= noise >> 7;
            noise ^= noise << 17;
            noise as u8
        })
        .collect();

    use faraday_vault::kdbx::xml::Document;
    use faraday_vault::kdbx::{self, Binaries, Binary, RANDOM_LEN, items};
    let random: [u8; RANDOM_LEN] =
        std::array::from_fn(|i| (i as u8).wrapping_mul(7).wrapping_add(3));
    let mut doc = Document::new([9; 16], "Big");
    items::stamp(&mut doc, 1, None).unwrap();
    let text = String::from_utf8(doc.write(kdbx::inner_key_of(&random)).to_vec()).unwrap();
    let entry = "<Entry><UUID>AQEBAQEBAQEBAQEBAQEBAQ==</UUID>\
        <String><Key>Title</Key><Value>big</Value></String>\
        <Binary><Key>big.bin</Key><Value Ref=\"0\"/></Binary></Entry>";
    let text = text.replace("</Group></Root>", &format!("{entry}</Group></Root>"));
    let binaries = [Binary {
        protected: false,
        data: faraday_vault::kdbx::Shared::new(zeroize::Zeroizing::new(attachment)),
    }];
    let contents = kdbx::Contents {
        document: text.as_bytes(),
        binaries: Binaries::all(&binaries),
        public_custom_data: None,
    };
    let cost = faraday_vault::Cost {
        memory_kib: 8 * 1024,
        passes: 1,
        lanes: 1,
    };
    kdbx::seal(
        &faraday_core::testkit::compute(),
        testkit_passphrase(),
        cost,
        &random,
        &contents,
        24 * 1024 * 1024,
    )
    .unwrap()
}

fn testkit_passphrase() -> &'static str {
    faraday_core::testkit::VAULT_PASSPHRASE
}

/// Opens the vault in the Inbox, saves a key into it and locks: the file
/// the lock seals into the Outbox.
fn open_change_and_lock(app: &mut faraday_core::Faraday) -> Vec<u8> {
    use faraday_core::{Action, testkit};
    use faraday_shell_api::{App, EntropyBytes, Event, Key};
    // It opens, a key is saved into it, and the lock seals it.
    app.press(Action::Vault(faraday_core::vault::VaultAction::Open(0)));
    for c in testkit::VAULT_PASSPHRASE.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::Vault(faraday_core::vault::VaultAction::Unlock));
    for t in 1..120u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
        if !app.vaults().open.is_empty() {
            break;
        }
    }
    let texts = app.drawn_texts();
    assert_eq!(
        app.vaults().open.len(),
        1,
        "the vault did not open: {:?} {texts:?}",
        app.vaults().unlock_error,
    );
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[1].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    let k = app.session.keys.len() - 1;
    app.press(Action::Vault(faraday_core::vault::VaultAction::SaveKey(k)));
    assert!(
        app.vaults().open[0].changes > 0,
        "the vault was not changed"
    );
    for i in 0..16u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x80 + i; 32])));
    }
    app.press(Action::Lock);
    testkit::settle(app);
    let sealed = app
        .outbox
        .iter()
        .position(|i| i.kind == faraday_core::wallet::FileKind::Vault)
        .expect("the vault was sealed into the Outbox");
    let bytes = app.outbox[sealed].bytes.to_vec();
    assert!(bytes.len() as u64 > faraday_files::MAX_READ);
    assert!(app.restart_requested());
    bytes
}

/// Stick "s" holding the vault `Big.kdbx` of `size` bytes.
fn stick_with(size: u64) -> faraday_core::StickInfo {
    faraday_core::StickInfo {
        make: Default::default(),
        serial: None,
        id: "s".into(),
        label: "STICK".into(),
        boot: false,
        files: vec![("Big.kdbx".into(), size)],
        vaults: vec!["Big.kdbx".into()],
    }
}

/// A disk process that answers each read of a piece with the next of these.
fn lying(requests: PathBuf, responses: PathBuf, answers: Vec<(u64, usize)>) {
    let open = |p: &PathBuf| {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(p)
            .unwrap()
    };
    let mut rx = open(&requests);
    let mut tx = open(&responses);
    let mut answers = answers.into_iter();
    while let Ok(Some((seq, _))) = receive(&mut rx) {
        let (total, len) = answers.next().unwrap_or((0, 0));
        let answer = Response::Part {
            total,
            bytes: vec![7; len],
        };
        send(&mut tx, seq, &answer.encode()).unwrap();
    }
}

fn read_against(tag: &str, answers: Vec<(u64, usize)>) -> Result<Vec<u8>, Text> {
    let dir = std::env::temp_dir().join(format!("faraday-lying-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let requests = fifo(&dir, "requests");
    let responses = fifo(&dir, "responses");
    let server = {
        let (a, b) = (requests.clone(), responses.clone());
        std::thread::spawn(move || lying(a, b, answers))
    };
    let mut disk = DiskProcess::open(&requests, &responses).unwrap();
    let read = Transfer::Read {
        stick: Text::of("s"),
        name: Text::of("Passwords.kdbx"),
    };
    assert!(disk.transfer_later(read).is_ok());
    let result = match done(&mut disk) {
        Transferred::Read { result, .. } => result,
        Transferred::Write { .. } => panic!("a write answered a read"),
    };
    drop(disk);
    drop(server);
    let _ = std::fs::remove_dir_all(dir);
    result
}

#[test]
fn a_vault_read_fails_when_the_disk_process_states_a_length_it_should_not() {
    // The same lengths, kept, read whole.
    let ok = read_against("ok", vec![(PIECE as u64 + 5, PIECE), (PIECE as u64 + 5, 5)]);
    assert_eq!(ok.map(|v| v.len()), Ok(PIECE + 5));
    // Over the longest vault file.
    let over = faraday_files::MAX_VAULT_READ + 1;
    assert!(read_against("over", vec![(over, PIECE)]).is_err());
    // A length that changes between pieces.
    let changed = vec![(PIECE as u64 + 5, PIECE), (PIECE as u64 + 6, 6)];
    assert!(read_against("changed", changed).is_err());
    // A piece longer or shorter than asked.
    assert!(read_against("long", vec![(10, 11)]).is_err());
    assert!(read_against("short", vec![(PIECE as u64 + 5, 100)]).is_err());
}

/// The blocking read, which the shell's thread makes of a file that is not
/// carried off the thread, with the same answers.
fn blocking_read_against(tag: &str, answers: Vec<(u64, usize)>) -> Result<Vec<u8>, Text> {
    let dir =
        std::env::temp_dir().join(format!("faraday-lying-block-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let requests = fifo(&dir, "requests");
    let responses = fifo(&dir, "responses");
    let server = {
        let (a, b) = (requests.clone(), responses.clone());
        std::thread::spawn(move || lying(a, b, answers))
    };
    let mut disk = DiskProcess::open(&requests, &responses).unwrap();
    let result = Sticks::read(&mut disk, "s", "Passwords.kdbx");
    drop(disk);
    drop(server);
    let _ = std::fs::remove_dir_all(dir);
    result
}

#[test]
fn a_blocking_read_fails_when_the_disk_process_states_a_length_it_should_not() {
    let ok = blocking_read_against("ok", vec![(PIECE as u64 + 5, PIECE), (PIECE as u64 + 5, 5)]);
    assert_eq!(ok.map(|v| v.len()), Ok(PIECE + 5));
    let over = faraday_files::MAX_VAULT_READ + 1;
    assert!(blocking_read_against("over", vec![(over, PIECE)]).is_err());
    let changed = vec![(PIECE as u64 + 5, PIECE), (PIECE as u64 + 6, 6)];
    assert!(blocking_read_against("changed", changed).is_err());
    assert!(blocking_read_against("long", vec![(10, 11)]).is_err());
    assert!(blocking_read_against("short", vec![(PIECE as u64 + 5, 100)]).is_err());
}
