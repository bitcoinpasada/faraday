//! §4.3 Entry: the field, the candidate strip, the pads and the reserved
//! error line.
//!
//! The keyboard itself is [`crate::widgets::Widget::Keyboard`]; its
//! geometry lives in [`crate::widgets::keyboard`] and its numbers in
//! [`crate::tokens`]. What this module adds is the fixed geometry §2.5
//! asks for: "The PIN pad, the keyboard, the secret panel and the record
//! table are the same size in the same place every time. Content never
//! resizes a control."

use alloc::string::String;
use alloc::vec::Vec;

use crate::geom::SizeClass;
use crate::layout::{Align, Id, Justify, Node};
use crate::organisms;
use crate::text::{Font, TextAlign};
use crate::tokens;
use crate::widgets::keyboard::{self, KeyboardKind};
use crate::widgets::{Tone, Widget};

/// §4.3 Text field: "52 dp, single line — 40 dp on `small`, a letter
/// key's own height — the value being typed. Always directly above its
/// keyboard. The tail stays in view for long values. Nothing is typed
/// without a field."
///
/// `mono` picks the monospace face for a value made of characters — a
/// path, a hex string, a PIN's dots — and the proportional face for a
/// word. The height is [`tokens::field`] whatever the value is.
pub fn text_field(class: SizeClass, value: impl Into<String>, mono: bool) -> Node {
    let font = if mono {
        Font::mono(tokens::TITLE)
    } else {
        Font::regular(tokens::TITLE)
    };
    organisms::field_at(tokens::field(class), value, font, TextAlign::End)
}

/// The same field with its value centred: the dots of a PIN, which are
/// read as a count and not as a tail.
fn dots_field(class: SizeClass, value: impl Into<String>) -> Node {
    organisms::field_at(
        tokens::field(class),
        value,
        Font::mono(tokens::TITLE),
        TextAlign::Center,
    )
}

/// What a candidate strip shows: its words, the ids the two rows report
/// under, which of them is the one word left and which a tap has
/// selected.
pub struct Strip<'a> {
    /// The size class the strip is drawn for.
    pub class: SizeClass,
    /// The first row's strip.
    pub first: Id,
    /// The second row's strip, on the classes that have one.
    pub second: Id,
    /// The words that still match, in list order.
    pub words: &'a [String],
    /// One candidate left, which the strip outlines.
    pub accepting: bool,
    /// More candidates remain than the strip holds, so its last cell
    /// pages.
    pub more: bool,
    /// The list's words are one character each.
    pub one_char: bool,
    /// The cell a tap has selected, counting across both rows.
    pub selected: Option<usize>,
}

