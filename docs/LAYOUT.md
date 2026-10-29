# Layout at every window size

**Kind:** Reference
**Status:** Built: the five size classes and density (F101), the navigation bar, rail and expanded rail, and the top app bar over every page on every class but compact short, Home and the step flows included (§3); Home's lead tile and tile grid (§4); list and detail (§5, §17); the supporting pane, Learn in it on extra-large windows (§6); the page area to the window's edge (§15); reading width and wide content (§14); Make codex32 shares on a wide window (§16); the tours at every class's size and §11's tests. Components keep their size (§8): the size set and the type roles are in `faraday-ui`; the screens of `faraday-core` still write some sizes in place, a count `tools/lint-tokens.counts` holds and that only falls (`tools/lint-tokens.sh`): enforcement, not open work.

Faraday lays its screens out by Material 3's layout and navigation
rules at every size, from the Pi's 2.8-inch panel to a full monitor.
Only the rules are Material's: the look stays Faraday's own
(`faraday-ui`, DESIGN's tokens, components and themes). The owner asked
for Material 3 and left the design calls to this spec (2026-10-23);
`docs/history/LAYOUT-LOG.md` §10 records each call and why. The Starter follows the same rules
(`docs/STARTER.md` §3.1d, F91). DESIGN §3 states these rules in
short.

## 1. What changes

Moved to `docs/history/LAYOUT-LOG.md`.

## 2. Size classes

A window's class is read from its size in dp at the screen's density
(§7), divided by the text size factor (F85), so a larger text steps the
layout down as a smaller window would.

| Class | Width | Typical screen |
|---|---|---|
| Compact | under 600 dp | the Pi panel (268 × 358), a narrow window, a 7-inch panel in portrait (565 × 964 at its 170 dpi) |
| Medium | 600 to 839 dp | a half-screen window |
| Expanded | 840 to 1199 dp | a 7-inch panel in landscape (964 × 565), a 10-inch panel in portrait (853 × 1365 at its 150 dpi), a small laptop window |
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
title with where the arrow goes under it, "Back to Tools" (F146), and
at the right end its actions: the mode pill off mainnet, the
keyboard button where typing goes, the ? where the page has a Learn
page, and the ⋮ menu where the page has more. It replaces the wide
layout's back link, title row and top-right row (DESIGN §4.1). It stays
while the page scrolls under it.

**Beside the expanded rail** every page has the top app bar across the room from the rail to the
window's edge (the side sheet's room left out), drawn after the page,
which is clipped under its foot and scrolls under it; the bar takes the
surface-container colour while the page is scrolled. The page hands its
name and its way back to the bar as it is drawn (`screens::bar_head`),
the same way back its back link took; a page with none, a destination,
shows its name alone. The mode pill, the keyboard button and the ? stand
at the bar's right end, Files' stick chip left of them; what stood beside
a page's title (Files' Scan and its in-memory pill, Vaults' Create a
vault, a vault's state, Rename and Lock, the stick visit's chips, Tools'
Find) leads the page under the bar. List and detail panes start under
it (`PANE_TOP_BAR`), every other page `PAGE_TOP_BAR` under it. Over a
step flow the bar carries the flow's name and way back, Cancel and the
codex32 eye, as the one-column layouts' bar does; the flow's column keeps
its chip and Steps only · Guided at its head. Home's bar reads Home; the
wordmark stays at the head of the expanded rail. Supporting panes start
level with the page's first line under the bar.

**The name on Home's bar** (owner, 2026-10-25): the mark, then the
wordmark where it ends before the bar's actions. Faraday Demo's name,
FARADAY DEMO, stands on one line where it fits; where it does not
(Narrow's 400 dp, a phone-wide window, the Pi panel previewed with
Practice · Testnet beside it) DEMO stands under FARADAY at 0.7 of its
height, 0.3 of it below, both from the same left edge, the pair centred
where the line would be (`Ui::wordmark_stacked`, `WORDMARK_STACK_WORD`,
`WORDMARK_STACK_GAP`), made smaller where FARADAY needs it down to
capitals 8 dp tall (`WORDMARK_CAP_MIN`); under
that the mark stands alone. The device's name, FARADAY alone, stands on
one line or not at all, as before (`Ui::wordmark_fitted`).

