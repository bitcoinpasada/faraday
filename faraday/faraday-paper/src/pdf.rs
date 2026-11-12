//! A small PDF writer for Faraday's printed sheets, and the two sheets.
//!
//! The pages use PDF's standard fonts (Helvetica and Courier), so nothing
//! is embedded, and draw QR codes and grids as filled squares. The sheets
//! go to the Outbox as PDFs on every shell; the device itself prints
//! nothing. Neither sheet carries a secret (`faraday-core`'s `wallet_backup`).

use faraday_codec::qr::{Ecc, Payload, QrMatrix, encode};
use faraday_crypto::work::Compute;
use faraday_draw::tokens::{
    MOVE_TABLE_COLS, MOVE_TABLE_INSET, MOVE_TABLE_ROW, MOVE_TABLE_ROWS, MOVE_TABLE_RULE_GAP,
    PATHS_END, PATHS_FIELD_ROW, PATHS_FIELDS_GAP, PATHS_GAP, PATHS_HEAD_GAP, PATHS_LINE,
    PATHS_PARA_GAP, PATHS_RULE_DROP, PATHS_TITLE_GAP, PATHS_WAIT_X, SHARE_LINES_GAP,
    SHARE_QUORUM_GAP, SHEET_FOOT_SIZE, SHEET_LABEL_SIZE, SHEET_RULE_GREY, SHEET_RULE_SIZE,
    SHEET_TEXT_SIZE, SHEET_TITLE_SIZE,
};
use faraday_strings::{EN, fill};

thread_local! {
    static PICTURES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// The documents written on this thread so far: the app's count of
/// costly work reads it as the pictures made.
pub fn documents_counted() -> u64 {
    PICTURES.with(std::cell::Cell::get)
}

/// The squares fixed for every SeedQR of side `n`: finders and their
/// separators, timing and the alignment square. The format areas are left
/// out: they depend on the mask the seed's code was given.
pub fn structural_fixed(x: usize, y: usize, n: usize) -> bool {
    let finder = (x < 8 && y < 8) || (x >= n - 8 && y < 8) || (x < 8 && y >= n - 8);
    // The format bits go round the sixth row and column, which stay timing.
    let format = ((y == 8 && (x < 9 || x >= n - 8)) || (x == 8 && (y < 9 || y >= n - 8)))
        && x != 6
        && y != 6;
    let version = (n - 17) / 4;
    let align = version >= 2 && {
        let c = n - 7;
        x.abs_diff(c) <= 2 && y.abs_diff(c) <= 2
    };
    (finder && !format) || ((x == 6 || y == 6) && !format) || align
}

/// Whether a fixed square of a code of side `n` ([`structural_fixed`]) is
/// dark, as ISO/IEC 18004 draws it: a finder is a dark ring of 7 by 7, a
/// light ring, a dark 3 by 3 centre, with a light separator beside it;
/// timing alternates from dark at the sixth square; the alignment square
/// is a dark ring of 5 by 5 round a dark centre with a light ring between.
/// It depends on the side alone. A square that is not fixed is `false`.
pub fn structural_dark(x: usize, y: usize, n: usize) -> bool {
    if !structural_fixed(x, y, n) {
        return false;
    }
    // Distance from the code's nearest edges, which is the place in a finder.
    let (fx, fy) = (x.min(n - 1 - x), y.min(n - 1 - y));
    let in_finder = (x < 8 && y < 8) || (x >= n - 8 && y < 8) || (x < 8 && y >= n - 8);
    if in_finder {
        // Its separator is the eighth row and column from the corner.
        if fx > 6 || fy > 6 {
            return false;
        }
        return fx.abs_diff(3).max(fy.abs_diff(3)) != 2;
    }
    let c = n - 7;
    if (n - 17) / 4 >= 2 && x.abs_diff(c) <= 2 && y.abs_diff(c) <= 2 {
        return x.abs_diff(c).max(y.abs_diff(c)) != 1;
    }
    // Timing: the row or column 6.
    if y == 6 {
        x.is_multiple_of(2)
    } else {
        y.is_multiple_of(2)
    }
}

/// A4, in points.
const PAGE_W: f32 = 595.0;
const PAGE_H: f32 = 842.0;
const MARGIN: f32 = 48.0;

/// The most places a blank template's places page holds: a row each, 56
/// pt apart from 40 pt under its heading, the last row's second line (34
/// pt down) above the bottom margin. The Starter's count goes no higher.
pub const PLACES_MAX: usize = 13;

/// One page's drawing operators, with the origin at the top left and y
/// going down, as the screens draw.
#[derive(Default)]
struct Page {
    ops: Vec<u8>,
    /// A page not A4 portrait: its width and height in points
    /// ([`RawPage`]).
    media: Option<(f32, f32)>,
}

#[derive(Clone, Copy)]
enum Font {
    Sans,
    Bold,
    Mono,
}

impl Font {
    fn name(self) -> &'static str {
        match self {
            Font::Sans => "F1",
            Font::Bold => "F2",
            Font::Mono => "F3",
        }
    }
}

