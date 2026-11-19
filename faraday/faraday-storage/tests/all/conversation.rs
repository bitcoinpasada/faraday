//! A lock while the disk process is busy: the new app's conversation with
//! it starts clean. What the old app asked, answered late, never reaches
//! the new one, and a reader that cannot go on ends the waits at once.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use faraday_files::proto::{Request, Response, receive, send};
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
            _ => Response::Failed(Text::of("not asked")),
        };
        send(&mut tx, seq, &answer.encode()).unwrap();
    }
}

#[test]
fn a_request_during_a_slow_vault_write_is_refused_at_once() {
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

    // A listing meanwhile does not wait out the write: it is refused now
    // (it waited the 15 s of an ask and said the disk process was silent).
    let started = std::time::Instant::now();
    let listed = disk.list();
    assert!(listed.is_err());
    assert!(
        started.elapsed() < std::time::Duration::from_secs(2),
        "refused at once, not after a wait"
    );

    // The write itself goes on and ends with the disk process's answer.
    go.send(()).unwrap();
    let started = std::time::Instant::now();
    let mut done = Vec::new();
    while done.is_empty() && started.elapsed() < std::time::Duration::from_secs(5) {
        done = disk.transferred();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(done.len(), 1);
    // With the transfer over, a listing is answered.
    assert!(disk.list().is_ok());

    drop(disk);
    drop(server);
    let _ = std::fs::remove_dir_all(dir);
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
