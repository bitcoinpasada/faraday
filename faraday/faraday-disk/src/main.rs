//! The disk process (`PLAN.md` §4.3), as the image runs it: as `ofdisk`,
//! with no privilege, over the two FIFOs `rcS` makes.
//!
//! ```text
//! faraday-disk REQUESTS RESPONSES [SYS DEV]
//! faraday-disk --list [SYS DEV]
//! ```
//!
//! `--list` prints every partition considered and what became of it, and
//! exits: for a machine being brought up, from the dev image's console.
//!
//! It reads one request frame from REQUESTS and writes one response frame
//! to RESPONSES, for as long as the shell keeps the pipe open. When the
//! shell goes (the app locks and a fresh process starts), the pipes are
//! opened again and the next shell is served. A frame that does not parse
//! ends the conversation rather than being guessed at.
//!
//! A picture's QR codes are read on a thread of their own: a photograph
//! takes seconds on a Pi, and the requests asked meanwhile are answered
//! first. Its answer carries its request's sequence number, and goes only
//! to the shell that asked; the shell asks for one picture at a time
//! (`docs/SPEED.md` §9).

/// A line on stderr that is never fatal: once `rcS` has ended, the
/// console this process was started on may refuse writes, and
/// `eprintln!` panics when a write fails.
macro_rules! say {
    ($($t:tt)*) => {{
        use std::io::Write as _;
        let _ = writeln!(std::io::stderr(), $($t)*);
    }};
}

use std::fs::{File, OpenOptions};
use std::path::Path;
use std::process::ExitCode;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, PoisonError};

use faraday_disk::Disks;
use faraday_files::proto::{self, Request, Response};

/// The shell being served now, by the number of its conversation, and
/// the pipe its answers go down; `None` between shells.
type Now = Arc<Mutex<Option<(u64, File)>>>;

/// Sends `answer` to the shell of conversation `talk`, when it is still the
/// one served, and wipes it.
fn answer_to(now: &Now, talk: u64, seq: u32, mut answer: Response) -> std::io::Result<()> {
    let mut encoded = answer.encode();
    answer.wipe();
    let mut guard = now.lock().unwrap_or_else(PoisonError::into_inner);
    let sent = match guard.as_mut() {
        Some((t, tx)) if *t == talk => proto::send(tx, seq, &encoded),
        _ => Ok(()),
    };
    zeroize::Zeroize::zeroize(&mut encoded);
    sent
}

/// The thread that reads pictures for their codes: one picture at a time,
/// in the order asked, each answered to the conversation that asked. A
/// picture asked by a shell that has gone is not read.
fn pictures(now: Now) -> Sender<(u64, u32, Vec<u8>)> {
    let (jobs, asked) = mpsc::channel::<(u64, u32, Vec<u8>)>();
    std::thread::spawn(move || {
        for (talk, seq, mut bytes) in asked {
            let current = now
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .as_ref()
                .is_some_and(|(t, _)| *t == talk);
            if !current {
                zeroize::Zeroize::zeroize(&mut bytes);
                continue;
            }
            let answer = faraday_files::picture_answer(bytes);
            if let Err(e) = answer_to(&now, talk, seq, answer) {
                say!("faraday-disk: {e}");
            }
        }
    });
    jobs
}

fn serve(
    disks: &mut Disks,
    requests: &Path,
    responses: &Path,
    talk: u64,
    now: &Now,
    pictures: &Sender<(u64, u32, Vec<u8>)>,
) -> std::io::Result<()> {
    let mut rx = File::open(requests)?;
    let tx = OpenOptions::new().write(true).open(responses)?;
    *now.lock().unwrap_or_else(PoisonError::into_inner) = Some((talk, tx));
    let served = serve_on(disks, &mut rx, talk, now, pictures);
    // Whatever a picture still being read finds, this shell has gone.
    *now.lock().unwrap_or_else(PoisonError::into_inner) = None;
    served
}

fn serve_on(
    disks: &mut Disks,
    rx: &mut File,
    talk: u64,
    now: &Now,
    pictures: &Sender<(u64, u32, Vec<u8>)>,
) -> std::io::Result<()> {
    let mut last = String::new();
    while let Some((seq, mut frame)) = proto::receive(rx)? {
        // A file read or written may be a seed in the clear, and this
        // process outlives the app's locks: what passed through is wiped.
        let decoded = Request::decode(&frame);
        zeroize::Zeroize::zeroize(&mut frame);
        let answer = match decoded {
            // The picture's bytes are read here; its codes on the
            // pictures' thread, which answers.
            Ok(Request::ReadQr { stick, name }) => {
                match disks.handle(Request::Read { stick, name }) {
                    Response::Bytes(bytes) => {
                        if let Err(mpsc::SendError((_, _, mut bytes))) =
                            pictures.send((talk, seq, bytes))
                        {
                            zeroize::Zeroize::zeroize(&mut bytes);
                            Response::Failed("the pictures cannot be read now".into())
                        } else {
                            continue;
                        }
                    }
                    other => other,
                }
            }
            Ok(req) => {
                let answer = disks.handle(req);
                // A listing that changed is said on the console, for a
                // machine being brought up.
                if let Response::Sticks(s) = &answer {
                    let now: Vec<String> = s
                        .iter()
                        .map(|s| format!("{} {} files", s.id, s.files.len()))
                        .collect();
                    let now = now.join(", ");
                    if now != last {
                        say!("faraday-disk: listing [{now}]");
                        last = now;
                    }
                }
                answer
            }
            Err(_) => {
                answer_to(now, talk, seq, Response::Failed("malformed request".into()))?;
                return Ok(());
            }
        };
        answer_to(now, talk, seq, answer)?;
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--list") {
        let (sys, dev) = match args.len() {
            4 => (args[2].as_str(), args[3].as_str()),
            _ => ("/sys", "/dev"),
        };
        let disks = Disks::new(Path::new(sys), Path::new(dev));
        for line in disks.explain() {
            println!("{line}");
        }
        return ExitCode::SUCCESS;
    }
    if args.len() != 3 && args.len() != 5 {
        say!("usage: faraday-disk REQUESTS RESPONSES [SYS DEV]");
        return ExitCode::from(2);
    }
    let (sys, dev) = match args.len() {
        5 => (args[3].as_str(), args[4].as_str()),
        _ => ("/sys", "/dev"),
    };
    let mut disks = Disks::new(Path::new(sys), Path::new(dev));
    let now: Now = Arc::new(Mutex::new(None));
    let jobs = pictures(now.clone());
    // Each shell served is a conversation of its own.
    let mut talk = 0u64;
    loop {
        talk += 1;
        if let Err(e) = serve(
            &mut disks,
            Path::new(&args[1]),
            Path::new(&args[2]),
            talk,
            &now,
            &jobs,
        ) {
            say!("faraday-disk: {e}");
            // A pipe that cannot be opened is not opened again at once.
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}
