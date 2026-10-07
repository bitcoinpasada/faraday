//! §4.5 Long strings: two kinds, by what the person does with them.
//!
//! A comparison string is read character by character, so it is shown
//! whole. A reference string is only identified, so it is elided to its
//! head and tail and the whole of it lives one tap away on the Compare
//! screen.

use alloc::string::String;

use crate::geom::SizeClass;
use crate::layout::{Id, LayoutCtx, Node};
use crate::organisms;
use crate::tokens;
use crate::widgets::{Tone, Widget};

/// §4.5 Comparison string: "Whole, mono, chunked in fours with
/// alternating colour, largest size that fits, on as many lines as it
/// takes. Addresses on Verify, Addresses and Sign outputs; a txid. Never
/// elided."
///
/// §5 Transcribe: "at the largest type that fits: a comparison string
/// steps up the mono sizes the tokens name until the next step would not
/// fit the block's width and height in groups of four, and never past
/// the class's ceiling." The ramp runs from
/// [`tokens::comparison_max`] down to [`tokens::CHUNK_MIN_SIZE`], and
/// the planner takes the first step whose groups fit the rectangle the
/// block gives it, so a 111-character key on a 268 dp panel is read at
/// the mono token or below and a short string on a phone at the phone's
/// ceiling.
///
/// `center` puts the lines under the value they belong to rather than at
/// the left margin, for a code screen where the string sits under the
/// code.
pub fn comparison_string(class: SizeClass, id: Id, text: impl Into<String>, center: bool) -> Node {
    Node::widget(comparison(class, id, text, center))
}

/// The same string keeping its rectangle and drawing nothing, for the
/// masked half of a Secret screen: the same ramp and the same box, so
/// §4.10's "identical masked and revealed" holds whatever size the
/// revealed string takes.
pub fn masked_comparison_string(
    class: SizeClass,
    id: Id,
    text: impl Into<String>,
    center: bool,
) -> Node {
    Node::widget(comparison(class, id, text, center).masked())
}

/// The one chunked-string widget both forms are built from.
fn comparison(class: SizeClass, id: Id, text: impl Into<String>, center: bool) -> Widget {
    let w = Widget::chunked_between(
        id,
        text,
        tokens::comparison_max(class),
        tokens::CHUNK_MIN_SIZE,
    );
    if center { w.centered() } else { w }
}

/// A string elided to its head and tail, in groups of four: §4.5's
/// `xpub 6CUG … Au3f DVmz`, the first [`tokens::ELIDE_HEAD`] and last
/// [`tokens::ELIDE_TAIL`] characters as groups of
/// [`tokens::CHUNK_GROUP`] with [`tokens::ELLIPSIS`] between them.
///
/// The groups are the ones the Compare screen shows the whole string in,
/// so an eye that has checked four characters there checks the same four
/// here. A string that is not longer than its own elision is returned
/// whole, in the same groups.
pub fn elide(text: &str) -> String {
    let n = text.chars().count();
    if n <= tokens::ELIDE_HEAD + tokens::ELIDE_TAIL {
        return group(text.chars());
    }
    let mut out = group(text.chars().take(tokens::ELIDE_HEAD));
    out.push(' ');
    out.push_str(tokens::ELLIPSIS);
    out.push(' ');
    out.push_str(&group(text.chars().skip(n - tokens::ELIDE_TAIL)));
    out
}

