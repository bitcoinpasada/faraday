# Layout at every window size: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/LAYOUT.md` on 2026-10-28, when the last of
its open work was built and it became a Reference. Each keeps its
original number; the source keeps a one-line stub under the same
heading.

## Header: the spec's status as it was (from docs/LAYOUT.md)

Status: Partly built: size classes and density (L1, F101; the stick's from its EDID on a small screen, `docs/ORIENTATION.md` §2.1), navigation (L2), Home's lead tile and tile grid (L3), the supporting pane's place and its narrow strip, for codex32's Demo only card (§6); the page area to the window's edge (W1, §15, §19); list and detail for Wallets, Files, Vaults and Backups (L4, §5, §17); Settings' list of sections on compact windows and every section on one page from medium up (§17); Files' two lists each scrolling on its own beside the rail (§17); the codex32 entry's input panel (W3, §16.3), at the foot of a compact window that is not short too, above the navigation bar, and New key's dice and Create a vault's Generate with dice in it, and beside the sheets scrolling within itself; Learn in the supporting pane on extra-large windows (L6, §6); the tours at every class's size, §11's tests and DESIGN §3 written to this spec (L7); on wide windows the worksheet's fit and the string's boxes (W2, §16.1, §16.2), the view and the page following the work (W4, §16.4), the step-through's buttons fixed in Faraday Demo's host strip (W5, §16.5), Home's feed, Explore, the chart's buttons and tables at their columns' width (W6), the backup board (W7); the top app bar over every wide page, Home and the step flows included (§3); the rest specified, not built (F92)

Open: components that keep their size, part 2 (L5: the size set and the type roles are built in `faraday-ui`; the screens of `faraday-core` still write some sizes in place, a count `tools/lint-tokens.counts` holds and that only falls); planned.

## 1. What changes

Today a screen is laid out one of two ways (DESIGN §3): the small
panel, one column with Home as a menu, or the wide layout, the
240-unit sidebar beside a page designed at 1280 × 800 and scaled to
fit the window (F78). Three things follow from that, and this spec
removes them:

- **Everything between is the small panel stretched.** A 7-inch panel
  (about 960 × 580 dp) or a half-screen window (about 760 dp wide) gets
  the Pi's layout: Home's six menu tiles become boxes 470 dp wide with
  an icon in one corner.
- **A large monitor gets a scaled-up 1280 layout**, not more room. A
  1920 × 1080 window shows the same content as 1280 × 800, larger.
- **Components stretch.** Home's half-width tiles are about 500 dp
  wide at 1280 and over 600 at 1600; buttons in a full-width row take
  the row.

## 10. The design calls (2026-10-23)

The owner asked for Material 3 and left these calls to the spec:

1. **Five classes, not three.** Material 3 adds large and extra-large
   for monitors; the owner asked that a full monitor get what Material
   gives it. A monitor gets the expanded rail and more panes, not a
   larger copy of the 1280 layout.
2. **The Pi keeps its Home menu.** Material's bar is 80 dp, about a
   quarter of the panel's 358 dp height, and Faraday has more places
   than a bar holds. On a short window the menu is one press from every
   page through the back chevron, and the panel's step flows already
   take the whole screen. This is Material's guidance for a short
   window (no bar where height is scarce) applied with Faraday's own
   hub.
3. **A bar on compact windows that are not short**, with five
   destinations, Material's most; Learn, Settings and Lock go in the top
   app bar, where Material puts actions that are not destinations.
4. **The rail from medium up to expanded, and on low windows.** The
   7-inch panel is the reason: it is wide but low, and the rail costs
   no height.
5. **The expanded rail from large up is today's sidebar.** It is what
   Material calls an expanded navigation rail (formerly a standard
   drawer); it keeps the flows, keys, wallets and sticks it lists, which
   a collapsed rail cannot. Its look changes to Material's destinations.
6. **List and detail from expanded up** for every screen that is a list
   and the one chosen: the wider window shows both instead of a wider
   list.
7. **Learn as a supporting pane only on extra-large**, where there is
   room for the page and Learn side by side at full width.
8. **No scaling to fit.** Material lays out at the screen's density;
   F78's fixed 1280 × 800 page goes, and with it F78's cost of smaller
   text on scale-2 screens.
9. **Nothing stretches** (owner, 2026-10-23, and F91 for the Starter).

## 12. Plan

| Batch | What | Model |
|---|---|---|
| L1 | The classes and density (§2, §7): `SizeClass` with the five widths and two heights in place of `is_compact`; the fit to 1280 × 800 and F78's floor removed; the tokens (§9) | Opus |
| L2 | Navigation (§3): the bar, the rail and the top app bar as `faraday-ui` components; the sidebar drawn as the expanded rail; the status strip; DESIGN §4.1 | Opus |
| L3 | Home (§4): the lead tile and the tile grid | Sonnet |
| L4 | List and detail (§5) for Wallets, Files, Vaults and Backups, and Settings (§17); built | Opus |
| L5 | Components keep their size (§8): buttons, fields, the measure, across the screens; the size set (`BUTTON_H`, `FIELD_H` outlined, `LIST_1`–`LIST_3`) and the type roles in place of the fifteen sizes, by file owner, after the Material 3 components (`OUTLINE`, Tonal, the field box) are in `faraday-ui`; the `lint-tokens` counts fall | Sonnet |
| L6 | The supporting pane (§6); built | Sonnet |
| L7 | The tours at the new sizes, the tests (§11), DESIGN §3 rewritten to this spec, then this plan to `docs/history/` and this file a Reference; built but for the move, which waits on the rest of the Open line | Sonnet |

L1 before the rest; L2 before L3 and L4. Each batch is timed in a
late-flow session (`docs/SPEED.md` §7) where it adds drawing work, and
the frame budgets stay as they are. The text size work (F85,
`docs/TEXT-SIZE.md`) builds on L1's classes and goes after it.

## 13. Wide windows: what is wasted today (2026-10-24)

The owner, from Faraday Demo on Make codex32 shares' dice entry
(2026-10-24): wide empty space left of the flow and right of the Demo
only card; the checksum worksheet scrolls although the window has room
for it whole; the card and the top strip's pills stand mid-screen, not
at the window's edge; the pad is at the page's foot, below the tree, the
tracks and the worksheet, so the pad and the rolls it fills are never in
view together; the string's boxes are small; the step-through's buttons
move as its line wraps. The owner's screenshot matches the 2560 × 1440
tour's frame (`just faraday-snap 2560x1440 roller`): the window lays out
at about 2560 dp wide. The audit rendered the default tour and codex32,
roller, board, walletflow, spend, vaultshow, glance, inbox and guide at
1280 × 800, 1920 × 1080 and 2560 × 1440.

**One cause for most of it.** The wide frame caps the navigation and
the page together at 1600 dp and centres the page in the rest
(`MAX_W`, `faraday/faraday-core/src/screens/mod.rs`): at 1920 the page
is 1360 dp with 160 dp empty each side, at 2560 1360 dp with 480 dp
each side. Everything placed from the page's right edge (the mode pill,
the ?, the eye, the keyboard button, Home's Scan button, the supporting
pane) is that far in from the window's edge. At 1280 × 800 the cap does
not bind and nothing is wasted.

