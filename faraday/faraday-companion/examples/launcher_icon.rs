//! Draws Faraday Companion's launcher icon foreground from Faraday's own
//! mark (`just companion-icon`), as the Starter's menu icon is drawn
//! (`faraday-starter`'s `examples/icon.rs`).
//!
//! ```text
//! cargo run -p faraday-companion --example launcher_icon -- OUT.png
//! ```
//!
//! The PNG is the foreground layer of an Android adaptive icon: 432 x 432
//! pixels for its 108 dp canvas at the densest screens, the mark centred
//! inside the 66 dp circle no launcher's mask clips, on nothing. The tile
//! behind it, the shell's `launcher_background`, is transparent, so the
//! launcher shows the shield alone and the keyhole shows the wallpaper.
//! RGBA, 8 bits, with no chunks beyond what PNG needs.

use std::fs::File;
use std::io::BufWriter;
use std::process::ExitCode;

use faraday_ui::Ui;
use osk_shell_api::{BootState, DisplayInfo, SecureHardware};
use osk_ui::Canvas;

/// The canvas: 108 dp at 4 pixels a dp.
const SIZE: u16 = 432;
/// The mark's box, in pixels: 52 dp, as tall as a shield can be with its
/// top corners still inside the 66 dp (264 px) circle a launcher's round
/// mask keeps.
const MARK: f32 = 208.0;
/// How far below the canvas's centre the mark's box sits, in pixels: a
/// shield is wide at the top and a point at the bottom, so centred by its
/// box it looks high.
const DROP: f32 = 7.0;

fn main() -> ExitCode {
    let Some(out) = std::env::args().nth(1) else {
        eprintln!("usage: launcher_icon OUT.png");
        return ExitCode::from(2);
    };
    let mut canvas = Canvas::new(&DisplayInfo {
        width: SIZE,
        height: SIZE,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    });
    let mut hits: Vec<(osk_ui::Rect, ())> = Vec::new();
    let mut ui = Ui::new(&mut canvas, 1.0, &mut hits, None);
    let at = (f32::from(SIZE) - MARK) / 2.0;
    ui.mark(at, at + DROP, MARK, MARK);

    // The canvas holds premultiplied pixels; PNG wants straight ones.
    let frame = canvas.frame();
    let mut rgba = frame.rgba.to_vec();
    for px in rgba.chunks_exact_mut(4) {
        let a = u32::from(px[3]);
        if a != 0 && a != 255 {
            for c in &mut px[..3] {
                *c = ((u32::from(*c) * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
    let Ok(file) = File::create(&out) else {
        eprintln!("launcher_icon: cannot write {out}");
        return ExitCode::FAILURE;
    };
    let mut enc = png::Encoder::new(BufWriter::new(file), u32::from(SIZE), u32::from(SIZE));
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let written = enc
        .write_header()
        .and_then(|mut w| w.write_image_data(&rgba).and_then(|()| w.finish()));
    if written.is_err() {
        eprintln!("launcher_icon: cannot write {out}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
