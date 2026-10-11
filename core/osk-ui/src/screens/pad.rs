//! §5 Pad: "dots or count field directly above the pad, progress and
//! masked entries for dice and coins, pad". Used by the PIN (lock, set,
//! repeat), the dice pad and the coin pad.

use alloc::string::String;
use alloc::vec::Vec;

use crate::components::{self, Entries};
use crate::geom::SizeClass;
use crate::layout::{Id, Node};
use crate::organisms;
use crate::text::{Font, TextAlign};
use crate::tokens;
use crate::widgets::Widget;
use crate::widgets::keyboard::{self, KeyboardKind};

use super::entry::WordsSoFar;
use super::{Action, Chrome, Frame};

/// Which pad the screen shows.
pub enum PadKind {
    /// §4.3 PIN pad: "3 × 4 pad, fixed size, centred, 280 dp wide at
    /// most." A shuffle changes digits, not layout.
    Pin {
        /// The scramble seed, when the shuffle setting is on.
        scramble: Option<u32>,
        /// §4.3: "✓ dimmed until the input is acceptable." False while
        /// the entry is too short to confirm.
        done: bool,
    },
    /// §4.3 Dice pad: six faces.
    Dice {
        /// §4.3: "✓ dimmed until the input is acceptable." False until
        /// the run reaches the target the wizard asked for, so the ✓ key
        /// and the Continue under it are never in opposite states.
        done: bool,
    },
    /// §4.3 Coin pad: two sides.
    Coin {
        /// As [`PadKind::Dice`].
        done: bool,
    },
    /// §4.3 Card pad: thirteen ranks and four suits.
    Cards {
        /// As [`PadKind::Dice`].
        done: bool,
    },
}

impl PadKind {
    /// The keyboard kind whose geometry this pad has.
    fn keyboard(&self) -> KeyboardKind {
        match self {
            PadKind::Pin { .. } => KeyboardKind::Pin,
            PadKind::Dice { .. } => KeyboardKind::Dice,
            PadKind::Coin { .. } => KeyboardKind::Coin,
            PadKind::Cards { .. } => KeyboardKind::Cards,
        }
    }

    /// The pad itself.
    fn widget(&self, id: Id) -> Widget {
        match self {
            PadKind::Pin { scramble, done } => Widget::Keyboard {
                id,
                kind: KeyboardKind::Pin,
                enabled: if *done {
                    keyboard::ALL_KEYS
                } else {
                    keyboard::ALL_KEYS | keyboard::DONE_DISABLED
                },
                scramble: *scramble,
            },
            PadKind::Dice { done } => Widget::pad(id, KeyboardKind::Dice, *done),
            PadKind::Coin { done } => Widget::pad(id, KeyboardKind::Coin, *done),
            PadKind::Cards { done } => Widget::pad(id, KeyboardKind::Cards, *done),
        }
    }
}

/// What the row directly above the pad states.
pub enum Field {
    /// One dot per digit typed, read as a count and not as a value
    /// (§4.3).
    Dots(usize),
    /// A count in the field's own row: "23 of 50".
    Count(String),
    /// Nothing above the pad but the progress.
    Nothing,
}

/// What a [`pad`] screen shows.
pub struct Pad<'a> {
    /// The app bar's title: "Enter your PIN", "Set a PIN", "Repeat it".
    pub title: String,
    /// The pad's application id.
    pub id: Id,
    /// Which pad.
    pub kind: PadKind,
    /// The field directly above the pad, at the pad's width.
    pub field: Field,
    /// §4.3: "A progress line '23 of 50 · 59 bits' and a bar."
    pub progress: Option<(String, f32)>,
    /// §4.3: "the entries so far in groups of five, masked as a secret."
    pub entries: Option<Entries<'a>>,
    /// §4.3: the words a run of rolls has completed so far, in a masked
    /// panel in the space above the pad group. `None` on a pad that
    /// builds no words, which is every pad but the diceware one.
    pub words: Option<WordsSoFar<'a>>,
    /// §4.3: the app bar's eye, on a pad that draws a words panel, with
    /// the share of its 30 s reveal still to run (§4.10).
    pub eye: Option<(Id, Option<f32>)>,
    /// What the caption line says when nothing is wrong: which deck
    /// the card pad's draws come from. `None` leaves the line empty.
    pub caption: Option<String>,
    /// §4.3 Inline error: the reserved caption line under the field.
    /// "PINs differed", "Wrong PIN · 4 left".
    pub error: Option<String>,
    /// §4.3: "Continue when the target is reached." A PIN pad has none:
    /// its ✓ is a key.
    pub action: Option<Action>,
}

