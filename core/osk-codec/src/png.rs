//! A QR code as a PNG file (`docs/PLANNING.md` §16.134 rule 5): black
//! modules on white, the four-module quiet zone round them, each module
//! a square of whole pixels.
//!
//! The image is one-bit greyscale, so a row is a filter byte and one
//! bit per pixel. The pixels are deflated in stored blocks, which is
//! what deflate allows for data it does not compress: a one-bit image
//! of a code is a few kilobytes, and a file written this way needs no
//! compressor and no new crate. The chunks carry the CRC-32 the fountain
//! code already has, and the zlib stream its Adler-32.

use alloc::vec::Vec;

use crate::qr::{QUIET_ZONE, QrMatrix};
use crate::ur::bytewords::crc32;

/// The eight bytes every PNG file starts with.
const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// The most bytes one stored deflate block holds.
const STORED_MAX: usize = 0xFFFF;

/// `matrix` as a PNG file, each module `scale` × `scale` pixels, with
/// [`QUIET_ZONE`] light modules on every side. A `scale` of 0 is taken
/// as 1.
pub fn qr_png(matrix: &QrMatrix, scale: usize) -> Vec<u8> {
    let scale = scale.max(1);
    let side = (matrix.size() + 2 * QUIET_ZONE) * scale;
    let row_len = side.div_ceil(8);
    // Every row: filter type 0 (none), then the pixels, a set bit for
    // white, most significant bit first. The bits past the last pixel
    // are white too.
    let mut raw = Vec::with_capacity(side * (row_len + 1));
    for py in 0..side {
        raw.push(0);
        let start = raw.len();
        raw.resize(start + row_len, 0xFF);
        let y = (py / scale).checked_sub(QUIET_ZONE);
        for px in 0..side {
            let x = (px / scale).checked_sub(QUIET_ZONE);
            let dark = match (x, y) {
                (Some(x), Some(y)) if x < matrix.size() && y < matrix.size() => matrix.module(x, y),
                _ => false,
            };
            if dark {
                raw[start + px / 8] &= !(0x80 >> (px % 8));
            }
        }
    }

    let mut out = Vec::with_capacity(raw.len() + raw.len() / STORED_MAX * 5 + 128);
    out.extend_from_slice(&SIGNATURE);

    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&(side as u32).to_be_bytes());
    header.extend_from_slice(&(side as u32).to_be_bytes());
    // Bit depth 1, colour type 0 (greyscale), deflate, the one filter
    // method, no interlace.
    header.extend_from_slice(&[1, 0, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &header);

    chunk(&mut out, b"IDAT", &zlib_stored(&raw));
    chunk(&mut out, b"IEND", &[]);
    out
}

/// `data` as a zlib stream of stored deflate blocks.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let blocks = data.len().div_ceil(STORED_MAX).max(1);
    let mut out = Vec::with_capacity(data.len() + blocks * 5 + 6);
    // CMF: deflate with a 32 KiB window; FLG: no dictionary, the check
    // bits that make the pair a multiple of 31.
    out.extend_from_slice(&[0x78, 0x01]);
    if data.is_empty() {
        out.extend_from_slice(&[1, 0, 0, 0xFF, 0xFF]);
    }
    let mut chunks = data.chunks(STORED_MAX).peekable();
    while let Some(block) = chunks.next() {
        let last = chunks.peek().is_none();
        let len = block.len() as u16;
        out.push(u8::from(last));
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(block);
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

/// One chunk: its length, its type, its data and the CRC-32 of the
/// type and the data.
fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let from = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let crc = crc32(&out[from..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

/// Adler-32 (RFC 1950 §8.2).
fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65_521;
    let (mut a, mut b) = (1u32, 0u32);
    // 5552 bytes is the most that can be summed before `b` can pass
    // `u32::MAX`.
    for block in data.chunks(5552) {
        for &byte in block {
            a += u32::from(byte);
            b += a;
        }
        a %= MOD;
        b %= MOD;
    }
    (b << 16) | a
}