/// Characters in groups of [`tokens::CHUNK_GROUP`], separated by a
/// space.
fn group(chars: impl Iterator<Item = char>) -> String {
    let mut out = String::new();
    for (i, c) in chars.enumerate() {
        if i > 0 && i % tokens::CHUNK_GROUP == 0 {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// §4.5 Reference row: "A row: label above, the elided value below as
/// `xpub 6CUG … Au3f DVmz` (first 8 and last 8, in groups of four),
/// chevron. The whole row is the target."
///
/// Every long string wherever it appears inside another screen — xpubs,
/// master keys, seed hex, signatures, txids, an address in a
/// transaction review. Tap opens the Compare screen, where the whole
/// string is read.
///
/// The value is under the label on every class: an elided key is a value
/// like any other (§4.1), and the row is [`tokens::stacked_row`] high
/// like every other row that carries one.
pub fn reference_row(class: SizeClass, id: Id, label: impl Into<String>, value: &str) -> Node {
    Node::widget(
        Widget::list_row(Some(id), label, Some(elide(value)), None)
            .with_height(reference_row_height(class))
            .with_value_below(true, Tone::Text),
    )
}

/// §4.11 Record: "A long string is a reference row drawn flat inside the
/// table: the label in the label column, the elided value on the line
/// under it, a chevron at the right, no surface."
///
/// The same row as [`reference_row`], without the card under it and
/// without the indent that card carries, so the label starts where every
/// other label in the table starts and the table reads as one table.
pub fn flat_reference_row(class: SizeClass, id: Id, label: impl Into<String>, value: &str) -> Node {
    Node::widget(
        Widget::list_row(Some(id), label, Some(elide(value)), None)
            .with_height(reference_row_height(class))
            .with_value_below(true, Tone::Text)
            .flat(),
    )
}

/// Height in dp of a [`reference_row`], which is also the height of a
/// [`super::secrets::secret_row`] and of every other row whose value
/// sits under its label.
pub fn reference_row_height(class: SizeClass) -> f32 {
    tokens::stacked_row(class)
}

/// §4.5 Descriptor: "Structure, not a string: function, origin in the
/// accent, the key as an elided chip, suffix and checksum whole. On
/// export, Explore and a scanned config."
///
/// The key inside it is elided by the same rule as a reference string,
/// and opens the same Compare screen when `key_id` is given.
pub fn descriptor(ctx: &LayoutCtx, avail_dp: f32, text: &str, key_id: Option<Id>) -> Node {
    organisms::descriptor(ctx, avail_dp, text, key_id)
}

/// The label a Compare screen puts over the string it shows: the kind of
/// string, in the label token, so the value under it is the only thing
/// in the mono face.
pub fn compare_label(text: impl Into<String>) -> Node {
    let text = text.into();
    let font = super::text::label_font(&text);
    Node::widget(Widget::text(text, font, Tone::Muted))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::{Rect, Scale};
    use crate::layout::{LayoutCtx, solve};
    use crate::widgets::HitTarget;

    const XPUB: &str = "xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz";

    /// The four sizes `just snapshots` renders.
    const REFERENCE: [(Rect, u16, SizeClass); 4] = [
        (Rect::new(0, 0, 240, 320), 143, SizeClass::Small),
        (Rect::new(0, 0, 480, 640), 286, SizeClass::Small),
        (Rect::new(0, 0, 1080, 2340), 420, SizeClass::Mobile),
        (Rect::new(0, 0, 960, 640), 160, SizeClass::Wide),
    ];

    /// §4.5: "the elided value below as `xpub 6CUG … Au3f DVmz` (first
    /// 8 and last 8, in groups of four)."
    #[test]
    fn an_elided_value_is_two_groups_of_four_an_ellipsis_and_two_more() {
        let short = elide(XPUB);
        assert_eq!(short, "xpub 6CUG … Au3f DVmz");
        // Eight characters at each end, in groups of four, and the
        // ellipsis between them.
        let head: String = XPUB.chars().take(tokens::ELIDE_HEAD).collect();
        let tail: String = XPUB
            .chars()
            .skip(XPUB.chars().count() - tokens::ELIDE_TAIL)
            .collect();
        let bare: String = short.chars().filter(|c| *c != ' ').collect();
        assert!(bare.starts_with(&head), "{short}");
        assert!(bare.ends_with(&tail), "{short}");
        assert!(short.contains(tokens::ELLIPSIS), "{short}");
        for group in short.split(' ').filter(|g| *g != tokens::ELLIPSIS) {
            assert_eq!(group.chars().count(), tokens::CHUNK_GROUP, "{short}");
        }
        // A string no longer than its own elision stays whole, in the
        // same groups.
        assert_eq!(elide("73c5da0a"), "73c5 da0a");
    }

    /// §4.5: "The whole row is the target." One row high on every
    /// class, and nothing inside it is a target of its own.
    #[test]
    fn the_whole_reference_row_is_the_target_and_nothing_inside_it_is() {
        for (area, dpi, class) in REFERENCE {
            let ctx = LayoutCtx::new(Scale::for_class(dpi, class), class);
            let row = reference_row(class, Id(1), "Account key", XPUB);
            let l = solve(&Node::column().child(row), area, &ctx);
            let r = l.rect(Id(1)).expect("the row");
            let target = l
                .items
                .iter()
                .find(|p| p.hit == Some(HitTarget::Tap(Id(1))))
                .expect("a target");
            assert_eq!(target.rect, r, "the whole row is the target");
            // Nothing inside it is a target of its own: on a touch class
            // the chip alone is never aimed at.
            assert_eq!(l.items.iter().filter(|p| p.hit.is_some()).count(), 1);
        }
    }
}
