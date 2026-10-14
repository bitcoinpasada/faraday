//! Draws the Starter's 256x256 menu icon from the app's own mark
//! (`just faraday-starter-icon`).
//!
//! ```text
//! cargo run -p faraday-starter --example icon -- OUT.png [--bare]
//! ```
//!
//! The mark is [`faraday_core::ui::Ui::mark`], the shield drawn beside the
//! name on every page, so the icon follows it. It sits on a dark rounded
//! square, or with `--bare` on nothing. The PNG is RGBA, 8 bits, with no
//! chunks beyond what PNG needs.

use std::fs::File;
use std::io::BufWriter;
use std::process::ExitCode;

use faraday_core::ui::{Ui, pal};
use osk_shell_api::{BootState, DisplayInfo, SecureHardware};
use osk_ui::{Canvas, Rect};

const SIZE: u16 = 256;
/// The square's corner radius, in pixels.
const RADIUS: f32 = 48.0;
/// The mark's box, in pixels, centred in the square.
const MARK: f32 = 176.0;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(out) = args.next() else {
        eprintln!("usage: icon OUT.png [--bare]");
        return ExitCode::from(2);
    };
    let bare = args.next().as_deref() == Some("--bare");
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
    if !bare {
        let all = Rect::new(0, 0, i32::from(SIZE), i32::from(SIZE));
        canvas.fill_rounded_rect(all, RADIUS, pal::BG);
    }
    let mut hits = Vec::new();
    let mut ui = Ui::new(&mut canvas, 1.0, &mut hits, None);
    let at = (f32::from(SIZE) - MARK) / 2.0;
    ui.mark(at, at, MARK, MARK);

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
    let file = BufWriter::new(File::create(&out).expect("create the PNG"));
    let mut enc = png::Encoder::new(file, u32::from(SIZE), u32::from(SIZE));
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().expect("PNG header");
    w.write_image_data(&rgba).expect("PNG data");
    w.finish().expect("PNG end");
    ExitCode::SUCCESS
}
