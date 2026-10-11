//! Streams the kernel's `vivid` test driver, when one is loaded.
//!
//! `vivid` is the in-tree V4L2 test pattern generator: it presents a
//! capture node that behaves like a camera and paints a moving pattern.
//! It is the only way to exercise this crate's `ioctl` path without
//! hardware, and it is exactly the path that cannot be unit-tested — the
//! struct layouts and request numbers are checked in the crate's own
//! tests, but only a driver says whether they are right.
//!
//! The module is not loaded here: `modprobe` needs root, and this test
//! never asks for it. With no `vivid` node the test prints why it did
//! nothing and passes, so `just` is the same on a box with the module
//! and a box without.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use opensigner_v4l2::{Camera, MAX_FPS, Size};

/// A V4L2 node takes one streaming owner at a time, and cargo runs the
/// tests in this binary on threads of their own, so they take turns.
static DEVICE: Mutex<()> = Mutex::new(());

/// The turn, surviving a poisoned lock: a failed test still frees the
/// device, and the next one has something to say about it.
fn device_turn() -> MutexGuard<'static, ()> {
    DEVICE.lock().unwrap_or_else(|e| e.into_inner())
}

/// The first `/dev/videoN` whose driver is `vivid` *and* which
/// `find()` would pick — that is, a capture node, not the output or
/// metadata nodes `vivid` also creates.
fn vivid_node() -> Option<PathBuf> {
    let path = opensigner_v4l2::find()?;
    let name = path.file_name()?.to_str()?;
    let n = name.strip_prefix("video")?;
    let driver = std::fs::read_to_string(format!("/sys/class/video4linux/video{n}/name")).ok()?;
    driver.starts_with("vivid").then_some(path)
}

#[test]
fn streams_ten_frames_from_vivid() {
    let _turn = device_turn();
    let Some(path) = vivid_node() else {
        eprintln!(
            "skipped: no vivid capture device. Load the kernel's test driver \
             (modprobe vivid) to run this test; it is not loaded from here."
        );
        return;
    };

    let mut camera = Camera::open(&path, Size::VGA).expect("vivid should open at 640x480");
    let size = camera.size();
    eprintln!(
        "{}: {} at {}x{}",
        path.display(),
        camera.format(),
        size.width,
        size.height
    );

    let mut frames = Vec::new();
    let start = Instant::now();
    // Twenty tries for ten frames: vivid paints at its own rate and a
    // poll may time out.
    for _ in 0..20 {
        if frames.len() == 10 {
            break;
        }
        if let Some(frame) = camera
            .frame(Duration::from_millis(500))
            .expect("a dequeue should not fail")
        {
            frames.push(frame);
        }
    }
    assert_eq!(frames.len(), 10, "vivid should deliver ten frames");
    // Uncapped: this is the driver's own rate, which is what `spawn`'s
    // cap is measured against in the next test.
    eprintln!(
        "ten frames in {:.2}s: {:.1} a second uncapped",
        start.elapsed().as_secs_f64(),
        10.0 / start.elapsed().as_secs_f64()
    );

    for f in &frames {
        assert_eq!(
            f.luma.len(),
            usize::from(f.width) * usize::from(f.height),
            "a frame's luma is width x height bytes"
        );
        assert!(f.width > 0 && f.height > 0);
        // A frame of one value is a conversion that read the wrong
        // plane or the wrong stride; vivid's pattern is never flat.
        let first = f.luma[0];
        assert!(
            f.luma.iter().any(|&v| v != first),
            "a frame should not be one flat value"
        );
        // A colour device gives colour: the preview is drawn from it,
        // and only a GREY device leaves it out.
        if camera.format() == "GREY" {
            assert_eq!(f.chroma, None, "GREY has no colour to give");
        } else {
            let uv = f.chroma.as_ref().expect("a colour format gives chroma");
            let (cw, ch) = (
                usize::from(f.width).div_ceil(2),
                usize::from(f.height).div_ceil(2),
            );
            assert_eq!(uv.len(), cw * ch * 2, "one pair per 2x2 block of luma");
        }
    }
}

/// The rate cap of §16.33 item 4, measured against a real driver:
/// `spawn` sends at most ten frames a second however fast the device
/// runs, because the core's scanner ticks on its own clock and a Pi 3B+
/// has better things to do than decode thirty frames a second.
#[test]
fn spawn_delivers_at_the_capped_rate() {
    let _turn = device_turn();
    let Some(path) = vivid_node() else {
        eprintln!("skipped: no vivid capture device (modprobe vivid to run this test).");
        return;
    };
    let camera = Camera::open(&path, Size::VGA).expect("vivid should open at 640x480");
    let (tx, rx) = sync_channel(2);
    let stop = Arc::new(AtomicBool::new(false));
    let thread = opensigner_v4l2::spawn(camera, tx, Arc::clone(&stop));

    // Two seconds of capture, drained as fast as a shell's event loop
    // would: the count is what the shell would have delivered.
    let window = Duration::from_secs(2);
    let start = Instant::now();
    let mut frames = 0u32;
    while start.elapsed() < window {
        if rx.recv_timeout(Duration::from_millis(200)).is_ok() {
            frames += 1;
        }
    }
    stop.store(true, Ordering::Relaxed);
    let _ = thread.join();

    let elapsed = start.elapsed().as_secs_f64();
    let rate = f64::from(frames) / elapsed;
    eprintln!("spawn delivered {frames} frames in {elapsed:.2}s: {rate:.1} a second");
    let cap = f64::from(MAX_FPS);
    assert!(
        rate <= cap + 1.0,
        "the cap is {cap} a second, got {rate:.1}"
    );
    // The floor is loose on purpose: what binds here is vivid's own
    // rate, about five frames a second on this driver, not the cap. The
    // assertion above is the property under test; this one only catches
    // a `spawn` that has stopped sending at all.
    assert!(rate >= 1.0, "spawn should keep sending: got {rate:.1}");
}
