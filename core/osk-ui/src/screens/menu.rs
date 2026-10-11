//! §5 Menu: "rows". Used by Keys, the key menu, the Backup menu, Sign
//! entry, Settings and Explore.

use alloc::string::String;
use alloc::vec::Vec;

use crate::components;
use crate::geom::SizeClass;
use crate::layout::{Id, Node};
use crate::tokens;
use crate::widgets::{Icon, Tone};

use super::{Action, Chrome, Frame, KeyContext};

/// One row of a [`menu`]. Every kind of row a menu mixes is here, so a
/// screen never picks a row height or a mask policy of its own.
pub enum Row {
    /// §4.1 Menu row: icon, label, optional value, chevron.
    Menu {
        /// Application id.
        id: Id,
        /// The area's mark, where the row has one. §4.1 makes it
        /// optional, and a label too long for a 268 dp panel beside one
        /// is a label the row shows whole without it.
        icon: Option<Icon>,
        /// What the row opens.
        label: String,
        /// The current state, where the row states one.
        value: Option<String>,
        /// The value's colour role: caution for "not verified" (§4.8).
        tone: Tone,
    },
    /// §4.1 Menu row whose value is read character by character rather
    /// than as words, and so is set in the mono face (§16.126): a
    /// wallet row, whose value is the fingerprint and script the wallet
    /// is made of.
    MonoMenu {
        /// Application id.
        id: Id,
        /// The area's mark, where the row has one.
        icon: Option<Icon>,
        /// What the row opens.
        label: String,
        /// The characters the row states.
        value: String,
    },
    /// §4.2 Mode on a screen: a label, its current value and a chevron.
    Value {
        /// Application id.
        id: Id,
        /// What the value is.
        label: String,
        /// The value.
        value: String,
        /// The value's colour role.
        tone: Tone,
    },
    /// A row whose value is a fingerprint: a §4.2 value row that opens a
    /// Choice where it has an id, a fact row where it has none, the
    /// fingerprint glyph before the value (`docs/PLANNING.md` §16.131).
    Fingerprint {
        /// Application id, where the row opens a Choice.
        id: Option<Id>,
        /// What the value is: "Key".
        label: String,
        /// The fingerprint, or the placeholder while there is none.
        value: String,
    },
    /// §4.6 Path row: "label 'Path' above, the string below, chevron
    /// when it opens the editor". The value is mono, because a path is
    /// read character by character.
    Path {
        /// Application id.
        id: Id,
        /// What the path is.
        label: String,
        /// The path itself, `m/84h/0h/0h`.
        value: String,
    },
    /// §4.2 value row whose value is read character by character and is
    /// short enough to show whole — a descriptor's summary, "wpkh ·
    /// 73c5da0a · #qf45pmyh" — so it is set in the mono face and not
    /// elided.
    MonoValue {
        /// Application id.
        id: Id,
        /// What the value is.
        label: String,
        /// The value, whole.
        value: String,
    },
    /// §4.4 Key row: a fingerprint and its subtitle.
    Key {
        /// Application id.
        id: Id,
        /// The key's name.
        fingerprint: String,
        /// "mainnet · passphrase".
        subtitle: String,
    },
    /// §4.5 Reference row: a label and the elided value, opening the
    /// Compare screen.
    Reference {
        /// Application id.
        id: Id,
        /// What the string is.
        label: String,
        /// The whole string; the row shows its head and tail.
        value: String,
    },
    /// §4.10 Long secret: a row that opens the Secret screen.
    Secret {
        /// Application id.
        id: Id,
        /// What the secret is.
        label: String,
        /// The value when it is already shown elsewhere; `None` masks
        /// it.
        value: Option<String>,
    },
    /// §4.12 Value: a fact the menu states and no row opens — the
    /// derivation path a wallet export is taken at. Label above, value
    /// below, no chevron and no hit target.
    Fact {
        /// What the value is.
        label: String,
        /// The value.
        value: String,
        /// The monospace face, for a value made of characters: a path.
        mono: bool,
        /// The value's colour role: danger for a weak session key (§4.8).
        tone: Tone,
    },
    /// §4.2 Toggle: a boolean setting.
    Toggle {
        /// Application id.
        id: Id,
        /// What is switched.
        label: String,
        /// The current state.
        on: bool,
        /// Why it cannot be switched, where it cannot: §4.11's two or
        /// three words under the label, and the row is dead. `None` on
        /// a live toggle.
        reason: Option<String>,
    },
    /// §4.11 Record row inside a Menu: a fact drawn flat, with no
    /// surface under it. A menu's rows all open something (§4.1 "Tap
    /// opens"), so a fact among them is drawn as the table row it is
    /// rather than as a row that leads nowhere.
    Flat {
        /// What the value is.
        label: String,
        /// The value.
        value: String,
        /// Whether the value is read character by character.
        mono: bool,
    },
    /// §4.13 Row action: a way out of a result.
    Action {
        /// Application id.
        id: Id,
        /// The action's mark.
        icon: Icon,
        /// The verb.
        label: String,
    },
    /// §4.11 Dimmed control: a dead row, with the two or three words
    /// under its label that say why.
    Dimmed {
        /// The row's mark, where it has one. A row that states a value
        /// carries none, dimmed or not.
        icon: Option<Icon>,
        /// What the row would open.
        label: String,
        /// Why it cannot: "no camera", "needs Tier B". `None` where the
        /// answer is only that the feature is not built, which is
        /// nothing a row can usefully say.
        reason: Option<String>,
    },
}

