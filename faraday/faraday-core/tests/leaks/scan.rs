//! A scanned code's copies: a SeedQR read out of a camera frame by the
//! scanner, and a frame holding a code shrunk by the camera shells. What
//! is freed unwiped on the test thread is found by the allocator; the
//! decoder's stack buffers are not seen by it.

use crate::stick_visit::digits;
use crate::{arm, finish};
use faraday_codec::qr::{self, Ecc, Payload};
use faraday_core::testkit;
use faraday_leak_scan::Form;
use zeroize::Zeroize;

/// The digits and the forms they are looked for in: all of them, and
/// the first 60, which is what the decoder's 64-byte block holds while
/// the payload grows.
fn forms(digits: &[u8]) -> Vec<Form> {
    vec![
        Form::new("SeedQR digits", digits.to_vec()),
        Form::new("SeedQR digits, first 60", digits[..60].to_vec()),
    ]
}

/// A SeedQR of the test seed, scanned the way a camera frame is, leaves
/// no freed copy of its digits.
#[test]
fn a_seedqr_scanned_from_a_frame_leaves_no_freed_copy() {
    let digits = std::mem::ManuallyDrop::new(digits());
    let c = testkit::compute();
    let m = qr::encode(&c, Payload::Numeric(&digits), Ecc::Low).expect("fits a code");
    let (width, height, luma) = m.to_luma(4, 4);
    let wanted = forms(&digits);
    let armed = arm(&wanted);
    let (read, _) = faraday_scanner::look(width, height, &luma, 1, (width as u16, height as u16));
    let mut read = read.expect("the code reads");
    assert_eq!(read, *digits, "the code read as made");
    read.zeroize();
    drop(read);
    finish("a SeedQR scanned from a frame", armed, false);
}

/// A full-size frame whose first row holds a code's digits, shrunk by an
/// integer factor, leaves no freed copy of the row.
#[test]
fn a_frame_holding_a_code_shrunk_leaves_no_freed_copy() {
    let digits = std::mem::ManuallyDrop::new(digits());
    let (width, height) = (128usize, 8usize);
    let mut luma = vec![255u8; width * height];
    luma[..digits.len()].copy_from_slice(&digits);
    let wanted = forms(&digits);
    let armed = arm(&wanted);
    let (w, h, mut small) = faraday_camera::scale::downscale(luma, width, height, 2);
    assert_eq!((w, h), (64, 4));
    small.zeroize();
    drop(small);
    finish("a frame holding a code shrunk", armed, false);
}