/// §5 Pad: "dots or count field directly above the pad, progress and
/// masked entries for dice and coins, pad."
///
/// §4.3 and §2.6: the pad, its field and the run above them are the
/// bottom group on every class, so the PIN, dice and coin pads share one
/// rectangle wherever a flow shows one. The field and the pad are one
/// group
/// inside the pad's own width cap, so the two read as one control and
/// [`super::PAD_SLOT`] keeps one rectangle whatever the title says or
/// how much has been typed (§2.5). A dice or coin run puts its masked
/// entries, its progress line and its bar inside that same group and at
/// that same width, directly above the pad, with Continue under it.
///
/// Made of [`components::pad_group`] (§4.3) for the field and the pad,
/// [`components::inline_error`] (§4.3) for the reserved caption line
/// under the field, and [`components::entropy_progress`] (§4.3, §4.14)
/// for the line, the bar and the entries.
pub fn pad(c: &Chrome, p: Pad<'_>) -> Node {
    let class = c.class();
    let kind = p.kind.keyboard();
    // §2.6: what the app bar, the pad group and the action row leave is
    // the space the words panel has to fit, and the shape of the group
    // is known before it is built.
    let panel_space = space_above(
        c,
        kind,
        !matches!(p.field, Field::Nothing),
        p.progress.is_some(),
        p.action.is_some(),
    );

    let field = match p.field {
        Field::Dots(n) => {
            let dots: String = core::iter::repeat_n('\u{2022}', n).collect();
            Some(
                organisms::field_at(
                    tokens::FIELD,
                    dots,
                    Font::mono(tokens::TITLE),
                    TextAlign::Center,
                )
                .id(super::FIELD),
            )
        }
        // §16.126: a count of rolls or cards is digits read as digits.
        Field::Count(text) => Some(
            organisms::field_at(
                tokens::FIELD,
                text,
                Font::mono(tokens::TITLE),
                TextAlign::Center,
            )
            .id(super::FIELD),
        ),
        Field::Nothing => None,
    };
    // §4.3: the caption line is reserved whether or not it says
    // anything, so the pad keeps one rectangle across the lock screen
    // and both set-PIN steps whatever the last entry did. A pad with no
    // field reserves it all the same: the dice pad's line is empty, and
    // the card pad's says which deck is being drawn from until the
    // tracker refuses a card.
    let caption = match (p.error, p.caption) {
        (Some(error), _) => components::inline_error(class, Some(error)),
        (None, caption) => components::inline_caption(class, caption),
    }
    .id(super::ERROR);
    let field = Some(match field {
        Some(f) => Node::column()
            .gap(tokens::GAP_SMALL)
            .child(f)
            .child(caption),
        None => caption,
    });

    // The pad's own rectangle is the one the screen reserves: as tall as
    // its rows, and as wide as the group it shares with the field.
    let slot = Node::column()
        .id(super::PAD_SLOT)
        .height(slot_height(kind, class, c.inset_bottom()))
        .child(Node::widget(p.kind.widget(p.id)));
    // §4.3: the run so far goes inside the pad group, at the pad's
    // width, directly above the pad — "the eye moves nowhere between the
    // pad and what it typed".
    // The run is drawn at the pad's own width, which is what the group
    // it sits in has: a run wider than that would be cut mid-group.
    let run_width = c.pane().width_dp.min(tokens::PAD_MAX_WIDTH) - 2.0 * tokens::PAD;
    let above = p.progress.map(|(count, fraction)| {
        components::entropy_progress(class, run_width, count, fraction, p.entries.as_ref())
            .id(super::ENTRIES)
            .height(components::entropy_progress_height(class))
    });
    let group = Node::column()
        .gap(tokens::GAP)
        .child(components::entry::pad_group(
            class,
            c.inset_bottom(),
            kind,
            above,
            field,
            slot,
        ));

    // §4.3: the panel of words the run has completed sits in the space
    // §2.6 centres, above the pad group, on a class that has the room for
    // it — the same block a word entry draws it in.
    let mut block = Node::column();
    let panel_room = components::secrets::panel_room(c.column().width_dp);
    if let Some(w) = p.words.filter(|w| {
        class != SizeClass::Small
            && components::secrets::panel_fits(class, panel_room, panel_space, w.width, w.total)
    }) {
        block = block.child(components::words_so_far(
            class, w.panel, w.words, w.total, w.revealed, w.width, panel_room,
        ));
    }

    let footer: Vec<Node> = p
        .action
        .map(|a| c.actions(alloc::vec![a]))
        .unwrap_or_default();
    let trailing = p
        .eye
        .map(|(id, remaining)| components::bar_eye(id, remaining));
    c.fixed(
        Frame::new(&p.title, c.place(block, Some(group)))
            .trailing(trailing)
            .footer(footer)
            .dimmed(),
    )
}

