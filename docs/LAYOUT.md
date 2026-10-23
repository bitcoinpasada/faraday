# Layout at every window size

**Kind:** Spec
**Status:** Partly built: size classes and density (L1, F101); the rest specified, not built (F92)
**Open:** the stick's density from its screen's EDID (§7; today its kernel command line's, F101), navigation (L2), Home's grid (L3), list and detail panes (L4), components that keep their size (L5), the supporting pane (L6), tours and tests (L7); planned.

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
- **Cards and tiles** sit in grids of columns with a minimum width,
  never stretched beyond the grid's column.
- **Text** runs at most about 70 characters a line (`MEASURE`, at the
  body size); a column wider than that keeps its text to the measure.
- **Fields** are as wide as what they take: a word box, an amount, a
  path, a passphrase each have their width; a descriptor or an address
  takes the column.
- **Margins and spacers** are Material's: 16 dp on compact, 24 dp
  otherwise, on the 4 dp grid.

## 9. Tokens

In `core/faraday-draw/src/tokens.rs`: the class widths
(`WINDOW_MEDIUM` 600, `WINDOW_EXPANDED` 840, `WINDOW_LARGE` 1200,
`WINDOW_EXTRA_LARGE` 1600) and heights (`WINDOW_SHORT` 480,
`WINDOW_LOW` 600); `RAIL_W` 80, `RAIL_EXPANDED_W` 240 (today's sidebar
width), `NAV_BAR_H` 80, `APP_BAR_H` 64, `NAV_PILL_W` 56, `NAV_PILL_H`
32; `MARGIN_COMPACT` 16, `MARGIN_WIDE` 24, `SPACER` 24; `TILE_MIN`
200, `LEAD_MAX` 560, `LIST_PANE` 360, `COLUMN_MAX` 720, `SHEET_MAX`
560, `SUPPORT_PANE` 400, `MEASURE` 70 characters. The Starter's
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
at 1280 lays out as medium.

The snapshot tours gain the sizes 760x1000, 960x580@133, 1920x1080 and
2560x1440 beside today's.

## 12. Plan

| Batch | What | Model |
|---|---|---|
| L1 | The classes and density (§2, §7): `SizeClass` with the five widths and two heights in place of `is_compact`; the fit to 1280 × 800 and F78's floor removed; the tokens (§9) | Opus |
| L2 | Navigation (§3): the bar, the rail and the top app bar as `faraday-ui` components; the sidebar drawn as the expanded rail; the status strip; DESIGN §4.1 | Opus |
| L3 | Home (§4): the lead tile and the tile grid | Sonnet |
| L4 | List and detail (§5) for Wallets, Files, Vaults and Backups | Opus |
| L5 | Components keep their size (§8): buttons, fields, the measure, across the screens; the `lint-tokens` counts fall | Sonnet |
| L6 | The supporting pane (§6) | Sonnet |
| L7 | The tours at the new sizes, the tests (§11), DESIGN §3 rewritten to this spec, then this plan to `docs/history/` and this file a Reference | Sonnet |

L1 before the rest; L2 before L3 and L4. Each batch is timed in a
late-flow session (`docs/SPEED.md` §7) where it adds drawing work, and
the frame budgets stay as they are. The text size work (F85,
`docs/TEXT-SIZE.md`) builds on L1's classes and goes after it.
