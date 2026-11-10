//! The framebuffer: its geometry, and the conversion from the core's
//! premultiplied RGBA8888 frame to what the panel wants.
//!
//! Five formats are supported:
//!
//! - **RGB565**, 16 bits per pixel, little-endian, red in the top five
//!   bits. A framebuffer that is 16 bits (a DRM/KMS one that is not
//!   Faraday's Pi kernel) is written this way, as is the SeedSigner
//!   fork's `hardware/DPI28.py`.
//! - **BGR888** and **RGB888**, 24 bits per pixel, three bytes and no
//!   pad. A UEFI firmware that hands over a 24-bit mode is the reason
//!   these are here.
//! - **BGRA8888**, 32 bits per pixel, blue first, alpha forced opaque.
//!   This is what the legacy firmware framebuffer reports.
//! - **RGBA8888**, 32 bits per pixel, red first: the core's own bytes. The
//!   Pi's kernel is patched so that vc4's framebuffer is this
//!   (`faraday/image/boards/pi3/linux-patches/`, `docs/FRAMES.md` §5); it
//!   is taken for a 32-bit framebuffer whose sysfs `name` is `vc4drmfb`,
//!   and the presenter writes the frame to it with no conversion.
//!
//! Which one is in front of us is a runtime question, not a build-time
//! one, so it is read from sysfs at start-up along with the size and the
//! row stride. A padded stride is written row by row rather than skewing
//! the image, which the Python driver only warns about.
//!
//! **Which way round the three bytes of a 24-bit pixel go** is the one
//! thing a bit depth does not say. The kernel's `simplefb` format table
//! (`drivers/video/fbdev/simplefb.c`) has one 24-bit entry, `r8g8b8`,
//! with red at bit offset 16, green at 8 and blue at 0 — the same layout
//! as `x8r8g8b8` without the pad byte. Little-endian puts bit 0 in the
//! first byte, so blue is the byte at the lowest address and the memory
//! order is blue, green, red. That is the default here. A framebuffer
//! driver that publishes `red`, `green` and `blue` bitfields in sysfs is
//! believed instead of the default, so a machine whose firmware chose the
//! other order is drawn correctly rather than in false colour.
//!
//! The core's frame is premultiplied, so compositing it over a black
//! panel is the identity: the stored red, green and blue *are* the
//! colour. That is the same reasoning the desktop shell's blit uses.

use std::fs;
use std::path::Path;

/// The pixel format of a framebuffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Depth {
    /// 16 bits: 5 red, 6 green, 5 blue, little-endian.
    Rgb565,
    /// 24 bits: blue, green, red, in that order in memory.
    Bgr888,
    /// 24 bits: red, green, blue, in that order in memory.
    Rgb888,
    /// 32 bits: blue, green, red, alpha.
    Bgra8888,
    /// 32 bits: red, green, blue, alpha (or a pad byte): the core's own
    /// frame, which needs no conversion.
    Rgba8888,
}

impl Depth {
    /// Bytes one pixel occupies.
    pub fn bytes_per_pixel(self) -> usize {
        match self {
            Depth::Rgb565 => 2,
            Depth::Bgr888 | Depth::Rgb888 => 3,
            Depth::Bgra8888 | Depth::Rgba8888 => 4,
        }
    }

    /// Bits one pixel occupies, which is what sysfs and `--depth` name it
    /// by.
    pub fn bits(self) -> u32 {
        self.bytes_per_pixel() as u32 * 8
    }

    /// The depth a `bits_per_pixel` value names, or `None` for one this
    /// shell cannot write.
    ///
    /// 24 is the byte order a firmware hands over unless it says
    /// otherwise; [`Geometry::from_sysfs`] is where a framebuffer that
    /// says otherwise is read.
    pub fn from_bits(bits: u32) -> Option<Depth> {
        match bits {
            16 => Some(Depth::Rgb565),
            24 => Some(Depth::Bgr888),
            32 => Some(Depth::Bgra8888),
            _ => None,
        }
    }
}

/// Everything the shell needs to know about the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    /// Visible width in pixels.
    pub width: u16,
    /// Visible height in pixels.
    pub height: u16,
    /// Pixel format.
    pub depth: Depth,
    /// Bytes from the start of one row to the start of the next. Equal to
    /// `width * bytes_per_pixel` on every framebuffer seen so far, but the
    /// kernel is free to pad.
    pub stride: usize,
}

