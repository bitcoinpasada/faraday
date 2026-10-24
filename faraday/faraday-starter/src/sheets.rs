//! The codex32 blank templates (`docs/CODEX32.md` §11): the share card
//! with its numbered boxes, and the booklet's worksheets (dice,
//! checksum, translation) with the booklet's tables, drawn with PDF's
//! standard fonts and the Symbol font for the booklet's second alphabet.
//!
//! A blank template holds no secret: the checksum table, the principal
//! tables and the derivation tables are computed here from BIP 93's
//! arithmetic (`faraday_bip::codex32`), and the tests check them against
//! the booklet's tables (`tools/vectors/codex32-booklet/tables.json`).
//! The booklet's MIT notice is kept beside those tables and on the
//! codex32 Learn page, not printed on the sheets (owner, §1.1 item 6).
//! The sheet file's `kind: codex32` fields are `chars` (48, 74 or 127),
//! `copy` (`string`, `codex32qr`), `worksheets` (`yes` or `no`) and
//! `places`.

use faraday_bip::codex32::{
    INDEX_ORDER, SYMBOL_GLYPHS, checksum_table, pair_symbol, translation_symbols,
};

use faraday_core::pdf::{RawPage, raw_document};
use faraday_crypto::work::Compute;
use faraday_strings::{EN, fill};

/// A4, in points.
const PAGE_W: f32 = 595.0;
const PAGE_H: f32 = 842.0;
const MARGIN: f32 = 48.0;

/// Text in PDF's WinAnsi encoding, escaped for a string literal.
fn pdf_text(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for c in s.chars() {
        match c {
            '(' | ')' | '\\' => {
                out.push(b'\\');
                out.push(c as u8);
            }
            '·' => out.push(0xB7),
            c if (c as u32) < 0x80 => out.push(c as u8),
            _ => out.push(b'?'),
        }
    }
    out
}

/// The fields of a sheet file, in order.
fn fields(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|l| {
            l.split_once(':')
                .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        })
        .collect()
}

fn get<'a>(f: &'a [(String, String)], key: &str) -> Option<&'a str> {
    f.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

/// The bech32 alphabet in value order, lower case.
const ALPHA: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
/// The booklet's order of the characters in its tables.
const BOOK: &[u8; 32] = b"ACDEFGHJKLMNPQRSTUVWXYZ023456789";
/// The characters the booklet asks handwriting to slash.
const SLASHED: &[u8] = b"07ZS";

/// The booklet's pink, the checksum boxes (`/pink`).
const PINK: [f32; 3] = [1.0, 0.9, 0.9];
/// The booklet's light grey, the rows read from a table.
const GREY: [f32; 3] = [0.93, 0.93, 0.93];

/// Landscape A4, in points.
const L_W: f32 = PAGE_H;
const L_H: f32 = PAGE_W;
const L_MARGIN: f32 = 30.0;

/// Characters of checksum in a 48- or 74-character string.
const CHECKSUM_LEN: usize = 13;
/// The residue of `ms` carried 13 characters on: the row under the
/// string on every checksum worksheet (the booklet's; the tests check it
/// against `faraday-bip`'s worksheet).
pub const HRP_RESIDUE: &[u8; 13] = b"33XW87RR3YLJG";
/// The final row of a valid string's checksum worksheet.
pub const SECRET_SHARE: &[u8; 13] = b"SECRETSHARE32";

/// Steps of the checksum ladder, two characters each: 16 at 48
/// characters, 29 at 74.
pub fn ladder_steps(chars: usize) -> usize {
    (chars - 3 - CHECKSUM_LEN) / 2
}

/// The box where sum row `j` of the ladder starts; its lookup row starts
/// two boxes on.
pub fn sum_box(j: usize) -> usize {
    2 * j + 4
}

/// The value of a bech32 character, either case.
fn val(c: u8) -> u8 {
    ALPHA
        .iter()
        .position(|&a| a == c.to_ascii_lowercase())
        .unwrap_or(0) as u8
}

/// The upper-case character of a value.
fn up(v: u8) -> u8 {
    ALPHA[usize::from(v & 31)].to_ascii_uppercase()
}

/// Multiplication in GF(32), BIP 93's field: x^5 + x^3 + 1.
fn gf_mul(a: u8, b: u8) -> u8 {
    let (mut a, mut res) = (a, 0u8);
    for i in 0..5 {
        if (b >> i) & 1 == 1 {
            res ^= a;
        }
        a <<= 1;
        if a >= 32 {
            a ^= 41;
        }
    }
    res
}

// ---------------------------------------------------------------------
// The drawing
// ---------------------------------------------------------------------

