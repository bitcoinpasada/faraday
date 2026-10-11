//! §5 Entry: "field, candidates or presets, keyboard". Used by words, a
//! passphrase, a path, an address and hex.

use alloc::string::String;
use alloc::vec::Vec;

use crate::components;
use crate::geom::{Edges, SizeClass};
use crate::layout::{Align, Id, Justify, Node};
use crate::organisms;
use crate::tokens;
use crate::widgets::keyboard::{self, KeyMask, KeyboardKind};
use crate::widgets::{Tone, Widget};

use super::Chrome;
use super::choice::Item;

/// §4.3 Candidates: "One row of up to 3 (`small`) or two rows of 4
/// (`mobile`) between the field and the keys." The strip is part of the
/// entry group, not of the block above it, so a screen can carry both.
pub struct Candidates {
    /// The first row's strip.
    pub first: Id,
    /// The second row's strip, on the classes that have one.
    pub second: Id,
    /// The words that still match.
    pub words: Vec<String>,
    /// One candidate left, which the strip outlines.
    pub accepting: bool,
    /// More candidates remain than the strip holds, so its last cell
    /// pages (§4.3).
    pub more: bool,
    /// The list's words are one character each — a Chinese list, looked
    /// up by a Mandarin reading — so the strip is ten cells a row and two
    /// rows on every class, at the keyboard's key pitch. It is the list
    /// that says so and not the candidates left, so the strip keeps its
    /// rectangle as the reading is typed.
    pub one_char: bool,
    /// The cell a tap has selected, counting across both rows: on the
    /// small panel a candidate is selected by one tap and accepted by
    /// the next (`docs/PLANNING.md` §16.50).
    pub selected: Option<usize>,
}

/// What sits in the space above the field.
pub enum Above {
    /// Nothing: the space is empty.
    Nothing,
    /// §4.6 Path editor: "the presets as two labelled groups, each with
    /// its own check, because they are two independent choices."
    Presets {
        /// "Purpose": the four script types, the path as each row's
        /// value, one check (§4.6).
        purpose: (String, Vec<Item>),
        /// "Chain": the Receive | Change pair (§4.2).
        chain: (String, (Id, String), (Id, String), bool),
    },
    /// §4.3: "A fact row above the entry group where what is typed
    /// derives one value the person will recognise: the fingerprint of a
    /// passphrase key or a child seed." The row is one
    /// [`components::Record`] in the space the block reserves, so the
    /// value changes without anything moving. One row is what the space
    /// holds: a 268 dp panel leaves the field and the keyboard about a
    /// row's height above them and no more.
    Fact(FactRow),
    /// The same reserved space holding more than one fact, for a screen
    /// whose answer is several values at once: the Units calculator,
    /// where every unit but the one being typed is a row above the
    /// field and there is nothing to press ✓ for.
    Facts(Vec<FactRow>),
    /// §4.3: "An address being typed (Verify) is the one value whose tail
    /// is not enough: the space above the entry group holds the typed
    /// address whole, as a comparison string at the largest size that
    /// fits, on every class."
    Whole {
        /// The string's application id, for the chunk planner.
        id: Id,
        /// What has been typed so far; empty before the first character.
        text: String,
    },
}

/// §4.3 on `mobile` and `wide`: the words accepted so far, in a masked
/// panel above the entry group.
pub struct WordsSoFar<'a> {
    /// The panel's application id, which is also its touch surface.
    pub panel: Id,
    /// The words accepted so far, in order.
    pub words: &'a [String],
    /// How many words the secret has: twelve or twenty-four. The panel
    /// keeps a row for every one of them, so it does not grow as the
    /// entry goes on.
    pub total: usize,
    /// Whether the words are shown: a finger on the panel, or the app
    /// bar's eye within its 30 s (§4.10).
    pub revealed: bool,
    /// How wide a word of this wordlist is.
    pub width: crate::components::secrets::WordWidth,
}