impl Geometry {
    /// Bytes in one converted row, without the padding.
    pub fn row_bytes(self) -> usize {
        usize::from(self.width) * self.depth.bytes_per_pixel()
    }

    /// A centred window of `width` by `height` inside this screen, and the
    /// screen pixel its top-left corner is at. `None` when it is empty or
    /// does not fit. The window keeps the screen's stride.
    pub fn window(self, (width, height): (u16, u16)) -> Option<(Geometry, (u16, u16))> {
        if width == 0 || height == 0 || width > self.width || height > self.height {
            return None;
        }
        let at = ((self.width - width) / 2, (self.height - height) / 2);
        let inside = Geometry {
            width,
            height,
            ..self
        };
        Some((inside, at))
    }

    /// Reads `virtual_size`, `bits_per_pixel` and `stride` from a sysfs
    /// framebuffer directory (`/sys/class/graphics/fb0`).
    ///
    /// Every field must be there and make sense; a framebuffer we cannot
    /// describe is one we will not write to, and the error says which
    /// field it was so that a person reading a boot report can act on it.
    pub fn from_sysfs(dir: &Path) -> Result<Geometry, String> {
        let name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("the framebuffer")
            .to_string();
        let size = read_numbers(&dir.join("virtual_size"))
            .ok_or_else(|| format!("{name} does not say how many pixels it has"))?;
        let (w, h) = match (size.first(), size.get(1)) {
            (Some(&w), Some(&h)) => (w, h),
            _ => return Err(format!("{name} does not say how many pixels it has")),
        };
        let bits = read_numbers(&dir.join("bits_per_pixel"))
            .and_then(|v| v.first().copied())
            .and_then(|b| u32::try_from(b).ok())
            .ok_or_else(|| format!("{name} does not say how deep its pixels are"))?;
        let depth = Depth::from_bits(bits).ok_or_else(|| {
            format!("{name} is {bits} bits per pixel, which this shell cannot write")
        })?;
        let depth = match depth {
            Depth::Bgr888 => three_byte_order(dir),
            Depth::Bgra8888 if is_vc4_fbdev(dir) => Depth::Rgba8888,
            other => other,
        };
        let (width, height) = match (u16::try_from(w), u16::try_from(h)) {
            (Ok(width), Ok(height)) if width != 0 && height != 0 => (width, height),
            _ => {
                return Err(format!(
                    "{name} is {w} by {h} pixels, which is not a screen"
                ));
            }
        };
        let packed = usize::from(width) * depth.bytes_per_pixel();
        // A framebuffer with no `stride` file is assumed packed.
        let stride = read_numbers(&dir.join("stride"))
            .and_then(|v| v.first().copied())
            .and_then(|s| usize::try_from(s).ok())
            .filter(|s| *s >= packed)
            .unwrap_or(packed);
        Ok(Geometry {
            width,
            height,
            depth,
            stride,
        })
    }
}

/// Whether the framebuffer is the Pi's vc4 fbdev emulation, which
/// Faraday's kernel patch makes 32 bits with red first in memory. Its
/// `name` file reads `vc4drmfb`; any other name, or none, is a
/// framebuffer whose 32 bits are blue first.
fn is_vc4_fbdev(dir: &Path) -> bool {
    fs::read_to_string(dir.join("name")).is_ok_and(|n| n.trim() == "vc4drmfb")
}

/// Which way round the three bytes of a 24-bit pixel go.
///
/// A framebuffer driver may publish the bitfield of each channel as
/// `red`, `green` and `blue`, whose first number is the bit offset. The
/// channel at offset 0 is the byte at the lowest address, because every
/// target this runs on is little-endian. With no such files the answer is
/// `simplefb`'s `r8g8b8`: blue at offset 0, so blue first in memory.
fn three_byte_order(dir: &Path) -> Depth {
    let offset = |file: &str| {
        read_numbers(&dir.join(file))
            .and_then(|v| v.first().copied())
            .filter(|o| *o >= 0)
    };
    match (offset("red"), offset("blue")) {
        (Some(red), Some(blue)) if red < blue => Depth::Rgb888,
        _ => Depth::Bgr888,
    }
}

