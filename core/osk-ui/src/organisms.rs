//! Composite pieces built from widgets and layout nodes (UX.md §5, §6).
//!
//! Every non-Home screen has the same chrome: a 56 dp **app bar** with a
//! back chevron, a centred title and an optional trailing action; a
//! scrolling body; and, where the screen has one main action, a
//! bottom-anchored accent button. Wizards and documents differ only in
//! whether that button is there. Each function returns a [`Node`] subtree;
//! nothing here is stateful.

use alloc::string::String;
use alloc::vec::Vec;

use crate::geom::{Edges, Metrics};
use crate::layout::LayoutCtx;
use crate::layout::{Align, Id, Justify, Node};
use crate::text::{Font, TextAlign};
use crate::widgets::keyboard;
use crate::widgets::{Icon, PanelStyle, Tone, Widget, tokens};

/// A hub tile description.
pub struct TileSpec {
    /// Application id.
    pub id: Id,
    /// Icon.
    pub icon: Icon,
    /// Label.
    pub label: String,
    /// Enabled.
    pub enabled: bool,
    /// A count in the tile's corner; `None` leaves it plain.
    pub badge: Option<String>,
}

/// Columns in the hub grid for the pane the grid is drawn in: two on a
/// panel taller than it is wide, four on one wider than it is tall, and
/// three in a `Wide` content pane.
pub fn hub_columns(m: &Metrics) -> usize {
    tokens::hub_columns(m.class, m.width_dp > m.height_dp)
}

/// A grid of equal tiles in rows of [`hub_columns`] (UX.md §4). Every row
/// carries the same weight, so the rows share the height the grid is
/// given and no tile is larger than another; every gap is one grid step.
/// A short row is filled with empty cells rather than stretched.
///
/// A row is at most [`tokens::TILE_ASPECT`] times one cell's width — a
/// square, since `docs/DESIGN.md` §4.1 says a tile is never taller than
/// it is wide. [`tile_grid_height`] is the height at which every row is
/// that square; a grid given less draws its tiles wider than they are
/// tall rather than scrolling, and a grid given more packs at the bottom
/// of the space, which is where a thumb reaches it.
pub fn tile_grid(m: &Metrics, tiles: Vec<TileSpec>) -> Node {
    let cols = hub_columns(m);
    let cell_w = (m.width_dp - 2.0 * tokens::PAD - (cols as f32 - 1.0) * tokens::GAP) / cols as f32;
    let max_row = (cell_w * tokens::TILE_ASPECT).max(tokens::TILE_MIN_HEIGHT);
    let mut grid = Node::column().gap(tokens::GAP).justify(Justify::End);
    let mut tiles = tiles.into_iter().peekable();
    while tiles.peek().is_some() {
        let mut row = Node::row()
            .gap(tokens::GAP)
            .weight(1.0)
            .min_height(tokens::TILE_MIN_HEIGHT)
            .max_height(max_row);
        for _ in 0..cols {
            match tiles.next() {
                Some(t) => {
                    row = row.child(
                        Node::widget(
                            Widget::tile(t.id, t.icon, t.label, t.enabled).with_badge(t.badge),
                        )
                        .weight(1.0),
                    );
                }
                None => row = row.child(Node::column().weight(1.0)),
            }
        }
        grid = grid.child(row);
    }
    grid
}

/// Height in dp at which a [`tile_grid`] of `tiles` tiles in a pane `m`
/// wide draws every tile as the square §4.1 asks for: the rows at one
/// cell's own width, and the grid step between them.
pub fn tile_grid_height(m: &Metrics, tiles: usize) -> f32 {
    let cols = hub_columns(m);
    let cell_w = (m.width_dp - 2.0 * tokens::PAD - (cols as f32 - 1.0) * tokens::GAP) / cols as f32;
    let row = (cell_w * tokens::TILE_ASPECT).max(tokens::TILE_MIN_HEIGHT);
    let rows = tiles.div_ceil(cols).max(1) as f32;
    rows * row + (rows - 1.0) * tokens::GAP
}

