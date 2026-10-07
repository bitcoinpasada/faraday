//! §4.4 Identity and keys: the fingerprint, the key row, the key
//! context and the Sign-with chips.

use alloc::string::String;
use alloc::vec::Vec;

use crate::geom::SizeClass;
use crate::layout::{Align, Id, Node};
use crate::text::Font;
use crate::tokens;
use crate::widgets::{Icon, Tone, VALUE_SEPARATOR, Widget};

/// §4.4 Fingerprint: "Eight hex characters in mono, never split. The
/// key's name everywhere."
///
/// Never chunked: eight characters are read at a glance, and groups of
/// four would make a name look like a string to compare (§4.5).
pub fn fingerprint(text: impl Into<String>) -> Node {
    Node::widget(Widget::text(text, Font::mono(tokens::MONO), Tone::Text))
}

/// The glyph a value takes when it is a fingerprint, or several joined
/// by [`VALUE_SEPARATOR`]: the fingerprint glyph (`docs/PLANNING.md`
/// §16.131 rule 1). `None` for anything else — the placeholder for no
/// value, "Typed words", a member named by its place — so a value that
/// stands in for a key where there is none carries no glyph.
pub fn fingerprint_glyph(value: &str) -> Option<Icon> {
    let is_fingerprint = |s: &str| s.len() == 8 && s.bytes().all(|b| b.is_ascii_hexdigit());
    value
        .split(VALUE_SEPARATOR)
        .all(is_fingerprint)
        .then_some(Icon::Fingerprint)
}

/// A widget with the fingerprint glyph before its value where the
/// value is a fingerprint, and unchanged where it is not.
fn with_fingerprint_glyph(w: Widget, value: &str) -> Widget {
    match fingerprint_glyph(value) {
        Some(g) => w.with_value_glyph(g),
        None => w,
    }
}

/// §4.4 Key row: "The fingerprint glyph, the fingerprint, what the key
/// is made of, chevron. The Keys list and every key chooser."
///
/// The glyph says the eight hex characters are a fingerprint, which is
/// what a row of a list of keys states (`docs/PLANNING.md` §16.127
/// rule 1), and the fingerprint is read one character at a time, so it
/// is set in the mono face (§16.126).
pub fn key_row(
    class: SizeClass,
    id: Id,
    fingerprint: impl Into<String>,
    subtitle: impl Into<String>,
) -> Node {
    Node::widget(
        Widget::list_row(Some(id), fingerprint, Some(subtitle.into()), None)
            .with_icon(Icon::Fingerprint)
            .with_height(tokens::stacked_row(class))
            .with_mono_title(),
    )
}

/// §4.4 Key context: "A two-line row like every other row with a value:
/// 'Key' above, the fingerprint(s) below in mono, chevron. No chip, no
/// caret. Only on Sign entry and review, Verify and Explore, where the
/// screen acts on a key the user can switch. Never on Entry, never with
/// 'Sign with'. Tap opens the key chooser."
///
/// The same two-line row as a value row, a key row and a reference row,
/// at [`tokens::stacked_row`]: the key a screen acts on is a value like
/// any other, and a chip inside a row would be a second target on a row
/// that is already one. A multisig's fingerprints share the value line,
/// separated by the middle dot the rest of the interface uses.
pub fn key_context(
    class: SizeClass,
    id: Option<Id>,
    label: impl Into<String>,
    keys: &[String],
) -> Node {
    let value = keys.join(VALUE_SEPARATOR);
    let row = Widget::list_row(id, label, Some(value.clone()), None)
        .with_height(tokens::stacked_row(class))
        .with_value_below(true, Tone::Text);
    let row = with_fingerprint_glyph(row, &value);
    // A chip that opens nothing states the keys and no more, so it
    // carries no chevron (§4.1: tap opens).
    Node::widget(if id.is_some() {
        row
    } else {
        row.without_chevron()
    })
}

/// The same row drawn flat, for a Record screen: inside a table every
/// row is a table row (§4.11), so the key row there has no surface and
/// no indent, like the reference row beside it.
pub fn flat_key_context(
    class: SizeClass,
    id: Option<Id>,
    label: impl Into<String>,
    keys: &[String],
) -> Node {
    let value = keys.join(VALUE_SEPARATOR);
    let row = Widget::list_row(id, label, Some(value.clone()), None)
        .with_height(tokens::stacked_row(class))
        .with_value_below(true, Tone::Text)
        .flat();
    let row = with_fingerprint_glyph(row, &value);
    Node::widget(if id.is_some() {
        row
    } else {
        row.without_chevron()
    })
}

