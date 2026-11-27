//! The tour root, which never ships: it builds any of the three product crates' models over the
//! fixture adapters and writes a picture at each named step (09 §7; 06 §8).

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "a step is taken only by a tour's body, written with its feature"
    )
)]

mod png;
mod runner;
mod screens;
mod targets;
mod tours;

use std::process::ExitCode;

fn main() -> ExitCode {
    match runner::parse() {
        Ok(command) => runner::run(&command),
        Err(e) => {
            eprintln!("faraday-snapshot: {e}");
            ExitCode::from(2)
        }
    }
}
