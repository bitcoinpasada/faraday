//! The boot copier (`PLAN.md` §5.5), as the image runs it: as `ofboot`,
//! with no privilege, over the two FIFOs `rcS` makes.
//!
//! ```text
//! faraday-boot REQUESTS RESPONSES [SYS DEV VERSION]
//! faraday-boot --list [SYS DEV VERSION]
//! faraday-boot --ask list|read|forget REQUESTS RESPONSES
//! faraday-boot --ask write ID REQUESTS RESPONSES
//! ```
//!
//! It reads one request frame from REQUESTS and writes one response frame
//! to RESPONSES, for as long as the shell keeps the pipe open; when the
//! shell goes, the pipes are opened again and the next one is served. A
//! frame that does not parse ends the conversation rather than being
//! guessed at.
//!
//! `--list` prints the running release and every boot partition handed
//! out with the release string found on it, and exits. `--ask` sends one
//! request over the pipes, as the app's shell does, and prints the
//! answer. Both are for a machine being brought up, from the dev image's
//! console; neither can do anything the app cannot ask for.

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

use faraday_boot::Boots;
use faraday_boot::client::Client;
use faraday_boot::proto::{self, Request, Response};

fn serve(boots: &mut Boots, requests: &Path, responses: &Path) -> std::io::Result<()> {
    let mut rx = File::open(requests)?;
    let mut tx = OpenOptions::new().write(true).open(responses)?;
    while let Some((seq, frame)) = proto::receive(&mut rx)? {
        let answer = match Request::decode(&frame) {
            Ok(req) => {
                let what = format!("{req:?}");
                let answer = boots.handle(req);
                match &answer {
                    Response::Parts(_) | Response::Forgotten => {}
                    other => say!("faraday-boot: {what}: {other:?}"),
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

/// The roots: the device's, or the ones given after `at`.
fn roots(args: &[String], at: usize) -> (String, String, String) {
    match args.get(at..at + 3) {
        Some([s, d, v]) => (s.clone(), d.clone(), v.clone()),
        _ => ("/sys".into(), "/dev".into(), "/proc/version".into()),
    }
}

fn ask(args: &[String]) -> ExitCode {
    let (req, pipes) = match args.get(2).map(String::as_str) {
        Some("list") => (Request::List, 3),
        Some("read") => (Request::ReadSource, 3),
        Some("forget") => (Request::Forget, 3),
        Some("write") if args.len() == 6 => (
            Request::Write {
                target: args[3].clone(),
            },
            4,
        ),
        _ => {
            say!("usage: faraday-boot --ask list|read|forget|write ID REQUESTS RESPONSES");
            return ExitCode::from(2);
        }
    };
    let (Some(requests), Some(responses)) = (args.get(pipes), args.get(pipes + 1)) else {
        say!("usage: faraday-boot --ask list|read|forget|write ID REQUESTS RESPONSES");
        return ExitCode::from(2);
    };
    let answer = Client::open(Path::new(requests), Path::new(responses))
        .map_err(|e| e.to_string())
        .and_then(|mut c| c.ask(&req));
    match answer {
        Ok(Response::Parts(parts)) => {
            for p in parts {
                println!(
                    "part {} size {} release {} source {}",
                    p.id,
                    p.size,
                    p.release.as_deref().unwrap_or("none"),
                    p.source
                );
            }
            ExitCode::SUCCESS
        }
        Ok(Response::Failed(why)) => {
            println!("failed {why}");
            ExitCode::FAILURE
        }
        Ok(Response::Pulled) => {
            println!("pulled");
            ExitCode::FAILURE
        }
        Ok(other) => {
            println!("{other:?}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            say!("faraday-boot: {e}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("--list") => {
            let (sys, dev, version) = roots(&args, 2);
            let mut boots = Boots::new(Path::new(&sys), Path::new(&dev), Path::new(&version));
            for line in boots.explain() {
                println!("{line}");
            }
            return ExitCode::SUCCESS;
        }
        Some("--ask") => return ask(&args),
        _ => {}
    }
    if args.len() != 3 && args.len() != 6 {
        say!("usage: faraday-boot REQUESTS RESPONSES [SYS DEV VERSION]");
        return ExitCode::from(2);
    }
    let (sys, dev, version) = roots(&args, 3);
    let mut boots = Boots::new(Path::new(&sys), Path::new(&dev), Path::new(&version));
    loop {
        if let Err(e) = serve(&mut boots, Path::new(&args[1]), Path::new(&args[2])) {
            say!("faraday-boot: {e}");
            // A pipe that cannot be opened is not opened again at once.
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}