/// Text in PDF's WinAnsi encoding, escaped for a string literal.
pub fn pdf_text(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for c in s.chars() {
        let b = match c {
            '(' | ')' | '\\' => {
                out.push(b'\\');
                c as u8
            }
            '·' => 0xB7,
            '©' => 0xA9,
            '…' => 0x85,
            '–' => 0x96,
            '—' => 0x97,
            '×' => 0xD7,
            // WinAnsi has no arrow; a menu path reads as well with ›.
            '→' => 0x9B,
            c if (c as u32) < 0x80 => c as u8,
            _ => b'?',
        };
        out.push(b);
    }
    out
}

impl Page {
    fn op(&mut self, s: &str) {
        self.ops.extend_from_slice(s.as_bytes());
        self.ops.push(b'\n');
    }

    fn text(&mut self, x: f32, y: f32, size: f32, font: Font, s: &str) {
        self.ops.extend_from_slice(
            format!(
                "BT /{} {size} Tf {x:.2} {:.2} Td (",
                font.name(),
                PAGE_H - y - size
            )
            .as_bytes(),
        );
        self.ops.extend_from_slice(&pdf_text(s));
        self.ops.extend_from_slice(b") Tj ET\n");
    }

    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, grey: f32) {
        self.op(&format!(
            "{grey:.2} g {x:.2} {:.2} {w:.2} {h:.2} re f",
            PAGE_H - y - h
        ));
    }

    fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, width: f32, grey: f32) {
        self.op(&format!(
            "{grey:.2} G {width:.2} w {x1:.2} {:.2} m {x2:.2} {:.2} l S",
            PAGE_H - y1,
            PAGE_H - y2
        ));
    }

    /// Text wrapped to `width`, Courier being 0.6 em per character and
    /// Helvetica taken as 0.5. Returns the height used.
    fn wrapped(&mut self, x: f32, y: f32, width: f32, size: f32, font: Font, s: &str) -> f32 {
        let per = match font {
            Font::Mono => size * 0.6,
            _ => size * 0.5,
        };
        let max = ((width / per).floor() as usize).max(8);
        let mut lines: Vec<String> = Vec::new();
        if matches!(font, Font::Mono) {
            let chars: Vec<char> = s.chars().collect();
            for chunk in chars.chunks(max) {
                lines.push(chunk.iter().collect());
            }
        } else {
            let mut line = String::new();
            for word in s.split_whitespace() {
                if !line.is_empty() && line.len() + 1 + word.len() > max {
                    lines.push(std::mem::take(&mut line));
                }
                if !line.is_empty() {
                    line.push(' ');
                }
                line.push_str(word);
            }
            if !line.is_empty() {
                lines.push(line);
            }
        }
        let lh = size * 1.35;
        for (i, l) in lines.iter().enumerate() {
            self.text(x, y + i as f32 * lh, size, font, l);
        }
        lines.len() as f32 * lh
    }

    fn qr(&mut self, m: &QrMatrix, x: f32, y: f32, side: f32) {
        let n = m.size() as f32;
        let cell = side / n;
        for row in 0..m.size() {
            for col in 0..m.size() {
                if m.module(col, row) {
                    // A hair over a cell, so neighbouring squares leave no seam.
                    self.rect(
                        x + col as f32 * cell,
                        y + row as f32 * cell,
                        cell + 0.05,
                        cell + 0.05,
                        0.0,
                    );
                }
            }
        }
    }
}

