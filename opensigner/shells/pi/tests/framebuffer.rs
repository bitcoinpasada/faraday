//! What the panel would show, checked against a file.
//!
//! The shell is run exactly as it runs on the device, except that `--fb`
//! points at a regular file and `--input` at `/dev/null`. The file is then
//! decoded and compared with the frame the core produces in this process,
//! pixel for pixel, plus two colours a person can check by eye: the black
//! ground and the accent of the Home tiles.
//!
//! No device file is opened, so this runs anywhere `cargo test` does.

use std::path::PathBuf;
use std::process::Command;

use opensigner_core::{AssuranceTier, BuildInfo, OpenSigner};
use osk_shell_api::{App, BootState, Command as UiCommand, DisplayInfo, Event, SecureHardware};

const WIDTH: u16 = 480;
const HEIGHT: u16 = 640;
const DPI: u16 = 286;

/// The theme's accent (`osk_ui::Theme::DARK.primary`, 0xff9f0a) in RGB565.
const ACCENT_565: u16 = 0xfce1;
/// The theme's ground, black.
const BLACK_565: u16 = 0x0000;

/// Runs the shell against a file and returns what it wrote.
fn run(name: &str, depth: u32) -> Vec<u8> {
    let path: PathBuf = [env!("CARGO_TARGET_TMPDIR"), name].iter().collect();
    let _ = std::fs::remove_file(&path);
    let status = Command::new(env!("CARGO_BIN_EXE_opensigner-pi"))
        .args(["--fb", path.to_str().expect("utf-8 path")])
        // /dev/null ends at once, so the reader thread stops and the loop
        // never waits on a touch that cannot come.
        .args(["--input", "/dev/null"])
        .args(["--size", "480x640"])
        .args(["--depth", &depth.to_string()])
        .args(["--dpi", "286"])
        .args(["--frames", "1"])
        .status()
        .expect("the shell runs");
    assert!(status.success(), "the shell exited with {status}");
    std::fs::read(&path).expect("the shell wrote the framebuffer")
}

/// The core's Home frame: whatever it draws in answer to `Display`, which
/// is the first frame the shell writes too.
fn home_frame() -> Vec<u8> {
    let mut app = OpenSigner::new(
        AssuranceTier::A,
        BuildInfo {
            version: env!("CARGO_PKG_VERSION"),
            core_hash: None,
        },
    );
    app.event(Event::Display(DisplayInfo {
        width: WIDTH,
        height: HEIGHT,
        dpi: DPI,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    let mut drew = false;
    while let Some(c) = app.poll_command() {
        if c == UiCommand::Draw {
            drew = true;
            break;
        }
    }
    assert!(drew, "the core asks for a draw as soon as it has a display");
    let frame = app.frame();
    assert_eq!((frame.width, frame.height), (WIDTH, HEIGHT));
    frame.rgba.to_vec()
}

/// The reference packing, written out longhand so that the test does not
/// simply agree with the shell's own arithmetic.
fn rgb565(r: u8, g: u8, b: u8) -> u16 {
    let (r, g, b) = (u16::from(r) >> 3, u16::from(g) >> 2, u16::from(b) >> 3);
    (r << 11) | (g << 5) | b
}

#[test]
fn the_panel_gets_the_home_screen_in_rgb565() {
    let written = run("home-565.fb", 16);
    let pixels = usize::from(WIDTH) * usize::from(HEIGHT);
    assert_eq!(written.len(), pixels * 2, "480 x 640 at two bytes a pixel");

    let rgba = home_frame();
    let decoded: Vec<u16> = written
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let expected: Vec<u16> = rgba
        .chunks_exact(4)
        .map(|p| rgb565(p[0], p[1], p[2]))
        .collect();
    assert_eq!(decoded.len(), expected.len());
    let wrong = decoded
        .iter()
        .zip(&expected)
        .position(|(a, b)| a != b)
        .map(|i| (i % usize::from(WIDTH), i / usize::from(WIDTH)));
    assert_eq!(wrong, None, "first pixel that differs, as (x, y)");

    // Two colours by eye: the corner is the black ground, and the Home
    // tiles' icons are the accent.
    assert_eq!(decoded[0], BLACK_565, "the top-left corner is the ground");
    let accent = decoded.iter().filter(|&&p| p == ACCENT_565).count();
    assert!(
        accent > 500,
        "the Home tiles draw their icons in the accent; found {accent} accent pixels"
    );
}

#[test]
fn a_thirty_two_bit_panel_gets_bgra() {
    let written = run("home-8888.fb", 32);
    let pixels = usize::from(WIDTH) * usize::from(HEIGHT);
    assert_eq!(written.len(), pixels * 4, "480 x 640 at four bytes a pixel");

    let rgba = home_frame();
    let wrong = written
        .chunks_exact(4)
        .zip(rgba.chunks_exact(4))
        .position(|(out, src)| out != [src[2], src[1], src[0], 0xff]);
    assert_eq!(wrong, None, "index of the first pixel that differs");

    // The accent, blue first and opaque.
    let accent = written
        .chunks_exact(4)
        .filter(|p| *p == [0x0a, 0x9f, 0xff, 0xff])
        .count();
    assert!(accent > 500, "found {accent} accent pixels");
}