/// A band of equal tiles in one row, at the height a tile needs for its
/// mark and its word ([`tokens::tile_band_height`]). §4.1 says a tile is
/// never taller than it is wide, and a band of three is well inside
/// that. Every tile takes the same share of the width, with one grid
/// step between them.
///
/// This is the band Home draws over its actions, where a grid would take
/// the height the list needs.
pub fn tile_band(class: crate::geom::SizeClass, tiles: Vec<TileSpec>) -> Node {
    Node::row()
        .gap(tokens::GAP)
        .height(tokens::tile_band_height(class))
        .children(tiles.into_iter().map(|t| {
            Node::widget(Widget::tile(t.id, t.icon, t.label, t.enabled).with_badge(t.badge))
                .weight(1.0)
        }))
}

/// One entry of a [`sidebar`].
pub struct SidebarItem {
    /// Application id; `None` for an entry that is not available yet.
    pub id: Option<Id>,
    /// Icon.
    pub icon: Icon,
    /// Label.
    pub label: String,
    /// Drawn as the current area.
    pub selected: bool,
}

/// A navigation sidebar: one row per item, the current one in the accent
/// colour, on a divider from the content. Fixed at [`SIDEBAR_WIDTH`].
///
/// `dimmed` draws every row disabled and gives none of them a hit
/// target: the sidebar keeps its place on the screen while a wizard, the
/// scanner, the lock screen or a hold-to-confirm step owns the pane, so
/// the content never shifts sideways between two screens of one flow,
/// and navigation never shares a screen with a secret or a destructive
/// step (`docs/PLANNING.md` §16.29).
pub fn sidebar(items: &[SidebarItem], dimmed: bool) -> Node {
    let mut col = Node::column()
        .padding(Edges::symmetric(tokens::GAP, tokens::PAD))
        .gap(tokens::GAP_SMALL);
    for item in items {
        let id = if dimmed { None } else { item.id };
        col = col.child(Node::widget(
            // The rail marks the current area with the accent, not
            // with a chevron on every row. A dimmed row is the area's
            // name in the dimmed tone and nothing else (§4.1).
            Widget::list_row(id, item.label.clone(), None, None)
                .with_icon(item.icon)
                .with_enabled(id.is_some())
                .without_chevron()
                .with_tone(if item.selected && !dimmed {
                    Tone::Primary
                } else {
                    Tone::Text
                }),
        ));
    }
    Node::row()
        .child(col.width(tokens::SIDEBAR_WIDTH))
        .child(Node::widget(Widget::VerticalDivider))
}

/// The `Wide` template (UX.md §2, §4): a [`sidebar`] on the left, the
/// selected area's `content` taking the rest. Every `Wide` screen uses
/// it; `dimmed` is the only difference between them. `id` names the
/// sidebar's own rectangle, which a layout test asserts never moves.
pub fn sidebar_frame(id: Id, items: &[SidebarItem], dimmed: bool, content: Node) -> Node {
    Node::row()
        .child(sidebar(items, dimmed).id(id))
        .child(content.weight(1.0))
}

/// `content` at most `max_dp` wide, centred in the width it is given.
///
/// `docs/DESIGN.md` §3: "On `wide` the capped column, and every list in
/// it, is centred in the pane under the centred title." One function, so
/// the column, the QR, the pad, the viewfinder and a record's table all
/// share the pane's centre with the app bar's title above them and the
/// primary action below.
pub fn capped(max_dp: f32, content: Node) -> Node {
    Node::row()
        .justify(Justify::Center)
        .child(content.weight(1.0).max_width(max_dp))
}