**Faraday Demo's sticks** stand outside the app, in its host strip
at the window's foot (owner, 2026-10-25; F118, `docs/DESKTOP-HOST.md`),
with Samples, the preview size and the screen's practice aids: the app's
bar holds what the device's does, the mode pill and the ?. Held bare
(`--panel`), the app draws the pills as the stick and the Pi do while
a backup flow uses sticks: beside the expanded rail in the top app bar,
in one row left of its actions, in the longest form that fits beside
the room kept for the page's name (`APP_BAR_PILLS_TITLE_ROOM`), so the page under the bar stays where it is as they come and go
(`stick::pills::bar`); on the one-column layouts in a strip above the
bar or beside the rail.

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
sheet over it; it follows the page's steps as the person moves on,
closes when the person moves to another screen unless it is pinned,
and its close button puts it away. Below extra-large, Learn stays the
sheet it is today.

A supporting pane holds its place (owner, 2026-10-24): it does not
scroll with the page, which scrolls beside it, and a pane taller than
the window scrolls within itself. Its top is the page content's first
line under the top app bar (`SUPPORT_PANE_TOP`), against the right edge with the 24 dp margin
(`MARGIN_WIDE`) and the 24 dp spacer from the page (`SPACER`). On a
window too narrow for the page beside it, what the person acts on in
it sits in a strip pinned at the window's foot that opens to the whole
pane over the page and never scrolls away. Built for Make codex32
shares' dice sheet in its column (§16.1, `codex32/pane_screen.rs`)
and for Learn (`screens/learn_pane.rs`). The "Demo only" card that stood here is
Faraday Demo's host strip now, outside the app (F118,
`docs/DESKTOP-HOST.md`).

Learn's pane, from 1600 dp: the ? and Learn in the sidebar, and the
bar's Cryptography, open it beside the page, which is laid out in the
room left of it, so nothing of the page is under it and the page takes
presses as before; nothing else opens it, on any screen or size (owner,
2026-10-29). The pane
is a card in the surface-container role, Material's large shape. Its
header row, as Material's side sheet has, holds the headline **Learn**
in Title L and, at its end, the pin and the close button, each
Material's standard icon button in a 48 dp target
(`LEARN_PANE_CLOSE_TOP`); the pin is a toggle, the push pin outlined
in on-surface-variant while off ("Keep open") and filled in primary
while on ("Unpin"), sized to read as large as the × beside it
(`LEARN_PANE_PIN_GLYPH`), its button at the end of its target nearest
the close button's, the two targets meeting without overlapping
(owner, 2026-10-29). The tabs stand under it (`LEARN_PANE_TABS_TOP`)
across the pane's whole width, a single divider under them, so
each label shows whole (Material 3 Tabs; Where keys are kept's tab
reads Key storage). Where a screen's labels together are wider than
the row, it shows whole tabs only, the one on show among them sharing
the row, and a chevron (Material's standard icon button in a 48 dp
target) at an end with tabs past it, whose press shows the next tab
that way (`faraday-ui`'s tabs). Its tabs and ticked steps are the sheet's, and
on the Spend tab alone **Spending, step by step** stands above them;
the page's title is in Title L and its text in Body M. The close button, the
? pressed again, Escape and the mouse's back button put it away, and
take its pin off. When the person moves to another screen the pane
closes, unless it is pinned (owner, 2026-10-29); pinned, it stays open
as the person moves on until it is closed or unpinned. The pin is kept
for the session in memory only: a lock or a power-off clears it. When
the step changes, or a pinned pane's screen, the pane shows that
screen's Learn pages, on the tab it showed where they have it, else on
the first.
One pane stands at a time: the codex32 entry (its dice sheet's
column), Back up and a wallet's backup flow (the board's lists) keep
the place for their own, and Learn is the sheet there; a pane open on
arriving there is put away. A window resized across 1600 dp moves
Learn between the pane and the sheet, on the page it shows; the sheet
has no pin, so a pane that comes back from it is unpinned.

## 7. Density

