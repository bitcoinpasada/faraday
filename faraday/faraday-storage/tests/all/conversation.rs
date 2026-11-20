//! A lock while the disk process is busy: the new app's conversation with
//! it starts clean. What the old app asked, answered late, never reaches
//! the new one, and a reader that cannot go on ends the waits at once.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use faraday_files::proto::{Request, Response, receive, send};
use faraday_shell_api::App;
use faraday_storage::{DiskProcess, Sticks, Transfer, Transferred};
use faraday_text::Text;

fn fifo(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    let made = std::process::Command::new("mkfifo").arg(&path).status();
    assert!(made.unwrap().success());
    path
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("faraday-conv-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn rw(p: &Path) -> std::fs::File {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(p)
        .unwrap()
}

/// A disk process busy on its first request until `go` comes, as on a long
/// write: it then answers in order what has been asked, the first request's
/// answer being one of a file's bytes.
fn busy_server(requests: PathBuf, responses: PathBuf, go: mpsc::Receiver<()>) {
    let mut rx = rw(&requests);
    let mut tx = rw(&responses);
    let mut first = true;
    while let Ok(Some((seq, frame))) = receive(&mut rx) {
        let req = Request::decode(&frame).unwrap();
        if first {
            first = false;
            go.recv().unwrap();
        }
        let answer = match req {
            Request::Open { token } => Response::Opened(token),
            Request::List => Response::Sticks(Vec::new()),
            Request::ReadPart { .. } => Response::Part {
                total: 3,
                bytes: b"old".to_vec(),
            },
            _ => Response::Failed(Text::of("not asked")),
        };
        send(&mut tx, seq, &answer.encode()).unwrap();
    }
}

#[test]
fn an_answer_to_the_app_before_a_lock_does_not_reach_the_one_after() {
    let dir = scratch("late");
    let requests = fifo(&dir, "requests");
    let responses = fifo(&dir, "responses");
    let (go, wait) = mpsc::channel();
    let server = {
        let (a, b) = (requests.clone(), responses.clone());
        std::thread::spawn(move || busy_server(a, b, wait))
    };

    // The first app asks for a vault part, and the lock comes (the process
    // is gone, its pipe ends closed) before the disk process has answered.
    {
        let mut old = rw(&requests);
        let asked = Request::ReadPart {
            stick: Text::of("s"),
            name: Text::of("Passwords.kdbx"),
            from: 0,
            len: 10,
        };
        send(&mut old, 1, &asked.encode()).unwrap();
        // The disk process has the request before the app is gone.
        std::thread::sleep(std::time::Duration::from_millis(300));
    }

    // The new app opens the pipes again; the disk process finishes the
    // old request, then answers the new one.
    let mut new = DiskProcess::open(&requests, &responses).unwrap();
    go.send(()).unwrap();
    let listed = new.list();
    assert!(
        listed.is_ok_and(|l| l.is_empty()),
        "the new app is answered its own request"
    );

    drop(new);
    drop(server);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_reader_that_cannot_go_on_ends_the_waits_at_once() {
    let dir = scratch("reader");
    let requests = fifo(&dir, "requests");
    let responses = fifo(&dir, "responses");
    let mut disk = DiskProcess::open(&requests, &responses).unwrap();
    // The disk process sends a frame whose length no frame has.
    let mut tx = rw(&responses);
    tx.write_all(&[0xff; 8]).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(200));

    let asked = disk.transfer_later(Transfer::Read {
        stick: Text::of("s"),
        name: Text::of("Passwords.kdbx"),
    });
    assert!(asked.is_ok());
    let started = std::time::Instant::now();
    let mut done = Vec::new();
    while done.is_empty() && started.elapsed() < std::time::Duration::from_secs(5) {
        done = disk.transferred();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(done.len(), 1, "the transfer ends in seconds, not minutes");
    assert!(matches!(&done[0], Transferred::Read { result, .. } if result.is_err()));

    let _ = std::fs::remove_dir_all(dir);
}

/// A disk process that answers the opening at once and then holds the next
/// request, a vault's piece being written, until `go` comes.
fn slow_write_server(requests: PathBuf, responses: PathBuf, go: mpsc::Receiver<()>) {
    let mut rx = rw(&requests);
    let mut tx = rw(&responses);
    let mut held = false;
    while let Ok(Some((seq, frame))) = receive(&mut rx) {
        let req = Request::decode(&frame).unwrap();
        if !held && !matches!(req, Request::Open { .. }) {
            held = true;
            go.recv().unwrap();
        }
        let answer = match req {
            Request::Open { token } => Response::Opened(token),
            Request::List => Response::Sticks(Vec::new()),
            Request::Piece { bytes, .. } => Response::Held(bytes.len() as u64),
            Request::Write { .. } => Response::Written(Text::of("Passwords.kdbx")),
            Request::Space { .. } => Response::Space { free: 5, size: 9 },
            _ => Response::Failed(Text::of("not asked")),
        };
        send(&mut tx, seq, &answer.encode()).unwrap();
    }
}

#[test]
fn a_call_made_during_a_vault_transfer_is_refused_as_busy_at_once() {
    let dir = scratch("slow-write");
    let requests = fifo(&dir, "requests");
    let responses = fifo(&dir, "responses");
    let (go, wait) = mpsc::channel();
    let server = {
        let (a, b) = (requests.clone(), responses.clone());
        std::thread::spawn(move || slow_write_server(a, b, wait))
    };
    let mut disk = DiskProcess::open(&requests, &responses).unwrap();

    let vault = vec![7u8; faraday_files::proto::PIECE + 1];
    let asked = disk.transfer_later(Transfer::Write {
        stick: Text::of("s"),
        name: Text::of("Passwords.kdbx"),
        bytes: zeroize::Zeroizing::new(vault),
        copies: Vec::new(),
        replace: false,
        renamed: false,
        on_taken: faraday_files::OnTaken::Beside,
    });
    assert!(asked.is_ok());
    assert!(disk.transferred().is_empty(), "the write is under way");

    let started = std::time::Instant::now();
    assert!(
        disk.space("s").is_err(),
        "refused while the vault is in pieces"
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(1));

    go.send(()).unwrap();
    let started = std::time::Instant::now();
    let mut done = Vec::new();
    while done.is_empty() && started.elapsed() < std::time::Duration::from_secs(5) {
        done = disk.transferred();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(done.len(), 1);
    assert!(matches!(&done[0], Transferred::Write { result: Ok(_), .. }));
    // With the write over, the space is answered.
    assert_eq!(disk.space("s").unwrap(), (5, 9));

    drop(disk);
    drop(server);
    let _ = std::fs::remove_dir_all(dir);
}

/// Sticks that report a vault in pieces for a few passes, answer the space
/// of a stick, and count the answers.
struct InPieces {
    passes: u32,
    spaces: u32,
    held: faraday_storage::Held,
    dirs: faraday_storage::Dirs,
}

impl Sticks for InPieces {
    fn read(&mut self, stick: &str, name: &str) -> Result<Vec<u8>, Text> {
        self.dirs.read(stick, name)
    }
    fn write(
        &mut self,
        c: &faraday_crypto::work::Compute,
        stick: &str,
        name: &str,
        bytes: &[u8],
        copies: &[[u8; 32]],
        replace: bool,
        renamed: bool,
        on_taken: faraday_files::OnTaken,
    ) -> Result<Text, faraday_files::WriteRefused> {
        self.dirs
            .write(c, stick, name, bytes, copies, replace, renamed, on_taken)
    }
    fn transferred(&mut self) -> Vec<Transferred> {
        self.passes = self.passes.saturating_sub(1);
        Vec::new()
    }
    fn busy(&self) -> bool {
        self.passes > 0
    }
    fn held_queue(&mut self) -> Option<&mut faraday_storage::Held> {
        Some(&mut self.held)
    }
    fn wipe(&mut self, stick: &str, name: &str) -> Result<faraday_files::Wiped, Text> {
        self.dirs.wipe(stick, name)
    }
    fn space(&mut self, _stick: &str) -> Result<(u64, u64), Text> {
        self.spaces += 1;
        Ok((5, 9))
    }
    fn zero_free(&mut self, stick: &str, from: u32) -> Result<(u32, u32, bool), Text> {
        self.dirs.zero_free(stick, from)
    }
    fn make_empty(&mut self, stick: &str) -> Result<(), Text> {
        self.dirs.make_empty(stick)
    }
    fn read_qr_later(&mut self, stick: &str, name: &str) {
        self.dirs.read_qr_later(stick, name);
    }
    fn pictures_read(&mut self) -> Vec<faraday_storage::PictureRead> {
        self.dirs.pictures_read()
    }
    fn rename(
        &mut self,
        c: &faraday_crypto::work::Compute,
        stick: &str,
        from: &str,
        to: &str,
        vaults: &[Vec<[u8; 32]>],
    ) -> Result<(), faraday_files::Unrenamed> {
        self.dirs.rename(c, stick, from, to, vaults)
    }
    fn name(&mut self, stick: &str, label: &str) -> Result<(), Text> {
        self.dirs.name(stick, label)
    }
}

#[test]
fn during_a_transfer_the_boxes_are_kept_at_once_and_the_sticks_space_is_answered_after_it() {
    let mut app = faraday_core::testkit::new_on_test_net();
    app.storage(faraday_core::StorageEvent::Sticks(vec![
        faraday_core::StickInfo {
            make: Default::default(),
            serial: None,
            id: Text::of("s"),
            label: Text::of("STICK"),
            boot: false,
            files: vec![(Text::of("a.txt"), 1)],
            vaults: Vec::new(),
        },
    ]));
    app.press(faraday_core::Action::Nav(faraday_core::Screen::Files));
    let _ = app.frame();
    // Delete files asks the stick for its space; a lock then keeps
    // the boxes and exits.
    app.press(faraday_core::Action::DeleteFiles);
    app.press(faraday_core::Action::LockUnsealed);
    assert!(app.restart_requested());

    // What the last save left: the lock's save replaces it.
    let mut boxes = faraday_storage::Boxes::Memory {
        inbox: faraday_core::files_from(vec![(Text::of("old.txt"), b"x".to_vec())]),
        outbox: Vec::new(),
        kept: Vec::new(),
    };
    let mut sticks = InPieces {
        passes: 3,
        spaces: 0,
        held: std::collections::VecDeque::new(),
        dirs: faraday_storage::Dirs::default(),
    };
    faraday_storage::serve_with(&mut app, &mut boxes, None, &mut sticks);
    assert!(
        matches!(&boxes, faraday_storage::Boxes::Memory { inbox, .. } if inbox.is_empty()),
        "the boxes are kept while the vault is in pieces"
    );
    assert_eq!(sticks.spaces, 0, "the stick is not asked behind the vault");

    faraday_storage::serve_with(&mut app, &mut boxes, None, &mut sticks);
    assert_eq!(sticks.spaces, 1, "answered once the transfer is over");
    faraday_storage::serve_with(&mut app, &mut boxes, None, &mut sticks);
    assert_eq!(sticks.spaces, 1, "and once only");
}

#[test]
fn after_the_reader_ends_a_large_write_fails_at_once_and_does_not_block() {
    let dir = scratch("dead-write");
    let requests = fifo(&dir, "requests");
    let responses = fifo(&dir, "responses");
    let mut disk = DiskProcess::open(&requests, &responses).unwrap();
    // A frame that cannot be read ends the reader; nobody reads requests.
    let mut tx = rw(&responses);
    tx.write_all(&[0xff; 8]).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(200));

    let started = std::time::Instant::now();
    assert!(disk.list().is_err());
    let asked = disk.transfer_later(Transfer::Write {
        stick: Text::of("s"),
        name: Text::of("Passwords.kdbx"),
        bytes: zeroize::Zeroizing::new(vec![7u8; 4 * faraday_files::proto::PIECE]),
        copies: Vec::new(),
        replace: false,
        renamed: false,
        on_taken: faraday_files::OnTaken::Beside,
    });
    assert!(asked.is_ok());
    let mut done = Vec::new();
    while done.is_empty() && started.elapsed() < std::time::Duration::from_secs(5) {
        done = disk.transferred();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(done.len(), 1);
    assert!(matches!(&done[0], Transferred::Write { result, .. } if result.is_err()));
    assert!(started.elapsed() < std::time::Duration::from_secs(5));

    let _ = std::fs::remove_dir_all(dir);
}