/// A keyboard capped at [`KEYBOARD_MAX_WIDTH`] and centred, so that a
/// key never grows past a thumb's reach on a desktop window. Vertical
/// constraints belong on the returned row.
pub fn keyboard_slot(keyboard: Node) -> Node {
    Node::row()
        .justify(Justify::Center)
        .child(keyboard.weight(1.0).max_width(tokens::KEYBOARD_MAX_WIDTH))
}

/// A PIN, dice or coin pad capped at [`keyboard::PAD_MAX_WIDTH`] and
/// centred: a pad wider than that letterboxes its cells, which is what a
/// phone or a desktop window would otherwise do to it.
pub fn pad_slot(pad: Node) -> Node {
    Node::row()
        .justify(Justify::Center)
        .child(pad.weight(1.0).max_width(keyboard::PAD_MAX_WIDTH))
}

/// Flex weight of a screen body relative to a weighted footer.
const BODY_SHRINK_WEIGHT: f32 = tokens::BODY_SHRINK_WEIGHT;

/// The app bar (UX.md §5): a back chevron in a 48 dp target at the left,
/// the title centred, an optional trailing action at the right. Each side
/// slot keeps its width whether or not it holds a control, so the title
/// is centred on the bar and not on what is left of it. A trailing slot
/// of two actions is the one exception: it takes its extra width from
/// the title.
pub fn app_bar(back: Option<Id>, title: impl Into<String>, trailing: Option<Node>) -> Node {
    bar(back, title, trailing, Font::semibold(tokens::TITLE))
}

/// The same bar titled in the mono face: a screen whose title is read
/// character by character, which is a fingerprint (§16.126).
pub fn app_bar_mono(back: Option<Id>, title: impl Into<String>, trailing: Option<Node>) -> Node {
    bar(back, title, trailing, Font::mono(tokens::TITLE))
}

fn bar(back: Option<Id>, title: impl Into<String>, trailing: Option<Node>, font: Font) -> Node {
    // A trailing node that has claimed a width of its own keeps it: two
    // actions need two targets. The leading slot stays one target wide,
    // because taking a second one from the title costs more than the
    // title's centring does.
    let slot = |content: Option<Node>| match content {
        Some(n) => {
            let w = n.fixed.0.unwrap_or(tokens::TOUCH);
            n.width(w)
        }
        None => Node::column().width(tokens::TOUCH),
    };
    Node::row()
        .height(tokens::APP_BAR)
        .padding(Edges::symmetric(tokens::GAP, 0.0))
        .align(Align::Center)
        .child(slot(back.map(|id| {
            Node::widget(Widget::icon_button(
                id,
                Icon::Back,
                tokens::ICON_SMALL,
                Tone::Text,
            ))
        })))
        .child(
            Node::widget(Widget::text(title, font, Tone::Text).align(TextAlign::Center))
                .weight(1.0),
        )
        .child(slot(trailing))
}

/// An app-bar trailing action.
pub fn bar_action(id: Id, icon: Icon) -> Node {
    Node::widget(Widget::icon_button(
        id,
        icon,
        tokens::ICON_SMALL,
        Tone::Text,
    ))
}

/// Two app-bar actions in one trailing slot, which is twice as wide as
/// one and takes the extra width from the title.
pub fn bar_actions(first: Node, second: Node) -> Node {
    Node::row()
        .width(2.0 * tokens::TOUCH)
        .child(first)
        .child(second)
}

