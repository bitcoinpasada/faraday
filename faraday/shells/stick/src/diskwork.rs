//! The shell's own calls to the disk process on a thread of its own
//! (`docs/FRAMES.md` §7): the look for plugged-in sticks and the writes of
//! the timings file and the boot report.
//!
//! Each of them waits on a stick: a listing is a round trip to the disk
//! process and a write is a new file written, read back and renamed. On
//! the loop that held a frame for as long as the stick took. Here the
//! loop asks and reads the answer on a later pass without waiting.
//!
//! The disk process is one conversation, so the handle is shared under a
//! lock and this thread holds it for the length of one call; the app's
//! own file calls take the same lock. The thread holds the sticks' list
//! and the text of two diagnostic files, and nothing of the app, a key or
//! a vault. One job of each kind is out at a time. The thread ends when
//! the sender is dropped: [`Work::finish`] does that and joins it, so a
//! write in progress at exit is done before the final ones are made. A
//! thread that ends without answering ends the loop the way a panic on
//! the main thread does.

use std::path::Path;
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use faraday_core::StickInfo;
use faraday_storage::DiskProcess;

use crate::panel::lock;
use crate::system::System;
use crate::{bootreport, devnodes, radios};

/// The disk process's handle, shared by the loop and this thread.
pub type Disk = Arc<Mutex<DiskProcess>>;

/// The disk for one pass of the loop. The app asked (a request waits for
/// the stick): wait for the lock, the thread holding it for one call at a
/// time. Nothing asked: take it only if it is free, and `None` when the
/// thread is mid-call, so the pass serves nothing and no frame waits on a
/// slow listing. A lock poisoned by a panic is taken all the same.
pub fn for_pass<T>(disk: &Mutex<T>, asked: bool) -> Option<MutexGuard<'_, T>> {
    if asked {
        return Some(lock(disk));
    }
    match disk.try_lock() {
        Ok(g) => Some(g),
        Err(TryLockError::Poisoned(e)) => Some(e.into_inner()),
        Err(TryLockError::WouldBlock) => None,
    }
}

/// What a look at the sticks found.
pub struct Listing {
    /// The sticks, or why the disk process did not say.
    pub sticks: Result<Vec<StickInfo>, faraday_text::Text>,
    /// The sticks the listing found unusable: their ids, and whether the
    /// reason is that they are read-only.
    pub problems: Vec<(faraday_text::Text, bool)>,
}

/// The sticks now, as the disk process lists them. A stick not handed out
/// yet is kept from a clean app (`hide_waiting`): the grant hands it over
/// within half a second, and the app's first look at the boot stick then
/// sees its files rather than an empty one.
pub fn list(disk: &mut DiskProcess, hide_waiting: bool) -> Listing {
    let sticks = disk.list().map(|mut s| {
        if hide_waiting {
            s.retain(|s| !disk.waiting().iter().any(|w| *w == *s.id));
        }
        s
    });
    Listing {
        sticks,
        problems: disk.problems().to_vec(),
    }
}

/// A call for the thread to make.
enum Job {
    List {
        hide_waiting: bool,
    },
    Timings {
        stick: String,
        name: String,
        text: String,
    },
    BootReport(Box<BootReport>),
}

/// The boot report to compose and write: what the loop has seen, copied.
pub struct BootReport {
    pub stick: String,
    pub name: String,
    pub inventory: String,
    pub system: System,
    pub elapsed: Duration,
}

/// What a job came to.
pub enum Reply {
    Listed(Listing),
    /// Whether the timings were written.
    Timings(bool),
    BootReport {
        name: String,
        wrote: Result<(), String>,
        /// How long the write itself took.
        took: Duration,
    },
}

/// The thread and what is out to it.
pub struct Work {
    request: Option<Sender<Job>>,
    reply: Receiver<Reply>,
    thread: Option<JoinHandle<()>>,
    listing: bool,
    timings: bool,
    boot_report: bool,
}

impl Work {
    /// Starts the thread, or `None` when it cannot be started (the loop
    /// then makes the calls itself).
    pub fn start(disk: Disk) -> Option<Work> {
        let (request, requests) = channel::<Job>();
        let (replies, reply) = channel();
        let thread = thread::Builder::new()
            .name(String::from("disk-work"))
            .spawn(move || {
                for job in requests {
                    if replies.send(run(&disk, job)).is_err() {
                        return;
                    }
                }
            })
            .ok()?;
        Some(Work {
            request: Some(request),
            reply,
            thread: Some(thread),
            listing: false,
            timings: false,
            boot_report: false,
        })
    }

