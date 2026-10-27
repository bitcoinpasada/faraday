//! What the scanner's viewfinder shows of a camera frame, the frames read
//! by the camera shells' own scanner (`faraday_scanner::look`): a code
//! found is outlined where it is in the picture, and gone when it leaves;
//! a code too fine for the camera is said to be, told apart from one
//! large enough whose picture is spoiled (`docs/QR.md` §8 items 4 and 8).

use faraday_codec::qr::{self, Ecc, Payload, QUIET_ZONE};
use faraday_core::testkit;
use faraday_core::{Action, Faraday, StorageEvent};
use faraday_shell_api::{App, Event};

/// The camera frame's size.
const W: usize = 640;
const H: usize = 480;

/// How a camera spoils the picture of a code.
#[derive(Clone, Copy)]
enum Spoil {
    /// A clean picture.
    None,
    /// Each pixel of the code flipped, one in five: sensor noise, which
    /// a code at under three pixels a module cannot survive.
    Pixels,
    /// Whole modules flipped, about one in seven: glare and smudges over
    /// a code large enough to read.
    Modules,
}

/// A white `W` × `H` camera frame with a code of `bytes` bytes in its
/// middle at `scale` pixels a module, spoiled as `spoil` says; the three
/// finder patterns are left whole, so the scanner finds the code.
fn camera_frame(bytes: usize, scale: usize, spoil: Spoil) -> Vec<u8> {
    let c = testkit::compute();
    let payload: Vec<u8> = (0..bytes).map(|i| (i * 7 + i / 13) as u8).collect();
    let m = qr::encode(&c, Payload::Bytes(&payload), Ecc::Low).unwrap();
    let n = m.size();
    let (cw, ch, mut code) = m.to_luma(scale, QUIET_ZONE);
    let mut seed = 12_345u32;
    let mut chance = |permille: u32| {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        (seed >> 16) % 1000 < permille
    };
    let finder = |mx: usize, my: usize| {
        (mx < 9 && my < 9) || (mx + 9 > n && my < 9) || (mx < 9 && my + 9 > n)
    };
    let q = QUIET_ZONE;
    for my in 0..n {
        for mx in 0..n {
            let whole = matches!(spoil, Spoil::Modules) && chance(150);
            for y in 0..scale {
                for x in 0..scale {
                    let flip = match spoil {
                        Spoil::None => false,
                        Spoil::Pixels => chance(200),
                        Spoil::Modules => whole,
                    };
                    if flip && !finder(mx, my) {
                        let i = ((q + my) * scale + y) * cw + (q + mx) * scale + x;
                        code[i] = 255 - code[i];
                    }
                }
            }
        }
    }
    let mut px = vec![255u8; W * H];
    let (x0, y0) = ((W - cw) / 2, (H - ch) / 2);
    for y in 0..ch {
        px[(y0 + y) * W + x0..(y0 + y) * W + x0 + cw].copy_from_slice(&code[y * cw..(y + 1) * cw]);
    }
    px
}

/// The camera shows `luma`: the picture is drawn, and what the scanner
/// sees of it is told to the app, as the shells do.
fn show(app: &mut Faraday, luma: &[u8]) {
    app.event(Event::CameraFrame {
        width: W as u16,
        height: H as u16,
        luma: luma.to_vec(),
        chroma: None,
    });
    let (_, seen) = faraday_scanner::look(W, H, luma, 1, (W as u16, H as u16));
    if let Some(seen) = seen {
        app.storage(StorageEvent::QrSeen {
            width: seen.width,
            height: seen.height,
            corners: seen.corners,
            module_tenths: (seen.module_px * 10.0).clamp(0.0, 65535.0) as u16,
            read: seen.read,
        });
    }
}