A window is laid out at the screen's density: the desktop app at the
compositor's scale, the stick on a screen of 11 inches or less at the
density its EDID gives (the pixels over the inches of its diagonal,
`docs/ORIENTATION.md` §2.1) and on any other screen at the kernel
command line's `faraday.dpi` (160, doubled from 2,800 pixels wide,
F101), the Pi panel at its 286 dpi. The wide layout's fit of 1280 × 800 to the
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
  its sheet or the panel, and no line is drawn over another. A view
  that pans sideways (a codex32 worksheet) is scrolled there, not cut.
  The small panel's bar holds Back, the page's name, Cancel, the eye,
  the mode pill and the ?: the name is shortened, or left off where
  they leave it no room, and the ? where they would reach Back's target
  (DESIGN §4.1). Where the page offers a cryptographic action, **How
  Faraday does it** stands left of the ? and goes into the ⋮ first,
  before the ?, its row the menu's first (not built;
  `docs/CRYPTO-LEARN.md` §8.3). The test is `tests/all/pi_panel.rs`: on the stick's
  Pi (480 × 640 at 286 dpi) and in Faraday Demo's Pi preview, on
  Mainnet, Testnet and in Practice, it walks every screen and sheet
  (listed by a match that fails to build when one is added), each step
  card, tab and section a page offers, and the flows' steps (New key
  with BIP-39, SLIP-39 and Make codex32 shares from New key and from
  Make a new wallet, every kind of Make a new wallet and Restore, a
  spend, the Spend tab, a vault's contents, the backup flow of each
  kind at each step, every Tools tile), scrolled top to foot, and fails
  on a control or a line past the sides of its clip, a line drawn over
  another, or two touch targets of the bar that overlap.
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
    outline edge, 2 dp primary when focused, 2 dp error with its
    error line under it, the label kept outside. The desktop is not
    drawn denser: a field is 56 on every window, Material's default
    density, since Material keeps its denser steps for a dense layout
    used with a pointer alone, and the same screens are touched on the
    Pi and the stick;
  - list rows 56, 72 or 88 dp by one, two or three lines (`LIST_1`,
    `LIST_2`, `LIST_3`), the headline 16 and the supporting line 14.

  Built (L5, part 1): the sizes are tokens in
  `core/faraday-draw/src/tokens.rs` (`BUTTON_H`, `BUTTON_PAD`,
  `FIELD_H`, `FIELD_EDGE`, `FIELD_EDGE_ACTIVE`, `FIELD_PAD`,
  `FIELD_GAP`, `LIST_1` to `LIST_3`), and `faraday-ui` draws them:
  `Ui::button` labels every button in `LABEL_L`; `Ui::field_label` and
  `Ui::field_box` are the outlined field, its label above it and its
  error line under it (`Ui::field_height` measures it); `Ui::list_item`
  is the one-, two- or three-line list item, by how much its
  supporting text takes (`faraday_ui::roles::ListItem`). The
  multi-choice list's rows are list items, `parts::button_rows` and the
  sheets' and dialogs' buttons are `BUTTON_H`. A key of the on-screen
  keyboards and pads keeps its own label (`key_label`).

  Built (L5, part 2, the shell of `faraday-core`): Home, Wallets and
  its start, Restore, Settings, the sidebar, the compact screens and
  their top bar, the step flows' head and cards (`flow.rs`), the Files
  menu, the in-progress and visit-taken sheets, the menus, Glance, the
  Modes sheet, Explore, the word lists and the list pane's item write
  their text by role and their buttons at `BUTTON_H`; the segmented
  buttons are `SEGMENTED_H` with `LABEL_L`. A step flow's heading is
  `HEADLINE_S` in its medium face on every flow; a sheet's head is
  `sheet_head`. The compact bar is Material's small top app bar, `APP_BAR_H`
  (64 dp) on every compact window and on the Pi's panel: the page's name
  in `TITLE_L` (or `TITLE_M`, below) over the place Back goes in `LABEL_M`, its actions each in
  a 48 dp target and Cancel Material's text button; where Title L would be
  cut beside the actions the name is Title M, shortened with an ellipsis
  only where that is cut too (DESIGN §4.1), and
  the actions that would reach the back arrow's target go into
  Material's overflow menu, a ⋮ at the bar's end: the ?, then the
  keyboard button, then the codex32 eye (DESIGN §4.1). The text fields that were boxes filled and edged in
  place (Explore's path, index and address, the word list's search, the
  rename fields, the passphrase field of the small panel, Glance's
  field) are `Ui::field_box`, 56 dp, their text in `BODY_L`; the list
  pane's item is `Ui::list_item`.

  Built (L5, part 2): Files and the stick visit, Spend, the sheets (the
  scanner, the dialogs, Learn), Decode, the entry forms, the tool
  catalogue, Check a message, Sign a message, Set the time, Add
  randomness, Transfer and Tools: every line of text is a
  `TypeRole` (`text_as`, `wrap_as` and the rest), every button
  `BUTTON_H` tall, the Kind rows' names `TITLE_S` and their second lines
  `BODY_S`; a monospaced value is its role `.mono()`.

  Built (L5, part 2): Family, the seeds, Backups, Wipe, Vanity, Codes,
  Boot import, Lightning, the plan's cards, BIP 85, Silent payments and
  the roller's five-dice screen draw by role and `BUTTON_H`.

  Built (L5, part 3): the key generator, the codex32 screens (work,
  dice, tour, tracker, worksheet, backup and correction), Create and
  the Board draw by role and `BUTTON_H`. The worksheet's and the string
  boxes' characters, and the cell notes, keep the sizes their cell
  geometry gives. The vault and backup flow screens still write their
  own sizes.

  Built (L5, part 3): Faraday Demo's host strip, its menus and Samples
  sheet, the samples' own sheet, and Faraday Starter's pages draw by
  role: the strip's controls and a menu's rows `LABEL_L`, its section
  head and a menu's sub-heading `TITLE_S`; the Starter's section labels
  and card headings `TITLE_S`, its lines and value rows `BODY_M`, step
  heads `TITLE_M`, every button `BUTTON_H`. The Starter's rows that
  open or choose something (the checklist, a row that opens a page, the
  write modes, a choice) are list items: `LIST_2` with a headline in
  `BODY_L` over its line in `BODY_M`, `LIST_1` with the headline alone,
  a value at the right in `LABEL_S`.
