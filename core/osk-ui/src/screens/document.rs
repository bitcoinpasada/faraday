//! §5 Document: "scrolling text with headings". Used by the tiers page,
//! About, Learn, and the message a person is about to sign or has just
//! checked, which [`reading`] draws with the rows that say which key and
//! which address it is for above it.

use alloc::string::String;
use alloc::vec::Vec;

use crate::components;
use crate::layout::{Id, Node};
use crate::organisms;
use crate::tokens;

use super::{Action, Chrome, Frame, Row};

/// One section of a [`document`]: a heading and its paragraphs.
pub struct Section {
    /// The id the heading carries, where the host needs to name it: a
    /// page opened at one of its sections scrolls that heading to the
    /// top (`docs/PLANNING.md` §16.105). `None` on every other section.
    pub id: Option<Id>,
    /// The heading.
    pub heading: String,
    /// The paragraphs under it.
    pub paragraphs: Vec<String>,
}

/// §5 Document: "scrolling text with headings."
///
/// The one place §2.1 allows prose: "Sentences appear only as a one-line
/// reason on a dimmed control, or in Learn." The body scrolls and draws its own scrollbar; on `wide` it is
/// capped at [`tokens::DOCUMENT_MAX_WIDTH`], because a line that spans a
/// desktop window is a line nobody finds their way back along.
///
/// Made of [`organisms::section`] for the headings and
/// [`components::explainer`] (§4.12) for the paragraphs.
pub fn document(c: &Chrome, title: &str, sections: Vec<Section>) -> Node {
    document_with(c, title, sections, None, None)
}

/// §5 Document that ends in one row, carries one bottom action, or
/// both: the page a person is reading and the one flow it is about.
///
/// The row is the same component a [`super::menu`] draws, so the way on
/// from a page reads here as it does everywhere else, and the action is
/// [`Chrome::actions`] (§4.13). A Document with neither is
/// [`document`].
pub fn document_with(
    c: &Chrome,
    title: &str,
    sections: Vec<Section>,
    row: Option<Row>,
    action: Option<Action>,
) -> Node {
    let mut column = Node::column().gap(tokens::GAP);
    for s in sections {
        let heading = organisms::section(s.heading);
        column = column.child(match s.id {
            Some(id) => heading.id(id),
            None => heading,
        });
        for p in s.paragraphs {
            column = column.child(components::explainer(p));
        }
    }
    let text = if c.m.is_wide() {
        organisms::capped(tokens::DOCUMENT_MAX_WIDTH, column)
    } else {
        column
    };
    let body = match row {
        Some(row) => Node::column()
            .gap(tokens::GAP)
            .child(text)
            .child(row.node(c.class())),
        None => text,
    };
    let frame = Frame::new(title, body);
    let frame = match action {
        Some(a) => frame.footer(c.actions(alloc::vec![a])),
        None => frame,
    };
    c.scrolling(frame)
}

/// A Document over one text: what the text is about as rows above it,
/// the text itself, and one way on.
pub struct Reading<'a> {
    /// The app bar's title.
    pub title: &'a str,
    /// What the text is about, one fact per row: the key, the address,
    /// the format. Each row is §4.1's, so a value row opens its Choice
    /// and a reference row opens Compare.
    pub rows: Vec<Row>,
    /// The text, one paragraph per line, so a message keeps the shape
    /// it was written in.
    pub paragraphs: Vec<String>,
    /// The ways on under the text: the hold that signs, the two a note
    /// carries, or none.
    pub actions: Vec<Action>,
}

/// §5 Document, with rows above the text: the message being signed or
/// checked (§5, Document).
///
/// The rows are the same components a Menu draws, so the key, the
/// address and the format read here as they do everywhere else, and the
/// text under them scrolls as any other Document's does. §2.1 allows the
/// prose because the text is the value being read, not the screen
/// explaining itself.
///
/// Made of the same rows a [`super::menu`] draws (§4.1, §4.2, §4.5) over
/// [`components::explainer`] (§4.12), with [`Chrome::actions`] under
/// them (§4.13).
pub fn reading(c: &Chrome, r: Reading<'_>) -> Node {
    let class = c.class();
    let mut body = Node::column().gap(tokens::menu_row_gap(class));
    body = body.children(r.rows.into_iter().map(|row| row.node(class)));
    let mut text = Node::column().gap(tokens::GAP);
    for p in r.paragraphs {
        text = text.child(components::explainer(p));
    }
    body = body.child(if c.m.is_wide() {
        organisms::capped(tokens::DOCUMENT_MAX_WIDTH, text)
    } else {
        text
    });
    c.scrolling(Frame::new(r.title, body).footer(c.actions(r.actions)))
}