/// How a box is drawn.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// A thin rule round white: a cell to fill in.
    Plain,
    /// A heavier outline: the given data.
    Bold,
    /// Pink and bold: the checksum.
    Pink,
    /// Grey: a row read from a table.
    Grey,
}

/// One page and its height, drawn from the top left, y going down.
struct Canvas {
    ops: Vec<u8>,
    media: Option<(f32, f32)>,
    h: f32,
}

impl Canvas {
    fn portrait() -> Self {
        Canvas {
            ops: Vec::new(),
            media: None,
            h: PAGE_H,
        }
    }

    fn landscape() -> Self {
        Canvas {
            ops: Vec::new(),
            media: Some((L_W, L_H)),
            h: L_H,
        }
    }

    fn op(&mut self, s: &str) {
        self.ops.extend_from_slice(s.as_bytes());
        self.ops.push(b'\n');
    }

    /// Text already escaped for a PDF string, in font `font` (`F1` to `F4`).
    fn raw(&mut self, x: f32, y: f32, size: f32, font: &str, escaped: &[u8]) {
        let head = format!(
            "0 g BT /{font} {size} Tf {x:.2} {:.2} Td (",
            self.h - y - size
        );
        self.ops.extend_from_slice(head.as_bytes());
        self.ops.extend_from_slice(escaped);
        self.ops.extend_from_slice(b") Tj ET\n");
    }

    fn sans(&mut self, x: f32, y: f32, size: f32, bold: bool, s: &str) {
        let font = if bold { "F2" } else { "F1" };
        self.raw(x, y, size, font, &pdf_text(s));
    }

    fn mono(&mut self, x: f32, y: f32, size: f32, s: &str) {
        self.raw(x, y, size, "F3", &pdf_text(s));
    }

    /// One Courier character centred in a box, slashed where the booklet
    /// asks handwriting to be (0, 7, S and Z): standard Courier has no
    /// slashed zero, so the slash is a stroke over the glyph.
    fn mono_char(&mut self, x: f32, y: f32, w: f32, h: f32, size: f32, c: u8) {
        let gx = x + (w - 0.6 * size) / 2.0;
        let gy = y + (h - size) / 2.0;
        self.raw(gx, gy, size, "F3", &pdf_text(&char::from(c).to_string()));
        if SLASHED.contains(&c) {
            let base = gy + size * 0.93;
            self.line(
                gx + 0.05 * size,
                base,
                gx + 0.55 * size,
                base - 0.62 * size,
                0.5,
                0.0,
            );
        }
    }

    /// A symbol of the booklet's second alphabet, centred in a box.
    fn glyph(&mut self, x: f32, y: f32, w: f32, h: f32, size: f32, v: u8) {
        let (font, byte) = glyph_bytes(v);
        let gx = x + (w - 0.55 * size) / 2.0;
        let gy = y + (h - size) / 2.0;
        self.raw(gx, gy, size, font, &[byte]);
    }

    fn fill(&mut self, x: f32, y: f32, w: f32, h: f32, rgb: [f32; 3]) {
        self.op(&format!(
            "{:.2} {:.2} {:.2} rg {x:.2} {:.2} {w:.2} {h:.2} re f",
            rgb[0],
            rgb[1],
            rgb[2],
            self.h - y - h
        ));
    }

    fn frame(&mut self, x: f32, y: f32, w: f32, h: f32, width: f32, grey: f32) {
        self.op(&format!(
            "{grey:.2} G {width:.2} w {x:.2} {:.2} {w:.2} {h:.2} re S",
            self.h - y - h
        ));
    }

    fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, width: f32, grey: f32) {
        self.op(&format!(
            "{grey:.2} G {width:.2} w {x1:.2} {:.2} m {x2:.2} {:.2} l S",
            self.h - y1,
            self.h - y2
        ));
    }

    /// A box of `kind`, with `c` printed in it if given.
    fn cell(&mut self, x: f32, y: f32, w: f32, h: f32, kind: Kind, c: Option<u8>) {
        match kind {
            Kind::Pink => self.fill(x, y, w, h, PINK),
            Kind::Grey => self.fill(x, y, w, h, GREY),
            Kind::Plain | Kind::Bold => {}
        }
        let (width, grey) = match kind {
            Kind::Plain | Kind::Grey => (0.4, 0.25),
            Kind::Bold | Kind::Pink => (1.3, 0.0),
        };
        self.frame(x, y, w, h, width, grey);
        if let Some(c) = c {
            let size = (h * 0.62).min(w * 0.9 / 0.6).min(14.0);
            self.mono_char(x, y, w, h, size, c);
        }
    }

    /// The footer every page carries.
    fn footer(&mut self) {
        let y = self.h - 22.0;
        self.sans(L_MARGIN, y, 8.0, false, EN.pdf_print_actual_size);
    }
}