/// One row of [`Above::Facts`].
pub struct FactRow {
    /// The row's application id where the row is a §4.2 mode that opens
    /// a Choice — "Search by · Number" — and `None` where the row is a
    /// fact and nothing more.
    pub id: Option<Id>,
    /// What the value is.
    pub label: String,
    /// The value, or the app's placeholder while there is none.
    pub value: String,
    /// The monospace face, for a fingerprint.
    pub mono: bool,
    /// The fingerprint glyph before the value, where the value is a
    /// fingerprint (`docs/PLANNING.md` §16.131).
    pub glyph: bool,
}

impl FactRow {
    /// A fact: a label, a value, nothing to tap.
    pub fn new(label: impl Into<String>, value: impl Into<String>, mono: bool) -> Self {
        FactRow {
            id: None,
            label: label.into(),
            value: value.into(),
            mono,
            glyph: false,
        }
    }

    /// A fact whose value is a fingerprint, or the placeholder while
    /// there is none: the monospace face, and the fingerprint glyph
    /// before a fingerprint (`docs/PLANNING.md` §16.131).
    pub fn fingerprint(label: impl Into<String>, value: impl Into<String>) -> Self {
        FactRow {
            glyph: true,
            ..FactRow::new(label, value, true)
        }
    }

    /// §4.2 Mode: the row opens a Choice over this screen.
    pub fn mode(id: Id, label: impl Into<String>, value: impl Into<String>) -> Self {
        FactRow {
            id: Some(id),
            label: label.into(),
            value: value.into(),
            mono: false,
            glyph: false,
        }
    }
}

/// What an [`entry`] screen types into.
pub struct Entry<'a> {
    /// The app bar's title, which carries the progress where there is
    /// any: "Word 7 of 12" (§4.1).
    pub title: &'a str,
    /// The value being typed.
    pub value: String,
    /// The monospace face, for a value made of characters rather than
    /// words: a path, hex, an address.
    pub mono: bool,
    /// What the space above the field holds.
    pub above: Above,
    /// §4.3's candidate strip, between the field and the keys.
    pub candidates: Option<Candidates>,
    /// The words accepted so far, for word entry. `None` on a screen
    /// that types something else, and left off the 268 dp panel, which
    /// shows the word being typed and nothing else (§4.3).
    pub words: Option<WordsSoFar<'a>>,
    /// §4.3: the app bar's eye, on a screen that draws a words panel,
    /// with the share of its 30 s reveal still to run (§4.10). `None`
    /// where there is no panel to show.
    pub eye: Option<(Id, Option<f32>)>,
    /// The keyboard's id and kind.
    pub keyboard: (Id, KeyboardKind),
    /// BIP-39: the letters that can still lead to a word. Ignored by the
    /// other kinds.
    pub enabled: KeyMask,
    /// The reserved caption line under the field (§4.3).
    pub error: Option<String>,
}

