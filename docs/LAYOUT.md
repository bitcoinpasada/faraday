# Layout at every window size

**Kind:** Spec
**Status:** Partly built: size classes and density (L1, F101), navigation (L2), Home's lead tile and tile grid (L3), the supporting pane's place and its narrow strip, for codex32's Demo only card (§6); the page area to the window's edge (W1, §15, §19); list and detail for Wallets, Files, Vaults and Backups (L4, §5, §17); Settings' list of sections on compact windows and every section on one page from medium up (§17); the codex32 entry's input panel (W3, §16.3); the rest specified, not built (F92); the rest of wide windows (§13 to §19) specified, not built
**Open:** the stick's density from its screen's EDID (§7; today its kernel command line's, F101), the top app bar over the wide pages and the step flows (§3; they keep their own heads), Files' two lists each scrolling on its own on the rail's windows (§17; there the page scrolls as one), components that keep their size (L5), Learn in the supporting pane (L6), tours and tests (L7), wide windows: wide content, the codex32 worksheet's fit, the string's boxes, the input panel on compact windows that are not short (§16.3; they keep the small panel's context over the pad), the panel beside the sheets scrolling within itself (it stands at the foot where it is taller than the window), New key's dice and Create a vault's Generate with dice in the panel (their pad, progress line and entries already stand together in one card), the page following a whole sheet, fixed step buttons, the open card's rule at the work width while it holds wide content (§14), and the pages that keep the old 1360 dp area (Wallets' first page, Tools, the tools' catalogue, Codes, a stick visit, an open vault's contents, Transfer, Upgrade, the backup flow's cards but the board; §19) and Decode's outputs at their columns' width (§14 to §17, W2, W4, W5); planned.

Faraday lays its screens out by Material 3's layout and navigation
rules at every size, from the Pi's 2.8-inch panel to a full monitor.
Only the rules are Material's: the look stays Faraday's own
(`faraday-ui`, DESIGN's tokens, components and themes). The owner asked
for Material 3 and left the design calls to this spec (2026-10-23);
§10 records each call and why. The Starter follows the same rules
(`docs/STARTER.md` §3.1d, F91). This spec replaces DESIGN §3's two
layouts once built.

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

## 2. Size classes

A window's class is read from its size in dp at the screen's density
(§7), divided by the text size factor (F85), so a larger text steps the
layout down as a smaller window would.

| Class | Width | Typical screen |
|---|---|---|
| Compact | under 600 dp | the Pi panel (268 × 358), a narrow window, a 7-inch panel in portrait (580 × 960) |
| Medium | 600 to 839 dp | a half-screen window, a 10-inch panel in portrait (800 × 1280) |
| Expanded | 840 to 1199 dp | a 7-inch panel in landscape (960 × 580), a small laptop window |
| Large | 1200 to 1599 dp | the desktop app's default window (1280 × 800), a 10-inch panel in landscape |
| Extra-large | 1600 dp and over | a full monitor at 1×: 1920 × 1080, 2560 × 1440 |

Height matters where it is short. Under 480 dp tall is a **short**
window (the Pi panel), and under 600 dp tall a **low** one (the 7-inch
panel): §3 says what each changes.

Every class lays the screen out again at its own width. Nothing is
scaled to fill a window.

## 3. Navigation

Faraday's places: **Home, Wallets, Files, Vaults, Tools** (the
destinations a person moves between), **Learn** and **Settings** (opened,
then closed), **Lock**, and two that appear while they apply: **Stick
visit** (a stick attached and nothing held) and **Transfer** (the
desktop app).

