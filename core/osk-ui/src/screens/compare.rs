//! §5 Compare: "label, one comparison string (or a descriptor as
//! structure), Done". Reached from every reference row, and from a txid.

use crate::components;
use crate::layout::{Id, LayoutCtx, Node};
use crate::tokens;

use super::{Action, ActionRow, Chrome, Frame};

/// What a [`compare`] or [`compare_descriptor`] screen shows.
pub struct Compare<'a> {
    /// The kind the title names: "Signature", "Address", "Descriptor".
    pub title: &'a str,
    /// The instance, where there is one: "#0 · 73c5da0a". `None` where
    /// the kind is the whole of what there is to say.
    pub label: Option<&'a str>,
    /// The string itself.
    pub value: &'a str,
    /// The id its chunked lines carry, for a layout test.
    pub id: Id,
    /// The one way out.
    pub done: Action,
    /// The extended key inside a descriptor, which opens on a Compare
    /// screen of its own. Ignored by [`compare`].
    pub key: Option<Id>,
    /// The row that copies the string, where the string is public and
    /// the shell has a clipboard (§4.10, PLANNING §16.88).
    pub copy: Option<ActionRow>,
    /// A line under the block that states what has just happened and
    /// then stops: "Copied", "No clipboard".
    pub caption: Option<alloc::string::String>,
}

/// §5 Compare: "one comparison string, whole and chunked."
///
/// §4.5: "Whole, mono, chunked in fours with alternating colour, largest
/// size that fits, on as many lines as it takes. Never elided." The
/// screen is the string and one way out.
///
/// §5 Compare: the title names the kind ("Signature", "Address") and
/// `label` names the instance ("#0 · 73c5da0a", "Output 1") where there
/// is one. `None` where the kind is the whole of what there is to say,
/// so the screen never carries the same text twice.
///
/// §2.6: the label and the string are one block, centred in the space
/// above Done. The string gives its height back before the block does,
/// so a string too long for the space takes a smaller size from the mono
/// ramp rather than pushing the label off the top.
///
/// Made of [`components::compare_label`] and
/// [`components::comparison_string`] (§4.5) with
/// [`components::primary`] (§4.13) under them.
pub fn compare(c: &Chrome, v: Compare<'_>) -> Node {
    let Compare {
        title,
        label,
        value,
        id,
        done,
        copy,
        caption,
        ..
    } = v;
    let mut block = Node::column().gap(tokens::GAP);
    if let Some(label) = label {
        block = block.child(components::strings::compare_label(label));
    }
    let block = block.child(
        components::comparison_string(c.class(), id, value, false)
            .shrink(1.0)
            .min_height(0.0),
    );
    let block = match caption {
        Some(caption) => block.child(components::reason(caption)),
        None => block,
    };
    // The group takes an id of its own, so the row sits inside a
    // column rather than being the group.
    let rows = copy.map(|row| Node::column().child(row.node(c.class())));
    c.fixed(Frame::new(title, c.place(block, rows)).footer(c.actions(alloc::vec![done])))
}

/// §5 Compare for a descriptor: "one comparison string (or a descriptor
/// as structure), Done."
///
/// §4.5: "Descriptor: Structure: function, origin in the accent, the key
/// as a reference value, suffix and checksum whole. On the Compare
/// screen for a descriptor; elsewhere a reference row." A descriptor is
/// not read character by character the way a key is — the origin and the
/// checksum are what a person checks — so this screen shows its parts
/// rather than chunking the whole of it into fours.
///
/// `ctx` is the frame's scale, which the block needs to measure its
/// tokens before it wraps them; `key` opens the extended key inside it on
/// a Compare screen of its own.
///
/// Made of [`components::descriptor`] (§4.5) under
/// [`components::strings::compare_label`], with
/// [`components::primary`] (§4.13) under them.
pub fn compare_descriptor(c: &Chrome<'_>, ctx: &LayoutCtx, v: Compare<'_>) -> Node {
    let Compare {
        title,
        label,
        value,
        done,
        copy,
        caption,
        key,
        ..
    } = v;
    let avail = (c.column().width_dp - 2.0 * tokens::PAD).max(0.0);
    let mut block = Node::column().gap(tokens::GAP);
    if let Some(label) = label {
        block = block.child(components::strings::compare_label(label));
    }
    let block = block.child(components::descriptor(ctx, avail, value, key));
    let block = match caption {
        Some(caption) => block.child(components::reason(caption)),
        None => block,
    };
    // The group takes an id of its own, so the row sits inside a
    // column rather than being the group.
    let rows = copy.map(|row| Node::column().child(row.node(c.class())));
    c.scrolling(Frame::new(title, c.place(block, rows)).footer(c.actions(alloc::vec![done])))
}
