//! §5 Scanner: "square viewfinder with the state inside, and the band
//! of ways in under it". Used by Scan.

use alloc::vec::Vec;

use crate::components;
use crate::layout::Node;
use crate::organisms::{self, TileSpec};
use crate::tokens;

use super::{Action, Chrome, Frame};

/// What a [`scanner`] screen shows.
pub struct Scanner<'a> {
    /// The app bar's title, which names what is expected: "Scan a
    /// transaction" (§4.9).
    pub title: &'a str,
    /// The state drawn inside the square: "Waiting for camera".
    pub state: &'a str,
    /// The share of the parts read, while a code arrives in several.
    /// The count itself is the `state` line (§4.9).
    pub parts: Option<f32>,
    /// The band under the square: "Read a file", "Paste", "Type", in
    /// that order, as equal tiles. A way this shell cannot serve is a
    /// dimmed tile. There is no torch control (§4.9).
    pub ways: Vec<TileSpec>,
    /// The camera's latest frame, drawn inside the square under the
    /// brackets. `None` before the first frame and where the shell has
    /// no camera (§4.9).
    pub preview: Option<components::Preview>,
    /// The one thing the person does here where the screen is not
    /// waiting for a code: the shutter of Create's camera-noise step.
    /// `None` on the scanner proper, whose way out is the chevron.
    pub action: Option<Action>,
}

/// §5 Scanner: "square viewfinder with the state inside, and a band of
/// the ways in under it."
///
/// §4.9: "A square viewfinder with corner brackets, as wide as the pane
/// allows; the state ('Waiting for camera', '3 of 8') drawn inside the
/// square, at its bottom; a progress bar inside it for parts. No torch
/// control. Under it one band of equal tiles — Read a file, Paste, Type
/// — and the square takes what the band leaves. The expected type is in
/// the title." There is no bottom action: the back chevron is the way
/// out (§4.1), and the `wide` sidebar is inert while the camera is on.
///
/// The band is §4.1's tile idiom, the one Home draws over its actions,
/// and it is one band on every class: three ways in cost the height of
/// one tile rather than of three rows, which is what leaves the 268 dp
/// panel a viewfinder worth looking through.
///
/// §2.6 and §5: the Scanner is a Show screen turned around. The square
/// is the centred block and the band under it is the bottom group, so
/// the viewfinder sits where a code would and the band where a toggle
/// would.
///
/// Made of [`components::viewfinder`] (§4.9) and
/// [`organisms::tile_band`] (§4.1).
pub fn scanner(c: &Chrome, s: Scanner<'_>) -> Node {
    let class = c.class();
    let m = c.pane();
    // The room the square has: the pane's width less the screen padding,
    // and the height under the app bar less the band below it.
    let band_dp = if s.ways.is_empty() {
        0.0
    } else {
        tokens::tile_band_height(class) + tokens::GAP
    };
    let below = band_dp
        + if s.action.is_some() {
            tokens::CTA + tokens::GAP
        } else {
            0.0
        };
    let side = components::viewfinder_side(
        class,
        m.width_dp - 2.0 * tokens::PAD,
        m.height_dp - tokens::APP_BAR - tokens::PAD - below,
    );
    let block = Node::column().child(components::viewfinder(
        super::VIEWFINDER,
        super::VIEWFINDER_STATE,
        side,
        s.state,
        s.parts,
        s.preview,
    ));
    let has_band = !s.ways.is_empty();
    let band = organisms::tile_band(class, s.ways);
    c.fixed(
        Frame::new(s.title, c.place(block, has_band.then_some(band)))
            .footer(c.actions(s.action.into_iter().collect()))
            .dimmed(),
    )
}
