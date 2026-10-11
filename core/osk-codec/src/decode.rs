//! QR detection and decoding from a camera frame (`docs/PLANNING.md`
//! §4.4: the shell decodes on a worker beside the core, which keeps the
//! preview and the routing; §16.36).
//!
//! Backed by `rqrr` (`docs/deps/rqrr.md`), which needs `std`; this module
//! is behind the crate's `decode` feature and is the only `std` code in
//! the core (`docs/PLANNING.md` §16.20).

use alloc::vec::Vec;

use crate::classify::{PayloadKind, classify};

/// One QR code found in a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    /// The payload bytes: digits for numeric mode, the text's bytes for
    /// alphanumeric, the raw bytes for byte mode.
    pub bytes: Vec<u8>,
    /// What [`classify`] makes of the bytes.
    pub kind: PayloadKind,
}

/// Finds and decodes every QR code in an 8-bit luma image of
/// `width × height` pixels (row-major, one byte per pixel). Codes whose
/// error correction fails are skipped. A frame with the wrong number of
/// bytes yields nothing.
pub fn decode_luma(width: usize, height: usize, luma: &[u8]) -> Vec<Decoded> {
    if width == 0 || height == 0 || luma.len() != width * height {
        return Vec::new();
    }
    let mut image =
        rqrr::PreparedImage::prepare_from_greyscale(width, height, |x, y| luma[y * width + x]);
    let mut out = Vec::new();
    for grid in image.detect_grids() {
        let mut bytes = Vec::new();
        if grid.decode_to(&mut bytes).is_err() {
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
        let m = qr::encode(payload, Ecc::Low).unwrap();
        let (w, h, px) = m.to_luma(4, QUIET_ZONE);
        let found = decode_luma(w, h, &px);
        assert_eq!(found.len(), 1, "one code in the frame");
        assert_eq!(found[0].bytes, expect);
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
        let big: Vec<u8> = (0..2000u32).map(|i| b'a' + (i % 26) as u8).collect();
        let m = qr::encode(Payload::Bytes(&big), Ecc::Low).unwrap();
        assert!(m.version() >= 30);
        let (w, h, px) = m.to_luma(3, QUIET_ZONE);
        let found = decode_luma(w, h, &px);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].bytes, big);
    }

    #[test]
    fn empty_or_wrong_sized_frames_yield_nothing() {
        assert!(decode_luma(0, 0, &[]).is_empty());
        assert!(decode_luma(10, 10, &[0; 50]).is_empty());
        assert!(decode_luma(64, 64, &[128; 64 * 64]).is_empty());
    }
}
