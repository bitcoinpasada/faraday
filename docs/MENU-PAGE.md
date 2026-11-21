# One menu page container

**Kind:** Reference
**Status:** Built (F298): the container (`screens/menu_page.rs`), and Home, Tools, Check, Wallets start and Settings drawn on it.

The menu pages are the pages a person picks a place or a tool from:
Home, Tools (`Screen::Catalog`, the sidebar's Tools), Check, Wallets
start (`Screen::WalletsStart`, Wallets with none loaded) and Settings.
They are drawn by one container, Material 3's feed layout: equal items
in a grid whose column count follows the room. Vault contents is not a
menu page; it is list and detail (`docs/LAYOUT.md` §5). The decision is
F298; this document is how the container draws and what each page gives
it.

Checked against Material 3's feed canonical layout as Android's
adaptive guidance states it (Material's own feed page, m3.material.io,
could not be read when this was written): "A feed layout arranges equivalent
content elements in a configurable grid for quick, convenient viewing of
a large amount of content", and "the grid can adapt from a single,
scrolling column to a multi-column scrolling feed of content"
(developer.android.com, "Canonical layouts"); its sample sizes the grid
by a minimum item width (`GridCells.Adaptive`), not by a fixed column
count.

## 1. What a menu page declares

A page hands the container, each frame, only what it alone knows:

- **Its name**, for the top app bar, and whether the bar carries a way
  back (Check's does).
- **Whether it has the search bar** (Home and Tools), and what typing in
  it shows: today's results, `catalog::home_results`.
- **Its sections, in order.** Each section has an optional label and one
  of three bodies:
  - **tiles**: faces (`catalog::Face`: icon, name, line, what a press
    does), laid out in the grid (§4);
  - **cards**: blocks the page draws in a given width, returning their
    height, laid out in the grid by column (§6; Settings);
  - **a block across the page**: something the page draws in the
    page's width, returning its height (Home's lead tile and first
    actions, Wallets start's lists of wallets and keys).
- **The most columns it takes**, where fewer than the grid's: Settings
  takes two at most (§6).
- **What stands above the search bar** on a one-column layout, where it
  has any: Home's box of what is loaded, and on the Pi's panel its mark
  and name (`docs/LAYOUT.md` §4).

A page draws nothing of the page's frame itself: not its width, its
inset, its centring, the bar's place, a column count or a gap.

## 2. Width and centring

- **On the wide layout** (the expanded rail, `docs/LAYOUT.md` §3) the
  page is at most `MENU_PAGE_W` wide, a new token: three tiles at the
  minimum and the two gaps between them (3 × `CATALOG_TILE_MIN` + 2 ×
  `MENU_GAP`, 1112 dp). It stands `PAGE_INSET` in from each side of the
  room beside the navigation and is centred in that room where the room
  is wider. All five pages start and end at the same edges, so a person
  moving between them sees the grid stay where it was.
- **On the one-column layouts** (the rail's windows, the phone-size
  window and the Pi's panel: every layout where `Ui::compact` is set)
  the page takes the area less `compact::M` at each side.

`MENU_PAGE_W` replaces `TRANSFER_W` for these pages; Transfer keeps its
own. The screen area map (`screens/mod.rs`, `area`) gives the five
pages one area, the menu page's, in place of today's `READING_TRANSFER_W`
for four of them and `Area::Work` for Settings.

## 3. The search bar's place

Where a page has it (Home, Tools): Material's search bar
(`home_search_bar`, `SEARCH_BAR_H`), the first thing under the top app
bar, centred in the page and at most `LEAD_MAX` wide on the wide
layout, the page's full width on the one-column layouts; on the
phone-size window it stands under what Home puts above it (§1). It stays
still while the page scrolls under it, as Home's does today
(`home_pinned`), and the whole page scrolls as one where the window
leaves no room under the bar for about three rows. Tools takes Home's
rule: today its bar scrolls with its tiles.

## 4. Columns

One minimum for every tile: `CATALOG_TILE_MIN` (360 dp). The grid takes
as many columns as the page holds tiles at least that wide with
`MENU_GAP` between them, and at least one; each tile takes the row's
share; a row with fewer tiles than columns leaves the rest empty. There
is no column floor: a tile is never drawn narrower than the minimum
unless the page itself is narrower (the Pi's panel), and then one tile
takes the row. Tools' "at least three beside the navigation"
(`CATALOG_COLUMNS_MIN`) goes: in Material's feed the room sets the
count, and the floor is what makes a tile narrower than its minimum
today (about 305 dp at 1280 × 800).

By width class (`docs/LAYOUT.md` §2) this gives, at one dp a pixel:

| Class | Window | Columns |
|---|---|---|
| Compact | the Pi's panel, 480 × 640, any window under 600 dp | one; the Pi's panel's Home menu two to a row (§7) |
| Medium | 600 to 839 dp | one (two need 760 dp beside the rail) |
| Expanded | 840 to 1199 dp, a 1024 × 768 window | two |
| Large | 1280 × 800, the desktop app's default | two, three from about 1448 dp |
| Extra-large | 1920 × 1080 and over | three, the page at `MENU_PAGE_W` |

So no page holds more than three columns, and Settings two (§6). The
column count follows the page's width alone, never the window's class,
so a larger text size steps it down as a narrower window would.

## 5. Spacing and section labels

- **Between tiles and cards**, across and down, one gap: `MENU_GAP`, a
  new token, 16 dp (2 × `GRID`), which Home uses today as a bare
  number. It replaces Tools' `TILE_GAP` (14, a bare number in
  `catalog.rs`), `CATALOG_ROW_GAP` (10) and Settings' `SPACER` between
  its columns.
- **A section's label**: `section_label` in Title S, `GRID` under it to
  the first row, and `2 × GRID` after the section's last row before the
  next label (today's `catalog::section`). A section without a label
  starts at the gap.
- **The page's top**: `PANE_TOP_BAR` under the top app bar on every
  layout; it replaces Tools' 14, Check's `CHECK_TOP` and Settings' list's
  `compact::M` at the top.
- **The page's foot**: `MARGIN_WIDE` under the last section, and on a
  page with Scan room for it (`docs/LAYOUT.md` §15).
- **Tiles of a page are one height**, the tallest face's at the tile
  width (`Grid::matching`), as Tools and Check have it; a face that does
  not fit its line wraps and its row grows.

## 6. Cards of different heights (Settings)

A page of cards stands in the same page, at the same edges and with the
same gap as the others. Its columns are worked out as tiles' are, with
`CATALOG_TILE_MIN` the least width of a card, at most the page's own
most (Settings: two): one column on a narrow page, two where two fit.

Cards are of different heights and are not set in rows: each column is
a stack, a card under the one before it with `MENU_GAP` between. The
container divides them where the two columns' heights come out
closest, at the page's width and the person's text size and scale, so a
card may move from one column to the other as these change (Settings;
owner, 2026-11-21, `docs/LAYOUT.md` §17). The order reads down the first
column and then the second, and on one column down the one. The division
is worked out when the width, text size, scale or a card's height
changes, kept, and read while drawing (`docs/SPEED.md` §3.3): a card
whose height drawn differs from the one kept, its content having
changed, is divided afresh by the next frame. Each card stays reading
content (`docs/LAYOUT.md` §14): at two columns in `MENU_PAGE_W` it is
under `COLUMN_MAX`, so `COLUMN_MAX` no longer caps Settings' columns.

## 7. The one-column layouts and the small panel

On every layout where `Ui::compact` is set, the 480 × 640 panel and the
phone-size window among them, the container draws the same sections in
the area less `compact::M` at each side: the top app bar names the page
(`compact::bar`), what the page puts above the bar, the bar full width
(§3), then the sections, tiles by the rule of §4. A compact window is
under 600 dp, so it is one column there; a rail window wide enough takes
two.

- **Home** on the one-column layouts is drawn by the container too:
  `compact::home`'s own grid of places is gone, and its places are a tiles
  section. The Pi's panel
  menu (`Nav::Menu`) keeps its places two to a row, as the panel's
  navigation (`docs/LAYOUT.md` §3), with Lock and Power off at its
  foot: a page that declares it holds its two-to-a-row grid there and
  nowhere else.
- **Settings** keeps Material's list and detail one pane at a time on a
  compact window (`listed()`): the list is a section of list items, and
  a section opened is its card alone on the page, the top app bar naming
  it. This is the container's path for a page of cards on a compact
  window; from medium up §6 holds.

## 8. What each page gives it

- **Tools** (`screens/catalog.rs`): its name, the search bar and
  `home_results`, and one tiles section per group (`catalog::GROUPS`,
  `TILES`); `draw` and `tiles_by_group` give the faces.
- **Check** (`check/screen.rs`): its name with the way back, and one
  tiles section per group.
- **Home** (`screens/home.rs`, `compact/mod.rs`): its name (the wordmark
  in the sidebar's head on the wide layout), the search bar and its
  results, the lead tile and the first actions as blocks across the
  page, the loaded wallets beside Backups and the Keys as tiles
  sections, and on the one-column layouts the box of what is loaded
  above the bar; on the Pi's panel, its places two to a row (§7).
- **Wallets start** (`screens/wallets_start.rs`): its name, the five
  ways in as a tiles section, and its lists of wallets and of keys
  without a wallet as sections.
- **Settings** (`screens/settings.rs`): its name, its sections' cards
  (`part`) as a cards section of at most two columns, and on a compact
  window the list.

Tokens (`core/faraday-draw/src/tokens.rs`): `MENU_PAGE_W`, `MENU_GAP`
and `CATALOG_TILE_MIN`, the one tile minimum.

The container is shared code beside the flow container
(`faraday/faraday-core/src/flow.rs`): a menu page module of its own,
`faraday/faraday-core/src/screens/menu_page.rs`, called from each page's
`draw`. `docs/LAYOUT.md` §4 and §17 and `docs/DESIGN.md` §5's Home,
Catalog and Settings rows describe the pages it draws.

## 9. What the tests hold

What a person sees and does, at the snapshot sizes (the Pi's panel,
480 × 640, a rail window, 1280 × 800, 1920 × 1080):

- every tile's name and line show whole, none cut, on every menu page;
- no two tiles or cards overlap, and none runs under the navigation,
  the search bar or Scan, at any scroll position;
- a press on a tile opens what its face says, on every page and layout;
- typing on Home or Tools shows the matching tools under the bar, a
  result opens its tool, and clearing the bar brings the page back;
- the search bar stays in view while the page scrolls under it;
- Settings' cards read in their order down the first column and then the
  second, the two columns come out about equal in height, and on a compact window a section opens from the list and
  Back returns to the list where it was scrolled;
- the Pi's panel Home shows its places and Lock and Power off.

They never state a column count, a width, a token's value or one page's
geometry against another's.
