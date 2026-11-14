//! Times the steps a file vault is sealed at (`faraday_vault::file`'s
//! `STEPS_KIB`, `PASSES` and `LANES`, `docs/VAULT.md` §3.1) on this
//! machine: the key derivation a seal runs, over a KDBX file with a
//! one-element document. Prints one row of the reference table, a cell
//! for each step.
//!
//! ```text
//! cargo run --release -p faraday-vault --example argon2_bench -- "Recent x86-64 laptop"
//! ```
//!
//! Run it in release: the vault is unlocked by a release build. Each
//! step is timed three times and the middle time is kept.

use std::time::Instant;

use faraday_crypto::work::Compute;
use faraday_vault::Cost;
use faraday_vault::file::{LANES, PASSES, STEPS_KIB};
use faraday_vault::kdbx::{self, Contents, Error, RANDOM_LEN};

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: a tool that draws nothing (docs/COMPUTE.md §3)"
)]
fn main() {
    let compute = Compute::new(|_| {});
    let c = &compute;
    let machine = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "This machine".to_string());
    let mut cells = Vec::new();
    for kib in STEPS_KIB {
        let mib = kib / 1024;
        let cost = Cost {
            memory_kib: kib,
            passes: PASSES,
            lanes: LANES,
        };
        let contents = Contents {
            document: b"<KeePassFile/>",
            binaries: faraday_vault::kdbx::Binaries::all(&[]),
            public_custom_data: None,
        };
        let mut times = Vec::new();
        let mut refused = None;
        for _ in 0..3 {
            let start = Instant::now();
            match kdbx::seal(c, "argon2-bench", cost, &[7; RANDOM_LEN], &contents, 4096) {
                Ok(_) => times.push(start.elapsed().as_secs_f64()),
                Err(Error::Memory(m)) => {
                    refused = Some(format!("cannot allocate {m} MiB"));
                    break;
                }
                Err(e) => {
                    refused = Some(format!("{e:?}"));
                    break;
                }
            }
        }
        let cell = match refused {
            Some(why) => why,
            None => {
                times.sort_by(f64::total_cmp);
                format!("{:.1} s", times[1])
            }
        };
        eprintln!("{mib} MiB, {PASSES} passes, {LANES} lanes: {cell}");
        cells.push(cell);
    }
    println!("| {machine} | {} |", cells.join(" | "));
}
