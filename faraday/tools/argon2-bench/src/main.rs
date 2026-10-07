//! Times the vault presets of `docs/VAULT.md` §3.1 on this machine: the
//! slot-key derivation an unlock runs, at each preset's memory and passes,
//! one lane. Prints one row of the reference table.
//!
//! ```text
//! cargo run --release -p faraday-argon2-bench -- "Recent x86-64 laptop"
//! ```
//!
//! Run it in release: the vault is unlocked by a release build. Each
//! preset is timed three times and the middle time is kept.

use std::time::Instant;

use faraday_vault::{Cost, Error, Header, SLOT_SIZES, derive};

const PRESETS: [(&str, u32, u32); 4] = [
    ("Light", 64, 3),
    ("Standard", 256, 3),
    ("Strong", 1024, 4),
    ("Maximum", 2048, 2),
];

fn main() {
    let machine = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "This machine".to_string());
    let mut cells = Vec::new();
    for (name, mib, passes) in PRESETS {
        let header = Header {
            cost: Cost {
                memory_kib: mib * 1024,
                passes,
                lanes: 1,
            },
            slot_len: SLOT_SIZES[0],
            salt: [7; 32],
        };
        let mut times = Vec::new();
        let mut refused = None;
        for _ in 0..3 {
            let start = Instant::now();
            match derive(&header, b"argon2-bench") {
                Ok(_) => times.push(start.elapsed().as_secs_f64()),
                Err(Error::Memory(m)) => {
                    refused = Some(format!("cannot allocate {m} MiB"));
                    break;
                }
                Err(e) => {
                    refused = Some(e.reason());
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
        eprintln!("{name}: {mib} MiB, {passes} passes: {cell}");
        cells.push(cell);
    }
    println!("| {machine} | {} |", cells.join(" | "));
}
