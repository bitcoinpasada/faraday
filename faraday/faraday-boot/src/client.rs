//! The boot copier from the shell's side: requests go down one FIFO and
//! answers come up the other, as with the disk process
//! (`faraday-storage::DiskProcess`). A thread reads the answers, so a
//! copier that never answers costs a timeout rather than the shell; an
//! answer that comes after its timeout is known by its sequence number
//! and dropped.

use std::fs::{File, OpenOptions};
use std::path::Path;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use crate::proto::{self, Request, Response};

/// How long a listing may take: a stick seen for the first time has its
/// boot partition read for its release string.
const ASK_LIST: Duration = Duration::from_secs(60);
/// How long reading the source may take: a 48 MB partition on a slow
/// stick.
const ASK_READ: Duration = Duration::from_secs(120);
/// How long a write may take: written, flushed and read back.
const ASK_WRITE: Duration = Duration::from_secs(600);

/// The copier's two pipes, open.
pub struct Client {
    tx: File,
    answers: Receiver<(u32, Vec<u8>)>,
    seq: u32,
}

impl Client {
    /// Opens the two FIFOs. Both are opened read-write, which on a FIFO
    /// never waits for the other end: a copier that is not up yet answers
    /// when it is.
    pub fn open(requests: &Path, responses: &Path) -> std::io::Result<Client> {
        let rw = |p: &Path| OpenOptions::new().read(true).write(true).open(p);
        let tx = rw(requests)?;
        let mut rx = rw(responses)?;
        let (send, answers) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(Some(frame)) = proto::receive(&mut rx) {
                if send.send(frame).is_err() {
                    return;
                }
            }
        });
        Ok(Client {
            tx,
            answers,
            seq: 0,
        })
    }

    /// Sends one request and waits for its answer.
    pub fn ask(&mut self, req: &Request) -> Result<Response, String> {
        self.seq = self.seq.wrapping_add(1);
        proto::send(&mut self.tx, self.seq, &req.encode()).map_err(|e| e.to_string())?;
        let wait = match req {
            Request::List | Request::Forget => ASK_LIST,
            Request::ReadSource => ASK_READ,
            Request::Write { .. } => ASK_WRITE,
        };
        let until = Instant::now() + wait;
        loop {
            let left = until.saturating_duration_since(Instant::now());
            match self.answers.recv_timeout(left) {
                Ok((seq, frame)) if seq == self.seq => {
                    return Response::decode(&frame).map_err(|_| "a malformed answer".to_string());
                }
                Ok(_) => continue,
                Err(_) => return Err("the boot copier does not answer".to_string()),
            }
        }
    }
}
