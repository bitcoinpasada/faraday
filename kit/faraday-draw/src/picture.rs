//! A labelled picture drawn on `Canvas` in the baked fonts and written as PNG (09 §3.2).

use alloc::vec::Vec;

use faraday_codec::qr::{QUIET_ZONE, QrMatrix};
use faraday_crypto::Secret;
use faraday_crypto::work::Compute;
use faraday_port::values::{Display, Orientation};
use faraday_tokens::dims;
use faraday_tokens::palette::Color;

use crate::canvas::Canvas;
use crate::geom::{Dp, Rect, Scale, SizeClass};
use crate::text::{Font, TextAlign};

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

/// Costs what `faraday_codec::png` costs. `code` with `title` and then each of `lines` under
/// it, as a PNG file. The words are the caller's (section 10 §6). The pixels drawn are wiped
/// before it returns, and the file is a wiping buffer, since the code may be a secret's.
pub fn labelled_png(c: &Compute, code: &QrMatrix, title: &str, lines: &[&str]) -> Secret<Vec<u8>> {
    let mut canvas = draw(code, title, lines);
    let bytes = png(c, &canvas);
    canvas.clear(Color::WHITE);
    bytes
}

/// `code` with its title and label lines under it, drawn: what
/// [`labelled_png`] writes. The canvas records the text it drew
/// ([`Canvas::ink`]).
pub fn draw(code: &QrMatrix, title: &str, lines: &[&str]) -> Canvas {
    let scale = Scale::for_class(DPI, SizeClass::Mobile);
    let w = i32::from(WIDTH);
    let modules = code.size() + 2 * QUIET_ZONE;
    let unit = (usize::from(WIDTH) / modules.max(1)).max(1);
    let side = (modules * unit) as i32;
    let margin = scale.px(Dp(dims::PAD));
    let gap = scale.px(Dp(dims::GAP_SMALL));
    let text_w = w - 2 * margin;
    let title_font = Font::semibold(dims::TITLE);
    let line_font = Font::regular(dims::BODY);

    // The label's height, measured as it will be drawn.
    let height_of =
        |font: Font, s: &str| crate::text::wrapped_height(&font.sized(scale), s, text_w).max(0);
    let mut h = side + height_of(title_font, title);
    for l in lines {
        h += gap + height_of(line_font, l);
    }
    h += margin;
    let h = h.clamp(side, i32::from(MAX_HEIGHT));

    let mut canvas = Canvas::new(&Display {
        width: WIDTH,
        height: h as u16,
        dpi: DPI,
        orientation: Orientation::Portrait,
        rotation_offered: None,
        external: false,
        preview: None,
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

/// Costs a PNG encoding of every pixel: a drawn picture as an eight-bit greyscale PNG file.
pub fn png(_c: &Compute, canvas: &Canvas) -> Secret<Vec<u8>> {
    let frame = canvas.pixels();
    // Opaque pixels, so the premultiplied bytes are the colour; the picture is black on white,
    // and its grey is the colour's luma.
    let grey = Secret::new(
        frame
            .rgba
            .chunks_exact(4)
            .map(|p| ((u32::from(p[0]) * 3 + u32::from(p[1]) * 6 + u32::from(p[2])) / 10) as u8)
            .collect::<Vec<u8>>(),
    );
    Secret::new(faraday_codec::png::grey_png(
        usize::from(frame.width),
        usize::from(frame.height),
        grey.expose(),
    ))
}
