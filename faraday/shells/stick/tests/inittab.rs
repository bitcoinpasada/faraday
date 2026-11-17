//! The image's inittab restarts the computer when the app ends with
//! Restart's status and powers it off otherwise (`docs/FLOWS.md`,
//! Restart; F42): the app's line, run with a stub app, `reboot` and
//! `poweroff`; the disk process is ended and the block cache dropped
//! before either, and not after a lock.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use faraday_core::{RESTART_CODE, RESTART_COMPUTER_CODE};

const APP: &str = "/bin/setpriv --nnp /usr/bin/setuidgid faraday /usr/bin/faraday \
                   $(cat /etc/faraday/args /run/faraday-screen/args 2>/dev/null)";

fn image(file: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../image");
    fs::read_to_string(path.join(file)).expect("an inittab in the image tree")
}

/// The command of the line that runs the app, from `sh -c '` to its
/// closing quote.
fn app_line(inittab: &str) -> String {
    let line = inittab
        .lines()
        .find(|l| l.contains("/usr/bin/faraday "))
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
    run_in(inittab, codes, "")
}

/// [`run`] with `trace` added to the stub app's body, which runs before
/// its exit; the scratch directory is the `D` variable.
fn run_in(inittab: &str, codes: &[u8], trace: &str) -> String {
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
            "D={d}; {trace}\nn=$(head -n 1 {q}); sed -i 1d {q}; exit $n",
            d = dir.display(),
            q = queue.display()
        ),
    );
    // What the disk process's end and the cache drop have done by the
    // time the computer goes is written in front of its own line.
    let drop_file = dir.join("drop_caches");
    let stop_file = dir.join("disk-stop");
    let seen = format!(
        "[ -e {s} ] && echo stop-file >> {l}; [ \"$(cat {d} 2>/dev/null)\" = 3 ] && echo caches-dropped >> {l}",
        s = stop_file.display(),
        d = drop_file.display(),
        l = log.display()
    );
    let reboot = stub(
        "reboot",
        &format!("{seen}; echo reboot $* >> {}", log.display()),
    );
    let poweroff = stub(
        "poweroff",
        &format!("{seen}; echo poweroff $* >> {}", log.display()),
    );
    let killall = stub("killall", &format!("echo killall $* >> {}", log.display()));
    let command = app_line(inittab)
        .replace(APP, &app.display().to_string())
        .replace("2>/dev/kmsg", "2>/dev/null")
        .replace("> /dev/kmsg", "> /dev/null")
        .replace("sleep $w", ":")
        .replace("/usr/bin/killall", &killall.display().to_string())
        .replace("/run/faraday-disk-stop", &stop_file.display().to_string())
        .replace("/proc/sys/vm/drop_caches", &drop_file.display().to_string())
        .replace("/sbin/reboot", &reboot.display().to_string())
        .replace("/sbin/poweroff", &poweroff.display().to_string());
    assert!(command.contains(&app.display().to_string()), "{command}");
    assert!(!command.contains("sleep "), "a wait left in {command}");
    let status = Command::new("/bin/sh")
        .arg("-c")
        .arg(&command)
        .status()
        .expect("the line runs");
    assert!(status.success() || fs::metadata(&log).is_ok(), "{status}");
    fs::read_to_string(&log).unwrap_or_default()
}

fn check(inittab: &str) {
    let ended = "killall faraday-disk\nstop-file\ncaches-dropped\n";
    let lock = RESTART_CODE;
    let restart = RESTART_COMPUTER_CODE;
    assert_eq!(
        run(inittab, &[restart]),
        format!("{ended}reboot -f\n"),
        "Restart's status"
    );
    assert_eq!(
        run(inittab, &[0]),
        format!("{ended}poweroff -f\n"),
        "a power-off"
    );
    assert_eq!(
        run(inittab, &[1]),
        format!("{ended}poweroff -f\n"),
        "a failure"
    );
    assert_eq!(
        run(inittab, &[lock, lock, restart]),
        format!("{ended}reboot -f\n"),
        "after two locks, the last status counts"
    );
    assert_eq!(
        run(inittab, &[lock, 0]),
        format!("{ended}poweroff -f\n"),
        "a lock, then off"
    );
}

#[test]
fn the_release_inittab_restarts_on_restarts_status_and_powers_off_otherwise() {
    check(&image("common/rootfs-overlay/etc/inittab"));
}

/// A lock starts the app again as a new process that holds nothing the
/// old one held (`docs/SECURITY-MODEL.md`, Stateless): each run of the
/// app, standing for a session with a vault open, notes its process id and
/// whether a variable the earlier run set in its memory is there; after
/// each lock the next run has a new id and finds none.
fn check_new_process(inittab: &str) {
    let trace = "echo pid $$ >> $D/log; \
                 [ -n \"$OPEN_VAULT\" ] && echo old-state >> $D/log; \
                 OPEN_VAULT=seed; export OPEN_VAULT";
    let lock = RESTART_CODE;
    let log = run_in(inittab, &[lock, lock, lock, 0], trace);
    let pids: Vec<&str> = log.lines().filter_map(|l| l.strip_prefix("pid ")).collect();
    assert_eq!(pids.len(), 4, "the app ran after each lock: {log}");
    let mut unique = pids.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), 4, "a new process each time: {log}");
    assert!(
        !log.contains("old-state"),
        "state carried over a lock: {log}"
    );
    assert!(log.ends_with("poweroff -f\n"), "{log}");
}

#[test]
fn a_lock_starts_the_app_again_as_a_new_process_with_none_of_the_old_state() {
    for file in [
        "common/rootfs-overlay/etc/inittab",
        "variants/dev/inittab.in",
    ] {
        check_new_process(&image(file));
    }
}

#[test]
fn the_dev_inittab_does_the_same() {
    check(&image("variants/dev/inittab.in"));
}

/// The release image writes no boot report: nothing in its init opens the
/// kernel log to the app, its BusyBox has no `dmesg`, and `build.sh` hands
/// the app `--boot-report` only inside the dev branch. The dev image has
/// all three.
#[test]
fn only_the_dev_image_can_write_a_boot_report() {
    assert!(!image("common/rootfs-overlay/etc/inittab").contains("dmesg_restrict"));
    assert!(image("variants/dev/inittab.in").contains("echo 0 > /proc/sys/kernel/dmesg_restrict"));
    assert!(!image("common/busybox.fragment").contains("CONFIG_DMESG=y"));
    assert!(image("variants/dev/busybox.fragment").contains("CONFIG_DMESG=y"));

    let build = image("build.sh");
    let lines: Vec<&str> = build.lines().collect();
    let flag = lines
        .iter()
        .position(|l| l.contains("--boot-report") && !l.trim_start().starts_with('#'))
        .expect("build.sh passes --boot-report to the dev app");
    let branch = lines[..flag]
        .iter()
        .rev()
        .find(|l| l.starts_with("if "))
        .expect("the flag is inside an if");
    assert_eq!(*branch, "if [ \"$DEV\" = 1 ]; then");
}
