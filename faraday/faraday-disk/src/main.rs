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

use faraday_disk::Disks;
use faraday_files::proto::{self, Request, Response};

fn serve(disks: &mut Disks, requests: &Path, responses: &Path) -> std::io::Result<()> {
    let mut rx = File::open(requests)?;
    let mut tx = OpenOptions::new().write(true).open(responses)?;
    let mut last = String::new();
    while let Some((seq, frame)) = proto::receive(&mut rx)? {
        let answer = match Request::decode(&frame) {
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
                let failed = Response::Failed("malformed request".into());
                let _ = proto::send(&mut tx, seq, &failed.encode());
                return Ok(());
            }
        };
        proto::send(&mut tx, seq, &answer.encode())?;
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
    loop {
        if let Err(e) = serve(&mut disks, Path::new(&args[1]), Path::new(&args[2])) {
            say!("faraday-disk: {e}");
            // A pipe that cannot be opened is not opened again at once.
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}
