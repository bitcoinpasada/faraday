//! The one worker thread: Argon2id and QR encoding never run while
//! drawing (`docs/COMPUTE.md` §1). A job goes in over an `mpsc` channel,
//! the answer comes back, and the shell is woken through a closure.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use aqrv::{Item, Refused, Size, Step};
use faraday_codec::qr::{Ecc, Payload, QrMatrix, encode};
use faraday_crypto::work::Compute;
use zeroize::Zeroizing;

/// What the thread is asked.
pub enum Job {
    /// Time one 64 MiB run.
    Time,
    /// Seal an item and draw its code.
    Make {
        /// Which request this is, so a cancelled one is told apart.
        id: u64,
        /// The item.
        item: Item,
        /// The passphrase.
        passphrase: Zeroizing<String>,
        /// The memory step.
        step: Step,
        /// The size.
        size: Size,
        /// Random bytes to draw from, from the shell's entropy channel.
        random: Zeroizing<Vec<u8>>,
    },
}

/// Why no code was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fail {
    /// The library refused the item or the passphrase.
    Refused,
    /// Argon2id's memory could not be had.
    NotEnoughMemory,
}

/// A code made.
pub struct Made {
    /// The envelope's bytes.
    pub bytes: Vec<u8>,
    /// The QR code of those bytes, kept to draw from.
    pub matrix: QrMatrix,
    /// The size.
    pub size: Size,
    /// The memory step.
    pub step: Step,
}

/// What the thread answers.
pub enum Answer {
    /// The 64 MiB run's time.
    Timed(Duration),
    /// A made code, or why not, for request `id`.
    Made(u64, Box<Result<Made, Fail>>),
}

/// The thread's two ends.
pub struct Worker {
    jobs: Sender<Job>,
    answers: Receiver<Answer>,
}

impl Worker {
    /// Starts the thread; `wake` is called after each answer is sent.
    pub fn start(wake: impl Fn() + Send + 'static) -> Worker {
        let (jobs, job_rx) = channel::<Job>();
        let (answer_tx, answers) = channel::<Answer>();
        std::thread::spawn(move || {
            // A token cannot cross threads, so the thread mints its own.
            let c = Compute::new(|_| {});
            while let Ok(job) = job_rx.recv() {
                let answer = answer(&c, job);
                if answer_tx.send(answer).is_err() {
                    break;
                }
                wake();
            }
        });
        Worker { jobs, answers }
    }

    /// Asks the thread for `job`.
    pub fn ask(&self, job: Job) {
        let _ = self.jobs.send(job);
    }

    /// The next answer, if one is ready.
    pub fn poll(&self) -> Option<Answer> {
        self.answers.try_recv().ok()
    }

    /// The next answer, waiting for it.
    pub fn wait(&self) -> Option<Answer> {
        self.answers.recv().ok()
    }
}

/// Costs a 64 MiB Argon2id run (`Job::Time`) or an Argon2id run at the
/// step and a QR encoding (`Job::Make`).
fn answer(c: &Compute, job: Job) -> Answer {
    match job {
        Job::Time => {
            let started = Instant::now();
            let item = Item::Note(Zeroizing::new("x".to_owned()));
            let mut random = |b: &mut [u8]| b.fill(0x5a);
            let _ = aqrv::seal(c, &item, "timing", Step::Mib64, Size::Small, &mut random);
            Answer::Timed(started.elapsed())
        }
        Job::Make {
            id,
            item,
            passphrase,
            step,
            size,
            random,
        } => Answer::Made(
            id,
            Box::new(make(c, &item, &passphrase, step, size, &random)),
        ),
    }
}

fn make(
    c: &Compute,
    item: &Item,
    passphrase: &str,
    step: Step,
    size: Size,
    random: &[u8],
) -> Result<Made, Fail> {
    let mut at = 0;
    let mut short = false;
    let mut draw = |out: &mut [u8]| {
        if at + out.len() <= random.len() {
            out.copy_from_slice(&random[at..at + out.len()]);
            at += out.len();
        } else {
            // Never reuse a byte: the pool ran out, so the code is not made.
            short = true;
            out.fill(0);
        }
    };
    let sealed = aqrv::seal(c, item, passphrase, step, size, &mut draw);
    let bytes = match sealed {
        Ok(_) if short => return Err(Fail::Refused),
        Ok(bytes) => bytes,
        Err(Refused::NotEnoughMemory) => return Err(Fail::NotEnoughMemory),
        Err(_) => return Err(Fail::Refused),
    };
    let ecc = match size {
        Size::Small => Ecc::Low,
        Size::Large => Ecc::Medium,
    };
    let matrix = encode(c, Payload::Bytes(&bytes), ecc).map_err(|_| Fail::Refused)?;
    Ok(Made {
        bytes,
        matrix,
        size,
        step,
    })
}