| Class | Navigation | Learn, Settings, Lock |
|---|---|---|
| Compact, short (the Pi) | Home is the menu, a button for each place, as today; every other page has the 44-unit bar with its back chevron | on Home's menu; Lock and the power button at its foot |
| Compact | the navigation bar at the foot, 80 dp: Home, Wallets, Files, Vaults, Tools | the top app bar: the ? (Learn at the page's section), the ⋮ menu (Settings), Lock |
| Medium; Expanded | the navigation rail at the left, 80 dp: the five destinations, then Learn and Settings under a divider, Lock at its foot | on the rail |
| Expanded and low (the 7-inch panel) | the rail, as above | on the rail |
| Large; Extra-large | the expanded rail at the left, 240 dp: today's sidebar, its tabs drawn as Material's destinations (icon and label in a row, the current one on an accent pill), its sections under caption headings | on the expanded rail |

**Stick visit and Transfer** join the destinations while they apply:
on the rail and the expanded rail after Tools, on the bar in place of
Tools (Tools stays on Home's grid).

**What the sidebar lists** besides its tabs (flows in progress with
their open step, the loaded keys, the wallets, the session strip, the
sticks attached) stays on the expanded rail. Where there is no expanded
rail, it is **Home's status strip** (§4): the same rows, as chips at the
top of Home, each opening what its sidebar row opens. A flow in
progress also shows its chip at the top of every destination while it
waits, so it is never more than one press away.

**The top app bar** (every class but compact short), 64 dp across the
page: the back arrow where the page has a way back (F77), the page's
title, and at the right end its actions: the mode pill off mainnet, the
keyboard button where typing goes, the ? where the page has a Learn
page, and the ⋮ menu where the page has more. It replaces the wide
layout's back link, title row and top-right row (DESIGN §4.1). It stays
while the page scrolls under it.

**Faraday Demo's sticks** stand outside the app, in its host strip
at the window's foot (owner, 2026-10-25; F118, `docs/DESKTOP-HOST.md`),
with Samples, the preview size and the screen's practice aids: the app's
bar holds what the device's does, the mode pill and the ?. Held bare
(`--panel`), the app draws the pills in a strip above its bar, as the
stick and the Pi do while a backup flow uses sticks.

**A flow** (Make a new wallet, the backup flow, Add a key, a spend) keeps
the navigation beside it, so a person can leave and come back to it
(its chip says where); a step that must not be left (writing a stick,
a key being made) hides the navigation for that step, as DESIGN's
Cancel already rules (F51).

## 4. Home

Home is Material's feed layout: the lead tile while something is under
way or waiting (`docs/FLOWS.md`, Home), then the tiles in a grid.

- **The status strip** (compact, medium, expanded): the sidebar's rows
  as chips, at the top (§3).
- **The lead tile**: its icon, its line and its one button at its end,
  as wide as that needs and at most `LEAD_MAX` (560 dp).
- **The tiles**: each its icon beside its name and one line under the
  name, at least `TILE_MIN` (200 dp) wide. The grid takes as many
  columns as fit; a row with fewer tiles than columns leaves the rest
  empty. Today's tiles keep their order (`docs/FLOWS.md`, Home).
- **Scan** stays the floating button at the bottom-right.

## 5. Panes

Material's canonical layouts, by what a screen shows:

| Screen | Compact, Medium | Expanded and up |
|---|---|---|
| Wallets, Files, Vaults, Backups (a list, and the one chosen) | the list; choosing one opens its page | **list and detail**: the list in a pane `LIST_PANE` (360 dp) wide, the chosen one's page beside it; nothing chosen shows the first |
| Settings | compact: the list of its sections, each opening as its own page; medium: every section on one page, one column | every section on one page, two columns where they fit (§17) |
| Home | §4 | §4 |
| A flow's step, a form, a sheet's page | one column, at most `COLUMN_MAX` (720 dp), centred in the space beside the navigation | the same; on extra-large, the step's Guided text and Learn in the supporting pane (§6) |
| The backup board, the structure chart, Explore | the whole space beside the navigation | the same |
| A sheet | the window's width less a margin (compact), centred at most `SHEET_MAX` (560 dp) wide otherwise | the same |

Panes are separated by a 24 dp spacer and each has its own scroll.

## 6. The supporting pane

On extra-large, Learn opens as a pane at the right, `SUPPORT_PANE`
(400 dp) wide, beside the page it explains, instead of the Learn
sheet over it; it follows the page's section as the person moves on,
and its close button puts it away. Below extra-large, Learn stays the
sheet it is today.

A supporting pane holds its place (owner, 2026-10-24): it does not
scroll with the page, which scrolls beside it, and a pane taller than
the window scrolls within itself. Its top is the page content's, below
the top-right strip of the network pill and the eye, never level with
them (`SUPPORT_PANE_TOP`), against the right edge with the 24 dp margin
(`MARGIN_WIDE`) and the 24 dp spacer from the page (`SPACER`). On a
window too narrow for the page beside it, what the person acts on in
it sits in a strip pinned at the window's foot that opens to the whole
pane over the page and never scrolls away. Built for Make codex32
shares' dice sheet in its column (§16.1, `codex32/pane_screen.rs`);
Learn's pane is planned. The "Demo only" card that stood here is
Faraday Demo's host strip now, outside the app (F118,
`docs/DESKTOP-HOST.md`).

## 7. Density

A window is laid out at the screen's density: the desktop app at the
compositor's scale, the stick at the density its display reports
(EDID's physical size, 1 dp a pixel where it reports none), the Pi
panel at its 286 dpi. The wide layout's fit of 1280 × 800 to the
window, and F78's floor, go: a scale-2 laptop shows a large layout at
the compositor's scale, and a half-screen window on it a medium one,
each at the text size the person's system asks for.

## 8. Components keep their size

- **Buttons** are as wide as their label and padding. A row of them
  sits at the start of its column. Only on compact may a card's or a
  sheet's buttons take its width, one under another where they do not
  fit side by side.
- **Nothing is cut off at the sides**, down to the Pi's panel (268 dp
  wide): a line and the button beside it put the button under the
  line where the two do not fit; a button whose label names more than
  the column holds (Sign with three fingerprints) has that label as a
  line above it and a short label of its own (Sign); a name and its
  source beside a step's Continue wrap in the room left of it. Text
  wraps or is shortened with an ellipsis; it never runs past its card,
  its sheet or the panel.
- **Cards and tiles** sit in grids of columns with a minimum width,
  never stretched beyond the grid's column.
- **Text** runs at most about 70 characters a line (`MEASURE`, at the
  body size); a column wider than that keeps its text to the measure.
- **Fields** are as wide as what they take: a word box, an amount, a
  path, a passphrase each have their width; a descriptor or an address
  takes the column.
- **Margins and spacers** are Material's: 16 dp on compact, 24 dp
  otherwise, on the 4 dp grid.
- **One size set** (owner, 2026-10-23, from the Material 3 review):
  - every button 40 dp tall (`BUTTON_H`), its label 14;
  - every text field 56 dp tall (`FIELD_H`), outlined: a 1 dp
    `OUTLINE` edge, 2 dp `ACCENT` when focused, 2 dp `ERR` with its
    error line under it, the label kept outside. Whether the desktop
    is drawn denser (48) is decided in L5 and recorded;
  - list rows 56, 72 or 88 dp by one, two or three lines (`LIST_1`,
    `LIST_2`, `LIST_3`), the headline 16 and the supporting line 14.
- **Type** is Material's roles, by size: `HEADLINE_M` 28, `HEADLINE_S`
  24, `TITLE_L` 22, `TITLE_M` 16, `TITLE_S` 14, `BODY_L` 16, `BODY_M`
  14, `BODY_S` 12, `LABEL_L` 14, `LABEL_M` 12, `LABEL_S` 11. They
  replace the fifteen sizes the screens write today, each moved to its
  nearest role by use (13 to 12 or 14, 15 to 14 or 16, 17 to 16, 26 to
  24 or 28, 10 to 11). A page's title is `HEADLINE_M`, a step flow's
  heading one of 28 or 24 for every flow, a sheet's head 24 (22 on the
  small panel), and `TITLE` 20 is retired. Each role carries its line
  height. The typeface stays Noto (F91, F92).