/// The font and byte of the symbol of value `v` in the booklet's second
/// alphabet: the Symbol font for Greek, aleph, the circled plus, the
/// diamond and the heart; Helvetica's WinAnsi for the rest.
fn glyph_bytes(v: u8) -> (&'static str, u8) {
    match SYMBOL_GLYPHS[usize::from(v & 31)] {
        '×' => ("F1", 0xD7),
        'ℵ' => ("F4", 0xC0),
        'α' => ("F4", b'a'),
        'β' => ("F4", b'b'),
        'Γ' => ("F4", b'G'),
        'Δ' => ("F4", b'D'),
        'ε' => ("F4", b'e'),
        'η' => ("F4", b'h'),
        'Θ' => ("F4", b'Q'),
        'Λ' => ("F4", b'L'),
        'μ' => ("F4", b'm'),
        'Ξ' => ("F4", b'X'),
        'Π' => ("F4", b'P'),
        'ρ' => ("F4", b'r'),
        'Σ' => ("F4", b'S'),
        'Φ' => ("F4", b'F'),
        'Ψ' => ("F4", b'Y'),
        'Ω' => ("F4", b'W'),
        '@' => ("F1", b'@'),
        '#' => ("F1", b'#'),
        '%' => ("F1", b'%'),
        '¢' => ("F1", 0xA2),
        '¥' => ("F1", 0xA5),
        '€' => ("F1", 0x80),
        '¤' => ("F1", 0xA4),
        '⊕' => ("F4", 0xC5),
        '†' => ("F1", 0x86),
        '‡' => ("F1", 0x87),
        '§' => ("F1", 0xA7),
        '¶' => ("F1", 0xB6),
        '♦' => ("F4", 0xA8),
        '♥' => ("F4", 0xA9),
        _ => ("F1", b'?'),
    }
}

// ---------------------------------------------------------------------
// The sheet
// ---------------------------------------------------------------------

/// Costs a picture drawn and the checksum table made (through
/// `document`). The PDF of a codex32 sheet file.
pub fn sheet(c: &Compute, text: &str) -> Result<Vec<u8>, String> {
    let f = fields(text);
    let f = f.as_slice();
    if get(f, "kind") != Some("codex32") {
        return Err(EN.pdf_not_a_faraday_sheet.into());
    }
    let chars: usize = get(f, "chars").and_then(|v| v.parse().ok()).unwrap_or(48);
    if ![48, 74, 127].contains(&chars) {
        return Err(EN.pdf_c32_length.into());
    }
    let copy = get(f, "copy").unwrap_or("string");
    let qr = chars == 48 && copy.split_whitespace().any(|w| w == "codex32qr");
    let string = copy.split_whitespace().any(|w| w == "string") || !qr;
    let worksheets = get(f, "worksheets") == Some("yes") && chars != 127;
    let places: usize = get(f, "places")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1)
        .clamp(1, 20);
    let mut pages = vec![card(chars, string, qr)];
    if worksheets {
        pages.push(dice());
        pages.extend(checksum_pages(chars)?);
        if chars == 48 {
            for k in [2, 3] {
                pages.push(translation(k, chars));
            }
        }
        pages.push(principal_page());
        pages.extend(checksum_table_pages());
    }
    pages.push(places_page(places));
    Ok(raw_document(
        c,
        pages
            .into_iter()
            .map(|p| RawPage {
                ops: p.ops,
                size: p.media,
            })
            .collect(),
    ))
}

/// The places page, as the words template has it.
fn places_page(places: usize) -> Canvas {
    let mut q = Canvas::portrait();
    q.sans(MARGIN, MARGIN, 18.0, true, EN.pdf_places);
    q.footer();
    let right = PAGE_W - MARGIN;
    let mut y = MARGIN + 40.0;
    for _ in 0..places {
        q.sans(MARGIN, y, 11.0, false, EN.pdf_place);
        q.line(MARGIN + 36.0, y + 12.0, MARGIN + 196.0, y + 12.0, 0.5, 0.55);
        q.sans(MARGIN + 204.0, y, 11.0, false, EN.pdf_holds);
        q.line(MARGIN + 238.0, y + 12.0, right, y + 12.0, 0.5, 0.55);
        q.line(MARGIN + 238.0, y + 34.0, right, y + 34.0, 0.5, 0.55);
        y += 56.0;
    }
    q
}

// ---------------------------------------------------------------------
// The share card
// ---------------------------------------------------------------------

