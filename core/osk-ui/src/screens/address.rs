//! §5 Address: "the QR at the class's side, the label ('Receive 2') and
//! the whole address (comparison string) under it". Reached from a row
//! of [`super::addresses`], and from a Verify result's address.

use alloc::rc::Rc;
use alloc::string::String;

use osk_codec::qr::QrMatrix;

use crate::components;
use crate::layout::{Id, Node};

use crate::tokens;

use super::{ActionRow, Chrome, Frame};

/// What an [`address`] screen shows.
pub struct Address<'a> {
    /// The app bar's title, which names the address: "Receive 2".
    pub title: &'a str,
    /// The label under the QR, which is the address's name (§4.9).
    pub label: &'a str,
    /// The address itself and the id its chunked lines page under.
    pub address: (Id, &'a str),
    /// The QR the address stands for.
    pub code: Rc<QrMatrix>,
    /// The row under it that copies the address, where the shell has a
    /// clipboard to copy it to (§4.10, PLANNING §16.88).
    pub copy: Option<ActionRow>,
    /// The row under it that saves the code as a picture
    /// (`docs/PLANNING.md` §16.134 rule 4).
    pub save: Option<ActionRow>,
    /// A §4.2 value row above the copy row, which opens the Choice of
    /// the form the address is shown in: its id, its label and the form
    /// now shown. `None` on an address that has one form
    /// (`docs/PLANNING.md` §16.113).
    pub format: Option<(Id, String, String)>,
    /// A line under the block that states what has just happened and
    /// then stops: "Copied", "No clipboard".
    pub caption: Option<String>,
}

/// §5 Address: "the QR at the class's side, the label ('Receive 2') and
/// the whole address (comparison string) under it."
///
/// One address, one screen: §4.1 forbids a pager between two views of
/// one thing, so the Address | QR pager is gone and the back chevron is
/// the way out. §4.5 shows the address whole and chunked, because this
/// is the screen where it is compared character by character; §4.9 draws
/// the QR at the class's side with its label under it.
///
/// §2.6 makes the whole screen something the eye reads: the code, its
/// label and the address are one block, centred in the space under the
/// app bar, and the back chevron is the only control. The square is
/// [`tokens::qr_side`] on every class (§4.9: "The side is the token on
/// every screen that shows a QR: the Address screen, the QR screen, the
/// Secret screen"); the string under it is what gives way, by taking a
/// smaller step of the mono ramp, because §4.5 already draws it at "the
/// largest size that fits".
///
/// Made of [`components::qr_block`] and [`components::code_label`]
/// (§4.9) and [`components::comparison_string`] (§4.5).
pub fn address(c: &Chrome, a: Address<'_>) -> Node {
    let class = c.class();
    let block = Node::column()
        .gap(tokens::GAP)
        .child(components::qr_block(class, a.code, None))
        .child(components::code_label(a.label))
        .child(
            components::comparison_string(class, a.address.0, a.address.1, true)
                .shrink(1.0)
                .min_height(0.0),
        );
    let block = match a.caption {
        Some(caption) => block.child(components::reason(caption)),
        None => block,
    };
    // The group takes an id of its own, so the rows sit inside a
    // column rather than being the group.
    let mut column = Node::column().gap(tokens::menu_row_gap(class));
    let mut any = false;
    if let Some((id, label, value)) = a.format {
        column = column.child(components::script_type_row(class, id, &label, &value));
        any = true;
    }
    if let Some(row) = a.copy {
        column = column.child(row.node(class));
        any = true;
    }
    if let Some(row) = a.save {
        column = column.child(row.node(class));
        any = true;
    }
    let rows = any.then_some(column);
    c.fixed(Frame::new(a.title, c.place(block, rows)).footer(alloc::vec::Vec::new()))
}