/// §5 Entry: "field, candidates or presets, keyboard."
///
/// §4.3: "Bottom-anchored, full-bleed, one group with the field." The
/// field, its candidate strip or its reserved error line, and the
/// keyboard are the bottom group of §2.6, and the keyboard reaches both
/// edges under it. There is no bottom action: ✓ on the keyboard is the
/// way on, and it is dead until the input is acceptable.
///
/// §4.3 gives the space above the group its own content, and there are
/// four things it can be:
///
/// - the masked words panel of the words accepted so far, on `mobile` and
///   `wide`, "drawn only once a word has been accepted (an empty panel
///   says nothing), at the fixed geometry of the word count, and while it
///   is drawn the app bar carries the eye";
/// - the path editor's two labelled groups (§4.6), which scroll from the
///   top on a class whose height they outgrow;
/// - the typed address, whole, as a comparison string (§4.3), which is
///   the one value whose tail is not enough;
/// - one fact row (§4.3), where what is typed derives a value the person
///   will recognise — the fingerprint of the key a passphrase or a child
///   index opens — or where a §4.2 mode row says what the field means.
///
/// On `small` word entry shows the word being typed and nothing else.
///
/// §4.4: "An Entry screen never carries the key row: typing words or a
/// passphrase is not about a key yet." There is no key context here and
/// no way to pass one.
///
/// §4.3 also reserves the caption line: "One caption line under the
/// field, always reserved at the caption's line box for the class.
/// Appears in the danger tone; nothing moves." It is drawn under the
/// candidates, so a word that matches nothing says so where every other
/// entry screen says it, and the keyboard stays where it was.
///
/// Made of [`components::text_field`], [`components::candidate_strip`]
/// and [`components::inline_error`] (§4.3),
/// [`components::choice_value_row`] and [`components::pair`] (§4.2) for
/// the presets, [`components::comparison_string`] (§4.5) for the typed
/// address, [`components::record`] (§4.11) for the fact row and
/// [`components::words_so_far`] (§4.10) for the panel.
pub fn entry(c: &Chrome, e: Entry<'_>) -> Node {
    let class = c.class();
    let (id, kind) = e.keyboard;
    // §4.3's typed address and its fact row are the blocks that are the
    // space: each keeps its rectangle whether or not anything has been
    // typed.
    // Several facts are the space rather than a value floating in it:
    // the table takes the height it needs, so the block is not the
    // reserved rectangle a single value gets.
    let reserved = matches!(e.above, Above::Whole { .. } | Above::Fact(_));
    // §4.3 gives the path editor's two groups the same priority it gives
    // the typed address: on `wide`, where the block is 7 dp inside the
    // space it is given, the keyboard keeps to its own least height so
    // §2.6's block fits without scrolling. `small` scrolls the groups
    // (§4.6) and `mobile` has the room for both, so both keep the key
    // height their class names.
    let dense_keys = matches!(e.above, Above::Presets { .. }) && c.class() == SizeClass::Wide;
    // §4.3: an Entry whose reserved block does not fit above the field
    // at the class's key height drops its keyboard to the floor, and a
    // fact row that still does not fit is drawn on one line rather than
    // over the field.
    let (floor_keys, max_keys) = key_bounds(c, kind);
    let need = reserved_height(c, &e.above);
    let tight_keys = reserved && room_above(c, &e, max_keys) < need;
    let one_line = matches!(e.above, Above::Fact(_))
        && room_above(c, &e, if tight_keys { floor_keys } else { max_keys }) < need;
    let mut block = Node::column().gap(tokens::GAP);
    match &e.above {
        Above::Presets { purpose, chain } => {
            block = block.child(presets(c, purpose, chain));
        }
        Above::Whole { id, text } => {
            // §4.3: the block keeps its rectangle whether or not anything
            // has been typed, so the field under it does not move when the
            // first character arrives. The string measures nothing and is
            // stretched to the block, which is what reserves the space's
            // height; §2.6 then centres it in that space, and inside it
            // the string takes as much of the mono ramp as fits (§5
            // Transcribe).
            block = block.child(
                Node::stack()
                    .child(
                        Node::column()
                            .justify(Justify::Center)
                            .child(Node::column().id(super::ABOVE).child(
                                components::comparison_string(class, *id, text.clone(), true),
                            ))
                            .align_self(Align::Stretch)
                            .max_height(0.0)
                            .max_width(0.0),
                    )
                    .weight(1.0)
                    .min_height(0.0),
            );
        }
        Above::Fact(_) | Above::Facts(_) => {
            let facts: Vec<&FactRow> = match &e.above {
                Above::Fact(f) => alloc::vec![f],
                Above::Facts(f) => f.iter().collect(),
                _ => Vec::new(),
            };
            // The same reserved space the typed address takes: the row
            // measures nothing and is centred in what is left, so the
            // field and the keys stay where they are as the value
            // changes from one keystroke to the next.
            let rows: Vec<components::Record> =
                facts.into_iter().map(|f| fact(f, one_line)).collect();
            let table = components::record(&c.column(), rows);
            block = block.child(if matches!(e.above, Above::Facts(_)) {
                // §2.6: "A block that does not fit pages or scrolls."
                // Several facts outgrow the space a 240 dp panel leaves
                // above a field and a keyboard, so the table is the
                // part that scrolls, as the path editor's groups are.
                Node::column()
                    .id(super::ABOVE)
                    .child(Node::scroll(super::SCROLL, Node::column().child(table)))
            } else {
                Node::stack()
                    .child(
                        Node::column()
                            .justify(Justify::Center)
                            .child(table.id(super::ABOVE))
                            .align_self(Align::Stretch)
                            .max_height(0.0)
                            .max_width(0.0),
                    )
                    .weight(1.0)
                    .min_height(0.0)
            });
        }
        Above::Nothing => {}
    }
    // §4.3, §4.10: the panel of words accepted so far is drawn where
    // the space above the group has the height for it — two columns
    // where the width allows them, one where it does not — and is left
    // off where even one column is taller than that space, which is what
    // the 240 dp panels hit.
    let panel_space = room_above(c, &e, max_keys);
    let panel_room = components::secrets::panel_room(c.column().width_dp);
    if let Some(w) = e.words.filter(|w| {
        class != SizeClass::Small
            && components::secrets::panel_fits(class, panel_room, panel_space, w.width, w.total)
    }) {
        block = block.child(components::words_so_far(
            class, w.panel, w.words, w.total, w.revealed, w.width, panel_room,
        ));
    }

    // §4.3's group is padded like every other body, save for a
    // one-character candidate strip, which is full-bleed like the keys —
    // so the padding is on the children rather than on the group.
    let mut group = Node::column().gap(tokens::GAP_SMALL).child(padded(
        components::text_field(class, e.value, e.mono).id(super::FIELD),
    ));
    if let Some(Candidates {
        first,
        second,
        words,
        accepting,
        more,
        one_char,
        selected,
    }) = e.candidates
    {
        // A one-character strip is the keyboard's ten-key row, so it
        // reaches both edges as the keys do and its cell is exactly a key
        // wide, whatever is left in it.
        let bleed = one_char;
        let strip = components::candidate_strip(components::Strip {
            class,
            first,
            second,
            words: &words,
            accepting,
            more,
            one_char,
            selected,
        });
        group = group.child(if bleed { strip } else { padded(strip) });
    }
    // §4.3: the caption line under the field is there whether or not it
    // says anything, so the strip, the keys and the field never move.
    group = group.child(padded(
        components::inline_error(class, e.error).id(super::ERROR),
    ));

    let trailing = e
        .eye
        .map(|(id, remaining)| components::bar_eye(id, remaining));
    let pane = Node::column()
        .justify(Justify::End)
        .align(Align::Stretch)
        .child(organisms::app_bar(c.back, e.title, c.trailing(trailing)))
        .child(
            c.place(body_block(c, block, reserved), Some(group))
                .weight(tokens::BODY_SHRINK_WEIGHT)
                .min_height(0.0),
        )
        .child(keys(c, id, kind, e.enabled, tight_keys || dense_keys));
    c.framed(true, pane)
}

