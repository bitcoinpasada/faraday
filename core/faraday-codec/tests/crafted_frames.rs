//! A camera frame is the untrusted side's: a crafted one may be any
//! picture, and the scanner must end it in bounded time, with no read,
//! the same as any frame it cannot read. (The scanner's stop waits for
//! the frame being decoded, so this bound is what a stop can cost.)

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

/// How long the frames that hold the search to its budget may take, on a
/// loaded machine in any build. They take about 150 ms alone.
const LIMIT: Duration = Duration::from_secs(5);

/// What the search alone spends on a frame, and whether that was all it may.
fn spent(frame: &[u8]) -> (u64, bool) {
    let mut image = rqrr::PreparedImage::prepare_from_greyscale(W, H, |x, y| frame[y * W + x]);
    drop(image.detect_grids());
    (image.work().0, image.out_of_work())
}

/// Full-height stripes, their runs of black and white repeating `runs`.
fn stripes(runs: &[usize]) -> Vec<u8> {
    let mut row = Vec::with_capacity(W);
    for (i, run) in runs.iter().cycle().enumerate() {
        if row.len() >= W {
            break;
        }
        row.extend(std::iter::repeat_n(
            if i % 2 == 0 { 25u8 } else { 235 },
            *run,
        ));
    }
    row.truncate(W);
    row.repeat(H)
}

/// The three corner marks and the timing lines of a version 2 code at
/// `MODULE` pixels a module, and around the point where the alignment
/// search goes, more radial spokes than the region table has entries, each
/// too small to be taken for the pattern and far enough apart to stay apart.
fn spokes() -> Vec<u8> {
    const MODULE: usize = 40;
    let (ox, oy) = (200, 10);
    let c = Compute::unchecked();
    let code = qr::encode(&c, Payload::Bytes(b"spokes around the search"), Ecc::Low).unwrap();
    let (side, _, modules) = code.to_luma(1, 0);
    assert_eq!(side, 25);
    let centre = |o: usize| (o + 21 * MODULE + MODULE / 2) as f64;
    let (cx, cy) = (centre(ox), centre(oy));
    let turn = std::f64::consts::TAU;
    let sectors = 750.0;
    let mut img = vec![235u8; W * H];
    for y in 0..H {
        for x in 0..W {
            let (mx, my) = (x.wrapping_sub(ox) / MODULE, y.wrapping_sub(oy) / MODULE);
            if mx < side && my < side {
                let mark = (mx <= 7 || mx >= 17) && my <= 7 || mx <= 7 && my >= 17;
                if (mark || mx == 6 || my == 6) && modules[my * side + mx] < 128 {
                    img[y * W + x] = 25;
                }
            }
            if mx <= 8 || my <= 8 {
                continue;
            }
            let (dx, dy) = (x as f64 + 0.5 - cx, y as f64 + 0.5 - cy);
            let r = dx.hypot(dy);
            let sector = ((dy.atan2(dx) + turn / 2.0) / turn * sectors) as usize;
            if (150.0..=343.0).contains(&r) && sector.is_multiple_of(2) {
                img[y * W + x] = 25;
            }
        }
    }
    img
}

#[test]
fn a_frame_of_repeating_stripes_ends_without_a_read() {
    let patterns: [&[usize]; 2] = [&[1, 1, 3, 1, 1, 1], &[2, 2, 5, 2, 2, 1]];
    for runs in patterns {
        let frame = stripes(runs);
        assert!(
            spent(&frame).1,
            "{runs:?} should have taken all of its budget"
        );
        let (_, read) = decode_within(frame, LIMIT)
            .unwrap_or_else(|| panic!("{runs:?} still being decoded after {LIMIT:?}"));
        assert_eq!(read, 0);
    }
}

#[test]
fn a_frame_of_spokes_around_the_alignment_search_ends_without_a_read() {
    let frame = spokes();
    // No grid is fitted here, so this is all fills, and many times what
    // finding a real dense code's regions takes (about 1.2 million).
    assert!(spent(&frame).0 > 20_000_000, "the search crossed no spokes");
    let (_, read) = decode_within(frame, LIMIT).expect("still being decoded");
    assert_eq!(read, 0);
}

