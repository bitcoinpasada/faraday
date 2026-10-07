//! §5 Choice: "rows with a check, Continue". Used by source, word count,
//! language, script type, arrival, unit, network and timers.

use alloc::string::String;
use alloc::vec::Vec;

use crate::components;
use crate::geom::SizeClass;
use crate::layout::{Id, Node};
use crate::tokens;
use crate::widgets::Icon;

use super::{Action, Chrome, Frame};

/// One option of a [`choice`].
pub struct Item {
    /// Application id. `None` is an option that is not available: the
    /// row is dimmed, has no hit target, and `reason` says why (§4.11).
    pub id: Option<Id>,
    /// The option.
    pub label: String,
    /// Why a dimmed option is dimmed, in two or three words.
    pub reason: Option<String>,
    /// Whether this row carries the accent check.
    pub chosen: bool,
    /// The second line of a key chooser's row: "mainnet · passphrase",
    /// "with passphrase". `None` on an ordinary option, whose label is
    /// the whole of it.
    pub subtitle: Option<String>,
    /// §4.8: what the person should weigh about an option they can
    /// still take — "trusts this device" — at the row's trailing edge,
    /// in the caution tone.
    pub caution: Option<String>,
    /// The glyph in front of the option, which a key chooser's row
    /// carries and an ordinary option does not.
    pub icon: Option<Icon>,
    /// Whether the label is read character by character — a key's
    /// fingerprint — and so is set in the mono face.
    pub mono: bool,
    /// The same for the second line: a scanned cosigner's path.
    pub mono_subtitle: bool,
}

impl Item {
    /// An option that has not been chosen.
    pub fn new(id: Id, label: impl Into<String>) -> Self {
        Item {
            id: Some(id),
            label: label.into(),
            reason: None,
            chosen: false,
            subtitle: None,
            caution: None,
            icon: None,
            mono: false,
            mono_subtitle: false,
        }
    }

    /// The option the check is on: the row the user tapped, or the
    /// default a screen offers pre-checked (§4.2).
    pub fn chosen(id: Id, label: impl Into<String>, chosen: bool) -> Self {
        Item {
            id: Some(id),
            label: label.into(),
            reason: None,
            chosen,
            subtitle: None,
            caution: None,
            icon: None,
            mono: false,
            mono_subtitle: false,
        }
    }

    /// §4.4: a key chooser's row — the fingerprint glyph, the
    /// fingerprint, what the key is made of under it, and the check the
    /// rest of the list carries.
    pub fn key(
        id: Id,
        fingerprint: impl Into<String>,
        subtitle: impl Into<String>,
        chosen: bool,
    ) -> Self {
        Item {
            id: Some(id),
            label: fingerprint.into(),
            reason: None,
            chosen,
            subtitle: Some(subtitle.into()),
            caution: None,
            icon: Some(Icon::Fingerprint),
            mono: true,
            mono_subtitle: false,
        }
    }

    /// The same option with its label in the mono face: a key a chooser
    /// dims, still named by its fingerprint.
    pub fn mono(mut self) -> Self {
        self.mono = true;
        self
    }

    /// The same option with its second line in the mono face: the path
    /// under a scanned cosigner's fingerprint.
    pub fn mono_subtitle(mut self) -> Self {
        self.mono_subtitle = true;
        self
    }

    /// An option that cannot be taken, and the two or three words §4.11
    /// puts under its label.
    pub fn dimmed(label: impl Into<String>, reason: impl Into<String>) -> Self {
        Item {
            id: None,
            label: label.into(),
            reason: Some(reason.into()),
            chosen: false,
            subtitle: None,
            caution: None,
            icon: None,
            mono: false,
            mono_subtitle: false,
        }
    }

    /// §4.8: an option that can be taken but carries a caution about
    /// what taking it means, stated at the row's trailing edge.
    pub fn cautioned(
        id: Id,
        label: impl Into<String>,
        caution: impl Into<String>,
        chosen: bool,
    ) -> Self {
        Item {
            id: Some(id),
            label: label.into(),
            reason: None,
            chosen,
            subtitle: None,
            caution: Some(caution.into()),
            icon: None,
            mono: false,
            mono_subtitle: false,
        }
    }