/// Costs the document written out, its pages compressed: the finished
/// document.
fn document(_c: &Compute, pages: Vec<Page>) -> Vec<u8> {
    PICTURES.with(|c| c.set(c.get().wrapping_add(1)));
    let mut out: Vec<u8> = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets: Vec<usize> = Vec::new();
    let n = pages.len();
    // 1 catalog, 2 pages, 3..5 fonts, then a page and its contents each.
    let page_ids: Vec<usize> = (0..n).map(|i| 6 + 2 * i).collect();
    let obj = |out: &mut Vec<u8>, offsets: &mut Vec<usize>, body: &[u8]| {
        offsets.push(out.len());
        let id = offsets.len();
        out.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    };
    obj(&mut out, &mut offsets, b"<< /Type /Catalog /Pages 2 0 R >>");
    let kids: Vec<String> = page_ids.iter().map(|i| format!("{i} 0 R")).collect();
    obj(
        &mut out,
        &mut offsets,
        format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" ")).as_bytes(),
    );
    for base in ["/Helvetica", "/Helvetica-Bold", "/Courier"] {
        obj(
            &mut out,
            &mut offsets,
            format!(
                "<< /Type /Font /Subtype /Type1 /BaseFont {base} /Encoding /WinAnsiEncoding >>"
            )
            .as_bytes(),
        );
    }
    // The Symbol font, after the pages: the codex32 sheets' second
    // alphabet (Greek letters, aleph, the circled plus, diamond, heart).
    let symbol_id = 6 + 2 * n;
    for (i, page) in pages.iter().enumerate() {
        let contents = page_ids[i] + 1;
        let (w, h) = page.media.unwrap_or((PAGE_W, PAGE_H));
        obj(
            &mut out,
            &mut offsets,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w} {h}] \
                 /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R /F4 {symbol_id} 0 R >> >> /Contents {contents} 0 R >>"
            )
            .as_bytes(),
        );
        let mut body = format!("<< /Length {} >>\nstream\n", page.ops.len()).into_bytes();
        body.extend_from_slice(&page.ops);
        body.extend_from_slice(b"endstream");
        obj(&mut out, &mut offsets, &body);
    }
    obj(
        &mut out,
        &mut offsets,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Symbol >>",
    );
    let xref = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes(),
    );
    for o in &offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            offsets.len() + 1
        )
        .as_bytes(),
    );
    out
}

/// A page drawn by another crate, as PDF operators with the origin at
/// the bottom left, and its size in points where it is not A4 portrait.
/// Fonts: `/F1` Helvetica, `/F2` Helvetica-Bold, `/F3` Courier, `/F4`
/// Symbol. The Starter's codex32 templates are drawn this way.
pub struct RawPage {
    /// The page's operators.
    pub ops: Vec<u8>,
    /// Width and height in points, or A4 portrait.
    pub size: Option<(f32, f32)>,
}

/// Costs the document written out, as `document` does. The PDF of pages
/// drawn elsewhere ([`RawPage`]); holds no information beyond its pages.
pub fn raw_document(c: &Compute, pages: Vec<RawPage>) -> Vec<u8> {
    let pages = pages
        .into_iter()
        .map(|p| Page {
            ops: p.ops,
            media: p.size,
        })
        .collect();
    document(c, pages)
}