/// §4.4 Sign with: "Caption and a row of fingerprint chips on Confirm.
/// Tap toggles a key in a multisig."
///
/// The chips are the targets here, and only here: each one stands for
/// one key that will or will not sign, so the row cannot be one target.
pub fn sign_with(caption: impl Into<String>, keys: &[(Id, String, bool)]) -> Node {
    let mut chips = Node::row().gap(tokens::CHIP_GAP).align(Align::Center);
    for (id, label, on) in keys {
        chips = chips.child(Node::widget(with_fingerprint_glyph(
            Widget::chip(Some(*id), label.clone(), *on),
            label,
        )));
    }
    Node::column()
        .gap(tokens::GAP_SMALL)
        .child(Node::widget(Widget::text(
            caption,
            Font::regular(tokens::LABEL),
            Tone::Muted,
        )))
        .child(chips)
}

/// The fingerprints of a multisig as plain chips, for a table row that
/// names the keys without offering a choice: each chip is a fingerprint
/// and whether it is one of ours, which takes the accent (§4.4).
///
/// No ids: the row states who the cosigners are. Choosing among them is
/// [`sign_with`], and only the Hold screen does that.
///
/// A table's value column holds one fingerprint chip on `small` and two
/// elsewhere; more on a line run past the screen's edge.
pub fn key_chips(class: SizeClass, keys: &[(String, bool)]) -> Node {
    let per_line = match class {
        SizeClass::Small => 1,
        _ => KEY_CHIPS_PER_LINE,
    };
    Node::column().gap(tokens::CHIP_GAP).children(
        keys.chunks(per_line)
            .map(|line| {
                Node::row()
                    .gap(tokens::CHIP_GAP)
                    .align(Align::Center)
                    .children(
                        line.iter()
                            .map(|(k, ours)| {
                                Node::widget(with_fingerprint_glyph(
                                    Widget::compact_chip(None, k.clone(), *ours),
                                    k,
                                ))
                            })
                            .collect::<Vec<Node>>(),
                    )
            })
            .collect::<Vec<Node>>(),
    )
}

/// How many cosigner chips [`key_chips`] puts on one line above
/// `small`.
const KEY_CHIPS_PER_LINE: usize = 2;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::{Rect, Scale};
    use crate::layout::{LayoutCtx, solve};
    use crate::widgets::HitTarget;

    /// The four sizes `just snapshots` renders.
    const REFERENCE: [(Rect, u16, SizeClass); 4] = [
        (Rect::new(0, 0, 240, 320), 143, SizeClass::Small),
        (Rect::new(0, 0, 480, 640), 286, SizeClass::Small),
        (Rect::new(0, 0, 1080, 2340), 420, SizeClass::Mobile),
        (Rect::new(0, 0, 960, 640), 160, SizeClass::Wide),
    ];

    /// §4.4: "A two-line row like every other row with a value ... No
    /// chip, no caret." One [`tokens::stacked_row`] high on every class,
    /// with the whole row as the one target, for one key and for a
    /// multisig alike.
    #[test]
    fn the_whole_key_context_row_is_the_one_target() {
        let one = alloc::vec![String::from("73c5da0a")];
        let both = alloc::vec![String::from("73c5da0a"), String::from("0f1a2b3c")];
        for (area, dpi, class) in REFERENCE {
            let ctx = LayoutCtx::new(Scale::for_class(dpi, class), class);
            for keys in [&one, &both] {
                let row = key_context(class, Some(Id(1)), "Key", keys);
                let l = solve(&Node::column().child(row), area, &ctx);
                let targets: Vec<HitTarget> =
                    l.items.iter().filter_map(|p| p.hit).collect::<Vec<_>>();
                assert_eq!(targets, alloc::vec![HitTarget::Tap(Id(1))]);
            }
        }
    }

    /// §4.4: a multisig's fingerprints share the one value line.
    #[test]
    fn a_multisig_key_context_names_every_fingerprint() {
        let keys = alloc::vec![String::from("73c5da0a"), String::from("0f1a2b3c")];
        let row = key_context(SizeClass::Small, Some(Id(1)), "Key", &keys);
        match row.as_widget() {
            Some(Widget::ListRow { subtitle, .. }) => {
                let value = subtitle.as_ref().expect("a value");
                assert!(
                    value.contains("73c5da0a") && value.contains("0f1a2b3c"),
                    "{value}"
                );
            }
            _ => panic!("the key context is one row"),
        }
    }
}
