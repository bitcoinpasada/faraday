//! The image's inittab restarts the computer when the app ends with
//! Restart's status and powers it off otherwise (`docs/FLOWS.md`,
//! Restart; F42): the app's line, run with a stub app, `reboot` and
//! `poweroff`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use faraday_core::{RESTART_CODE, RESTART_COMPUTER_CODE};

const APP: &str = "/bin/setpriv --nnp /usr/bin/setuidgid opensigner /usr/bin/opensigner-pi \
                   $(cat /etc/opensigner/args)";

fn image(file: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../image");
    fs::read_to_string(path.join(file)).expect("an inittab in the image tree")
}

/// The command of the line that runs the app, from `sh -c '` to its
/// closing quote.
fn app_line(inittab: &str) -> String {
    let line = inittab
        .lines()
        .find(|l| l.contains("opensigner-pi"))
        .expect("a line that runs the app");
    let start = line.find("sh -c '").expect("a shell command") + "sh -c '".len();
    line[start..line.rfind('\'').expect("a closing quote")].to_string()
}

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Scratch {
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "faraday-inittab-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch directory");
        Scratch(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Runs the line with an app that ends with each of `codes` in turn,
/// and says what the line did at the end: `reboot`, `poweroff` or
/// nothing.
fn run(inittab: &str, codes: &[u8]) -> String {
    let scratch = Scratch::new();
    let dir = &scratch.0;
    let queue = dir.join("codes");
    let log = dir.join("log");
    let list: Vec<String> = codes.iter().map(u8::to_string).collect();
    fs::write(&queue, list.join("\n") + "\n").expect("the codes");
    let stub = |name: &str, body: &str| {
        let path = dir.join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("a stub");
        Command::new("chmod")
            .arg("+x")
            .arg(&path)
            .status()
            .expect("chmod");
        path
    };
    let app = stub(
        "app",
        &format!(
            "n=$(head -n 1 {q}); sed -i 1d {q}; exit $n",
            q = queue.display()
        ),
    );
    let reboot = stub("reboot", &format!("echo reboot $* >> {}", log.display()));
    let poweroff = stub(
        "poweroff",
        &format!("echo poweroff $* >> {}", log.display()),
    );
    let command = app_line(inittab)
        .replace(APP, &app.display().to_string())
        .replace("2>/dev/kmsg", "2>/dev/null")
        .replace("> /dev/kmsg", "> /dev/null")
        .replace("sleep 60", ":")
        .replace("/sbin/reboot", &reboot.display().to_string())
        .replace("/sbin/poweroff", &poweroff.display().to_string());
    assert!(command.contains(&app.display().to_string()), "{command}");
    let status = Command::new("/bin/sh")
        .arg("-c")
        .arg(&command)
        .status()
        .expect("the line runs");
    assert!(status.success() || fs::metadata(&log).is_ok(), "{status}");
    fs::read_to_string(&log).unwrap_or_default()
}

fn check(inittab: &str) {
    let lock = RESTART_CODE;
    let restart = RESTART_COMPUTER_CODE;
    assert_eq!(run(inittab, &[restart]), "reboot -f\n", "Restart's status");
    assert_eq!(run(inittab, &[0]), "poweroff -f\n", "a power-off");
    assert_eq!(run(inittab, &[1]), "poweroff -f\n", "a failure");
    assert_eq!(
        run(inittab, &[lock, lock, restart]),
        "reboot -f\n",
        "after two locks, the last status counts"
    );
    assert_eq!(
        run(inittab, &[lock, 0]),
        "poweroff -f\n",
        "a lock, then off"
    );
}

#[test]
fn the_release_inittab_restarts_on_restarts_status_and_powers_off_otherwise() {
    check(&image("common/rootfs-overlay/etc/inittab"));
}

#[test]
fn the_dev_inittab_does_the_same() {
    check(&image("variants/dev/inittab.in"));
}
