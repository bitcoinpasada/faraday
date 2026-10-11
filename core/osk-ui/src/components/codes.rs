//! §4.9 Codes: the QR a screen shows, the mode rows under it, the
//! progress of an animated run, and the scanner's viewfinder.
//!
//! "The word is 'QR', never 'code'."

use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use zeroize::Zeroizing;

use osk_codec::qr::QrMatrix;

use crate::geom::SizeClass;
use crate::layout::{Align, Id, Justify, Node};
use crate::text::{Font, TextAlign};
use crate::tokens;
use crate::widgets::{PanelStyle, Tone, Widget};

/// §4.9 QR out: "A square at the class's side (198 / 260 / 240 dp),
/// centred, its label under it."
///
/// One side per class, from [`tokens::qr_side`], rather than as much of
/// the width as is going: a code is the same square wherever a flow
/// shows one, which is what §2.5's fixed geometry means for something a
/// camera is aimed at. `hold` makes it a secret code: blank, with the
/// code's own mark on it, until its panel is held or the app bar's eye
/// is showing it.
pub fn qr_block(class: SizeClass, matrix: Rc<QrMatrix>, hold: Option<(Id, bool)>) -> Node {
    let widget = match hold {
        Some((id, revealed)) => Widget::secret_qr(matrix, id, revealed),
        None => Widget::qr(matrix),
    };
    let side = tokens::qr_side(class);
    // §4.9: "The side is the token on every screen that shows a QR ...
    // The screen finds the room; on `small` the toggle row and the
    // progress row give way ... before the square does." So the square is
    // the class's side and nothing else: never the space left over, and
    // never a share of the height. It carries [`crate::screens::CODE`],
    // so a layout test can measure the square a camera is aimed at.
    Node::row().justify(Justify::Center).height(side).child(
        Node::widget(widget)
            .id(crate::screens::CODE)
            .width(side)
            .height(side),
    )
}

/// Pages a transcription grid takes on this class: four quadrants on
/// `small`, where a 268 dp panel has no room for a whole symbol at a
/// size a pen can work at, and one page everywhere else
/// (`docs/PLANNING.md` §8.2 item 5a).
pub fn qr_grid_pages(class: SizeClass) -> usize {
    match class {
        SizeClass::Small => 4,
        _ => 1,
    }
}

/// The side in modules of one quadrant of a `modules`-square symbol:
/// half of it, plus one guide band, so that two neighbouring quadrants
/// share exactly [`tokens::QR_GRID_BAND`] modules and a person can line
/// the pages up on the band they have already drawn.
pub fn qr_grid_span(modules: usize) -> usize {
    (modules + tokens::QR_GRID_BAND).div_ceil(2).min(modules)
}

/// The region page `page` of a `modules`-square symbol draws on
/// `class`: the first module, and the side in modules. Page order is
/// top left, top right, bottom left, bottom right.
pub fn qr_grid_region(class: SizeClass, modules: usize, page: usize) -> ((u16, u16), u16) {
    if qr_grid_pages(class) == 1 {
        return ((0, 0), modules as u16);
    }
    let span = qr_grid_span(modules);
    let far = modules.saturating_sub(span);
    let x = if page.is_multiple_of(2) { 0 } else { far };
    let y = if page < 2 { 0 } else { far };
    ((x as u16, y as u16), span as u16)
}

/// §4.9 turned into something a hand can copy: the QR's modules as
/// squares with the grid drawn over them, at the class's own side.
///
/// The grid is a secret wherever the code is one, so it takes the same
/// `hold` a secret QR does: the panel it sits in reveals it, and the app
/// bar's eye reveals it for 30 s (§4.10).
pub fn qr_grid(matrix: Rc<QrMatrix>, origin: (u16, u16), span: u16, hold: (Id, bool)) -> Node {
    // The square is as large as the class allows and as large as the
    // panel gives it, whichever is smaller ([`tokens::qr_grid_side`]);
    // it measures itself, so the panel is the square and no taller.
    Node::row().justify(Justify::Center).child(
        Node::widget(Widget::secret_qr_grid(matrix, origin, span, hold.0, hold.1))
            .id(crate::screens::CODE),
    )
}

/// The square and the label under it, which is how §4.9 shows a QR
/// wherever one is shown.
pub fn qr_with_label(
    class: SizeClass,
    matrix: Rc<QrMatrix>,
    hold: Option<(Id, bool)>,
    label: impl Into<String>,
) -> Node {
    Node::column()
        .gap(tokens::GAP_SMALL)
        .child(qr_block(class, matrix, hold))
        .child(code_label(label))
}

/// The label §4.9 puts under a QR: what it is, in the label token,
/// centred under it.
pub fn code_label(text: impl Into<String>) -> Node {
    let text = text.into();
    let font = super::text::label_font(&text);
    Node::widget(Widget::text(text, font, Tone::Muted).align(TextAlign::Center))
}

