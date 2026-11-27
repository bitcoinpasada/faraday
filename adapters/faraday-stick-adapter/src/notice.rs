//! A line of text over the top of the panel, drawn into the panel's bands
//! as the arrow is (`panel.rs`), so the core's frame is untouched.
//!
//! A dev image says which file it has just written this way (`main.rs`),
//! because the kernel console is off the screen once the app runs. The
//! shell has no font of its own and takes no dependency for one: this is
//! a 5 by 7 dot face of lower-case letters, digits and a few marks, which
//! is every character a file name and "written" need. Another character
//! is left blank.

use crate::fb::{self, Geometry};
use crate::orient::Turn;

/// Panel pixels on the short side per whole step of the text's scale: a
/// 480-pixel panel draws it at 2, a 1080-line one at 4.
const LINES_PER_STEP: u16 = 240;

/// The most lines a notice has.
const MAX_LINES: usize = 2;

/// The five dots of each of the seven rows of `c`, the leftmost in bit 4.
fn glyph(c: char) -> [u8; 7] {
    match c {
        'a' => [0, 0, 0b01110, 0b00001, 0b01111, 0b10001, 0b01111],
        'b' => [
            0b10000, 0b10000, 0b10110, 0b11001, 0b10001, 0b10001, 0b11110,
        ],
        'c' => [0, 0, 0b01110, 0b10000, 0b10000, 0b10001, 0b01110],
        'd' => [
            0b00001, 0b00001, 0b01101, 0b10011, 0b10001, 0b10001, 0b01111,
        ],
        'e' => [0, 0, 0b01110, 0b10001, 0b11111, 0b10000, 0b01110],
        'f' => [
            0b00110, 0b01001, 0b01000, 0b11100, 0b01000, 0b01000, 0b01000,
        ],
        'g' => [0, 0b01111, 0b10001, 0b10001, 0b01111, 0b00001, 0b01110],
        'h' => [
            0b10000, 0b10000, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001,
        ],
        'i' => [0b00100, 0, 0b01100, 0b00100, 0b00100, 0b00100, 0b01110],
        'j' => [0b00010, 0, 0b00110, 0b00010, 0b00010, 0b10010, 0b01100],
        'k' => [
            0b10000, 0b10000, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010,
        ],
        'l' => [
            0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        'm' => [0, 0, 0b11010, 0b10101, 0b10101, 0b10101, 0b10101],
        'n' => [0, 0, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001],
        'o' => [0, 0, 0b01110, 0b10001, 0b10001, 0b10001, 0b01110],
        'p' => [0, 0, 0b11110, 0b10001, 0b11110, 0b10000, 0b10000],
        'q' => [0, 0, 0b01111, 0b10001, 0b01111, 0b00001, 0b00001],
        'r' => [0, 0, 0b10110, 0b11001, 0b10000, 0b10000, 0b10000],
        's' => [0, 0, 0b01111, 0b10000, 0b01110, 0b00001, 0b11110],
        't' => [
            0b01000, 0b01000, 0b11100, 0b01000, 0b01000, 0b01001, 0b00110,
        ],
        'u' => [0, 0, 0b10001, 0b10001, 0b10001, 0b10011, 0b01101],
        'v' => [0, 0, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'w' => [0, 0, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010],
        'x' => [0, 0, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001],
        'y' => [0, 0, 0b10001, 0b10001, 0b01111, 0b00001, 0b01110],
        'z' => [0, 0, 0b11111, 0b00010, 0b00100, 0b01000, 0b11111],
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ],
        '6' => [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ],
        '.' => [0, 0, 0, 0, 0, 0b01100, 0b01100],
        '-' => [0, 0, 0, 0b11111, 0, 0, 0],
        '_' => [0, 0, 0, 0, 0, 0, 0b11111],
        ':' => [0, 0b01100, 0b01100, 0, 0b01100, 0b01100, 0],
        '/' => [
            0b00001, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b10000,
        ],
        _ => [0; 7],
    }
}

/// Where the text sits in the frame, in frame pixels.
struct Layout<'a> {
    scale: usize,
    lines: Vec<&'a str>,
    /// Rows of the frame the bar covers.
    height: usize,
    /// The frame's width.
    width: usize,
}