- **Type** is Material's roles, by size: `HEADLINE_M` 28, `HEADLINE_S`
  24, `TITLE_L` 22, `TITLE_M` 16, `TITLE_S` 14, `BODY_L` 16, `BODY_M`
  14, `BODY_S` 12, `LABEL_L` 14, `LABEL_M` 12, `LABEL_S` 11. They
  replace the fifteen sizes the screens write today, each moved to its
  nearest role by use (13 to 12 or 14, 15 to 14 or 16, 17 to 16, 26 to
  24 or 28, 10 to 11). A page's title is `HEADLINE_M`, a step flow's
  heading one of 28 or 24 for every flow, a sheet's head 24 (22 on the
  small panel), and `TITLE` 20 is retired. Each role carries its line
  height. The typeface stays Noto (F91, F92).

  Built (L5, part 1): the roles are `TypeRole` values in
  `core/faraday-draw/src/tokens.rs`, each its size, its line height
  and its face (`TypeFace`): Material's regular is Noto Sans, its
  medium Noto Sans SemiBold, the heavier text face Faraday has;
  `.mono()` sets a role in Noto Sans Mono for data, and `.medium()` and
  `.regular()` change its face. `faraday-ui` draws text by role
  (`faraday/faraday-ui/src/roles.rs`): `Ui::text_as`, `text_mid_as`,
  `text_right_as`, `measure_as`, `fit_as`, `wrap_as`, `wrap_lines_as`,
  `wrap_height_as` and `line_as`, each the method of the same name at
  the role's size and face, so `ui.text(x, y, 12.0, W::R, ink, s)`
  becomes `ui.text_as(x, y, BODY_S, ink, s)`; `line_as` is the role's
  line height and `wrap_as` spaces its lines by it. A page's title
  (`parts::title`) is `HEADLINE_M`, a section's label `TITLE_S`, a
  sheet's or dialog's head `tokens::sheet_head`, the top app bar's
  title `TITLE_L`, the navigation labels `LABEL_M` (the rail's rows
  `LABEL_L`), the snackbar and a rich tooltip `BODY_M`, a plain
  tooltip `BODY_S`.

## 9. Tokens