/// The comma-separated numbers on the first line of a sysfs file.
fn read_numbers(path: &Path) -> Option<Vec<i64>> {
    let text = fs::read_to_string(path).ok()?;
    let line = text.lines().next()?;
    let mut out = Vec::new();
    for token in line.split(',') {
        let token = token.trim();
        if !token.is_empty() {
            out.push(token.parse::<i64>().ok()?);
        }
    }
    (!out.is_empty()).then_some(out)
}

/// One opaque pixel as RGB565, little-endian: `rrrrrggg gggbbbbb` stored
/// low byte first.
pub fn rgb565(r: u8, g: u8, b: u8) -> [u8; 2] {
    let packed = ((u16::from(r) & 0xf8) << 8) | ((u16::from(g) & 0xfc) << 3) | (u16::from(b) >> 3);
    packed.to_le_bytes()
}

/// One opaque pixel as BGR888: three bytes, blue at the lowest address,
/// which is what a firmware framebuffer in `r8g8b8` holds.
pub fn bgr888(r: u8, g: u8, b: u8) -> [u8; 3] {
    [b, g, r]
}

/// One opaque pixel as RGB888: three bytes, red at the lowest address.
pub fn rgb888(r: u8, g: u8, b: u8) -> [u8; 3] {
    [r, g, b]
}

/// One opaque pixel as BGRA8888: the byte order the Raspberry Pi's legacy
/// framebuffer wants.
pub fn bgra8888(r: u8, g: u8, b: u8) -> [u8; 4] {
    [b, g, r, 0xff]
}

/// One opaque pixel as RGBA8888: the core's own bytes, alpha opaque.
pub fn rgba8888(r: u8, g: u8, b: u8) -> [u8; 4] {
    [r, g, b, 0xff]
}

/// Costs a pass over every pixel: converts `rgba` (a whole number of
/// pixels) into `out`, which must be exactly `pixels × bytes_per_pixel`
/// long. Each depth is one loop over whole pixels with fixed-size
/// arrays, which the compiler turns into a byte shuffle over many pixels
/// at a time.
pub fn convert_into(rgba: &[u8], depth: Depth, out: &mut [u8]) {
    match depth {
        Depth::Rgb565 => {
            for (s, d) in rgba
                .as_chunks::<4>()
                .0
                .iter()
                .zip(out.as_chunks_mut::<2>().0)
            {
                *d = rgb565(s[0], s[1], s[2]);
            }
        }
        Depth::Bgr888 => {
            for (s, d) in rgba
                .as_chunks::<4>()
                .0
                .iter()
                .zip(out.as_chunks_mut::<3>().0)
            {
                *d = [s[2], s[1], s[0]];
            }
        }
        Depth::Rgb888 => {
            for (s, d) in rgba
                .as_chunks::<4>()
                .0
                .iter()
                .zip(out.as_chunks_mut::<3>().0)
            {
                *d = [s[0], s[1], s[2]];
            }
        }
        Depth::Bgra8888 => {
            for (s, d) in rgba
                .as_chunks::<4>()
                .0
                .iter()
                .zip(out.as_chunks_mut::<4>().0)
            {
                *d = [s[2], s[1], s[0], 0xff];
            }
        }
        Depth::Rgba8888 => {
            for (s, d) in rgba
                .as_chunks::<4>()
                .0
                .iter()
                .zip(out.as_chunks_mut::<4>().0)
            {
                *d = [s[0], s[1], s[2], 0xff];
            }
        }
    }
}

/// The most threads a frame's rows are converted on.
const MOST_THREADS: usize = 8;

/// The fewest rows worth a thread of their own.
const ROWS_PER_THREAD: usize = 64;