/// Whether module (x, y) of a version 3 QR (29 × 29) is dark in a fixed
/// pattern every Codex32QR shares: the finders, the timing lines, the
/// alignment square and the dark module. The separators and the format
/// areas are not drawn.
fn qr_fixed_dark(x: usize, y: usize) -> bool {
    let n = 29;
    if x == 8 && y == n - 8 {
        return true;
    }
    // A finder at (ox, oy): a dark ring, a light ring, a dark 3 x 3.
    for (ox, oy) in [(0, 0), (n - 7, 0), (0, n - 7)] {
        if (ox..ox + 7).contains(&x) && (oy..oy + 7).contains(&y) {
            let (dx, dy) = (x - ox, y - oy);
            let ring = dx == 0 || dx == 6 || dy == 0 || dy == 6;
            let core = (2..=4).contains(&dx) && (2..=4).contains(&dy);
            return ring || core;
        }
    }
    // The timing lines run between the finders, dark on even squares.
    if y == 6 && (8..n - 8).contains(&x) {
        return x.is_multiple_of(2);
    }
    if x == 6 && (8..n - 8).contains(&y) {
        return y.is_multiple_of(2);
    }
    // The alignment square, centred on (n - 7, n - 7).
    let (dx, dy) = (x.abs_diff(n - 7), y.abs_diff(n - 7));
    dx <= 2 && dy <= 2 && (dx == 2 || dy == 2 || (dx == 0 && dy == 0))
}

/// Which squares of a Codex32QR grid are drawn grey as printed already.
pub fn codex32qr_fixed() -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for y in 0..29 {
        for x in 0..29 {
            if qr_fixed_dark(x, y) {
                out.push((x, y));
            }
        }
    }
    out
}

/// The share card: `ms1` printed in boxes 1 to 3, the labelled boxes,
/// every box numbered, rows of eight in groups of four; the Codex32QR
/// grid where asked; the fingerprint box; the alphabet at the foot.
fn card(chars: usize, string: bool, qr: bool) -> Canvas {
    let mut p = Canvas::portrait();
    p.sans(
        MARGIN,
        MARGIN,
        18.0,
        true,
        &fill(EN.pdf_c32_card, &[&chars.to_string()]),
    );
    p.sans(MARGIN, MARGIN + 26.0, 9.0, false, EN.pdf_written_by_hand);
    p.sans(MARGIN, MARGIN + 38.0, 9.0, false, EN.pdf_c32_hand);
    let top = 118.0;
    let rows = chars.div_ceil(8);
    let pitch = ((690.0 - top) / rows as f32).min(54.0);
    let bsz = (pitch - 20.0).min(34.0);
    let gap = 12.0;
    let bx = |i: usize| MARGIN + (i % 8) as f32 * bsz + ((i % 8) / 4) as f32 * gap;
    let by = |i: usize| top + (i / 8) as f32 * pitch;
    let strip = 8.0 * bsz + gap;
    if string {
        for i in 0..chars {
            let (x, y) = (bx(i), by(i));
            p.mono(x + 1.0, y - 7.5, 6.0, &(i + 1).to_string());
            if i < 3 {
                p.cell(x, y, bsz, bsz, Kind::Bold, Some(b"ms1"[i]));
            } else {
                p.cell(x, y, bsz, bsz, Kind::Plain, None);
            }
        }
        // Threshold (box 4), Identifier (boxes 5 to 8), Share index (box 9).
        let under = by(0) + bsz + 2.0;
        p.sans(bx(3), under, 6.0, false, EN.pdf_c32_threshold);
        p.line(bx(4), under + 0.5, bx(7) + bsz, under + 0.5, 0.4, 0.4);
        p.sans(bx(4), under + 2.0, 6.0, false, EN.pdf_c32_identifier);
        p.sans(bx(8), by(8) + bsz + 2.0, 6.0, false, EN.pdf_c32_index);
    }
    if qr {
        let (gx, cell) = if string {
            let gx = MARGIN + strip + 24.0;
            (gx, (PAGE_W - MARGIN - gx) / 29.0)
        } else {
            (MARGIN + 20.0, 15.0)
        };
        let gy = top;
        for (x, y) in codex32qr_fixed() {
            p.fill(
                gx + x as f32 * cell,
                gy + y as f32 * cell,
                cell,
                cell,
                [0.62, 0.62, 0.62],
            );
        }
        for i in 0..=29 {
            let (width, grey) = if i % 5 == 0 { (0.6, 0.45) } else { (0.3, 0.75) };
            let o = i as f32 * cell;
            p.line(gx + o, gy, gx + o, gy + 29.0 * cell, width, grey);
            p.line(gx, gy + o, gx + 29.0 * cell, gy + o, width, grey);
        }
        p.sans(gx, gy + 29.0 * cell + 4.0, 7.0, false, EN.pdf_c32_grid);
    }
    // The fingerprint, eight hex digits, filled by hand.
    p.sans(MARGIN, 692.0, 9.0, false, EN.pdf_fingerprint);
    for i in 0..8 {
        p.cell(
            MARGIN + i as f32 * 24.0,
            706.0,
            22.0,
            22.0,
            Kind::Plain,
            None,
        );
    }
    // The alphabet, 0, 7, S and Z slashed.
    p.sans(MARGIN, 744.0, 8.0, false, EN.pdf_c32_alphabet);
    let pitch = (PAGE_W - 2.0 * MARGIN) / 32.0;
    for (i, c) in ALPHA.iter().enumerate() {
        p.mono_char(
            MARGIN + i as f32 * pitch,
            758.0,
            pitch,
            20.0,
            14.0,
            c.to_ascii_uppercase(),
        );
    }
    p.footer();
    p
}

