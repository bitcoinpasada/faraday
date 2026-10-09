//! A small PDF writer for Faraday's printed sheets, and the two sheets.
//!
//! The pages use PDF's standard fonts (Helvetica and Courier), so nothing
//! is embedded, and draw QR codes and grids as filled squares. The sheets
//! go to the Outbox as PDFs on every shell; the device itself prints
//! nothing. Neither sheet carries a secret ([`crate::backup`]).

use osk_codec::qr::{Ecc, Payload, QrMatrix, encode};

use crate::backup::structural_fixed;

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

/// The finished document.
fn document(pages: Vec<Page>) -> Vec<u8> {
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
    for base in ["Helvetica", "Helvetica-Bold", "Courier"] {
        obj(
            &mut out,
            &mut offsets,
            format!(
                "<< /Type /Font /Subtype /Type1 /BaseFont /{base} /Encoding /WinAnsiEncoding >>"
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

/// The fields of a sheet file, in order.
fn fields(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some("faraday-sheet 1") {
        return Err("not a Faraday sheet".into());
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

/// The PDF of a sheet file.
pub fn sheet(text: &str) -> Result<Vec<u8>, String> {
    let f = fields(text)?;
    match get(&f, "kind") {
        Some("wallet") => Ok(wallet_sheet(&f)),
        Some("blank") => blank_sheet(&f),
        Some("share") => Ok(share_sheet(&f)),
        other => Err(format!(
            "a sheet of kind {other:?} is not one this build prints"
        )),
    }
}

fn wallet_sheet(f: &[(String, String)]) -> Vec<u8> {
    let mut p = Page::default();
    let name = get(f, "name").unwrap_or("Wallet");
    let mut y = MARGIN;
    p.text(
        MARGIN,
        y,
        20.0,
        Font::Bold,
        &format!("{name} · wallet backup"),
    );
    y += 30.0;
    p.text(MARGIN, y, 11.0, Font::Sans, get(f, "shape").unwrap_or(""));
    y += 18.0;
    p.text(
        MARGIN,
        y,
        10.0,
        Font::Sans,
        "Xpubs only. This sheet spends nothing; with the seeds it rebuilds the wallet.",
    );
    y += 28.0;
    let descriptor = get(f, "descriptor").unwrap_or("");
    let qr_side = 190.0;
    if let Ok(m) = encode(Payload::Bytes(descriptor.as_bytes()), Ecc::Medium) {
        p.qr(&m, PAGE_W - MARGIN - qr_side, y, qr_side);
    }
    p.text(MARGIN, y, 10.0, Font::Bold, "Descriptor");
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
    p.text(MARGIN, y, 10.0, Font::Bold, "Xpubs");
    y += 16.0;
    for (k, v) in f.iter().filter(|(k, _)| k == "key") {
        let _ = k;
        y += p.wrapped(MARGIN, y, PAGE_W - 2.0 * MARGIN, 8.0, Font::Mono, v) + 6.0;
    }
    y += 14.0;
    p.text(MARGIN, y, 10.0, Font::Bold, "First addresses");
    y += 16.0;
    for (_, v) in f.iter().filter(|(k, _)| k == "address") {
        p.text(MARGIN, y, 8.5, Font::Mono, v);
        y += 13.0;
    }
    y += 30.0;
    for label in ["Written on", "Kept at", "Goes with seed of key"] {
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
    p.text(MARGIN, PAGE_H - MARGIN, 8.0, Font::Sans, "Made by Faraday. Check the first address against your wallet software before relying on this sheet.");
    document(vec![p])
}

/// One sheet of a split backup: what it is, how many rebuild the wallet,
/// and its lines as text and as a QR code to scan.
fn share_sheet(f: &[(String, String)]) -> Vec<u8> {
    let mut p = Page::default();
    let name = get(f, "name").unwrap_or("Wallet");
    let part = get(f, "part").unwrap_or("");
    let quorum = get(f, "quorum").unwrap_or("");
    let mut y = MARGIN;
    p.text(
        MARGIN,
        y,
        20.0,
        Font::Bold,
        &format!("{name} · split backup, sheet {part}"),
    );
    y += 30.0;
    p.text(MARGIN, y, 11.0, Font::Sans, get(f, "shape").unwrap_or(""));
    y += 18.0;
    p.text(
        MARGIN,
        y,
        10.0,
        Font::Sans,
        &format!(
            "Part of the wallet's descriptor: xpubs only, it spends nothing. Sheets enough to \
             hold every xpub ({quorum} wallet) rebuild it."
        ),
    );
    y += 28.0;
    let lines: Vec<&str> = f
        .iter()
        .filter(|(k, _)| k == "line")
        .map(|(_, v)| v.as_str())
        .collect();
    let text = lines.join("\n");
    let qr_side = 210.0;
    if let Ok(m) = encode(Payload::Bytes(text.as_bytes()), Ecc::Medium) {
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
        "Made by Faraday. Scan the code into Faraday, or type the lines, one sheet at a time.",
    );
    document(vec![p])
}

fn blank_sheet(f: &[(String, String)]) -> Result<Vec<u8>, String> {
    let words: usize = get(f, "words").and_then(|w| w.parse().ok()).unwrap_or(24);
    if words != 12 && words != 24 {
        return Err("the template is for 12 or 24 words".into());
    }
    let mut p = Page::default();
    let mut y = MARGIN;
    p.text(
        MARGIN,
        y,
        18.0,
        Font::Bold,
        &format!("Seed sheet · {words} words"),
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
    p.text(
        MARGIN,
        y,
        9.0,
        Font::Sans,
        "Written by hand. Never photographed, scanned or printed once filled in.",
    );
    y += 26.0;
    // The fields to fill after a test restore.
    let network = format!("Network: {}", get(f, "network").unwrap_or("mainnet"));
    for (i, label) in ["Fingerprint", "First address", network.as_str()]
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
    for i in 0..words {
        let x = MARGIN + (i / rows) as f32 * colw;
        let ly = y + (i % rows) as f32 * 22.0;
        p.text(x, ly, 9.0, Font::Sans, &format!("{:>2}", i + 1));
        p.line(x + 20.0, ly + 11.0, x + colw - 30.0, ly + 11.0, 0.5, 0.55);
    }
    y += rows as f32 * 22.0 + 18.0;
    // The SeedQR grid: its fixed squares printed black, the rest ruled
    // for filling in by hand.
    let n = if words == 24 { 29 } else { 25 };
    let probe = probe_matrix(words)?;
    let side = (PAGE_H - MARGIN - 40.0 - y)
        .min(PAGE_W - 2.0 * MARGIN - 40.0)
        .min(380.0);
    let cell = side / n as f32;
    let gx = MARGIN + 20.0;
    let gy = y + 14.0;
    for row in 0..n {
        for col in 0..n {
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
    for i in 0..=n {
        let (width, grey) = if i % 5 == 0 { (0.6, 0.45) } else { (0.3, 0.75) };
        let o = i as f32 * cell;
        p.line(gx + o, gy, gx + o, gy + side, width, grey);
        p.line(gx, gy + o, gx + side, gy + o, width, grey);
        if i % 5 == 0 && i < n {
            p.text(gx + o + 1.0, gy - 11.0, 6.5, Font::Mono, &i.to_string());
            p.text(MARGIN, gy + o + 1.0, 6.5, Font::Mono, &i.to_string());
        }
    }
    p.text(MARGIN, PAGE_H - MARGIN, 8.0, Font::Sans, "There is no line for a passphrase. Written beside the words, it would stop being a second factor.");
    // The places, to fill in by hand: a line each, nothing printed on it.
    let places: usize = get(f, "places")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1)
        .clamp(1, 20);
    let mut q = Page::default();
    let mut y = MARGIN;
    q.text(MARGIN, y, 18.0, Font::Bold, "Places");
    y += 40.0;
    let right = PAGE_W - MARGIN;
    for _ in 0..places {
        q.text(MARGIN, y, 11.0, Font::Sans, "Place");
        q.line(MARGIN + 36.0, y + 12.0, MARGIN + 196.0, y + 12.0, 0.5, 0.55);
        q.text(MARGIN + 204.0, y, 11.0, Font::Sans, "holds");
        q.line(MARGIN + 238.0, y + 12.0, right, y + 12.0, 0.5, 0.55);
        q.line(MARGIN + 238.0, y + 34.0, right, y + 34.0, 0.5, 0.55);
        y += 56.0;
    }
    Ok(document(vec![p, q]))
}

/// A SeedQR of `words` words from a fixed dummy seed: its fixed squares
/// are the same as every other seed of that length.
fn probe_matrix(words: usize) -> Result<QrMatrix, String> {
    let sentence = if words == 24 {
        format!("{} art", ["abandon"; 23].join(" "))
    } else {
        format!("{} about", ["abandon"; 11].join(" "))
    };
    let m = osk_bip::bip39::Mnemonic::parse(osk_bip::bip39::Language::English, &sentence)
        .map_err(|e| e.to_string())?;
    osk_codec::seedqr::encode_seedqr(&m).map_err(|e| format!("{e:?}"))
}