/// One fact row of the block above the field, on two lines or, where
/// the space holds one line and not two, on one (§4.3).
fn fact(f: &FactRow, one_line: bool) -> components::Record {
    if one_line {
        return components::Record::OneLine {
            id: f.id,
            label: f.label.clone(),
            value: f.value.clone(),
            mono: f.mono,
            glyph: f.glyph,
        };
    }
    if f.glyph && f.id.is_none() {
        return components::Record::fingerprint(f.label.clone(), f.value.clone());
    }
    match (f.id, f.mono) {
        (Some(id), _) => components::Record::value(id, f.label.clone(), f.value.clone()),
        (None, true) => components::Record::mono(f.label.clone(), f.value.clone()),
        (None, false) => components::Record::text(f.label.clone(), f.value.clone(), Tone::Text),
    }
}

/// The least and greatest height in dp this screen's keyboard node takes
/// on this class.
fn key_bounds(c: &Chrome, kind: KeyboardKind) -> (f32, f32) {
    let class = c.class();
    let m = c.pane();
    keyboard::node_heights(
        kind,
        class,
        c.inset_bottom(),
        m.width_dp,
        m.height_dp - tokens::APP_BAR,
        keyboard::min_key_height(kind, class),
    )
}

/// Height in dp the block above the field asks for: the fact row on its
/// two lines, or the least a typed address is drawn in.
fn reserved_height(c: &Chrome, above: &Above) -> f32 {
    match above {
        Above::Fact(f) => components::record_height(c.class(), &[fact(f, false)]),
        Above::Whole { .. } => tokens::TYPED_VALUE_BLOCK,
        _ => 0.0,
    }
}