## 9. Tokens

In `core/faraday-draw/src/tokens.rs`: the class widths
(`WINDOW_MEDIUM` 600, `WINDOW_EXPANDED` 840, `WINDOW_LARGE` 1200,
`WINDOW_EXTRA_LARGE` 1600) and heights (`WINDOW_SHORT` 480,
`WINDOW_LOW` 600); `RAIL_W` 80, `RAIL_EXPANDED_W` 240 (today's sidebar
width), `NAV_BAR_H` 80, `APP_BAR_H` 64, `NAV_PILL_W` 56, `NAV_PILL_H`
32; `MARGIN_COMPACT` 16, `MARGIN_WIDE` 24, `SPACER` 24; `TILE_MIN`
200, `LEAD_MAX` 560, `LIST_PANE` 360, `COLUMN_MAX` 720, `SHEET_MAX`
560, `SUPPORT_PANE` 400, `SUPPORT_PANE_TOP` 96 (below the top-right
strip's two 48 dp rows), `MEASURE` 70 characters. The Starter's
(`docs/STARTER.md` §3.1d) are the same names where they are the same
thing.

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

## 11. Tests

User-facing, each at the sizes that show it: a 268 × 358 panel shows
Home's menu and no bar; a 400 × 800 window shows the bar's five
destinations and each opens its screen; 760 × 1000 and 960 × 580 show
the rail with Learn, Settings and Lock; 1280 × 800 and 1920 × 1080 show
the expanded rail with a flow in progress listed; Home's tiles never
grow past their column and a row of two at 1920 leaves columns empty;
Wallets at 1280 shows the list and the chosen wallet together, and at
760 the list alone; a flow in progress shows its chip on every
destination; at 1920 × 1080 Learn opens beside the page; a scale-2
window of 1440 × 900 pixels lays out as medium at 720 dp; Larger text
at 1280 lays out as medium. On the Pi's 480 × 640 panel at 286 dpi,
every screen (empty, with a wallet and its keys, online and in Practice),
the stateless sheets, the scanners, every step of a spend and the
sample spend draw no control and no text past a side of the panel or
of the card or sheet it is in, at every place the page scrolls to
(`faraday-core` `pi_panel`, `faraday-samples` `pi_panel`).

The snapshot tours gain the sizes 760x1000, 960x580@133, 1920x1080 and
2560x1440 beside today's.

## 12. Plan

| Batch | What | Model |
|---|---|---|
| L1 | The classes and density (§2, §7): `SizeClass` with the five widths and two heights in place of `is_compact`; the fit to 1280 × 800 and F78's floor removed; the tokens (§9) | Opus |
| L2 | Navigation (§3): the bar, the rail and the top app bar as `faraday-ui` components; the sidebar drawn as the expanded rail; the status strip; DESIGN §4.1 | Opus |
| L3 | Home (§4): the lead tile and the tile grid | Sonnet |
| L4 | List and detail (§5) for Wallets, Files, Vaults and Backups, and Settings (§17); built | Opus |
| L5 | Components keep their size (§8): buttons, fields, the measure, across the screens; the size set (`BUTTON_H`, `FIELD_H` outlined, `LIST_1`–`LIST_3`) and the type roles in place of the fifteen sizes, by file owner, after the Material 3 components (`OUTLINE`, Tonal, the field box) are in `faraday-ui`; the `lint-tokens` counts fall | Sonnet |
| L6 | The supporting pane (§6) | Sonnet |
| L7 | The tours at the new sizes, the tests (§11), DESIGN §3 rewritten to this spec, then this plan to `docs/history/` and this file a Reference | Sonnet |

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

## 14. Reading width and wide content

The owner's call, recorded as F114 (`docs/decisions/F114.md`).

A page holds two kinds of content, and each has its own width:

- **Reading content** (text, fields, buttons, a flow's card heads and
  rows, lists of rows): at most `COLUMN_MAX` (720 dp), its text at
  most `MEASURE` (§8). Material keeps body text to a readable line
  length and gives a larger window more panes, not longer lines
  (Material 3, Layout: canonical layouts and window size classes;
  Android's adaptive guidance, "Canonical layouts" and "Use window size
  classes", developer.android.com).
- **Wide content** (a worksheet, the dice tracks and tree, the backup
  board, the structure chart, the Codex32QR copy grid, a table, a grid
  of tiles): the **work width**, the whole room between the navigation
  and the supporting pane, less `MARGIN_WIDE` at the start and `SPACER`
  before the pane (or `MARGIN_WIDE` at the window's edge where no pane
  stands). Wide content never stretches what it draws (§8): it uses the
  room to show more of itself at once, or its cells larger up to a
  token maximum (§16.1).

A page of reading content only is centred in the room beside the
navigation (left of a supporting pane where one stands). A page that
holds wide content starts at the navigation's edge plus its margin: its
reading parts keep their width at the start, and its wide parts run on
to the work width's end. While a flow's open card holds wide content,
the card's rule runs the work width; its head and summary stay where the
column puts them. On one-column windows (the rail's, §3) a page sits in a
`COLUMN_MAX` column centred beside the rail, except Make codex32 shares,
whose string, sheets and input panel take the whole area beside the rail
(owner, 2026-10-25; `screens/mod.rs`, `whole_area`).

## 15. The right edge

The page area is the window less the navigation, at every size: the
1600 dp cap goes. What stands at the right stands against the window's
edge:

- the top strip's row (the mode pill and the ?; §3) and, under it, the eye and the keyboard
  button: their right end `MARGIN_WIDE` (24 dp) from
  the window's right edge, as Material's top app bar puts its trailing
  actions;
- a page's own head controls level with that row (a flow's Cancel and
  its Steps only and Guided where no back link stands above the
  heading, Files' stick chip) end a gap before the row; those below its
  foot (under a back link, the stick visit's stick chips) keep the
  page's width;
- a supporting pane (§6): its right edge `MARGIN_WIDE` from the
  window's, its top `SUPPORT_PANE_TOP`, `SPACER` from the page;
- Home's Scan button: `MARGIN_WIDE` from the window's right and foot
  (Material's floating action button margin on medium windows and up);
- toasts and notices: centred on the page area, as today.

Material's pane scaffold keeps a 24 dp spacer between panes from
expanded windows up, allows up to three panes side by side from large
widths, and gives a pane 360 dp by default (412 dp from large up)
(Compose Material 3 adaptive, `PaneScaffoldDirective`). Faraday keeps
`SUPPORT_PANE` at 400 dp. At 1280 × 800 nothing moves: the page area
already ends at the window's edge.

The Starter's held frame keeps its own rule (`docs/STARTER.md` §3.1d).

## 16. Make codex32 shares on a wide window

Where this section names the "Demo only" card, its column or its strip,
read Faraday Demo's host strip (F118, `docs/DESKTOP-HOST.md` §3.1):
the card is gone from the app, and the entry and its sheets take the
room it took.

### 16.1 The checksum worksheet fits where the window allows

A worksheet's cell is chosen from the room: the largest size between
`CODEX32_CELL_FIT_MIN` (20 dp, the size compact windows draw today) and
`CODEX32_CELL_FIT_MAX` (32 dp) at which the whole sheet fits the work
width (beside the input panel where it stands beside, §16.3) and the
view's height, the window's height less the input panel at the foot and
`MARGIN_WIDE` above and below. The characters scale with the cell (16 at
28 dp, as today: four sevenths of the cell). Where it fits, the sheet is
drawn whole: no view, no pan, a drag on it does nothing, and the page
scrolls to it (§16.4). Where it does not, the worksheet is as today: the
class's cell (`codex32_cell`), a view as wide as the work width and at
most `codex32_sheet_view` tall (on extra-large, as tall as the view's
height above, up to the sheet's), panning both ways by drag, wheel and
keys (`docs/CODEX32.md` §7.5).

A 48-character sheet at cell c is 48c + 48 dp wide and 35c + 72 tall
(35 rows, 48 columns, the booklet's gaps). With the Demo only card
beside the page and d6 dice:

| Window (dp) | Work width | The input panel | The worksheet |
|---|---|---|---|
| 1280 × 800 | 750 (the card's narrow column) | at the foot, 132 dp | does not fit: as today, cell 28 in a 400 dp view, panning |
| 1920 × 1080 | 1134 | at the foot, 132 dp | whole at cell 22: 1104 × 842 |
| 2000 × 1090 | 1214 | at the foot, 132 dp | whole at cell 23: 1152 × 877 |
| 2560 × 1440 | 1774 (1326 beside the panel) | beside, 424 dp | whole at cell 26: 1296 × 982 |

A 74-character sheet (61 rows, 74 columns: 74c + 76 by 61c + 124)
fits from about 3840 × 2160 at one dp a pixel; below that it pans as
today. The cell is a function of the window's size in dp and the
sheet's shape alone, so drawing and the follow (§16.4) take the same
one; it is arithmetic, worked out where the sheet is laid out. The
translation worksheets and the review's tabs (`docs/CODEX32.md` §6.5)
take the same rule; a tab's sheet that fits shows no scroll of its own.

**Side by side.** The dice sheet's tree and tracks stand side by side
wherever the work width holds the tracks at full size and the tree at
`CODEX32_TREE_MIN_W` (300 dp) or more, the tree growing to
`CODEX32_TREE_MAX_W` (owner, 2026-10-25), the rolls having moved to the
input panel (§16.3). A track runs only as far as the dice's faces: d6
shows pads 1 to 6 and no "Die pads (d7+)" group, d8 up to 8, d10 up to
10, d12 up to 12, d20 all twenty, the booklet's places otherwise kept;
changing the dice shows or hides the pads past 6. Tracks and tree stand
side by side from a 566 dp work width for six-faced dice and from 776 dp
for twenty-faced ones (for d6 from 960 × 580 up, in Testnet and in Practice, where the "Demo only" card narrows to its 118 dp column (small buttons, 12 dp card padding), 8 dp from the window's edge, on 1280 × 800 and smaller so they do, the work width running up to `SPACER` before it, `docs/CODEX32.md` §5.5). The dice sheet stands only with the
Checksum tab of the share being rolled, never with a derived share's
tab or the Secret's (owner, 2026-10-24). Its head row, "Dice
de-biasing" with Material's expand chevron, is one 48 dp press that
opens it to its tracks and tree or closes it; it closes by itself when
a share's rolling is done, keeping the last tracks for a look, and
each roll opens it again. **The dice sheet's column** (owner,
2026-10-24): where the checksum worksheet still draws whole beside it
and the column fits the window's height above the input panel, the
dice sheet stands in a column at the right, in the supporting pane's
place, the tree over the tracks, with the Demo only card under it; the
column scrolls within itself where text drawn larger passes it. Its
height is measured, `CODEX32_DICE_COLUMN_H` (720 dp) and the card's
`CODEX32_COLUMN_CARD_H` (480 dp): in Testnet the column stands from
1920 × 1080, in Practice from 2560 × 1440. Below that both sheets stand in
the page, the sheet of the work in hand first so the page scrolls the
least to it: the dice sheet while a character is rolled, the worksheet
while the step-through works the checksum. The
Codex32QR copy grid (`copy_grid`,
`faraday/faraday-core/src/screens/parts.rs`) already sizes its cells
from its width up to `C32_GRID_MAX` and stands its zoom beside the grid
where the width holds both: given the work width in place of the
column's, it gains the same way.

### 16.2 The string's boxes

The share's string over the sheets ("Share A · character 7 of 26", box
1 to 48, each numbered) runs **24 boxes a row**, six groups of four: 48
characters are two even rows, 74 three rows of 24 and one of 2, 127
five of 24 and one of 7. The box is as large as the room gives, between
`CODEX32_STRING_BOX_MIN` (24 dp) and `CODEX32_STRING_BOX_MAX` (40 dp);
its character at 0.6 of the box and its number at 0.4 of the box,
never under `TEXT_MIN`. Where 24 do not fit at the least size, a row
takes the first of 16, 12, 8 or 4 that does. Compact windows take as
many whole groups of four as fit at `codex32_box` (the Pi: two groups,
8 a row). The groups of four start at box 1 and the numbers are the
booklet's (`docs/CODEX32.md` §3, §7.5). The row of 24 is Faraday's own:
the booklet's worksheets bring the string down two boxes at a time
under a first row of 16, and neither they nor secretcodex32.com set a
row length for writing a string out; the cards print it in groups of
four. At the sizes above: 16 a row at 27 dp (1280), 24 a row at 40 dp
(1920 and up).

### 16.3 The input panel: the pad and the rolls in one view

What the person presses and what it fills stand together. The **input
panel** holds: the rolls of the character being entered (each die's
name over its marker, the first roll, beside its die, the second, as
the tracks draw them, and "lower · 0" or "higher · 1" under them, the
five dice in one row; the latest equal pair struck under the row; moved
here from the dice sheet, which keeps the tree and the tracks, so the
rolls are drawn once), the step tracker's **Working** area
(`docs/CODEX32.md` §5.7), always beside the pad, and the pad: "Your
dice" at the start of its first row (under it the face waiting for its
units digit, or ten-faced dice's "0 is 10"), then a key per face and
Backspace, each `PAD_KEY_W` (48 dp) square, `GAP_SMALL` apart, no
padding of their own (§8), in one row where it fits, else in as few
even rows as fit; six-faced dice keep their keys in one row, larger
dice take a second row only where it makes the panel shorter. A face is
one key, so nothing waits from the pad; digits typed on a computer's
keyboard settle on the next key or Enter. The dice's size is not in the
panel (owner, 2026-10-25): it is chosen with the dice's colours on the
Entropy card, Material's segmented button of d4 to d20, d6 the default,
at every size, and the panel follows it; to change it the person goes
back to that card. The Demo only card says what the computer rolls.
Once every character is in, **Continue** stands in the
pad's place, the flow's one way on; the page has no Continue of its own
while the panel stands out of it (owner, 2026-10-24). At the foot the
panel takes the shortest of the arrangements the width allows (owner,
2026-10-25: as few rows as the width allows, no empty rows): the rolls,
the pad and the Working area in one row, where the Working area keeps
`CODEX32_WORKING_SIDE_MIN` after the pad, the latest struck pair under
the rolls or under the pad's keys; the rolls beside the Working area,
the pad under both; the rolls beside the Working area over the pad; the
Working area under the rolls beside the pad; or the three one under
another; where the pad's last row has room, the latest struck pair and
the Working area's Previous and Next stand at its end, so the row above
holds only the rolls and two lines of working. Two more take the
Working area as one paragraph, "Working", where the step stands and its
working run on from one line, wrapping under it only where they do not
fit, in as many lines as the longest step needs at that width
(`CODEX32_WORKING_TEXT_MAX` and `CODEX32_WORKING_HEAD_W`): the pad over
the paragraph beside the rolls, Previous and Next at the end of the
pad's row; and the rolls and the pad on one row, the struck pair at the
keys' end or under them, over the paragraph across the panel with
Previous and Next at its end (owner, 2026-10-25). "Your dice" is left
out where the keys of dice under ten faces fit one row only without it.
One row of all three takes about 1 220 dp inside the panel's margins
for six-faced dice (the rolls 372, the pad 440, the Working area 360
and two spacers), so it stands from 1920 × 1080 up. The panel is, for
six faces and for twenty: 132 and 216 dp on a 960 × 580 window (144
and 220 before), 108 and 196 at 1280 × 800 (132 and 196), 108 and 160
at 1366 × 768 (132 and 176), 88 and 116 at 1920 × 1080 (108 and 144).
The pad's key of the roll
the Working area shows stands pressed: Material's pressed state layer in
the accent and an accent outline, a stronger layer over it as the roll is
shown easing away over `PRESS_EMPHASIS_MS` (none under Reduce motion),
whether the person pressed it or the Practice walk's Next or Play rolled it,
so the walk reads as a person pressing. It stands:

- **Beside**, where the work width holds `CODEX32_ENTRY_W` (424 dp),
  `SPACER` and the worksheet at `CODEX32_CELL_FIT_MIN` (1456 dp in
  all; 2560 × 1440): a column at the work area's start that holds its
  place as the page scrolls beside it, its top at `SUPPORT_PANE_TOP`,
  scrolling within itself where it is taller than the window, as the
  supporting pane does (§6). The sheets stand beside it.
- **At the foot** everywhere else from medium up, and on compact
  windows that are not short: pinned at the window's foot across the
  page area (beside the rail or the expanded rail, above the bar),
  surface-container-low with a rule along its top, as Material's
  bottom-anchored input (the on-screen keyboard) stands; its parts in
  the shortest of the arrangements above. The page leaves room under its last row so it
  scrolls clear of it, as it does for the keyboard. The supporting pane
  stands above it (its foot `SPACER` over the panel's top), and so does
  the Demo only strip where the card does not fit beside (§6).
- **In the page**, as today, where the panel would take more than
  `INPUT_PANEL_MAX` (40 %) of the window's height (a d20's pad on a low
  window).
- **On the Pi panel** (compact, short) as today: the tiles, one context
  panel and the pad under it, all on screen (`docs/CODEX32.md` §6.4.7).

Characters typed (`docs/CODEX32.md` §5.2) put the codex32 keyboard in
the panel's place by the same rule; a computer's own keyboard types at
every size. New key's dice for BIP-39 words and Create a vault's
**Generate with dice** (`docs/CODEX32.md` §5.6) take the same panel:
their pad, their entries' latest group and the progress line in one
view.

### 16.4 The view follows the work

The view moves only where the work would otherwise be off screen, and
never while a character is being rolled (owner, 2026-10-25):

- **A roll moves nothing.** While a character's rolls are entered (a
  face on the pad, a digit, Backspace), nothing scrolls or pans: the
  die tracks, the tree and the Working area change in place, the pad
  stays put, and the dice sheet keeps its layout (the tree's line
  naming the leaf and, where the computer's dice are offered, the line
  naming the dice it rolled keep their room while empty).
- **A character in its box glides once.** When a character goes into
  its box (its last roll, a character typed, Use a sample), the view
  goes to that box with the timed move of `docs/CODEX32.md` §7.5
  (`motion::SHEET_FOLLOW_MS`, Material's standard curve; at once under
  Reduce motion), however the person last dragged it, and not at all
  where the box is on screen already; the page moves first only where
  too little of the worksheet's view stands above the input panel to
  hold it (`Faraday::worksheet_follow_calm`, `codex32::Follow`).
- **The Working area's steps.** Its Previous and Next move the view only
  where the cells the step marks would otherwise be off screen.
- **The Demo walk.** Its Previous, Next, arrow keys and Play send the
  view to the cell being worked as before
  (`Faraday::worksheet_follow`).

Only the small panel (`docs/CODEX32.md` §6.4.7), whose pad stands in
the page, keeps the page at the entry's foot at every press. Besides:

- **The page follows a whole sheet.** Where the worksheet is drawn
  whole (§16.1) it has no view to move, so the page's own scroll makes
  the same least move, over the same time on the same curve, to bring
  the cell being worked into the room above the input panel.
- **The dice follow too.** On the step-through's rolls and its
  character's step, as its Previous, Next or Play shows them (not as the
  person's own face shows one), the page brings the marked die's track, or the
  tree, into its room above the input panel by the least move, over
  the same time on the same curve, at once under Reduce motion, once
  the frame after the step has drawn where it is; in the dice sheet's
  column nothing moves.
- **Panned across, a scrollbar along the foot.** While the worksheet's
  view pans sideways its overlay scrollbar runs along the view's foot
  as well as down its right edge, as Material's horizontal scrollbar
  does, and fades with it.
- **The top strip's actions keep a surface.** On every wide page the
  mode pill, the ? and the eye keep the surface under them, from the
  left of the row to the window's edge and `SUPPORT_PANE_TOP` down
  (`screens::top_actions_clear`, one rule for every page), so a page
  scrolls under them as under Material's top app bar.
- **The dice sheet glides.** Its head row's chevron, a roll and a
  share's last roll open and close it by a reveal of its height over
  `MOTION_MEDIUM2` on the standard easing, at once under Reduce motion.
- **One cell size.** `Codex32Make::follow_front` places by
  `Geom::of(class)`; it takes the fitted cell (§16.1), so the follow
  and the drawing agree.

### 16.5 The step-through's buttons stay put

Previous, Next and Play stand in Faraday Demo's host strip (F118,
`docs/DESKTOP-HOST.md` §3.1), each at one place whatever the step says,
Play and Pause sharing the wider's width. The step's count and its
working ("Step 14 of 19 · character 3", then the working) are the
Working area's, beside the pad (§16.3; owner, 2026-10-25). Nothing
is measured to place them.

## 17. The other screens

| Screen | Rule |
|---|---|
| Home | the feed (§4) across the work width: as many `TILE_MIN` columns as fit, tiles at their size, the rest of a short row empty; Scan at the right edge (§15) |
| Wallets, Files, Vaults, Backups | list and detail (§5, L4) in the whole page area: the list `LIST_PANE` at the start, the detail beside it to the right margin; the detail's reading parts at `COLUMN_MAX`, its chart and tables in the detail's width. Files' two lists ("From the stick", "For the stick") are two list panes side by side; its stick status chip moves to the top strip's row |
| The structure chart (a wallet's detail, the chart sheet) | wide content in the detail's width; its nodes keep their size (§8); the wallet's buttons under the chart, not at the card's foot |
| The backup board | wide content: the places laid out in as many columns as the work width holds at the board's place width, so three places stand side by side at 2560 and two at 1920; the board's lists (seeds, vaults, watch-only software) a supporting pane at the right edge (§15) |
| Explore | reading content: the path field as wide as a path (§8), the values in mono in the column, wrapping in groups; the addresses table as wide as its columns, at the start |
| Spend and every step flow | reading content in the column, centred (§14); a table in a step (Spend's transaction) as wide as its columns; on extra-large the step's Guided text and Learn in the supporting pane (§6, L6) |
| Settings | (owner, 2026-10-25) On a compact window, the Pi panel and narrow windows, Material's list and detail one pane at a time: the Steps only · Guided choice, then the sections as list items, Mode first with the mode it is on, opening the mode sheet; any other opens that section's card as its own page, the top app bar naming it, and Back (the bar's and the app's) returns to the list where it was scrolled. From medium windows up, no list: every section's card on one page under the Steps only · Guided choice, centred in the page area, in two columns where each is at least `SETTINGS_CARD_MIN` (360 dp) wide, else one; each column at most `COLUMN_MAX` (§14). The first column holds Mode, Session, Time and Randomness, the second Appearance on, so the order reads down the first and then the second and a card never moves between them. Medium keeps one page: measured at 160 dpi with no keys loaded, the page is 2,235 dp tall at 600 × 900 (2.7 screens; 2,787 dp, 3.3 screens, with Guided), 2,039 dp at 768 × 1024 (2.1; 2.6 with Guided) and 1,981 dp at 800 × 1280 (1.6; 2.0), at or under about three screens but with Guided on the smallest, which a list would only split into a press and a Back per section |
| Learn | the sheet below extra-large; the supporting pane on extra-large (§6, L6), at the right edge (§15), except on the codex32 entry, where it stays a sheet and the Demo only card keeps the pane (owner, 2026-10-24) |
| The Starter | the same rules (`docs/STARTER.md` §3.1d): its page area is the window less the rail, its top bar's actions at the window's edge, a destination's cards in a column of at most `CONTENT_MAX` and a task page's text in `COL_W`, centred in the room beside the rail; nothing is capped by the window |
| Sheets and dialogs | as §5: `SHEET_MAX`, centred |
| The Pi panel and compact windows | unchanged by §13 to §17, but for the input panel on compact windows that are not short (§16.3) |

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
  beside §11's sizes.

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
stands (the Demo only card uses it; L6's pane and the board's lists
will). Past an extra-large window's 1360 dp of room (`page_area`,
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
where the area holds both, under the stick's row, the page scrolling
as one. Compact and medium windows are as before.

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
`TABLE_AMOUNT_W`), never wider than the step. On the small panel and the
rail's windows nothing changed; Explore's values group only on the wide
layout. Left at the old 1360 dp area, centred, past an extra-large
window's room: Wallets' first page, Tools, the catalogue, Codes, a stick
visit, an open vault's contents, Transfer, Upgrade and the backup flow (step flows and reading
pages, for a batch to give each its reading area).

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
chip off, as before. The page re-lays from its centred area to the
work width when the board's card opens, since the plan's other cards
keep the old area. At 1280 × 800, every window up to 1600 dp wide and
the small panel the board is as before.