/// Costs a pass over every pixel of rows `a..=b` of an unturned frame:
/// converts them into `out` (the whole panel's rows, `convert_turned`'s)
/// and copies them into `last`, the kept frame, in one pass per row. The
/// rows are independent, so they go in bands over threads, the first
/// band on the calling thread.
pub fn convert_keep(
    rgba: &[u8],
    (a, b): (usize, usize),
    depth: Depth,
    fw: usize,
    last: &mut [u8],
    out: &mut [u8],
) {
    let (src_row, out_row) = (fw * 4, fw * depth.bytes_per_pixel());
    let rows = b + 1 - a;
    let n = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(MOST_THREADS)
        .min(rows.div_ceil(ROWS_PER_THREAD))
        .max(1);
    let per = rows.div_ceil(n);
    let src = rgba[a * src_row..(b + 1) * src_row].chunks(per * src_row);
    let kept = last[a * src_row..(b + 1) * src_row].chunks_mut(per * src_row);
    let dst = out[a * out_row..(b + 1) * out_row].chunks_mut(per * out_row);
    let mut bands: Vec<_> = src.zip(kept).zip(dst).collect();
    let work = |((s, k), d): ((&[u8], &mut [u8]), &mut [u8])| {
        for ((s, k), d) in s
            .chunks_exact(src_row)
            .zip(k.chunks_exact_mut(src_row))
            .zip(d.chunks_exact_mut(out_row))
        {
            k.copy_from_slice(s);
            convert_into(s, depth, d);
        }
    };
    let Some(mine) = (!bands.is_empty()).then(|| bands.remove(0)) else {
        return;
    };
    if bands.is_empty() {
        work(mine);
        return;
    }
    std::thread::scope(|scope| {
        for band in bands {
            scope.spawn(move || work(band));
        }
        work(mine);
    });
}

/// The side of the square tiles a turned frame is converted in: a
/// column's writes stay in cache across a tile.
const TILE: usize = 16;

/// Costs a pass over every pixel of rows `a..=b` of the core's frame:
/// converts them into `out`, the whole panel `panel` pixels across and
/// down in its own format, where `turn` puts them (`docs/ORIENTATION.md`
/// §3). `rgba` is the whole frame, `turn.frame_size(panel)` in size. At
/// 0 the rows are the panel's rows, at 180 its rows reversed, and at 90
/// and 270 a band of its columns, walked in tiles of [`TILE`] pixels.
pub fn convert_turned(
    rgba: &[u8],
    (a, b): (usize, usize),
    depth: Depth,
    turn: crate::orient::Turn,
    panel: (u16, u16),
    out: &mut [u8],
) {
    let (fw, _) = turn.frame_size(panel);
    let fw = usize::from(fw);
    if turn == crate::orient::Turn::R0 {
        let bytes = depth.bytes_per_pixel();
        let row = fw * bytes;
        convert_into(
            &rgba[a * fw * 4..(b + 1) * fw * 4],
            depth,
            &mut out[a * row..(b + 1) * row],
        );
        return;
    }
    let walk = Walk {
        rgba,
        fw,
        rows: (a, b),
        turn,
        panel: (usize::from(panel.0), usize::from(panel.1)),
    };
    match depth {
        Depth::Rgb565 => walk.put(out, rgb565),
        Depth::Bgr888 => walk.put(out, bgr888),
        Depth::Rgb888 => walk.put(out, rgb888),
        Depth::Bgra8888 => walk.put(out, bgra8888),
        Depth::Rgba8888 => walk.put(out, rgba8888),
    }
}

/// Rows of the core's frame on their way to a turned panel.
struct Walk<'a> {
    rgba: &'a [u8],
    /// The frame's width in pixels.
    fw: usize,
    /// The first and last row, inclusive.
    rows: (usize, usize),
    turn: crate::orient::Turn,
    /// The panel's width and height in pixels.
    panel: (usize, usize),
}

impl Walk<'_> {
    /// Costs a pass over every pixel of the rows: each converted by
    /// `pixel` and put where the turn puts it, tile by tile.
    fn put<const B: usize>(&self, out: &mut [u8], pixel: fn(u8, u8, u8) -> [u8; B]) {
        let (a, b) = self.rows;
        let pw = self.panel.0;
        for tv in (a..=b).step_by(TILE) {
            for tu in (0..self.fw).step_by(TILE) {
                for v in tv..(tv + TILE).min(b + 1) {
                    let src = &self.rgba[v * self.fw * 4..];
                    for u in tu..(tu + TILE).min(self.fw) {
                        let (x, y) = self.turn.to_panel(self.panel, (u, v));
                        let s = &src[u * 4..u * 4 + 3];
                        let d = (y * pw + x) * B;
                        out[d..d + B].copy_from_slice(&pixel(s[0], s[1], s[2]));
                    }
                }
            }
        }
    }
}