/// Height in dp left for the block above the field when the keyboard
/// node is `keys_dp` tall: what the app bar, the entry group and the
/// keys leave of the pane.
fn room_above(c: &Chrome, e: &Entry<'_>, keys_dp: f32) -> f32 {
    let m = c.pane();
    m.height_dp - m.inset_top_dp - tokens::APP_BAR - c.body_gap() - group_height(c, e) - keys_dp
}

/// Height in dp the entry group takes: the field, the candidate strip
/// where the screen has one, and the reserved caption line (§4.3).
fn group_height(c: &Chrome, e: &Entry<'_>) -> f32 {
    group_height_of(c.class(), e.candidates.as_ref().map(|k| k.one_char))
}

/// The same, from the shape of the strip alone.
fn group_height_of(class: SizeClass, candidates: Option<bool>) -> f32 {
    let mut h = tokens::field(class) + tokens::GAP_SMALL + tokens::caption_line(class);
    if let Some(one_char) = candidates {
        let rows = tokens::candidate_rows(class, one_char) as f32;
        h += tokens::GAP_SMALL
            + rows * tokens::candidate_cell(class, one_char)
            + (rows - 1.0) * tokens::CANDIDATE_GAP;
    }
    h
}

/// One child of the entry group, inside the screen's own padding.
fn padded(node: Node) -> Node {
    Node::column()
        .padding(Edges::symmetric(tokens::PAD, 0.0))
        .child(node)
}

/// The block above the entry group, capped on `wide` and padded like
/// every other body. `reserved` makes it the space itself — it measures
/// nothing and takes what is left — which is what keeps §4.3's typed
/// address from moving the field under it.
fn body_block(c: &Chrome, block: Node, reserved: bool) -> Node {
    let block = c.capped(block).padding(Edges::symmetric(tokens::PAD, 0.0));
    if reserved {
        block.weight(1.0).min_height(tokens::TYPED_VALUE_BLOCK)
    } else {
        block
    }
}