fn scanning(width: u16, height: u16) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(faraday_shell_api::DisplayInfo {
        width,
        height,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: faraday_shell_api::SecureHardware::None,
        boot: faraday_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
    app.press(Action::Scan);
    app
}

/// The drawn frame's pixels and width.
fn drawn(app: &mut Faraday) -> (Vec<u8>, usize) {
    let f = app.frame();
    (f.rgba.to_vec(), usize::from(f.width))
}

/// The box (left, top, right, bottom) round the pixels where `pick` holds.
fn bounds(
    (px, w): &(Vec<u8>, usize),
    pick: impl Fn(usize, &[u8]) -> bool,
) -> Option<(usize, usize, usize, usize)> {
    let mut b: Option<(usize, usize, usize, usize)> = None;
    for (i, p) in px.chunks_exact(4).enumerate() {
        if pick(i, p) {
            let (x, y) = (i % w, i / w);
            b = Some(b.map_or((x, y, x, y), |(l, t, r, bo)| {
                (l.min(x), t.min(y), r.max(x), bo.max(y))
            }));
        }
    }
    b
}

/// A pixel in colour, not the grey of the picture.
fn coloured(p: &[u8]) -> bool {
    let (hi, lo) = (p[..3].iter().max().unwrap(), p[..3].iter().min().unwrap());
    hi - lo > 48
}

/// The coloured pixels of `after` that `before` did not have, within
/// `near` of the box `b`.
fn new_colour_near(
    before: &(Vec<u8>, usize),
    after: &(Vec<u8>, usize),
    b: (usize, usize, usize, usize),
    near: usize,
) -> Option<(usize, usize, usize, usize)> {
    let w = after.1;
    bounds(after, |i, p| {
        let (x, y) = (i % w, i / w);
        x + near >= b.0
            && x <= b.2 + near
            && y + near >= b.1
            && y <= b.3 + near
            && coloured(p)
            && before.0[i * 4..i * 4 + 4] != *p
    })
}

#[test]
fn a_code_in_the_picture_is_outlined_where_it_is_and_not_once_it_is_gone() {
    for (width, height) in [(1280, 800), (480, 640)] {
        let mut app = scanning(width, height);
        let blank = vec![255u8; W * H];
        show(&mut app, &blank);
        let empty = drawn(&mut app);
        // No code, no outline: nothing seen, nothing in colour added.
        let code = camera_frame(100, 6, Spoil::None);
        app.event(Event::CameraFrame {
            width: W as u16,
            height: H as u16,
            luma: code.clone(),
            chroma: None,
        });
        let picture = drawn(&mut app);
        // Where the code is drawn: what the picture of it changed.
        let at = bounds(&picture, |i, p| empty.0[i * 4..i * 4 + 4] != *p).expect("no code drawn");
        assert!(new_colour_near(&empty, &picture, at, 24).is_none());
        // Seen by the scanner: outlined round it, close to its edge.
        show(&mut app, &code);
        let outlined = drawn(&mut app);
        let line = new_colour_near(&picture, &outlined, at, 24).expect("no outline");
        let round = line.0 <= at.0 && line.1 <= at.1 && line.2 >= at.2 && line.3 >= at.3;
        let close = [
            at.0.abs_diff(line.0),
            at.1.abs_diff(line.1),
            at.2.abs_diff(line.2),
            at.3.abs_diff(line.3),
        ]
        .iter()
        .all(|&d| d <= 16);
        assert!(
            round && close,
            "{width}x{height}: the code is at {at:?}, the outline at {line:?}"
        );
        // The code leaves the picture: the outline goes with it.
        for _ in 0..12 {
            show(&mut app, &blank);
        }
        let gone = drawn(&mut app);
        assert!(
            new_colour_near(&empty, &gone, at, 24).is_none(),
            "{width}x{height}: the outline stayed"
        );
    }
}

fn note(app: &Faraday) -> String {
    app.scan().and_then(|s| s.note.clone()).unwrap_or_default()
}

#[test]
fn a_code_too_fine_for_the_camera_is_told_apart_from_a_poor_picture() {
    let fine = "Too fine for this camera";
    let hint = "Hold it square-on, in focus, with its white margin in the frame";
    // A code at two pixels a module, its picture noisy.
    let mut app = scanning(1280, 800);
    let frame = camera_frame(300, 2, Spoil::Pixels);
    for t in [1_000, 3_000, 5_100, 7_000] {
        app.event(Event::Tick { now_ms: t });
        show(&mut app, &frame);
    }
    assert!(note(&app).starts_with(fine), "{}", note(&app));
    // A code large enough, its picture spoiled by glare.
    let mut app = scanning(1280, 800);
    let frame = camera_frame(100, 6, Spoil::Modules);
    for t in [1_000, 3_000, 5_100, 7_000] {
        app.event(Event::Tick { now_ms: t });
        show(&mut app, &frame);
    }
    assert_eq!(note(&app), hint);
}
