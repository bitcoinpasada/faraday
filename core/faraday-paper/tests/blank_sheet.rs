//! The blank template follows its `copy:` line (`docs/WALLETS.md` §13)
//! and says to print at its true size on every page.

use faraday_crypto::work::Compute;
use faraday_paper::blank::{BlankCopy, BlankWallet, sheet_blank};
use faraday_paper::pdf;

fn count(pdf: &[u8], s: &str) -> usize {
    pdf.windows(s.len()).filter(|w| *w == s.as_bytes()).count()
}

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-paper's tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::unchecked()
}

fn make(copy: BlankCopy) -> Vec<u8> {
    let c = compute();
    pdf::sheet(&c, &sheet_blank(24, copy, None, "mainnet", 2)).expect("a PDF")
}

/// Invariant: A blank template for words alone has the numbered word lines and no SeedQR grid.
/// Regression: The grid is drawn whatever the sheet's copy line says.
/// Owner: domain
#[test]
fn words_only_has_the_word_lines_and_no_grid() {
    let p = make(BlankCopy::Words);
    assert_eq!(count(&p, "(24) Tj"), 1);
    assert_eq!(count(&p, "(25) Tj"), 0);
}

/// Invariant: A blank template for a SeedQR alone has the grid and no word lines.
/// Regression: The word lines are drawn whatever the sheet's copy line says.
/// Owner: domain
#[test]
fn seedqr_only_has_the_grid_and_no_word_lines() {
    let p = make(BlankCopy::SeedQr);
    assert_eq!(count(&p, "(24) Tj"), 0);
    assert_eq!(count(&p, "(25) Tj"), 2);
}

/// Invariant: A blank template for both has the word lines and the grid, as a sheet without a copy line does.
/// Regression: A sheet file without a copy line is read as words alone.
/// Owner: domain
#[test]
fn both_has_both_and_so_does_a_file_without_a_copy_line() {
    let c = compute();
    let p = make(BlankCopy::Both);
    assert_eq!((count(&p, "(24) Tj"), count(&p, "(25) Tj")), (1, 2));
    let text = sheet_blank(24, BlankCopy::Both, None, "mainnet", 2);
    let without: String = text
        .lines()
        .filter(|l| !l.starts_with("copy:"))
        .map(|l| format!("{l}\n"))
        .collect();
    assert_eq!(pdf::sheet(&c, &without).unwrap(), p);
}

/// Invariant: Every page of a blank template says to print it at its true size.
/// Regression: The print-size line is left off a page, so a printer scales the grid.
/// Owner: domain
#[test]
fn every_page_says_to_print_at_actual_size() {
    for copy in BlankCopy::ALL {
        assert_eq!(count(&make(copy), "Print at 100%"), 2);
    }
}

/// Invariant: A blank template says its words are English BIP-39 and has a line to tick whether a passphrase is kept elsewhere.
/// Regression: The passphrase line is dropped from the template.
/// Owner: domain
#[test]
fn the_sheet_says_it_is_bip39_and_has_a_passphrase_tick_line() {
    let p = make(BlankCopy::Both);
    assert_eq!(count(&p, "BIP-39 words, English."), 1);
    assert_eq!(count(&p, "Passphrase: none [ ] kept elsewhere [ ]"), 1);
}

/// Invariant: A wallet's name, shape or path on a blank template stays on its own line, whatever it holds.
/// Regression: A name holding a line break writes a path line of its own, and the sheet prints that path.
/// Owner: domain
#[test]
fn a_line_break_in_a_wallet_value_adds_no_line_to_the_sheet() {
    let wallet = BlankWallet {
        name: "Savings\npath: 0'/0'/9'",
        shape: "one key\r\nnetwork: testnet",
        paths: &["84'/0'/0'\nname: other"],
    };
    let text = sheet_blank(24, BlankCopy::Words, Some(wallet), "Mainnet", 2);
    let starting = |key: &str| text.lines().filter(|l| l.starts_with(key)).count();
    assert_eq!(
        (starting("path:"), starting("network:"), starting("name:")),
        (1, 1, 1)
    );
    assert!(text.contains("network: mainnet\n"));
    assert!(text.contains("path: m/84'/0'/0' name: other\n"));
}
