//! The Starter's codex32 blank templates (`docs/CODEX32.md` §11,
//! `docs/STARTER.md` §3.8): what a person offered the Form row gets, and
//! that the worksheets print the booklet's tables.

use faraday_codec::qr::{Ecc, Payload, encode};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};
use faraday_starter::api::StarterCommand;
use faraday_starter::sheets as c32;
use faraday_starter::template::{CodexCopy, Form};
use faraday_starter::{Starter, StarterAction, TemplateChoice};

const TABLES: &str = include_str!("../../../../tools/vectors/codex32-booklet/tables.json");

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: the Starter's tests, which draw nothing"
)]
fn compute() -> faraday_crypto::work::Compute {
    faraday_crypto::work::Compute::new(|_| {})
}

fn starter() -> Starter {
    let mut s = Starter::new();
    s.event(Event::Display(DisplayInfo {
        width: 1280,
        height: 800,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    s
}

fn texts(s: &mut Starter) -> String {
    s.drawn_texts().join("\n")
}

fn commands(s: &mut Starter) -> Vec<StarterCommand> {
    std::iter::from_fn(|| s.next_command()).collect()
}

/// Chooses codex32 and the rest on the Blank templates page, as a person
/// taps through it.
fn choose(s: &mut Starter, chars: usize, copy: CodexCopy, worksheets: bool) {
    s.press(StarterAction::Templates);
    s.press(StarterAction::TemplateChoice(TemplateChoice::Form));
    s.press(StarterAction::TemplateForm(Form::Codex32));
    s.press(StarterAction::Back);
    s.press(StarterAction::TemplateChoice(TemplateChoice::Length));
    s.press(StarterAction::TemplateLength(chars));
    s.press(StarterAction::Back);
    s.press(StarterAction::TemplateChoice(TemplateChoice::Copy));
    s.press(StarterAction::TemplateCodexCopy(copy));
    s.press(StarterAction::Back);
    s.press(StarterAction::TemplateChoice(TemplateChoice::Worksheets));
    s.press(StarterAction::TemplateWorksheets(worksheets));
    s.press(StarterAction::Back);
}

/// The PDF the Save button makes, and its name.
fn saved(s: &mut Starter) -> (String, Vec<u8>) {
    s.press(StarterAction::SaveTemplate);
    let c = commands(s);
    let [StarterCommand::SaveTemplate { name, pdf }] = c.as_slice() else {
        panic!("one save expected: {c:?}");
    };
    (name.clone(), pdf.clone())
}

/// The pages of a PDF this writer made.
fn pages(pdf: &[u8]) -> usize {
    String::from_utf8_lossy(pdf)
        .matches("/Type /Page /Parent")
        .count()
}

/// Every string a PDF this writer made shows, in order.
fn shown(pdf: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 4 < pdf.len() {
        if &pdf[i..i + 4] == b"Td (" {
            i += 4;
            let mut cur = Vec::new();
            while i < pdf.len() {
                match pdf[i] {
                    b'\\' => {
                        i += 1;
                        cur.push(pdf[i]);
                    }
                    b')' => break,
                    c => cur.push(c),
                }
                i += 1;
            }
            out.push(cur);
        }
        i += 1;
    }
    out
}

fn text_of(pdf: &[u8]) -> String {
    shown(pdf)
        .iter()
        .map(|b| b.iter().map(|&c| char::from(c)).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------
// The booklet's tables, read from tables.json
// ---------------------------------------------------------------------

/// `"key": "value"` on a line, if it is one.
fn pair(line: &str) -> Option<(String, String)> {
    let t = line.trim().trim_end_matches(',');
    let (k, v) = t.split_once("\": \"")?;
    Some((
        k.trim_start_matches('"').to_string(),
        v.strip_suffix('"')?.to_string(),
    ))
}

/// A top-level table of tables.json: its columns and its rows.
fn table(name: &str) -> (String, Vec<(String, String)>) {
    let (mut inside, mut in_rows) = (false, false);
    let (mut columns, mut rows) = (String::new(), Vec::new());
    for line in TABLES.lines() {
        if line.starts_with(&format!(" \"{name}\": ")) {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        if !line.starts_with("  ") {
            break;
        }
        if line.trim() == "\"rows\": {" {
            in_rows = true;
        } else if in_rows && line.trim().starts_with('}') {
            in_rows = false;
        } else if let Some((k, v)) = pair(line) {
            if in_rows {
                rows.push((k, v));
            } else if k == "columns" {
                columns = v;
            }
        }
    }
    (columns, rows)
}

/// The derivation table of tables.json for `k` initial shares A, C, D, ...
/// (the listings from S, A, C, ... are the second group).
fn derivation(k: usize) -> (String, Vec<(String, String)>) {
    let (mut want, mut in_rows) = (false, false);
    let (mut columns, mut rows) = (String::new(), Vec::new());
    for line in TABLES.lines().skip_while(|l| !l.contains("\"derivation\"")) {
        let t = line.trim();
        if t.starts_with("\"k\": ") {
            want = t == format!("\"k\": {k},");
            (columns, rows) = (String::new(), Vec::new());
        } else if t == "\"rows\": {" {
            in_rows = true;
        } else if in_rows && t.starts_with('}') {
            in_rows = false;
        } else if t.starts_with("\"initial\": ") {
            if want && !t.contains('S') {
                return (columns, rows);
            }
        } else if let Some((key, v)) = pair(line) {
            if in_rows {
                rows.push((key, v));
            } else if key == "columns" {
                columns = v;
            }
        }
    }
    panic!("no derivation table for k = {k}");
}

#[test]
fn the_principal_tables_are_the_booklets() {
    let [addition, translation, recovery, fusion] = c32::principal_tables();
    for (name, t) in [
        ("addition", &addition),
        ("translation", &translation),
        ("recovery", &recovery),
        ("fusion", &fusion),
    ] {
        let (columns, rows) = table(name);
        assert!(!rows.is_empty(), "{name} not read");
        assert_eq!(t.column_text(), columns, "{name} columns");
        let got: Vec<(String, String)> = t
            .row_text()
            .into_iter()
            .map(|(h, r)| (h.to_string(), r))
            .collect();
        assert_eq!(got, rows, "{name} rows");
    }
}

#[test]
fn the_derivation_tables_are_the_booklets() {
    for k in 2..=8 {
        let (columns, rows) = derivation(k);
        let t = c32::derivation_table(k);
        assert_eq!(t.column_text(), columns, "k = {k} columns");
        let got: Vec<(String, String)> = t
            .row_text()
            .into_iter()
            .map(|(h, r)| (h.to_string(), r))
            .collect();
        assert_eq!(got, rows, "k = {k} rows");
    }
}

#[test]
fn the_checksum_table_is_the_booklets_and_is_printed_whole() {
    let (_, booklet) = table("checksum");
    // The checksum object has no "rows": its pairs are the rows.
    let booklet: Vec<(String, String)> = if booklet.is_empty() {
        TABLES
            .lines()
            .skip_while(|l| !l.starts_with(" \"checksum\": "))
            .skip(1)
            .take_while(|l| l.starts_with("  "))
            .filter_map(pair)
            .collect()
    } else {
        booklet
    };
    assert_eq!(booklet.len(), 1024);
    assert_eq!(c32::checksum_rows(), booklet);
    let mut s = starter();
    choose(&mut s, 48, CodexCopy::String, true);
    let (_, pdf) = saved(&mut s);
    let text = text_of(&pdf);
    for (key, row) in &booklet {
        assert!(text.contains(&format!("{key} {row}")), "{key} {row}");
    }
}

// ---------------------------------------------------------------------
// What the Starter offers and makes
// ---------------------------------------------------------------------

#[test]
fn the_form_row_offers_words_and_codex32_and_codex32_has_its_rows() {
    let mut s = starter();
    s.press(StarterAction::Templates);
    let page = texts(&mut s);
    assert!(
        page.contains("Form") && page.contains("BIP-39 words"),
        "{page}"
    );
    assert!(
        page.contains("Words") && !page.contains("Worksheets"),
        "{page}"
    );
    s.press(StarterAction::TemplateChoice(TemplateChoice::Form));
    let form = texts(&mut s);
    assert!(
        form.contains("BIP-39 words") && form.contains("codex32"),
        "{form}"
    );
    s.press(StarterAction::TemplateForm(Form::Codex32));
    s.press(StarterAction::Back);
    let page = texts(&mut s);
    for row in [
        "Form",
        "codex32",
        "Length",
        "48 characters",
        "Copy",
        "String",
    ] {
        assert!(page.contains(row), "{row} in {page}");
    }
    for row in ["Worksheets", "None", "Places"] {
        assert!(page.contains(row), "{row} in {page}");
    }
    assert!(!page.contains("SeedQR"), "{page}");
}

#[test]
fn the_codex32_choices_are_offered_where_they_exist() {
    let mut s = starter();
    choose(&mut s, 74, CodexCopy::String, true);
    s.press(StarterAction::TemplateChoice(TemplateChoice::Copy));
    let copy = texts(&mut s);
    assert!(copy.contains("String"), "{copy}");
    assert!(!copy.contains("Codex32QR"), "no grid at 74: {copy}");
    // A grid cannot be chosen at 74.
    s.press(StarterAction::TemplateCodexCopy(CodexCopy::Both));
    assert_eq!(s.template.codex_copy, CodexCopy::String);
    s.press(StarterAction::Back);
    // At 127 there are no worksheets: the choice goes back to none.
    s.press(StarterAction::TemplateChoice(TemplateChoice::Length));
    s.press(StarterAction::TemplateLength(127));
    assert!(!s.template.worksheets);
    s.press(StarterAction::Back);
    s.press(StarterAction::TemplateChoice(TemplateChoice::Worksheets));
    assert!(!texts(&mut s).contains("Dice, checksum"));
    s.press(StarterAction::Back);
    // At 48 the grid is offered.
    s.press(StarterAction::TemplateChoice(TemplateChoice::Length));
    s.press(StarterAction::TemplateLength(48));
    s.press(StarterAction::Back);
    s.press(StarterAction::TemplateChoice(TemplateChoice::Copy));
    assert!(texts(&mut s).contains("Codex32QR"));
}

#[test]
fn each_template_saves_under_its_name_with_the_pages_section_11_lists() {
    // (length, copy, worksheets, name, pages)
    let cases = [
        (48, CodexCopy::String, false, "48-chars-string", 2),
        (48, CodexCopy::Codex32Qr, false, "48-chars-codex32qr", 2),
        (48, CodexCopy::Both, false, "48-chars-string-codex32qr", 2),
        // card, dice twice, checksum, translation k = 2 and 3, tables,
        // checksum table twice, places.
        (
            48,
            CodexCopy::String,
            true,
            "48-chars-string-worksheets",
            10,
        ),
        // card, dice twice, checksum twice, tables, checksum table twice,
        // places.
        (74, CodexCopy::String, true, "74-chars-string-worksheets", 9),
        (74, CodexCopy::String, false, "74-chars-string", 2),
        (127, CodexCopy::String, false, "127-chars-string", 2),
    ];
    for (chars, copy, worksheets, name, count) in cases {
        let mut s = starter();
        choose(&mut s, chars, copy, worksheets);
        let (got, pdf) = saved(&mut s);
        assert_eq!(got, format!("faraday-blank-template-codex32-{name}.pdf"));
        assert!(pdf.starts_with(b"%PDF"));
        assert_eq!(pages(&pdf), count, "{name}");
        let sheet = s.template.sheet();
        assert!(sheet.contains(&format!("\nchars: {chars}\n")), "{sheet}");
        assert_eq!(
            pdf,
            c32::sheet(&compute(), &sheet).expect("a PDF"),
            "{name}"
        );
    }
}

#[test]
fn the_card_has_the_boxes_the_labels_the_fingerprint_and_the_alphabet() {
    let mut s = starter();
    choose(&mut s, 48, CodexCopy::Both, false);
    let (_, pdf) = saved(&mut s);
    let text = text_of(&pdf);
    for word in [
        "Codex32 share",
        "Threshold",
        "Identifier",
        "Share index",
        "Fingerprint",
        "Codex32QR",
        "QPZRY9X8GF2TVDW0S3JN54KHCE6MUA7L",
    ] {
        // The alphabet is one character a box, so look for its letters.
        if word.starts_with("QPZ") {
            let letters: Vec<String> = shown(&pdf)
                .iter()
                .filter(|b| b.len() == 1)
                .map(|b| char::from(b[0]).to_string())
                .collect();
            assert!(letters.join("").contains(word), "alphabet");
        } else {
            assert!(text.contains(word), "{word}");
        }
    }
    // ms1 in boxes 1 to 3 and the numbers 1 to 48.
    let singles: Vec<String> = shown(&pdf)
        .iter()
        .map(|b| String::from_utf8_lossy(b).into())
        .collect();
    for n in 1..=48 {
        assert!(singles.contains(&n.to_string()), "box number {n}");
    }
    assert!(!singles.contains(&"49".to_string()));
}

#[test]
fn the_worksheets_pages_are_landscape_and_use_the_symbol_font() {
    let mut s = starter();
    choose(&mut s, 48, CodexCopy::String, true);
    let (_, pdf) = saved(&mut s);
    let raw = String::from_utf8_lossy(&pdf);
    assert_eq!(raw.matches("/MediaBox [0 0 842 595]").count(), 5);
    assert!(raw.contains("/BaseFont /Symbol"));
    let text = text_of(&pdf);
    for word in [
        "Dice de-biasing worksheet",
        "Checksum worksheet",
        "Translation worksheet",
        "k = 3",
        "Derivation table",
        "SECRETSHARE32",
    ] {
        let whole = word == "SECRETSHARE32";
        if whole {
            let letters: String = shown(&pdf)
                .iter()
                .filter(|b| b.len() == 1)
                .map(|b| char::from(b[0]))
                .collect();
            assert!(letters.contains(word));
        } else {
            assert!(text.contains(word), "{word}");
        }
    }
}

#[test]
fn the_dice_worksheet_is_the_booklets_tracks_and_tree() {
    let mut s = starter();
    choose(&mut s, 48, CodexCopy::String, true);
    let (_, pdf) = saved(&mut s);
    let text = text_of(&pdf);
    for word in [
        "Dice de-biasing worksheet · page 1 of 2",
        "Dice de-biasing worksheet · page 2 of 2",
        "Die tracks",
        "Die pads",
        "Die pads (d7+)",
    ] {
        assert!(text.contains(word), "{word}");
    }
    // Five tracks, each with its free space and pads 1 to 20.
    assert_eq!(text.matches("Free space").count(), 5);
    let shown: Vec<String> = shown(&pdf)
        .iter()
        .map(|b| String::from_utf8_lossy(b).into())
        .collect();
    let dice = shown
        .iter()
        .position(|s| s.starts_with("Dice de-biasing"))
        .expect("the dice worksheet");
    // After the title and the tree's 32 leaves.
    let pads: Vec<&String> = shown[dice + 33..]
        .iter()
        .take_while(|s| !s.starts_with("Checksum worksheet"))
        .collect();
    for n in 1..=20 {
        let n = n.to_string();
        assert_eq!(pads.iter().filter(|s| ***s == n).count(), 5, "pad {n}");
    }
    // The tree's 32 leaves, left to right, in the booklet's order, under
    // the title and over the tracks' head.
    let leaves: String = shown[dice + 1..dice + 33].concat();
    assert_eq!(leaves, "QPZRY9X8GF2TVDW0S3JN54KHCE6MUA7L");
    assert_eq!(shown[dice + 33], "Die tracks");
}

#[test]
fn a_blank_template_holds_no_secret() {
    let mut s = starter();
    choose(&mut s, 48, CodexCopy::Both, true);
    let sheet = s.template.sheet();
    for field in [
        "descriptor",
        "key:",
        "address:",
        "name:",
        "shape:",
        "seed:",
        "ms1",
    ] {
        assert!(!sheet.contains(field), "{sheet}");
    }
    let (_, pdf) = saved(&mut s);
    let text = text_of(&pdf).to_lowercase();
    // No string of 48 characters or any run of the bech32 alphabet longer
    // than the alphabet itself.
    assert!(!text.contains("ms10"), "a codex32 string");
    for word in ["wallet", "passphrase", "recovery phrase"] {
        assert!(!text.contains(word), "{word}");
    }
    // Print makes the same bytes as Save.
    s.press(StarterAction::PrintTemplate);
    let c = commands(&mut s);
    let [StarterCommand::PrintTemplate { pdf: printed, .. }] = c.as_slice() else {
        panic!("{c:?}");
    };
    assert_eq!(*printed, pdf);
}

#[test]
fn the_codex32qr_grid_marks_the_squares_every_code_shares() {
    // A 48-character string in a version 3 code: the squares the card
    // prints grey are dark in it, and no other fixed square is.
    let string = "MS12NAMEA320ZYXWVUTSRQPNMLKJHGFEDCAA2467QGJTR2KP";
    let m = encode(
        &compute(),
        Payload::Alphanumeric(string.as_bytes()),
        Ecc::Medium,
    )
    .expect("a code");
    assert_eq!(m.size(), 29);
    let fixed = c32::codex32qr_fixed();
    for y in 0usize..29 {
        for x in 0usize..29 {
            if fixed.contains(&(x, y)) {
                assert!(m.module(x, y), "({x}, {y}) is dark in a code");
            }
        }
    }
    // Where the pattern is fixed, the grey squares are exactly the dark ones.
    for y in 0usize..29 {
        for x in 0usize..29 {
            let region = (x < 8 && y < 8)
                || (x >= 21 && y < 8)
                || (x < 8 && y >= 21)
                || ((8..21).contains(&x) && y == 6)
                || ((8..21).contains(&y) && x == 6)
                || (x.abs_diff(22) <= 2 && y.abs_diff(22) <= 2);
            if region {
                assert_eq!(m.module(x, y), fixed.contains(&(x, y)), "({x}, {y})");
            }
        }
    }
    // The three finders, the timing lines, the alignment square and the
    // dark module are all there.
    for (x, y) in [
        (0, 0),
        (28, 0),
        (0, 28),
        (8, 21),
        (22, 22),
        (10, 6),
        (6, 10),
    ] {
        assert!(fixed.contains(&(x, y)), "({x}, {y})");
    }
    assert!(!fixed.contains(&(9, 6)) && !fixed.contains(&(1, 1)));
}

#[test]
fn the_checksum_worksheets_ladder_is_the_booklets() {
    use faraday_bip::codex32::ChecksumWorksheet;
    for chars in [48, 74] {
        let w = ChecksumWorksheet::new(chars).expect("a length");
        assert_eq!(c32::ladder_steps(chars), w.steps());
        for j in 0..=w.steps() {
            assert_eq!(c32::sum_box(j), w.sum_box(j), "{chars} sum row {j}");
        }
        for j in 0..w.steps() {
            assert_eq!(
                c32::sum_box(j) + 2,
                w.lookup_box(j),
                "{chars} lookup row {j}"
            );
        }
        let hrp: Vec<u8> = w
            .hrp_row()
            .iter()
            .map(|&v| b"QPZRY9X8GF2TVDW0S3JN54KHCE6MUA7L"[usize::from(v)])
            .collect();
        assert_eq!(hrp, c32::HRP_RESIDUE);
        let target: Vec<u8> = w
            .target()
            .iter()
            .map(|&v| b"QPZRY9X8GF2TVDW0S3JN54KHCE6MUA7L"[usize::from(v)])
            .collect();
        assert_eq!(target, c32::SECRET_SHARE);
    }
}