// ---------------------------------------------------------------------
// The dice worksheet
// ---------------------------------------------------------------------

/// The dice worksheet: for each character, five dice with their first
/// and second roll and the bit; the character's box; and the 32-leaf
/// tree the five bits walk down, bit 1 first, 0 to the left.
fn dice() -> Canvas {
    let mut p = Canvas::portrait();
    p.sans(MARGIN, MARGIN, 18.0, true, EN.pdf_c32_dice);
    let x0 = MARGIN + 28.0;
    let (bw, bh, group) = (22.0, 22.0, 76.0);
    for d in 0..5 {
        let gx = x0 + d as f32 * group;
        p.sans(
            gx,
            78.0,
            8.0,
            true,
            &fill(EN.pdf_c32_die, &[&(d + 1).to_string()]),
        );
        for (j, head) in [
            EN.pdf_c32_roll_first,
            EN.pdf_c32_roll_second,
            EN.pdf_c32_bit,
        ]
        .iter()
        .enumerate()
        {
            p.sans(gx + j as f32 * bw, 90.0, 5.5, false, head);
        }
    }
    let rx = x0 + 5.0 * group;
    p.sans(rx, 78.0, 8.0, true, EN.pdf_c32_character);
    for r in 0..10 {
        let y = 102.0 + r as f32 * 32.0;
        p.sans(MARGIN, y + 6.0, 9.0, false, &(r + 1).to_string());
        for d in 0..5 {
            for j in 0..3 {
                let kind = if j == 2 { Kind::Grey } else { Kind::Plain };
                p.cell(x0 + d as f32 * group + j as f32 * bw, y, bw, bh, kind, None);
            }
        }
        p.cell(rx, y, 28.0, bh, Kind::Bold, None);
    }
    // The tree: leaves in the bech32 alphabet's value order.
    p.sans(MARGIN, 438.0, 8.0, false, EN.pdf_c32_tree);
    let leaf = 15.4;
    let lx = |i: usize| MARGIN + 6.0 + i as f32 * leaf + leaf / 2.0;
    let ly = |l: usize| 462.0 + l as f32 * 26.0;
    let centre = |l: usize, j: usize| {
        let span = 1usize << (5 - l);
        lx(j * span) / 2.0 + lx(j * span + span - 1) / 2.0
    };
    for l in 0..5 {
        p.sans(MARGIN - 18.0, ly(l) + 8.0, 7.0, false, &(l + 1).to_string());
        for j in 0..(1usize << l) {
            for child in [2 * j, 2 * j + 1] {
                p.line(
                    centre(l, j),
                    ly(l),
                    centre(l + 1, child),
                    ly(l + 1),
                    0.4,
                    0.2,
                );
            }
        }
    }
    p.mono(centre(0, 0) - 14.0, ly(0) + 4.0, 7.0, "0");
    p.mono(centre(0, 0) + 10.0, ly(0) + 4.0, 7.0, "1");
    for (i, c) in ALPHA.iter().enumerate() {
        p.mono_char(
            lx(i) - leaf / 2.0,
            ly(5) + 2.0,
            leaf,
            12.0,
            9.0,
            c.to_ascii_uppercase(),
        );
    }
    p.footer();
    p
}

// ---------------------------------------------------------------------
// The checksum worksheet
// ---------------------------------------------------------------------