/// §4.6 Path editor: "the presets as two labelled groups, each with its
/// own check, because they are two independent choices. 'Purpose': four
/// choice rows named in §4.6's vocabulary (Legacy, Nested, SegWit,
/// Taproot) with the path as the value under the name; 'Chain': the
/// Receive | Change pair."
///
/// Each group's label is §4.12's label above its rows. §2.6: "A block
/// that does not fit pages or scrolls" — two labels, four two-line rows
/// and a pair do not share a 268 dp panel with a field and a keyboard, so
/// the groups are the part that scrolls, and the checked row is scrolled
/// into view when the editor opens. On the taller classes the region is
/// exactly its content and nothing moves.
fn presets(
    c: &Chrome,
    purpose: &(String, Vec<Item>),
    chain: &(String, (Id, String), (Id, String), bool),
) -> Node {
    let class = c.class();
    let (purpose_label, items) = purpose;
    let (chain_label, left, right, change) = chain;
    let rows: Vec<Node> = items
        .iter()
        .map(|item| match item.id {
            Some(id) => components::preset_row(
                class,
                id,
                item.label.clone(),
                item.subtitle.clone().unwrap_or_default(),
                item.chosen,
            ),
            None => components::choice_row(
                class,
                None,
                item.label.clone(),
                item.reason.clone(),
                item.chosen,
            ),
        })
        .collect();
    // §2.6: the block fits the space above the field on `wide`, so the
    // rows keep the dense height and gap [`tokens::preset_row`] measures
    // rather than the full menu row a Choice screen gives its options.
    let groups = Node::column()
        .gap(tokens::preset_row_gap(class))
        .child(
            Node::column()
                .gap(tokens::preset_row_gap(class))
                .child(components::label(purpose_label.clone()))
                .child(
                    Node::column()
                        .gap(tokens::preset_row_gap(class))
                        .children(rows),
                ),
        )
        .child(
            Node::column()
                .gap(tokens::preset_row_gap(class))
                .child(components::label(chain_label.clone()))
                .child(components::pair(
                    (left.0, left.1.clone()),
                    (right.0, right.1.clone()),
                    *change,
                )),
        );
    // The region is what is drawn, and what a layout test puts against
    // the field; the rows inside it scroll and are clipped to it.
    Node::column()
        .id(super::ABOVE)
        .child(Node::scroll(super::SCROLL, groups))
        .shrink(1.0)
        .min_height(0.0)
}

/// What an entry screen's bottom group holds, for a view asking whether
/// the screen has the room above it for a words panel.
pub struct EntryRoom {
    /// The keyboard the screen types on.
    pub keyboard: KeyboardKind,
    /// The candidate strip's shape where the screen carries one: whether
    /// this list's words are one character each. `None` where the screen
    /// carries no strip.
    pub candidates: Option<bool>,
}

/// Whether an entry screen on this class has the room above its group
/// for a panel of `total` words of this list. A view asks before it
/// offers the app bar's eye, which belongs to the panel (§4.3).
///
/// §4.10: the panel takes two columns where the width allows them and
/// one where it does not, and is left off only where even one column is
/// taller than the space — which is what the 240 dp panels hit, and why
/// word entry there shows the word being typed and nothing else.
pub fn words_panel_fits(
    c: &Chrome,
    room: EntryRoom,
    width: components::secrets::WordWidth,
    total: usize,
) -> bool {
    // §4.3: "On `small` word entry shows the word being typed and
    // nothing else." There is no panel there on any list, so there is no
    // eye either: it would reveal nothing. The words are checked whole
    // on the Words screen once the last one is in.
    if c.class() == SizeClass::Small {
        return false;
    }
    let (_, max_keys) = key_bounds(c, room.keyboard);
    let m = c.pane();
    let space = m.height_dp
        - m.inset_top_dp
        - tokens::APP_BAR
        - c.body_gap()
        - group_height_of(c.class(), room.candidates)
        - max_keys;
    components::secrets::panel_fits(
        c.class(),
        components::secrets::panel_room(c.column().width_dp),
        space,
        width,
        total,
    )
}

/// The keyboard: full-bleed on a panel and a phone, capped and centred
/// in a desktop window, at the height [`keyboard::node_heights`] gives
/// this class — or at the least of that range where the block above it is
/// reserved (§4.3's typed address).
fn keys(c: &Chrome, id: Id, kind: KeyboardKind, enabled: KeyMask, tight: bool) -> Node {
    let class = c.class();
    let (min_h, max_h) = key_bounds(c, kind);
    // §4.3 gives the block above the field priority over the keys: a
    // block that does not fit at the class's key height drops the
    // keyboard to its floor, on whatever class it happens on.
    let max_h = if tight { min_h } else { max_h };
    let kb = Node::widget(Widget::Keyboard {
        id,
        kind,
        enabled,
        scramble: None,
    });
    let kb = if class == SizeClass::Wide {
        organisms::keyboard_slot(kb)
    } else {
        kb
    };
    kb.weight(tokens::KEYBOARD_SHRINK_WEIGHT)
        .min_height(min_h)
        .max_height(max_h)
}
