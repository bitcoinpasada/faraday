# Design system

**Kind:** Reference
**Status:** Built

Faraday and the Starter follow Material 3 (F92, F91) in their
components, type, spacing, touch targets, states, colour roles, shape,
elevation, motion and layout, with Faraday's own look; where this
document differs from Material 3, a decision says so. F108 records how
the tokens, colour roles and components follow it, shape included.

Every dimension in it is a name in
`core/faraday-draw/src/tokens.rs`, and every colour on a screen is one of
Material 3's colour roles from faraday-ui's palette (`docs/THEMES.md`);
`tools/lint-tokens.sh` keeps both that way, in Faraday's screens and the
Starter's.

## 1. Why this exists

Faraday's screens were built one at a time, so the same kind of content
could be drawn several ways. This document fixes, per kind of content,
how it is drawn, how it behaves and what draws it, and names every
screen and how it is laid out: the principles (§2), the two layouts and
the theme (§3), the content kinds (§4) and the screens (§5).

## 2. Principles

1. **Labels, not prose.** A working screen is titles, labels, values and
   actions. Longer text on one is a single line: the reason under a
   dimmed control ("Load a key first"), an error or warning line, a
   toast, a sheet's line naming what an act loses. Paragraphs are on
   Learn and About, which are not working screens, in Guided mode's
   walk-through at the top of an open step (§4.12), and behind the Spend
   tab's **More about this**, closed until pressed. Text longer than a
   label, wherever it appears, is plain statement of fact in an
   encyclopedia voice, never metaphor or flourish in place of direct
   statement.
2. **One kind of content, one rendering.** Each content kind in §4 is
   drawn the way its row says, by what the row names.
3. **One screen per kind of data.** Moved to `docs/history/DESIGN-LOG.md`.
4. **One item per row on touch screens.** Moved to `docs/history/DESIGN-LOG.md`.
5. **Fixed geometry for the keyboards.** An on-screen keyboard is the
   same size in the same place every time it is up: the docked keyboard
   at the foot of the panel or of the page beside the sidebar, word
   entry's keys under the word grid (§4.3).
6. **One placement law.** Moved to `docs/history/DESIGN-LOG.md`.
7. **Direct manipulation, few confirms.** A choice is made by pressing
   or ticking it. A single choice that decides a step moves the flow on
   by itself, and pressing the option already chosen moves on too; no
   button only repeats the choice just made (F103). Continue stays where
   a step holds more than one value (steppers, ticks, typing) or the
   press is the act (the words quiz checks a word and Continue answers);
   Back opens the step before with its choice kept (F104). Signing a
   transaction is a button, **Sign with** (§4.4). A hold (§4.13) confirms signing
   with a GPG or Secure Boot key from a vault, deleting from a
   vault, marking a key revoked, the stick a backup writes to, wiping
   a stick, deleting a vault file from a stick, zeroing a stick's free
   space or its whole files partition (F131), saving a secret
   without encryption (F129), and adding a wallet a key not of its own
   spends alone (F132); no hold shows a secret: its eye does
   (§4.10). A question
   before an act is a sheet: Lock's, and a flow's Cancel where ending it
   loses something (§4.13).
8. **Secrets are hidden where they are read.** Words, seeds, private
   keys, passphrases and shares are masked in place: a masked secret's
   eye shows its value until it is pressed again, a **Show** button shows one until
   **Hide**, a passphrase field shows dots (§4.3, §4.10). Faraday has no
   screen for one secret or one string alone (F68).
9. **One unit.** Amounts are in BTC with eight decimals; there is no
   unit setting (§4.7). A fee is in sats with its rate.
10. **No pressure.** Two things count down: the idle warning before the
    lock, and a TOTP code's bar for the time left in its period (§4.14).
    Status shows state, not urgency.
11. **Common idioms, not invented ones.** Where a phone or a desktop has
    a well-understood pattern, the screen uses it: a keyboard types into
    the field with the caret; a list of options is rows; several choices
    are ticked boxes; a fact is a label and its value. Spare space is
    never filled with decoration or with a second rendering of the same
    value.
12. **Every control is easy to hit and to reach.** A press is taken on
    at least 48 × 48 dp on every layout, larger than the control where
    it is drawn smaller, and never over a neighbour; every control shows
    when the pointer is over it, when it is pressed and when the
    keyboard is on it, and the keyboard reaches every one (§4.13,
    §4.15).

## 3. Sizes

Faraday lays its screens out by Material 3's layout rules at every
window size (F92, `docs/LAYOUT.md`), with its own look. A window is
laid out at its screen's own density, never scaled to fit: the desktop
app at the compositor's scale, the stick at the density its EDID gives
on a screen of 11 inches or less and at its kernel command line's
`faraday.dpi` on any other, the Pi panel at its 286 dpi, each times the
person's Display scale (F101, LAYOUT §7). Its size in dp, divided by
the text size's factor, gives its class (`Faraday::window()`,
`Faraday::layout_window`, LAYOUT §2):

| Class | Width | Typical screen |
|---|---|---|
| Compact | under 600 dp (`WINDOW_MEDIUM`) | the Pi panel (268 × 358), a narrow window, a 7-inch panel in portrait |
| Medium | 600 to 839 dp | a half-screen window |
| Expanded | 840 to 1199 dp (`WINDOW_EXPANDED`) | a 7-inch panel in landscape, a 10-inch panel in portrait, a small laptop window |
| Large | 1200 to 1599 dp (`WINDOW_LARGE`) | the desktop app's 1280 × 800, a 10-inch panel in landscape |
| Extra-large | 1600 dp and over (`WINDOW_EXTRA_LARGE`) | a full monitor: 1920 × 1080, 2560 × 1440 |

A window under 480 dp tall is short (`WINDOW_SHORT`, the Pi panel) and
one under 600 dp tall low (`WINDOW_LOW`, the 7-inch panel). The class
and the height give the navigation (`Faraday::nav`, LAYOUT §3):