    /// An option that cannot be taken with nothing to say about it: the
    /// feature is not built, and "not built yet" under a dimmed label
    /// tells a reader only what the dimming already said.
    pub fn dead(label: impl Into<String>) -> Self {
        Item {
            id: None,
            label: label.into(),
            reason: None,
            chosen: false,
            subtitle: None,
            caution: None,
            icon: None,
            mono: false,
            mono_subtitle: false,
        }
    }

    /// §4.6: an option whose value sits under its name — the path under
    /// "SegWit" in the path editor's purpose group.
    pub fn valued(id: Id, name: impl Into<String>, value: impl Into<String>, chosen: bool) -> Self {
        Item {
            id: Some(id),
            label: name.into(),
            reason: None,
            chosen,
            subtitle: Some(value.into()),
            caution: None,
            icon: None,
            mono: false,
            mono_subtitle: false,
        }
    }

    /// The row.
    fn node(self, class: SizeClass) -> Node {
        if let (Some(id), Some(caution)) = (self.id, self.caution) {
            return components::choice_caution_row(class, id, self.label, caution, self.chosen);
        }
        match (self.id, self.subtitle) {
            (Some(id), Some(subtitle)) => components::key_choice_row(
                class,
                id,
                self.icon,
                self.label,
                subtitle,
                self.chosen,
                (self.mono, self.mono_subtitle),
            ),
            _ if self.mono => {
                components::mono_choice_row(class, self.id, self.label, self.reason, self.chosen)
            }
            _ => components::choice_row(class, self.id, self.label, self.reason, self.chosen),
        }
    }
}

/// §5 Choice: "rows with a check, Continue."
///
/// §4.2: "A list of full-width rows, one option per row; the chosen row
/// carries an accent check; Continue at the bottom. Tap checks (the list
/// keeps its scroll position). Continue is dimmed until a row is
/// checked; a default is pre-checked where one exists."
///
/// §4.2 and §2.6: "The rows are one group centred in the space above
/// Continue; a list taller than the space scrolls from the top." The
/// question is the title, so the rows are the whole of the screen and
/// the Continue below them is pinned, in the same place whether the list
/// scrolls or not.
///
/// Made of [`components::choice_row`] (§4.2) in a
/// [`components::choice_list`], at the class's row height and gap, with
/// [`components::primary`] (§4.13) under them.
pub fn choice(c: &Chrome, title: &str, items: Vec<Item>, action: Action) -> Node {
    listed(c, title, None, items, action)
}

/// §5 Choice with §4.14's progress inside the centred block: "A bar with
/// a count '2 of 5' above it."
///
/// One question of a run is still a choice — the rows, the check and the
/// Continue are the same — so it is this screen and not another one; the
/// count and the bar say which question of how many, above the rows and
/// inside the one block §2.6 centres, so nothing about the list moves as
/// the run goes on. `progress` is the count and the share done.
pub fn choice_with_progress(
    c: &Chrome,
    title: &str,
    progress: (String, f32),
    items: Vec<Item>,
    action: Action,
) -> Node {
    listed(c, title, Some(progress), items, action)
}

/// The rows, the optional progress above them and the Continue under
/// them: the one body both forms of the screen are built from.
fn listed(
    c: &Chrome,
    title: &str,
    progress: Option<(String, f32)>,
    items: Vec<Item>,
    action: Action,
) -> Node {
    let class = c.class();
    let rows: Vec<Node> = items.into_iter().map(|i| i.node(class)).collect();
    let mut block = Node::column().gap(tokens::menu_row_gap(class));
    if let Some((count, fraction)) = progress {
        block = block.child(components::progress(fraction, count));
    }
    let block = block.child(components::choice_list(class, rows));
    c.scrolling(Frame::new(title, c.place(block, None)).footer(c.actions(alloc::vec![action])))
}

/// §4.2 Setting: "The same rows with the check on the current value; no
/// Continue. Tap applies at once and the screen stays. Used inside
/// Settings only."
///
/// A Setting is a Choose screen like any other (§5), so §2.6 places its
/// rows the same way: one group centred in the space under the app bar,
/// scrolling from the top when the list is taller than that space. The
/// only difference from [`choice`] is that there is nothing under them
/// to press.
pub fn settings(c: &Chrome, title: &str, items: Vec<Item>) -> Node {
    let class = c.class();
    let rows: Vec<Node> = items.into_iter().map(|i| i.node(class)).collect();
    let block = Node::column()
        .gap(tokens::menu_row_gap(class))
        .child(components::setting_list(class, rows));
    c.scrolling(Frame::new(title, c.place(block, None)))
}