/// The checksum worksheet's pages: the booklet's ladder at its box
/// numbers. At 48 characters one landscape page; at 74, two, the steps
/// shared between them.
fn checksum_pages(chars: usize) -> Result<Vec<Canvas>, String> {
    if ![48, 74].contains(&chars) {
        return Err(EN.pdf_c32_length.into());
    }
    let (steps, cl, last) = (ladder_steps(chars), CHECKSUM_LEN, chars - CHECKSUM_LEN);
    let bw = (L_W - 2.0 * L_MARGIN) / chars as f32;
    let (rp, first_page_steps, pages) = if chars == 48 {
        (14.0, steps, 1)
    } else {
        (15.0, 14, 2)
    };
    let size = if chars == 48 { 6.5 } else { 5.0 };
    let bx = |b: usize| L_MARGIN + (b - 1) as f32 * bw;
    let mut out = Vec::new();
    for page in 0..pages {
        let mut p = Canvas::landscape();
        let title = fill(EN.pdf_c32_checksum, &[&chars.to_string()]);
        let title = if pages > 1 {
            fill(
                EN.pdf_c32_page,
                &[&title, &(page + 1).to_string(), &pages.to_string()],
            )
        } else {
            title
        };
        p.sans(L_MARGIN, 20.0, 14.0, true, &title);
        for b in 1..=chars {
            let s = b.to_string();
            let tw = 0.6 * size * s.len() as f32;
            p.mono(bx(b) + (bw - tw) / 2.0, 46.0, size, &s);
        }
        let mut y = 58.0;
        let (from, to) = if page == 0 {
            (0, first_page_steps)
        } else {
            (first_page_steps, steps)
        };
        if page == 0 {
            for b in 1..=chars {
                if b <= 3 {
                    p.cell(bx(b), y, bw, rp - 1.0, Kind::Bold, Some(b"ms1"[b - 1]));
                } else if b > last {
                    p.cell(bx(b), y, bw, rp - 1.0, Kind::Pink, None);
                } else {
                    p.cell(bx(b), y, bw, rp - 1.0, Kind::Bold, None);
                }
            }
            y += rp;
            for (i, &c) in HRP_RESIDUE.iter().enumerate() {
                p.cell(bx(4 + i), y, bw, rp - 1.0, Kind::Plain, Some(c));
            }
            y += rp;
        }
        for j in from..to {
            for i in 0..cl {
                p.cell(bx(sum_box(j) + i), y, bw, rp - 1.0, Kind::Plain, None);
            }
            y += rp;
            for i in 0..cl {
                p.cell(bx(sum_box(j) + 2 + i), y, bw, rp - 1.0, Kind::Grey, None);
            }
            y += rp;
        }
        if page + 1 == pages {
            for (i, &c) in SECRET_SHARE.iter().enumerate() {
                p.cell(
                    bx(sum_box(steps) + i),
                    y,
                    bw,
                    rp - 1.0,
                    Kind::Plain,
                    Some(c),
                );
            }
        }
        // The key.
        let ky = L_H - 40.0;
        let mut kx = L_MARGIN + 160.0;
        for (kind, text) in [
            (Kind::Bold, EN.pdf_c32_key_given),
            (Kind::Pink, EN.pdf_c32_key_checksum),
            (Kind::Grey, EN.pdf_c32_key_lookup),
        ] {
            p.cell(kx, ky, 12.0, 10.0, kind, None);
            p.sans(kx + 16.0, ky, 8.0, false, text);
            kx += 190.0;
        }
        p.footer();
        out.push(p);
    }
    Ok(out)
}

// ---------------------------------------------------------------------
// The translation worksheet
// ---------------------------------------------------------------------

/// The translation worksheet for `k` initial shares at 48 characters:
/// each given string, its symbol, its translated row, and the sum, which
/// is the new share; and the booklet's derivation table for `k`.
fn translation(k: usize, chars: usize) -> Canvas {
    let mut p = Canvas::landscape();
    p.sans(
        L_MARGIN,
        20.0,
        14.0,
        true,
        &fill(
            EN.pdf_c32_translation,
            &[&k.to_string(), &chars.to_string()],
        ),
    );
    let gutter = 56.0;
    let bw = (L_W - 2.0 * L_MARGIN - gutter) / chars as f32;
    let bx = |b: usize| L_MARGIN + gutter + (b - 1) as f32 * bw;
    let h = 17.0;
    let size = 6.0;
    for b in 1..=chars {
        let s = b.to_string();
        let tw = 0.6 * size * s.len() as f32;
        p.mono(bx(b) + (bw - tw) / 2.0, 46.0, size, &s);
    }
    let mut y = 58.0;
    for &index in INDEX_ORDER.iter().take(k) {
        let name = char::from(index).to_ascii_uppercase().to_string();
        p.sans(
            L_MARGIN,
            y + 4.0,
            8.0,
            true,
            &fill(EN.pdf_c32_share, &[&name]),
        );
        for b in 1..=chars {
            let c = (b <= 3).then(|| b"ms1"[b - 1]);
            p.cell(bx(b), y, bw, h, Kind::Bold, c);
        }
        y += h + 8.0;
        p.sans(L_MARGIN, y + 4.0, 5.0, false, EN.pdf_c32_symbol);
        p.cell(L_MARGIN + gutter - 30.0, y, 26.0, h, Kind::Plain, None);
        for b in 4..=chars {
            p.cell(bx(b), y, bw, h, Kind::Grey, None);
        }
        y += h + 14.0;
    }
    p.sans(L_MARGIN, y + 4.0, 8.0, true, EN.pdf_c32_new_share);
    for b in 1..=chars {
        let c = (b <= 3).then(|| b"ms1"[b - 1]);
        p.cell(bx(b), y, bw, h, Kind::Bold, c);
    }
    y += h + 26.0;
    // The derivation table for k.
    let t = derivation_table(k);
    p.sans(
        L_MARGIN,
        y,
        9.0,
        true,
        &fill(EN.pdf_c32_derivation, &[&k.to_string()]),
    );
    draw_table(&mut p, L_MARGIN, y + 16.0, 24.0, 18.0, 11.0, &t);
    p.footer();
    p
}

