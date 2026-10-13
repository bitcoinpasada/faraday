//! Receive from Faraday (`docs/COMPANION.md` §4.2, §11): the device's
//! own codes for a file, a PSBT and a description, as `faraday-core`'s QR
//! sheet makes them, single and animated, read by the phone's camera
//! with frames missed and the loop read again, arrive whole and are kept
//! once.

mod common;

use common::*;
use faraday_companion::store::Kind;
use faraday_companion::{Act, Companion, Page};
use faraday_core::{QR_PARTS, QrFormat, QrSource, QrView};
use osk_codec::QrMatrix;

/// The camera held to the device's loop: the first pass misses every
/// third frame, then the loop comes round and is read again, until the
/// screen leaves the scanner.
fn read_loop(c: &mut Companion, frames: &[QrMatrix]) {
    for (i, f) in frames.iter().enumerate() {
        if c.page != Page::Receive {
            return;
        }
        if frames.len() > 1 && i % 3 == 1 {
            continue;
        }
        show(c, f);
    }
    for _ in 0..4 {
        for f in frames {
            if c.page != Page::Receive {
                return;
            }
            show(c, f);
        }
    }
}

/// `view` read whole, kept as `kind` with `bytes` (and `name`), and kept
/// once however often its loop is read.
fn arrives(view: &QrView, kind: Kind, bytes: &[u8], name: Option<&str>) {
    let mut c = receiving();
    read_loop(&mut c, &view.frames);
    let Page::Arrived { sha256, again } = c.page.clone() else {
        panic!("{}: {:?}", view.subtitle, c.page);
    };
    assert!(!again);
    let (_, kept) = drain(&mut c);
    assert!(kept_store(&kept).is_some(), "nothing handed to the shell");
    {
        let r = c.received();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].kind, kind, "{}", view.subtitle);
        assert_eq!(r[0].bytes, bytes, "{}", view.subtitle);
        assert_eq!(r[0].sha256, sha256);
        if let Some(n) = name {
            assert_eq!(r[0].name, n);
        }
    }
    assert!(c.texts().iter().any(|t| t == &c.received()[0].name));
    assert!(c.offers(Act::Open));
    // The device's code comes round again: still one file.
    c.press(Act::ScanAgain);
    read_loop(&mut c, &view.frames);
    assert!(
        matches!(c.page, Page::Arrived { again: true, .. }),
        "{:?}",
        c.page
    );
    assert_eq!(c.received().len(), 1);
    c.press(Act::Open);
    assert_eq!(c.page, Page::Record(sha256));
}

#[test]
fn a_psbt_as_one_code_arrives_and_is_kept_once() {
    let psbt = psbt();
    let view = QrView::psbt("spend.psbt", &psbt).expect("codes");
    assert_eq!(view.frames.len(), 1);
    arrives(&view, Kind::Psbt, &psbt, None);
}

#[test]
fn an_animated_psbt_arrives_whole_with_frames_missed() {
    let psbt = psbt();
    let view = QrView::of(
        "spend.psbt",
        QrSource::Psbt(psbt.clone()),
        QrFormat::Bbqr,
        QR_PARTS[0],
    )
    .expect("codes");
    assert!(view.frames.len() > 2, "{}", view.subtitle);
    arrives(&view, Kind::Psbt, &psbt, None);
}

#[test]
fn a_description_arrives_as_one_code_and_animated() {
    let d = description();
    let one = QrView::text("Spending", &d).expect("codes");
    assert_eq!(one.frames.len(), 1);
    arrives(&one, Kind::Description, d.as_bytes(), None);
    for format in [QrFormat::Bbqr, QrFormat::Parts] {
        let many =
            QrView::of("Spending", QrSource::Text(d.clone()), format, QR_PARTS[0]).expect("codes");
        assert!(many.frames.len() > 1, "{}", many.subtitle);
        arrives(&many, Kind::Description, d.as_bytes(), None);
    }
}

#[test]
fn a_file_in_the_envelope_arrives_under_its_own_name() {
    let notes = b"Meet at the north gate at noon.\n".to_vec();
    let one = QrView::of(
        "notes.txt",
        QrSource::File("notes.txt".into(), notes.clone()),
        QrFormat::Ur,
        QR_PARTS[1],
    )
    .expect("codes");
    assert_eq!(one.frames.len(), 1);
    arrives(&one, Kind::File, &notes, Some("notes.txt"));
    // A file too big for one code: a fountain-coded UR, and BBQr.
    let photo: Vec<u8> = (0..3000u32).map(|i| (i * 7 % 251) as u8).collect();
    for format in [QrFormat::Ur, QrFormat::Bbqr] {
        let many = QrView::of(
            "photo.bin",
            QrSource::File("photo.bin".into(), photo.clone()),
            format,
            QR_PARTS[2],
        )
        .expect("codes");
        assert!(many.frames.len() > 4, "{}", many.subtitle);
        arrives(&many, Kind::File, &photo, Some("photo.bin"));
    }
}

#[test]
fn parts_read_say_how_many_and_which_are_missing() {
    let psbt = psbt();
    let view = QrView::of(
        "spend.psbt",
        QrSource::Psbt(psbt),
        QrFormat::Bbqr,
        QR_PARTS[0],
    )
    .expect("codes");
    let n = view.frames.len();
    let mut c = receiving();
    show(&mut c, &view.frames[1]);
    assert_eq!(c.page, Page::Receive);
    let missing: Vec<String> = (1..=n).filter(|&k| k != 2).map(|k| k.to_string()).collect();
    let line = format!("1 of {n} · missing {}", missing.join(", "));
    assert!(c.texts().contains(&line), "{:?}", c.texts());
}