/// [`screen`] with the space under the body given. A body that sits
/// directly above an action row needs the grid step between them and no
/// more: the row carries the screen's own bottom margin under itself, so
/// spending a second one here costs a list its last row on a 268 dp
/// panel.
pub fn screen_at(
    scroll_id: Id,
    bar: Node,
    body: Node,
    body_min: f32,
    footer: Option<Node>,
    bottom_dp: f32,
    cap_dp: Option<f32>,
) -> Node {
    // Justified to the end so that the last child — a keyboard, a call
    // to action — reaches the bottom edge exactly. Two weighted children
    // can leave a pixel of the height unclaimed by integer division;
    // that pixel belongs under the app bar, not under the keyboard.
    let scroll = Node::scroll(
        scroll_id,
        body.padding(Edges {
            left: tokens::PAD,
            right: tokens::PAD,
            top: 0.0,
            bottom: bottom_dp,
        }),
    )
    .filling();
    // §3: "a scrollbar is drawn against the column it scrolls, not
    // against the window edge." The scrollbar is drawn at the scroll
    // region's own right edge, so on `wide` the cap goes round the
    // region and not inside it.
    let scroll = match cap_dp {
        Some(cap) => capped(cap, scroll.weight(BODY_SHRINK_WEIGHT).min_height(body_min))
            .weight(BODY_SHRINK_WEIGHT)
            .min_height(body_min),
        None => scroll.weight(BODY_SHRINK_WEIGHT).min_height(body_min),
    };
    let mut frame = Node::column()
        .justify(Justify::End)
        .child(bar)
        .child(scroll);
    if let Some(f) = footer {
        frame = frame.child(f);
    }
    frame
}

/// [`static_screen`] with the space under the body given
/// (`tokens::action_bottom` for the shell's bottom inset), so a body that
/// reaches the bottom of a phone stands clear of its gesture bar.
pub fn static_screen_at(bar: Node, body: Node, footer: Option<Node>, bottom_dp: f32) -> Node {
    let mut frame = Node::column().child(bar).child(
        body.weight(1.0)
            .padding(Edges {
                left: tokens::PAD,
                right: tokens::PAD,
                top: 0.0,
                bottom: bottom_dp,
            })
            .min_height(0.0),
    );
    if let Some(f) = footer {
        frame = frame.child(f);
    }
    frame
}

/// A row of footer buttons: secondary ones take their natural width, the
/// last (primary) button takes the rest.
fn footer_row(buttons: Vec<Node>) -> Node {
    let n = buttons.len();
    Node::row().gap(tokens::GAP).children(
        buttons
            .into_iter()
            .enumerate()
            .map(|(i, b)| if i + 1 == n { b.weight(1.0) } else { b }),
    )
}

/// [`cta`] with the space under the buttons given: the screen padding
/// plus the shell's bottom inset (`tokens::action_bottom`), so the
/// buttons stand clear of a phone's gesture bar.
pub fn cta_at(buttons: Vec<Node>, bottom_dp: f32) -> Node {
    footer_row(buttons).padding(Edges {
        left: tokens::PAD,
        right: tokens::PAD,
        top: tokens::GAP,
        bottom: bottom_dp,
    })
}

/// Where a screen's body sits in the space between the app bar and the
/// bottom action (UX review 2026-09-07, §2b.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Body {
    /// The body's own height, under the app bar, with the spare height
    /// below it. The default for a step: what belongs together stays
    /// together at the top, and the action stays at the bottom.
    Top,
    /// Centred in the whole space. Only a verdict screen ([`hero`]) uses
    /// this: an icon, a title and a sentence read better in the middle.
    Center,
    /// The body takes the whole space and a weighted child inside it
    /// absorbs what is left: a code, a viewfinder, a hub grid.
    Fill,
}

/// `body` placed in the space its frame gives it.
fn place(placement: Body, body: Node) -> Node {
    match placement {
        Body::Fill => body,
        Body::Center => Node::column().justify(Justify::Center).child(body),
        Body::Top => Node::column().child(body).child(Node::spacer()),
    }
}

/// A screen whose body must not scroll and whose actions are pinned to
/// the bottom: [`static_screen`] with an app bar and a [`cta`] row.
/// `placement` decides what happens to the height the body does not use.
pub fn action_screen(
    back: Option<Id>,
    trailing: Option<Node>,
    title: impl Into<String>,
    placement: Body,
    body: Node,
    footer: Vec<Node>,
) -> Node {
    action_screen_at(back, trailing, title, placement, body, footer, tokens::PAD)
}

