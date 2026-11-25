# The pane container

**Kind:** Reference
**Status:** Built (F322): the container (`screens/hub_layout/`); Tools, Check, a typed search's results, the step flows, Explore's and Lightning's keys, Files and Wallets' pair are placed by it, and it says how long a change of page takes. Not on it yet: `docs/WORKSPACE.md` §14.

Every page is made of panes (F322). One piece of code says where they stand,
so a page or a flow declares its panes and never works out a place itself. The
rules it follows are `docs/WORKSPACE.md` §4; this document is how the container
is asked and what it owns.

Checked against Material 3's supporting pane canonical layout
(https://m3.material.io/foundations/layout/canonical-layouts/supporting-pane);
the pair's sizes and the stack are F322's.

## 1. How a page declares its panes

A page or a flow has one `Panes`, a constant beside its code:

| Field | What it says |
|---|---|
| `left` | what stands at the left: the session pane in a mode (`Left::Session`, which the container draws), or a pane the page or flow draws itself at the place it is given (`Left::Own`: Your choices, Explore's and Lightning's keys) |
| `tile` | the width rule of what stands at its right: `Tile::Card`, a flow's step card, up to `COLUMN_MAX`; `Tile::Fill`, the room in two columns or more, up to `MENU_TILE_WIDE_MAX` (Tools and Check); `Tile::Detail`, a detail beside a list pane: the pane at the left margin, the detail to the right margin, none where the room holds less than both (List and detail); `Tile::Equal`, two columns as wide as each other a `VISIT_GAP` apart, in `READING_VISIT_W` between two `PAGE_INSET`s, centred in the room, none under the pair's least (Files) |
| `stacks` | whether the tile stands under This session's card where the window has the height |
| `narrow` | what a room too narrow for the pair gets: `Narrow::PaneAtRight` (a flow: the card, then its pane at the right) or `Narrow::OneColumn` (no pair) |

Its name is the page's own (`menu_page::Page::name`, a flow's heading).

| Who | `left` | `tile` | `stacks` | `narrow` |
|---|---|---|---|---|
| Tools, Check | `Session` | `Fill` | yes | `OneColumn` |
| A typed search's results | `Session`: the page's, or Home's rows | `Fill` | yes | `OneColumn` |
| A step flow (`flow/act.rs`) | `Own` | `Card` | no | `PaneAtRight` |
| Explore (`explore/pane.rs`) | `Own` | `Card` | no | `OneColumn` |
| Lightning (`lightning/pane.rs`) | `Own` | `Card` | no | `OneColumn` |
| Files (`screens/visit.rs`) | `Own`: In Faraday | `Equal`: the stick | no | `OneColumn`: a small panel's tabs, and a room under the pair's least keeps the two columns it had |
| Wallets and a wallet's page (`screens/wallets_pair.rs`) | `Session`: Wallets' mode | `Card` | no | `OneColumn` |

## 2. What it answers

All of it is arithmetic on the room, with no state and nothing worked out from
what is loaded. A height is asked for only after the one before it did not
fit: the stack needs This session's card and the tile at its width, and only a
page that does not stack is measured as a pair.

- `Panes::place(room, guide)`: the pair's `Place` (the pane's and the card's
  left edges and widths, and the gap), from the room's left edge, or none.
  `guide` is whether Guide's pane stands at the room's right edge.
- `Panes::stack(card_h, tile_h, h)`: the tile's top under This session's card
  where both fit the window `h` tall with a margin under them, or none.
- `Panes::pane_beside_work(room)`: the pane's width at the right of content
  that fills the room (a flow's wide content).
- `level(a, b, most)`: where the pair ends, level: the longer of the two, at
  most `most`.
- `lines` and `limits`: the pair's divider and outline, and the top and foot
  that the panes scroll between.
- `guide_place((x0, cw, h))`: where Guide's pane stands at the right edge (and `guide_place_at`, `guide_room`, `guide_top`); the glide and the dice sheet's column read it, no page works it out.
- `holds_guide(room)`: whether the room holds a flow's pair at its narrowest
  with Guide's pane beside it.
- `home_area(room)`: the area Home's column has, where a page stands that has
  no pair in this room.
- `page_turn(style, speed)`: what a change of page does under the Motion
  setting and how long it takes at the Speed, or none for at once.
  `STEP_MS`: how long a step of a flow takes.

`draw` and `draw_results` draw a menu page placed, its tile's surface and its
session pane; what fills the tile is the page's (`docs/MENU-PAGE.md`).

## 3. What it owns

- **Where the pair stands.** The pane is `LIST_PANE` wide, the gap `SPACER`,
  the card the rest up to `COLUMN_MAX`, the pair centred in the room beside
  the navigation. A narrower room narrows the card to `FLOW_CARD_MIN`, then
  the pane to `CHOICES_PANE_MIN`; narrower still, `narrow` says. With Guide's
  pane the pair moves left only as far as that pane needs.
- **Where the tile fills.** From the pane to the right page margin, at most
  `MENU_TILE_WIDE_MAX`, the pair centred past it; where the room holds one
  column the pair stands at a flow's place.
- **The stack.** Under This session's card as Home has it, as wide, `SPACER`
  under it, ending a `MARGIN_WIDE` above the window's foot.
- **The level ends** of every pair.
- **The time of a change of page** and what the Motion and Speed settings do
  in it (`docs/MOTION.md` §3.5); `fade.rs` draws the change.

## 4. Where the code is

`screens/hub_layout/`: `mod.rs` the declaration and its answers, `pair.rs` the
pair's arithmetic, `page.rs` a menu page drawn placed, `lines.rs` the pair's
lines, `turn.rs` the change of page.

## 5. What lint holds

`lint-components` counts, outside the container's folder, and the counts only
fall:

- `pane-own-place`: a page or a flow that takes a pane's widths or the room a
  pane leaves (`LIST_PANE`, `SUPPORT_PANE`, `CHOICES_PANE_MIN`,
  `CHOICES_PANE_MAX`, `FLOW_CARD_MIN`, `MENU_TILE_WIDE_MAX`, `side_panel_w`,
  `support_pane_room`; the container now has the guide's place, so the counts are none).
- `page-own-time`: a page that times a change of page itself (the
  Speed's `ms()`, what a Motion style does).

## 6. A new page

Declares its `Panes` beside its code, asks `place` with the room it is drawn
in, adds its row to §1's table, and draws what the panes hold.
