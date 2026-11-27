//! What the runtime keeps between frames that is not a number (04 K16, V19): the frost under a
//! sheet, and a moved region's rows. The kit is given the copies the runtime holds, read-only, and gives back the ones it
//! made this frame; it keeps none.

use faraday_draw::geom::Rect;
use faraday_port::values::Anchor;

use crate::Pressed;

/// A frosted page the runtime kept, lent to a frame: the one under `sheet`, made while `page`
/// was drawn.
#[derive(Clone, Copy)]
pub struct KeptFrost<'a> {
    pub sheet: Anchor,
    pub page: Anchor,
    pub pixels: &'a [u8],
}

/// A frosted page a frame made under `sheet` for the runtime to keep. The runtime moves the
/// pixels into a wiping buffer and refuses them when the frame showed a secret.
pub struct MadeFrost {
    pub sheet: Anchor,
    pub page: Anchor,
    pub pixels: Vec<u8>,
}

/// The rows a region left the frame before, lent to a frame that may move them (04 K16): the
/// region `region` of `page`, seen through `view`, drawn with its content `px` whole pixels up,
/// as [`faraday_draw::Canvas::copy_region`] took them before the stretch, the fades and the
/// bar. The runtime lends none unless nothing but a region's motion asked for the frame.
#[derive(Clone, Copy)]
pub struct KeptRows<'a> {
    pub page: Anchor,
    pub region: Anchor,
    pub view: Rect,
    pub px: i32,
    pub rows: &'a [u8],
}

/// The rows a frame took of a region for the runtime to keep. The runtime moves them into a
/// wiping buffer and refuses them when the frame showed a secret or a sheet.
pub struct MadeRows {
    pub page: Anchor,
    pub region: Anchor,
    pub view: Rect,
    pub px: i32,
    pub rows: Vec<u8>,
}

/// What a frame is given and gives back beyond its motion: grouped here so a child `Ui` carries
/// them as one.
pub(crate) struct Keeps<'f, A: Pressed> {
    /// Lent by the runtime.
    pub(crate) kept: &'f [KeptFrost<'f>],
    pub(crate) made: Vec<MadeFrost>,
    /// The rows lent, and the rows this frame took.
    pub(crate) rows: &'f [KeptRows<'f>],
    pub(crate) made_rows: Vec<MadeRows>,
    /// Regions drawn as moved frames.
    pub(crate) moved: usize,
    /// Where they moved: each region's view and how many rows its rows now stand lower (higher
    /// for a negative number).
    pub(crate) slid: Vec<(Rect, i32)>,
    /// The presses of the holds drawn, and those of them that are full.
    pub(crate) holds: Vec<A>,
    pub(crate) held: Vec<A>,
}

impl<'f, A: Pressed> Keeps<'f, A> {
    pub(crate) fn new(kept: &'f [KeptFrost<'f>], rows: &'f [KeptRows<'f>]) -> Self {
        Keeps {
            kept,
            made: Vec::new(),
            rows,
            made_rows: Vec::new(),
            moved: 0,
            slid: Vec::new(),
            holds: Vec::new(),
            held: Vec::new(),
        }
    }
}
