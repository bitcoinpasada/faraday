//! What an EDID base block says of a screen's size (`docs/ORIENTATION.md`
//! §2): the native pixels, how large it is, and so how dense.

use faraday_core::edid::{Size, size};

/// A 7-inch 800 × 1280 handheld's panel, as captured in the public EDID
/// database `linuxhw/EDID` (`Digital/Valve/VLV3001`). Its detailed timing
/// gives 60 × 60 mm, which is not the shape of its pixels; its screen
/// size is 10 × 15 cm.
const HANDHELD: &str = "\
00 ff ff ff ff ff ff 00 59 96 01 30 01 00 00 00
1e 1c 01 04 a5 0a 0f 78 16 00 00 00 00 00 00 00
00 00 00 00 00 00 01 00 01 00 01 00 01 00 01 00
01 00 01 00 01 00 0f 1b 20 48 30 00 2c 50 20 14
02 04 3c 3c 00 00 00 1e 00 00 00 fc 00 41 4e 58
37 35 33 30 20 55 0a 20 20 20 00 00 00 10 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 10
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 93";

fn bytes(hex: &str) -> Vec<u8> {
    hex.split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).expect("hex"))
        .collect()
}

/// A base block whose first detailed timing is `pixels` and `mm`, with a
/// screen size of `cm`.
fn block(pixels: (u16, u16), mm: (u16, u16), cm: (u8, u8)) -> Vec<u8> {
    let mut b = vec![0u8; 128];
    b[..8].copy_from_slice(&[0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0]);
    b[21] = cm.0;
    b[22] = cm.1;
    let d = &mut b[54..72];
    d[0] = 0x01; // a pixel clock: a timing
    d[2] = pixels.0 as u8;
    d[4] = ((pixels.0 >> 8) as u8) << 4;
    d[5] = pixels.1 as u8;
    d[7] = ((pixels.1 >> 8) as u8) << 4;
    d[12] = mm.0 as u8;
    d[13] = mm.1 as u8;
    d[14] = (((mm.0 >> 8) as u8) << 4) | ((mm.1 >> 8) as u8 & 0x0f);
    b
}

#[test]
fn a_captured_handheld_panel_is_portrait_seven_inches_and_dense() {
    let s = size(&bytes(HANDHELD)).expect("an EDID");
    assert_eq!(s.pixels, Some((800, 1280)), "native portrait");
    // The detailed timing's 60 × 60 mm is not the panel's shape: the
    // screen size is read instead.
    assert_eq!(s.mm, Some((100, 150)));
    let inches = f64::from(s.diagonal_tenth_mm().expect("a size")) / 254.0;
    assert!((7.0..7.2).contains(&inches), "{inches}");
    assert_eq!(s.dpi(), Some(213));
}

#[test]
fn a_panel_is_as_dense_as_its_pixels_over_its_inches() {
    // A 7-inch 1024 × 600 panel, landscape natively.
    let s = size(&block((1024, 600), (154, 90), (15, 9))).expect("an EDID");
    assert_eq!(s.pixels, Some((1024, 600)));
    assert_eq!(s.mm, Some((154, 90)), "the timing's size, the finer one");
    assert_eq!(s.dpi(), Some(169));
    // A 10-inch 800 × 1280 panel, portrait natively.
    let s = size(&block((800, 1280), (135, 216), (14, 22))).expect("an EDID");
    assert_eq!(s.dpi(), Some(150));
}

#[test]
fn a_block_without_a_size_gives_no_density() {
    // A projector or a capture device: no size at all.
    let s = size(&block((1920, 1080), (0, 0), (0, 0))).expect("an EDID");
    assert_eq!(
        s,
        Size {
            pixels: Some((1920, 1080)),
            mm: None
        }
    );
    assert_eq!(s.dpi(), None);
    // No detailed timing first: no pixels, the size still read.
    let mut b = block((0, 0), (0, 0), (30, 20));
    b[54] = 0;
    let s = size(&b).expect("an EDID");
    assert_eq!(s.pixels, None);
    assert_eq!(s.mm, Some((300, 200)));
    assert_eq!(s.dpi(), None);
    // Not an EDID.
    assert_eq!(size(&[0u8; 128]), None);
    assert_eq!(size(&[]), None);
}