/// [`action_screen`] with the space under the footer, or under the body
/// when there is none, given (`tokens::action_bottom` for the shell's
/// bottom inset).
pub fn action_screen_at(
    back: Option<Id>,
    trailing: Option<Node>,
    title: impl Into<String>,
    placement: Body,
    body: Node,
    footer: Vec<Node>,
    bottom_dp: f32,
) -> Node {
    let footer = (!footer.is_empty()).then(|| cta_at(footer, bottom_dp));
    static_screen_at(
        app_bar(back, title, trailing),
        place(placement, body),
        footer,
        bottom_dp,
    )
}

/// The one secret element (UX.md §5): a tinted surface with an accent
/// border holding `content`, which shows while a finger is anywhere on
/// the panel and re-masks on release. `caption` is the short line under
/// the content; there is no reveal button and no second caption.
///
/// The caption keeps its line in both states, and callers mask their
/// content to the revealed line count, so the panel does not resize
/// under the finger that reveals it (UX review 2026-09-07, finding 9).
///
/// Whatever the panel holds must fit it on one page: the surface is the
/// only hit target inside, so a paged string would steal the touch.
pub fn secret_panel(id: Id, content: Node, caption: Option<String>) -> Node {
    let mut column = Node::column()
        .pad(tokens::GAP)
        .gap(tokens::GAP)
        .justify(Justify::Center)
        .child(content);
    if let Some(text) = caption {
        column = column.child(
            Node::widget(
                Widget::text(text, Font::regular(tokens::CAPTION), Tone::Muted)
                    .align(TextAlign::Center),
            )
            .align_self(Align::Stretch),
        );
    }
    Node::stack()
        .child(Node::widget(Widget::secret_surface(id)))
        .child(column)
}

/// The same field at a height of its own, for a screen that has to buy
/// its rows back from somewhere: the lock card on a 358 dp panel.
pub fn field_at(height_dp: f32, value: impl Into<String>, font: Font, align: TextAlign) -> Node {
    Node::stack()
        .height(height_dp)
        .child(Node::widget(Widget::Panel {
            style: PanelStyle::Surface,
        }))
        .child(
            // A row centres the line on the cross axis; the text keeps
            // the whole width, and with it its own alignment.
            Node::row()
                .align(Align::Center)
                .padding(Edges::symmetric(tokens::GAP, 0.0))
                .child(
                    Node::widget(Widget::text(value, font, Tone::Primary).align(align)).weight(1.0),
                ),
        )
}

/// Height in dp of one line of a [`descriptor`], which is the height of
/// the compact chip its key is drawn in.
const DESCRIPTOR_LINE: f32 = tokens::CHIP_COMPACT;

/// Padding at each end of that chip, in dp.
const DESCRIPTOR_CHIP_PAD: f32 = tokens::CHIP_COMPACT_PAD;

/// A descriptor as the tokens a reader checks, wrapped at token
/// boundaries (UX review 2026-09-07, §2.9).
///
/// The function names and brackets are as they are, the key origin
/// `[73c5da0a/84h/0h/0h]` is whole and in the accent colour, the
/// `/<0;1>/*` suffix and the `#checksum` are whole, and only the
/// extended key is shortened — to its first and last eight characters,
/// in a chip that opens the whole key when `key_id` is given. Chunking
/// in fours is for a string a person compares character by character; a
/// descriptor is read by its parts.
///
/// `avail_dp` is the width the block is given.
pub fn descriptor(ctx: &LayoutCtx, avail_dp: f32, text: &str, key_id: Option<Id>) -> Node {
    descriptor_at(
        ctx,
        avail_dp,
        text,
        key_id,
        tokens::CAPTION,
        DESCRIPTOR_LINE,
    )
}

