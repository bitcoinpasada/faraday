//! QR detection and decoding from a camera frame.
//!
//! Backed by `rqrr` (`docs/deps/rqrr.md`), which needs `std`; this module
//! is behind the crate's `decode` feature and is the only `std` code in
//! the core.

use alloc::vec::Vec;

use faraday_crypto::work::{Compute, Work};
use zeroize::Zeroizing;

use crate::classify::{PayloadKind, classify};

/// One QR code found in a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    /// The payload bytes: digits for numeric mode, the text's bytes for
    /// alphanumeric, the raw bytes for byte mode. Wiped when dropped: a
    /// code read may be a SeedQR.
    pub bytes: Zeroizing<Vec<u8>>,
    /// What [`classify`] makes of the bytes.
    pub kind: PayloadKind,
}

/// Costs a thresholding and a search of the image, counted once its
/// size holds: finds and decodes every QR code in an 8-bit luma image of
/// `width × height` pixels (row-major, one byte per pixel). Codes whose
/// error correction fails are skipped. A frame with the wrong number of
/// bytes yields nothing.
pub fn decode_luma(c: &Compute, width: usize, height: usize, luma: &[u8]) -> Vec<Decoded> {
    if width
        .checked_mul(height)
        .is_none_or(|n| n == 0 || n != luma.len())
    {
        return Vec::new();
    }
    c.count(Work::QrDecode);
    let mut image =
        rqrr::PreparedImage::prepare_from_greyscale(width, height, |x, y| luma[y * width + x]);
    read(image.detect_grids())
}

/// Costs one or two thresholdings and searches of the picture (below):
/// the codes in a still picture of `width × height` luma pixels, those
/// printed dark on light, or, when it holds none, those printed light on
/// dark (a screenshot of a phone in dark mode). The picture is read once;
/// a picture with no dark-on-light code is searched a second time,
/// inverted. A camera frame is [`decode_luma`]'s, its shell inverting
/// every other frame. Costs one or two thresholdings and searches of the
/// picture: a third of a second for a 12-megapixel photograph on a
/// desktop, seconds on a Pi.
pub fn decode_picture(c: &Compute, width: usize, height: usize, luma: &[u8]) -> Vec<Decoded> {
    if width
        .checked_mul(height)
        .is_none_or(|n| n == 0 || n != luma.len())
    {
        return Vec::new();
    }
    c.count(Work::QrDecode);
    let mut image =
        rqrr::PreparedImage::prepare_from_greyscale(width, height, |x, y| luma[y * width + x]);
    let found = read(image.detect_grids());
    drop(image);
    if !found.is_empty() {
        return found;
    }
    let mut inverse = rqrr::PreparedImage::prepare_from_greyscale(width, height, |x, y| {
        255 - luma[y * width + x]
    });
    read(inverse.detect_grids())
}

/// Every grid the search found whose error correction holds, decoded.
fn read<G: rqrr::BitGrid>(grids: Vec<rqrr::Grid<G>>) -> Vec<Decoded> {
    let mut out = Vec::new();
    for grid in grids {
        // What a grid decodes to is wiped, whole or not.
        let mut bytes = Zeroizing::new(Vec::new());
        if grid.decode_to(&mut *bytes).is_err() {
            continue;
        }
        let kind = classify(&bytes);
        out.push(Decoded { bytes, kind });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qr::{self, Ecc, Payload, QUIET_ZONE};

    fn round_trip(payload: Payload<'_>, expect: &[u8]) -> PayloadKind {
        let c = Compute::unchecked();
        let m = qr::encode(&c, payload, Ecc::Low).unwrap();
        let (w, h, px) = m.to_luma(4, QUIET_ZONE);
        let found = decode_luma(&c, w, h, &px);
        assert_eq!(found.len(), 1, "one code in the frame");
        assert_eq!(found[0].bytes.as_slice(), expect);
        found[0].kind.clone()
    }

    #[test]
    fn encoder_and_decoder_agree() {
        let digits = [b'0'; 48];
        assert_eq!(
            round_trip(Payload::Numeric(&digits), &digits),
            PayloadKind::SeedQr { words: 12 }
        );
        let entropy = [
            0x5bu8, 0xbd, 0x9d, 0x71, 0xa8, 0xec, 0x79, 0x90, 0x83, 0x1a, 0xff, 0x35, 0x9d, 0x42,
            0x65, 0x45,
        ];
        assert_eq!(
            round_trip(Payload::Bytes(&entropy), &entropy),
            PayloadKind::CompactSeedQr { words: 12 }
        );
        // Bytes that include NUL, CR and LF survive byte mode.
        let tricky = b"\x00\r\n\xf5\\\xf5\x87\xf2T=\x01\t\r\n\xe7\x10";
        assert_eq!(
            round_trip(Payload::Bytes(tricky), tricky),
            PayloadKind::CompactSeedQr { words: 12 }
        );
        let psbt =
            b"cHNidP8BAHECAAAAAYVck7r58DUBsfm8sd4nwgC/MxICFftj1+0KIsRhSFiaAAAAAAD9////AmDqAAAA";
        assert_eq!(round_trip(Payload::Bytes(psbt), psbt), PayloadKind::Psbt);
        let xpub = b"xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz";
        assert_eq!(round_trip(Payload::Bytes(xpub), xpub), PayloadKind::Xpub);
        let addr = b"BC1QCR8TE4KR609GCAWUTMRZA0J4XV80JY8Z306FYU";
        assert_eq!(
            round_trip(Payload::Alphanumeric(addr), addr),
            PayloadKind::Address
        );
    }

    #[test]
    fn large_codes_decode_too() {
        let c = Compute::unchecked();
        let big: Vec<u8> = (0..2000u32).map(|i| b'a' + (i % 26) as u8).collect();
        let m = qr::encode(&c, Payload::Bytes(&big), Ecc::Low).unwrap();
        assert!(m.version() >= 30);
        let (w, h, px) = m.to_luma(3, QUIET_ZONE);
        let found = decode_luma(&c, w, h, &px);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].bytes.as_slice(), big);
    }

    #[test]
    fn empty_or_wrong_sized_frames_yield_nothing() {
        let c = Compute::unchecked();
        assert!(decode_luma(&c, 0, 0, &[]).is_empty());
        assert!(decode_luma(&c, 10, 10, &[0; 50]).is_empty());
        assert!(decode_luma(&c, 64, 64, &[128; 64 * 64]).is_empty());
    }

    /// A picture whose stated size overflows is refused, not wrapped
    /// round to the length of the bytes it came with.
    #[test]
    fn a_frame_whose_size_overflows_yields_nothing() {
        let c = Compute::unchecked();
        let wide = usize::MAX / 2 + 1;
        assert!(decode_luma(&c, wide, 2, &[]).is_empty());
        assert!(decode_picture(&c, 2, wide, &[]).is_empty());
    }
}