// ---------------------------------------------------------------------
// The booklet's tables
// ---------------------------------------------------------------------

/// One entry of a booklet table.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Entry {
    /// A character of the first alphabet (upper case).
    Char(u8),
    /// A symbol of the second alphabet, by value.
    Symbol(u8),
}

impl Entry {
    /// The entry as the booklet's text has it.
    pub fn text(self) -> char {
        match self {
            Entry::Char(c) => char::from(c),
            Entry::Symbol(v) => SYMBOL_GLYPHS[usize::from(v & 31)],
        }
    }
}

/// A table of the booklet: its column heads, and its rows each with a
/// head.
#[derive(Debug)]
pub struct Table {
    /// The column heads.
    pub columns: Vec<Entry>,
    /// The rows: a head and the entries.
    pub rows: Vec<(Entry, Vec<Entry>)>,
}

impl Table {
    /// The column heads as a string.
    pub fn column_text(&self) -> String {
        self.columns.iter().map(|e| e.text()).collect()
    }

    /// Each row as its head and its entries' text.
    pub fn row_text(&self) -> Vec<(char, String)> {
        self.rows
            .iter()
            .map(|(h, r)| (h.text(), r.iter().map(|e| e.text()).collect()))
            .collect()
    }
}

/// Cheap: the four principal tables of the booklet's page 4, in the
/// order Addition, Translation, Recovery, Fusion, worked out from BIP
/// 93's field.
pub fn principal_tables() -> [Table; 4] {
    let chars: Vec<u8> = BOOK.to_vec();
    let others: Vec<u8> = BOOK.iter().copied().filter(|&c| c != b'S').collect();
    let symbols: Vec<u8> = (1..32).collect();
    let addition = Table {
        columns: chars.iter().map(|&c| Entry::Char(c)).collect(),
        rows: chars
            .iter()
            .map(|&r| {
                (
                    Entry::Char(r),
                    chars
                        .iter()
                        .map(|&c| Entry::Char(up(val(r) ^ val(c))))
                        .collect(),
                )
            })
            .collect(),
    };
    let translation = Table {
        columns: symbols.iter().map(|&s| Entry::Symbol(s)).collect(),
        rows: chars
            .iter()
            .map(|&r| {
                (
                    Entry::Char(r),
                    symbols
                        .iter()
                        .map(|&s| Entry::Char(up(gf_mul(val(r), s))))
                        .collect(),
                )
            })
            .collect(),
    };
    let recovery = Table {
        columns: others.iter().map(|&c| Entry::Char(c)).collect(),
        rows: others
            .iter()
            .map(|&other| {
                (
                    Entry::Char(other),
                    others
                        .iter()
                        .map(|&own| {
                            Entry::Symbol(
                                pair_symbol(
                                    own.to_ascii_lowercase(),
                                    other.to_ascii_lowercase(),
                                    b's',
                                )
                                .unwrap_or(0),
                            )
                        })
                        .collect(),
                )
            })
            .collect(),
    };
    let fusion = Table {
        columns: symbols.iter().map(|&s| Entry::Symbol(s)).collect(),
        rows: symbols
            .iter()
            .map(|&r| {
                (
                    Entry::Symbol(r),
                    symbols
                        .iter()
                        .map(|&s| Entry::Symbol(gf_mul(r, s)))
                        .collect(),
                )
            })
            .collect(),
    };
    [addition, translation, recovery, fusion]
}