/// §4.3 Candidates: "Two rows of 3 at 36 dp (`small`), two rows of 4
/// (`mobile`), one row of natural-width chips (`wide`) between the field
/// and the keys. A tap on a candidate is the only thing that takes a
/// word. The lone candidate is outlined where a tap accepts."
///
/// `one_char` is a list whose words are one character — a Chinese one,
/// looked up by a Mandarin reading — and §4.3 gives it ten cells a row
/// and two rows on every class, at the keyboard's own key pitch. Two rows
/// of ten hold the largest tone group there is, so the strip shows a
/// whole group at once and its last cell almost never pages.
///
/// The second row is a strip of its own, because
/// [`crate::Action::Candidate`] numbers a cell within its strip: `first`
/// reports candidates 0 to [`tokens::candidates_per_row`] − 1 and
/// `second` reports the ones after them.
///
/// On `wide` a strip of words sits in a row rather than filling a column,
/// so it takes the width its words need: a pointer aims at a word, and
/// three words stretched across a desktop window are three targets a long
/// way from each other. A one-character strip is the keyboard's ten-key
/// row and keeps the width on every class.
///
/// The cells are a fixed grid: [`tokens::candidates_per_row`] of them on
/// every class but `wide`, whatever the count, the first cells filled and
/// the rest blank, so a cell does not change width from one word to the
/// next.
///
/// `accepting` outlines the one remaining candidate, which a tap takes
/// without choosing. It is the first cell of the grid, so a word typed
/// out in full is always tapped where the first candidate is. `selected` is the cell a tap has selected: on the
/// small panel a
/// candidate is selected by one tap and accepted by the next
/// (`docs/PLANNING.md` §16.50), and a selected cell is drawn as a chosen
/// cell is, on the accent colour.
pub fn candidate_strip(s: Strip<'_>) -> Node {
    let Strip {
        class,
        first,
        second,
        words,
        accepting,
        more,
        one_char,
        selected,
    } = s;
    let per_row = tokens::candidates_per_row(class, one_char);
    let cell = tokens::candidate_cell(class, one_char);
    // §4.3: the lone candidate is the first cell of the same grid, and
    // is outlined where a tap accepts, so the screen says what one tap
    // would take without reading as a chosen option. Where the strip is
    // tapped twice it is already selected, and the accent says it.
    let outlined = (accepting && words.len() == 1).then_some(0u8);
    // A one-character row is a row of keys: the rows meet, and the face
    // each cell paints inside itself is what separates them.
    let mut column = Node::column().gap(if one_char { 0.0 } else { tokens::CANDIDATE_GAP });
    let rows = tokens::candidate_rows(class, one_char);
    let last_row = rows - 1;
    // §16.121: one text size holds for the whole strip, so every row
    // carries the words of the row beside it and the size is decided
    // over both at once.
    let row_words = |row: usize| -> Vec<String> {
        words
            .iter()
            .skip(row * per_row)
            .take(per_row)
            .cloned()
            .collect()
    };
    for (row, id) in [first, second].into_iter().enumerate() {
        if row >= rows {
            break;
        }
        // §4.3: the last cell of the last row pages the strip when more
        // candidates remain than it holds.
        let strip = Node::widget(Widget::CandidateStrip {
            id,
            words: row_words(row),
            more: more && row == last_row,
            selected: selected
                .filter(|s| s / per_row == row)
                .map(|s| (s % per_row) as u8),
            outlined: outlined.filter(|_| row == 0),
        })
        .height(cell)
        .min_height(cell);
        column = column.child(if class == SizeClass::Wide && !one_char {
            Node::row().height(cell).child(strip)
        } else {
            strip
        });
    }
    column
}

/// Height in dp of a pad of `kind` on `class`, its bottom reserve — the
/// shell's `inset_bottom` included — with it. Fixed: the pad is the same
/// size in the same place whatever the screen above it says (§2.5).
fn pad_height(kind: KeyboardKind, class: SizeClass, inset_bottom_dp: f32) -> f32 {
    let key = keyboard::max_key_height(kind, class, tokens::PAD_MAX_WIDTH);
    keyboard::rows(kind) as f32 * key + tokens::key_bottom_reserve(inset_bottom_dp)
}

/// §4.3 PIN pad: "3 × 4 pad, fixed size, centred, 280 dp wide at most;
/// the dots field directly above it at the pad's width; the group
/// bottom-anchored on `mobile`."
///
/// The field and the pad are one group: the field sits inside the same
/// [`tokens::PAD_MAX_WIDTH`] cap, so it is exactly as wide as the pad
/// under it and the two read as one control. The dots are the field's
/// value, so the field keeps its [`tokens::field`] height at every
/// length. Nothing about the screen's title or the number of digits
/// typed can move the pad.
pub fn pin_pad(
    class: SizeClass,
    inset_bottom_dp: f32,
    id: Id,
    digits: usize,
    scramble: Option<u32>,
) -> Node {
    let dots: String = core::iter::repeat_n('\u{2022}', digits).collect();
    let pad = Node::widget(Widget::Keyboard {
        id,
        kind: KeyboardKind::Pin,
        enabled: keyboard::ALL_KEYS,
        scramble,
    });
    pad_group(
        class,
        inset_bottom_dp,
        KeyboardKind::Pin,
        None,
        Some(dots_field(class, dots)),
        pad,
    )
}

