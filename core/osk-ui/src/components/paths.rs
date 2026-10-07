//! §4.6 Derivation paths and script types.
//!
//! "No per-level crumbs": a path is a string wherever it labels
//! something, and a value row where it opens the editor.

use alloc::string::String;

use crate::geom::SizeClass;
use crate::layout::{Id, Node};
use crate::text::Font;
use crate::tokens;
use crate::widgets::{Tone, Widget};

use super::choice::{mono_value_row, value_row};

/// §4.6 Path: "A mono string: `m/84h/0h/0h/0/0`. Read-only wherever it
/// labels something."
pub fn path(text: impl Into<String>) -> Node {
    Node::widget(Widget::text(text, Font::mono(tokens::MONO), Tone::Text))
}

/// §4.6 Path row: "As a row: label 'Path' above, the string below,
/// chevron when it opens the editor."
pub fn path_row(class: SizeClass, id: Id, label: impl Into<String>, value: &str) -> Node {
    mono_value_row(class, id, label, String::from(value), Tone::Text)
}

/// §4.6 Script type: "A value row: 'Script type' above, 'SegWit' below,
/// chevron; chosen on a Choice screen. One vocabulary: SegWit, Taproot,
/// Nested, Legacy. Never 'BIP-84' or 'p2wpkh' on a working screen."
pub fn script_type_row(class: SizeClass, id: Id, label: impl Into<String>, value: &str) -> Node {
    value_row(class, id, label, String::from(value), Tone::Text)
}
