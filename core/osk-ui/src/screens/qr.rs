//! §5 QR: "QR, label, Animated toggle row, progress row", and the Save
//! as PNG row under them. Used by a wallet export and a transaction; a
//! SeedQR is a Secret instead.

use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use osk_codec::qr::QrMatrix;

use crate::components;
use crate::geom::SizeClass;
use crate::layout::{Id, Node};
use crate::tokens;

use super::{ActionRow, Chrome, Frame};

/// What a [`qr`] screen shows.
pub struct Qr<'a> {
    /// The app bar's title.
    pub title: &'a str,
    /// The QR, or this part of it when it is animated.
    pub matrix: Rc<QrMatrix>,
    /// What the QR is, under it (§4.9).
    pub label: &'a str,
    /// The switch's id and its word, or `None` where the payload is one
    /// code on every class and animation is not a choice to offer.
    pub toggle: Option<(Id, &'a str)>,
    /// Whether the QR is animated.
    pub animated: bool,
    /// The reason the app chose animation itself — "too dense" — which
    /// dims the row (§4.9).
    pub forced: Option<String>,
    /// The part shown and its count: "2 of 5". Absent while one QR says
    /// everything; the row keeps its height either way.
    pub progress: Option<(f32, String)>,
    /// The row that saves the one static code as a picture, on a code
    /// that is not a secret: live, or dimmed with the reason the toggle
    /// uses where the content is too dense for one code. `None` on a
    /// screen that offers no picture.
    pub save: Option<ActionRow>,
    /// A line under the label that states what has just happened and
    /// then stops: "Saved as opensigner-qr.png".
    pub caption: Option<String>,
}

/// §5 QR: "QR, label, Animated toggle row, progress row."
///
/// §4.9: "A square at the class's side (198 / 260 / 240 dp), centred,
/// its label under it. The side is the token on every screen that shows a
/// QR ... The screen finds the room; on `small` the toggle row and the
/// progress row give way (they shrink to one line each, or the label
/// goes) before the square does." So the square is
/// [`tokens::qr_side`] and never a share of what the rows left over: the
/// rows are the part that gives way, and [`tokens::qr_side`]'s own doc
/// comment carries the budget that makes the three of them fit a 358 dp
/// panel.
///
/// On `small` that means the toggle is one line
/// ([`tokens::qr_toggle_row`]) and the progress row is the bar alone
/// ([`tokens::qr_progress_row`]), its count folded into the title, which
/// is what §3 allows a class to do with progress that does not fit. The
/// progress row keeps its place whether or not the QR is animated, so
/// turning the switch on moves nothing.
///
/// §2.6 splits the screen in two: the QR and its label are read, so they
/// are one block centred in the space under the app bar; the toggle and
/// the progress row are touched, so they are the bottom group.
///
/// Made of [`components::qr_block`], [`components::code_label`],
/// [`components::animated_row`] and [`components::progress`] (§4.9,
/// §4.14).
pub fn qr(c: &Chrome, q: Qr<'_>) -> Node {
    qr_with_actions(c, q, Vec::new())
}

/// The same screen with §4.13's action row under it, for a QR a flow
/// carries on from: the group record a threshold wallet is dealt with,
/// which is saved and then continued past.
pub fn qr_with_actions(c: &Chrome, q: Qr<'_>, actions: Vec<super::Action>) -> Node {
    let class = c.class();
    // §3: "may fold progress into the title where a progress row does not
    // fit." On `small` the row is the bar and the count is in the title,
    // with the separator §4.1 gives a title that carries progress.
    let folded = class == SizeClass::Small;
    let title = match (&q.progress, folded) {
        (Some((_, count)), true) => alloc::format!("{} \u{00b7} {count}", q.title),
        _ => String::from(q.title),
    };
    let progress = match (q.progress, folded) {
        (Some((fraction, _)), true) => components::progress_of(fraction),
        (Some((fraction, count)), false) => components::progress(fraction, count),
        (None, _) => Node::column(),
    };
    let block = Node::column()
        .gap(tokens::GAP_SMALL)
        .child(components::qr_block(class, q.matrix, None))
        .child(components::code_label(q.label));
    let block = match q.caption {
        Some(caption) => block.child(components::reason(caption)),
        None => block,
    };
    // On `small` the code, the Animated row and the progress row take
    // the whole frame; a PNG row as well would draw the toggle over the
    // code's quiet zone, so a screen with the toggle carries none there.
    let save = q
        .save
        .filter(|_| !(class == SizeClass::Small && q.toggle.is_some()));
    let mut bottom = Node::column().gap(tokens::GAP_SMALL);
    if let Some((toggle, word)) = q.toggle {
        bottom = bottom.child(components::animated_row(
            class, toggle, word, q.animated, q.forced,
        ));
    }
    let mut bottom = bottom.child(progress.height(tokens::qr_progress_row(class)));
    if let Some(row) = save {
        bottom = bottom.child(row.node(class));
    }
    c.fixed(Frame::new(&title, c.place(block, Some(bottom))).footer(c.actions(actions)))
}