/// Costs a picture drawn (through `document`). A notes sheet
/// (`docs/STARTER.md` §3.4, F47): what a person takes to a computer's
/// BIOS, on one page. `rows` are labels and values, a value not known
/// printed as a line to write on; `steps` are numbered, each with its
/// lines under it. Holds nothing secret, and the document carries no
/// information beyond its page (no title, author, producer or dates).
pub fn notes(
    c: &Compute,
    title: &str,
    rows: &[(String, Option<String>)],
    steps: &[(String, Vec<String>)],
) -> Vec<u8> {
    let mut p = Page::default();
    let width = PAGE_W - 2.0 * MARGIN;
    let mut y = MARGIN;
    y += p.wrapped(MARGIN, y, width, 20.0, Font::Bold, title) + 18.0;
    let value_x = MARGIN + 190.0;
    let value_w = PAGE_W - MARGIN - value_x;
    for (label, value) in rows {
        let lh = p.wrapped(
            MARGIN,
            y + 3.0,
            value_x - MARGIN - 12.0,
            11.0,
            Font::Sans,
            label,
        );
        let vh = match value {
            Some(v) => p.wrapped(value_x, y, value_w, 14.0, Font::Bold, v),
            None => {
                p.line(value_x, y + 18.0, PAGE_W - MARGIN, y + 18.0, 0.5, 0.4);
                20.0
            }
        };
        y += lh.max(vh) + 14.0;
    }
    y += 12.0;
    for (n, (head, lines)) in steps.iter().enumerate() {
        p.text(MARGIN, y, 13.0, Font::Bold, &format!("{}.", n + 1));
        y += p.wrapped(MARGIN + 22.0, y, width - 22.0, 13.0, Font::Bold, head) + 4.0;
        for l in lines {
            y += p.wrapped(MARGIN + 22.0, y, width - 22.0, 11.0, Font::Sans, l) + 2.0;
        }
        y += 14.0;
    }
    document(c, vec![p])
}

/// The fields of a sheet that may appear more than once, one line each:
/// a share's lines, a wallet's keys and addresses, the envelopes' places
/// and what each holds, a wallet's spend paths and their waits. Every other field is read once ([`get`]).
const REPEATED: [&str; 7] = ["line", "key", "address", "place", "thing", "path", "wait"];

/// The fields of a sheet file, in order. Refused when a field other than
/// the [`REPEATED`] ones appears twice: which of the two the sheet means
/// cannot be told.
fn fields(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some("faraday-sheet 1") {
        return Err(EN.pdf_not_a_faraday_sheet.into());
    }
    let f: Vec<(String, String)> = lines
        .filter_map(|l| {
            l.split_once(':')
                .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        })
        .collect();
    for (i, (k, _)) in f.iter().enumerate() {
        if !REPEATED.contains(&k.as_str()) && f[..i].iter().any(|(e, _)| e == k) {
            return Err(fill(EN.pdf_sheet_names_twice, &[k]));
        }
    }
    Ok(f)
}

fn get<'a>(f: &'a [(String, String)], key: &str) -> Option<&'a str> {
    f.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

/// Costs a QR encoding and a picture drawn. The PDF of a sheet file.
pub fn sheet(c: &Compute, text: &str) -> Result<Vec<u8>, String> {
    let f = fields(text)?;
    match get(&f, "kind") {
        Some("wallet") => Ok(wallet_sheet(c, &f)),
        Some("blank") => blank_sheet(c, &f),
        Some("share") => Ok(share_sheet(c, &f)),
        Some("envelopes") => Ok(envelopes_sheet(c, &f)),
        other => Err(fill(EN.pdf_a_sheet_of_kind, &[other.unwrap_or_default()])),
    }
}

