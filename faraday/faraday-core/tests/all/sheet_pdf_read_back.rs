//! The PDF sheets a wallet's backup prints, read back: the code drawn on
//! the page scans to the same text as the lines printed beside it, and
//! that text is the wallet's own, checksum included.

use faraday_core::testkit;
use faraday_core::wallet::{Origin, Session};
use faraday_core::wallet_backup::{sheet_share, sheet_wallet, split_plan, split_share};
use faraday_paper::pdf;

/// One drawn operator of a page: a filled square or a shown string.
enum Op {
    Square { x: f32, y: f32, side: f32 },
    Text { mono: bool, text: String },
}

/// The page operators of the first page of `pdf`, in drawing order.
/// Streams are plain, so the file is read as text.
fn first_page(pdf: &[u8]) -> Vec<Op> {
    let file: String = pdf.iter().map(|&b| b as char).collect();
    let start = file.find("stream\n").expect("a page stream") + "stream\n".len();
    let end = start + file[start..].find("endstream").expect("its end");
    let mut ops = Vec::new();
    for line in file[start..end].lines() {
        if let Some(rest) = line.strip_suffix(" re f") {
            let n: Vec<f32> = rest
                .split_whitespace()
                .filter_map(|t| t.parse().ok())
                .collect();
            // `grey g x y w h`: the code's squares are black.
            if rest.contains(" g ") && n.len() == 5 && n[0] == 0.0 {
                ops.push(Op::Square {
                    x: n[1],
                    y: n[2],
                    side: n[3],
                });
            }
        } else if line.starts_with("BT ") && line.ends_with(") Tj ET") {
            let open = line.find('(').unwrap();
            let body = &line[open + 1..line.len() - ") Tj ET".len()];
            let mut text = String::new();
            let mut chars = body.chars();
            while let Some(ch) = chars.next() {
                text.push(if ch == '\\' {
                    chars.next().unwrap()
                } else {
                    ch
                });
            }
            ops.push(Op::Text {
                mono: line.contains("/F3 "),
                text,
            });
        }
    }
    ops
}

/// What the code drawn on the page scans to: its squares put back on a
/// grid, with a quiet border, and read by the camera's decoder.
fn scanned(ops: &[Op]) -> String {
    let squares: Vec<(f32, f32, f32)> = ops
        .iter()
        .filter_map(|o| match o {
            Op::Square { x, y, side } => Some((*x, *y, *side)),
            Op::Text { .. } => None,
        })
        .collect();
    // The drawn square is a hair over a cell (0.05).
    let cell = squares.iter().map(|s| s.2).fold(f32::MAX, f32::min) - 0.05;
    let x0 = squares.iter().map(|s| s.0).fold(f32::MAX, f32::min);
    let y_top = squares.iter().map(|s| s.1).fold(f32::MIN, f32::max);
    let col = |x: f32| ((x - x0) / cell).round() as usize;
    // Pages count up from the bottom: the first row is the highest.
    let row = |y: f32| ((y_top - y) / cell).round() as usize;
    // A finder pattern stands in each top corner, so the widest column
    // is the last of the code.
    let n = squares.iter().map(|s| col(s.0)).max().unwrap() + 1;
    let (scale, quiet) = (6, 4);
    let side = (n + 2 * quiet) * scale;
    let mut px = vec![255u8; side * side];
    for s in &squares {
        let (cx, cy) = (col(s.0) + quiet, row(s.1) + quiet);
        for dy in 0..scale {
            let at = (cy * scale + dy) * side + cx * scale;
            px[at..at + scale].fill(0);
        }
    }
    let c = testkit::compute();
    let found = faraday_codec::decode::decode_luma(&c, side, side, &px);
    assert_eq!(found.len(), 1, "one code on the page");
    String::from_utf8(found[0].bytes.to_vec()).expect("text")
}

/// The monospaced lines of the page, in order.
fn printed(ops: &[Op]) -> Vec<&str> {
    ops.iter()
        .filter_map(|o| match o {
            Op::Text { mono: true, text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_wallet_sheet_pdf_scans_and_reads_as_the_wallets_descriptor() {
    let c = testkit::compute();
    let mut s = Session::on(testkit::net());
    let i = s
        .add_wallet_confirmed(&c, "Savings", &testkit::savings(), "test", Origin::Named)
        .unwrap();
    let want = String::from(&*s.wallets[i].policy().to_descriptor_checksummed());
    assert!(want.contains('#'), "{want}");
    let sheet = String::from(&*sheet_wallet(&c, &s, &s.wallets[i]));
    let pdf = pdf::sheet(&c, &sheet).unwrap();
    let ops = first_page(&pdf);
    assert_eq!(scanned(&ops), want);
    // The descriptor is wrapped over the first lines; they join to it.
    let mut joined = String::new();
    for line in printed(&ops) {
        if joined.len() >= want.len() {
            break;
        }
        joined.push_str(line);
    }
    assert_eq!(joined, want);
}

#[test]
fn a_share_sheet_pdf_scans_and_reads_as_the_share_it_carries() {
    let c = testkit::compute();
    let mut s = Session::on(testkit::net());
    let i = s
        .add_wallet_confirmed(&c, "Savings", &testkit::savings(), "test", Origin::Named)
        .unwrap();
    let w = &s.wallets[i];
    let (m, n) = Session::quorum_keys(w);
    let plan = split_plan(n, m, m.saturating_sub(1));
    let share = split_share(w, &plan[0]).expect("a share");
    let sheet = String::from(&*sheet_share(w, 1, plan.len(), &share));
    let pdf = pdf::sheet(&c, &sheet).unwrap();
    let ops = first_page(&pdf);
    let lines: Vec<&str> = share.lines().map(str::trim).collect();
    assert_eq!(scanned(&ops), lines.join("\n"));
    let shown: String = printed(&ops).concat();
    let want: String = lines.concat();
    assert_eq!(shown, want);
}