/// Height in dp of the pad's rectangle: its rows at the key height the
/// class gives this pad, and the reserve that keeps the bottom row's
/// face clear of the edge. Fixed, so nothing the screen says resizes it
/// (§2.5).
/// Height in dp of the space §2.6 centres above the pad group: what the
/// app bar, the group and the screen's action row leave of the pane.
/// A view asks through [`pad_words_panel_fits`]; the screen asks here.
fn space_above(c: &Chrome, kind: KeyboardKind, field: bool, progress: bool, action: bool) -> f32 {
    let class = c.class();
    let m = c.pane();
    let mut group = slot_height(kind, class, c.inset_bottom()) + tokens::GAP;
    group += tokens::caption_line(class);
    if field {
        group += tokens::FIELD + tokens::GAP_SMALL;
    }
    if progress {
        group += components::entropy_progress_height(class) + tokens::GAP;
    }
    let footer = if action {
        tokens::CTA + tokens::action_bottom(c.inset_bottom())
    } else {
        0.0
    };
    m.height_dp - m.inset_top_dp - tokens::APP_BAR - c.body_gap() - group - footer
}

/// Whether a pad screen on this class has the room above its group for
/// a panel of `total` words of this list (§4.10). A view asks before it
/// offers the app bar's eye, which belongs to the panel.
pub fn pad_words_panel_fits(
    c: &Chrome,
    kind: PadKind,
    field: bool,
    progress: bool,
    action: bool,
    width: components::secrets::WordWidth,
    total: usize,
) -> bool {
    // §4.3: a 240 dp panel has no room above its pad for a words panel
    // on any list, and no eye either — what a run has completed is read
    // on the screen the run ends on.
    if c.class() == SizeClass::Small {
        return false;
    }
    let panel_room = components::secrets::panel_room(c.column().width_dp);
    let space = space_above(c, kind.keyboard(), field, progress, action);
    components::secrets::panel_fits(c.class(), panel_room, space, width, total)
}

fn slot_height(kind: KeyboardKind, class: SizeClass, inset_bottom_dp: f32) -> f32 {
    keyboard::rows(kind) as f32 * keyboard::max_key_height(kind, class, tokens::PAD_MAX_WIDTH)
        + tokens::key_bottom_reserve(inset_bottom_dp)
}
