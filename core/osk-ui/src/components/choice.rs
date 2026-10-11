//! §4.2 Choice: the checked list, the setting, the pair, the toggle and
//! the value row.
//!
//! "Rows of more than two chips are not used on any class." One idiom
//! everywhere: a list of rows, checked and then confirmed. The one
//! exception §2.4 allows is a [`pair`] of two short options.

use alloc::string::String;
use alloc::vec::Vec;

use crate::geom::SizeClass;
use crate::layout::{Id, Node};
use crate::tokens;
use crate::widgets::{ButtonStyle, Icon, Tone, Widget};

/// §4.2 Choice: "A list of full-width rows, one option per row; the
/// chosen row carries an accent check; Continue at the bottom. Tap
/// checks (the list keeps its scroll position)."
///
/// `chosen` draws the accent check. A tap reports the row and nothing
/// else — the list is rebuilt with the check moved, and [`crate::UiState`]
/// keeps the scroll offset it already had, so the row under the finger
/// stays under the finger.
///
/// `id` of `None` is an option that is not available: the row is dimmed,
/// has no hit target, and `note` says why in two or three words — under
/// the label, on every class, because §4.11 makes a dead row a two-line
/// row like every other row that carries a value. A dead row with no
/// note is one line at the class's own height: nothing is written under a
/// control that is dimmed because the feature is not built.
pub fn choice_row(
    class: SizeClass,
    id: Option<Id>,
    label: impl Into<String>,
    note: Option<String>,
    chosen: bool,
) -> Node {
    let height = if note.is_some() {
        tokens::stacked_row(class)
    } else {
        tokens::choice_row(class)
    };
    Node::widget(
        Widget::list_row(id, label, note, None)
            .with_height(height)
            .with_enabled(id.is_some())
            .with_selected(chosen)
            .without_chevron(),
    )
}

/// A [`choice_row`] whose label is read character by character — a key
/// a chooser dims, named by its fingerprint — set in the mono face.
pub fn mono_choice_row(
    class: SizeClass,
    id: Option<Id>,
    label: impl Into<String>,
    note: Option<String>,
    chosen: bool,
) -> Node {
    let height = if note.is_some() {
        tokens::stacked_row(class)
    } else {
        tokens::choice_row(class)
    };
    Node::widget(
        Widget::list_row(id, label, note, None)
            .with_height(height)
            .with_enabled(id.is_some())
            .with_selected(chosen)
            .without_chevron()
            .with_mono_title(),
    )
}

/// §4.2 Choice with §4.8's caution: a choice row that can be taken and
/// that says, at its trailing edge and in the caution tone, what taking
/// it means. The row keeps the one-line height every other option has,
/// so the list does not change shape around it.
pub fn choice_caution_row(
    class: SizeClass,
    id: Id,
    label: impl Into<String>,
    caution: impl Into<String>,
    chosen: bool,
) -> Node {
    Node::widget(
        Widget::list_row(Some(id), label, None, Some(caution.into()))
            .with_trailing_tone(Tone::Caution)
            .with_height(tokens::choice_row(class))
            .with_selected(chosen)
            .without_chevron(),
    )
}

/// §4.2 Choice and §4.6 Path editor: a choice row whose option carries a
/// value under its name — "SegWit" above, `m/84h/0h/0h` below — with the
/// accent check the rest of the list carries.
///
/// The same two-line row as every other row with a value
/// ([`tokens::stacked_row`]); `mono` draws the value in the monospace
/// face, which is what §4.6 gives a path.
pub fn choice_value_row(
    class: SizeClass,
    id: Id,
    label: impl Into<String>,
    value: impl Into<String>,
    mono: bool,
    chosen: bool,
) -> Node {
    Node::widget(
        Widget::list_row(Some(id), label, Some(value.into()), None)
            .with_height(tokens::stacked_row(class))
            .with_selected(chosen)
            .with_value_below(mono, Tone::Text)
            .without_chevron(),
    )
}

/// §4.6 Path editor preset: [`choice_value_row`] at the height and in
/// the list gap the editor's block can spare on this class
/// ([`tokens::preset_row`]), so the two groups and their labels fit the
/// space above the field without scrolling.
pub fn preset_row(
    class: SizeClass,
    id: Id,
    label: impl Into<String>,
    value: impl Into<String>,
    chosen: bool,
) -> Node {
    Node::widget(
        Widget::list_row(Some(id), label, Some(value.into()), None)
            .with_height(tokens::preset_row(class))
            .with_selected(chosen)
            .with_value_below(true, Tone::Text)
            .without_chevron()
            .tight(),
    )
}

