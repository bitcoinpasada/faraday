//! A camera frame is the untrusted side's: a crafted one may be any
//! picture, and the scanner must end it in bounded time, with no read,
//! the same as any frame it cannot read. (The scanner's stop waits for
//! the frame being decoded.)

use faraday_codec::decode_luma;
use faraday_codec::qr::{self, Ecc, Payload};
use faraday_crypto::work::Compute;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const W: usize = 1920;
const H: usize = 1080;

/// Decodes on another thread and waits up to `limit` for it: the time
/// the decode took and the number of codes it read, or `None` when it
/// did not end. The limit is far over what a loaded machine needs, so
/// only a stall misses it.
fn decode_within(frame: Vec<u8>, limit: Duration) -> Option<(Duration, usize)> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let c = Compute::unchecked();
        let t = Instant::now();
        let n = decode_luma(&c, W, H, &frame).len();
        let _ = tx.send((t.elapsed(), n));
    });
    rx.recv_timeout(limit).ok()
}

/// Squares laid out in a lattice, each the 1:1:3:1:1 ring of a code's
/// corner mark, seven pixels wide, `pitch` pixels apart.
fn capstone_lattice(pitch: usize) -> Vec<u8> {
    let mut img = vec![235u8; W * H];
    let mut y0 = 1;
    while y0 + 7 <= H {
        let mut x0 = 1;
        while x0 + 7 <= W {
            for dy in 0..7 {
                for dx in 0..7 {
                    let ring = dx == 0 || dx == 6 || dy == 0 || dy == 6;
                    let core = (2..=4).contains(&dx) && (2..=4).contains(&dy);
                    if ring || core {
                        img[(y0 + dy) * W + x0 + dx] = 25;
                    }
                }
            }
            x0 += pitch;
        }
        y0 += pitch;
    }
    img
}

#[test]
fn a_frame_of_tens_of_thousands_of_corner_marks_ends_without_a_read() {
    let frame = capstone_lattice(8);
    let (_, read) = decode_within(frame, Duration::from_secs(120))
        .expect("the frame was still being decoded after two minutes");
    assert_eq!(read, 0);
}

#[test]
fn a_real_code_still_reads_beside_a_few_stray_corner_marks() {
    let c = Compute::unchecked();
    let code = qr::encode(&c, Payload::Bytes(b"a payload"), Ecc::Low).unwrap();
    let (side, _, modules) = code.to_luma(1, 0);
    let module = 12;
    let mut img = vec![235u8; W * H];
    for y in 0..side * module {
        for x in 0..side * module {
            if modules[(y / module) * side + x / module] < 128 {
                img[(100 + y) * W + 100 + x] = 25;
            }
        }
    }
    let marks = capstone_lattice(500);
    let alone = decode_within(img.clone(), Duration::from_secs(120))
        .unwrap()
        .1;
    assert_eq!(alone, 1, "the code alone");
    for y in 0..H {
        for x in 900..W {
            img[y * W + x] = marks[y * W + x];
        }
    }
    let (_, read) = decode_within(img, Duration::from_secs(120)).expect("decode ended");
    assert_eq!(read, 1);
}