/// The one group §4.3 asks for: what the pad has typed so far directly
/// above the pad, all of it at the pad's width and centred together.
///
/// `above` is the dice and coin run — the masked entries, the progress
/// line and its bar — and `field` the PIN's dots. Both sit inside the
/// same [`tokens::PAD_MAX_WIDTH`] cap as the pad, so "the eye moves
/// nowhere between the pad and what it typed" (§4.3).
pub fn pad_group(
    class: SizeClass,
    inset_bottom_dp: f32,
    kind: KeyboardKind,
    above: Option<Node>,
    field: Option<Node>,
    pad: Node,
) -> Node {
    let mut column = Node::column().gap(tokens::GAP);
    if let Some(a) = above {
        column = column.child(a);
    }
    if let Some(f) = field {
        column = column.child(f);
    }
    column = column.child(pad.height(pad_height(kind, class, inset_bottom_dp)));
    organisms::pad_slot(column)
}

/// The entries so far of a dice or coin run, and how much of them is
/// shown (§4.3, §4.10).
pub struct Entries<'a> {
    /// One character per entry, oldest first: "3", "H".
    pub values: &'a [String],
    /// All of them are shown: a finger is on the run, or the eye is.
    pub revealed: bool,
    /// The newest one is still inside its half-second window
    /// ([`crate::UiState::flashing`]).
    pub flash: bool,
}

/// §4.3, §4.10: "the entries so far ... masked as a secret: the newest
/// entry shows for half a second, then masks; hold or the eye shows them
/// all. No colour on the groups."
///
/// Whatever the entropy becomes is a secret (§4.10's mask policy), so a
/// run of rolls masks like the words it turns into. The groups are five
/// long, which is how a person counts a run back, and every group is in
/// the same colour: a group is a place in the run, not a state.
pub fn entropy_entries(class: SizeClass, width_dp: f32, entries: &Entries<'_>) -> Node {
    let last = entries.values.len().saturating_sub(1);
    // A run longer than the width shows its most recent groups: a group
    // is a place in the run, so half a group says nothing, and the end
    // of the run is what the last entry went into.
    let groups = tokens::ENTROPY_GROUP * tokens::entropy_groups(width_dp, class);
    let first = entries.values.len().saturating_sub(groups);
    let mut text = String::new();
    for (i, value) in entries.values.iter().enumerate().skip(first) {
        if i > first && (i - first).is_multiple_of(tokens::ENTROPY_GROUP) {
            text.push(' ');
        }
        if entries.revealed || (entries.flash && i == last) {
            text.push_str(value);
        } else {
            text.push('\u{2022}');
        }
    }
    Node::widget(Widget::text(text, Font::mono(tokens::MONO), Tone::Text))
}

/// What §4.3 puts inside the pad group directly above a dice or coin
/// pad: "the entries so far in groups of five, masked as a secret ...
/// then the progress line '23 of 50 · 59 bits' and its bar."
///
/// The entries come first because they are what the last roll went into;
/// the line and the bar under them say how far the run has to go.
pub fn entropy_progress(
    class: SizeClass,
    width_dp: f32,
    count: impl Into<String>,
    fraction: f32,
    entries: Option<&Entries<'_>>,
) -> Node {
    let mut column = Node::column().gap(tokens::GAP_SMALL);
    if let Some(e) = entries {
        column = column.child(entropy_entries(class, width_dp, e));
    }
    column
        .child(Node::widget(Widget::text(
            count,
            Font::regular(tokens::LABEL),
            Tone::Muted,
        )))
        .child(Node::widget(Widget::ProgressBar { fraction }).height(tokens::BAR_HEIGHT))
}

/// Height in dp of [`entropy_progress`] at this class: the entries, the
/// progress line and the bar, with the dense gap between them.
pub fn entropy_progress_height(class: SizeClass) -> f32 {
    tokens::scaled_line(tokens::MONO, class)
        + tokens::GAP_SMALL
        + tokens::scaled_line(tokens::LABEL, class)
        + tokens::GAP_SMALL
        + tokens::BAR_HEIGHT
}