/// §4.9 QR out: "an 'Animated' toggle row under it, dimmed with the
/// reason 'too dense' when forced".
///
/// One row, not two settings and not a segmented control: the mode is a
/// boolean, and §4.2 draws a boolean as a switch on a full-width row.
/// `forced` carries the reason the app chose animation itself, which
/// dims the row and takes its hit target away.
///
/// The row is [`tokens::qr_toggle_row`] high, which is the one place
/// §4.9's "the toggle row ... gives way before the square does" is
/// written: on `small` it is one line and the reason sits beside the
/// label, because the class's square and its label leave nothing for a
/// second line; on the taller classes the reason is under the label like
/// every other dead row (§4.11).
pub fn animated_row(
    class: SizeClass,
    id: Id,
    label: impl Into<String>,
    on: bool,
    forced: Option<String>,
) -> Node {
    super::choice::toggle_row_at(
        tokens::qr_toggle_row(class),
        class != SizeClass::Small,
        id,
        label,
        on,
        forced,
    )
}

/// §4.9 "a progress bar row, always present, blank when static", and
/// §4.14 Progress: "A bar with a count '2 of 5' above it."
///
/// `count` of `None` is the row on `small`, where the bar is the whole of
/// it and the count has folded into the title (§3, [`tokens::qr_progress_row`]).
pub fn progress_bar(fraction: f32, count: Option<String>) -> Node {
    let mut column = Node::column().gap(tokens::GAP_SMALL);
    if let Some(count) = count {
        column = column.child(Node::widget(Widget::text(
            count,
            Font::regular(tokens::LABEL),
            Tone::Muted,
        )));
    }
    column.child(Node::widget(Widget::ProgressBar { fraction }).height(tokens::BAR_HEIGHT))
}

/// The camera's latest frame, for the square to show under its brackets
/// (`docs/DESIGN.md` §4.9). 8-bit luma, row-major, `width × height`
/// bytes, with the frame's NV12 chroma plane beside it where the camera
/// gave colour; the application keeps both small and shares the buffers
/// rather than copying them into the tree every frame.
pub struct Preview {
    /// Frame width in pixels.
    pub width: u16,
    /// Frame height in pixels.
    pub height: u16,
    /// The pixels, black 0 to white 255, wiped when the last holder
    /// drops them.
    pub pixels: Rc<Zeroizing<Vec<u8>>>,
    /// The chroma for those pixels, `⌈width / 2⌉ × ⌈height / 2⌉` pairs
    /// of interleaved U and V, or `None` for a grey frame. Wiped the
    /// same way.
    pub chroma: Option<Rc<Zeroizing<Vec<u8>>>>,
}

/// §4.9 Scanner: "A square viewfinder with corner brackets, as wide as
/// the pane allows; the state ('Waiting for camera', '3 of 8') drawn
/// inside the square, at its bottom; a progress bar inside it for
/// parts."
///
/// The square shows `preview`, the camera's latest frame, scaled to
/// cover it and cropped to it, in colour where the camera gave colour
/// and in grey where it did not; the brackets, the state line and the
/// parts bar are drawn over it. Without a frame — before the first one,
/// or where the shell has no camera — the square is empty.
///
/// The brackets are all the frame there is: a box round the preview
/// would compete with the code inside it. The state and the parts bar
/// are inside the square rather than under it, so the square is the
/// whole of the scanner and nothing about it moves when a part arrives.
/// `side_dp` is the square's side, which the screen works out from the
/// room its pane has.
///
/// The state line is the only line of words: §4.9 gives "3 of 8" as a
/// state, so a run of parts says so there and `parts` is the share of
/// them read, which is the bar under it and nothing else.
pub fn viewfinder(
    square_id: Id,
    state_id: Id,
    side_dp: f32,
    state: impl Into<String>,
    parts: Option<f32>,
    preview: Option<Preview>,
) -> Node {
    let mut band = Node::column()
        .pad(tokens::GAP)
        .gap(tokens::GAP_SMALL)
        .align(Align::Stretch)
        .child(Node::widget(
            Widget::text(state, Font::regular(tokens::LABEL), Tone::Muted).align(TextAlign::Center),
        ));
    if let Some(fraction) = parts {
        band =
            band.child(Node::widget(Widget::ProgressBar { fraction }).height(tokens::BAR_HEIGHT));
    }
    // The band is only as tall as the padded line and its bar, and it
    // carries its own translucent ground so the words read over a light
    // frame as well as a dark one.
    let inside = Node::column()
        .id(state_id)
        .justify(Justify::End)
        .align(Align::Stretch)
        .child(
            Node::stack()
                .child(Node::widget(Widget::Panel {
                    style: PanelStyle::Scrim,
                }))
                .child(band),
        );
    let mut square = Node::stack().id(square_id).width(side_dp).height(side_dp);
    if let Some(p) = preview {
        square = square.child(Node::widget(Widget::Luma {
            width: p.width,
            height: p.height,
            pixels: p.pixels,
            chroma: p.chroma,
        }));
    }
    // The brackets go last: they are the frame, and neither the preview
    // nor the state line's band may dim them.
    let square = square.child(inside).child(Node::widget(Widget::Panel {
        style: PanelStyle::Viewfinder,
    }));
    Node::row().justify(Justify::Center).child(square)
}

/// The side in dp of a square viewfinder in a pane `width_dp` wide with
/// `height_dp` of room left under the app bar and above whatever rows
/// sit below it. Capped on `wide`, where the pane would otherwise give
/// the square most of the window.
pub fn viewfinder_side(class: SizeClass, width_dp: f32, height_dp: f32) -> f32 {
    let side = width_dp.min(height_dp).max(0.0);
    if class == SizeClass::Wide {
        side.min(tokens::VIEWFINDER_MAX)
    } else {
        side
    }
}