#[test]
fn a_dense_code_reads_with_most_of_its_budget_left() {
    let c = Compute::unchecked();
    let payload: Vec<u8> = (0..qr::byte_capacity(23, Ecc::High))
        .map(|i| (i * 7 + i / 13) as u8)
        .collect();
    let code = qr::encode(&c, Payload::Bytes(&payload), Ecc::High).unwrap();
    let (side, _, modules) = code.to_luma(1, 0);
    let module = 8;
    let mut img = vec![235u8; W * H];
    for y in 0..side * module {
        for x in 0..side * module {
            if modules[(y / module) * side + x / module] < 128 {
                img[(100 + y) * W + 500 + x] = 25;
            }
        }
    }
    let mut image = rqrr::PreparedImage::prepare_from_greyscale(W, H, |x, y| img[y * W + x]);
    drop(image.detect_grids());
    let (spent, allowed) = image.work();
    assert!(spent < allowed / 4, "{spent} of {allowed}");
    let (_, read) = decode_within(img, LIMIT).expect("still being decoded");
    assert_eq!(read, 1);
}

#[test]
fn a_frame_of_tens_of_thousands_of_corner_marks_ends_without_a_read() {
    let frame = capstone_lattice(8);
    let (_, read) = decode_within(frame, Duration::from_secs(120))
        .expect("the frame was still being decoded after two minutes");
    assert_eq!(read, 0);
}

/// Few enough corner marks to pass the count, in groups the search tries
/// and fits: these spend the budget on copies and fits rather than fills.
#[test]
fn a_frame_of_a_few_hundred_corner_marks_ends_without_a_read() {
    for pitch in [100, 130, 200] {
        let (_, read) = decode_within(capstone_lattice(pitch), LIMIT)
            .unwrap_or_else(|| panic!("pitch {pitch} still being decoded after {LIMIT:?}"));
        assert_eq!(read, 0);
    }
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

/// A corner mark of `module` pixels a module, centred at `(cx, cy)` and
/// turned by `angle` radians.
fn draw_mark(img: &mut [u8], cx: f64, cy: f64, module: f64, angle: f64) {
    let (sin, cos) = angle.sin_cos();
    let reach = (module * 6.0) as i64;
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            let (x, y) = (cx as i64 + dx, cy as i64 + dy);
            if x < 0 || y < 0 || x >= W as i64 || y >= H as i64 {
                continue;
            }
            let (px, py) = (dx as f64, dy as f64);
            let d = ((px * cos + py * sin).abs()).max((py * cos - px * sin).abs()) / module;
            if d <= 1.5 || (d > 2.5 && d <= 3.5) {
                img[y as usize * W + x as usize] = 25;
            }
        }
    }
}

/// Three corner marks whose edges are nearly parallel, as far apart as
/// the frame allows, with the turns of the three a hair apart: the lines
/// the search draws through them cross far outside the frame, or nearly
/// never. Whatever it makes of them, the search must end and not wrap an
/// integer (this test is meant to run in a dev build, where it would panic).
#[test]
fn marks_with_nearly_parallel_edges_end_without_a_wrap() {
    let mut seed = 0x2545_f491_4f6c_dd1d_u64;
    let mut unit = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    for n in 0..24 {
        let module = 6.0 + (n % 4) as f64 * 3.0;
        let base = (unit() - 0.5) * 0.02;
        let hair = [1e-6, 1e-4, 1e-3][n % 3];
        let mut img = vec![235u8; W * H];
        let spots = [(80.0, 80.0), (1840.0, 90.0), (90.0 + unit() * 40.0, 990.0)];
        for (i, (x, y)) in spots.into_iter().enumerate() {
            draw_mark(&mut img, x, y, module, base + hair * i as f64);
        }
        let (_, read) = decode_within(img, LIMIT)
            .unwrap_or_else(|| panic!("frame {n} still being decoded after {LIMIT:?}"));
        assert!(read <= 1);
    }
}