/// §4.2 Choice and §4.4 Key row: the row of a key chooser that is a
/// Choice — the fingerprint glyph, the fingerprint above, what the key
/// is made of below, and the accent check the rest of the list carries.
///
/// A key chooser is a choice like any other, so the row keeps the key
/// row's two lines and takes the check instead of the chevron: one
/// rendering per content kind (§2.2), and the check says which key
/// Continue will take.
pub fn key_choice_row(
    class: SizeClass,
    id: Id,
    icon: Option<Icon>,
    fingerprint: impl Into<String>,
    subtitle: impl Into<String>,
    chosen: bool,
    (mono, mono_subtitle): (bool, bool),
) -> Node {
    let mut row = Widget::list_row(Some(id), fingerprint, Some(subtitle.into()), None)
        .with_height(tokens::stacked_row(class))
        .with_selected(chosen)
        .without_chevron();
    if let Some(icon) = icon {
        row = row.with_icon(icon);
    }
    if mono {
        row = row.with_mono_title();
    }
    if mono_subtitle {
        row = row.with_mono_subtitle();
    }
    Node::widget(row)
}

/// §4.2 Setting: "The same rows with the check on the current value; no
/// Continue. Tap applies at once and the screen stays."
pub fn setting_row(class: SizeClass, id: Id, label: impl Into<String>, current: bool) -> Node {
    choice_row(class, Some(id), label, None, current)
}

/// A column of [`choice_row`]s at the class's row height and gap.
pub fn choice_list(class: SizeClass, rows: Vec<Node>) -> Node {
    Node::column()
        .gap(tokens::menu_row_gap(class))
        .children(rows)
}

/// A column of [`setting_row`]s: the same list, with the mark on the
/// current value and no Continue under it.
pub fn setting_list(class: SizeClass, rows: Vec<Node>) -> Node {
    choice_list(class, rows)
}

/// §4.2 Pair: "Two equal buttons side by side, the chosen one on the
/// accent. Exactly two short options: Receive | Change, One code |
/// Animated where a toggle does not fit."
///
/// Two buttons and never three: §2.4 gives every other option list a row
/// of its own, and a pair is the one shape a finger can take in without
/// reading down a column.
pub fn pair(
    left: (Id, impl Into<String>),
    right: (Id, impl Into<String>),
    right_chosen: bool,
) -> Node {
    let style = |chosen: bool| {
        if chosen {
            ButtonStyle::Primary
        } else {
            ButtonStyle::Secondary
        }
    };
    Node::row()
        .gap(tokens::GAP)
        .child(
            Node::widget(Widget::button(left.0, left.1, style(!right_chosen)))
                .height(tokens::CTA)
                .weight(1.0),
        )
        .child(
            Node::widget(Widget::button(right.0, right.1, style(right_chosen)))
                .height(tokens::CTA)
                .weight(1.0),
        )
}

/// §4.2 Toggle: "A row with a switch. A boolean setting; dimmed with a
/// reason when forced."
///
/// The row is the target, not the switch: a switch is a small thing to
/// hit, and §2.4 gives the whole row to the finger. `reason` is the two
/// or three words §4.11 puts under a dead row — "too dense" — and giving
/// one dims the row and takes its hit target away, because a control the
/// screen has already decided is not a control.
pub fn toggle_row(
    class: SizeClass,
    id: Id,
    label: impl Into<String>,
    on: bool,
    reason: Option<String>,
) -> Node {
    let height = if reason.is_some() {
        tokens::stacked_row(class)
    } else {
        tokens::choice_row(class)
    };
    toggle_row_at(height, true, id, label, on, reason)
}

/// The same toggle at a height of its own, for the one screen that has to
/// buy the row back from somewhere: the QR screen on `small`, where the
/// class's square leaves 44 dp (§4.9, [`tokens::qr_toggle_row`]).
///
/// `below` puts the reason under the label, which is where §4.11 puts it;
/// the one row that cannot afford a second line passes `false` and the
/// reason sits beside the label instead.
pub fn toggle_row_at(
    height_dp: f32,
    below: bool,
    id: Id,
    label: impl Into<String>,
    on: bool,
    reason: Option<String>,
) -> Node {
    let dead = reason.is_some();
    let (subtitle, trailing) = if below {
        (reason, None)
    } else {
        (None, reason)
    };
    Node::widget(
        Widget::list_row(
            if dead { None } else { Some(id) },
            label,
            subtitle,
            trailing,
        )
        .with_height(height_dp)
        .with_enabled(!dead)
        .with_switch(on),
    )
}

