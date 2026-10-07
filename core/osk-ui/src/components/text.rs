//! §4.12 Text: the five text roles, as constructors that fix the token
//! and the tone so no screen picks a font.

use alloc::string::String;

use crate::layout::Node;
use crate::text::{Font, TextAlign};
use crate::tokens;
use crate::widgets::{Tone, Widget};

/// §4.12 Title: "Title token in the app bar."
///
/// A heading inside a body uses the same token, so a screen that carries
/// its own heading and one that puts it in the bar read alike.
pub fn title(text: impl Into<String>) -> Node {
    Node::widget(Widget::paragraph(
        text,
        Font::semibold(tokens::TITLE),
        Tone::Text,
    ))
}

/// §4.12 Label: "Label token, muted. Record labels, field labels, 'Sign
/// with', 'Using'."
pub fn label(text: impl Into<String>) -> Node {
    let text = text.into();
    let font = label_font(&text);
    // A label wider than its column wraps at a space rather than being
    // cut at the column's edge.
    Node::widget(Widget::paragraph(text, font, Tone::Muted))
}

/// The label token's face for `text`: mono where the label names a key —
/// a wallet called by its key's fingerprint, "#0 · 73c5da0a" — which is
/// read character by character (§3), the text face otherwise.
pub(crate) fn label_font(text: &str) -> Font {
    if crate::widgets::names_a_key(text) {
        Font::mono(tokens::LABEL)
    } else {
        Font::regular(tokens::LABEL)
    }
}

/// §4.12 Value: "Body or mono token. Everything a person reads or
/// compares."
///
/// `mono` picks the monospace face, which is what a value made of
/// characters rather than words needs: a fingerprint, a path, an amount.
/// `tone` is [`Tone::Text`] unless the value states a condition, which
/// §4.8 draws in the caution tone as a row value rather than a badge.
///
/// A value wraps rather than being cut: §4.11 gives a value that needs
/// the width its own rows, and a fee of two parts is one of them on a
/// 268 dp panel.
pub fn value(text: impl Into<String>, mono: bool, tone: Tone) -> Node {
    let font = if mono {
        Font::mono(tokens::MONO)
    } else {
        Font::regular(tokens::BODY)
    };
    Node::widget(Widget::paragraph(text, font, tone))
}

/// A value that is a fingerprint, or several: one line in the
/// monospace face with the fingerprint glyph before each
/// (`docs/PLANNING.md` §16.131). A placeholder in its place is drawn as
/// [`value`] draws a mono value.
pub fn fingerprint_value(text: impl Into<String>, tone: Tone) -> Node {
    let text = text.into();
    match super::identity::fingerprint_glyph(&text) {
        Some(g) => {
            Node::widget(Widget::text(text, Font::mono(tokens::MONO), tone).with_value_glyph(g))
        }
        None => value(text, true, tone),
    }
}

/// §4.11's dimmed reason as a line of its own: caption, muted, one line.
/// A row carries its reason inside itself; this is for the one place a
/// value is missing rather than dead ([`super::progress::no_value`]).
pub fn reason(text: impl Into<String>) -> Node {
    Node::widget(Widget::text(
        text,
        Font::regular(tokens::CAPTION),
        Tone::Muted,
    ))
}

/// §4.12 Explainer: "Body, a short paragraph, on a Learn page. Never on
/// a working screen and never behind a control on one."
///
/// The component exists so a Learn page's prose has one rendering.
pub fn explainer(text: impl Into<String>) -> Node {
    Node::widget(
        Widget::paragraph(text, Font::regular(tokens::BODY), Tone::Text).align(TextAlign::Start),
    )
}