| Screen | At 1920 and 2560 | Content | Rule |
|---|---|---|---|
| Make codex32 shares, the entry | page centred, 480 dp empty each side at 2560; the card column capped at 900; the worksheet in a view at most 400 dp tall, panning, the sheet 1392 × 1052 dp; the Demo only card 506 dp in from the edge; the pad at the page's foot, its keys stretched to the column (136 dp at 2560) | wide: the worksheets, the dice tracks and tree; the string's boxes; the pad | §14, §15, §16 |
| Make codex32 shares, the review tabs and the Strings card | the same cap; each worksheet in its view; the Codex32QR copy grid in the 900 column | wide: worksheets, the copy grid | §14, §16.1, §17 |
| Home | the tiles in three columns in the 1360 page, the rest empty; Scan 480 dp in from the edge | a grid of tiles (feed) | §17 |
| Back up a wallet, the board | the places in one column, the board's lists beside them, the right of the window empty | wide: the board | §17 |
| Wallets with the chosen wallet; the structure chart | list and detail inside the 1360 page; the chart small in a wide detail, its buttons far below it | list and detail; the chart (wide) | §17, L4 |
| Files | the two lists, 490 dp each, in the 1360 page; "Waiting for a stick" mid-screen | lists | §17, L4 |
| Vaults; Wallets' own list | one card or two columns of cards at the page's top left, the rest empty | lists | §17, L4 |
| Explore | the path field stretched to 1350 dp; long values on one line | fields, mono values, a table | §8, §17 |
| Settings | one 720 dp column of cards, off centre of the window, the pills far from it | reading | §14, §17 |
| Spend, the backup flow, New key, Create a vault (step flows) | one column centred in the 1360 page; the pills mid-screen | reading; Spend's transaction table | §14, §15 |
| Learn | the sheet centred over the window | reading | L6 |

