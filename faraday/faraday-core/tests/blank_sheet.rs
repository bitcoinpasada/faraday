//! The blank template follows its `copy:` line (`docs/NEW-WALLET.md`
//! §14.7) and says to print at its true size on every page.

use faraday_core::backup::{BlankCopy, sheet_blank};
use faraday_core::pdf;
use osk_bip::keys::Network;

fn count(pdf: &[u8], s: &str) -> usize {
    pdf.windows(s.len()).filter(|w| *w == s.as_bytes()).count()
}

fn make(copy: BlankCopy) -> Vec<u8> {
    pdf::sheet(&sheet_blank(24, copy, None, Network::Mainnet, 2)).expect("a PDF")
}

#[test]
fn words_only_has_the_word_lines_and_no_grid() {
    let p = make(BlankCopy::Words);
    assert_eq!(count(&p, "(24) Tj"), 1);
    assert_eq!(count(&p, "(25) Tj"), 0);
}

#[test]
fn seedqr_only_has_the_grid_and_no_word_lines() {
    let p = make(BlankCopy::SeedQr);
    assert_eq!(count(&p, "(24) Tj"), 0);
    assert_eq!(count(&p, "(25) Tj"), 2);
}

#[test]
fn both_has_both_and_so_does_a_file_without_a_copy_line() {
    let p = make(BlankCopy::Both);
    assert_eq!((count(&p, "(24) Tj"), count(&p, "(25) Tj")), (1, 2));
    let text = sheet_blank(24, BlankCopy::Both, None, Network::Mainnet, 2);
    let without: String = text
        .lines()
        .filter(|l| !l.starts_with("copy:"))
        .map(|l| format!("{l}\n"))
        .collect();
    assert_eq!(pdf::sheet(&without).unwrap(), p);
}

#[test]
fn every_page_says_to_print_at_actual_size() {
    for copy in BlankCopy::ALL {
        assert_eq!(count(&make(copy), "Print at 100%"), 2);
    }
}