| Navigation | When | What changes |
|---|---|---|
| Menu | A compact window that is short (the Pi's 2.8" panel). | One column: Home is a menu with a button for each place, and every other page has the top app bar with its way back (§4.1). |
| Navigation bar | A compact window that is not short. | One column under the top app bar, the 80 dp navigation bar at the foot with the five destinations (§4.1); Learn, Settings and Lock in the top app bar; Home leads with the status strip. |
| Navigation rail | A medium or expanded window, and a low one that is not compact and short (the 7-inch panel). | The 80 dp rail at the left with the five destinations, Learn and Settings under a divider and Lock at its foot; one column at most `COLUMN_MAX` (720) wide, centred beside it, under the top app bar (§4.1); Home leads with the status strip. |
| Expanded rail | A large or extra-large window that is not low. | The wide layout: the 240 dp expanded rail, its destinations Material's, listing the flows in progress, the keys, the wallets and the sticks, beside a page that runs to the window's edge (LAYOUT §15), under the top app bar on every page, Home and the step flows included (§4.1). A sheet is centred over the page. The keyboard comes up only from its button. |

`Faraday::is_compact` is every navigation but the expanded rail: there
a step flow is a page per step, a sheet is the window's width less a
margin (compact) or at most 560 dp, centred, and the keyboard
rises by itself under a field that takes typing. The one-column layouts
carry the same flows and steps as the wide layout and move the expanded
rail's lists to Home: the menu's rows on the small panel, the status
strip's chips beside the rail and above the bar, and a flow in
progress's chip at the top of every destination. A step that must not
be left (a stick being written) hides the navigation (F51).

What a page holds sets its width (F114, LAYOUT §14, §17):

- **Reading content** (text, fields, buttons, a flow's cards, lists of
  rows) stands in a column of at most `COLUMN_MAX`, its text at most
  about 70 characters a line, centred in the room beside the
  navigation.
- **Wide content** (the codex32 worksheets, the dice tracks and tree,
  the backup board, the structure chart, the copy grid, a table, Home's
  tiles and the tools' catalogue) takes the work width, from the
  navigation's edge plus `MARGIN_WIDE` to the supporting pane's spacer
  or the window's margin. It never stretches what it draws: it shows
  more of itself, or its cells larger up to a token maximum.
- **List and detail**, from expanded up: Wallets, Files, Vaults and
  Backups show their list in a `LIST_PANE` (360) pane and the one chosen
  beside it, each scrolling on its own (LAYOUT §5). Settings is a list
  of its sections on compact windows and every section on one page, in
  two columns where they fit, from medium up.
- **The supporting pane**, `SUPPORT_PANE` (400) at the right on
  extra-large windows: Learn beside the page, the backup board's lists,
  the codex32 dice sheet's column (LAYOUT §6).
- **What stands at the right** (the top strip's mode pill and ?, the
  eye, the keyboard button, the supporting pane, Home's Scan) stands
  `MARGIN_WIDE` (24) from the window's edge (LAYOUT §15).

Components keep their size (LAYOUT §8): a button is as wide as its
label, a field as wide as what it takes, cards and tiles sit in grids
of columns with a minimum width, and nothing is cut off at the sides,
down to the Pi's panel. Margins are 16 dp on compact windows and 24
otherwise, panes 24 apart (`MARGIN_COMPACT`, `MARGIN_WIDE`, `SPACER`).
Inside the Starter, the held screens keep its one-column rule (under
1280 dp, `docs/STARTER.md` §3.1d), the menu, and the Starter's own rail
beside its wide pages (F91).

Text size (Settings › Appearance: Standard, Large, Larger, F85)
multiplies every text size by 1, 1.2 or 1.4. Keyboards, QR codes and
icons keep their sizes, except a row's icon, which grows with its label;
a one-line slot that cannot grow fits its text down to Standard; a row
or button grows to its text. The one-column layout's thresholds are
multiplied by the text factor and the Display scale, so at Larger a
window takes the one-column layout where it would be 1.4 times too
small for Standard (`docs/TEXT-SIZE.md` §4).

Sizes are design units (dp). The tokens are names in
`core/faraday-draw/src/tokens.rs`: the type roles (below), the older
ramp the screens not yet moved to them still write (title 20, body 16,
label 14, caption 12), the grid and gaps, the radii, the touch floor,
the field heights, the board's and the chart's dimensions, the scanner's
and the QR code's.

Material 3's tokens are named there too (F108):

- **Shape**, Material's corner scale: `SHAPE_NONE` 0, `SHAPE_XS` 4,
  `SHAPE_S` 8, `SHAPE_M` 12, `SHAPE_L` 16, `SHAPE_XL` 28 and
  `shape_full`, round ends. Buttons, segmented buttons and tags are
  full, cards medium (`RADIUS`), chips and keys small (`RADIUS_SMALL`),
  menus, fields (`FIELD_RADIUS`) and the toast extra-small, the floating
  action large, dialogs and sheets extra-large; a checkbox's corners are
  `SHAPE_CHECKBOX`.
- **Touch targets**: `TOUCH`, 48, on every layout (principle 12), held by the touch-target test over the snapshot tours (`docs/M3-CHECKS.md` §2; its exceptions §3, F152).
- **States**: `STATE_HOVER` 8 %, `STATE_FOCUS` 10 %, `STATE_PRESS` 10 %
  and `STATE_DRAG` 16 %, the opacity of a control's content colour laid
  over it (F113); the focus ring, `FOCUS_RING` 3 thick and
  `FOCUS_OFFSET` 2 outside.
- **Icons**: `ICON_GLYPH` 24 where an icon stands alone,
  `ICON_GLYPH_SMALL` 20 in a dense place, `ICON_GLYPH_BUTTON` 18 in a
  button or a chip.
- **Text**: nothing is drawn under `TEXT_MIN`, 11, Material's
  label-small; `Ui` draws a smaller size asked for at it.
- **Type roles**: Material's type scale as `TypeRole`s, each a size, a
  line height and a face: `HEADLINE_M` 28/36, `HEADLINE_S` 24/32,
  `TITLE_L` 22/28 regular; `TITLE_M` 16/24 and `TITLE_S` 14/20 medium;
  `BODY_L` 16/24, `BODY_M` 14/20, `BODY_S` 12/16 regular; `LABEL_L`
  14/20, `LABEL_M` 12/16, `LABEL_S` 11/16 medium. Medium is drawn in
  Noto Sans SemiBold. `faraday-ui` draws by role (`Ui::text_as` and its
  kin, `docs/LAYOUT.md` §8); its components use them, and the screens
  move to them from the sizes they write (L5).
- **Size set**: buttons `BUTTON_H` 40 with their label in `LABEL_L`;
  text fields `FIELD_H` 56, outlined, label above and error line
  under (`Ui::field_box`); list items `LIST_1`, `LIST_2`, `LIST_3`, 56,
  72 and 88 (`Ui::list_item`).
- **Motion**: Material's durations, `MOTION_SHORT1` to
  `MOTION_EXTRA_LONG4` (50 to 1000 ms), and its easings,
  `EASE_STANDARD`, `EASE_STANDARD_DECELERATE`,
  `EASE_STANDARD_ACCELERATE`, `EASE_EMPHASIZED_DECELERATE` and
  `EASE_EMPHASIZED_ACCELERATE`; `docs/MOTION.md` says which motion
  takes which.
- **The toast**: `TOAST_H` 48, shown `TOAST_SHOW_MS`, 4 s.
- **Dialogs**: `DIALOG_MIN` 280 to `DIALOG_MAX` 560 wide, `DIALOG_PAD`
  24 inside; the scrim over a sheet's frost, `SCRIM_ALPHA`, 32 %.
- **Progress**: `PROGRESS_H` 4.

The widgets' own size classes, `small`, `mobile` and `wide`
(`faraday-draw`'s `SizeClass`), size the keyboards and the QR side;
Faraday's screens use `small` on the small panel and `wide` otherwise,
and `mobile` only scales the PNG a QR code is saved as.

Colour (`docs/THEMES.md`, F113): Faraday's colours are Material 3's
colour system. A theme is one seed colour and one Material scheme style;
ten are offered: Cobalt (the one a first start is in), Bitcoin Orange,
Lamplight, Mars, Forest, Frost, Dusk, Lavender, Rose and Monochrome. Each
is drawn in Light or Dark and at Standard, Medium or High contrast, so
`faraday_ui::Theme` is the triple (theme, mode, contrast) and a theme has
six schemes, sixty in all. Material's own method (HCT, tonal palettes,
roles taken from tones, contrast curves) makes them: the tables in
`faraday/faraday-ui/src/pal/schemes.rs` are generated by the vendored
material-color-utilities (`just themes`) and checked byte for byte by
`tools/lint-themes.sh`. Settings › Appearance offers Theme, Mode and
Contrast (THEMES §12); a settings file of before opens each of the
fourteen themes it named in the scheme THEMES §13 maps it to.

A screen draws in a role by its name (`faraday_ui::Role`, 45 to a
scheme), never in a colour value:

- **Surfaces.** The page and a whole-screen fill are surface; a card, a
  tile, a bottom sheet and the expanded rail are surface-container-low; a
  menu and the navigation bar are surface-container; a dialog is
  surface-container-high; a filled field, a track, an idle badge and a
  box inset on the page are surface-container-highest, and one of them
  pressed or chosen is surface-container-high. Standing higher is a
  higher container, drawn flat with a shadow by level (F108), never a
  tint.
- **Text and marks.** Text is on-surface and labels, supporting text and
  placeholders on-surface-variant; filled marks, filled buttons and text
  and icons read as the accent are primary, with on-primary on a filled
  mark; a tonal button and a chosen option are secondary-container with
  on-secondary-container; the focus ring is secondary; the toast is
  inverse-surface with inverse-on-surface.
- **Edges.** A card's, a rule's and a sheet's edge is outline-variant; a
  control's edge (an outlined button, a segmented button, a field, a chip,
  a ⋮) is outline, 3:1 or more on every surface.
- **Status.** OK (done, verified, can sign) and Warning (waiting,
  unsaved, needs attention) are Material custom colours built as the
  error roles are, harmonised with the theme's seed except where that
  would put them next to its primary; Error is Material's own. Each is
  text and marks in the colour (`ok`, `warning`, `error`) and a badge, tag
  or band in its container with the on-container colour for what is
  written on it. A state is never shown by colour alone: it has its icon
  and its words.
- **The board** keeps one look for each kind of box: places neutral
  (surface-container-low, a band in surface-container-high), sticks
  surface-container-highest, vaults the primary family, files the
  secondary family and paper the tertiary family; a vault and paper show
  their family by edge, band and label on surface-container-lowest, not
  by a container fill, which in a fidelity scheme is the seed's own tone.
- **Checksum** is a third custom colour, the codex32 booklet's pink,
  never harmonised, so a worksheet's checksum squares are the booklet's
  pink in every theme; its characters are on-checksum-container.
- **Drawn as they are** in every theme: a QR code's black and white, a
  camera's picture and overlay, the sticks' tones (chosen by mode and
  contrast, F142), and the scrim, black.

A chosen row, card or option, a toggle that is on and the row of the
place on screen take Material's selected state: filled in
secondary-container, everything on them, text, icons and a radio's mark,
in on-secondary-container (`faraday_ui::pal::selected_or`,
`on_selected`), never a tint of primary.

States are Material's: a control's content colour laid over it as a state
layer at hover 0.08, focus 0.10, pressed 0.10 and dragged 0.16
(`STATE_HOVER`, `STATE_FOCUS`, `STATE_PRESS`, `STATE_DRAG`); a control not
available now keeps its kind and shape, its content on-surface at 0.38
and its container on-surface at 0.12 (`DISABLED_CONTENT`,
`DISABLED_CONTAINER`).

Every scheme meets Material's own contrast constraints, and what the
screens rely on: text, the accent and the status colours 4.5:1 or more on
every surface (7:1 at High contrast), a control's edge 3:1 or more. A
disabled control is see-through and not held to a ratio. What is drawn is
checked on the frame: `faraday-core`'s `fills_read` for the screens, the
Starter's `colours_read` for its pages.

At High contrast nothing see-through lowers the contrast of what is on
it: a scheme at High contrast draws no shadow (`Ui::shadow`) and no fade
at a scrolled edge, and the scrollbar at full strength; a colour asked
for see-through is drawn at full strength as an edge, a rule or a mark,
and as a tint under what it holds (under `HIGH_CONTRAST_TINT_MAX`) as its
edge alone, the page left under the text (`Ui::fill`, `Ui::stroke`). State
layers, a control not available now and what fades in or out stay
see-through (`Ui::wash`); the state layers and the scrim are chosen so
text on them keeps 7:1 and edges 3:1. These rules follow Contrast High
(`Theme::is_high_contrast`) on any theme, not a theme of their own.

Faces: Noto Sans and Noto Sans SemiBold for text, Noto Sans Mono for
data, Noto Sans CJK for the characters the wordlists and their keyboards
need and the text faces lack, and a closed icon set (`Icon`, from the
SeedSigner icon font and Font Awesome 6 Free Solid) with Faraday's own
drawings beside it (`Glyph::Stick`, the shield `Ui::mark`, the wordmark
`Ui::wordmark`). Each glyph is stored once as an outline and filled at
the exact pixel size a display asks for.
The mark (F140) is a keyhole shield 100 by 135.3 units: its sides run
straight to the height over the golden ratio (83.62), then curve to the
tip; its top corners are rounded at the band times the golden ratio
(11.33). The band, the shield's lighter rim, is 7 units wide everywhere:
the inner shield is the outer one moved in by 7. The keyhole's head is as
wide as its foot, the inner width over the golden ratio squared (32.85);
its neck is 9.86, in the proportion F10 drew it to the head; the slot runs
from the head's centre to the foot the head's width times the golden
ratio; and the keyhole's middle, between the head's top and the foot,
stays where F137 put it (56.8 units down). The shield is in primary, the
band primary mixed half with white. `Ui::mark` draws it in the app and
the Starter's icon; the boot logo's pixel grid
(`faraday/image/common/faraday-mark.txt`) is it at 17 by 23 squares, set
by hand: the edge one square all round, a one-square stair to the tip, a
one-square neck.
Anything read character by character is set in the mono face:
fingerprints, hex, paths, addresses, extended keys, txids, a descriptor
row's value, and the words as typed and offered on word entry; prose,
labels, buttons and titles that are words stay in the text faces.

## 4. Content inventory

Each kind: how it is drawn, how it behaves, and what draws it. A name in
`code` is a function or type in `faraday-ui` (`faraday_ui::…`, `Ui::…`,
the `pal` colours), in `faraday-draw` (`Icon`, `KeyboardKind`, the
tokens), or in `faraday-core`, named by its module where it is not in
`screens/`.

**Elevation.** What floats casts Material 3's shadow of its level
(`Ui::shadow`, `Canvas::elevation`; sizes `ELEVATION_KEY` and
`ELEVATION_AMBIENT`, levels 1 to 5): two layers in black, a key layer at
30% and an ambient layer at 15%, each with its y offset, blur and
spread. The same black in every theme but the high-contrast ones, which
draw none: what floats there keeps its outline, so no text or edge pair
falls below 7:1 and 3:1. Level 1: a modal sheet (with its scrim), a
board chip and a raised card (`ELEVATION_SHEET`). Level 2: a menu, a
popup and the wallet list (`ELEVATION_MENU`). Level 3: the floating
action button, a toast and a notice (`ELEVATION_FAB`). Level 4: the
floating action button under the pointer and a dragged chip
(`ELEVATION_RAISED`).

### 4.1 Navigation

| Kind | Rendering | Behaviour |
|---|---|---|
| Expanded rail (wide, the sidebar) | 240 units (`RAIL_EXPANDED_W`) at the left (`sidebar`): the wordmark, FARADAY OFFLINE or FARADAY DEMO (F165); the tabs Home, Vaults, Files, Wallets, Tools, Learn, then the stick visit while a stick is attached and nothing is held, Transfer in the desktop app, and Settings, each one of Material's destinations (`faraday_ui::nav::destination_row`: icon and label in a row 48 units tall, the current one on an accent pill, Files and the stick visit with their badges as the rail has them); under its tab a row for each flow in progress with its open step at the right; the loaded keys under Keys and the wallets under Wallets, each row 48 units tall, the two groups one region that scrolls within the sidebar where they are taller than its room, the tabs and the foot staying put (Material 3 Navigation drawer: its content scrolls); Files, its count and under them the session strip, **Bring in · Open · Work · Write out**, the current stage in the accent, in one 48-unit row; the sticks attached; Lock and the power button. | A tab opens its screen; Learn opens the Learn sheet, or on an extra-large window Learn's supporting pane beside the page, and pressed again while the pane is shown puts it away (`docs/LAYOUT.md` §6); a flow's row resumes it; a key opens Explore on it; a wallet opens its card; the wheel, a trackpad or a finger over the lists scrolls them; the Files row opens Files (`docs/FLOWS.md`, Sidebar). |
| Top strip (desktop and Starter windows) | The top `TOP_BAND` (56) units of the desktop app's window, taller where a strip of stick pills is (a device's backup flow on the one-column layouts, and Faraday Demo held bare there with `--panel`), and the Starter's top app bar: its empty room, where no control or touch floor lies, is the window's handle (`Faraday::is_strip_room`, `faraday_window::drag`). | A press there and a drag move the window (`Window::drag_window`); a double-click maximises it or restores it; a press without movement does nothing. |
| Way back | The top app bar's back arrow, with where it lands under the page's name as "Back to Tools", "Back to Kind" (F146), on every page that has one, the step flows' included (`flow::Column`'s `back`, `screens::bar_head`). | It lands on the screen seen before this one, as it was seen, and is named after it (F77). In a flow with steps it opens the step before the open one, its choices as they were, and is named after that step; only the first step's lands on the screen before, and a running step takes none (F104, `StepFlow` in `faraday/faraday-core/src/trail.rs`). The mouse's back button does the same (`Ui::back`), and its forward button opens again the step or screen it left. |
| Bar (the menu's windows) | `compact::BAR_H`, Material's small top app bar at `APP_BAR_H` (64), across the top of every page but Home and a step flow on the small panel; a step flow keeps it on every one-column layout (`compact::bar`, `faraday_ui::nav::app_bar`): the chevron in a 48 dp target, the page's name in Title L, or Title M where Title L would be cut beside the tools (F139, where Faraday differs from Material's small top app bar), with where it goes back to under it in Label M, "Back to Wallets", or the name alone where that would be cut (F146), and at the right end the tools (`compact::tools`), each in a 48 dp target: **Cancel**, Material's text button, on a flow in progress, the eye on Make codex32 shares' sheets, the keyboard button where the page types, the mode pill off mainnet (Practice's reading "Practice" alone; `docs/MODES.md` §3), and the ? where the page has a Learn page. A step flow draws its own top and the same tools (`compact::flow_bar_tools`). On the Pi's 268 dp they make room as Material's top app bar does, never running under the chevron's 48 dp target: the page's name steps down to Title M, is shortened at a word with an ellipsis only where that is cut too ("Make a new…"), and is left off where not even its first word would show whole (never "Cre…"; the step's own heading names the page), and where they would still reach the chevron the ?, then the keyboard button, then the eye go into Material's overflow menu, a ⋮ at the bar's end (`compact::tools`), Cancel and the mode pill staying on the bar; no two of their targets overlap (owner, 2026-10-26, 2026-10-27; `docs/LAYOUT.md` §8). On a step whose press draws the system's randomness the ⋮ is drawn on the Pi's panel even where the tools fit, with the ? and How in it, and **Add randomness** is its first row (`docs/OWN-RANDOMNESS.md` §8.3, F157). | The chevron goes back as the back link does. The ⋮ opens its menu (on the small panel a sheet, as every menu there): **Add randomness** opens Add randomness as the Randomness row does, **Learn** opens the page's Learn as the ? does, **Show keyboard** or **Hide keyboard** and **Show sheets** or **Hide sheets** do what their buttons do. |
| Randomness row | `screens::randomness::row`, directly above every press that draws the system's randomness (`docs/OWN-RANDOMNESS.md` §8.1, F157): Material 3's two-line list item (Lists), the headline **Randomness** in Body L on-surface, the status line ("System: getrandom(2) · your own: 99 dice rolls") in Body M on-surface-variant, wrapping under the headline where the column is narrow, and as its trailing element Material's text button **Add randomness** in primary, in a 48 dp target; at least `LIST_2` (72) tall. On the Pi's panel the status line alone in Label M, cut at a count's comma with an ellipsis, never in a number, and Add randomness the first row of the bar's ⋮ (Bar). The Lock sheet keeps the whole row on every panel. Shown at Dealing SLIP-39 shares, codex32's Suggest, Create vault, Seal and export, a GPG or Secure Boot key's Make, Enrolment files, Hold to sign, Seed XOR's parts, Make the new set, a MuSig2 round's Sign here and Collect, and a lock that seals a changed vault (the Lock sheet, and the open vault's screen while it holds changes). Never blocks the press. | **Add randomness** opens Add randomness with no source chosen and nothing started; Back lands on the step as it was left, its sheet over it again. |
| Cryptography (not built; `docs/CRYPTO-LEARN.md` §8) | On a page or step flow whose screen offers a cryptographic action of the manifest, Material's standard icon button in a 48 dp target immediately left of the ?: `Icon::Method`, Material Symbols' `fact_check` in its filled form baked into the icon face, its plain tooltip "How Faraday does it", no selected state. The bar's tools then read Cancel, the eye, the keyboard button, Load samples, the mode pill, **How Faraday does it**, the ?, Lock and the ⋮. On a compact bar too full for it, the Pi's panel among them, it is the first to go into the overflow menu, before the ?, and its row there is the menu's first, "How Faraday does it" with its icon. In a flow it shows from the step that offers the action and on the steps after it that carry its result. The Starter's top app bar carries the same button left of the ⋮ on What to write, Confirm, Writing and Done, and on Start this computer from the stick while the BitLocker recovery key is offered. | It opens Learn as the ? does, on its **How Faraday does it** tab: the Learn sheet below extra-large, the supporting pane at extra-large, where pressed again while the pane shows that tab it puts the pane away and pressed while the pane shows another tab it moves to that tab. In the Starter it opens a sheet of the blocks, with **Close** at its foot. |
| Title | Every page is named by its top app bar, Home and the step flows too; while Make codex32 shares' **Characters** card is open and a character is still to come, the bar's title is where the entry stands, "Share A · character 7 of 26", "Share A · 7 of 26" where the whole would be cut (the Pi's panel), so it stays in view as the sheets scroll under the bar; where neither fits beside the bar's tools the card reads it in place of **Characters** (F146); on the wide layout Home's bar reads **Home**, the sidebar's head holding the wordmark. A step flow's column keeps at its head what is the flow's: the chip where the flow names what it works on at the left, **Steps only · Guided** at the right (`flow::column`); its **Cancel** is on the bar. | The step count sits beside a step's name as muted text (§4.14), never in the title. |
| Home | On the wide layout: the lead tile while something is under way or waiting, then half-width tiles two to a row (`docs/FLOWS.md`, Home). On the one-column layouts: the menu, a button for each of the sidebar's entries, led on the small panel by its mark and name and on the rail's and the bar's windows by the status strip; Lock and Power off at its foot on the small panel only. **Scan** floats at the bottom-right on both (`Ui::fab`). | A tile or a button opens its screen; Scan opens the scanner. |
| ⋮ menu | Three dots at the right end of a file row or a board chip, opening a popup under them on the wide layout and a sheet on the small panel (`faraday/faraday-core/src/menu.rs`; the board's, §4.17). | Arrows move, Enter chooses, Escape closes; a press outside closes it. |
| Pager | **Previous · Next** at the foot of Backups on the small panel, one wallet a page. | For runs of same-kind pages only. |
| Navigation rail | `RAIL_W` (80) at the window's left in `SIDEBAR` (`faraday_ui::nav::rail`, `nav/screen.rs`): Home, Wallets, Files, Vaults and Tools from the top, then the stick visit and Transfer while they apply; Learn and Settings under a divider; Lock while something is held and Power off at its foot. Each is its icon on a `NAV_PILL_W` × `NAV_PILL_H` pill (accent while current) over its caption label, `NAV_ITEM_H` (56) tall, never under 48 on a low window; Files carries the Inbox's count and the stick visit a dot while a stick waits (§4.8, Destination badge). Not drawn for a step that must not be left (a stick being written, a backup flow's key being made; F51). | A press opens the destination; Learn opens the Learn sheet; Lock and Power off as on Home. |
| Navigation bar | `NAV_BAR_H` (80) across the window's foot (`faraday_ui::nav::bar`): Home, Wallets, Files, Vaults and Tools, evenly spaced and drawn as the rail draws them; the stick visit or Transfer in place of Tools while it applies (Tools stays on Home). Not drawn while the keyboard is up, nor for a step that must not be left. | As the rail. |
| Top app bar | `APP_BAR_H` (64) across the page beside the rail or above the bar, and beside the expanded rail over every page, from the rail to the window's edge, its title the page's own (the name and way back the page hands over as it is drawn, `screens::bar_head`; Files' stick chip left of its actions) (`faraday_ui::nav::app_bar`): on Home the mark and the wordmark on the rail's and the bar's windows (FARADAY OFFLINE on the stick and the Pi, FARADAY DEMO on Faraday Demo, F165; the second word under FARADAY where the two do not fit side by side, as on the Pi's panel beside the mode pill; `docs/LAYOUT.md` §3), and beside the expanded rail, whose head holds the wordmark, the name **Home**; elsewhere the back chevron, the page's name, and under it where the chevron goes in Label M, "Back to Tools" (F146); at the right end, beside the expanded rail, a device's stick pills while a backup flow uses sticks, left of the rest, so the page under the bar does not move as they come and go (**Stick pills**, below; `stick::pills::bar`), the keyboard button where the page types, Load samples in Practice, the mode pill off mainnet, the ? where the page has a Learn page, and on the bar's windows Lock while something is held and the ⋮ menu (Settings, Power off; a sheet). Nothing stands above it: the wordmark is at the window's top, level with the rail's first destination. Under it, a chip for each flow in progress (`faraday_ui::nav::status_chip`). Over a step flow it carries the flow's name and way back and, at its right end, **Cancel** while the flow can be left and the eye on Make codex32 shares' sheets (`compact::bar_cancel`, `compact::bar_eye`), as the one-column layouts' bar does. | The chevron goes back as the back link does (F77); a flow's chip resumes it. |
| Stick pills (Faraday Demo) | In the host strip at the window's foot, outside the app (§4.19, F118): the boot stick's pill, then Stick 2, 3 and on, each with its dot (green while in, grey while out, an amber edge while the flow asks for it) and "Stick 2 · in" or "out", in the host's look. As many as fit beside the rest of the strip; the rest fold into **Sticks · N**, which opens the host's Sticks menu of every stick, in or out. Held bare (`--panel`), the app draws them as a device does: beside the expanded rail in the top app bar, left of its actions; on the one-column layouts in a strip above its bar. | A stick's pill, or its row in the menu, inserts the stick or pulls it (the shell's folder); a row closes the menu. |
| Sticks menu | Material's menu under its pill (`menu::draw`; a sheet on a compact window): "Sticks", then every stick with its state, "Stick 4 · in". It unrolls from its top edge on Material's emphasized decelerate (`motion::MENU_IN_MS`) and rolls up on its emphasized accelerate (`MENU_OUT_MS`); at once under Reduce motion. | A stick chosen inserts or pulls it and closes the menu; a press beside it, Escape or its pill closes it. |
| Status strip | At the top of Home beside the rail and above the bar: the sidebar's lists as chips, 32 units tall in a 48-unit touch row: each flow in progress with its open step, the keys (four, then "+ N more"), the wallets (the same), Files with its count, and the sticks attached. | Each opens what its sidebar row opens; a stick opens the stick visit, or the waiting import. |
| Navigation rail (Starter, medium and expanded) | 80 units at the window's left in `SIDEBAR`: the destinations Home, Make, Tools and Learn from the top, each its icon over its caption label, the current one on an accent pill behind the icon (`docs/STARTER.md` §3.1d, F91). Not drawn on a page inside a task. | A press opens the destination as it was last seen; a press on the current one goes to its top. |
| Navigation bar (Starter, compact) | 80 units across the window's foot: the same four destinations, evenly spaced, drawn as the rail draws them. Not drawn on a page inside a task. | As the rail. |
| Top app bar (Starter) | 64 units across the page: the wordmark on Home, a destination's name on the others, a back arrow and the page's name on a page inside a task; at the right end Install or Update while it applies (secondary) and the ⋮ menu (About, Uninstall while installed, Remove the boot entry while offered). It stays while the page scrolls under it. | The back arrow lands on the destination it was opened from, as it was seen (F77). |

### 4.2 Choice

| Kind | Rendering | Behaviour |
|---|---|---|
| Option row | One rounded box 48 units tall per option: its name at the left and its line at the right, the line under the name on the small panel; the chosen one in Material's selected state, secondary-container with its words in on-secondary-container (`kind_rows`). The everyday options first, the rest behind **More kinds**, a ghost button: on Create's Kind, Single key, backup in shares (codex32), ticked at entry, then Single key and Multisig (`docs/CODEX32.md` §6.9). | A press chooses it; on a card that asks only this, it moves on to the next card, and a press on the one chosen does too (F103). Used for: Create's and Restore's Kind, Create a vault's size. |
| Segmented button | One choice among two to five, side by side in one outlined container, round at its ends, the chosen segment in `SELECTED` with a tick (`Ui::segmented`); where the options do not fit the width, option buttons instead (`Ui::choices`). The chosen one is never drawn as the screen's filled button. | A press chooses it. Used for: Settings' Lock after, Power off after, Mode and Contrast (`SEGMENTED_H`), Display's Scale and Rotation, and Signed amounts; the QR sheet's format, speed and size; a flow's **Steps only · Guided**, its fill sliding between them, and the same control for the same setting at the top of Settings. |
| Filter chip | Material's filter chip, `FILTER_CHIP_H` tall with `SHAPE_S` corners, wrapping at the width `FILTER_CHIP_GAP` apart: unselected, edged in outline, its label in on-surface-variant; selected, filled in secondary-container with no edge, its label in on-secondary-container. Settings' Theme chips (`Ui::theme_chips`, THEMES §12, F120) are each filled in that theme's own primary-container in the mode and contrast on screen, their name in its on-primary-container, leading with that theme's primary as a dot; the chosen one carries the tick in place of the dot and a ring in its own primary, `THEME_CHIP_RING` thick, `THEME_CHIP_RING_GAP` outside it; Cobalt's carries **Default** after its name. | A press chooses it. Used for: Settings › Appearance's Theme, and the Starter's Appearance page. |
| Switch | Material's switch (`Ui::switch`, `SWITCH_W` by `SWITCH_H`): on, its track primary and its handle on-primary at the right; off, its track surface-container-highest edged in outline and a smaller handle in outline at the left; pressed, the handle larger. In a list row with its label at the left (`Ui::switch_row`). | A press turns it over. Used for: Settings' Reduce motion (MOTION §3.5). |
| Option buttons | A row of buttons that wraps at the width (`faraday_ui::parts::button_rows`, `wrap_buttons`), the chosen one `Style::Selected` (`Style::chosen`) and the others secondary. | A press chooses it, and on a step card that asks only this, moves on as an option row does (F103). Used for: Add a key's list and form, Explore's script, the scanner's cameras, Tools' modes, Make codex32 shares' Scheme and Entropy, its entry's strip of worksheets and its String · Codex32QR, Sign a message's format, BIP-85's application. |
| Tab row | Option buttons `CODEX32_TAB` high, one per item in its order, the shown one primary and a ✓ before each once shown, wrapping at the width; on a compact window the shown item's name over **Previous** and **Next** (`docs/CODEX32.md` §6.5). | A press shows that item. Used for: Make codex32 shares' Worksheets. |
| Multi choice | A list of full-width rows (`Ui::multi_list`), one option per row, a checkbox at its start; any number ticked; Continue under it. The question is the card's or the page's title. A row that may not be ticked is dimmed and inert. Several lists may share a page, each under its own label. | Tap ticks or unticks the row. Nothing has to be ticked. A row may carry a value under its label, muted (`Ui::multi_list_valued`; the Load samples sheet's type of each sample). Used for: the backup plan's Software card (the software and the form), on Back up and in the wallet flow, and the Load samples sheet. |
| Checkbox | A tick box (`Ui::checkbox`), `SHAPE_CHECKBOX` corners, a 2-unit edge while unticked; one ticked and not enabled shows its tick dimmed. | A press ticks or unticks it; a dimmed tick is done and a press does not undo it. |
| Stepper | A label and a count between − and +, each Material's outlined icon button (`Ui::outlined_icon_button`): an `ICON_BUTTON` circle edged in `OUTLINE`, the `Icon::Remove` or `Icon::Add` glyph at `ICON_GLYPH` in `MUTED`, a `TOUCH` target, its plain tooltip "Decrease" or "Increase" (`Ui::stepper`; the label in `STEPPER_LABEL_TEXT`, the count in `STEPPER_VALUE_TEXT` in `STEPPER_COUNT_W` between the buttons). On the wide layout the buttons stand `STEPPER_LABEL_COL` in from the label's start, or past a longer label; on the small panel at the row's right end. Not available, both are drawn at the disabled opacities. Material 3 has no stepper component; its parts are icon buttons (m3.material.io, Icon buttons: 40 dp container, 24 dp icon, 48 dp target). Used for Make codex32 shares' Other (threshold, strings), Create's Quorum and Account, the codex32 backup's counts and the board's keys left off. | Each press moves the count by one within its bounds; the keyboard reaches each button and Enter or Space presses it. |
| Slider | A track with a stop for each whole number and a round thumb carrying the value, the two ends' numbers under it (`CoreUi::slider`). | A press on a stop sets it; a drag presses each stop it crosses; the arrow keys step the slider last pressed. |

### 4.3 Entry

| Kind | Rendering | Behaviour |
|---|---|---|
| Text field | A box with `FIELD_RADIUS` corners and an `OUTLINE` edge (the accent while typing goes to it), what is typed in it and the caret where typing goes (`Ui::typed`, `Ui::caret`); what is selected is drawn on secondary-container behind it, in on-secondary-container (`Ui::selection_band`). A passphrase field shows dots, and **No passphrase** while it is empty (`pass_field`, `CoreUi::no_passphrase`); a field that may hold a secret has an eye at its right end (`secret_box`). | A press places the caret between the two characters nearest it and a drag selects, in every field, a masked one too (`docs/TEXT-CURSOR.md`); a field of several lines notes each line (`Ui::typed_lines`); a grid of words typed takes the caret across its cells, and a press on a cell puts it there. The eye shows the text in the clear until it is pressed again. A field that takes no typing while a stick is attached says so in place of its content. |
| Search bar | Material's search bar (m3.material.io, Search): `SEARCH_BAR_H` (56 dp) tall, fully rounded, `SurfaceContainerHigh`, the magnifying glass (`Icon::SEARCH`) leading at `SEARCH_INSET`, the placeholder ("Search entries") in `OnSurfaceVariant` at the body size while it is empty, what is typed and the caret otherwise, the accent edge while typing goes to it, and a × in a 48 dp target at its end while it holds text (`vault/screen/search.rs`). Over an open vault's Wallets, Entries and GPG keys and over the TOTP codes, nowhere else (`docs/VAULT.md` §12). | Filters the list under it as it is typed, by the title, case not counted; nothing found says so in a line ("No titles match", "No codes match"). The × empties it. On a compact window a press opens Material's search view: the bar at the page's top, what it finds under it, the keyboard under them; Done leaves it with the list filtered in place. |
| Keyboard | The on-screen keyboard docked at the foot of the panel or of the page beside the sidebar (`faraday_ui::keyboard::keyboard`): the path keyboard on Explore, the address keyboard on Explore's Find, the passphrase keyboard everywhere else (`KeyboardKind`). | On the small panel it rises by itself under a field that takes typing; on the wide layout only from the keyboard button. Its keys type into the field typing goes to, as a physical keyboard does (§4.15). On the small panel, when it rises, and when focus moves to another field while it is up, the page's region scrolls only as far as brings the field, with a label drawn above it (`Ui::field_label`), within the region, `KEYBOARD_CLEAR` clear of the keyboard's top and of the region's top (`compact::osk_follow`, after the pass that drew the field, which then draws again); put away, nothing moves. A search bar opens Material's search view instead. |
| Word keyboard | Word entry's own keys, in the page under the word grid (`word_keys`): the BIP-39 letters for a Latin list, and for the others the kana, jamo, pinyin or 注音 keyboard (`KeyboardKind::Kana`, `Jamo`, `Pinyin`, `Zhuyin`). A key that leads to no word on the list is dimmed. | Types the list being entered; the word is taken from the candidates. |
| Kana keyboard | The 五十音 as a grid: ten columns of the consonant groups あかさたなはまやらわ, five rows of the vowels あいうえお, every key one unit wide. The cells the grid leaves empty carry ゛, ゜ and 小; the backspace takes the bottom-right cell, ん the one above it. | Types the Japanese list. ゛and ゜ add the voicing marks the list is written with; 小 turns the last kana into its small form and back. か and ゛ read as が. |
| Jamo keyboard | The two-set layout on the QWERTY positions: ten, nine, and seven keys with shift at the left and the backspace at the right of the last row. | Types the Korean list. Shift gives the five doubled consonants and the two shifted vowels and releases after one key. The keys run through the two-set automaton, so ㄱㅏ reads as 가 and ㄱㅏㄱ as 각. |
| Pinyin keyboard | The BIP-39 letter rows, and a fourth row of the five tone keys 1 2 3 4 5 with the backspace ending it. | Types the Simplified Chinese list, looked up by a Mandarin reading: the syllable on the letters (`v` for `ü`), then its tone. A letter is live while some candidate's syllable continues with it; a tone key is live in the tones that syllable has. The candidates fill only once the tone is typed. |
| 注音 keyboard | The 大千 layout, four rows on the QWERTY positions, the tone marks ˊ ˇ ˋ ˙ in the first row and ˉ ending the second, the backspace ending the last row. | Types the Traditional Chinese list. Every tone is a mark key, ˉ for the first; the mark completes the reading and the candidates fill only then. |
| Word grid | One numbered cell per word of the count, the count's buttons (12 · 15 · 18 · 21 · 24) above it, the word being typed in its cell (`typed_cells`, `keyed_cells`); a status line under it states the words and the checksum. | Each word is checked against the list as it is typed; Tab finishes a word on a Latin list once only one fits. |
| Candidates | The words the keys typed can still become, a suggestion chip each (`SHAPE_S` corners, an `OUTLINE` edge) in the mono face, wrapping at the width (`keyed_pills`, `Ui::word_pill`). | A press on a pill takes the word. |
| Dice, coin, card keys | A row of buttons in New key's randomness step: 1 to 6 for a die, Heads and Tails for a coin, and the ranks and suits for cards, with the count so far, "{} of {}", and the words a run has made (keygen). A face a procedure does not take next is `Style::Disabled`. | Each press is one entry. |
| Dice roller | Faraday Demo in Practice only (F96, `docs/CODEX32.md` §5.6; owner, 2026-10-24): in Faraday Demo's host strip (§4.19), never beside the dice pad's entries: one die (`ROLLER_DIE`, pips drawn, outlined in the accent while it tumbles), then the strip's own **Roll** and **Rolls per click · 1**, which opens a menu of 1 · 5 · 10 · To the end · Number. The die is drawn by `fresh/roller/screen.rs`, its sizes and timings `ROLLER_*` tokens. The codex32 dice step's roller (`docs/CODEX32.md` §5.5, `fresh/roller/five_screen.rs`) is the same with five dice (`ROLLER_FIVE_DIE`) labelled Die 1 to Die 5 and **One click rolls**: One pass · One character · One share · To the end. | **Roll** lands all the click's values on the entries as presses, in the frame that shows the click; the one die then tumbles for Material's short 4, skipped under Reduce motion (MOTION); the five dice tumble and pause between values as before. Values are marked as this computer's: the result's entropy line says "simulated dice". |
| Code box | A value whose exact characters must be plain, in the mono face on a tinted, rounded box padded by tokens, with no quotes (`Ui::code_box`; `Ui::code_line` puts one after a line of text, below it where it does not fit). Used for: the Practice passphrase. | Not a field: it takes no press. |
| Error line | One line under the field or the grid in `ERR`: the codec's or the list's reason ("No word starts with that"). | Replaces the status line while it stands. |
| Use a sample | In Practice, with samples set (F81, `docs/MODES.md` §6.1), a button in Faraday Demo's host strip (§4.19) while the field where a key or an input is typed is empty; nothing is drawn inside the field, whose caret and placeholder stand as in every mode. Labelled "Use a sample", or by what it puts in: **Fill words** on Check my copy, the scanner's **Load sample seed**, **share**, **key**, **wallet**, **PSBT**, **SeedQR**, **code**, **address**, and **Load sample PSBT** where a step asks for the transaction. | Puts the next sample not loaded in the field as typed; a part of a set fills the next part of the set begun. Gone once the person types in the field or a sample fills it; Clear or the next part brings it back. Never drawn outside Practice. |

A seed is typed on Add a key (`Screen::Entry`): **Scan a SeedQR** above
the words, then the word grid, the candidates and the word keyboard,
then the passphrase field and **Add key**. A line in `WARN` under it says
never to type a seed that holds real bitcoin (`never_real_seed`); the
line has its own room one gap under the field, and what follows (New
key's **Show words**, Make a new wallet's passphrase note) starts under
it.

### 4.4 Identity and keys

| Kind | Rendering | Behaviour |
|---|---|---|
| Fingerprint | Eight hex characters in mono, never split (`fp_text`). | The key's name everywhere: the sidebar's Keys, a wallet's keys, "Sign with 73c5da0a", a board chip. |
| Key in the sidebar | The key's fingerprint and what it is ("passphrase" where it has one) under Keys (`sidebar`); a key recorded as a BIP-85 child seed and named by its fingerprint alone reads as the child seed it is, "BIP-85 Words · 73c5da0a · index 0" (`wallet::key_name`, F166), as do Explore's and BIP-85's key rows and its vault record's line. | A press opens Explore on that key. A key has no page of its own: BIP-85, a vanity address and a backup are Tools and wallet flows (F68). |
| Sign with | A primary button in the Spend tab's signing step, "Sign with" and the fingerprints of the loaded keys that sign this transaction, beside the count of inputs (`step_body`); once signed, "Signed with" and the fingerprints in `OK`. | A press signs every input those keys can sign. Where no loaded key signs, the step says so and has no button. |

### 4.5 Long strings

| Kind | Rendering | Behaviour |
|---|---|---|
| Short form | The first eight characters in two groups of four, an ellipsis and the last four, "bc1q 8f3x … k2pz" (`faraday_ui::short`); a string of sixteen characters or fewer is shown whole, in groups of four. | Inside a line that names a string rather than shows it to be read: the Spend review's outputs ("to bc1q 8f3x … k2pz"), its txid and its inputs, the first address on Restore's card and on the Silent payments screen. It opens nothing. |
| Whole string | Whole, mono, in groups of four (`faraday_ui::grouped`), wrapped where it does not fit, never cut. | Where the string is what a person reads or checks: a wallet card's first receive address, a vanity find, a decoded output, a signed message's address, an image's SHA-256. The Key explorer's addresses are whole and wrapped too (F76). A receive address (a wallet's card, Create's and Restore's Check, the chart's wallet sheet, the Spend tab's pages), a vanity find and Explore's address, public key and extended key open their QR sheet when pressed: a chevron at the row's end, the row a touch target tall with its text in its middle (`address_press`, `row_press`; `docs/QR.md` §7). A change address, and an address held back for a backup, open nothing. |
| Descriptor | A summary row, not the descriptor's text: "Descriptor" above; below, mono and wrapped, the wallet's shape ("2 of 3 · native SegWit multisig"), its keys' fingerprints, and " · #" and the checksum where the wallet did not come from a record; chevron (`descriptor_row`). | On Create, Restore, the key-making flow and a wallet's sheet on the chart. The row opens the wallet's QR sheet, which carries the descriptor's whole text under the code. |

Faraday has no screen that shows one string alone (F68): a string is
whole where it is read.

### 4.6 Derivation paths and script types

| Kind | Rendering | Behaviour |
|---|---|---|
| Path | A mono string, `m/84h/0h/0h/0/0`. As a row: "Path" above, the paths below, wrapped, with **Change** at the right while the wallet can be taken back (`path_row`, Create's Check). | **Change** goes back to the Account row (F98). |
| Account | Create's account, as the Path row is drawn: "Account" above, the number below, with **Change** at the right (`account_row`), at the foot of Quorum or the head of Keys (F98). | **Change** puts the Stepper (§4.2) in its place. |
| Path field | Explore's path: a field typed on the path keyboard (`KeyboardKind::Path`), with **Index -1** and **Index +1** beside it. | The values under it follow the path as it is typed. |
| Script type | Named in one vocabulary: Native SegWit, Nested SegWit, Taproot, Legacy; on Explore one button each (`explore::screen::script_name`), in a wallet's shape after its kind ("Single key · native SegWit"). | Never "BIP-84" or "p2wpkh" on a working screen. |

### 4.7 Amounts

| Kind | Rendering | Behaviour |
|---|---|---|
| Amount | In BTC with eight decimals, "0.00060000 BTC" (`faraday_ui::btc`). | One unit; there is no unit setting. |
| Fee | One value, the fee and its rate: "{fee} sat · {rate} sat/vB". | The Spend review's and Decode's. |

### 4.8 Badges

| Kind | Rendering |
|---|---|
| Network | A caution pill naming the mode off mainnet, `MODE_PILL_HEIGHT` tall (`modes::screen::pill`): on the top app bar. Pressing it opens the mode sheet; on mainnet the sheet is Settings' Mode row (F81, `docs/MODES.md`). |
| Load samples | In Practice only, when the app has samples (Faraday Demo), **Samples** in the host strip after the sticks (§4.19). Pressing it opens the Load samples sheet over the app: the line "In Practice, the sample passphrase is" with `faraday` in a code box, then a tick box row for each sample wallet and key with its type under its name (a wallet's kind and script, a key's form and size). |
| Chip | A rounded label with a dot, in a tone (`Ui::chip`): beside a step flow's heading, where the flow goes next or what it works on (`flow::Column`'s `chip`). A status tag is one in a status's container colour with its on-container colour on it (`Ui::tag`, `faraday_ui::Status`). |
| Pill | A rounded label with an edge (`Ui::pill`, `Ui::pill_in`): a stick's pill in its own tone (`Ui::stick_pill`, F142): filled in its tone while the stick is in, its label white or black on it; edged in its tone while the flow asks for it; otherwise quiet. The HOTP and Unreadable badges, in place of a TOTP code on Codes and on a vault entry, are the same shape. |
| Step badge | A step's number in a circle, ticked once done (`Ui::badge`; a guide step's, `faraday_ui::parts::step_circle`, filled in `OK` with its number in on-ok once ticked). |
| Pips | A threshold as `n` dots, `filled` of them filled (`Ui::pips`). |
| Destination badge | Material's badge on a destination's icon in the rail, the bar and the expanded rail, in `ERR` (`faraday_ui::nav::badge`): on Files, the large badge, `BADGE_LARGE` tall, with how many files the Inbox holds ("999+" past 999); on the stick visit, the small badge, a `BADGE_SMALL` dot, while a stick attached has not been visited since it arrived. None at nothing waiting. |

### 4.9 Codes

| Kind | Rendering | Behaviour |
|---|---|---|
| QR sheet | A sheet (`qr_sheet`): the title, an animated code's frame "{} · frame {} of {}", whether scanning it is safe — "Secret · whoever scans this can use it" in `ERR` or "Public · safe to scan" in `OK` — the code on a white card with its quiet zone (`faraday_ui::parts::qr_code`), a descriptor's whole text under it; then the format (UR · BBQr · Parts, Parts for text only), an animated code's speed (2/s · 3/s · 5/s) and how much each frame holds (Small · Medium · Large), **PNG** for one code of public content, and **Done**. On the small panel the code is as large as the panel allows and the format line gives way to it. | The format and size choices take effect at once. PNG saves the code with its label. |
| Scanner | A sheet (`scan_sheet`): the title names what is scanned ("Scan a SeedQR", "Scan a share", "Scan a setup code", "Scan a key for {wallet}", "Scan a QR code"), and a scanner a flow opens what the flow asks for ("Scan the PSBT", "Scan the description", "Scan Key {n}'s xpub", "Scan a cosigner's key", "Scan a transaction", "Scan an address"; `docs/SCANNING.md` §7.1); the camera's picture, its corner brackets white while looking and green while a QR that read is in view, a QR the scanner finds outlined where it is in the picture (`found_outline`: warning while found, ok once read, its corners rounded and edged each side in the role's on-colour), the state line and the bar of parts read on a translucent band at its foot (`viewfinder_over`); under it the ways in: **Type the words instead** where the scanner takes a seed, **Paste** (shown only where the shell has a clipboard), and **Cancel**, or **Done** on the Spend tab's intake; a camera chooser where there is more than one. | A code read routes by what it is (`docs/QR.md`): one classifier tells every kind apart (`scan_read`), and a scanner names a kind it does not take by what it is, "A transaction to sign · not used here" (`docs/SCANNING.md` §4). A scanner that takes a seed carries the never-type-a-real-seed line in `WARN`. Ctrl+V is Paste. A payload that is a secret is refused on the way in from the clipboard, except a seed pasted into a scanner that loads one (Home's Scan among them), on the desktop (F75). With no camera the picture says so and the ways in stay live. |
| Read card | The scanner sheet changed in place (`read_card` in `screens/sheets.rs`), not a dialog over it (`docs/SCANNING.md` §5; checked against Material 3's Dialogs and Snackbar, and Common buttons for the actions' emphasis): under the scanner's title, in place of the picture, the fact **Read** with what was read ("A transaction to sign", "A codex32 share") in the warning tone for a secret, the lines that say which one ("Key 73c5da0a", "Share C of NAME · 2 needed", "Example · account 1", "1 input · Savings", "2 of 3 · Native SegWit"), never a secret's value; text that is none of the kinds Faraday reads under them as a masked secret (§4.10), its eye in a `TOUCH` square (Material 3's icon button); at the foot **Scan again** (a text button) and **Cancel**, and at the end the way over, filled ("Add this key", "Sign it", "Use in Lightning", "Add to {vault}", "Unlock {vault}", "Make a vault" over "Not kept · scan it again once the vault is open"); the way over stands on the row above, at the card's foot, where the row does not fit. A kind Faraday takes nowhere has no way over. | The camera is off while it shows; **Scan again** wipes the code and turns it back on in the same scanner; **Cancel**, Back and a lock wipe it. A way over that loads a key is dimmed while a stick is in, with "Remove the stick first" on the card. Nothing is filed until a way over is pressed. The text's eye shows it, the next press hides it, and nothing is copied. Its facts are worked out at the read and kept with the card. |

### 4.10 Secrets

| Kind | Rendering | Behaviour |
|---|---|---|
| Masked secret | A box outlined faintly in `WARN`, bullets in place of the value and the eye (`Icon::Eye`, its tooltip "Show") in a `TOUCH` square at its right end (`secret_with`, `seedqr_box` in vault: a key's words and SeedQR, an entry's password, TOTP secret and notes, a note, a paperkey); while it shows, the value or the SeedQR in a box outlined in `WARN` and the struck eye (`Icon::EyeOff`, "Hide"). | A press or tap on the eye shows it, the next hides it; a hold is no different from a press. It hides again once its page is left: another screen, vault, kind or record, the item's form or another panel in its place, a sheet over it, a lock, the idle lock. The value is made only while it shows. An entry's TOTP secret that reads as TOTP has **Show as QR** under its box: its `otpauth://` URI on the QR sheet marked Secret, with no **PNG** (`docs/QR.md` §7.1). |
| Show and Hide | A **Show** button beside a masked value that becomes **Hide** (BIP-85's value, a vanity counter, a Lightning node key). | Shows the value until Hide. |
| Secret out | A sheet before a secret leaves the session (`secret_out_sheet`): "This is a secret" or "This may be a secret", what it is, who can use it, and the ways out — sealed in a vault, or unprotected on the stick. **Unprotected** opens **Save without encryption?** (`plain_sheet`, F129): where it goes as a plain file and that it is as safe as the place the stick is kept, **Hold to save without encryption**, and **Put it in a vault** with a vault open, else **Cancel**. The same sheet comes before Back up's plan keeps a seed's or a codex32 string's file on a stick at a place, and before Transfer's Receive saves a secret. | Cancel keeps it in memory; on the hold's sheet nothing is saved until the hold is held to its end. |
| Worksheet | A codex32 worksheet as the booklet draws it (`docs/CODEX32.md` §7.5, `faraday/faraday-core/src/codex32/screen.rs`): boxes `codex32_box` on a side by the window's class, numbered above in groups of four, the rows' names in a `CODEX32_LABEL_W` column, the staircase as the ladder works down; given boxes (the data, the header) outlined heavier in `TEXT`, the checksum squares in `CODEX32_CHECKSUM_FILL` and bolded, rows read from a table in `CODEX32_LOOKUP_FILL`, the final row under a rule; the box being filled outlined in the accent. The dice de-biasing sheet beside it: five tracks of pads `CODEX32_PAD`, the first value outlined, the second filled, an equal pair struck in `WARN`, and the tree of 32 leaves with the path from die 1 in the accent. | A press on a cell opens its working on a sheet (`CHART_SHEET_WIDTH` wide; the panel's width on a compact window) with **Done**; nothing is computed to open it. |
| Mask policy | Words, seeds, private keys, passphrases and shares are secrets. | Fingerprints, checksums and addresses are not. A secret is never copied to a clipboard. In Practice (F81, `docs/MODES.md` §6.2) a secret a sample filled is drawn unmasked, and a passphrase or password field holds `faraday` in a code box, unmasked, with no eye, once **Use faraday** in Faraday Demo's host strip (§4.19) is pressed; nothing is drawn inside an empty one, and where it is optional **No passphrase** shows in it as in every mode. |

### 4.11 Records and results

| Kind | Rendering | Behaviour |
|---|---|---|
| Fact | A label over its value, the value wrapped (`faraday_ui::sheet::kv`), or a label and value on one row in a card's table. | One fact per row. |
| Card | A surface set apart by its edge alone, `BORDER`, with `SHAPE_M` corners (`Ui::card`, F39): a wallet's card, a step's summary, Home's tiles. | — |
| Result | A line with its icon in its tone: done in `OK` ("Signed with 73c5da0a"), refused in `ERR`, waiting in `WARN`. | — |
| Warning | A line with the warning glyph in `WARN` (a backup's checks, a spend's cautions); "No warnings" where there are none. | — |
| Dimmed | A control not available now, in its own kind, faint (`Style::off`, `Style::when`): a filled or tonal button keeps its fill in `TEXT` at 12 %, an outlined one its edge; its label in `DIM`. Where a step the person can take would make it available, a short line under it says which (Tools' rows: "Load a key first", "Make a vault first"). | A press does nothing. |
| Toast | Material's snackbar (`Ui::snackbar`): its text in body medium (`TOAST_TEXT`) and inverse-on-surface on inverse-surface in the scheme on screen, at elevation level 3 with `SHAPE_XS` corners, `TOAST_PAD` each side; one line `TOAST_H` tall, two or more where the width does not hold it, `TOAST_PAD_Y` above and below; centred at the foot of the page, `TOAST_MARGIN` from its foot and sides, as wide as the window less those margins on a compact window and else as its text between `TOAST_MIN_W` and `TOAST_MAX_W`, each line of its text centred (F163; Material sets it from its start), standing `TOAST_STACK_GAP` above Home's Scan where Scan is in its way, as Material places a snackbar over a floating action button; rising in and fading after `TOAST_SHOW_MS`, 4 s (`notices`), not rising under Reduce motion: "{what} waits for the stick". The same place holds "Remove the stick to keep working" in `WARN` while a stick stands in the way. | Says what a press did where the screen does not change to show it. |

### 4.12 Text

| Kind | Rendering | Where |
|---|---|---|
| Title | §4.1. | Every page and sheet. |
| Section label | 14 units SemiBold, muted (`faraday_ui::parts::section_label`). | Over a group of rows. |
| Label | Caption or label size, muted, above or left of its value. | Every fact. |
| Value | Body or mono. | Everything a person reads or compares. |
| Walk-through | A short paragraph at the top of a step while it is open, in Guided mode only (`flow::Card`'s `guide`, `guide_text`); Steps only hides it. | Step flows (`docs/WALLETS.md` §6). |
| Explainer | Paragraphs on a Learn page, in the Learn sheet (`learn_sheet`), or on an extra-large window in Learn's supporting pane beside the page (`screens/learn_pane.rs`, `docs/LAYOUT.md` §6): a card in the surface-container role at the right, closed until Learn, the ? or Cryptography is pressed; a header row, as Material's side sheet has, with the headline **Learn** in Title L and at its end a pin (a toggle icon button: the push pin outlined while off, filled in primary while on) and a close button; under it the sheet's tabs across the pane's width (Material's primary tabs, one per page, each label whole: where they do not all fit, the row shows whole tabs only, the one on show among them, with a chevron, Material's standard icon button, at an end that has tabs past it, whose press shows the next one; on the Spend tab under **Spending, step by step** as a text button) and ticked steps, the page's title in Title L and its text in Body M; it follows the page's steps, closes when the person moves to another screen unless pinned, and scrolls within itself. | Never on a working screen outside Guided mode's walk-through and the one tooltip below. |
| Tooltip | Material 3's tooltips (`faraday_ui::tooltip`), drawn over everything once the screen is drawn (`Ui::draw_tip`), above what they are about with `TIP_GAP` between, or below where there is no room above, kept inside the window. Plain: body small (`TIP_PLAIN_TEXT`) in `TIP_PLAIN_INK` (Material's inverse on-surface) on `TIP_PLAIN_FILL` (inverse surface), `SHAPE_XS` corners, at most `TIP_PLAIN_MAX_W` wide, centred on its anchor. Rich: body medium (`TIP_RICH_TEXT`) in `TIP_RICH_INK` (on-surface variant) on `TIP_RICH_FILL` (surface container) at elevation 2, outlined in the high-contrast themes, `SHAPE_M` corners, at most `TIP_RICH_MAX_W` wide, starting where its anchor starts. A screen marks its anchor with `Ui::tip_anchor`; which shows is the pointer's state's (`faraday_ui::tooltip::Tips`). | Shows once the pointer rests on its anchor for `TIP_DELAY_MS`, or the keyboard stays on the anchor's control as long, or, for a rich one only, a finger holds it for `TIP_LONG_PRESS_MS` (then the lift presses nothing); a plain one never shows to a finger. Goes when the pointer leaves, the keyboard moves on, at Escape, or at the next press. A rich tooltip may also open at once on a press of its own icon button, and then stays until the next press or Escape. Used for: Create's Kind row, rich, its one explanation on a working screen (F112), on Material's info icon button (`Icon::Info`, `ICON_BUTTON` round with an `ICON_SMALL` glyph in `MUTED`, a `TOUCH` target that takes the press only on the button) right after "(codex32)", and on the word itself. The plain one is for icon-only controls, and names the action in a word, never explains it ("Back", "Learn", "More", "Lock", "Keyboard", "Power off", "Show" or "Hide" on a field's eye by what a press does, "Remove", "Link", "Warning", "Close"): the top app bar's back, ?, ⋮, Lock and keyboard, the small panel's bar and flow bar (back, ?, keyboard, the codex32 eye), the wide page's ? and keyboard and codex32 eye, Learn's pane's close button, the power buttons, the passphrase fields' eyes, and the backup board's ×, ⋮, link and warning marks (a mark that is no button owns its tooltip by itself, `BoardPress::Mark`). Where a sheet or popup covers the page, the page's tooltips go with its presses (`Ui::clear_hits`). The Starter's top bar marks its back and ⋮ the same way; its host does not yet show them. Checked against Material's guidance as Android's Compose documentation states it (developer.android.com, Tooltip: plain tooltips describe an icon button's element or action, rich tooltips give more detail and may be shown by a press of their anchor) and Material's Icon buttons (a 40 dp container, a 48 dp target); m3.material.io's pages are script-drawn and were not readable. |

The ? on a page with a Learn page opens the Learn sheet at the section
that page is about; on an extra-large window, Learn's supporting pane
beside the page (`docs/LAYOUT.md` §6).

Not built (`docs/CRYPTO-LEARN.md` §8.2, §8.5): on a screen that offers
a cryptographic action, the Learn sheet and pane carry a first tab,
**Cryptography**, before the pages' tabs. It holds a block per
action, the open step's first: the action's name in Title S, then each
field a Material list item, its label the headline in Body L on
on-surface and its value the supporting text in Body M on
on-surface-variant, data in the mono face, each entry of a field its own
paragraph, what is not checked in the Warning role with `Icon::Warning`;
a full-width divider between blocks. The same list at every size; an
item grows with its value, nothing is cut.

### 4.13 Actions

| Kind | Rendering | Behaviour |
|---|---|---|
| Button | `Ui::button` in a `Style`, Material's buttons, round at their ends: `Primary`, filled in the accent, the one action a step leads with; `Tonal`, filled in `SELECTED`, a second action that matters beside the filled one (Back up's **Show the seed** beside **Check my copy**, a vault's **Add another passphrase** beside **Create vault**, **Sign** on a PSBT in Files, whether or not a key is loaded); `Secondary`, outlined in `OUTLINE`; `Ghost`, a text button, its label in the accent; `Selected`, a chosen option, ticked (§4.2). Each kind not available now keeps its shape (§4.11). Hover, press and the keyboard on it lay the label's colour over it at the state layers' opacities, and the keyboard's place is ringed (`Ui::state_layer`, `Ui::focus_ring`). A press is taken on at least `TOUCH` square, the target centred on the button and sharing any gap with a neighbour's (`faraday_ui::Reach`). A row of them shares the width where every label fits its share and stands one under another, full width, where not (`faraday_ui::sheet::buttons`). The on-screen keyboard's keys are buttons with `RADIUS_SMALL` corners (`Ui::button_in`). | A press acts. |
| Hold | An outlined button that fills while held (`hold_button`): "Hold to sign" with a GPG or Secure Boot signing key from a vault, "Hold to delete from vault", "Hold to mark the key revoked", "Hold to save without encryption" (F129), "Hold to add this wallet" on the key path sheet (F132), "Hold to register" on the register sheet (F135), "Hold to pay this fee" on a spend's Review, "Hold to continue" under the device check's and the Starter's Continue anyway, and the Starter's "Hold to write the older release" on Confirm (F136), and on a stick\'s Delete files sheet "Hold to delete 1 file with zeros" (a vault among them), "Hold to zero the free space" and "Hold to zero the whole partition" (F131). | Acts once held to the end; released before, nothing happens. |
| Floating action | Filled in primary over the page, its icon and word on-primary (Material's primary FAB colour style, F138), with `SHAPE_L` corners, its `ICON_GLYPH` icon and its word, at the bottom-right (`Ui::fab`). | Home's **Scan**. |
| Dialog | Material's basic dialog (`faraday_ui::sheet::dialog`): its head, what it holds, and its actions, over the page's frost and scrim, `SHAPE_XL` corners; on the wide layout `DIALOG_MIN` to `DIALOG_MAX` wide with `DIALOG_PAD` inside and its actions at its end as wide as their labels, on the small panel the panel's width less a margin with its actions sharing the width. What is above the actions scrolls when it is taller than the window. `faraday_ui::sheet::dialog_in` centres one over the page right of a sidebar. Drawn with it: the Inbox's seed (`potential_sheet`), the device check, the pull, the lock, lock-ask, write-out, power and key path sheets (`kept_sheet`), **Which account to load?** (`silent::screen::account_dialog`, its two choices alike, Secondary, so neither is the default), the register sheet (a wallet registered from a spend, F135), the Sweep sheet (F171), the older copy an unlock stops at (**Open this copy?**, `vault::load_screen::older_copy`, `docs/VAULT-HARDENING.md` §5.3), BIP-85's **Record it as a child seed?** (`bip85::screen::record_dialog`, its two actions alike, Secondary, so neither is the default; F166) and Create's **A BIP-85 child seed** (`screens::create::child_notice`, its one action **OK**; F166), a new input device, the idle warning (`faraday_ui::sheet::kv_sheet`), Locked, where a secret goes, the SeedQR read, Save to a stick, the GPG signature, codex32's cell working on a wide window (Previous, Next and Done its actions), and the Starter's Restart, Continue anyway and Uninstall. Its actions run dismissive first, confirming last; a hold stays in its body. Not dialogs: the scanner, the QR code, the Learn and word-list readers, which fill the window with what they show; the menus and the other sheets of `faraday_ui::sheet::scroll_sheet`, and `faraday_ui::parts::sheet_box`, take the same corners. | A press beside it is its way out, where it has one. The keyboard reaches its controls, then its actions, in the order drawn; Enter or Space presses the one it is on. |
| Cancel a flow | **Cancel**, an outlined button at the right end of a flow's title row, beside **Steps only · Guided**, a segmented button (`flow::column`); on the small panel in the bar's tools (`compact::tools`). On every flow that can have a sidebar row but a backup flow whose key is made, which ends only by finishing (`docs/FLOWS.md`, Flows in progress, Cancel; F51). | Ends the flow and shows the screen it was opened from. When ending it loses something the person would notice, a sheet first: its question as the title ("Cancel making this wallet?"), one line naming what is lost, **Keep going** (secondary) and **Cancel the flow** (primary); a press beside the sheet keeps going. With nothing done past the first card, no sheet. |

### 4.14 Progress and state

| Kind | Rendering |
|---|---|
| Progress | A count "2 of 5": the Spend tab's signatures, New key's entries. Where work has a count, Material's determinate linear indicator, `PROGRESS_H` tall, its track filled by the share done (`Ui::progress` with `done`): a scan's parts, the Spend tab's ways tried ("Trying … 2 of 5"), Explore's Find, zeroing a stick, the Starter's write and read-back. A TOTP code's row has a bar under it for the time left in its period, the seconds at its end. Long work with no count shows Material's indeterminate linear indicator, a bar crossing its track, still under reduce motion; work that holds the frame until it is done (a vault's unlock, creation and sealing, an unlock from the boot import and the Spend tab) shows it still, along the foot of the line that says what the work is (`working_bar`). |
| Default | A step card (in Faraday a flat section of its flow: no fill and no box, divided from the next by a rule in `LINE`, its number, title and value on its row, `docs/DECISIONS.md` F39) whose value has a default is closed at entry, showing its value and a **Change** text button at the card's right in place of the chevron that opens it. The flow opens on the first card with no default; opening a closed card does not reset the cards after it. A card that asks one thing closes on the choice and opens the next (F103); Back opens the card before the open one, its choices as they were, Forward the card Back left, and the first card's Back leaves the flow (F104). On a small panel the same: the paged flow starts on the first open card, and that page carries at its head one row per earlier card closed on its default, "{title} · {value}" with **Change**, which opens it. Tapping the page's title (the card's name alone, "Kind", with "Step 1 of 8" beside it in the muted caption type) closes the card and shows the list of steps, from which any card opens; no step indicator is drawn. Create a wallet and New key are the exception: each opens on its first card — Kind, Start — whatever opened it, even with a default already ticked (Start has none: no choice is ticked until one is pressed); a card still closed on its default shows its value and **Change** the same way once Continue has passed it (`docs/history/NEW-WALLET.md` §2.1). |
| Empty state | A line and the buttons that start the flow: Wallets with no wallet loaded, "No wallet is loaded" and its ways in (Make a new wallet, Restore an existing wallet, Spend from a backup, Add a key); TOTP codes with no entry, "No entries with a TOTP secret" and Scan a setup code. |
| Locked | The sheet after the idle lock (`locked_sheet`): "Locked", that every key and vault was wiped, what waits for the stick, and Continue. The idle warning before it counts down (`idle_warn_sheet`). |
| Self-test failed | The whole window: "Self-test failed", the check that failed, and Exit (`selftest_failed`). |

### 4.15 Keys

A shell with a keyboard — a laptop booted from the stick, the desktop
shell — takes keys as a desktop app does. Nothing on a screen says so.

| Kind | Behaviour |
|---|---|
| Typing | Characters, Backspace, Delete and Enter go to the field typing goes to, and to the on-screen keyboard's field where it is up; Left, Right, Home and End move the caret, and Up and Down a line in a field of several lines; Backspace, Delete and a character act at the caret (`edit`, `docs/TEXT-CURSOR.md`). |
| Enter | Takes what is typed where the field has one action: a word on word entry, a share or a part, a name, a passphrase. |
| Tab | Finishes a word on a Latin list once only one fits; moves between a passphrase and its repeat. On the backup board, Tab and Shift+Tab move among the chips' ⋮s, ringed in the accent at `FOCUS_STROKE` (§4.17). Everywhere else, Tab and Shift+Tab move the keyboard's place through the controls in the order the frame drew them, round at the ends; the control it is on is ringed (`Ui::focus_ring`), and Enter or Space presses it. A press of the pointer or a finger takes it off. |
| Escape | Closes the open sheet or ⋮ menu (a sheet's Cancel), leaves a field, and goes back from Add a key and Set the time. |
| Up, Down | In a ⋮ menu, move between its rows; elsewhere scroll the region under the pointer, or the page's, by one wheel notch. On a slider the arrows step it (§4.2). |
| Ctrl+V | The scanner's Paste, and nothing anywhere else. |

### 4.16 Structure chart

How a thing is made of others and where each of those is: Faraday's
wallet at a glance (`docs/history/NEW-WALLET.md` §6), the wallet over its keys
over its backup; and the same chart from an open vault (§7.2), the
backup over the keys over the wallet.

| Kind | Rendering | Behaviour |
|---|---|---|
| Node | The summary row's box: `RADIUS`, the page's fill, an inner edge, `CHART_NODE_PAD` inside. A title in the label type, an optional tag at its right in the caption type (secret, sealed, public, in their tones), then caption lines that wrap; a line marked lost or exposed reads "Key 1 words · lost" in `ERR`, struck through by a rule of its colour; a node that offers a step carries a `CHART_BUTTON` button at its foot. | The whole node is the target: it opens the node's sheet. A caption line that stands for one thing (a thing a backup node holds, a key's way in) is a target of its own over it: its sheet, or its way in. |
| Sheet | What a node or a line opens (`docs/history/NEW-WALLET.md` §9). On the wide layout (beside the expanded rail: large windows and wider, not low) it is Material's standard side sheet (Material 3, Side sheets; `faraday_ui::sheet::side_sheet`): `SIDE_SHEET_W` wide (Material's 256 to 400 dp, at its widest) at the window's right edge, from the page's first line under the top app bar (`SUPPORT_PANE_TOP`) to the foot, in the surface colour at no elevation with a divider along its leading edge and no scrim; the page's room narrows by its width, so the chart and its lines stand beside it, never under it (where the room left cannot hold Wallets' list pane and a card at least as wide beside it, the wallet's card stands alone in it until the sheet closes, `docs/LAYOUT.md` §5), and stay live: a press on another node or line changes the sheet, and Where it is draws the key's lines bold with the sheet still open. Its header holds the head and Close, a text button at its end (Faraday's icon face has no close glyph); `SIDE_SHEET_PAD` inside; Back and Close shut it. It slides in from the edge on Material's emphasized decelerate (`motion::SIDE_SHEET_IN_MS`, medium 4) and out on its emphasized accelerate (`SIDE_SHEET_OUT_MS`, short 4), at once under Reduce motion (`docs/MOTION.md`). On the rail's and the bar's windows, where the chart is a column with no lines, it is a sheet `CHART_SHEET_WIDTH` wide centred over the chart, frosting it, and on a small panel the whole panel as a page, as before; there Where it is closes it to show the chart. Its head (icon and title), its facts as label-over-value rows, lines of what follows in the label type in their tones, a field being typed (its label in caption over a `CHART_SHEET_ROW` box with the caret; a passphrase as dots), then its actions as menu rows under optional caption headings, and its own primary button (Confirm, Keep, Check) beside Close (Back on a small panel; in the side sheet's header instead) at its foot. A change asked for shows the plan's three check lines as facts, "before → after", a line made worse in `WARN`. | Each row leads into the flow that does it; opening the sheet does nothing else. A change is made only by its sheet's Confirm (`docs/history/NEW-WALLET.md` §9.7). |
| Row | Nodes side by side at equal widths, between `CHART_NODE_MIN` and `CHART_NODE_MAX`, `CHART_NODE_GAP` apart, centred. A row wider than its column wraps onto further lines at the widths a full line gives; the nodes of one line take its tallest node's height. | — |
| Line | A `CHART_STROKE` run (`CHART_STROKE_BOLD` for a key whose sheet's Where it is shows its lines, every other line then the inner-edge colour) from the middle of a node's bottom edge to its target's top edge, orthogonal: down into the band under the line of nodes, across, down. Where a line of nodes stands between, it runs down the gap between two nodes, or beside them, so it never crosses a node. Each band is `CHART_BAND` above and below its runs, each source's run across `CHART_LANE` from the next; a target's lines come in at points spread along its top. Each source has its colour (`KEY_LINES`, the theme's own colours in turn) and its dash pattern (`CHART_DASHES`, solid first), so the lines are told apart without colour too; the parent's lines are the border colour, solid. | Lines are not targets. |
| Column | On a small panel the rows stack, one node a line, full width, under the rows' headings, and no lines are drawn: a node names what the lines would have joined it to. | — |
| Direction | The rows read top down either way: the parent first (the wallet, its keys, its backup, as the wallet's card draws it), or the parts first (the backup, the keys, the wallet at the foot, as an open vault draws it). Lines always leave an upper node's bottom edge and enter a lower node's top edge. The plan's three checks go above the chart read either way, as a strip: three boxes side by side where each is at least `CHART_NODE_MIN` wide, else one under another, each its line in caption and its value at `TITLE` size in its colour (`OK`, the accent, `WARN`, `ERR`); the backup's map panel and the end of the backup draw the same strip (`docs/history/NEW-WALLET.md` §14.6). The column reverses its rows and headings the same way, the strip staying on top. | — |

### 4.17 Board

Where things are put, by the person: Faraday's backup board
(`docs/BACKUP-FLOW.md` §17), a row for each place and beside them a list
of what can be kept. A thing is a chip; a place is divided into boxes,
and a chip is dropped on a box. The board is the body of a step, on Back
up's plan and on the wallet flow's plan card alike; on Back up it takes
the step's whole width. The narrow column is §19.12 item 3's.

A new plan opens on the board with **The standard way** placed, the
one way a backup starts (`docs/BACKUP-FLOW.md` §17.6, §17.21): each
seed's words on paper with the whole description, a place for each
key, no SeedQR. There is no Start from. The board is read-only until
**Customize backup**; where the list will be (the right-hand column,
or the narrow column's **Add**) stand two secondary buttons, **Add
watch-only software** (**Take off watch-only software** while the
software place is there) and **Customize backup**. A vault box whose
vault is not made yet reads "Vault n · sealed · made in the
checklist", or "Vault n · new" where that does not fit its band.

Three boxes nest, so paper, a stick and a vault are told apart at a
glance (`docs/BACKUP-FLOW.md` §17.14, §17.15): the **place**, the **stick
box** inside it, the **vault box** inside the stick box; Paper stands in
the place beside the sticks. One hue per kind of box, pale fills and
strong edges, the colour carried by edges and header bands and never by
colour alone: each box has its glyph and its label.

| Box | Fill | Edge | Header | Glyph |
|---|---|---|---|---|
| Place | `PLACE_FILL`, a pale green | `PLACE_EDGE`, `BOARD_BOX_EDGE`, and a left stripe `BOARD_PLACE_STRIPE` wide | a `PLACE_BAND` band `BOARD_HEAD` high: the glyph, the name and the × in `PLACE_TEXT`, the balance and how near it brings a spend as outlined tags (`BACKUP-FLOW.md` §17.22) | house |
| Stick | `STICK_FILL`, a cool slate grey | `STICK_EDGE`, `BOARD_STICK_EDGE` | "Stick n" and its × in `STICK_TEXT` | USB stick (SD card on the Pi) |
| Files | `FILES_FILL`, a pale blue | `FILES_EDGE`, `BOARD_BOX_EDGE` | "Files · not sealed" in `FILES_TEXT` | file |
| **+ stick** | `STICK_FILL` | `STICK_EDGE`, dashed (`BOARD_DASH`); solid while a chip it takes is held | "+ stick" in `STICK_TEXT` | |
| Vault | `VAULT_FILL`, a pale accent tint | `VAULT_EDGE`, `BOARD_VAULT_STROKE` | a solid `VAULT_EDGE` band, "Vault n · sealed", its marks and × in `VAULT_ON` | closed lock (`Icon::Lock`) |
| Paper, Scanned in | `PAPER_FILL`, a warm cream | `PAPER_EDGE`, a brown, `BOARD_BOX_EDGE` | "Paper" in `PAPER_TEXT` | file |
| Chip | `SURFACE` (`INNER` while pressed), the shadow of elevation 1 | `CHIP_EDGE`, a neutral a step stronger than `BORDER`, 1 dp | | |
| **+ place** | none | `MUTED`, dashed | "+ place" in `MUTED` | |

Each theme has its own values (`faraday-ui` `pal`, the themes' tables),
by the same rules: a pale, low-chroma fill; an edge at least 3:1 against
the fill round it (a place against the page, a stick and Paper against
the place, a vault and the files box against the stick); all text at least 4.5:1 on its
fill, the vault band's on the band. The light theme's are the approved
mockup's. A chip's edge is not held to 3:1: it stands on its own surface
fill, edge and shadow, and its text is `TEXT` on `SURFACE`.

| Theme | `PLACE_FILL` | `PLACE_BAND` | `PLACE_EDGE` | `PLACE_TEXT` | `STICK_FILL` | `STICK_EDGE` | `STICK_TEXT` | `VAULT_FILL` | `VAULT_EDGE` | `VAULT_ON` | `PAPER_FILL` | `PAPER_EDGE` | `PAPER_TEXT` | `CHIP_EDGE` | `FILES_FILL` | `FILES_EDGE` | `FILES_TEXT` |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Dark | #1E2E30 | #243F38 | #5E9A73 | #86B396 | #2B343E | #7D8B98 | #96A1AC | #2A3E4A | #83D8EE | #0D121A | #343735 | #B89850 | #C8A860 | #697179 | #2E4055 | #538CCD | #8DB0D9 |
| Light | #EDF6EF | #DCEEE1 | #5E9A73 | #1E5B35 | #E3E8ED | #7D8B98 | #36424D | #D3EAF3 | #0B7A9E | #FFFFFF | #FBF5E6 | #977A36 | #6A501A | #AEB9C3 | #E3ECF7 | #4F82BC | #3168A8 |
| Cobalt | #EDF6F0 | #DBEEE1 | #5A946E | #436D52 | #E5E8EA | #74818D | #5B656F | #D1E3EE | #0062A3 | #FFFFFF | #FAF4E6 | #977A36 | #80682E | #AFB2B4 | #E3ECF7 | #5083BD | #3168A8 |
| Nord | #3D4C54 | #3F5857 | #73A785 | #BAD4C3 | #485060 | #9AA5AF | #C2C8CF | #475666 | #88C0D0 | #2E3440 | #515358 | #C1A465 | #DECBA0 | #828793 | #445F7E | #8EB4DE | #CDDDEE |
| Catppuccin | #343E48 | #374C4C | #5E9A73 | #A2C4AE | #404354 | #8995A1 | #AEB7BF | #4A4561 | #CBA6F7 | #1E1E2E | #49464C | #B89850 | #D2B87D | #6F748A | #3D5571 | #7BA6D9 | #B6CDE7 |
| Tokyo Night | #283540 | #2D4445 | #5E9A73 | #92BAA0 | #353A4C | #7D8B98 | #A1ABB5 | #323C59 | #7AA2F7 | #1A1B26 | #3D3D44 | #B89850 | #C9AA63 | #626985 | #364C64 | #689AD3 | #A4C1E1 |
| Gruvbox | #353C35 | #384A3C | #5E9A73 | #9DC1AA | #414142 | #85929E | #AAB4BC | #52472F | #FABD2F | #282828 | #4A443A | #B89850 | #D0B476 | #7C7463 | #374D66 | #6A9BD4 | #A6C2E1 |
| Rosé Pine | #EDF2E5 | #DBEAD8 | #59926D | #426C50 | #E5E4E1 | #727E8A | #59636C | #D8E0DF | #286983 | #FFFAF3 | #FAF0DC | #977A36 | #7F662D | #B2AEB3 | #E3ECF7 | #4C80BB | #3168A8 |
| Bitcoin Orange | #2B2B1F | #2F3C29 | #5E9A73 | #83B193 | #37302D | #7D8B98 | #94A0AB | #483017 | #F7931A | #1C1410 | #403325 | #B89850 | #C8A860 | #786F67 | #2C3D52 | #4E88CB | #89AED8 |
| Bitcoin Orange Light | #EDF4EA | #DBEBDC | #59926D | #426C50 | #E5E5E5 | #73808C | #59636C | #FEE9D0 | #B46B13 | #000000 | #FAF2E0 | #977A36 | #7F662D | #BBB5AE | #E3ECF7 | #4D80BB | #3168A8 |
| Lamplight | #EDF2E4 | #DBEAD7 | #58916C | #416A4F | #E5E4E0 | #727E8A | #59636C | #EEDBCD | #A34E26 | #FFFAF2 | #FAF0DB | #977A36 | #7F662D | #B6AFA6 | #E3ECF7 | #4D80BB | #3168A8 |
| High contrast dark | #0A1A10 | #0F2B1A | #4CD27A | #9AF0B5 | #141414 | #B0B0B0 | #D0D0D0 | #2A2A00 | #FFFF00 | #000000 | #1E1600 | #FFC24D | #FFD98A | #B0B0B0 | #001A33 | #5CA8FF | #A8D0FF |
| High contrast light | #EAF7EE | #D2EFDA | #1B6B3A | #0F4D28 | #ECECEC | #4D4D4D | #262626 | #DCDCFF | #0000CC | #FFFFFF | #FFF3D6 | #7A5200 | #5C3D00 | #595959 | #DCEBFF | #0B4F9E | #0A3D7A |
| Mars | #2B2418 | #2F3A27 | #5E9A73 | #83B193 | #332B27 | #7D8B98 | #94A0AB | #4A2418 | #D9603B | #120D0B | #3F3123 | #B89850 | #C8A860 | #78706A | #2C3D52 | #4E88CB | #89AED8 |

| Kind | Rendering | Behaviour |
|---|---|---|
| Place | One row the main column's width, the places one under another `BOARD_PLACE_GAP` apart (`docs/BACKUP-FLOW.md` §17.15): the place box at `RADIUS`, its header band `BOARD_HEAD` high (the house glyph, the place's name in the label type, semibold, its line of fact as a tag `BOARD_TAG` high, outlined and written in the tone the chart gives it, `glance::alone_color`, on `SURFACE`, and a drawn × at its right, `BOARD_MARK`, `BOARD_MARK_STROKE`); under it, `BOARD_PAD` inside, left to right: each stick box, **+ stick** (`BOARD_ADD_STICK` wide) and Paper. The sticks and Paper share the width equally; when they would be narrower than `BOARD_BOX_MIN` each, Paper keeps `BOARD_BOX_MIN` and the sticks wrap onto further lines inside the place, as many to a line as fit. A row is as tall as its tallest box, every box drawn that tall (a wrapped line of sticks as tall as its tallest). After the last place, **+ place**, a row `BOARD_HEAD` high; the software place last. In the narrow column (`compact`) a place's parts stand one under another at the column's width, **+ stick** a row a chip high, under **Add**. | The name is a field only while a vault is open (a press focuses it with the whole name selected and no caret, an accent rule under it; the first key replaces it, an arrow, Home or End lets the selection go, §19.18); otherwise it is text. The × removes the place with what it keeps; with one place left it is not drawn. **+ place** adds an empty place. A place is itself a drop target for a vault's chip. |
| Zone | Paper and Scanned in: a box at `GAP` radius in the paper colours, `BOARD_PAD` inside: its glyph and label in caption, semibold, on a `BOARD_ZONE_LABEL` row, then its chips laid left to right and wrapping, `BOARD_CHIP_GAP` apart; an empty zone keeps a chip's height. **+ stick** is the dashed box with its label alone. A stick box is headed by its stick pill's number, one numbering across the places, from 2 (F146): "Stick 2", "Stick 3"; the software place's stick of public files is one too, with no × of its own and no **+ stick** beside it. | A press on a zone places the chip held. A zone is a drop target only for what it can keep in some form (`docs/BACKUP-FLOW.md` §17.3). |
| Stick box | A box at `GAP` radius in the stick colours, `BOARD_STICK_PAD` inside. Its head, `BOARD_STICK_HEAD` high: the USB stick glyph (`Glyph::Stick`, Faraday's own drawing: the icon face has none; the SD card's and "SD card n" on the Pi) and "Stick n" in caption, semibold, and its × at the right. Under the head the files box (`docs/BACKUP-FLOW.md` §17.17), on every stick, laid as Paper is (its glyph and label on a `BOARD_ZONE_LABEL` row, then its chips; empty, a chip's height), then, `BOARD_STICK_PAD` under it, the vault box when the stick carries one. An empty stick a press on **+ stick** added (§19.16) is the same box with its files box alone. | A press or a drop on the files box, or on the stick box outside the vault box, keeps the thing on the stick as a file. Its × takes the stick off with all it carries; after the lock it is drawn only on a stick that carries a vault and no file. |
| Vault box | A box at `GAP` radius inside the stick box in the vault colours, `BOARD_VAULT_PAD` inside. Its head, `BOARD_VAULT_HEAD` high, is the solid band, and the vault's chip: the closed lock, "Vault n · sealed" in the label type, semibold, its link and warning marks, its × at the right, all in `VAULT_ON`; hovered, focused or pressed, Material's state layer in `VAULT_ON` over it in the band's shape. Under the head the records it holds, chips laid as a zone's, each with the lock mark. | A press or a drop on the vault box seals the thing in the vault. The head selects the vault (a press on another place moves its stick); its × takes the vault's stick off this place. |
| Chip | A pill `BOARD_CHIP` high, as the table above draws it, `BOARD_PAD` at each end: its glyph where it has one (the fingerprint before a seed, the passphrase glyph before a passphrase, the closed lock before a vault; the description and a share have none), its name (a fingerprint in mono at the label size, anything else in the text face), then its forms: on the read-only board (before **Customize backup**) after a middle dot in caption, muted ("9a6a2580 · Words · SeedQR"); once customised, where the zone has more than one form for it, as **Form toggles**. Marks at its right, from the edge in: its ⋮ after a thin rule (once customised; **Chip's ⋮**), the × of a placed chip, the warning mark, the link mark, the lock mark (a placed chip in a vault: the lock glyph in `VAULT_EDGE`), each `BOARD_MARK` with `BOARD_CHIP_GAP` before it. A chip whose forms do not fit beside its name drops them to a second line under it (a caption line, or a row of toggles `BOARD_TAG` high). | One chip per thing per zone; its forms are what it carries there. |
| Chip in the list | The same pill, one per row, its count of places at the row's right in caption, muted ("2 places", "1 place", nothing at none); A vault's chip carries its × at the row's right. A cosigner's seed is outlined, not filled, its name dim, "Not here" for its count. A vault no place keeps a stick of is filled as any chip, its name and glyph dim (§19.16). On `wide` the list is the right-hand column where Back up's Map is when the board is not shown (`side_panel_w`, full height, `SURFACE` with a rule at its left), scrolling on its own with its overlay scrollbar, the list from its top, grouped under caption labels: Seeds, Passphrases, Wallet description (Whole, then **Keys left off each share** as a − n + stepper of buttons a chip high, then each share), Vaults with **+ new vault**, Watch-only software. | Never runs out: placing it places a copy. A cosigner's seed takes no press. A vault's × takes it off the plan; one that holds anything first shows what it holds in `WARN` under it, with **Take off**. |
| Add sheet | In the narrow column only, once the board is customised: **Add**, a secondary button a chip high at the top of the board, opens the list as a sheet (the small panel's full-width sheet, its head the add glyph and "Add", the list's groups, each chip with its ⋮, a way out at its foot). | A press on a chip in the sheet, or on its ⋮, opens its **⋮ menu** (Add to…) over the sheet; a box chosen places it and closes both (`docs/BACKUP-FLOW.md` §17.20 item 4). Nothing is dragged there. |
| Placed chip | The chip in its zone, with its × and its ⋮ once the board is customised. A chip of what an existing vault held before is outlined and dim and has no × and no ⋮. | A press selects it. The × takes it off. |
| Chip's ⋮ | Once the board is customised (`docs/BACKUP-FLOW.md` §17.20), at the right end of every chip that can move (in the list, placed, a vault's band): a rule 1 dp wide inset `GAP_SMALL` top and bottom in the ⋮'s colour at `BOARD_MENU_RULE`, then three dots `BOARD_MENU_DOT` across, spread over `BOARD_MARK`, in a part `BOARD_MARK` plus `BOARD_PAD` each side wide; `MUTED` on a chip (`DIM` while it is dragged or dim), `VAULT_ON` on a vault's band; `INNER` behind while pressed or hovered. The × and the other marks stand before the rule; a placed chip's toggles before them. With the keyboard on it, edged in the accent at `FOCUS_STROKE` (on a vault's band in `VAULT_ON`). Not drawn on the read-only board. | Its own press target: a press opens the chip's ⋮ menu and neither selects the chip nor starts its drag. Tab and Shift+Tab move the keyboard among the ⋮s on screen in the order drawn; Enter or Space opens the menu. |
| ⋮ menu | On `wide` a popup `BOARD_POPUP_WIDTH` wide under the ⋮, its right edge on the ⋮'s (above it where it does not fit below; scrolling where it fits neither), on `SURFACE` with `BORDER` and the shadow of elevation 2, no backdrop; on the small panel the full-width sheet with **Cancel** at its foot, its head the add glyph (Add to…) or the chip's glyph. Inside: its title in caption, semibold ("Add to…", "Move to…", "Copy to…", or the chip's name), each place's name in caption, semibold, `MUTED`, and under it the boxes, a row `BOARD_MENU_ROW` high each, the label type, set in `BOARD_PAD`: "Paper", "Stick 2 · files", "Stick 2 · Vault 1", "New stick"; then the software place's. A placed chip's first level: **Move to…**, **Copy to…**, **Take off**. The row the arrows are on is `INNER`, edged in the accent at `FOCUS_STROKE`. | Lists only the boxes that take the item in some form and do not keep it already. A box chosen places it as a drop there does (the checks, the balance sheet), and closes the menu; Move to… moves it with its forms, Copy to… keeps it in both (not for a share), Take off is its ×. Arrows move, Enter chooses, Escape goes back from Move to… or Copy to… to the first level and else closes it, the keyboard back on the ⋮; a press outside closes it. |
| Form toggles | Once the board is customised (`docs/BACKUP-FLOW.md` §17.3), on a placed chip whose zone has more than one form for it, after its name: a pill `BOARD_TAG` high per form, `BOARD_PAD` at each end, `BOARD_CHIP_GAP` apart, at `SURFACE`; the check glyph `BOARD_MARK` in the accent and its name in caption, `TEXT`, with the accent edge, where the form is kept; the glyph's room left empty, the name `MUTED` and the line edge, where it is not ("◉ 9a6a2580 [✓ Words] [✓ SeedQR] ×"). Where they do not fit beside the name, a row under it. There is no forms popup. | A tap turns the form on or off; the last form on stays on. A press on the chip's name selects it, as before. |
| Customize backup | `docs/BACKUP-FLOW.md` §17.6: while the board is read-only, a secondary button a chip high at the top of the right-hand column in place of the list, and in the narrow column in place of **Add**. Read-only, the board draws no × on places, sticks or chips, no **+ place** or **+ stick**, and chips read their forms as text. | A press opens the board for changes: the list (or **Add**) in its place, the board editable, for this plan from then on. |
| Balance sheet | `docs/BACKUP-FLOW.md` §17.5: the scrolling sheet the board's Add sheet is (its width on `wide`, the panel's less `GAP` each side in the narrow column): its head the eye glyph and "<Place> can see the wallet's balance", one line in the label type naming what there makes the description, then **Take it off** (secondary) and **Keep it** (primary). | Keep it closes it, the drop standing; Take it off undoes the drop. A press beside it is Keep it. |
| Partial descriptions | `docs/BACKUP-FLOW.md` §17.19: under the list's Whole, while the description splits, **Add partial descriptions to this backup** (or **Remove partial descriptions** while shares are kept), a secondary button a chip high, then **What is a share?**. No share chip is in the list and there is no keys-left-off stepper. | A press hands one share to every place (Faraday chooses which, and the keys each leaves off) and opens the **Partials sheet**; while on, a place added or taken off has them handed out again; Remove takes every share off. |
| Partials sheet | The Balance sheet's form: its head the file glyph and "Each place now holds a part of the description.", then, where the whole description is still kept, the line naming those places, then **Take it off every place** (secondary, only where taking it off leaves any M places able to rebuild it) and **Keep it** (primary). | Take it off every place removes the whole description from every place's paper and sticks; Keep it closes. |
| One place per key | The board's **+ place** is not drawn once the plan has one place per key of the wallet (`docs/BACKUP-FLOW.md` §17.19); a single key has one place, or two when its seed has a passphrase (the second for the passphrase alone). With a single place, "Any one place lost" reads "No: a single place" in `MUTED`, not as a warning, and the "every secret at one place" line is not shown for a single key. | None. |
| Extra copies line | On the backup's last card (Back up's done card, the wallet flow's Done), in Guided only: "More copies can be made once the wallet is made: …" in 13 regular `MUTED`, then **Extra copies**, a ghost button that opens that Learn page. | Hidden in Steps only. |
| Selected chip | Edged in the accent, `BOARD_EDGE` round its pill. | A press on a zone that takes it places it there (a placed chip moves); a press on it again, or on anything that is not the board, lets it go. |
| Dragged chip | A copy of the chip follows the pointer, centred under it, with the shadow of elevation 4 and the accent's edge; the chip it came from is dimmed where it stands. | A press is a drag once it moves past the scroll slop, and a tap otherwise. The lift drops it on the zone under it that takes it; anywhere else it goes back. |
| Checks under the board | Under the places, **+ place** and the software place, on `wide` and in the narrow column: the plan's three checks as the Map panel drew them (Any one place lost, One place found: can spend, One place found: sees the balance, each with its value, `glance::check_lines`), then the warning lines, each the warning glyph and its line in `WARN`. The top of the board is kept free. | The checks follow every drop on the next frame. |
| Edged drop target | The zone, the stick box, the vault box, the place or the list column edged in the accent, `BOARD_EDGE`, at its own radius. | While a chip is selected, every zone that takes it is edged; while one is dragged, only the one under the pointer that takes it. A zone that does not take it is never edged. |
| Window drop target | In Faraday Demo, the page beside the navigation edged in the accent, `BOARD_EDGE` (the whole window over a sheet). | While a file from this computer is dragged over the window, where the drop is taken (Transfer sends it); gone when it is dropped or the drag leaves. |
| Link mark | Two links of a chain drawn as stroked boxes, in the accent. | On a vault's chip whose sticks are at more than one place: what it holds is drawn at each, and a drop into it at one is in it at all. |
| Warning mark | The warning glyph in `WARN`, and the chip's pill edged in `WARN`. | On a placed chip that one of the plan's checks names. The checks themselves are lines under the board, the warning glyph before each, in `WARN`. |

### 4.18 The "Demo only" card

Replaced by Faraday Demo's host strip (§4.19, F118): what a page or
a sheet offers only in Practice stands outside the app, under "Practice" in the
strip, never in a card beside the page (owner, 2026-10-25). A page hands
each item over where it would have drawn it (`faraday_ui::demo`).

### 4.19 Faraday Demo's host strip

Faraday Demo holds the app in a host (`docs/DESKTOP-HOST.md`): the
app area is the device's screen; what only Faraday Demo or its Practice mode has
stands in a strip at the window's foot, in the host's own look: the
app's theme turned inside out (Material's inverse roles, and the
opposite mode's scheme for what stands on them), never the app's
surface or its components.

| Part | Looks | Behaves |
|---|---|---|
| The strip | `HOST_STRIP_H` (64 dp: its 48 dp controls with 8 dp above and below) along the window's foot, set by the window alone, whatever preview the app is shown at; in the app's inverse surface, so a light app has a dark strip and a dark app a light one in every theme, mode and contrast level; taller, to the roller's row, only while the page shows the computer's die. | The app is laid out in the window above it, or framed at a preview size centred above it; the strip is never inside the frame. Its row is centred in the window; where even folded it is wider than the window less its margins, it starts at the margin and scrolls sideways under the wheel or a finger, a scroll indicator along its foot. |
| Its controls | Outlined, `HOST_CONTROL_H` (48 dp) tall, their press their whole box, labels in Material's title medium (`HOST_LABEL`, 16 sp) in the app's inverse on-surface, outlines in the host scheme's outline, Material's state layers hovered and pressed; a control whose menu is open filled in the app's inverse primary; one that takes no press now at the disabled opacities. | Left to right, the always-there controls: the stick pills, **Samples** (Practice), **Size · <size>**; then the screen's practice aids in a section of their own, a tertiary container of the host scheme with its on-tertiary-container text: Practice, the aids, those with no room in **More**, and the lines saying what Practice puts in place of the person's own, cut to one line; the step's main press (codex32's Next, Roll) filled in tertiary. |
| Its menus | Material's menu, `HOST_MENU_W` (280 dp), rows `HOST_MENU_ROW` (48 dp) with title-medium labels, on the host scheme's surface container with an outline-variant edge, the chosen row checked; opening up from its control. | In on the emphasized decelerate (medium 1), out on the emphasized accelerate (short 3), at once under Reduce motion; a choice acts and closes it; a press beside it closes it and does nothing else. Size, Sticks, Play options (codex32's walk: how far Play runs, how fast), Rolls per click, More. Narrow, the strip gives way: the sticks fold, Size loses its size's name, the aids go into More (the step's main press last), Samples joins More, and narrowest More reads "Practice" (`docs/DESKTOP-HOST.md` §3). |
| The dice roller | The die (§4.3) in the host scheme, first among the aids in their section, the strip growing to its height while it shows; then the strip's **Roll** and **Rolls per click · N**. | As in §4.3; Rolls per click opens its menu. |

## 5. The screens

A screen is one of the thirty-four variants of `Screen` in
`faraday/faraday-core/src/lib.rs`, drawn in the file that file's code
map names: an area's `screen.rs`, or `screens/<name>.rs` for a screen
with no area. Each is drawn in one of three ways:

- **Step flow**: under the top app bar, which names the flow and carries
  its way back and **Cancel**, the step-card column (`flow::column`): its
  chip and **Steps only · Guided**, then one card open at a time, the others closed to a line that says what
  was chosen (§4.14, Default). On the small panel, a page per step with
  the bar's tools (§4.1).
- **Page**: under the top app bar, which names it, what the screen holds,
  scrolling under the bar where it is taller than the window.
- **Sheet**: a box over the page, which is frosted and dimmed behind it
  (`Ui::backdrop`, `faraday_ui::parts::sheet_box`); its head is its
  icon and title (`faraday_ui::sheet::sheet_head`), its buttons at its
  foot. On the small panel it is the panel's width less a margin, and a
  long one scrolls (`faraday_ui::sheet::scroll_sheet`). A press beside
  it closes it where closing loses nothing.

What each flow does, step by step, is `docs/FLOWS.md`; the Wallets tab
is `docs/WALLETS.md` and the Spend tab `docs/SPEND-FROM-BACKUP.md`.

| `Screen` | What it holds | Drawn as |
|---|---|---|
| `Home` | The lead tile while something is under way or waiting, then the tiles; on the small panel the menu of the sidebar's entries; **Scan** (`docs/FLOWS.md`, Home). | Page |
| `WalletsStart` | The Wallets tab's first page: the wallets loaded, the keys without a wallet, and every way in by what the person has. | Page |
| `Wallets` | The wallets as a list and the chosen wallet's card: its name and shape, a `tr()` tree's key path line under the shape (the danger line, in the error colour, for a key not of the wallet; F132), the backup line, the signatures possible here, **At a glance** (§4.16), the first receive address and the card's actions (`docs/WALLETS.md`). | Page |
| `Spend` | Signing a transaction: the review (what it sends, to whom, the fee, change, the txid, warnings), the signers, **Sign with** (§4.4), and the signed PSBT and finished transaction. | Step flow |
| `Family` | The Spend tab: spending from a backup, step by step (`docs/SPEND-FROM-BACKUP.md`). | Step flow |
| `Files` | The Inbox and the Outbox: a row per file with its kind, size and ⋮, and what each file can do (Sign, Decode, Nonce check, Load the wallet, Unlock, Import into the vault). | Page |
| `Visit` | A stick visit: what is on the stick, ticked to bring in, and what goes out to it. | Page |
| `Entry` | Add a key: a seed typed or scanned (§4.3). | Page |
| `Backup` | Backing up a wallet: the plan, the board (§4.17), each copy made and checked. | Step flow |
| `BackupFlow` | The shared backup flow for a key that is not a wallet's: where it will be kept, its sticks and vaults, the lock and the writes (`docs/BACKUP-FLOW.md`); a GPG key restored from its paperkey, with a head card of its public key and line 1 and the lines typed or a paperkey code scanned at step 4 (`docs/GPG-PAPERKEY.md` §4.2); also the setup of a new wallet's vaults, Bring in the sticks and Vaults, with Make a new wallet's Guided switch. | Step flow |
| `Backups` | Every wallet known and where its backup is, a card each; on the small panel one wallet a page with **Previous · Next**. | Page |
| `Create` | Making a new wallet: Kind, the quorum, whether vaults are part of the backup, the keys, Check with **Back up wallet** at its foot (no Back up card). | Step flow |
| `Restore` | Restoring a wallet from its description and its seeds. | Step flow |
| `KeyGen` | Making a new key: Start (no choice ticked until one is pressed; SLIP-39 shares are chosen here, under Other forms), the length (BIP-39 words chosen, no count until one is pressed: 12 words and 24 words, the rest under **Other**, a menu of 15, 18 and 21; a count pressed is taken and Randomness opens, with no Continue), the source of randomness, its entries (§4.3) or, for This device, the Generator card with Add randomness's sources on it (Pointer and keys among them on the desktop app), Camera first and none chosen until one is pressed (no pad, key or camera live before that), each pad and its Add once chosen, "Randomness added to the pool" once any is added, and **Skip**, a text button at the start of the foot row while nothing is added, **Make the words** filled at its end, the passphrase, lock in; or, with Codex32 chosen, Make codex32 shares' cards (`docs/CODEX32.md` §6): Scheme, Identifier, Entropy, the entry with the worksheets filling live (§4.10 Worksheet), Worksheets, Strings and the Result **Strings made**. | Step flow |
| `Message` | Signing a message with a single-key wallet's key. | Step flow |
| `CheckMessage` | Checking a signed message against its address. | Page |
| `Decode` | A transaction taken apart, field by field. | Page |
| `NonceCheck` | A signed PSBT no key here is left to sign, or a finished transaction, brought in by the camera, Files or Tools: the file's name, Send it's nonce check of it, whether it is finished, and **Save to a stick**, **Decode it**, **Show the raw hex** and **Show as QR** (`docs/FLOWS.md`, Nonce check). | Page |
| `Explore` | A loaded key at a path, opened from its row under Keys: the path field, the script, the address, public key and extended key; the account's addresses and Find an address (F76). Behind their eyes, on the stick and on mainnet too, each one press to show and one to hide and all closed when Explore opens: the key's secret (its words in their numbered grid and its passphrase, or its master seed in hex with its codex32 secret and the strings held), the root's and the account's extended private keys, and an address's private key (WIF) under the address and under each address on the Addresses page; leaving Explore, the idle warning and a lock hide them all (F125, `docs/KEYS.md` §3). | Page |
| `Bip85` | BIP-85's applications from a loaded key: a child seed's words, a private key, a password, at an index. | Step flow |
| `Silent` | Silent payments: a loaded key's address, its labels and the payments checked. | Step flow |
| `Vanity` | A vanity address for a loaded key: the script, the prefix, the dial, the grind and its find. | Step flow |
| `Lightning` | A Lightning node's key, from a loaded key or an LND aezeed; its node id with **Show as QR** and **PNG** on a row of their own (`docs/QR.md` §7). | Page |
| `Catalog` | Tools: every flow on one page, by group, each with its line and the standard it follows; the sidebar's Tools. | Page |
| `Tools` | The calculators: hashes, encodings, a descriptor's checksum, an xpub's spellings, units, the Miniscript compiler. | Page |
| `Vaults` | The vaults in Files and those open. | Page |
| `Unlock` | Unlocking one slot of a vault with its passphrase. | Page |
| `VaultContents` | An open vault: its kinds, the chosen kind's items (under a search bar for Wallets, Entries and GPG keys, §4.3; each title whole, wrapped where it does not fit), the chosen item, with the add actions; a wallet's item is its chart read parts first (§4.16); an item's secrets masked, each behind its eye (§4.10). On the small panel the kinds are a row of tabs over the list, and each keeps where its list was scrolled to (owner, 2026-10-26). | Page |
| `CreateVault` | Making a vault: where it will be opened, the cost of unlocking, the space per passphrase, its name and passphrases, its size. | Step flow |
| `Codes` | TOTP codes for the entries in open vaults, with the time they are computed for, under a search bar (§4.3, `docs/TOTP.md` §5). | Page |
| `SetTime` | Setting the time by hand: Date and Time (`docs/TOTP.md` §3). | Page |
| `Randomness` | Add randomness, from Settings or from the Randomness row at a press that draws the system's randomness (§4, Randomness row), its top app bar going back there: the status line, the label **Add randomness to the pool** over the source (Camera, Dice, Coins, Cards and, on the desktop app, Pointer and keys), none chosen and nothing started until one is pressed, and New key's pad for it, the batch and its Add, or Pointer and keys' area and its count; once a batch is added, a check mark and "Added to the pool 3 times · last: 6 dice rolls", a pressed state layer easing away behind it at each batch (`docs/OWN-RANDOMNESS.md` §2.1, §2.2). New key's Generator card draws the same sources on itself. | Page |
| `Settings` | **Steps only · Guided**, the flows' control and setting; then Mode; the session (the keys loaded) and Lock; lock and power-off timers; the time; Randomness (the status line and Add randomness, `docs/OWN-RANDOMNESS.md` §2.4); Appearance (Theme as filter chips, one per theme, Cobalt's marked Default; Mode, Light · Dark; Contrast, Standard · Medium · High; Text size, Standard · Large · Larger, `docs/TEXT-SIZE.md` §2; the Reduce motion switch, THEMES §12); Display (Scale, and on a 7-inch or 10-inch panel Rotation, Landscape · Portrait, `docs/ORIENTATION.md` §6); what is remembered; About. With Guided on, each card carries a one- or two-sentence explainer under its title, beside the Guided text's accent bar (what Lock, Remove every key and the timers do; what reads the clock; how added randomness is mixed; what Appearance, Display and Signed amounts set, and under Rotation what each way suits; when the self-test runs); with Steps only, labels, values and actions only. On a compact window a list of the sections, each its own page; from medium up every card on one page (`docs/LAYOUT.md` §17). | Page |
| `Transfer` | The desktop app as the device's QR link: files out as codes, in by camera. Only in the desktop app. | Page |

The sheets (`Sheet` in `faraday/faraday-core/src/lib.rs`, drawn in
`faraday/faraday-core/src/screens/sheets.rs` and the area that owns
them) include the QR sheet and the scanner (§4.9), the Learn sheet, the word list (a tab per list, Back, and at the start of the row under them a search field that finds a word by its word, number, bits or dice and lists only what it finds), a secret on its
way out (§4.10), the lock and its question, the idle warning and the
locked sheet (§4.14), power, a new input device (a dialog; for a new USB
touchscreen the whole window instead: its title, name and **Tap the
target**, **Tries left** once a tap missed, the target, a filled icon
button, `ICON_BUTTON` round in a `TOUCH` target, at the place drawn for
it between them and **Ignore this device** at the bottom end, F153),
what is attached that
can capture the screen or the keys, a seed read by the camera, a chart
node's sheet (§4.16; a standard side sheet beside the chart on the wide
layout, the one sheet that leaves the page under it live), the board's list, its balance and partials sheets
and its ⋮ menu (§4.17), a file's ⋮ menu (§4.1), Rename and Details, New
key's Other menu of word counts (drawn as a ⋮ menu is, under its button), the
mode sheet, Load samples (§4.8), Restore a GPG key (the public keys in Files
that a GPG key is restored against, and **Scan the public key**, `docs/GPG-PAPERKEY.md` §4.1), the key path sheet (a wallet a key not
of its own spends alone, its danger line, Cancel and **Hold to add this
wallet**; F132), **Which account to load?** (`Account`: a Coldcard export
holding a single-sig account and a silent payments account, the file's
name, **Cancel**, **Single-sig · BIP-84** and **Silent payments ·
BIP-352**, the two alike; `docs/INTEROP.md` §5 I9), the register sheet (the wallet a spend's PSBT defines,
its shape, cosigners and name, Cancel and **Hold to register**; F135),
the older-copy dialog an unlock stops at (the vault's name, what its seal
count found, **Keep locked** and **Open**; `docs/VAULT-HARDENING.md`
§5.3), **Record it as a child seed?** (`ChildRecord`: BIP-85 child words
held before Save into a vault or Load as a key, what each answer keeps,
**Don't record** and **Record**, alike; a press beside it or Escape puts
the words nowhere; F166), **A BIP-85 child seed** (`ChildNotice`: a
wallet just made with a key recorded as a child seed, that Faraday
records it only inside a vault and that nothing written for its backup
says so, **OK**; once per wallet made; F166), the Sweep sheet (a WIF
private key typed or scanned: the key's field, then its addresses with
their QR codes, then its PSBT's review and **Hold to sign**, **Done**
erasing the key; `docs/INTEROP.md` §5 I11, F171), and the question a
flow's Cancel asks (§4.13).

In Faraday (`faraday-ui`) every page's working area is one colour, the
theme's surface (`SURFACE`), and so is a sheet; the sidebar keeps its
own shade (`SIDEBAR`). A step flow's steps are flat sections divided by
a thin rule in `LINE`, with no fill and no box. The right-hand column
(Back up's list or Map, the side panel, `side_panel_w`) is the same
colour, divided from the page by the same rule. A card or tile on the
page is set apart by its edge alone, `BORDER` (`docs/DECISIONS.md` F39).

## 6. What changes from today

Moved to `docs/history/DESIGN-LOG.md`.

## 7. Open questions for the owner

Moved to `docs/history/DESIGN-LOG.md`.

## 8. How this gets built

Moved to `docs/history/DESIGN-LOG.md`.