## 18. How this fits the plan already made

- **L1** (built) gives the classes the rules read. **W1** (§15) is the
  page area every later batch measures from, so it lands before L4 and
  L6.
- **L4** (list and detail) builds its panes in W1's whole page area;
  Files, Vaults, Wallets and Backups take §17's rows there, and no W
  batch builds a list pane of its own.
- **L5** (components keep their size) owns the buttons, fields and keys
  across the screens; W3 brings in `PAD_KEY_W` for the dice pads it
  moves, and L5 holds the other pads and the keyboards to it.
- **L6** (Learn in the supporting pane) uses the pane's place built for
  the Demo only card (`codex32/pane_screen.rs`), made the one place for
  any supporting pane by W1; on the codex32 entry one pane stands at a
  time (§19).
- **L7** (tours and tests) adds 2560x1440 and 2000x1090 to the tours
  beside §11's sizes; built.

## 19. Plan for wide windows

| Batch | What | Model | Estimate |
|---|---|---|---|
| W1 | The page area to the window's edge (§15): `MAX_W` goes; the top strip's row, the eye, the keyboard button, Scan and the supporting pane against the window's right margin; reading pages centred in the room, pages with wide content at the start (§14); the supporting pane's place one function for the Demo only card and Learn | Sonnet | half a day |
| W2 | The codex32 worksheet's fit (§16.1) and the string's boxes (§16.2): the fitted cell for the checksum, translation and review sheets, the dice sheet's tree beside its tracks, the copy grid in the work width; tokens `CODEX32_CELL_FIT_MIN`, `CODEX32_CELL_FIT_MAX`, `CODEX32_STRING_BOX_MIN`, `CODEX32_STRING_BOX_MAX` | Opus | a day |
| W3 | The input panel (§16.3) for the codex32 entry, then New key's dice and Create a vault's dice; the rolls moved into it; the typed characters' keyboard; tokens `CODEX32_ENTRY_W`, `PAD_KEY_W`, `INPUT_PANEL_MAX` | Opus | a day and a half |
| W4 | The follow (§16.4): the test of a drag then an entry, at once; the page following a whole sheet, and `follow_front` on the fitted cell, with W2 | Sonnet | half a day |
| W5 | The step-through's buttons fixed (§16.5), in the card and the strip | Sonnet | a quarter of a day |
| W6 | Home's feed in the work width, Explore's fields and table, the chart's buttons, tables at their columns' width (§17) | Sonnet | a day |
| W7 | The backup board's places in columns and its lists a supporting pane (§17) | Opus | a day and a half |

Order: W1, W2, W3, then W4 (its first test may land at once), W5 at any
time; W6 and W7 after L4. Each batch that adds work while drawing is
timed in a late-flow session (`docs/SPEED.md` §7); the fitted cell and
the panel's place are arithmetic on the window's size. Every new number
is a token (`tools/lint-tokens.sh`).

**Tests** (user-facing, each at the sizes named):

- At 1920 × 1080 and 2560 × 1440 the Demo only card's right edge and
  the ?'s are 24 dp from the window's right edge.
- At 1920 × 1080 a 48-character share's checksum worksheet is drawn
  whole: every cell from box 1 to the final row has its press region
  once the page is scrolled to it, and a drag on the sheet moves
  nothing; at 1280 × 800 it is in a view that a drag pans.
