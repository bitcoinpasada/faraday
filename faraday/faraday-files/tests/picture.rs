//! A picture on a stick is read for QR codes in the disk process, which
//! a stick's maker can hand any PNG: one that says it is larger than
//! `MAX_PIXELS` is refused from its header, before its pixels are
//! decoded. A code is read whether it is printed dark on light or light
//! on dark (a screenshot of a phone in dark mode).

use faraday_codec::qr::{self, Ecc, Payload, QUIET_ZONE};
use faraday_files::qr_in_png;

/// A token for the work these tests ask, counting nothing. Cheap.
#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-files' tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> faraday_crypto::work::Compute {
    faraday_crypto::work::Compute::new(|_| {})
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut body = kind.to_vec();
    body.extend_from_slice(data);
    out.extend_from_slice(&body);
    out.extend_from_slice(&faraday_crypto::crc32(&body).to_be_bytes());
}

/// A grey PNG whose header says `w` × `h`, with a few bytes of picture
/// data: what it would take to decode it is the header's to say.
fn claims(w: u32, h: u32) -> Vec<u8> {
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 0, 0, 0, 0]);
    chunk(&mut png, b"IHDR", &ihdr);
    // A zlib stream of one stored block: one row's filter byte and a
    // pixel, far short of the picture.
    chunk(
        &mut png,
        b"IDAT",
        &[0x78, 0x01, 0x01, 0x02, 0x00, 0xFD, 0xFF, 0x00, 0x00],
    );
    chunk(&mut png, b"IEND", &[]);
    png
}

#[test]
fn a_picture_larger_than_the_limit_is_refused_from_its_header() {
    let c = compute();
    for (w, h) in [(4097, 4096), (4096, 4097), (65_535, 65_535), (100_000, 200)] {
        assert_eq!(
            qr_in_png(&c, &claims(w, h)),
            Err("the image is too large".to_string()),
            "{w} × {h}"
        );
    }
}

#[test]
fn a_picture_at_the_limit_is_decoded() {
    let c = compute();
    // Its pixels are read, and the few bytes there are run out.
    let got = qr_in_png(&c, &claims(4096, 4096));
    assert!(
        matches!(&got, Err(e) if e != "the image is too large"),
        "{got:?}"
    );
}

/// A picture of one code holding `payload`, four pixels a module, dark
/// on light or, `light`, light on dark.
fn code_png(payload: &[u8], light: bool) -> Vec<u8> {
    let c = compute();
    let m = qr::encode(&c, Payload::Bytes(payload), Ecc::Low).expect("a code");
    let (w, h, mut px) = m.to_luma(4, QUIET_ZONE);
    if light {
        for p in &mut px {
            *p = 255 - *p;
        }
    }
    faraday_codec::png::grey_png(w, h, &px)
}

#[test]
fn a_code_is_read_dark_on_light_and_light_on_dark() {
    let c = compute();
    let payload = b"xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz";
    for light in [false, true] {
        assert_eq!(
            qr_in_png(&c, &code_png(payload, light)),
            Ok(vec![payload.to_vec()]),
            "light on dark: {light}"
        );
    }
}