/// Costs a comparison of the two frames: the bands of rows of `next`
/// that differ from `prev`, both frames of `row` bytes a row, each band
/// a first and a last row (inclusive), sorted, with no unchanged row
/// inside one; empty when the two are the same. A `prev` of another size
/// (none yet) makes every row a changed one. Only these rows need
/// writing to the panel: fewer bytes cross to the framebuffer, and a
/// caret and the clock are two bands, not the rows between them.
pub fn changed_rows(prev: &[u8], next: &[u8], row: usize) -> Vec<(usize, usize)> {
    let rows = next.len() / row.max(1);
    if rows == 0 {
        return Vec::new();
    }
    if prev.len() != next.len() {
        return vec![(0, rows - 1)];
    }
    let mut bands: Vec<(usize, usize)> = Vec::new();
    for y in 0..rows {
        if prev[y * row..(y + 1) * row] == next[y * row..(y + 1) * row] {
            continue;
        }
        match bands.last_mut() {
            Some((_, end)) if *end + 1 == y => *end = y,
            _ => bands.push((y, y)),
        }
    }
    bands
}

/// Bands of rows, each a first and last row inclusive, sorted and with
/// the ones that touch or overlap joined, so each row is written once.
pub fn merge_bands(mut bands: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    bands.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::with_capacity(bands.len());
    for (first, last) in bands {
        match merged.last_mut() {
            Some((_, end)) if first <= *end + 1 => *end = (*end).max(last),
            _ => merged.push((first, last)),
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_turned_frame_lands_where_the_map_puts_each_pixel() {
        use crate::orient::Turn;
        // A 5 × 3 panel, every frame pixel a colour of its own.
        let panel = (5u16, 3u16);
        for turn in [Turn::R0, Turn::R90, Turn::R180, Turn::R270] {
            let (fw, fh) = turn.frame_size(panel);
            let (fw, fh) = (usize::from(fw), usize::from(fh));
            let rgba: Vec<u8> = (0..fw * fh)
                .flat_map(|i| [i as u8 * 8, 255 - i as u8, i as u8, 255])
                .collect();
            let mut out = vec![0u8; 5 * 3 * 4];
            convert_turned(&rgba, (0, fh - 1), Depth::Bgra8888, turn, panel, &mut out);
            for v in 0..fh {
                for u in 0..fw {
                    let (x, y) = turn.to_panel((5, 3), (u, v));
                    let s = &rgba[(v * fw + u) * 4..];
                    let d = (y * 5 + x) * 4;
                    assert_eq!(
                        out[d..d + 4],
                        bgra8888(s[0], s[1], s[2]),
                        "{turn:?} {u},{v}"
                    );
                }
            }
        }
    }

    #[test]
    fn bands_that_touch_are_written_as_one() {
        assert_eq!(
            merge_bands(vec![(40, 58), (10, 20), (21, 30), (50, 60)]),
            vec![(10, 30), (40, 60)]
        );
        assert_eq!(merge_bands(Vec::new()), Vec::<(usize, usize)>::new());
    }

    #[test]
    fn only_the_rows_that_changed_are_written() {
        // Four rows of two 16-bit pixels.
        let prev = [0u8; 16];
        let mut next = prev;
        assert_eq!(changed_rows(&prev, &next, 4), vec![]);
        next[5] = 1;
        assert_eq!(changed_rows(&prev, &next, 4), vec![(1, 1)]);
        next[13] = 1;
        assert_eq!(changed_rows(&prev, &next, 4), vec![(1, 1), (3, 3)]);
        next[9] = 1;
        assert_eq!(changed_rows(&prev, &next, 4), vec![(1, 3)]);
        next[0] = 1;
        assert_eq!(changed_rows(&prev, &next, 4), vec![(0, 3)]);
        // Nothing drawn before: every row is new.
        assert_eq!(changed_rows(&[], &next, 4), vec![(0, 3)]);
        assert_eq!(changed_rows(&prev, &[], 4), vec![]);
    }

    /// A sysfs framebuffer directory of this test's own. The last path
    /// element is `fb0`, because that is the name the reasons carry.
    fn dir(tag: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("faraday-pi-fb-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join("fb0");
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    /// Takes the directory `dir` made away again.
    fn clean(dir: &std::path::Path) {
        std::fs::remove_dir_all(dir.parent().expect("a parent")).expect("clean up");
    }

    #[test]
    fn geometry_comes_from_sysfs() {
        let dir = dir("fb0");
        std::fs::write(dir.join("virtual_size"), "480,640\n").expect("write");
        std::fs::write(dir.join("bits_per_pixel"), "16\n").expect("write");
        std::fs::write(dir.join("stride"), "960\n").expect("write");
        assert_eq!(
            Geometry::from_sysfs(&dir),
            Ok(Geometry {
                width: 480,
                height: 640,
                depth: Depth::Rgb565,
                stride: 960,
            })
        );
        // The legacy firmware path: 32 bits, 1920-byte rows.
        std::fs::write(dir.join("bits_per_pixel"), "32\n").expect("write");
        std::fs::write(dir.join("stride"), "1920\n").expect("write");
        assert_eq!(
            Geometry::from_sysfs(&dir),
            Ok(Geometry {
                width: 480,
                height: 640,
                depth: Depth::Bgra8888,
                stride: 1920,
            })
        );
        clean(&dir);
    }

    #[test]
    fn the_pis_thirty_two_bit_framebuffer_is_red_first_and_any_other_is_blue_first() {
        let dir = dir("fb32");
        std::fs::write(dir.join("virtual_size"), "1920,1080\n").expect("write");
        std::fs::write(dir.join("bits_per_pixel"), "32\n").expect("write");
        std::fs::write(dir.join("stride"), "7680\n").expect("write");
        let depth = |dir: &std::path::Path| Geometry::from_sysfs(dir).map(|g| g.depth);
        assert_eq!(depth(&dir), Ok(Depth::Bgra8888), "no name");
        std::fs::write(dir.join("name"), "EFI VGA\n").expect("write");
        assert_eq!(depth(&dir), Ok(Depth::Bgra8888), "another driver");
        std::fs::write(dir.join("name"), "vc4drmfb\n").expect("write");
        assert_eq!(depth(&dir), Ok(Depth::Rgba8888));
        // Only at 32 bits.
        std::fs::write(dir.join("bits_per_pixel"), "16\n").expect("write");
        assert_eq!(depth(&dir), Ok(Depth::Rgb565));
        clean(&dir);
    }

    #[test]
    fn a_twenty_four_bit_framebuffer_is_blue_first_unless_it_says_otherwise() {
        let dir = dir("fb24");
        std::fs::write(dir.join("virtual_size"), "1366,768\n").expect("write");
        std::fs::write(dir.join("bits_per_pixel"), "24\n").expect("write");
        assert_eq!(
            Geometry::from_sysfs(&dir),
            Ok(Geometry {
                width: 1366,
                height: 768,
                depth: Depth::Bgr888,
                stride: 1366 * 3,
            })
        );
        // A driver that publishes its bitfields is believed: red at the
        // bottom of the pixel is red at the lowest address.
        std::fs::write(dir.join("red"), "0,8\n").expect("write");
        std::fs::write(dir.join("green"), "8,8\n").expect("write");
        std::fs::write(dir.join("blue"), "16,8\n").expect("write");
        assert_eq!(
            Geometry::from_sysfs(&dir).map(|g| g.depth),
            Ok(Depth::Rgb888)
        );
        clean(&dir);
    }

    #[test]
    fn a_depth_this_shell_cannot_write_says_so() {
        let dir = dir("fb30");
        std::fs::write(dir.join("virtual_size"), "1920,1080\n").expect("write");
        std::fs::write(dir.join("bits_per_pixel"), "30\n").expect("write");
        assert_eq!(
            Geometry::from_sysfs(&dir),
            Err(String::from(
                "fb0 is 30 bits per pixel, which this shell cannot write"
            ))
        );
        clean(&dir);
    }

    #[test]
    fn a_row_of_pixels_converts_at_twenty_four_bits() {
        // Red, green, blue and white, premultiplied and opaque.
        let row = [
            0xff, 0x00, 0x00, 0xff, 0x00, 0xff, 0x00, 0xff, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff,
        ];
        let mut out = [0u8; 12];
        convert_into(&row, Depth::Bgr888, &mut out);
        assert_eq!(
            out,
            [
                0x00, 0x00, 0xff, 0x00, 0xff, 0x00, 0xff, 0x00, 0x00, 0xff, 0xff, 0xff
            ]
        );
        convert_into(&row, Depth::Rgb888, &mut out);
        assert_eq!(
            out,
            [
                0xff, 0x00, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff
            ]
        );
    }
}