/// §4.3 Dice pad: "Fixed pads like the PIN pad." Six faces in two rows,
/// with the masked entries, the progress line and the bar inside the
/// group directly above them.
pub fn dice_pad(
    class: SizeClass,
    inset_bottom_dp: f32,
    id: Id,
    count: impl Into<String>,
    fraction: f32,
    entries: Option<&Entries<'_>>,
) -> Node {
    pad_group(
        class,
        inset_bottom_dp,
        KeyboardKind::Dice,
        Some(entropy_progress(
            class,
            tokens::PAD_MAX_WIDTH,
            count,
            fraction,
            entries,
        )),
        None,
        Node::widget(Widget::dice_pad(id)),
    )
}

/// §4.3 Coin pad: the same fixed pad with two sides instead of six
/// faces.
pub fn coin_pad(
    class: SizeClass,
    inset_bottom_dp: f32,
    id: Id,
    count: impl Into<String>,
    fraction: f32,
    entries: Option<&Entries<'_>>,
) -> Node {
    pad_group(
        class,
        inset_bottom_dp,
        KeyboardKind::Coin,
        Some(entropy_progress(
            class,
            tokens::PAD_MAX_WIDTH,
            count,
            fraction,
            entries,
        )),
        None,
        Node::widget(Widget::coin_pad(id)),
    )
}

/// The same reserved line saying a state rather than a fault — "Deck
/// 1" under the card pad — in the muted tone, so a line that is not a
/// fault is not drawn as one.
pub fn inline_caption(class: SizeClass, message: Option<String>) -> Node {
    Node::widget(
        Widget::text(
            message.unwrap_or_default(),
            Font::regular(tokens::CAPTION),
            Tone::Muted,
        )
        .align(TextAlign::Center),
    )
    .min_height(tokens::caption_line(class))
}

/// §4.3 Inline error: "One caption line under the field, always
/// reserved at caption height. Appears in the danger tone; nothing
/// moves."
///
/// Exactly one caption line and no padding around it. A line of text
/// measures its own line box whether or not it has anything in it, so
/// the empty line and the message are the same rectangle;
/// [`tokens::caption_line`] is the floor under that, at the class's own
/// type scale, so the keyboard under the line never shifts and the
/// message is never cut where the ramp is largest.
pub fn inline_error(class: SizeClass, message: Option<String>) -> Node {
    Node::widget(
        Widget::text(
            message.unwrap_or_default(),
            Font::regular(tokens::CAPTION),
            Tone::Danger,
        )
        .align(TextAlign::Center),
    )
    .min_height(tokens::caption_line(class))
}

