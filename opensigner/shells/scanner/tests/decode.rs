//! What a person pointing the camera at a code sees: the code is read,
//! whether it is a plain one or a dense one, it is delivered once, and
//! a camera that runs faster than the board can decode is read at the
//! code the camera is on now, not at one it has moved off.

use opensigner_scanner::{Scanner, decode_frame};
use osk_bip::bip39::{Language, Mnemonic};
use osk_codec::qr::{Ecc, Payload, QUIET_ZONE};
use osk_codec::seedqr;

/// Draws `luma` (`w` by `h`) into the middle of a 640 × 480 white
/// frame, at an even offset so that the reduction's 2 × 2 blocks line
/// up with it: a code held up to a VGA camera.
fn frame_640x480(w: usize, h: usize, luma: &[u8]) -> (u16, u16, Vec<u8>) {
    let (fw, fh) = (640usize, 480usize);
    assert!(w <= fw && h <= fh, "{w}x{h} does not fit");
    let (ox, oy) = (((fw - w) / 2) & !1, ((fh - h) / 2) & !1);
    let mut out = vec![255u8; fw * fh];
    for y in 0..h {
        out[(oy + y) * fw + ox..(oy + y) * fw + ox + w].copy_from_slice(&luma[y * w..y * w + w]);
    }
    (fw as u16, fh as u16, out)
}

/// A frame picturing `bytes` as a QR code, held up to a VGA camera.
fn frame_of(bytes: &[u8], scale: usize) -> (u16, u16, Vec<u8>) {
    let matrix = osk_codec::qr::encode(Payload::Bytes(bytes), Ecc::Low).expect("a code that fits");
    let (w, h, px) = matrix.to_luma(scale, QUIET_ZONE);
    frame_640x480(w, h, &px)
}

/// The frame a person is most likely to hold up: a code large enough
/// that the box-averaged copy still has it, which is what the scanner
/// reads first and what costs a quarter of the pixels.
#[test]
fn a_code_is_read_from_the_reduced_copy() {
    let m = Mnemonic::parse(Language::English, &(["abandon"; 11].join(" ") + " about")).unwrap();
    let matrix = seedqr::encode_seedqr(&m).unwrap();
    let (w, h, px) = matrix.to_luma(8, QUIET_ZONE);
    let (fw, fh, mut luma) = frame_640x480(w, h, &px);
    // Every 2 × 2 block becomes a diagonal pair either side of the
    // module's value: the reduction averages it back to the module, and
    // the full frame has nothing left to find. What is read can then
    // only have come from the reduced copy.
    let fw_u = usize::from(fw);
    for by in 0..usize::from(fh) / 2 {
        for bx in 0..fw_u / 2 {
            let at = |y: usize, x: usize| (by * 2 + y) * fw_u + bx * 2 + x;
            let v = if luma[at(0, 0)] < 128 { 64u8 } else { 191 };
            for (y, x, p) in [
                (0, 0, v + 64),
                (0, 1, v - 64),
                (1, 0, v - 64),
                (1, 1, v + 64),
            ] {
                luma[at(y, x)] = p;
            }
        }
    }

    let bytes = decode_frame(fw, fh, &luma).expect("the SeedQR is read");
    assert_eq!(
        bytes.len(),
        48,
        "twelve four-digit word numbers: {}",
        String::from_utf8_lossy(&bytes)
    );
}

/// A code too dense to survive the reduction — a long payload at three
/// pixels a module — is still read: the full frame is what the scanner
/// falls back to.
#[test]
fn a_dense_code_is_still_read() {
    let dense: Vec<u8> = (0..2000u32).map(|i| b'a' + (i % 26) as u8).collect();
    let (w, h, luma) = frame_of(&dense, 3);
    assert_eq!(decode_frame(w, h, &luma), Some(dense));
}

/// Waits for the worker to deliver a payload. Nothing here is timed:
/// the loop gives up its slice until the answer is there, and fails
/// rather than hanging for ever if it never comes.
fn read(scanner: &Scanner) -> Vec<u8> {
    for _ in 0..100_000 {
        if let Some(bytes) = scanner.poll() {
            return bytes;
        }
        std::thread::yield_now();
    }
    panic!("the scanner read nothing");
}

/// One code held up to the camera is delivered once, and the frames it
/// was found in do not deliver it again.
#[test]
fn a_code_is_delivered_once() {
    let (w, h, luma) = frame_of(b"hello, world", 4);
    let mut scanner = Scanner::new();
    scanner.offer(w, h, luma);
    assert_eq!(read(&scanner), b"hello, world");
    // Stopping joins the worker, so anything it had still to say has
    // been said by the time this returns.
    scanner.stop();
    assert_eq!(scanner.poll(), None, "one frame, one payload");
    assert!(!scanner.running(), "the worker is gone");
}

/// A camera delivers frames faster than a small board decodes them.
/// The frames that pile up are not queued: each one replaces the last,
/// so what is read is where the camera is pointed now. A person who
/// swings the camera from one code to another gets the second one, and
/// does not sit through a backlog of the first.
#[test]
fn the_newest_frame_is_the_one_read() {
    const BURST: usize = 60;
    let (w, h, old) = frame_of(b"the code it moved off", 4);
    let (_, _, new) = frame_of(b"the code it is on now", 4);

    let scanner = Scanner::new();
    for _ in 0..BURST {
        scanner.offer(w, h, old.clone());
    }
    scanner.offer(w, h, new.clone());

    let mut seen = Vec::new();
    loop {
        let bytes = read(&scanner);
        let last = bytes == b"the code it is on now";
        seen.push(bytes);
        if last {
            break;
        }
    }
    assert!(
        seen.len() < BURST,
        "the frames it replaced were not decoded: {} payloads for {} frames",
        seen.len(),
        BURST + 1
    );
}