/// §4.2 Value row: "Label above, value below, chevron. A mode shown as
/// its value; tap opens the Choice screen for it."
///
/// This is also §4.6's path row and script-type value, and §4.8's state
/// value, which is the same row in the caution tone. One height for all
/// of them ([`tokens::stacked_row`]), so a list that mixes them reads as
/// one list.
pub fn value_row(
    class: SizeClass,
    id: Id,
    label: impl Into<String>,
    value: impl Into<String>,
    tone: Tone,
) -> Node {
    stacked(class, id, label, value, false, tone)
}

/// The same row for a value made of characters rather than words: a
/// path, a fingerprint, an elided key (§4.6).
pub fn mono_value_row(
    class: SizeClass,
    id: Id,
    label: impl Into<String>,
    value: impl Into<String>,
    tone: Tone,
) -> Node {
    stacked(class, id, label, value, true, tone)
}

/// The one two-line row every kind of value row is built from.
fn stacked(
    class: SizeClass,
    id: Id,
    label: impl Into<String>,
    value: impl Into<String>,
    mono: bool,
    tone: Tone,
) -> Node {
    Node::widget(
        Widget::list_row(Some(id), label, Some(value.into()), None)
            .with_height(tokens::stacked_row(class))
            .with_value_below(mono, tone),
    )
}

/// The two-line row whose value is a fingerprint: a value row that
/// opens a Choice where it has an id, a fact row where it has none, the
/// value in the monospace face with the fingerprint glyph before it
/// (`docs/PLANNING.md` §16.131).
pub fn fingerprint_row(
    class: SizeClass,
    id: Option<Id>,
    label: impl Into<String>,
    value: impl Into<String>,
) -> Node {
    let value = value.into();
    let row = Widget::list_row(id, label, Some(value.clone()), None)
        .with_height(tokens::stacked_row(class))
        .with_value_below(true, Tone::Text);
    let row = if id.is_some() {
        row
    } else {
        row.without_chevron()
    };
    Node::widget(match super::identity::fingerprint_glyph(&value) {
        Some(g) => row.with_value_glyph(g),
        None => row,
    })
}

/// §4.12 Value: the same two-line row for a fact a screen states and
/// nothing opens — the derivation path a wallet export is taken at.
///
/// No id, so the row is not a target and carries no chevron: §4.2's
/// value row promises a Choice screen behind it, and a row that promises
/// one and opens nothing is worse than a row that promises nothing. The
/// tone is the value's, so a state a person should act on — a weak
/// session key — is stated in the danger tone (§4.8).
pub fn fact_row(
    class: SizeClass,
    label: impl Into<String>,
    value: impl Into<String>,
    mono: bool,
    tone: Tone,
) -> Node {
    Node::widget(
        Widget::list_row(None, label, Some(value.into()), None)
            .with_height(tokens::stacked_row(class))
            .with_value_below(mono, tone)
            .without_chevron(),
    )
}

/// The same row drawn flat, for a Record's table: inside a table every
/// row is a table row (§4.11), so it has no surface and its label starts
/// where every other label in the table starts.
pub fn flat_fact_row(
    class: SizeClass,
    label: impl Into<String>,
    value: impl Into<String>,
    mono: bool,
) -> Node {
    Node::widget(
        Widget::list_row(None, label, Some(value.into()), None)
            .with_height(tokens::stacked_row(class))
            .with_value_below(mono, Tone::Text)
            .without_chevron()
            .flat(),
    )
}

/// The same value row with a leading icon, for a menu that mixes rows
/// that open a screen with rows that state a value.
pub fn value_row_with_icon(
    class: SizeClass,
    id: Id,
    icon: Icon,
    label: impl Into<String>,
    value: impl Into<String>,
    tone: Tone,
) -> Node {
    Node::widget(
        Widget::list_row(Some(id), label, Some(value.into()), None)
            .with_height(tokens::stacked_row(class))
            .with_icon(icon)
            .with_value_below(false, tone),
    )
}