/// The bottom-anchored group §4.3 describes: the field, its candidates
/// or its error line, and the keyboard as one block, with spare height
/// above the block rather than inside it.
pub fn entry_group(above: Node, keyboard: Node) -> Node {
    Node::column()
        .justify(Justify::End)
        .align(Align::Stretch)
        .gap(tokens::ENTRY_DOCK_GAP)
        .child(above)
        .child(keyboard)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::{Rect, Scale};
    use crate::layout::{LayoutCtx, solve};
    use crate::organisms;

    /// The four sizes `just snapshots` renders: the smallest supported
    /// panel, the 2.8" reference panel, a phone, and the desktop window.
    const REFERENCE: [(Rect, u16, SizeClass); 4] = [
        (Rect::new(0, 0, 240, 320), 143, SizeClass::Small),
        (Rect::new(0, 0, 480, 640), 286, SizeClass::Small),
        (Rect::new(0, 0, 1080, 2340), 420, SizeClass::Mobile),
        (Rect::new(0, 0, 960, 640), 160, SizeClass::Wide),
    ];

    /// §4.3: "Same geometry on the lock screen and both set-PIN steps."
    /// The title above it and the number of digits typed are content;
    /// §2.5 says content never resizes a control.
    #[test]
    fn the_pin_pad_keeps_one_rectangle_whatever_the_screen_says() {
        for (area, dpi, class) in REFERENCE {
            let ctx = LayoutCtx::new(Scale::for_class(dpi, class), class);
            let mut seen: Option<Rect> = None;
            for title in ["Enter your PIN", "Set a PIN", "Repeat it"] {
                for digits in [0, 2, 8] {
                    let screen = organisms::action_screen(
                        Some(Id(1)),
                        None,
                        title,
                        organisms::Body::Fill,
                        pin_pad(class, 0.0, Id(2), digits, None).weight(1.0),
                        alloc::vec![Node::widget(crate::widgets::Widget::button(
                            Id(3),
                            "Continue",
                            crate::widgets::ButtonStyle::Primary,
                        ))],
                    );
                    let pad = solve(&screen, area, &ctx).rect(Id(2)).expect("the pad");
                    match seen {
                        None => seen = Some(pad),
                        Some(first) => assert_eq!(
                            pad, first,
                            "{title} with {digits} digits moved the pad at {}x{}",
                            area.w, area.h
                        ),
                    }
                }
            }
            let pad = seen.expect("a pad");
            assert!(
                !pad.is_empty(),
                "the pad has a rectangle at {}x{}",
                area.w,
                area.h
            );
        }
    }

    /// §4.3: "One caption line under the field, always reserved.
    /// Appears in the danger tone; nothing moves."
    #[test]
    fn nothing_moves_when_an_error_appears_under_the_field() {
        for (area, dpi, class) in REFERENCE {
            let ctx = LayoutCtx::new(Scale::for_class(dpi, class), class);
            let line_of = |message: Option<String>| {
                solve(
                    &Node::column().child(inline_error(class, message).id(Id(1))),
                    area,
                    &ctx,
                )
                .rect(Id(1))
                .expect("the error line")
            };
            let empty = line_of(None);
            let filled = line_of(Some(String::from("Not a word in the list")));
            assert_eq!(empty, filled, "at {}x{}", area.w, area.h);
        }
    }

    /// §4.3, §4.10: "the entries so far in groups of five, masked as a
    /// secret: the newest entry shows for half a second, then masks;
    /// hold or the eye shows them all."
    #[test]
    fn dice_entries_mask_in_groups_of_five() {
        let values: Vec<String> = ["3", "6", "1", "1", "4", "2", "5"]
            .iter()
            .map(|v| String::from(*v))
            .collect();
        let text = |e: &Entries<'_>| match entropy_entries(
            SizeClass::Small,
            tokens::PAD_MAX_WIDTH - 2.0 * tokens::PAD,
            e,
        )
        .as_widget()
        {
            Some(Widget::Text { text, .. }) => text.clone(),
            _ => panic!("the entries are one line of text"),
        };
        // Masked, with the newest still inside its window.
        assert_eq!(
            text(&Entries {
                values: &values,
                revealed: false,
                flash: true,
            }),
            "••••• •5"
        );
        // A run wider than the pad keeps its most recent groups whole:
        // §4.3 groups by five, and half a group is not a place in the
        // run. Fifty rolls on a 268 dp panel are four of them.
        let fifty: Vec<String> = (0..50).map(|i| alloc::format!("{}", i % 6 + 1)).collect();
        let groups =
            tokens::entropy_groups(tokens::PAD_MAX_WIDTH - 2.0 * tokens::PAD, SizeClass::Small);
        let run = text(&Entries {
            values: &fifty,
            revealed: false,
            flash: false,
        });
        assert_eq!(
            run.split(' ').count(),
            groups,
            "{groups} groups fit the pad: {run:?}"
        );
        for group in run.split(' ') {
            assert_eq!(
                group.chars().count(),
                tokens::ENTROPY_GROUP,
                "a group is cut short: {run:?}"
            );
        }
        // The window closed: every entry is a bullet.
        assert_eq!(
            text(&Entries {
                values: &values,
                revealed: false,
                flash: false,
            }),
            "••••• ••"
        );
        // Held, or shown by the eye.
        assert_eq!(
            text(&Entries {
                values: &values,
                revealed: true,
                flash: false,
            }),
            "36114 25"
        );
    }
}