impl<'a> Layout<'a> {
    fn new(geometry: Geometry, turn: Turn, text: &'a str) -> Layout<'a> {
        let scale = usize::from((geometry.width.min(geometry.height) / LINES_PER_STEP).max(1));
        let (fw, fh) = turn.frame_size((geometry.width, geometry.height));
        let (fw, fh) = (usize::from(fw), usize::from(fh));
        let pad = 2 * scale;
        let cols = (fw.saturating_sub(2 * pad) / (6 * scale)).max(1);
        let mut lines: Vec<&str> = Vec::new();
        let mut rest = text;
        while !rest.is_empty() && lines.len() < MAX_LINES {
            if rest.chars().count() <= cols {
                lines.push(rest);
                break;
            }
            // Break at the last space that fits, else in the word.
            let cut = rest
                .char_indices()
                .take(cols + 1)
                .filter(|&(_, c)| c == ' ')
                .map(|(i, _)| i)
                .last()
                .unwrap_or_else(|| rest.char_indices().nth(cols).map_or(rest.len(), |(i, _)| i));
            lines.push(&rest[..cut]);
            rest = rest[cut..].trim_start();
        }
        let n = lines.len().max(1);
        Layout {
            scale,
            height: (2 * pad + 7 * scale + (n - 1) * 9 * scale).min(fh),
            width: fw,
            lines,
        }
    }

    /// Whether the frame pixel `(u, v)` is a dot of the text.
    fn dot(&self, u: usize, v: usize) -> bool {
        let s = self.scale;
        if u < 2 * s || v < 2 * s {
            return false;
        }
        let (x, y) = (u - 2 * s, v - 2 * s);
        let (line, y) = (y / (9 * s), y % (9 * s));
        let (cell, x) = (x / (6 * s), x % (6 * s));
        if y >= 7 * s || x >= 5 * s {
            return false;
        }
        let Some(c) = self.lines.get(line).and_then(|l| l.chars().nth(cell)) else {
            return false;
        };
        (glyph(c)[y / s] >> (4 - x / s)) & 1 == 1
    }
}

/// The rows of the panel the notice covers: the first and the last,
/// inclusive.
pub fn rows(geometry: Geometry, turn: Turn, text: &str) -> (usize, usize) {
    let l = Layout::new(geometry, turn, text);
    let panel = (usize::from(geometry.width), usize::from(geometry.height));
    let (_, a) = turn.to_panel(panel, (0, 0));
    let (_, b) = turn.to_panel(panel, (l.width - 1, l.height - 1));
    (a.min(b), a.max(b))
}

/// Draws the part of the notice that falls in `out`, a band of the
/// panel's packed rows starting at row `first`, in the panel's format:
/// white text on a black bar across the top of the frame as the person
/// sees it.
pub fn draw_rows(out: &mut [u8], geometry: Geometry, turn: Turn, text: &str, first: usize) {
    let bytes = geometry.depth.bytes_per_pixel();
    let row = geometry.row_bytes();
    let (lo, hi) = rows(geometry, turn, text);
    let last = first + out.len() / row;
    if hi < first || lo >= last {
        return;
    }
    let l = Layout::new(geometry, turn, text);
    let paint = |ink: [u8; 4]| {
        let mut pixel = [0u8; 4];
        fb::convert_into(&ink, geometry.depth, &mut pixel[..bytes]);
        pixel
    };
    let (black, white) = (paint([0, 0, 0, 0xff]), paint([0xff; 4]));
    // Only this band's pixels are looked at: a frame that changes writes one
    // row at a time, and on a turned panel every row crosses the bar.
    for y in lo.max(first)..=hi.min(last - 1) {
        for x in 0..usize::from(geometry.width) {
            let (u, v) = turn.to_frame((geometry.width, geometry.height), (x as u16, y as u16));
            let (u, v) = (usize::from(u), usize::from(v));
            if v >= l.height {
                continue;
            }
            let at = (y - first) * row + x * bytes;
            if let Some(slice) = out.get_mut(at..at + bytes) {
                let ink = if l.dot(u, v) { &white } else { &black };
                slice.copy_from_slice(&ink[..bytes]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fb::Depth;

    const TEXT: &str = "faraday-timings.txt written";

    fn panel(width: u16, height: u16) -> Geometry {
        Geometry {
            width,
            height,
            depth: Depth::Rgba8888,
            stride: usize::from(width) * 4,
        }
    }

    /// The whole panel, with the line drawn one band of `step` rows at a
    /// time, as a frame's changed rows are written.
    fn drawn(g: Geometry, turn: Turn, step: usize) -> Vec<u8> {
        let row = g.row_bytes();
        let mut out = vec![0x55u8; row * usize::from(g.height)];
        for (i, band) in out.chunks_mut(row * step).enumerate() {
            draw_rows(band, g, turn, TEXT, i * step);
        }
        out
    }

    #[test]
    fn the_line_reads_the_same_at_every_turn_and_however_the_rows_are_written() {
        let upright = panel(640, 480);
        let plain = drawn(upright, Turn::R0, 480);
        assert_eq!(drawn(upright, Turn::R0, 1), plain);
        // The bar is at the top with text in it, and the rest is untouched.
        let at = |u: usize, v: usize| plain[(v * 640 + u) * 4];
        let (_, last) = rows(upright, Turn::R0, TEXT);
        assert!((0..640).any(|u| (0..=last).any(|v| at(u, v) == 0xff)));
        assert_eq!(at(0, 0), 0);
        assert_eq!(at(0, last + 1), 0x55);
        for (g, turn) in [
            (panel(480, 640), Turn::R90),
            (panel(640, 480), Turn::R180),
            (panel(480, 640), Turn::R270),
        ] {
            let whole = drawn(g, turn, usize::from(g.height));
            assert_eq!(drawn(g, turn, 1), whole, "{turn:?}");
            let size = (usize::from(g.width), usize::from(g.height));
            for v in 0..480 {
                for u in 0..640 {
                    let (x, y) = turn.to_panel(size, (u, v));
                    assert_eq!(whole[(y * size.0 + x) * 4], at(u, v), "{turn:?} {u} {v}");
                }
            }
        }
    }
}