/// Costs a QR encoding and a picture drawn (through `encode` and
/// `document`).
fn wallet_sheet(c: &Compute, f: &[(String, String)]) -> Vec<u8> {
    let mut p = Page::default();
    let name = get(f, "name").unwrap_or(EN.pdf_wallet);
    let mut y = MARGIN;
    p.text(
        MARGIN,
        y,
        20.0,
        Font::Bold,
        &fill(EN.pdf_wallet_backup, &[name]),
    );
    y += 30.0;
    p.text(MARGIN, y, 11.0, Font::Sans, get(f, "shape").unwrap_or(""));
    y += 18.0;
    p.text(MARGIN, y, 10.0, Font::Sans, EN.pdf_xpubs_only_note);
    y += 28.0;
    let descriptor = get(f, "descriptor").unwrap_or("");
    let qr_side = 190.0;
    if let Ok(m) = encode(c, Payload::Bytes(descriptor.as_bytes()), Ecc::Medium) {
        p.qr(&m, PAGE_W - MARGIN - qr_side, y, qr_side);
    }
    p.text(MARGIN, y, 10.0, Font::Bold, EN.pdf_descriptor);
    y += 16.0;
    let used = p.wrapped(
        MARGIN,
        y,
        PAGE_W - 2.0 * MARGIN - qr_side - 16.0,
        8.0,
        Font::Mono,
        descriptor,
    );
    y = (y + used).max(y + qr_side - 16.0) + 24.0;
    p.text(MARGIN, y, 10.0, Font::Bold, EN.pdf_xpubs);
    y += 16.0;
    for (k, v) in f.iter().filter(|(k, _)| k == "key") {
        let _ = k;
        y += p.wrapped(MARGIN, y, PAGE_W - 2.0 * MARGIN, 8.0, Font::Mono, v) + 6.0;
    }
    y += 14.0;
    p.text(MARGIN, y, 10.0, Font::Bold, EN.pdf_first_addresses);
    y += 16.0;
    for (_, v) in f.iter().filter(|(k, _)| k == "address") {
        p.text(MARGIN, y, 8.5, Font::Mono, v);
        y += 13.0;
    }
    y += 30.0;
    for label in [
        EN.pdf_written_on_label,
        EN.pdf_kept_at_label,
        EN.pdf_goes_with_seed,
    ] {
        p.text(MARGIN, y, 10.0, Font::Sans, label);
        p.line(
            MARGIN + 130.0,
            y + 11.0,
            PAGE_W - MARGIN,
            y + 11.0,
            0.5,
            0.4,
        );
        y += 26.0;
    }
    p.text(
        MARGIN,
        PAGE_H - MARGIN,
        8.0,
        Font::Sans,
        EN.pdf_made_by_faraday_check,
    );
    // The spend paths and the table of deposits stand on a page after the
    // xpubs, where there is room for the table.
    let mut q = Page::default();
    if spend_paths_block(&mut q, f, MARGIN) > 0.0 {
        q.text(
            MARGIN,
            PAGE_H - MARGIN,
            SHEET_FOOT_SIZE,
            Font::Sans,
            EN.pdf_made_by_faraday_check,
        );
        return document(c, vec![p, q]);
    }
    document(c, vec![p])
}

/// The **Spend paths** block of a wallet sheet (`docs/BACKUP-FLOW.md`
/// §18.4), drawn from `y`: per path its keys and wait; then, where a path
/// has a relative wait, the line about coins left unmoved and the table
/// to fill in by hand. Nothing for a sheet file without `path` fields.
/// Returns its height.
fn spend_paths_block(p: &mut Page, f: &[(String, String)], y0: f32) -> f32 {
    let mut y = y0;
    let paths: Vec<&str> = f
        .iter()
        .filter(|(k, _)| k == "path")
        .map(|(_, v)| v.as_str())
        .collect();
    if paths.is_empty() {
        return 0.0;
    }
    p.text(MARGIN, y, SHEET_LABEL_SIZE, Font::Bold, EN.pdf_spend_paths);
    y += PATHS_HEAD_GAP;
    let waits = f
        .iter()
        .filter(|(k, _)| k == "wait")
        .map(|(_, v)| v.as_str());
    for (keys, wait) in paths.iter().zip(waits) {
        p.text(MARGIN, y, SHEET_TEXT_SIZE, Font::Sans, keys);
        p.text(MARGIN + PATHS_WAIT_X, y, SHEET_TEXT_SIZE, Font::Sans, wait);
        y += PATHS_LINE;
    }
    y += PATHS_GAP;
    if let Some(u) = get(f, "unmoved") {
        let mut n = u.split_whitespace();
        let (days, blocks, margin) = (n.next(), n.next(), n.next());
        let keys = get(f, "alone").unwrap_or("");
        let line = match (days, blocks) {
            (Some(d), Some("-")) => fill(EN.pdf_unmoved_days, &[d, keys]),
            (Some(d), Some(b)) => fill(EN.pdf_unmoved_blocks, &[d, b, keys]),
            _ => String::new(),
        };
        y += p.wrapped(
            MARGIN,
            y,
            PAGE_W - 2.0 * MARGIN,
            SHEET_TEXT_SIZE,
            Font::Sans,
            &line,
        ) + PATHS_PARA_GAP;
        y += move_table(p, y, &fill(EN.pdf_move_before, &[margin.unwrap_or("")]));
    }
    y - y0 + PATHS_END
}