/// The same block at a monospace size and line height of its own, for a
/// screen whose budget names them (`docs/COMPOSITION.md` §6).
pub fn descriptor_at(
    ctx: &LayoutCtx,
    avail_dp: f32,
    text: &str,
    key_id: Option<Id>,
    size_dp: f32,
    line_dp: f32,
) -> Node {
    let font = Font::mono(size_dp);
    let face = font.sized(ctx.scale);
    let avail = ctx.px(avail_dp);
    let chip_pad = 2 * ctx.px(DESCRIPTOR_CHIP_PAD);
    let mut column = Node::column();
    let mut line = Node::row().align(Align::Center).height(line_dp);
    let mut used = 0;
    let mut empty = true;
    for t in crate::descriptor::tokens(text) {
        let is_key = t.kind == crate::descriptor::Kind::Key;
        let width = crate::text::width(&face, &t.text) + if is_key { chip_pad } else { 0 };
        if !empty && used + width > avail {
            column = column.child(line);
            line = Node::row().align(Align::Center).height(line_dp);
            used = 0;
        }
        used += width;
        empty = false;
        line = line.child(Node::widget(if is_key {
            Widget::compact_chip(key_id, t.text, false).with_label_size(size_dp)
        } else {
            Widget::text(t.text, font, descriptor_tone(t.kind))
        }));
    }
    column.child(line)
}

/// The colour role of a descriptor token: the origin is the part a
/// reader checks against their own key, so it takes the accent.
fn descriptor_tone(kind: crate::descriptor::Kind) -> Tone {
    match kind {
        crate::descriptor::Kind::Origin => Tone::Primary,
        crate::descriptor::Kind::Syntax => Tone::Muted,
        _ => Tone::Text,
    }
}

/// A labelled section heading inside a document.
pub fn section(title: impl Into<String>) -> Node {
    Node::column()
        .padding(Edges {
            top: tokens::GAP,
            ..Edges::default()
        })
        .child(Node::widget(
            Widget::paragraph(title, Font::semibold(tokens::CAPTION), Tone::Muted)
                .align(TextAlign::Start),
        ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::{Rect, Scale, SizeClass};
    use crate::layout::{LayoutCtx, solve};
    use crate::widgets::HitTarget;

    /// The 2.8" reference panel, portrait.
    fn small() -> (Rect, LayoutCtx) {
        (
            Rect::new(0, 0, 480, 640),
            LayoutCtx::new(Scale::new(286), SizeClass::Small),
        )
    }

    #[test]
    fn a_secret_panel_reveals_on_press_and_masks_on_release() {
        use crate::state::{Action, UiState};
        use osk_shell_api::{Event, TouchPhase};
        let (area, ctx) = small();
        let panel = Node::column().child(
            secret_panel(
                Id(4),
                Node::widget(Widget::text("hidden", Font::mono(tokens::MONO), Tone::Text)),
                Some(String::from("Touch and hold to show")),
            )
            .weight(1.0),
        );
        let l = solve(&panel, area, &ctx);
        let surface = l
            .items
            .iter()
            .find(|p| p.hit == Some(HitTarget::Reveal(Id(4))))
            .expect("the panel is one touch surface");
        assert_eq!(surface.rect, area, "the whole panel reveals");
        let c = surface.rect.center();
        let touch = |phase| Event::Touch {
            x: c.x as u16,
            y: c.y as u16,
            phase,
        };
        let mut state = UiState::new(ctx.scale);
        assert!(!state.is_held(Id(4)));
        assert_eq!(
            state.event(&l, touch(TouchPhase::Down)),
            Some(Action::Redraw)
        );
        assert!(state.is_held(Id(4)), "revealed while the finger is down");
        assert_eq!(state.event(&l, touch(TouchPhase::Up)), Some(Action::Redraw));
        assert!(!state.is_held(Id(4)), "masked again on release");
        // No hold completes, so no irreversible action can hang off it.
        state.event(&l, touch(TouchPhase::Down));
        assert_eq!(
            state.event(&l, Event::Tick { now_ms: 10_000 }),
            None,
            "a reveal never completes a hold"
        );
    }
}