    /// Whether the timings' write is out.
    pub fn timings_out(&self) -> bool {
        self.timings
    }

    /// Whether the boot report's write is out.
    pub fn boot_report_out(&self) -> bool {
        self.boot_report
    }

    fn send(&self, job: Job) -> bool {
        self.request.as_ref().is_some_and(|r| r.send(job).is_ok())
    }

    /// Asks for a look at the sticks, unless one is out.
    pub fn ask_list(&mut self, hide_waiting: bool) {
        if !self.listing {
            self.listing = self.send(Job::List { hide_waiting });
        }
    }

    /// Asks for the timings file to be written, unless a write is out.
    pub fn write_timings(&mut self, stick: String, name: &str, text: String) {
        if !self.timings {
            self.timings = self.send(Job::Timings {
                stick,
                name: name.to_string(),
                text,
            });
        }
    }

    /// Asks for the boot report to be composed and written, unless a
    /// write is out.
    pub fn write_boot_report(&mut self, report: BootReport) {
        if !self.boot_report {
            self.boot_report = self.send(Job::BootReport(Box::new(report)));
        }
    }

    /// What the thread has finished since the last call; empty while
    /// nothing is. Never waits.
    ///
    /// # Panics
    ///
    /// When the thread ended without answering, which only a panic on it
    /// does.
    pub fn take(&mut self) -> Vec<Reply> {
        let mut done = Vec::new();
        loop {
            match self.reply.try_recv() {
                Ok(reply) => done.push(self.settle(reply)),
                Err(TryRecvError::Empty) => return done,
                Err(TryRecvError::Disconnected) => {
                    assert!(
                        !(self.listing || self.timings || self.boot_report),
                        "the thread that calls the disk process ended"
                    );
                    return done;
                }
            }
        }
    }

    fn settle(&mut self, reply: Reply) -> Reply {
        match reply {
            Reply::Listed(_) => self.listing = false,
            Reply::Timings(_) => self.timings = false,
            Reply::BootReport { .. } => self.boot_report = false,
        }
        reply
    }

    /// Ends the thread after the jobs out to it are done, and gives what
    /// they came to. Waits as long as the write in progress takes.
    ///
    /// # Panics
    ///
    /// When the thread panicked.
    pub fn finish(&mut self) -> Vec<Reply> {
        self.request = None;
        if let Some(thread) = self.thread.take() {
            assert!(
                thread.join().is_ok(),
                "the thread that calls the disk process panicked"
            );
        }
        let mut done = Vec::new();
        while let Ok(reply) = self.reply.try_recv() {
            done.push(self.settle(reply));
        }
        done
    }
}

/// One job, on the thread.
fn run(disk: &Disk, job: Job) -> Reply {
    match job {
        Job::List { hide_waiting } => Reply::Listed(list(&mut lock(disk), hide_waiting)),
        Job::Timings { stick, name, text } => Reply::Timings(
            lock(disk)
                .write_copies(&stick, &name, text.as_bytes(), &[], true, false)
                .is_ok(),
        ),
        Job::BootReport(r) => {
            let kernel = bootreport::kernel_log(Path::new(bootreport::DMESG));
            let mut blocks = r.system.block(kernel.as_deref().ok());
            blocks.push_str(&radios::block(Path::new("/")));
            blocks.push_str(&devnodes::block(Path::new("/")));
            let text = bootreport::compose(&kernel, &r.inventory, &blocks, r.elapsed);
            let start = Instant::now();
            let wrote = lock(disk)
                .write_copies(&r.stick, &r.name, text.as_bytes(), &[], true, false)
                .map(drop)
                .map_err(|e| String::from(&*e));
            Reply::BootReport {
                name: r.name,
                wrote,
                took: start.elapsed(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pass_with_nothing_asked_skips_a_busy_disk_and_one_asking_waits() {
        let m = Mutex::new(1u8);
        assert!(for_pass(&m, false).is_some());
        let held = lock(&m);
        assert!(for_pass(&m, false).is_none());
        drop(held);
        assert!(for_pass(&m, true).is_some());
    }

    #[test]
    fn a_disk_lock_poisoned_by_a_panic_is_still_taken() {
        let m = Arc::new(Mutex::new(1u8));
        let m2 = Arc::clone(&m);
        let _ = thread::spawn(move || {
            let _g = m2.lock().unwrap();
            panic!("held");
        })
        .join();
        assert!(for_pass(&m, false).is_some());
        assert!(for_pass(&m, true).is_some());
    }
}