- At 2560 × 1440 the same sheet is whole at a larger cell than at 1920.
- The string's boxes: at 1920 × 1080 a 48-character share shows boxes
  1 to 24 in its first row and 25 to 48 in its second; a 74-character
  share's last row holds 73 and 74.
- At 1920 × 1080 with d6, a press on the pad's 3 puts a 3 on the rolls,
  and the pad and the rolls are both on screen; after the page is
  scrolled to the worksheet's foot, both still are. At 2560 × 1440 the
  pad stands beside the rolls and stays where it is as the page
  scrolls; at 1280 × 800 the pad is at the window's foot.
- At 480 × 640 at 286 dpi the entry is as today (the Pi's context over
  the pad).
- The view dragged away from the front cell, then a value entered: the
  view moves back to the front over several frames, every one part of
  the way, and under Reduce motion it is there at once. Where the sheet
  is whole, the page scrolls to the front cell the same way.
- The Demo only card's Next has the same rectangle at every step of a
  character and across characters, in the card and in the strip.
- Home at 2560 × 1440: Scan stands 24 dp from the window's right and
  foot. The board at 2560 × 1440 shows three places side by side.

**Answered by the owner (2026-10-24):**

- The rolls move from the dice sheet into the input panel (W3).
- A reading page sits centred in the room beside the navigation, not at
  the start (§14, built in W1).
- Settings becomes list and detail from expanded windows up, built
  with L4 (§5, §17); replaced by the owner's call of 2026-10-25: every
  section on one page from medium up, the list on compact windows only
  (§17).
- Learn, opened on the codex32 entry on an extra-large window, stays a
  sheet; the Demo only card keeps the supporting pane, so one pane
  stands at a time (§6, L6).

