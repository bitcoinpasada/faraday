//! Fills volvelle-website's checksum worksheet (`../volvelle/`, CC0)
//! for every string in a list, as a person generating a checksum would:
//! the header and data boxes typed in, the checksum boxes left for the
//! sheet to work from its fixed bottom row. Writes every cell of every
//! sheet, row by row, as JSON.
//!
//!     cargo run --manifest-path tools/vectors/codex32-volvelle/harness/Cargo.toml -- \
//!         tools/vectors/codex32-volvelle/strings.txt tools/vectors/codex32-volvelle/cells.json
//!
//! A row is a string of its cells' characters, upper case, a space for
//! a blank cell. The rows are volvelle's: the first row (13 boxes from
//! box 4), the residue of `MS`, then for each step a sum row (13 sums
//! and the step's two data boxes) and a residue row, and the final row.
//! The sheet's checksum, read back from its boxes, must be the string's
//! own; the run stops if it is not.

#![allow(dead_code)]

#[path = "../../volvelle/checksum_worksheet.rs"]
mod checksum_worksheet;
#[path = "../../volvelle/error.rs"]
mod error;
#[path = "../../volvelle/fe.rs"]
mod fe;

use checksum_worksheet::Worksheet;
use fe::Checksum;

const CHECKSUM: usize = 13;

fn sheet(s: &str) -> Vec<String> {
    let size = s.len();
    let chars: Vec<char> = s[3..].to_ascii_uppercase().chars().collect();
    let data_len = size - 2 - CHECKSUM - 1;
    let pairs = data_len / 2;
    let mut w = Worksheet::new("ms", size, Checksum::Codex32, 0).expect("a sheet of this size");
    for (i, ch) in chars[..CHECKSUM].iter().enumerate() {
        w.handle_input_change(0, i, &ch.to_string()).expect("row 0");
    }
    for p in 0..pairs {
        for k in 0..2 {
            let at = CHECKSUM + 2 * p + k;
            if at < data_len {
                w.handle_input_change(2 + 2 * p, CHECKSUM + k, &chars[at].to_string())
                    .expect("a data box");
            }
        }
    }
    let mut flat = String::new();
    w.cells_into_str(&mut flat);
    let flat: Vec<char> = flat.chars().collect();
    let mut lens = vec![CHECKSUM, CHECKSUM];
    for _ in 0..pairs {
        lens.push(CHECKSUM + 2);
        lens.push(CHECKSUM);
    }
    lens.push(CHECKSUM);
    assert_eq!(lens.iter().sum::<usize>(), flat.len(), "{s}: cell count");
    let mut rows = Vec::new();
    let mut at = 0;
    for n in lens {
        rows.push(flat[at..at + n].iter().collect::<String>());
        at += n;
    }
    // The string as the sheet holds it: the first row, then each sum
    // row's two data boxes.
    let mut back: String = rows[0].clone();
    for p in 0..pairs {
        back.extend(rows[2 + 2 * p].chars().skip(CHECKSUM));
    }
    assert_eq!(back, chars.iter().collect::<String>(), "{s}: the sheet's checksum");
    rows
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let list = std::fs::read_to_string(&args[1]).expect("the list of strings");
    let mut out = String::from(
        "{\n \"source\": \"volvelle-website volvelle-wasm/src/checksum_worksheet.rs (README.md)\",\n \"sheets\": [\n",
    );
    let strings: Vec<&str> = list.lines().filter(|l| !l.trim().is_empty()).collect();
    for (n, s) in strings.iter().enumerate() {
        let rows = sheet(s.trim());
        out.push_str(&format!("  {{\n   \"string\": \"{}\",\n   \"rows\": [\n", s.trim()));
        for (i, r) in rows.iter().enumerate() {
            let comma = if i + 1 < rows.len() { "," } else { "" };
            out.push_str(&format!("    \"{r}\"{comma}\n"));
        }
        let comma = if n + 1 < strings.len() { "," } else { "" };
        out.push_str(&format!("   ]\n  }}{comma}\n"));
    }
    out.push_str(" ]\n}\n");
    std::fs::write(&args[2], out).expect("the cells file");
}
