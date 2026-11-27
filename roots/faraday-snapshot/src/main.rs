//! The tour root, which never ships: it builds any of the three product crates' models over the
//! fixture adapters and writes a picture at each named step (09 §7; 06 §8).

#![expect(dead_code, reason = "the fields are read once the bodies are written")]

mod png;
mod runner;
mod screens;
mod targets;
mod tours;

use std::process::ExitCode;

fn main() -> ExitCode {
    let command = runner::parse();
    runner::run(&command)
}