/// The table to fill in by hand, one row per deposit: when it was
/// received, its amount, the date to move it before. `rule` stands over
/// it. Returns its height.
fn move_table(p: &mut Page, y0: f32, rule: &str) -> f32 {
    let mut y = y0;
    p.text(MARGIN, y, SHEET_RULE_SIZE, Font::Sans, rule);
    y += PATHS_LINE;
    let w = (PAGE_W - 2.0 * MARGIN) / MOVE_TABLE_COLS as f32;
    for (i, head) in [EN.pdf_received_on, EN.pdf_amount, EN.pdf_move_before_col]
        .iter()
        .enumerate()
    {
        p.text(
            MARGIN + i as f32 * w + MOVE_TABLE_INSET,
            y,
            SHEET_TEXT_SIZE,
            Font::Bold,
            head,
        );
    }
    y += PATHS_LINE;
    for _ in 0..MOVE_TABLE_ROWS {
        for i in 0..MOVE_TABLE_COLS {
            let x = MARGIN + i as f32 * w;
            p.line(
                x,
                y + PATHS_LINE,
                x + w - MOVE_TABLE_RULE_GAP,
                y + PATHS_LINE,
                0.5,
                SHEET_RULE_GREY,
            );
        }
        y += MOVE_TABLE_ROW;
    }
    y - y0
}

/// Costs a picture drawn (through `document`). What goes in each
/// place's envelope: a heading per place, its things under it, a line
/// to tick each.
fn envelopes_sheet(c: &Compute, f: &[(String, String)]) -> Vec<u8> {
    let mut p = Page::default();
    let name = get(f, "name").unwrap_or(EN.pdf_wallet);
    let mut y = MARGIN;
    p.text(
        MARGIN,
        y,
        20.0,
        Font::Bold,
        &fill(EN.pdf_envelopes, &[name]),
    );
    y += 30.0;
    p.text(MARGIN, y, 11.0, Font::Sans, get(f, "shape").unwrap_or(""));
    y += 30.0;
    for (k, v) in f {
        match k.as_str() {
            "place" => {
                y += 8.0;
                p.text(MARGIN, y, 12.0, Font::Bold, v);
                y += 20.0;
            }
            "thing" => {
                p.line(MARGIN, y + 9.0, MARGIN + 9.0, y + 9.0, 0.5, 0.4);
                p.text(MARGIN + 18.0, y, 10.0, Font::Sans, v);
                y += 16.0;
            }
            _ => {}
        }
    }
    document(c, vec![p])
}

/// Costs a QR encoding and a picture drawn (through `encode` and
/// `document`). One sheet of a split backup: what it is, how many
/// rebuild the wallet, and its lines as text and as a QR code to scan.
fn share_sheet(c: &Compute, f: &[(String, String)]) -> Vec<u8> {
    let mut p = Page::default();
    let name = get(f, "name").unwrap_or(EN.pdf_wallet);
    let part = get(f, "part").unwrap_or("");
    let quorum = get(f, "quorum").unwrap_or("");
    let mut y = MARGIN;
    p.text(
        MARGIN,
        y,
        20.0,
        Font::Bold,
        &fill(EN.pdf_split_backup_sheet, &[name, part]),
    );
    y += 30.0;
    p.text(MARGIN, y, 11.0, Font::Sans, get(f, "shape").unwrap_or(""));
    y += 18.0;
    p.text(
        MARGIN,
        y,
        10.0,
        Font::Sans,
        &fill(EN.pdf_part_of_the_wallet, &[quorum]),
    );
    y += SHARE_QUORUM_GAP;
    if get(f, "recovery") == Some("yes") {
        p.text(
            MARGIN,
            y,
            SHEET_LABEL_SIZE,
            Font::Sans,
            EN.pdf_has_recovery_path,
        );
        y += SHARE_QUORUM_GAP;
    }
    y += SHARE_LINES_GAP;
    let lines: Vec<&str> = f
        .iter()
        .filter(|(k, _)| k == "line")
        .map(|(_, v)| v.as_str())
        .collect();
    let text = lines.join("\n");
    let qr_side = 210.0;
    if let Ok(m) = encode(c, Payload::Bytes(text.as_bytes()), Ecc::Medium) {
        p.qr(&m, PAGE_W - MARGIN - qr_side, y, qr_side);
    }
    for line in &lines {
        y += p.wrapped(
            MARGIN,
            y,
            PAGE_W - 2.0 * MARGIN - qr_side - 16.0,
            8.0,
            Font::Mono,
            line,
        ) + 4.0;
    }
    p.text(
        MARGIN,
        PAGE_H - MARGIN,
        8.0,
        Font::Sans,
        EN.pdf_made_by_faraday,
    );
    document(c, vec![p])
}