**Built in W1:** the frame no longer caps the page; the top strip's
row, the eye, the keyboard button, the supporting pane and Home's Scan
stand `MARGIN_WIDE` from the window's right edge (Scan, drawn by the frame, also from the foot);
`screens::support_pane_place` is the one place any supporting pane
stands (the Demo only card uses it, and so do Learn's pane and the
board's lists). Past an extra-large window's 1360 dp of room (`page_area`,
`faraday-core/src/screens/mod.rs`), a reading page is laid out in its
own area (`READING_PAGE_W`, or Settings', Set the time's and Add
randomness' own) centred in the room; New key takes the whole room;
Home and every other page keeps the 1360 dp area (`PAGE_AREA_UNTIL_W`)
centred, as before, until its batch (L4, W6, W7) gives it the work
width. At 1280 × 800 and every window under 1600 dp wide the page is
where it was; only the ?, the eye and Scan moved in to the 24 dp
margin. The Starter's frame already runs to the window's edge with its
pages centred (`docs/STARTER.md` §3.1d), and its top bar's ⋮ stands at
the edge.

**Built in L4:** from expanded windows up, beside the rail and the
expanded rail alike, Wallets, Vaults and Backups (and Settings, until
2026-10-25, §17) show their
list in a pane `LIST_PANE` wide at the start and the one chosen beside
it to the right margin, `SPACER` between, each scrolling on its own
(`panes/`, `Slot::ListPane`); the list's items are Material's list
items, the chosen one in the selected state, and nothing chosen shows
the first (Backups' first wallet, Settings' first section, the first
vault; Wallets shows the wallet last chosen or loaded). The detail's
reading parts keep `COLUMN_MAX`; the wallet's chart takes the detail's
width. Choosing a locked vault shows its Unlock beside the list; an
open vault's Contents, a list and detail of their own, take the page
(Material's nested list and detail shows the deepest two panes).
Settings' sections were the compact window's cards, one at a time;
since 2026-10-25 every card is on one page from medium up (§17). Files'
From the stick and For the stick are two list panes side by side, what
stands above them at their width, and its stick status a chip in the top
strip's row; on the rail's windows the two lists stand side by side
where the area holds both, under the stick's row: From the stick
scrolls with the page, and For the stick stands in a pane of its own
beside it, its top following the head as the page scrolls it away and
then holding at the page's top, its list scrolling on its own under its
own scrollbar (`screens/files.rs`, `files_beside`). Compact and medium windows are as before.

**Built in W6:** Home takes the work width: its title, lead and tiles
start beside the navigation and the tiles run as many columns as fit to
`MARGIN_WIDE` from the window's edge, each at the row's share and never
under `TILE_MIN`, a short row leaving the rest empty. Explore is reading
content in its own area (`READING_EXPLORE_W`), centred in the room: its
column `COLUMN_MAX`, its path field as wide as the path
(`PATH_FIELD_W` at the least, up to the column), the address, public
key and extended key in groups of four wrapping between groups, and its
Addresses and Find pages in the same column. A wallet's chart is drawn
as wide as its widest row of nodes at `CHART_NODE_MAX` needs
(`glance::natural_width`), never under the 720 dp column nor past the
detail: a small chart stands over the reading parts, a wide one runs on
towards the card's end; the wallet's buttons (and a silent payments
wallet's) stand under the body, after the first receive address, and
are pinned at the card's foot, the body scrolling above them, only where
the body is taller than the card. Spend's transaction table is as wide
as its columns (`TABLE_LABEL_W`, the longest outpoint or address,
`TABLE_AMOUNT_W`), never wider than the step; Decode a transaction's inputs and outputs likewise, each amount just past the longest outpoint or address (`DECODE_VALUE_ROOM`), never wider than its column. On the small panel and the
rail's windows nothing changed; Explore's values group only on the wide
layout. The pages W6 left at the old 1360 dp area past an
extra-large window's room have their areas since: the pages below and
the backup flow ("Built after W7").

**Built since W6:** Tools, Transfer and a stick visit are
reading content, each laid out in its own area centred in the room past
an extra-large window's 1360 dp (`READING_TOOLS_W`,
`READING_TRANSFER_W`, `READING_VISIT_W`), a stick
visit's two lists each growing to `COLUMN_MAX`; the tools' catalogue
takes the work width, as many tiles to a row as `CATALOG_TILE_MIN`
leaves room for and never fewer than three, so four stand side by side
at 1920 dp and five at 2560 dp. At 1280 × 800 and every window up to
1600 dp wide they are as before (§17).

**Built since W6 (2026-10-26):** Wallets' first page and the TOTP codes
are reading content in `READING_PAGE_W`, centred in the room; an open
vault's contents are list and detail in the whole room from the
navigation (`Area::Work`): its kinds and items at the start, the item
beside them, its reading parts at `COLUMN_MAX` and a wallet's chart on
to the window's edge, Lock and Rename ending left of the top row's
pills and ? (§15). Checked at 1920 × 1080 and 2560 × 1600.

**Built in W7:** while the backup board is shown (Back up's plan, a
wallet flow's plan) on a window whose room is wider than the old page
area, the page takes the work width from the navigation
(`screens::board_work_end`): the cards keep a reading column, their
body `COLUMN_MAX`, and the board's places run on to the supporting
pane's spacer in as many columns as fit at `BOARD_PLACE_W` (480 dp),
each at the row's share, two at 1920 dp and three at 2560 dp; the
places in a line of columns are as tall as the tallest, their sticks
and Paper with them; **+ place** and the software place take the next
cells; the checks under the board keep the column. The board's lists
(seeds, the description, vaults, watch-only software) stand in the
supporting pane (`support_pane_place`), an outlined card as tall as the
list, scrolling on its own, beside the "Demo only" card where that
stands too. A drag drops into any column, and onto the pane to take a
chip off, as before. At 1280 × 800, every window up to 1600 dp wide and
the small panel the board is as before.

**Built after W7:** Back up and a wallet's backup flow are pages that
hold wide content whether the board's card is open or not, so on a
window whose room is wider than the old page area they start at the
navigation on every card (`screens::backup_work_end`): their reading
column, a card's body `COLUMN_MAX`, stands at the same place and width
with the board open or closed, and opening the board's card adds its
columns and its supporting pane beside the column without moving
anything; the checklist, the plan's other cards and the flow's other
steps stand in that column too. With the board closed, Back up's map
panel stands at the window's edge, as it does at 1280 × 800, and a vault
made in the flow has its form at the flow's start. While the board's card is
open its rules, the one above it and the one under it, run to the work
width's end (`flow::column_wide`); its head and summary stay in the
column (§14). At 1280 × 800, every window up to 1600 dp wide and the
small panel nothing moved.
