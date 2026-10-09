//! A QR code as a picture to keep: the code, black on white, with a
//! title and label lines under it, written as a PNG file.
//!
//! The picture is drawn on an off-screen [`Canvas`] (tiny-skia on the
//! CPU, in the app's own fonts), each module of the code a square of
//! whole pixels, and written as eight-bit greyscale so that the edges of
//! the text keep their shades ([`osk_codec::png::grey_png`]). Encoding a
//! picture here is allowed: only decoding stays in the disk process
//! (`PLAN.md` §12 item 6).
//!
//! Whoever calls it decides what the code holds and what the label
//! says; the label is plain statements, one a line, each wrapped to the
//! picture's width.

use osk_codec::qr::{QUIET_ZONE, QrMatrix};
use osk_shell_api::{BootState, DisplayInfo, SecureHardware};
use osk_ui::tokens;
use osk_ui::{Canvas, Color, Dp, Font, Rect, Scale, SizeClass, TextAlign};

/// The picture's width in pixels, whatever the code's size: the code is
/// scaled to it in whole pixels a module, and the label is set at one
/// size in every picture.
const WIDTH: u16 = 640;

/// The density the picture is drawn at: 1.6 pixels a dp, so the label
/// reads at arm's length on a phone and on paper.
const DPI: u16 = 256;

/// The tallest picture: the label stops at it. A label of a few lines
/// is far shorter. Between the code alone and this height, a picture
/// [`WIDTH`] wide at [`DPI`] is of the phone's size class, whose type
/// ramp the label is measured at.
const MAX_HEIGHT: u16 = 1600;

/// `code` with `title` and then each of `lines` under it, as a PNG
/// file. The pixels drawn are wiped before this returns, so a secret's
/// code leaves only the file.
pub fn labelled_png(code: &QrMatrix, title: &str, lines: &[String]) -> Vec<u8> {
    let mut canvas = draw(code, title, lines);
    let bytes = png(&canvas);
    canvas.clear(Color::WHITE);
    bytes
}

/// `code` with its title and label lines under it, drawn: what
/// [`labelled_png`] writes. The canvas records the text it drew
/// ([`Canvas::ink`]).
pub fn draw(code: &QrMatrix, title: &str, lines: &[String]) -> Canvas {
    let scale = Scale::for_class(DPI, SizeClass::Mobile);
    let w = i32::from(WIDTH);
    let modules = code.size() + 2 * QUIET_ZONE;
    let unit = (usize::from(WIDTH) / modules.max(1)).max(1);
    let side = (modules * unit) as i32;
    let margin = scale.px(Dp(tokens::PAD));
    let gap = scale.px(Dp(tokens::GAP_SMALL));
    let text_w = w - 2 * margin;
    let title_font = Font::semibold(tokens::TITLE);
    let line_font = Font::regular(tokens::BODY);

    // The label's height, measured as it will be drawn.
    let height_of =
        |font: Font, s: &str| osk_ui::text::wrapped_height(&font.sized(scale), s, text_w).max(0);
    let mut h = side + height_of(title_font, title);
    for l in lines {
        h += gap + height_of(line_font, l);
    }
    h += margin;
    let h = h.clamp(side, i32::from(MAX_HEIGHT));

    let mut canvas = Canvas::new(&DisplayInfo {
        width: WIDTH,
        height: h as u16,
        dpi: DPI,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: true,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    });
    debug_assert_eq!(
        canvas.scale(),
        scale,
        "the label was measured at another size"
    );
    canvas.record_ink(true);
    canvas.clear(Color::WHITE);

    // The code, centred across the width; its quiet zone is the margin
    // above it and between it and the title.
    let left = (w - side) / 2;
    let unit = unit as i32;
    for y in 0..code.size() {
        for x in 0..code.size() {
            if code.module(x, y) {
                canvas.fill_rect(
                    Rect::new(
                        left + (x + QUIET_ZONE) as i32 * unit,
                        (y + QUIET_ZONE) as i32 * unit,
                        unit,
                        unit,
                    ),
                    Color::BLACK,
                );
            }
        }
    }

    let mut y = side;
    y += canvas.text_wrapped(
        Rect::new(margin, y, text_w, h - y),
        title,
        title_font,
        Color::BLACK,
        TextAlign::Start,
    );
    for l in lines {
        y += gap;
        if y >= h {
            break;
        }
        y += canvas.text_wrapped(
            Rect::new(margin, y, text_w, h - y),
            l,
            line_font,
            Color::BLACK,
            TextAlign::Start,
        );
    }
    canvas
}

/// A drawn picture as an eight-bit greyscale PNG file.
pub fn png(canvas: &Canvas) -> Vec<u8> {
    let frame = canvas.frame();
    // Opaque pixels, so the premultiplied bytes are the colour; the
    // picture is black on white, and its grey is the colour's luma.
    let grey = zeroize::Zeroizing::new(
        frame
            .rgba
            .chunks_exact(4)
            .map(|p| ((u32::from(p[0]) * 3 + u32::from(p[1]) * 6 + u32::from(p[2])) / 10) as u8)
            .collect::<Vec<u8>>(),
    );
    osk_codec::png::grey_png(usize::from(frame.width), usize::from(frame.height), &grey)
}

/// One picture to write: the file's name, the code, and the title and
/// label lines under it.
pub struct Labelled {
    /// The file's name.
    pub name: String,
    /// The code.
    pub code: QrMatrix,
    /// The first line under the code, set large.
    pub title: String,
    /// The lines under the title, one statement each.
    pub lines: Vec<String>,
}

impl Labelled {
    /// The picture as a PNG file.
    pub fn png(&self) -> Vec<u8> {
        labelled_png(&self.code, &self.title, &self.lines)
    }

    /// The picture drawn, its text recorded.
    pub fn draw(&self) -> Canvas {
        draw(&self.code, &self.title, &self.lines)
    }
}