/// The Starter's blank **Spend paths** block (`docs/STARTER.md` §3.8): a
/// page of three paths with their keys and wait to fill in, the line
/// about coins left unmoved with its numbers blank, and the table of
/// deposits.
fn blank_paths_page() -> Page {
    let mut p = Page::default();
    let mut y = MARGIN;
    p.text(MARGIN, y, SHEET_TITLE_SIZE, Font::Bold, EN.pdf_spend_paths);
    y += PATHS_TITLE_GAP;
    p.text(
        MARGIN,
        y,
        SHEET_TEXT_SIZE,
        Font::Sans,
        EN.pdf_path_keys_wait,
    );
    y += PATHS_FIELDS_GAP;
    for _ in 0..3 {
        p.line(
            MARGIN,
            y + PATHS_RULE_DROP,
            PAGE_W - MARGIN,
            y + PATHS_RULE_DROP,
            0.5,
            SHEET_RULE_GREY,
        );
        y += PATHS_FIELD_ROW;
    }
    y += PATHS_RULE_DROP;
    y += p.wrapped(
        MARGIN,
        y,
        PAGE_W - 2.0 * MARGIN,
        SHEET_TEXT_SIZE,
        Font::Sans,
        EN.pdf_unmoved_blank,
    ) + PATHS_LINE;
    move_table(&mut p, y, EN.pdf_move_before_blank);
    p.text(
        MARGIN,
        PAGE_H - MARGIN + PATHS_LINE,
        SHEET_FOOT_SIZE,
        Font::Sans,
        EN.pdf_print_actual_size,
    );
    p
}