/// Cheap: the booklet's derivation table for `k` initial shares (A, C,
/// D, ...): for each initial share, the translation symbol it takes
/// towards each other share index.
pub fn derivation_table(k: usize) -> Table {
    let k = k.clamp(2, 8);
    let initial = &INDEX_ORDER[..k];
    let targets = &INDEX_ORDER[k..];
    let mut rows: Vec<(Entry, Vec<Entry>)> = initial
        .iter()
        .map(|&i| (Entry::Char(i.to_ascii_uppercase()), Vec::new()))
        .collect();
    for &t in targets {
        let mut out = [0u8; 8];
        if translation_symbols(initial, t, &mut out[..k]).is_ok() {
            for (row, &v) in rows.iter_mut().zip(&out[..k]) {
                row.1.push(Entry::Symbol(v));
            }
        }
    }
    Table {
        columns: targets
            .iter()
            .map(|&t| Entry::Char(t.to_ascii_uppercase()))
            .collect(),
        rows,
    }
}

/// Costs 1,024 small residue steps. The booklet's checksum table (its
/// pages 15 and 16): for each leading pair, in the booklet's order, the
/// 13 characters it selects.
pub fn checksum_rows() -> Vec<(String, String)> {
    let mut order: Vec<u8> = BOOK.to_vec();
    order.sort_unstable();
    let mut out = Vec::with_capacity(1024);
    for &a in &order {
        for &b in &order {
            let row = checksum_table(val(a), val(b));
            out.push((
                format!("{}{}", char::from(a), char::from(b)),
                row.iter().map(|&v| char::from(up(v))).collect(),
            ));
        }
    }
    out
}

/// Draws `t` with its heads at (x, y), cells of `cw` by `ch`.
fn draw_table(p: &mut Canvas, x: f32, y: f32, cw: f32, ch: f32, size: f32, t: &Table) {
    fn put(p: &mut Canvas, g: (f32, f32, f32, f32, f32), e: Entry) {
        match e {
            // A table is read, not handwritten: no slashes.
            Entry::Char(c) => p.raw(
                g.0 + (g.2 - 0.6 * g.4) / 2.0,
                g.1 + (g.3 - g.4) / 2.0,
                g.4,
                "F3",
                &[c],
            ),
            Entry::Symbol(v) => p.glyph(g.0, g.1, g.2, g.3, g.4, v),
        }
    }
    p.fill(x, y, cw * (t.columns.len() + 1) as f32, ch, GREY);
    for (j, &e) in t.columns.iter().enumerate() {
        put(p, (x + (j + 1) as f32 * cw, y, cw, ch, size), e);
    }
    for (i, (head, row)) in t.rows.iter().enumerate() {
        let ry = y + (i + 1) as f32 * ch;
        p.fill(x, ry, cw, ch, GREY);
        put(p, (x, ry, cw, ch, size), *head);
        for (j, &e) in row.iter().enumerate() {
            put(p, (x + (j + 1) as f32 * cw, ry, cw, ch, size), e);
        }
    }
    let (w, h) = (
        cw * (t.columns.len() + 1) as f32,
        ch * (t.rows.len() + 1) as f32,
    );
    p.frame(x, y, w, h, 0.4, 0.3);
    p.line(x + cw, y, x + cw, y + h, 0.4, 0.3);
    p.line(x, y + ch, x + w, y + ch, 0.4, 0.3);
}

/// The booklet's four principal tables on one page.
fn principal_page() -> Canvas {
    let mut p = Canvas::portrait();
    p.sans(MARGIN, MARGIN, 18.0, true, EN.pdf_c32_tables);
    let tables = principal_tables();
    let names = [
        EN.pdf_c32_addition,
        EN.pdf_c32_translation_table,
        EN.pdf_c32_recovery,
        EN.pdf_c32_fusion,
    ];
    for (i, (t, name)) in tables.iter().zip(names).enumerate() {
        let x = MARGIN + (i % 2) as f32 * 256.0;
        let y = 100.0 + (i / 2) as f32 * 300.0;
        p.sans(x, y - 14.0, 9.0, true, name);
        draw_table(&mut p, x, y, 7.1, 8.0, 5.5, t);
    }
    p.footer();
    p
}

/// The checksum table on two pages, seven columns each.
fn checksum_table_pages() -> Vec<Canvas> {
    let rows = checksum_rows();
    let mut out = Vec::new();
    for (n, chunk) in rows.chunks(512).enumerate() {
        let mut p = Canvas::portrait();
        p.sans(
            MARGIN,
            MARGIN,
            14.0,
            true,
            &fill(EN.pdf_c32_checksum_table, &[&(n + 1).to_string(), "2"]),
        );
        for (i, (key, row)) in chunk.iter().enumerate() {
            let (col, r) = (i / 74, i % 74);
            p.mono(
                MARGIN + col as f32 * 71.0,
                80.0 + r as f32 * 9.0,
                7.0,
                &format!("{key} {row}"),
            );
        }
        p.footer();
        out.push(p);
    }
    out
}