In `core/faraday-draw/src/tokens.rs`: the class widths
(`WINDOW_MEDIUM` 600, `WINDOW_EXPANDED` 840, `WINDOW_LARGE` 1200,
`WINDOW_EXTRA_LARGE` 1600) and heights (`WINDOW_SHORT` 480,
`WINDOW_LOW` 600); `RAIL_W` 80, `RAIL_EXPANDED_W` 240 (today's sidebar
width), `NAV_BAR_H` 80, `APP_BAR_H` 64, `NAV_PILL_W` 56, `NAV_PILL_H`
32; `MARGIN_COMPACT` 16, `MARGIN_WIDE` 24, `SPACER` 24; `TILE_MIN`
200, `LEAD_MAX` 560, `LIST_PANE` 360, `COLUMN_MAX` 720, `SHEET_MAX`
560, `SUPPORT_PANE` 400, `SUPPORT_PANE_TOP` 96 (below the top-right
strip's two 48 dp rows), `MEASURE` 70 characters; the size set and
the type roles of §8 (`BUTTON_H`, `FIELD_H`, `LIST_1` to `LIST_3`,
`HEADLINE_M` to `LABEL_S`). The Starter's
(`docs/STARTER.md` §3.1d) are the same names where they are the same
thing.

## 10. The design calls (2026-10-23)

Moved to `docs/history/LAYOUT-LOG.md`.

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
at 1280 lays out a class down, at 914 × 571 dp, with the rail. On the Pi's 480 × 640 panel at 286 dpi,
every screen (empty, with a wallet and its keys, online and in Practice),
the stateless sheets, the scanners, every step of a spend and the
sample spend draw no control and no text past a side of the panel or
of the card or sheet it is in, at every place the page scrolls to
(`faraday-core` `pi_panel`, `faraday-samples` `pi_panel`).

Built (L7): `navigation`, `window_class`, `home` and `list_detail` in
`faraday-core`'s tests hold each of these; a short row of Home's tiles
at 1920 keeps its column's width (`home`), Wallets at 1280 shows the
list and the chosen wallet together and at 760 the list alone
(`list_detail`).

The snapshot tours (`just faraday-snapshots`) take a window of every
class: 1280x800, 1920x1080, 2000x1090, 2560x1440 and 3840x2160,
1024x768, the half-screen 760x1000, the Pi's 480x640 at 286 dpi and
240x320 at 143, and the stick's 7-inch and 10-inch panels at both
orientations (1024x600 and 600x1024 at 170 dpi, which stand for
960x580@133, 1280x800 and 800x1280 at 150), each again at Larger.

## 12. Plan

Moved to `docs/history/LAYOUT-LOG.md`.

## 13. Wide windows: what is wasted today (2026-10-24)

Moved to `docs/history/LAYOUT-LOG.md`.

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
to the work width's end. A page whose card can hold wide content starts
there on every card, the wide card open or not, so nothing moves when it
opens (Back up and a wallet's backup flow, whose plan holds the backup
board). While a flow's open card holds wide content, the card's rules,
above and under it, run the work width; its head and summary stay where
the column puts them. On one-column windows (the rail's, §3) a page sits in a
`COLUMN_MAX` column centred beside the rail, except Make codex32 shares,
whose string, sheets and input panel take the whole area beside the rail
(owner, 2026-10-25; `screens/mod.rs`, `whole_area`).

## 15. The right edge

The page area is the window less the navigation, at every size: the
1600 dp cap goes. What stands at the right stands against the window's
edge:

- the top app bar's actions (§3): the ?, the mode pill, the keyboard,
  the codex32 eye and a flow's Cancel, each in its 48 dp target from
  the bar's end, as Material's top app bar puts its trailing actions;
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
  supporting pane does (§6). The sheets stand beside it. Built: it stands
  beside wherever the work width holds it, however low the window, and
  where it is taller than the window it is as tall as the window less
  the margin and scrolls on its own under the pointer, the page beside
  it keeping its place (`Slot::InputPanel`); its rolls' heads fit their
  columns at a larger text.
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
  window, a phone-wide compact one), with the small panel's tiles and
  context over the pad on a compact window.
- **On the Pi panel** (compact, short) as today: the tiles, one context
  panel and the pad under it, all on screen (`docs/CODEX32.md` §6.4.7).

Characters typed (`docs/CODEX32.md` §5.2) put the codex32 keyboard in
the panel's place by the same rule; a computer's own keyboard types at
every size. New key's dice for BIP-39 words and Create a vault's
**Generate with dice** (`docs/CODEX32.md` §5.6) take the same panel:
their pad, their entries' latest group and the progress line in one
view.

Built for both, at the foot by the same rule (not on the Pi panel, and
in the card where the panel would take more than `INPUT_PANEL_MAX` of
the window's height): New key's Entries card, while dice are pressed one
at a time, stands its count of rolls with the roll just made at its end,
the bar under it and the pad of six faces (`DICE_PAD_KEY_H` tall) in
the panel, the words so far, the check and the card's buttons in the
page above it (`keygen/dice_panel.rs`); Generate with dice stands the
rolls' field, the strength line and a pad of the six faces and
Backspace in the panel, the list's choice, the words and Use these
words in the card (`vault/dice_screen.rs`). While the on-screen keyboard
is up for the rolls' field, its keys are the pad and the panel stands
in the card. The page scrolls clear of the panel. Dice typed as one
string keep their field in the card.

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
| The structure chart (a wallet's detail, the chart sheet) | wide content in the detail's width; its nodes keep their size (§8); the wallet's buttons under the chart, not at the card's foot. A node's sheet is Material's standard side sheet at the window's right edge, `SIDE_SHEET_W` wide, the page's room narrowed by it so the chart and its highlighted lines stand beside it (DESIGN §4.16), and where the room left cannot hold the list pane and a card at least as wide beside it (`panes::screen::fits`), the wallet's card stands alone in it, as on a medium window, until the sheet closes; on the rail's and the bar's windows the centred sheet or the panel's page, as before |
| Back on a compact window | closes what is open on the page before it leaves it: the search view of a vault's items or of the codes (the bar's back arrow, naming the list, and the system Back close it as Done and Escape do, the query kept and the list filtered), then a Settings section to the list |
| Tools | reading content in its own area (`READING_TOOLS_W`), centred: its `TOOLS_COLUMN_W` column after its inset |
| The tools' catalogue | a grid of tiles, wide content: as many columns as `CATALOG_TILE_MIN` leaves room for in the work width, never fewer than `CATALOG_COLUMNS_MIN`, each at the row's share |
| Transfer, a stick visit | reading content, each in its own area centred in the room: Transfer's Send and Receive side by side, `TRANSFER_W` at most (`READING_TRANSFER_W`); a stick visit's two lists, each at most `COLUMN_MAX`, `VISIT_GAP` apart (`READING_VISIT_W`); each between two `PAGE_INSET`s |
| The backup board | wide content: the places laid out in as many columns as the work width holds at the board's place width, so three places stand side by side at 2560 and two at 1920; the board's lists (seeds, vaults, watch-only software) a supporting pane at the right edge (§15); the page it is on (Back up, a wallet's backup flow) starts at the navigation on every card, so its cards stand still when the board's card opens, and the open card's rules run the work width (§14) |
| Explore | reading content: the path field as wide as a path (§8), the values in mono in the column, wrapping in groups; the addresses table as wide as its columns, at the start |
| Spend and every step flow | reading content in the column, centred (§14); a table in a step (Spend's transaction) as wide as its columns; on extra-large the step's Guided text and Learn in the supporting pane (§6, L6) |
| Settings | (owner, 2026-10-25) On a compact window, the Pi panel and narrow windows, Material's list and detail one pane at a time: the Steps only · Guided choice, then the sections as list items, Mode first with the mode it is on, opening the mode sheet; any other opens that section's card as its own page, the top app bar naming it, and Back (the bar's and the app's) returns to the list where it was scrolled. From medium windows up, no list: every section's card on one page under the Steps only · Guided choice, centred in the page area, in two columns where each is at least `SETTINGS_CARD_MIN` (360 dp) wide, else one; each column at most `COLUMN_MAX` (§14). The first column holds Mode, Session, Time and Randomness, the second Appearance on, so the order reads down the first and then the second and a card never moves between them. Medium keeps one page: measured at 160 dpi with no keys loaded, the page is 2,235 dp tall at 600 × 900 (2.7 screens; 2,787 dp, 3.3 screens, with Guided), 2,039 dp at 768 × 1024 (2.1; 2.6 with Guided) and 1,981 dp at 800 × 1280 (1.6; 2.0), at or under about three screens but with Guided on the smallest, which a list would only split into a press and a Back per section |
| Learn | the sheet below extra-large; the supporting pane on extra-large (§6, L6), at the right edge (§15), except on the codex32 entry, where it stays a sheet and the Demo only card keeps the pane (owner, 2026-10-24), and on Back up and a wallet's backup flow, whose board's lists keep it |
| The Starter | the same rules (`docs/STARTER.md` §3.1d): its page area is the window less the rail, its top bar's actions at the window's edge, a destination's cards in a column of at most `CONTENT_MAX` and a task page's text in `COL_W`, centred in the room beside the rail; nothing is capped by the window |
| Sheets and dialogs | as §5: `SHEET_MAX`, centred |
| The Pi panel and compact windows | unchanged by §13 to §17, but for the input panel on compact windows that are not short (§16.3) |

## 18. How this fits the plan already made

Moved to `docs/history/LAYOUT-LOG.md`.

## 19. Plan for wide windows

Moved to `docs/history/LAYOUT-LOG.md`.
