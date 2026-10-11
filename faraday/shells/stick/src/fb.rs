//! The framebuffer: its geometry, and the conversion from the core's
//! premultiplied RGBA8888 frame to what the panel wants.
//!
//! Four formats are supported:
//!
//! - **RGB565**, 16 bits per pixel, little-endian, red in the top five
//!   bits. This is what the DRM/KMS path reports, and what the Pi's panel
//!   is (the SeedSigner fork's `hardware/DPI28.py` writes the same).
//! - **BGR888** and **RGB888**, 24 bits per pixel, three bytes and no
//!   pad. A UEFI firmware that hands over a 24-bit mode is the reason
//!   these are here.
//! - **BGRA8888**, 32 bits per pixel, blue first, alpha forced opaque.
//!   This is what the legacy firmware framebuffer reports.
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
}

impl Depth {
    /// Bytes one pixel occupies.
    pub fn bytes_per_pixel(self) -> usize {
        match self {
            Depth::Rgb565 => 2,
            Depth::Bgr888 | Depth::Rgb888 => 3,
            Depth::Bgra8888 => 4,
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

/// Converts `rgba` (a whole number of pixels) into `out`, which must be
/// exactly `pixels × bytes_per_pixel` long.
pub fn convert_into(rgba: &[u8], depth: Depth, out: &mut [u8]) {
    match depth {
        Depth::Rgb565 => {
            for (src, dst) in rgba.chunks_exact(4).zip(out.chunks_exact_mut(2)) {
                dst.copy_from_slice(&rgb565(src[0], src[1], src[2]));
            }
        }
        Depth::Bgr888 => {
            for (src, dst) in rgba.chunks_exact(4).zip(out.chunks_exact_mut(3)) {
                dst.copy_from_slice(&bgr888(src[0], src[1], src[2]));
            }
        }
        Depth::Rgb888 => {
            for (src, dst) in rgba.chunks_exact(4).zip(out.chunks_exact_mut(3)) {
                dst.copy_from_slice(&rgb888(src[0], src[1], src[2]));
            }
        }
        Depth::Bgra8888 => {
            for (src, dst) in rgba.chunks_exact(4).zip(out.chunks_exact_mut(4)) {
                dst.copy_from_slice(&bgra8888(src[0], src[1], src[2]));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sysfs framebuffer directory of this test's own. The last path
    /// element is `fb0`, because that is the name the reasons carry.
    fn dir(tag: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("osk-pi-fb-{}-{tag}", std::process::id()));
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
