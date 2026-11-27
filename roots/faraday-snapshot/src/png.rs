//! The picture written at each step, with no metadata.

use std::io;
use std::path::Path;

const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
/// The most a stored deflate block holds.
const BLOCK: usize = 65_535;

/// Writes `rgba` (`width` by `height`) as a PNG with no text, time or colour-profile chunk.
pub fn write(path: &Path, width: u32, height: u32, rgba: &[u8]) -> io::Result<()> {
    std::fs::write(path, encode(width, height, rgba)?)
}

/// The file's bytes: the signature, `IHDR`, one `IDAT` of stored deflate blocks, `IEND`.
///
/// The frame is premultiplied (the port's `Frame`), and the picture is of an opaque panel, so it
/// is written as 8-bit RGB: the alpha byte is dropped.
fn encode(width: u32, height: u32, rgba: &[u8]) -> io::Result<Vec<u8>> {
    let (w, h) = (width as usize, height as usize);
    if w == 0 || h == 0 || rgba.len() != w * h * 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a picture is width x height x 4 bytes and not empty",
        ));
    }
    let mut raw = Vec::with_capacity(h * (1 + w * 3));
    for row in rgba.chunks_exact(w * 4) {
        raw.push(0);
        for px in row.chunks_exact(4) {
            raw.extend_from_slice(&px[..3]);
        }
    }
    let mut out = Vec::from(SIGNATURE);
    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(&mut out, *b"IHDR", &header);
    chunk(&mut out, *b"IDAT", &zlib(&raw));
    chunk(&mut out, *b"IEND", &[]);
    Ok(out)
}

fn chunk(out: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
    let len = u32::try_from(data.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&len.to_be_bytes());
    let mut body = Vec::with_capacity(4 + data.len());
    body.extend_from_slice(&kind);
    body.extend_from_slice(data);
    out.extend_from_slice(&body);
    out.extend_from_slice(&crc32(&body).to_be_bytes());
}

/// A zlib stream of stored blocks: no compression, so no table to get wrong.
fn zlib(raw: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let mut blocks = raw.chunks(BLOCK).peekable();
    if blocks.peek().is_none() {
        out.extend_from_slice(&[1, 0, 0, 0xFF, 0xFF]);
    }
    while let Some(block) = blocks.next() {
        out.push(u8::from(blocks.peek().is_none()));
        let len = u16::try_from(block.len()).unwrap_or(u16::MAX);
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(block);
    }
    out.extend_from_slice(&adler32(raw).to_be_bytes());
    out
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                0xEDB8_8320 ^ (crc >> 1)
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn adler32(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in bytes {
        a = (a + u32::from(byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Invariant: A picture a tour saves carries its pixels and no metadata: no text, time or colour-profile chunk.
    /// Regression: The writer gains a text or time chunk, so a picture names the machine or the hour it was made.
    /// Owner: root
    #[test]
    fn a_picture_has_its_pixels_and_no_metadata_chunk() {
        let rgba = [10u8, 20, 30, 255, 40, 50, 60, 255];
        let bytes = encode(2, 1, &rgba).unwrap_or_default();
        assert_eq!(bytes[..8], SIGNATURE);
        let mut kinds = Vec::new();
        let mut at = 8;
        while at < bytes.len() {
            let len = u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
            kinds.push(String::from_utf8_lossy(&bytes[at + 4..at + 8]).into_owned());
            at += 12 + len as usize;
        }
        assert_eq!(kinds, ["IHDR", "IDAT", "IEND"]);
        // The stored pixels: filter byte 0, then the two RGB triples.
        let idat = bytes.windows(6).position(|w| w == [0, 10, 20, 30, 40, 50]);
        assert!(idat.is_some());
    }

    /// Invariant: A frame whose size does not match its pixels is refused, not written as a broken picture.
    /// Regression: The writer trusts the width and height and writes a file no viewer opens.
    /// Owner: root
    #[test]
    fn a_frame_of_the_wrong_size_is_refused() {
        assert!(encode(2, 2, &[0; 4]).is_err());
    }
}
