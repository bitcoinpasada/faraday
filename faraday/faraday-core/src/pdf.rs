//! A small PDF writer for Faraday's printed sheets, and the two sheets.
//!
//! The pages use PDF's standard fonts (Helvetica and Courier), so nothing
//! is embedded, and draw QR codes and grids as filled squares. The sheets
//! go to the Outbox as PDFs on every shell; the device itself prints
//! nothing. Neither sheet carries a secret ([`crate::wallet_backup`]).

use crate::strings::EN;
use crate::strings::fill;
use faraday_codec::qr::{Ecc, Payload, QrMatrix, encode};
use faraday_crypto::work::Compute;

use crate::wallet_backup::structural_fixed;

/// A4, in points.
const PAGE_W: f32 = 595.0;
const PAGE_H: f32 = 842.0;
const MARGIN: f32 = 48.0;

/// One page's drawing operators, with the origin at the top left and y
/// going down, as the screens draw.
#[derive(Default)]
struct Page {
    ops: Vec<u8>,
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
fn pdf_text(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for c in s.chars() {
        let b = match c {
            '(' | ')' | '\\' => {
                out.push(b'\\');
                c as u8
            }
            '·' => 0xB7,
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
    crate::work::count(crate::work::Work::Picture);
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
    for (i, page) in pages.iter().enumerate() {
        let contents = page_ids[i] + 1;
        obj(
            &mut out,
            &mut offsets,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_W} {PAGE_H}] \
                 /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R >> >> /Contents {contents} 0 R >>"
            )
            .as_bytes(),
        );
        let mut body = format!("<< /Length {} >>\nstream\n", page.ops.len()).into_bytes();
        body.extend_from_slice(&page.ops);
        body.extend_from_slice(b"endstream");
        obj(&mut out, &mut offsets, &body);
    }
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

/// The fields of a sheet file, in order.
fn fields(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some("faraday-sheet 1") {
        return Err(EN.pdf_not_a_faraday_sheet.into());
    }
    Ok(lines
        .filter_map(|l| {
            l.split_once(':')
                .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        })
        .collect())
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
        other => Err(fill(EN.pdf_a_sheet_of_kind, &[&format!("{other:?}")])),
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
    p.text(MARGIN, y, 10.0, Font::Sans, EN.pdf_xpubs_only_this);
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
    for label in [EN.pdf_written_on, EN.pdf_kept_at, EN.pdf_goes_with_seed] {
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
    document(c, vec![p])
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
    y += 28.0;
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

/// Costs a QR encoding and a picture drawn (through `probe_matrix` and
/// `document`).
fn blank_sheet(c: &Compute, f: &[(String, String)]) -> Result<Vec<u8>, String> {
    let words: usize = get(f, "words").and_then(|w| w.parse().ok()).unwrap_or(24);
    if words != 12 && words != 24 {
        return Err(EN.pdf_the_template_is.into());
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
    let probe = probe_matrix(c, words)?;
    let side = (PAGE_H - MARGIN - 40.0 - y)
        .min(PAGE_W - 2.0 * MARGIN - 40.0)
        .min(380.0);
    let cell = side / n as f32;
    let gx = MARGIN + 20.0;
    let gy = y + 14.0;
    let drawn = if show_grid { n } else { 0 };
    for row in 0..drawn {
        for col in 0..drawn {
            if structural_fixed(col, row, n) && probe.module(col, row) {
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
        .clamp(1, 20);
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
    Ok(document(c, vec![p, q]))
}

/// Costs a QR encoding (through `encode_seedqr`). A SeedQR of `words`
/// words from a fixed dummy seed: its fixed squares are the same as
/// every other seed of that length.
fn probe_matrix(c: &Compute, words: usize) -> Result<QrMatrix, String> {
    // All-zero entropy: "abandon" ×11 "about", or ×23 "art".
    let entropy = [0u8; 32];
    let len = if words == 24 { 32 } else { 16 };
    let m = faraday_bip::bip39::Mnemonic::from_entropy(
        faraday_bip::bip39::Language::English,
        &entropy[..len],
    )
    .map_err(|e| e.to_string())?;
    faraday_codec::seedqr::encode_seedqr(c, &m).map_err(|e| format!("{e:?}"))
}
