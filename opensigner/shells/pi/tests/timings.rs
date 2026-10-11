//! What the dev card leaves behind for the owner to read.
//!
//! The release card has no console, no network and no login, so the only
//! way a number reaches a person is a file on the `OSKDATA` partition.
//! The shell is run as it runs on the device, except that `--fb` points
//! at a regular file, `--input` at `/dev/null` and `--files` at a
//! directory standing in for the card.

use std::path::PathBuf;
use std::process::Command;

/// The five buckets a block names.
const BUCKETS: [&str; 5] = ["touch", "tick", "camera", "convert", "write"];

/// Runs the shell against files and returns the directory it was given.
fn run(name: &str, timings: bool) -> PathBuf {
    let dir: PathBuf = [env!("CARGO_TARGET_TMPDIR"), "timings", name]
        .iter()
        .collect();
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("clear the directory");
    }
    std::fs::create_dir_all(&dir).expect("make the directory");
    let fb = dir.join("panel.fb");
    let mut command = Command::new(env!("CARGO_BIN_EXE_opensigner-pi"));
    command
        .args(["--fb", fb.to_str().expect("utf-8 path")])
        .args(["--input", "/dev/null"])
        .args(["--files", dir.to_str().expect("utf-8 path")])
        .args(["--size", "480x640"])
        .args(["--depth", "16"])
        .args(["--dpi", "286"])
        .args(["--frames", "1"])
        .arg("--no-camera");
    if timings {
        command.arg("--timings");
    }
    let status = command.status().expect("the shell runs");
    assert!(status.success(), "the shell exited with {status}");
    dir
}

#[test]
fn a_dev_run_leaves_its_timings_on_the_card() {
    let dir = run("dev", true);
    let text =
        std::fs::read_to_string(dir.join("opensigner-timings.txt")).expect("the timings file");

    let mut lines = text.lines();
    let header = lines.next().expect("a header line");
    assert!(
        header.contains("opensigner-pi") && header.contains("480x640"),
        "the header names the binary and the panel: {header}"
    );
    let block = lines.next().expect("a block");
    assert!(
        block.contains("passes") && block.contains("touches"),
        "the block starts with the clock, the passes and the touches: {block}"
    );
    for bucket in BUCKETS {
        assert!(
            text.lines().any(|l| l.starts_with(bucket)),
            "the block names {bucket}:\n{text}"
        );
    }
}

#[test]
fn a_release_run_writes_no_timings_at_all() {
    let dir = run("release", false);

    assert!(!dir.join("opensigner-timings.txt").exists());
}