/// Costs a picture drawn (through `document`).
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn blank_sheet(c: &Compute, f: &[(String, String)]) -> Result<Vec<u8>, String> {
    let words: usize = get(f, "words").and_then(|w| w.parse().ok()).unwrap_or(24);
    if words != 12 && words != 24 {
        return Err(EN.pdf_template_word_count.into());
    }
    // What is copied onto the sheet: a file without a `copy:` line, or
    // with none of its words, reads as both.
    let copy = get(f, "copy").unwrap_or("");
    let (mut show_words, mut show_grid) = (
        copy.split_whitespace().any(|c| c == "words"),
        copy.split_whitespace().any(|c| c == "seedqr"),
    );
    if !show_words && !show_grid {
        (show_words, show_grid) = (true, true);
    }
    let mut p = Page::default();
    let mut y = MARGIN;
    p.text(
        MARGIN,
        y,
        18.0,
        Font::Bold,
        &fill(EN.pdf_seed_sheet_words, &[&words.to_string()]),
    );
    y += 26.0;
    if let Some(name) = get(f, "name") {
        p.text(
            MARGIN,
            y,
            10.0,
            Font::Sans,
            &format!("{name} · {}", get(f, "shape").unwrap_or("")),
        );
        y += 16.0;
    }
    p.text(MARGIN, y, 9.0, Font::Sans, EN.pdf_written_by_hand);
    y += 26.0;
    // The fields to fill after a test restore.
    let network = fill(EN.pdf_network, &[get(f, "network").unwrap_or("mainnet")]);
    for (i, label) in [EN.pdf_fingerprint, EN.pdf_first_address, network.as_str()]
        .iter()
        .enumerate()
    {
        let x = MARGIN + i as f32 * 170.0;
        p.text(x, y, 9.0, Font::Sans, label);
        if i < 2 {
            p.line(x, y + 26.0, x + 150.0, y + 26.0, 0.5, 0.4);
        }
    }
    y += 44.0;
    // Word lines, in two columns.
    let rows = words / 2;
    let colw = (PAGE_W - 2.0 * MARGIN) / 2.0;
    for i in (0..words).filter(|_| show_words) {
        let x = MARGIN + (i / rows) as f32 * colw;
        let ly = y + (i % rows) as f32 * 22.0;
        p.text(x, ly, 9.0, Font::Sans, &format!("{:>2}", i + 1));
        p.line(x + 20.0, ly + 11.0, x + colw - 30.0, ly + 11.0, 0.5, 0.55);
    }
    if show_words {
        y += rows as f32 * 22.0 + 18.0;
    }
    // The SeedQR grid: its fixed squares printed black, the rest ruled
    // for filling in by hand.
    let n = if words == 24 { 29 } else { 25 };
    let side = (PAGE_H - MARGIN - 40.0 - y)
        .min(PAGE_W - 2.0 * MARGIN - 40.0)
        .min(380.0);
    let cell = side / n as f32;
    let gx = MARGIN + 20.0;
    let gy = y + 14.0;
    let drawn = if show_grid { n } else { 0 };
    for row in 0..drawn {
        for col in 0..drawn {
            if structural_dark(col, row, n) {
                p.rect(
                    gx + col as f32 * cell,
                    gy + row as f32 * cell,
                    cell,
                    cell,
                    0.0,
                );
            }
        }
    }
    for i in (0..=n).filter(|_| show_grid) {
        let (width, grey) = if i % 5 == 0 { (0.6, 0.45) } else { (0.3, 0.75) };
        let o = i as f32 * cell;
        p.line(gx + o, gy, gx + o, gy + side, width, grey);
        p.line(gx, gy + o, gx + side, gy + o, width, grey);
        if i % 5 == 0 && i < n {
            p.text(gx + o + 1.0, gy - 11.0, 6.5, Font::Mono, &i.to_string());
            p.text(MARGIN, gy + o + 1.0, 6.5, Font::Mono, &i.to_string());
        }
    }
    p.text(
        MARGIN,
        PAGE_H - MARGIN,
        8.0,
        Font::Sans,
        EN.pdf_there_is_no_line,
    );
    p.text(
        MARGIN,
        PAGE_H - MARGIN + 14.0,
        8.0,
        Font::Sans,
        EN.pdf_print_actual_size,
    );
    // The places, to fill in by hand: a line each, nothing printed on it.
    let places: usize = get(f, "places")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1)
        .clamp(1, PLACES_MAX);
    let mut q = Page::default();
    let mut y = MARGIN;
    q.text(MARGIN, y, 18.0, Font::Bold, EN.pdf_places);
    q.text(
        MARGIN,
        PAGE_H - MARGIN + 14.0,
        8.0,
        Font::Sans,
        EN.pdf_print_actual_size,
    );
    y += 40.0;
    let right = PAGE_W - MARGIN;
    for _ in 0..places {
        q.text(MARGIN, y, 11.0, Font::Sans, EN.pdf_place);
        q.line(MARGIN + 36.0, y + 12.0, MARGIN + 196.0, y + 12.0, 0.5, 0.55);
        q.text(MARGIN + 204.0, y, 11.0, Font::Sans, EN.pdf_holds);
        q.line(MARGIN + 238.0, y + 12.0, right, y + 12.0, 0.5, 0.55);
        q.line(MARGIN + 238.0, y + 34.0, right, y + 34.0, 0.5, 0.55);
        y += 56.0;
    }
    let mut pages = vec![p, q];
    if get(f, "paths") == Some("blank") {
        pages.push(blank_paths_page());
    }
    Ok(document(c, pages))
}