impl Row {
    /// The row at the class's height.
    pub(super) fn node(self, class: SizeClass) -> Node {
        match self {
            Row::Menu {
                id,
                icon,
                label,
                value,
                tone,
            } => components::menu_row(class, id, icon, label, value, tone),
            Row::MonoMenu {
                id,
                icon,
                label,
                value,
            } => components::menu_row_mono(class, id, icon, label, value),
            Row::Value {
                id,
                label,
                value,
                tone,
            } => components::value_row(class, id, label, value, tone),
            Row::Fingerprint { id, label, value } => {
                components::fingerprint_row(class, id, label, value)
            }
            Row::Path { id, label, value } => components::path_row(class, id, label, &value),
            Row::MonoValue { id, label, value } => {
                components::mono_value_row(class, id, label, value, Tone::Text)
            }
            Row::Key {
                id,
                fingerprint,
                subtitle,
            } => components::key_row(class, id, fingerprint, subtitle),
            Row::Reference { id, label, value } => {
                components::reference_row(class, id, label, &value)
            }
            Row::Secret { id, label, value } => {
                components::secret_row(class, id, label, value.as_deref())
            }
            Row::Fact {
                label,
                value,
                mono,
                tone,
            } => components::fact_row(class, label, value, mono, tone),
            Row::Flat { label, value, mono } => {
                components::flat_fact_row(class, label, value, mono)
            }
            Row::Toggle {
                id,
                label,
                on,
                reason,
            } => components::toggle_row(class, id, label, on, reason),
            Row::Action { id, icon, label } => components::row_action(class, id, icon, label),
            Row::Dimmed {
                icon,
                label,
                reason,
            } => components::dimmed_row(class, icon, label, reason),
        }
    }
}

/// §5 Menu: "rows."
///
/// §4.1: "Full-width rows: icon, label, optional value, chevron. Tap
/// opens." One item per row on every class (§2.4), at the class's row
/// height and gap. The screen scrolls only when it is longer than the
/// panel; a row is its own way on, so most menus have no bottom action.
///
/// `actions` is for the menu that also starts something the list cannot
/// hold: Keys carries "Create a key" and "Load a key" under its rows,
/// which is §4.14's empty state ("A row that starts the flow") kept in
/// one place whether the list is empty or full. The row is §4.13's, so
/// the two share the width on a touch class and sit bottom-right on
/// `wide`.
///
/// Made of [`components::menu_row`], [`components::value_row`],
/// [`components::path_row`], [`components::key_row`],
/// [`components::reference_row`], [`components::secret_row`],
/// [`components::toggle_row`], [`components::row_action`] (§4.1, §4.2,
/// §4.4, §4.5, §4.6, §4.10, §4.13) and [`components::key_context`]
/// (§4.4) at the top of the body.
pub fn menu(
    c: &Chrome,
    title: &str,
    key: Option<KeyContext>,
    rows: Vec<Row>,
    actions: Vec<Action>,
) -> Node {
    menu_framed(c, Frame::new(title, Node::column()), key, rows, actions)
}

/// The same menu titled by a fingerprint, which the app bar sets in the
/// mono face because it is read character by character (§16.126). The
/// key's own page is the one screen that asks for it.
pub fn menu_mono_title(
    c: &Chrome,
    title: &str,
    key: Option<KeyContext>,
    rows: Vec<Row>,
    actions: Vec<Action>,
) -> Node {
    menu_framed(
        c,
        Frame::new(title, Node::column()).mono_title(),
        key,
        rows,
        actions,
    )
}

fn menu_framed(
    c: &Chrome,
    frame: Frame<'_>,
    key: Option<KeyContext>,
    rows: Vec<Row>,
    actions: Vec<Action>,
) -> Node {
    let class = c.class();
    let mut body = Node::column().gap(tokens::menu_row_gap(class));
    if let Some(k) = key {
        body = body.child(k.node(class, false));
    }
    body = body.children(rows.into_iter().map(|r| r.node(class)));
    c.scrolling(frame.body(body).footer(c.actions(actions)))
}
