# Design system

**Kind:** Spec
**Status:** Built but for the items under Open
**Open:** the Guide's Cryptography tab and its row of the components (`docs/CRYPTO-LEARN.md` §8), not built

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
   Guide and About, which are not working screens, and behind the Spend
   tab's **More about this**, closed until pressed. Text longer than a
   label, wherever it appears, is plain statement of fact in an
   encyclopedia voice, never metaphor or flourish in place of direct
   statement.
2. **One kind of content, one rendering.** Each content kind in §4 is
   drawn the way its row says, by what the row names.
3. **One screen per kind of data.** Seed words, a PIN, a QR code, an
   address, a secret string, a derivation path each have one screen that
   every flow reuses. The flow changes the title and the action, not the
   layout.
4. **One item per row on touch screens.** Options are a list, one per
   row. The one exception is a pair of short options (Receive | Change),
   which sit side by side as two equal buttons.
5. **Fixed geometry for the keyboards.** An on-screen keyboard is the
   same size in the same place every time it is up: the docked keyboard
   at the foot of the panel or of the page beside the sidebar, word
   entry's keys under the word grid (§4.3).
6. **One placement law.** Every screen has one intent, and one law
   places it. A screen navigates, chooses, enters, helps understand,
   transcribes, shows to a camera or reads (§5). Actions and the controls
   the thumb works (buttons, keyboards, pads, toggles, pairs) are
   bottom-anchored; lists and documents start at the top and scroll;
   everything else is one block, vertically centred in the space between
   the app bar and the actions. Nothing else is top-anchored, and no
   block is placed by how tall it happens to be. A block that does not
   fit pages or scrolls. The same law holds on `wide` inside the width
   caps, with the primary action bottom-right at its natural width.
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
    at least 48 × 48 dp, 44 × 44 dp on the compact and short layout
    (the 2.8-inch, F188), larger than the control where
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
| Navigation bar | A compact window that is not short. | One column under the top app bar, the 80 dp navigation bar at the foot with the five destinations, Check not among them (§4.1, F255); Guide, Settings and Lock in the top app bar; Home leads with the status strip. |
| Navigation rail | A medium or expanded window, and a low one that is not compact and short (the 7-inch panel). | The 80 dp rail at the left with the six destinations (Check under Home, F255), Guide and Settings under a divider and Lock at its foot; one column at most `COLUMN_MAX` (720) wide, centred beside it, under the top app bar (§4.1); Home leads with the status strip. |
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
  extra-large windows: Guide beside the page, the backup board's lists,
  the codex32 dice sheet's column (LAYOUT §6).
- **What stands at the right** (the top strip's mode pill and ?, the
  eye, the keyboard button, the supporting pane, the Scan button) stands
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
- **Touch targets**: `TOUCH`, 48, and `TOUCH_COMPACT_SHORT`, 44, on the compact and short layout (principle 12, F188), held by the touch-target test over the snapshot tours (`docs/M3-CHECKS.md` §2; its exceptions §3, F152).
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
at a scrolled edge (on the small panel the bottom fade is shallower than the top, so a text button resting at the fold reads at its own emphasis), and the scrollbar at full strength; a colour asked
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
| Expanded rail (wide, the sidebar) | 240 units at the left (`sidebar`): the wordmark, FARADAY OFFLINE or FARADAY DEMO (F165); the tabs Home, Check (a shield with a tick, `Icon::Verified`, F255), Wallets, Files, Vaults, Tools, Guide, then Transfer in the desktop app, and Settings, each one of Material's destinations (`faraday_ui::nav::destination_row`: icon and label in a row 48 units tall, the current one on an accent pill, Files with its badge as the rail has it and, while files wait for the stick, "{n} to write" at its right end); under its tab a row for each flow in progress with its open step at the right; the loaded keys under Keys and the wallets under Wallets, each row 48 units tall, the two groups one region that scrolls within the sidebar where they are taller than its room, the tabs and the foot staying put (Material 3 Navigation drawer: its content scrolls); the sticks attached, a row that stands only while a stick is attached or an import waits, so that a loaded key's and wallet's rows keep their room at 1280 by 800 (with the idle lock's pill too); Lock and Power off (Quit in Faraday Demo), each a labelled button, side by side while something is held. | A tab opens its screen; Guide opens the Guide sheet, or on an extra-large window Guide's supporting pane beside the page, and pressed again while the pane is shown puts it away (`docs/LAYOUT.md` §6); a flow's row resumes it; a key opens Explore on it; a wallet opens its card; the wheel, a trackpad or a finger over the lists scrolls them; Files is the one tab that opens Files (`docs/FLOWS.md`, Sidebar). |
| Top strip (desktop and Starter windows) | The top `TOP_BAND` (56) units of the desktop app's window, taller where a strip of stick pills is (a device's backup flow on the one-column layouts, and Faraday Demo held bare there with `--panel`), and the Starter's top app bar: its empty room, where no control or touch floor lies, is the window's handle (`Faraday::is_strip_room`, `faraday_window::drag`). | A press there and a drag move the window (`Window::drag_window`); a double-click maximises it or restores it; a press without movement does nothing. |
| Way back | The top app bar's back arrow, with where it lands under the page's name as "Back to Tools", "Back to Kind" (F146), on every page that has one, the step flows' included (`flow::Column`'s `back`, `screens::bar_head`). | It lands on the screen seen before this one, as it was seen, and is named after it (F77). In a flow with steps it opens the step before the open one, its choices as they were, and is named after that step; only the first step's lands on the screen before, and a running step takes none (F104, `StepFlow` in `faraday/faraday-core/src/trail.rs`). The mouse's back button does the same (`Ui::back`), and its forward button opens again the step or screen it left. |
| Bar (the menu's windows) | `compact::BAR_H`, Material's small top app bar at `APP_BAR_H` (64), across the top of every page but Home and a step flow on the small panel; a step flow keeps it on every one-column layout (`compact::bar`, `faraday_ui::nav::app_bar`): the chevron in a 48 dp target, the page's name in Title L, or Title M where Title L would be cut beside the tools and the pill (F139, where Faraday differs from Material's small top app bar), with where it goes back to under it in Label M, "Back to Wallets", or the name alone where that would be cut (F146), and at the right end the tools (`compact::tools`), each in a 48 dp target: **Cancel**, Material's text button, on a flow in progress (on the compact layouts it is the ⋮'s first row instead, below), the eye on Make codex32 shares' sheets, the keyboard button where the page types, and the ? where the page has a Guide page. The mode pill, off mainnet (Practice's reading "Practice" alone; `docs/MODES.md` §3), is not among them: it stands at the end of the bar's first line, centred on the page's name's line, its right edge a gap left of the tools (`compact::tools`, `faraday_ui::nav::pill_top`), F246. The name has the first line up to the pill: Title L where it fits, else Title M, else it wraps at a word onto the second line in Title M (the next smaller role where two lines would not fit `APP_BAR_H`), and where it wraps it takes both lines and "Back to" is left off, the chevron's tooltip still saying where it goes; the name is shortened at a word with an ellipsis only where one word does not fit a line (owner, 2026-11-11, replacing F223's second line; a difference from Material's one-line title, F246). A step flow draws its own top and the same tools (`compact::flow_bar_tools`). On the Pi's 268 dp they make room as Material's top app bar does, never running under the chevron's 48 dp target: the page's name steps down to Title M, is shortened at a word with an ellipsis only where that is cut too ("Make a new…"), and is left off where not even its first word would show whole (never "Cre…"; the step's own heading names the page), and where they would still reach the chevron the ?, then the keyboard button, then the eye go into Material's overflow menu, a ⋮ at the bar's end (`compact::tools`), as the pill takes room on the first; no two of their targets overlap (owner, 2026-10-26, 2026-10-27; `docs/LAYOUT.md` §8). On the Pi's panel the ⋮ is on every page but Home, ending with **Lock** (while something is held) and **Power off**, labelled; Home's foot carries the same two, each with its label (`compact::power_in_menu`). On the compact layouts only (the small panel and a window under 600 dp wide, 480x640 among them), a flow's **Cancel** is not on the bar: it is the ⋮'s first row on every page that can be cancelled, the ⋮ drawn there for it, so the bar has one leave control, the chevron, and the page's name has the room Cancel took, stepping down to Title M or shortening only as above (`compact::cancel_in_menu`; owner, 2026-11-11, replacing the 2026-10-26 and 2026-10-27 call that kept Cancel on the bar; Material's small top app bar, one navigation icon, action icons and the rest in the overflow). Wider layouts keep Cancel on the bar. On a step whose press draws the system's randomness the ⋮ is drawn on the Pi's panel even where the tools fit, with the ? and How in it, and **Add randomness** is its first row (`docs/OWN-RANDOMNESS.md` §8.3, F157). | The chevron goes back as the back link does. The ⋮ opens its menu (on the small panel a sheet, as every menu there): **Add randomness** opens Add randomness as the Randomness row does, **Guide** opens the page's Guide as the ? does, **Cancel** (compact layouts) does what Cancel on the bar does, **Lock** (on the Pi's panel, while something is held) and **Power off** (on the Pi's panel) do what Home's foot buttons do, **Show keyboard** or **Hide keyboard** and **Show sheets** or **Hide sheets** do what their buttons do. |
| Randomness row | `screens::randomness::row`, directly above every press that draws the system's randomness (`docs/OWN-RANDOMNESS.md` §8.1, F157): Material 3's two-line list item (Lists), the headline **Randomness** in Body L on-surface, the status line ("System: getrandom(2) · your own: 99 dice rolls") in Body M on-surface-variant, wrapping under the headline where the column is narrow, and as its trailing element Material's text button **Add randomness** in primary, in a 48 dp target; at least `LIST_2` (72) tall. On the Pi's panel the status line alone in Label M, cut at a count's comma with an ellipsis, never in a number, and Add randomness the first row of the bar's ⋮ (Bar). The Lock sheet keeps the whole row on every panel. Shown at Dealing SLIP-39 shares, codex32's Suggest, Create vault, Seal and export, a GPG or Secure Boot key's Make, Enrolment files, Hold to sign, Seed XOR's parts, Make the new set, a MuSig2 round's Sign here and Collect, and a lock that seals a changed vault (the Lock sheet, and the open vault's screen while it holds changes). Never blocks the press. | **Add randomness** opens Add randomness with no source chosen and nothing started; Back lands on the step as it was left, its sheet over it again. |
| Nothing to work on | `screens::parts::nothing_to_work_on`: a tool that works on a file from Files and has none. An outlined card (Material 3's Cards) with the line that says so in Title S on-surface and, under it on the same card, **Scan** (filled; left out where the tool's input cannot come by QR, such as a wallet file, and then **Open Files** is the filled button), **Open Files** and, while a stick is attached and no key is open, **Bring in from {stick}** (outlined buttons, Common buttons), wrapping where the card is narrow. With files waiting, the tool shows them as a list (Lists): a row each, the file's name and a chevron, a touch target tall (`file_choices`). | **Scan** opens the scanner for what the tool reads; **Open Files** opens Files; **Bring in from {stick}** opens the stick visit, which returns to the tool when the stick is pulled; a list row opens the tool on that file. |
| Cryptography (not built; `docs/CRYPTO-LEARN.md` §8) | On a page or step flow whose screen offers a cryptographic action of the manifest, Material's standard icon button in a 48 dp target immediately left of the ?: `Icon::Method`, Material Symbols' `fact_check` in its filled form baked into the icon face, its plain tooltip "How Faraday does it", no selected state. The bar's tools then read Cancel, the eye, the keyboard button, Load samples, the mode pill, **How Faraday does it**, the ?, Lock and the ⋮. On a compact bar too full for it, the Pi's panel among them, it is the first to go into the overflow menu, before the ?, and its row there is the menu's first, "How Faraday does it" with its icon. In a flow it shows from the step that offers the action and on the steps after it that carry its result. The Starter's top app bar carries the same button left of the ⋮ on What to write, Confirm, Writing and Done, and on Start this computer from the stick while the BitLocker recovery key is offered. | It opens Guide as the ? does, on its **How Faraday does it** tab: the Guide sheet below extra-large, the supporting pane at extra-large, where pressed again while the pane shows that tab it puts the pane away and pressed while the pane shows another tab it moves to that tab. In the Starter it opens a sheet of the blocks, with **Close** at its foot. |
| Title | Every page is named by its top app bar, Home and the step flows too; while Make codex32 shares' **Characters** card is open and a character is still to come, the bar's title is where the entry stands, "Share A · character 7 of 26", "Share A · 7 of 26" where the whole would be cut (the Pi's panel), so it stays in view as the sheets scroll under the bar; where neither fits beside the bar's tools the card reads it in place of **Characters** (F146); on the wide layout Home's bar reads **Home**, the sidebar's head holding the wordmark. A step flow's column keeps at its head what is the flow's: the chip where the flow names what it works on at the left (`flow::column`); its **Cancel** is on the bar (on the compact layouts, the ⋮'s first row). | The step count sits beside a step's name as muted text (§4.14), never in the title. |
| Home | A search bar under the buttons on every window but the small panel's menu (Material's Search bar; `docs/LAYOUT.md` §4); something typed shows the matching tools under it, the buttons staying and nothing above the bar moving (with nothing loaded the three first actions are one row of heading-only buttons, stacked on the phone-size window; on a window too short to leave room under the bar the whole page scrolls as one); where the layout is one column, a press on the bar brings the on-screen keyboard up. On the wide layout: the lead tile while something is under way or waiting, then half-width tiles two to a row (`docs/FLOWS.md`, Home). On the one-column layouts: the menu, a button for each of the sidebar's entries in the rail's order (Check, Wallets, Files, Vaults, Tools, then Guide and Settings; on the small panel's menu Wallets comes before Check; Backups after Wallets once a wallet is known), led on the small panel by its mark and name and then the box of what is loaded, and on the rail's and the bar's windows by the box alone; on the small panel a summary card stands under the box only while vaults are open; Lock and Power off at its foot on the small panel only. **Scan** floats at the bottom-right on both (`Ui::fab`), and on the top page of Check, Wallets, Files, Vaults and Tools except on the small panel's menu layout. | A tile or a button opens its screen; Scan opens the scanner. |
| Pager | **Previous · Next** at the foot of Backups on the small panel, one wallet a page. | For runs of same-kind pages only. |
| Navigation rail | `RAIL_W` (80) at the window's left in `SIDEBAR` (`faraday_ui::nav::rail`, `nav/screen.rs`): Home, Check, Wallets, Files, Vaults and Tools from the top (F255), then Transfer while it applies; Guide and Settings under a divider; Lock while something is held and Power off (Quit in Faraday Demo) at its foot. Each is its icon on a `NAV_PILL_W` × `NAV_PILL_H` pill (accent while current) over its caption label, `NAV_ITEM_H` (56) tall, never under 48 on a low window; Files carries the Inbox's count, or a dot while a stick waits (§4.8, Destination badge). Not drawn for a step that must not be left (a stick being written, a backup flow's key being made; F51). | A press opens the destination; Guide opens the Guide sheet; Lock and Power off as on Home. |
| Navigation bar | `NAV_BAR_H` (80) across the window's foot (`faraday_ui::nav::bar`): Home, Wallets, Files, Vaults and Tools, evenly spaced (Check is not one: Material 3 gives the bar three to five destinations, and Home's Check tile opens it; F255) and drawn as the rail draws them; never changed for a stick, and Transfer in place of Tools while it applies (Tools stays on Home). Not drawn while the keyboard is up, nor for a step that must not be left. | As the rail. |
| Top app bar | `APP_BAR_H` (64) across the page beside the rail or above the bar, and beside the expanded rail over every page, from the rail to the window's edge, its title the page's own (the name and way back the page hands over as it is drawn, `screens::bar_head`; Files' stick chip left of its actions) (`faraday_ui::nav::app_bar`): on Home the mark and the wordmark on the rail's and the bar's windows (FARADAY OFFLINE on the stick and the Pi, FARADAY DEMO on Faraday Demo, F165; the second word under FARADAY where the two do not fit side by side, as on the Pi's panel beside the mode pill; `docs/LAYOUT.md` §3), and beside the expanded rail, whose head holds the wordmark, the name **Home**; elsewhere the back chevron, the page's name, and under it where the chevron goes in Label M, "Back to Tools" (F146); at the right end, beside the expanded rail, a device's stick pills while a backup flow uses sticks, left of the rest, so the page under the bar does not move as they come and go (**Stick pills**, below; `stick::pills::bar`), the keyboard button where the page types, a warning mark named Wireless keyboard on every page while a wireless receiver other than Logitech's Bolt is attached (it stays after the reminder is understood), Load samples in Practice, the mode pill off mainnet, the ? where the page has a Guide page, and on the bar's windows Lock while something is held and the ⋮ menu (Settings, Power off; a sheet). Nothing stands above it: the wordmark is at the window's top, level with the rail's first destination. Under it, a chip for each flow in progress (`faraday_ui::nav::status_chip`). Over a step flow it carries the flow's name and way back and, at its right end, **Cancel** while the flow can be left and the eye on Make codex32 shares' sheets (`compact::bar_cancel`, `compact::bar_eye`), as the one-column layouts' bar does. | The chevron goes back as the back link does (F77); a flow's chip resumes it. |
| Stick pills (Faraday Demo) | In the host strip at the window's foot, outside the app (§4.19, F118): the boot stick's pill, then Stick 2, 3 and on, each with its dot (green while in, grey while out, an amber edge while the flow asks for it) and "Stick 2 · in" or "out", in the host's look. As many as fit beside the rest of the strip; the rest fold into **Sticks · N**, which opens the host's Sticks menu of every stick, in or out. Held bare (`--panel`), the app draws them as a device does: beside the expanded rail in the top app bar, left of its actions; on the one-column layouts in a strip above its bar, which an open sheet covers with the page, dimmed and out of reach. | A stick's pill, or its row in the menu, inserts the stick or pulls it (the shell's folder); a row closes the menu. |
| Sticks menu | Material's menu under its pill (`menu::draw`; a sheet on a compact window): "Sticks", then every stick with its state, "Stick 4 · in". It unrolls from its top edge on Material's emphasized decelerate (`motion::MENU_IN_MS`) and rolls up on its emphasized accelerate (`MENU_OUT_MS`); at once under Reduce motion. | A stick chosen inserts or pulls it and closes the menu; a press beside it, Escape or its pill closes it. |
| Box of what is loaded | At the top of Home beside the rail, above the bar and, under the mark and name, on the small panel (`nav::screen::loaded_card`): one outlined card (Material 3's outlined card, `SurfaceContainerLow`, `OutlineVariant`, `SHAPE_M`), a plain list-item line in it for each item, its icon and no outline of its own, `LOADED_LINE` (48) tall, a touch target on the small panel: each flow in progress, its name and "in progress"; each key, fingerprint and name (four, then "+ N more"); each wallet by its name (the same); Files, its count and what it is ("0 files · no stick", "3 files from the stick", "2 files to write to the stick"); an import waiting. | Each line opens what its sidebar row opens: a flow resumes, a key opens Explore, a wallet its card, Files opens Files. |
| Navigation rail (Starter, medium and expanded) | 80 units at the window's left in `SIDEBAR`: the destinations Home, Make, Tools and Guide from the top, each its icon over its caption label, the current one on an accent pill behind the icon (`docs/STARTER.md` §3.1d, F91). Not drawn on a page inside a task. | A press opens the destination as it was last seen; a press on the current one goes to its top. |
| Navigation bar (Starter, compact) | 80 units across the window's foot: the same four destinations, evenly spaced, drawn as the rail draws them. Not drawn on a page inside a task. | As the rail. |
| Top app bar (Starter) | 64 units across the page: the wordmark on Home, a destination's name on the others, a back arrow and the page's name on a page inside a task; at the right end Install or Update while it applies (secondary) and the ⋮ menu (About, Uninstall while installed, Remove the boot entry while offered); on Linux, last, the window's close button (not built; `docs/STARTER.md` §3.1d). It stays while the page scrolls under it. | The back arrow lands on the destination it was opened from, as it was seen (F77). The close button does what the window's own close does. |

### 4.2 Choice

| Kind | Rendering | Behaviour |
|---|---|---|
| Option row | One rounded box 48 units tall per option: its name at the left and its line at the right, the line under the name on the small panel; the chosen one in Material's selected state, secondary-container with its words in on-secondary-container (`kind_rows`). The everyday options first, the rest behind **More kinds**, a ghost button: on Create's Kind, Single key, then Multisig, none ticked at entry, and on a single key's Backup card, Words (BIP-39) and Shares (codex32), none ticked at entry (F240, `docs/CODEX32.md` §6.9). | A press chooses it; on a card that asks only this, it moves on to the next card, and a press on the one chosen does too (F103). Used for: Create's and Restore's Kind, Create's Backup card, Create a vault's size. |
| Segmented button | One choice among two to five, side by side in one outlined container, round at its ends, the chosen segment in `SELECTED` with a tick (`Ui::segmented`); where the options do not fit the width, option buttons instead (`Ui::choices`). The chosen one is never drawn as the screen's filled button. | A press chooses it. Used for: Settings' Lock after, Power off after, Mode and Contrast (`SEGMENTED_H`), Display's Scale and Rotation, What is remembered's two rows; Create's path choice on Quorum (§4.6); the QR sheet's format, speed and size; Settings' Contrast and Rotation rows. |
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
| Text field | A box with `FIELD_RADIUS` corners and an `OUTLINE` edge (the accent while typing goes to it), what is typed in it and the caret where typing goes (`Ui::typed`, `Ui::caret`); what is selected is drawn on secondary-container behind it, in on-secondary-container (`Ui::selection_band`). A passphrase field shows dots, and **No passphrase** while it is empty (`pass_field`, `CoreUi::no_passphrase`); a field that may hold a secret has an eye at its right end (`secret_box`). | A press places the caret between the two characters nearest it and a drag selects, in every field, a masked one too (`docs/TEXT-CURSOR.md`); a field of several lines notes each line (`Ui::typed_lines`); Sign a message's field grows to its whole message, and in it and on that flow's Sign step and Check a message a control, zero-width or line-break character other than a newline is drawn as a mark, ◌ or ¶; a grid of words typed takes the caret across its cells, and a press on a cell puts it there. The eye shows the text in the clear until it is pressed again. A field that takes no typing while a stick is attached says so in place of its content. |
| Search bar | Material's search bar (m3.material.io, Search): `SEARCH_BAR_H` (56 dp) tall, fully rounded, `SurfaceContainerHigh`, the magnifying glass (`Icon::SEARCH`) leading at `SEARCH_INSET`, the placeholder ("Search entries") in `OnSurfaceVariant` at the body size while it is empty, what is typed and the caret otherwise, the accent edge while typing goes to it, and a × in a 48 dp target at its end while it holds text (`vault/screen/search.rs`). Over an open vault's Wallets, Entries and GPG keys and over the TOTP codes (`docs/VAULT.md` §12), and under Home's buttons (`screens/catalog.rs`, `home_search_bar`: typing lands in it with no press first; it shows its blinking caret while empty once it is pressed (on the one-column layouts a press on it also brings the keyboard up), and its hover layer takes the bar's rounded shape), nowhere else. | Filters the list under it as it is typed, by the title, case not counted; nothing found says so in a line ("No titles match", "No codes match"). The × empties it. On a compact window a press opens Material's search view: the bar at the page's top, what it finds under it, the keyboard under them; Done leaves it with the list filtered in place. |
| Keyboard | The on-screen keyboard docked at the foot of the panel or of the page beside the sidebar (`faraday_ui::keyboard::keyboard`): the path keyboard on Explore, the address keyboard on Explore's Find, the passphrase keyboard everywhere else (`KeyboardKind`). | On the one-column layouts it rises by itself under a field that takes typing, and on a press on a bar that takes typing with no field to focus (Home's search bar, Find a tool, Explore's path), put away by its keyboard button or by leaving the page; on the wide layout only from the keyboard button. Its keys type into the field typing goes to, as a physical keyboard does (§4.15). On the small panel, when it rises, and when focus moves to another field while it is up, the page's region scrolls only as far as brings the field, with a label drawn above it (`Ui::field_label`), within the region, `KEYBOARD_CLEAR` clear of the keyboard's top and of the region's top (`compact::osk_follow`, after the pass that drew the field, which then draws again), and again when that field grows or moves in the page, as a line wraps; where the region is too short for the field and both clears, the clears give way so the field stands whole above the keyboard; put away, nothing moves. A search bar opens Material's search view instead. |
| Word keyboard | Word entry's own keys, in the page under the word grid (`word_keys`): the BIP-39 letters for a Latin list, and for the others the kana, jamo, pinyin or 注音 keyboard (`KeyboardKind::Kana`, `Jamo`, `Pinyin`, `Zhuyin`). A key that leads to no word on the list is dimmed. | Types the list being entered; the word is taken from the candidates. |
| Kana keyboard | The 五十音 as a grid: ten columns of the consonant groups あかさたなはまやらわ, five rows of the vowels あいうえお, every key one unit wide. The cells the grid leaves empty carry ゛, ゜ and 小; the backspace takes the bottom-right cell, ん the one above it. | Types the Japanese list. ゛and ゜ add the voicing marks the list is written with; 小 turns the last kana into its small form and back. か and ゛ read as が. |
| Jamo keyboard | The two-set layout on the QWERTY positions: ten, nine, and seven keys with shift at the left and the backspace at the right of the last row. | Types the Korean list. Shift gives the five doubled consonants and the two shifted vowels and releases after one key. The keys run through the two-set automaton, so ㄱㅏ reads as 가 and ㄱㅏㄱ as 각. |
| Pinyin keyboard | The BIP-39 letter rows, and a fourth row of the five tone keys 1 2 3 4 5 with the backspace ending it. | Types the Simplified Chinese list, looked up by a Mandarin reading: the syllable on the letters (`v` for `ü`), then its tone. A letter is live while some candidate's syllable continues with it; a tone key is live in the tones that syllable has. The candidates fill only once the tone is typed. |
| 注音 keyboard | The 大千 layout, four rows on the QWERTY positions, the tone marks ˊ ˇ ˋ ˙ in the first row and ˉ ending the second, the backspace ending the last row. | Types the Traditional Chinese list. Every tone is a mark key, ˉ for the first; the mark completes the reading and the candidates fill only then. |
| Word grid | One numbered cell per word of the count, the count's buttons (12 · 15 · 18 · 21 · 24) above it, the word being typed in its cell (`typed_cells`, `keyed_cells`); a status line under it states the words and the checksum. | Each word is checked against the list as it is typed; Tab finishes a word on a Latin list once only one fits. |
| Candidates | The words the keys typed can still become, a suggestion chip each (`SHAPE_S` corners, an `OUTLINE` edge) in the mono face, wrapping at the width (`keyed_pills`, `Ui::word_pill`). | A press on a pill takes the word. |
| Dice, coin, card keys | A row of buttons in New key's randomness step: 1 to 6 for a die, Heads and Tails for a coin, and the ranks and suits for cards, with the count so far, "{} of {}", and the words a run has made (keygen). A face a procedure does not take next is `Style::Disabled`. On a small panel a chosen rank keeps its check, so the ranks wrap into rows as wide as a chosen one needs, all alike. | Each press is one entry. |
| Dice roller | Faraday Demo in Practice only (F96, `docs/CODEX32.md` §5.6; owner, 2026-10-24): in Faraday Demo's host strip (§4.19), never beside the dice pad's entries: one die (`ROLLER_DIE`, pips drawn, outlined in the accent while it tumbles), then the strip's own **Roll** and **Rolls per click · 1**, which opens a menu of 1 · 5 · 10 · To the end · Number. The die is drawn by `fresh/roller/screen.rs`, its sizes and timings `ROLLER_*` tokens. The codex32 dice step's roller (`docs/CODEX32.md` §5.5, `fresh/roller/five_screen.rs`) is the same with five dice labelled Die 1 to Die 5 and **One click rolls**: One pass · One character · One share · To the end. | **Roll** lands all the click's values on the entries as presses, in the frame that shows the click; the one die then tumbles for Material's short 4, skipped under Reduce motion (MOTION); the five dice tumble and pause between values as before. Values are marked as this computer's: the result's entropy line says "simulated dice". |
| Code box | A value whose exact characters must be plain, in the mono face on a tinted, rounded box padded by tokens, with no quotes (`Ui::code_box`; `Ui::code_line` puts one after a line of text, below it where it does not fit). Used for: the Practice passphrase. | Not a field: it takes no press. |
| Error line | One line under the field or the grid in `ERR`: the codec's or the list's reason ("No word starts with that"). | Replaces the status line while it stands. |
| Use a sample | In Practice, with samples set (F81, `docs/MODES.md` §6.1), a button in Faraday Demo's host strip (§4.19) while the field where a key or an input is typed is empty; nothing is drawn inside the field, whose caret and placeholder stand as in every mode. Labelled "Use a sample", or by what it puts in: **Fill words** on Check my copy, the scanner's **Load sample seed**, **share**, **key**, **wallet**, **PSBT**, **SeedQR**, **code**, **address**, and **Load sample PSBT** where a step asks for the transaction. | Puts the next sample not loaded in the field as typed; a part of a set fills the next part of the set begun. Gone once the person types in the field or a sample fills it; Clear or the next part brings it back. Never drawn outside Practice. |

A seed is typed on Add a key (`Screen::Entry`): **Scan a SeedQR** above
the words, then the word grid, the candidates and the word keyboard,
then the passphrase field and **Add a key**. A line in `WARN` under it says
never to type a seed that holds real bitcoin (`never_real_seed`); the
line has its own room one gap under the field, and what follows (New
key's **Show words**, Make a new wallet's passphrase note) starts under
it.

### 4.4 Identity and keys

| Kind | Rendering | Behaviour |
|---|---|---|
| Fingerprint | Eight hex characters in mono, never split (`fp_text`). | The key's name everywhere: the sidebar's Keys, a wallet's keys, "Sign with 73c5da0a", a board chip. |
| Key in the sidebar | The key's fingerprint and what it is ("passphrase" where it has one) under Keys (`sidebar`); a key recorded as a BIP-85 child seed and named by its fingerprint alone reads as the child seed it is, "BIP-85 Words · 73c5da0a · index 0" (`wallet::key_name`, F166), as do Explore's and BIP-85's key rows and its vault record's line. | A press opens Explore on that key. A key has no page of its own: BIP-85, a vanity address and a backup are Tools and wallet flows (F68). |
| Sign with | A primary button in the Spend tab's signing step, "Sign with" and the fingerprints of the loaded keys that sign this transaction, beside the count of inputs (`step_body`); once signed, "Signed with" and the fingerprints in `OK`. | A press signs every input those keys can sign; a press before the earlier steps are done opens the first step not done and signs nothing. Where no loaded key signs, the step says so and has no button. A PSBT already complete (signed elsewhere, or made complete by Collect) is gone through here too: its primary reads **Review** and opens the first of Transaction and Check not done ("Finish step {n} first"), and the Finish step shows that line in place of the two result cards, until Review (and Check, where the wallet is known) is done for that exact PSBT; a Collect answer that changes the PSBT opens Review again. |
| Key block | The cosigners of a wallet of more than one key (`wallet::keycheck`), one entry each, a gap (`GAP`) apart: the number (**Key 2**) with "fingerprint · path" (`kv`, §4.11), the extended public key whole, mono and wrapped, and one state row: an `ICON_GLYPH_BUTTON` mark at the left and the state in Body M beside it. **Yours, can sign** (`Icon::Done`, `OK`; "Yours, can sign · your single-key account" at a single-key account path; **Yours, signs after its backup** while that key's backup is not done); **Checked by you** (`Icon::Done`, `OK`); **Not checked** (an empty checkbox, `OnSurfaceVariant`); and, only in Check a multisig setup, a key that claims a loaded key's fingerprint and is not that key: `Icon::Error` and "The fingerprint is 73c5da0a's; the key at m/48h/0h/0h/2h is not" in `ERR`. Under the entries one line in Body M with `GAP_SMALL` under it: "2 keys not checked · compare each with its own device" ("1 key not checked · compare it with its own device") in `OnSurfaceVariant`, or **Every key checked** in `OK`, and no line while a red mark stands and no key is not checked. A seed given to Check a multisig setup adds "This seed" in `OK` under its key. A wallet of one key has no block: its key row shows its state only where it is Yours or red. | A press anywhere on a **Not checked** or **Checked by you** entry (`press_box` over the whole entry, `Action::KeyTick`) ticks it as compared with that signer's own device, and another press unticks it; a **Yours** or red entry takes no press. A tick is the person's own word: nothing is written to the key or the wallet's descriptor, and it is kept in the session and in an open vault's record of the wallet (`docs/FLOWS.md`, Check a multisig setup). A key a loaded seed then proves is Yours, and one the seed no longer holds goes back to Not checked. Drawn on the wallet's sheet on the chart (under the Descriptor row and the first addresses), in the Spend review (§5), and as the keys of Check a multisig setup's Result and the confirmation sheet's body (§5). While a key is not checked, the wallet's chart node carries "2 keys not checked" in `LABEL_M`, `Warning`, and its receive addresses (the Wallets card's first address, the receive QR sheet) carry "2 keys of this wallet are not checked" in `LABEL_L`, `Warning` ("1 key of this wallet is not checked"). |

### 4.5 Long strings

| Kind | Rendering | Behaviour |
|---|---|---|
| Short form | The first eight characters in two groups of four, an ellipsis and the last four, "bc1q 8f3x … k2pz" (`faraday_ui::short`); a string of sixteen characters or fewer is shown whole, in groups of four. | Inside a line that names a string rather than shows it to be read: the Spend review's outputs ("to bc1q 8f3x … k2pz"), its txid and its inputs, the first address on Restore's card and on the Silent payments screen. It opens nothing. |
| Whole string | Whole, mono, in groups of four (`faraday_ui::grouped`), wrapped where it does not fit, never cut. It wraps only at the space between two groups, never inside one, so every full line holds the same number of groups and the last holds the rest: a group is read as one unit, as the other device's screen shows it. As a fact (§4.11, `kv`) its label stands above it in the label's type ("Address"), the string under it in body medium mono. On the 240x320 panel, the narrowest, a line inside a card holds at least four groups at Standard text size, so a native SegWit address is three lines and a Taproot one four; where the width holds fewer, the type does not shrink and the lines grow. | Where the string is what a person reads or checks: a wallet card's first receive address, a vanity find, a decoded output, a signed message's address, an image's SHA-256, the address on Check an address's result (§5). The Key explorer's addresses are whole and wrapped too (F76). A receive address (a wallet's card, Create's and Restore's Check, the chart's wallet sheet, the Spend tab's pages), a vanity find and Explore's address, public key and extended key open their QR sheet when pressed: a chevron at the row's end, the row a touch target tall with its text in its middle (`address_press`, `row_press`; `docs/QR.md` §7). A change address, and an address held back for a backup, open nothing. |
| Descriptor | A summary row, not the descriptor's text: "Descriptor" above; below, mono and wrapped, the wallet's shape ("2 of 3 · native SegWit multisig"), its keys' fingerprints, and " · #" and the checksum where the wallet did not come from a record; chevron (`descriptor_row`). | On Create, Restore, the key-making flow and a wallet's sheet on the chart. The row opens the wallet's QR sheet, which carries the descriptor's whole text under the code. |

Faraday has no screen that shows one string alone (F68): a string is
whole where it is read.

### 4.6 Derivation paths and script types

| Kind | Rendering | Behaviour |
|---|---|---|
| Path | A mono string, `m/84h/0h/0h/0/0`. As a row: "Path" above, the paths below, wrapped, with **Change** at the right while the wallet can be taken back (`path_row`, Create's Check). | **Change** goes back to the Account row (F98). |
| Path choice | Create's path for a multisig that is a descriptor, above the Account row on Quorum: the path as a value (**BIP-48**, or **m/45'** for legacy multisig, the first chosen at entry and whenever the kind changes) with a small **Change**, as the Account row has it; once Change is pressed, a Segmented button (§4.2, `Ui::choices`) of two, the kind's own path and **BIP-87** (`path_choice`). Checked against Material 3's Segmented buttons. The made wallet shows the choice made, as text with no Change. | **Change** opens the two; a press on one chooses it; the Account row follows (a legacy multisig has one only at BIP-87). |
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
| Chip | A rounded label with a dot, in a tone (`Ui::chip`): beside a step flow's heading, where the flow goes next or what it works on (`flow::Column`'s `chip`). A status tag is one in a status's container colour with its on-container colour on it (`Ui::tag`, `faraday_ui::Status`). Its press is taken on the touch floor square round it, whatever its height (`docs/LAYOUT.md` §2). |
| Pill | A rounded label with an edge (`Ui::pill`, `Ui::pill_in`): a stick's pill in its own tone (`Ui::stick_pill`, F142): filled in its tone while the stick is in, its label white or black on it; edged in its tone while the flow asks for it; otherwise quiet. The HOTP and Unreadable badges, in place of a TOTP code on Codes and on a vault entry, are the same shape. |
| Step badge | A step's number in a circle, ticked once done (`Ui::badge`; a guide step's, `faraday_ui::parts::step_circle`, filled in `OK` with its number in on-ok once ticked). |
| Pips | A threshold as `n` dots, `filled` of them filled (`Ui::pips`). |
| Destination badge | Material's badge on a destination's icon in the rail, the bar and the expanded rail, in `ERR` (`faraday_ui::nav::badge`): on Files, the large badge, `BADGE_LARGE` tall, with how many files the Inbox holds ("999+" past 999); otherwise, on Files, the small badge, a `BADGE_SMALL` dot, while a stick attached has not been visited since it arrived. None at nothing waiting. |

### 4.9 Codes

| Kind | Rendering | Behaviour |
|---|---|---|
| QR sheet | A sheet (`qr_sheet`): the title, an animated code's frame "{} · frame {} of {}", whether scanning it is safe — "Secret · whoever scans this can use it" in `ERR` or "Public · safe to scan" in `OK` — the code on a white card with its quiet zone (`faraday_ui::parts::qr_code`), a descriptor's whole text under it; then the format (UR · BBQr · Parts, Parts for text only), an animated code's speed (2/s · 3/s · 5/s) and how much each frame holds (Small · Medium · Large), **PNG** for one code of public content, and **Done**. On the small panel the code is as large as the panel allows and the format line gives way to it. | The format and size choices take effect at once. PNG saves the code with its label. |
| Scanner | A sheet (`scan_sheet`): the title names what is scanned ("Scan a SeedQR", "Scan a share", "Scan a setup code", "Scan a key for {wallet}", "Scan a QR code"), and a scanner a flow opens what the flow asks for ("Scan the PSBT", "Scan the description", "Scan Key {n}'s xpub", "Scan a cosigner's key", "Scan a transaction", "Scan an address"; `docs/SCANNING.md` §7.1); the camera's picture, its corner brackets white while looking and green while a QR that read is in view, a QR the scanner finds outlined where it is in the picture (`found_outline`: warning while found, ok once read, its corners rounded and edged each side in the role's on-colour), the state line and the bar of parts read on a translucent band at its foot (`viewfinder_over`); under it the ways in: **Type the words instead** where the scanner takes a seed, **Paste** (shown only where the shell has a clipboard), and **Cancel**, or **Done** on the Spend tab's intake; a camera chooser where there is more than one. | A code read routes by what it is (`docs/QR.md`): one classifier tells every kind apart (`scan_read`), and a scanner names a kind it does not take by what it is, "A transaction to sign · not used here" (`docs/SCANNING.md` §4). A scanner that takes a seed carries the never-type-a-real-seed line in `WARN`. Ctrl+V is Paste. A payload that is a secret is refused on the way in from the clipboard, except a seed pasted into a scanner that loads one (Home's Scan among them), on the desktop (F75). With no camera the picture says so and the ways in stay live. |
| Read card | The scanner sheet changed in place (`read_card` in `screens/sheets.rs`), not a dialog over it (`docs/SCANNING.md` §5; checked against Material 3's Dialogs and Snackbar, and Common buttons for the actions' emphasis): under the scanner's title, in place of the picture, the fact **Read** with what was read ("A transaction to sign", "A codex32 share") in the warning tone for a secret, the lines that say which one ("Key 73c5da0a", "Share C of NAME · 2 needed", "Example · account 1", "1 input · Savings", "2 of 3 · Native SegWit"), never a secret's value; text that is none of the kinds Faraday reads under them as a masked secret (§4.10), its eye in a `TOUCH` square (Material 3's icon button); at the foot **Scan again** (a text button) and **Cancel**, and at the end the way over, filled ("Add this key", "Sign it", "Use in Lightning", "Add to {vault}", "Unlock {vault}", "Make a vault" over "Not kept · scan it again once the vault is open"); the way over stands on the row above, at the card's foot, where the row does not fit. A kind Faraday takes nowhere has no way over. | The camera is off while it shows; **Scan again** wipes the code and turns it back on in the same scanner; **Cancel**, Back and a lock wipe it. A way over that loads a key is dimmed while a stick is in, with "Remove the stick first" on the card. Nothing is filed until a way over is pressed. The text's eye shows it, the next press hides it, and nothing is copied. Its facts are worked out at the read and kept with the card. |

### 4.10 Secrets

| Kind | Rendering | Behaviour |
|---|---|---|
| Masked secret | A box outlined faintly in `WARN`, bullets in place of the value and the eye (`Icon::Eye`, its tooltip "Show") in a `TOUCH` square at its right end (`secret_with`, `seedqr_box` in vault: a key's words and SeedQR, an entry's password, TOTP secret and notes, a note, a paperkey; and on a GPG key's **Sign a file** list each small Inbox text file's text, `sign_text`, a file being able to hold a secret, its box kept to the file's own line of text, drawn in the page's mono Body S); while it shows, the value or the SeedQR in a box outlined in `WARN` and the struck eye (`Icon::EyeOff`, "Hide"). | A press or tap on the eye shows it, the next hides it; a hold is no different from a press. It hides again once its page is left: another screen, vault, kind or record, the item's form or another panel in its place, a sheet over it, a lock, the idle lock (the Sign a file list's text also when the list is closed). The value is made only while it shows. An entry's TOTP secret that reads as TOTP has **Show as QR** under its box: its `otpauth://` URI on the QR sheet marked Secret, with no **PNG** (`docs/QR.md` §7.1). A paperkey has the same button under its box (F215), its code one static code with no format choice. |
| Show and Hide | A **Show** button beside a masked value that becomes **Hide** (BIP-85's value, a vanity counter, a Lightning node key). | Shows the value until Hide. A masked field of text a person types or keeps (a key's words, a password, a note) takes the eye; a grid of words, which is a layout and not a field, takes the **Show** / **Hide** button; one secret is never given both. |
| Secret out | A sheet before a secret leaves the session (`secret_out_sheet`): "This is a secret" or "This may be a secret", what it is, who can use it, and the ways out — sealed in a vault, or unprotected on the stick. **Unprotected** opens **Save without encryption?** (`plain_sheet`, F129): where it goes as a plain file and, for a stick, that any computer it is plugged into can read it and it is kept away from an online computer, **Hold to save without encryption**, and **Put it in a vault** with a vault open, else **Cancel**. The same sheet comes before Back up's plan keeps a seed's or a codex32 string's file on a stick at a place, and before Transfer's Receive saves a secret. Once a secret file is written to a stick or read from one, the stick's visit shows, in the error tone, the stick's name and the files, that it is not to go into an online computer, and the button **Delete unprotected secrets** (Delete files with those chosen), until they are deleted or the stick is out. | Cancel keeps it in memory; on the hold's sheet nothing is saved until the hold is held to its end. |
| Delete a file no stick holds | A dialog (`outbox_delete_sheet`, Material 3 Dialogs) over Files for a vault, or an unprotected secret, in For the stick that no write has put on a stick: **Delete {name}?** with the error icon, what is lost (everything, or the changes since the stick last written), **Hold to delete the vault** or **Hold to delete the file**, and **Cancel**. The file row's ⋮ ends with **Delete** in place of **Remove**. | Cancel keeps it; nothing is deleted until the hold is held to its end. |
| Remove every key | A sheet (`remove_keys_sheet`) that Settings' **Remove every key** opens while a loaded key is kept nowhere else, by the Lock sheet's own rule: **Remove every key**, "Keys kept nowhere else are lost" over the list of those keys, that every key loaded is removed from memory, **Hold to remove every key** and **Cancel**. With every key kept elsewhere, Remove every key acts at one press. | Cancel keeps every key; nothing is removed until the hold is held to its end. |
| Worksheet | A codex32 worksheet as the booklet draws it (`docs/CODEX32.md` §7.5, `faraday/faraday-core/src/codex32/screen.rs`): boxes `codex32_box` on a side by the window's class, numbered above in groups of four, the rows' names in a `CODEX32_LABEL_W` column, the staircase as the ladder works down; given boxes (the data, the header) outlined heavier in `TEXT`, the checksum squares in the checksum container and bolded, rows read from a table in the highest surface container, the final row under a rule; the box being filled outlined in the accent. The dice de-biasing sheet beside it: five tracks of pads `CODEX32_PAD`, the first value outlined, the second filled, an equal pair struck in `WARN`, and the tree of 32 leaves with the path from die 1 in the accent. | A press on a cell opens its working on a sheet (`CHART_SHEET_WIDTH` wide; the panel's width on a compact window) with **Done**; nothing is computed to open it. |
| Mask policy | Words, seeds, private keys, passphrases and shares are secrets. | Fingerprints, checksums and addresses are not. A secret is never copied to a clipboard. In Practice (F81, `docs/MODES.md` §6.2) a secret a sample filled is drawn unmasked, and a passphrase or password field holds `faraday` in a code box, unmasked, with no eye, once **Use faraday** in Faraday Demo's host strip (§4.19) is pressed; nothing is drawn inside an empty one, and where it is optional **No passphrase** shows in it as in every mode. A passphrase being made is entered twice (`screens::pass_pair`): while the two differ "The two passphrases differ" shows under the second and the step's primary action waits. So is a BIP-39 passphrase typed for a key (Add a key, the potential wallet, a vault's Load with a BIP-39 passphrase), which nothing checks, and a KeePass export's; a passphrase that opens something that checks it (Unlock, Restore, Spend from a backup, a wallet's missing key) and a vault entry's password stay single. On a vault's item page a secret already showing (a masked field's eye, a seed's code, a TOTP secret's or a paperkey's code on the QR sheet) is hidden at once when a device that can capture the screen arrives, with the device check's sheet up; it shows again with no press once the device is gone and checked again or let through, and a Lock in between drops it. |

### 4.11 Records and results

| Kind | Rendering | Behaviour |
|---|---|---|
| Fact | A label over its value, the value wrapped (`faraday_ui::sheet::kv`), or a label and value on one row in a card's table. | One fact per row. |
| Card | A surface set apart by its edge alone, `BORDER`, with `SHAPE_M` corners (`Ui::card`, F39): a wallet's card, a step's summary, Home's tiles. | — |
| Result | A line with its icon in its tone: done in `OK` ("Signed with 73c5da0a"), refused in `ERR`, waiting in `WARN`. | — |
| Warning | A line with the warning glyph in `WARN` (a backup's checks, a spend's cautions); "No warnings" where there are none. | — |
| Dimmed | A control not available now, in its own kind, faint (`Style::off`, `Style::when`): a filled or tonal button keeps its fill in `TEXT` at 12 %, an outlined one its edge; its label in `DIM`. Where a step the person can take would make it available, a short line under it says which (Tools' rows: "Load a key first", "Make a vault first"). | A press does nothing. |
| Toast | Material's snackbar (`Ui::snackbar`): its text in body medium (`TOAST_TEXT`) and inverse-on-surface on inverse-surface in the scheme on screen, at elevation level 3 with `SHAPE_XS` corners, `TOAST_PAD` each side; one line `TOAST_H` tall, two or more where the width does not hold it, `TOAST_PAD_Y` above and below; centred at the foot of the page, `TOAST_MARGIN` from its foot and sides, as wide as the window less those margins on a compact window and else as its text between `TOAST_MIN_W` and `TOAST_MAX_W`, each line of its text centred (F163; Material sets it from its start), standing `TOAST_STACK_GAP` above Home's Scan where Scan is in its way, with a close button (`Ui::snackbar_closable`) only for the list of what was plugged in at start (a `TOUCH` square × in the ink at its end, the text keeping the room left), as Material places a snackbar over a floating action button; rising in and fading after `TOAST_SHOW_MS`, 4 s (`notices`), not rising under Reduce motion: "{what} waits for the stick". The same place holds "Remove the stick to keep working" in `WARN` while a stick stands in the way. | Says what a press did where the screen does not change to show it. |

### 4.12 Text

| Kind | Rendering | Where |
|---|---|---|
| Title | §4.1. | Every page and sheet. |
| Section label | 14 units SemiBold, muted (`faraday_ui::parts::section_label`). | Over a group of rows. |
| Label | Caption or label size, muted, above or left of its value. | Every fact. |
| Value | Body or mono. | Everything a person reads or compares. |
| Explainer | Paragraphs on a Guide page, in the Guide sheet (`guide_sheet`), or on an extra-large window in Guide's supporting pane beside the page (`screens/guide_pane.rs`, `docs/LAYOUT.md` §6): a card in the surface-container role at the right, closed until Guide, the ? or Cryptography is pressed; a header row, as Material's side sheet has, with the headline **Guide** in Title L and at its end a pin (a toggle icon button: the push pin outlined while off, filled in primary while on) and a close button; under it the sheet's tabs across the pane's width (Material's primary tabs, **Guide** and, where the screen offers one, **Cryptography**, each label whole), then the guide's page, its title in Title L, its section headings in Title S and its text in Body M, with a **Contents** text button at its foot listing every page in Material's one-line list items; it follows the page's steps, closes when the person moves to another screen unless pinned, and scrolls within itself. | Never on a working screen outside the one tooltip below. |
| Tooltip | Material 3's tooltips (`faraday_ui::tooltip`), drawn over everything once the screen is drawn (`Ui::draw_tip`), above what they are about with `TIP_GAP` between, or below where there is no room above, kept inside the window. Plain: body small (`TIP_PLAIN_TEXT`) in `TIP_PLAIN_INK` (Material's inverse on-surface) on `TIP_PLAIN_FILL` (inverse surface), `SHAPE_XS` corners, at most `TIP_PLAIN_MAX_W` wide, centred on its anchor. Rich: body medium (`TIP_RICH_TEXT`) in `TIP_RICH_INK` (on-surface variant) on `TIP_RICH_FILL` (surface container) at elevation 2, outlined in the high-contrast themes, `SHAPE_M` corners, at most `TIP_RICH_MAX_W` wide, starting where its anchor starts. A screen marks its anchor with `Ui::tip_anchor`; which shows is the pointer's state's (`faraday_ui::tooltip::Tips`). | Shows once the pointer rests on its anchor for `TIP_DELAY_MS`, or the keyboard stays on the anchor's control as long, or, for a rich one only, a finger holds it for `TIP_LONG_PRESS_MS` (then the lift presses nothing); a plain one never shows to a finger. Goes when the pointer leaves, the keyboard moves on, at Escape, or at the next press. A rich tooltip may also open at once on a press of its own icon button, and then stays until the next press or Escape. Used for: Create's Backup card, on its Shares (codex32) row (moved from Kind by F240), rich, an explanation on a working screen (F112), and Create's quorum card, after BIP-48 on the multisig path row, rich, the second and last (F287), each on Material's info icon button (`Icon::Info`, `ICON_BUTTON` round with an `ICON_SMALL` glyph in `MUTED`, a `TOUCH` target that takes the press only on the button) right after "(codex32)" and, on the path row, after the value or the segmented button, and for codex32 on the word itself. The plain one is for icon-only controls, and names the action in a word, never explains it ("Back", "Guide", "More", "Lock", "Keyboard", "Power off", "Show" or "Hide" on a field's eye by what a press does, "Remove", "Link", "Warning", "Close"): the top app bar's back, ?, ⋮, Lock and keyboard, the small panel's bar and flow bar (back, ?, keyboard, the codex32 eye), the wide page's ? and keyboard and codex32 eye, Guide's pane's close button, the power buttons, the passphrase fields' eyes, and the backup board's ×, ⋮, link and warning marks (a mark that is no button owns its tooltip by itself, `BoardPress::Mark`). Where a sheet or popup covers the page, the page's tooltips go with its presses (`Ui::clear_hits`). The Starter's top bar marks its back and ⋮ the same way; its host does not yet show them. Checked against Material's guidance as Android's Compose documentation states it (developer.android.com, Tooltip: plain tooltips describe an icon button's element or action, rich tooltips give more detail and may be shown by a press of their anchor) and Material's Icon buttons (a 40 dp container, a 48 dp target); m3.material.io's pages are script-drawn and were not readable. |

The ? on a page with a Guide page opens the Guide sheet at the section
that page is about; on an extra-large window, Guide's supporting pane
beside the page (`docs/LAYOUT.md` §6).

Not built (`docs/CRYPTO-LEARN.md` §8.2, §8.5): on a screen that offers
a cryptographic action, the Guide sheet and pane carry a first tab,
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
| Button | `Ui::button` in a `Style`, Material's buttons, round at their ends: `Primary`, filled in the accent, the one action a step leads with; `Tonal`, filled in `SELECTED`, a second action that matters beside the filled one (Back up's **Show the seed** beside **Check my copy**, a vault's **Add another passphrase** beside **Create vault**, Spend's **Scan the QR codes you have** beside an open vault's **Load … wallets with keys**, **Sign** on a PSBT in Files, whether or not a key is loaded); `Secondary`, outlined in `OUTLINE`; `Ghost`, a text button, its label in the accent; `Selected`, a chosen option, ticked (§4.2). Each kind not available now keeps its shape (§4.11). Hover, press and the keyboard on it lay the label's colour over it at the state layers' opacities, and the keyboard's place is ringed (`Ui::state_layer`, `Ui::focus_ring`). A press is taken on at least `TOUCH` square, the target centred on the button and sharing any gap with a neighbour's (`faraday_ui::Reach`). A text button standing alone under body text or a field, or first in a row at the content's edge, has its label on that edge and its container (state layer, touch target) reaching out past it by `BUTTON_PAD` (less where the edge is nearer), as Material's text button does (`Ui::text_button`); in a row of buttons the others keep the row's own spacing, and a text button first on a card's row has its label at the card's text. A row of them shares the width where every label fits its share and stands one under another, full width, where not (`faraday_ui::sheet::buttons`). A button may carry an icon before its label, as Material's buttons with an icon do (`Ui::button_icon`, `ICON_GLYPH_BUTTON` and `ICON_GAP`): Check an address's **Scan** and **Paste**. The on-screen keyboard's keys are buttons with `RADIUS_SMALL` corners (`Ui::button_in`). | A press acts. |
| Hold | An outlined button that fills while held (`hold_button`): "Hold to sign" with a GPG or Secure Boot signing key from a vault, "Hold to delete from vault", "Hold to mark the key revoked", "Hold to save without encryption" (F129), "Hold to add this wallet" on the wallet confirmation sheet (F254, F132), "Hold to register" on the register sheet (F135), "Hold to delete the vault" on a file no stick holds (F136), "Hold to pay this fee" on a spend's Review, "Hold to continue" under the device check's and the Starter's Continue anyway, and the Starter's "Hold to write the older release" on Confirm (F136), and on a stick\'s Delete files sheet "Hold to delete 1 file with zeros" (a vault among them), "Hold to zero the free space" and "Hold to zero the whole partition" (F131), or "Hold to delete a vault's key file" when the files chosen include a `.keyx` or a file used as a key file this power-on, and "Hold to remove every key" on the Remove every key sheet. | Acts once a pointer or a finger has held it to the end; released before, nothing happens. No key sets it off (§4.15). |
| Key tick | A press on a key of a wallet of several keys that is not checked ticks it; a ticked key unticks on another press (§4.4, Key block). | The one control that is a whole entry, not a button: its mark is the checkbox (`Ui::checkbox`, `SHAPE_CHECKBOX`) while unticked and `Icon::Done` once ticked. |
| Floating action | Filled in primary over the page, its icon and word on-primary (Material's primary FAB colour style, F138), with `SHAPE_L` corners, its `ICON_GLYPH` icon and its word, at the bottom-right (`Ui::fab`). | **Scan**, on Home and on the top page of Check, Wallets, Files, Vaults and Tools; on the small panel's menu layout, on Home alone. Those pages' scrolling content ends `HOME_RESULTS_FOOT` lower, so its last row scrolls clear of it. |
| Dialog | Material's basic dialog (`faraday_ui::sheet::dialog`): its head, what it holds, and its actions, over the page's frost and scrim, `SHAPE_XL` corners; on the wide layout `DIALOG_MIN` to `DIALOG_MAX` wide with `DIALOG_PAD` inside and its actions at its end as wide as their labels, on the small panel the panel's width less a margin with its actions sharing the width. What is above the actions scrolls when it is taller than the window, under a divider a pad above them, so a control cut at its edge takes no part of an action's touch target. `faraday_ui::sheet::dialog_in` centres one over the page right of a sidebar. Drawn with it: the Inbox's seed (`potential_sheet`), the device check, the pull, the lock, lock-ask, write-out, power and wallet confirmation sheets (`kept_sheet`), **Which account to load?** (`silent::screen::account_dialog`, its two choices alike, Secondary, so neither is the default), **Published words** (`screens::sheets::published_sheet`, a key on Mainnet, in three variants: words published, **Keep for testing** and, primary, **Remove**; a key Faraday Demo wrote on an online computer, a hold in its body ("Hold to keep this key") and, primary, **Remove**; a public sample seed from a vault, **Remove** only), the register sheet (a wallet registered from a spend, F135), the Sweep sheet (F171), an open vault's **Change passphrase** and **Memory** (`vault::rekey_screen`, `docs/VAULT.md` §8), a vault's name taken on a stick (`visit_taken::name_taken_sheet`, its overwrite a hold in its body; F203), the older copy an unlock stops at (**Open this copy?**, `vault::load_screen::older_copy`, `docs/VAULT.md` §11), Tools' **GPG key** with no vault open (`GpgTile`, `backup_flow::gpg_restore_screen::tile_dialog`: rows **Make a key** and **Restore from a paperkey**, the second dimmed in Practice, and Cancel; checked against Material 3's Dialogs), the Unlock screen's **Choose a key file** (`vault::unlock_keys`, a radio list of the files in Files that are not vaults, Cancel and Choose; `docs/VAULT.md` §4), BIP-85's **Record it as a child seed?** (`bip85::screen::record_dialog`, its two actions alike, Secondary, so neither is the default; F166) and Create's **A BIP-85 child seed** (`screens::create::child_notice`, its one action **OK**; F166), **Remove codex32 set** and **Remove BIP-85 origin** (`vault::trim::draw`, Cancel and Remove; `docs/VAULT.md` §7), a checked stick's sheet at Bring in the sticks (`backup_flow::stick_sheet`, its one action **Close**; `docs/STICK-SETUP.md` §5), a new input device, the idle warning (`faraday_ui::sheet::kv_sheet`), Locked, where a secret goes, the SeedQR read, the Krux encrypted code sheet (`seeds::kef::sheet`), Save to a stick, the GPG signature, codex32's cell working on a wide window (Previous, Next and Done its actions), and the Starter's Restart, Continue anyway and Uninstall. Its actions run dismissive first, confirming last; a hold stays in its body. Not dialogs: the scanner, the QR code, the Guide and word-list readers, which fill the window with what they show; the menus and the other sheets of `faraday_ui::sheet::scroll_sheet`, and `faraday_ui::parts::sheet_box`, take the same corners. | A press beside it is its way out, where it has one. The keyboard reaches its controls, then its actions, in the order drawn; Enter or Space presses the one it is on. |
| Cancel a flow | **Cancel**, an outlined button at the right end of a flow's title row (`flow::column`); on the other one-column layouts in the bar's tools (`compact::tools`), and on the compact layouts the first row of the bar's ⋮ menu (`compact::overflow_sheet`; owner, 2026-11-11). On every flow that can have a sidebar row but a backup flow whose key is made, which ends only by finishing (`docs/FLOWS.md`, Flows in progress, Cancel; F51). | Ends the flow and shows the screen it was opened from. When ending it loses something the person would notice, a sheet first: its question as the title ("Cancel making this wallet?"), one line naming what is lost, **Keep going** (secondary) and **Cancel the flow** (primary); a press beside the sheet keeps going. With nothing done past the first card, no sheet. New key opened on its own has Cancel once something is entered and the key is not locked in, asking "Discard this key?" and returning to where it was opened. |
| Ending a sheet | One name for one act: **Cancel** abandons what was started or typed and changes nothing; **Close** dismisses what was only read; **Done** finishes what was made or changed. A sheet with one action that is the point of it (a hold apart) has it filled, `Primary`; Cancel and Close are `Secondary`, never filled and never `Ghost`, and Cancel stands beside the action that confirms (dismissive first, confirming last). Checked against Material 3's Buttons, Dialogs and Bottom sheets (their dismissive action beside a confirming one). | **Cancel** and Escape leave a sheet as it was; the same word on every sheet. |

### 4.14 Progress and state

| Kind | Rendering |
|---|---|
| Progress | A count "2 of 5": the Spend tab's signatures, New key's entries. Where work has a count, Material's determinate linear indicator, `PROGRESS_H` tall, its track filled by the share done (`Ui::progress` with `done`): a scan's parts, the Spend tab's ways tried ("Trying … 2 of 5"), Explore's Find, zeroing a stick, the Starter's write and read-back. A TOTP code's row has a bar under it for the time left in its period, the seconds at its end. Long work with no count shows Material's indeterminate linear indicator, a bar crossing its track, still under reduce motion; work that holds the frame until it is done (a vault's unlock, creation and sealing, an unlock from the boot import and the Spend tab) shows it still, along the foot of the line that says what the work is (`working_bar`). While a job runs on the worker (`Faraday::busy`, `docs/WORKERS.md` §4) the screen stays and the same indicator moves: indeterminate, a bar crossing its track in `BUSY_PASS_MS` (still under reduce motion), or determinate where the job counts, under the job's line in the accent (`busy_line`; `busy_foot` along the window's foot where no sheet is open, Faraday Demo's Samples sheet in its foot). Every action but Back, Cancel, Lock and what changes how the screen is drawn is drawn in Material's disabled state and takes no press (`Ui::keep_live`): a button off, and a row, link, chip or card faded to the disabled content opacity over its own container (`Ui::fade_disabled`). Checked against Material 3's Progress indicators, "Linear", Buttons, "States", and Lists, the disabled item. |
| Default | A step card (in Faraday a flat section of its flow: no fill and no box, divided from the next by a rule in `LINE`, its number, title and value on its row, `docs/DECISIONS.md` F39) whose value has a default is closed at entry, showing its value and a **Change** text button at the card's right in place of the chevron that opens it. The flow opens on the first card with no default; opening a closed card does not reset the cards after it. A card that asks one thing closes on the choice and opens the next (F103); Back opens the card before the open one, its choices as they were, Forward the card Back left, and the first card's Back leaves the flow (F104). On a small panel the same: the paged flow starts on the first open card, and that page carries at its head one row per earlier card closed on its default, "{title} · {value}" with **Change**, which opens it. On the 7-inch panel, where a row each would stack under the touch floor and taller rows would take the room the open step's controls are sized to (the word keyboard), two or more such cards share one row a touch target tall, their "{title} · {value}" one after another, whose **Change** shows the list of steps; a lone row's press reaches down over the gap under its rule, so it is a touch target tall too. Tapping the page's title (the card's name alone, "Kind", with "Step 1 of 8" beside it in the muted caption type; where the number of steps can change, as in Spend, Restore and Back up, it reads "Step 1" alone, so a stated total never moves) closes the card and shows the list of steps, from which any card opens; no step indicator is drawn. Create a wallet and New key are the exception: each opens on its first card — Kind, Start — whatever opened it, even with a default already ticked (Start has none: no choice is ticked until one is pressed); a card still closed on its default shows its value and **Change** the same way once Continue has passed it (`docs/history/NEW-WALLET.md` §2.1). |
| Empty state | A line and the buttons that start the flow: Wallets with no wallet loaded, "No wallet is loaded" and its ways in (Make a new wallet, Restore an existing wallet, Spend from a backup, Add a key); TOTP codes with no entry, "No entries with a TOTP secret" and Scan a setup code. |
| Locked | The sheet after the idle lock (`locked_sheet`): "Locked", that every key and vault was wiped, what waits for the stick, and Continue. The idle warning before it counts down (`idle_warn_sheet`), over any sheet but a lock's own, a code on the QR sheet taken off the screen with it. |
| Self-test failed | The whole window: "Self-test failed", the check that failed, and Exit (`selftest_failed`). |

### 4.15 Keys

A shell with a keyboard — a laptop booted from the stick, the desktop
shell — takes keys as a desktop app does. Nothing on a screen says so.

| Kind | Behaviour |
|---|---|
| Typing | Characters, Backspace, Delete and Enter go to the field typing goes to, and to the on-screen keyboard's field where it is up; Left, Right, Home and End move the caret, and Up and Down a line in a field of several lines; Backspace, Delete and a character act at the caret (`edit`, `docs/TEXT-CURSOR.md`). |
| Enter | Takes what is typed where the field has one action: a word on word entry, a share or a part, a name, a passphrase. |
| Tab | Finishes a word on a Latin list once only one fits; moves between a passphrase and its repeat. On the backup board, Tab and Shift+Tab move among the chips' ⋮s, ringed in the accent at `FOCUS_STROKE` (§4.17). Everywhere else, Tab and Shift+Tab move the keyboard's place through the controls in the order the frame drew them, round at the ends; the control it is on is ringed (`Ui::focus_ring`), and Enter or Space presses it. A hold is not among them: no key sets one off. A press of the pointer or a finger takes it off. |
| Escape | Closes the open sheet or ⋮ menu (a sheet's Cancel), leaves a field; with none of these it leaves the page as the top bar's back arrow and the mouse's back button do (one act). |
| Up, Down | In a ⋮ menu, move between its rows; elsewhere scroll the region under the pointer, or the page's, by one wheel notch. On a slider the arrows step it (§4.2). |
| Ctrl+V | The scanner's Paste, and nothing anywhere else. |

### 4.16 Structure chart

How a thing is made of others and where each of those is: Faraday's
wallet at a glance, the wallet over its keys
over its backup; and the same chart from an open vault (§7.2), the
backup over the keys over the wallet.

| Kind | Rendering | Behaviour |
|---|---|---|
| Node | The summary row's box: `RADIUS`, the page's fill, an inner edge, `CHART_NODE_PAD` inside. A title in the label type, an optional tag at its right in the caption type (secret, sealed, public, in their tones), then caption lines that wrap; under the name of a place the age of its oldest copy's last check ("Last checked 14 months ago", "Never checked") in `WARN` with **To check** at the line's right when a copy there is due, after 12 months or sooner by a recovery wallet's margin, and **Older copy** in `WARN` for a stick read from an older copy of its vault (`docs/BACKUP-FLOW.md` §18.3); the strip's fourth line reads "Every place checked in the last 12 months" or "To check: " and the places, and "Set the time to see what is due" with **Set the time** while the clock is not checked; a wallet's node and sheet carry **I moved every coin** and a **Coins last moved** row (§18.4); a line marked lost or exposed reads "Key 1 words · lost" in `ERR`, struck through by a rule of its colour; a node that offers a step carries a `CHART_BUTTON` button at its foot. | The whole node is the target: it opens the node's sheet. A caption line that stands for one thing (a thing a backup node holds, a key's way in) is a target of its own over it: its sheet, or its way in. |
| Sheet | What a node or a line opens (`docs/history/NEW-WALLET.md` §9). On the wide layout (beside the expanded rail: large windows and wider, not low) it is Material's standard side sheet (Material 3, Side sheets; `faraday_ui::sheet::side_sheet`): `SIDE_SHEET_W` wide (Material's 256 to 400 dp, at its widest) at the window's right edge, from the page's first line under the top app bar (`SUPPORT_PANE_TOP`) to the foot, in the surface colour at no elevation with a divider along its leading edge and no scrim; the page's room narrows by its width, so the chart and its lines stand beside it, never under it (where the room left cannot hold Wallets' list pane and a card at least as wide beside it, the wallet's card stands alone in it until the sheet closes, `docs/LAYOUT.md` §5), and stay live: a press on another node or line changes the sheet, and Where it is draws the key's lines bold with the sheet still open. Its header holds the head and Close, a text button at its end (Faraday's icon face has no close glyph); `SIDE_SHEET_PAD` inside; Back and Close shut it. It slides in from the edge on Material's emphasized decelerate (`motion::SIDE_SHEET_IN_MS`, medium 4) and out on its emphasized accelerate (`SIDE_SHEET_OUT_MS`, short 4), at once under Reduce motion (`docs/MOTION.md`). On the rail's and the bar's windows, where the chart is a column with no lines, it is a sheet `CHART_SHEET_WIDTH` wide centred over the chart, frosting it, and on a small panel the whole panel as a page, as before; there Where it is closes it to show the chart. Its head (icon and title), its facts as label-over-value rows, lines of what follows in the label type in their tones, a field being typed (its label in caption over a `CHART_SHEET_ROW` box with the caret; a passphrase as dots), then its actions as menu rows under optional caption headings, and its own primary button (Keep, Check) beside Close (Back on a small panel; in the side sheet's header instead) at its foot. | Each row leads into the flow that does it; opening the sheet does nothing else. The sheet changes nothing in the plan's map: each key, place, line and mark sheet, and a SeedQR drawn in part, has one **Change in the plan** row, which opens the backup board (§4.17) with that place or thing selected. |
| Row | Nodes side by side at equal widths, between `CHART_NODE_MIN` and `CHART_NODE_MAX`, `CHART_NODE_GAP` apart, centred. A row wider than its column wraps onto further lines at the widths a full line gives; the nodes of one line take its tallest node's height. | — |
| Line | A `CHART_STROKE` run (`CHART_STROKE_BOLD` for a key whose sheet's Where it is shows its lines, every other line then the inner-edge colour) from the middle of a node's bottom edge to its target's top edge, orthogonal: down into the band under the line of nodes, across, down. Where a line of nodes stands between, it runs down the gap between two nodes, or beside them, so it never crosses a node. Each band is `CHART_BAND` above and below its runs, each source's run across `CHART_LANE` from the next; a target's lines come in at points spread along its top. Each source has its colour (`KEY_LINES`, the theme's own colours in turn) and its dash pattern (`CHART_DASHES`, solid first), so the lines are told apart without colour too; the parent's lines are the border colour, solid. | Lines are not targets. |
| Column | On a small panel the rows stack, one node a line, full width, under the rows' headings, and no lines are drawn: a node names what the lines would have joined it to. | — |
| Direction | The rows read top down either way: the parent first (the wallet, its keys, its backup, as the wallet's card draws it), or the parts first (the backup, the keys, the wallet at the foot, as an open vault draws it). Lines always leave an upper node's bottom edge and enter a lower node's top edge. The plan's three checks go above the chart read either way, as a strip: three boxes side by side where each is at least `CHART_NODE_MIN` wide, else one under another, each its line in caption and its value at `TITLE` size in its colour (`OK`, the accent, `WARN`, `ERR`); the wallet card and an open vault's chart add a fourth line under them, in the box style at body size, with every copy checked in the last 12 months in `OK`, the places due in `WARN` (a press opens the first one's sheet) or, with the clock not checked, "Set the time to see what is due" with **Set the time**; the backup's map panel and the end of the backup draw the same strip (`docs/history/NEW-WALLET.md` §14.6). The column reverses its rows and headings the same way, the strip staying on top. | — |

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
| Chip | A pill one touch target high (48 dp; 44 dp on the compact and short layout, F188), as the table above draws it, `BOARD_PAD` at each end: its glyph where it has one (the fingerprint before a seed, the passphrase glyph before a passphrase, the closed lock before a vault; the description and a share have none), its name (a fingerprint in mono at the label size, anything else in the text face), then its forms: on the read-only board (before **Customize backup**) after a middle dot in caption, muted ("9a6a2580 · Words · SeedQR"); once customised, where the zone has more than one form for it, as **Form toggles**. Marks at its right, from the edge in: its ⋮ after a thin rule (once customised; **Chip's ⋮**), the × of a placed chip, the warning mark, the link mark, the lock mark (a placed chip in a vault: the lock glyph in `VAULT_EDGE`), each `BOARD_MARK` with `BOARD_CHIP_GAP` before it. A chip whose forms do not fit beside its name drops them to a second line under it (a caption line, or a row of toggles `BOARD_TAG` high). | One chip per thing per zone; its forms are what it carries there. |
| Chip in the list | The same pill, one per row, its count of places at the row's right in caption, muted ("2 places", "1 place", nothing at none); A vault's chip carries its × at the row's right. A cosigner's seed is outlined, not filled, its name dim, "Not here" for its count. A vault no place keeps a stick of is filled as any chip, its name and glyph dim (§19.16). On `wide` the list is the right-hand column where Back up's Map is when the board is not shown (`side_panel_w`, full height, `SURFACE` with a rule at its left), scrolling on its own with its overlay scrollbar, the list from its top, grouped under caption labels: Seeds, Passphrases, Wallet description (Whole, then **Keys left off each share** as a − n + stepper of buttons a chip high, then each share), Vaults with **+ new vault**, Watch-only software. | Never runs out: placing it places a copy. A cosigner's seed takes no press. A vault's × takes it off the plan; one that holds anything first shows what it holds in `WARN` under it, with **Take off**. |
| Add sheet | In the narrow column only, once the board is customised: **Add**, a secondary button a chip high at the top of the board, opens the list as a sheet (the small panel's full-width sheet, its head the add glyph and "Add", the list's groups, each chip with its ⋮, a way out at its foot). | A press on a chip in the sheet, or on its ⋮, opens its **⋮ menu** (Add to…) over the sheet; a box chosen places it and closes both (`docs/BACKUP-FLOW.md` §17.20 item 4). Nothing is dragged there. |
| Placed chip | The chip in its zone, with its × and its ⋮ once the board is customised. A chip of what an existing vault held before is outlined and dim and has no × and no ⋮. | A press selects it. The × takes it off. |
| Chip's ⋮ | Once the board is customised (`docs/BACKUP-FLOW.md` §17.20), at the right end of every chip that can move (in the list, placed, a vault's band): a rule 1 dp wide inset `GAP_SMALL` top and bottom in the ⋮'s colour at `BOARD_MENU_RULE`, then three dots `BOARD_MENU_DOT` across, spread over `BOARD_MARK`, in a part `BOARD_MARK` plus `BOARD_PAD` each side wide; `MUTED` on a chip (`DIM` while it is dragged or dim), `VAULT_ON` on a vault's band; `INNER` behind while pressed or hovered. The × and the other marks stand before the rule; a placed chip's toggles before them. With the keyboard on it, edged in the accent at `FOCUS_STROKE` (on a vault's band in `VAULT_ON`). Not drawn on the read-only board. | Its own press target: a press opens the chip's ⋮ menu and neither selects the chip nor starts its drag. Tab and Shift+Tab move the keyboard among the ⋮s on screen in the order drawn; Enter or Space opens the menu. |
| ⋮ menu | On `wide` a popup `BOARD_POPUP_WIDTH` wide under the ⋮, its right edge on the ⋮'s (above it where it does not fit below; scrolling where it fits neither), on `SURFACE` with `BORDER` and the shadow of elevation 2, no backdrop; on the small panel the full-width sheet with **Cancel** at its foot, its head the add glyph (Add to…) or the chip's glyph. Inside: its title in caption, semibold ("Add to…", "Move to…", "Copy to…", or the chip's name), each place's name in caption, semibold, `MUTED`, and under it the boxes, a row 32 high each, the label type, set in `BOARD_PAD`: "Paper", "Stick 2 · files", "Stick 2 · Vault 1", "New stick"; then the software place's. A placed chip's first level: **Move to…**, **Copy to…**, **Take off**. The row the arrows are on is `INNER`, edged in the accent at `FOCUS_STROKE`. | Lists only the boxes that take the item in some form and do not keep it already. A box chosen places it as a drop there does (the checks, the balance sheet), and closes the menu; Move to… moves it with its forms, Copy to… keeps it in both (not for a share), Take off is its ×. Arrows move, Enter chooses, Escape goes back from Move to… or Copy to… to the first level and else closes it, the keyboard back on the ⋮; a press outside closes it. |
| Form toggles | Once the board is customised (`docs/BACKUP-FLOW.md` §17.3), on a placed chip whose zone has more than one form for it, after its name: a pill `BOARD_TAG` high per form, `BOARD_PAD` at each end, `BOARD_CHIP_GAP` apart, at `SURFACE`; the check glyph `BOARD_MARK` in the accent and its name in caption, `TEXT`, with the accent edge, where the form is kept; the glyph's room left empty, the name `MUTED` and the line edge, where it is not ("◉ 9a6a2580 [✓ Words] [✓ SeedQR] ×"). Where they do not fit beside the name, a row under it. There is no forms popup. | A tap turns the form on or off; the last form on stays on. A press on the chip's name selects it, as before. |
| Customize backup | `docs/BACKUP-FLOW.md` §17.6: while the board is read-only, a secondary button a chip high at the top of the right-hand column in place of the list, and in the narrow column in place of **Add**. Read-only, the board draws no × on places, sticks or chips, no **+ place** or **+ stick**, and chips read their forms as text. | A press opens the board for changes: the list (or **Add**) in its place, the board editable, for this plan from then on. |
| Balance sheet | `docs/BACKUP-FLOW.md` §17.5: the scrolling sheet the board's Add sheet is (its width on `wide`, the panel's less `GAP` each side in the narrow column): its head the eye glyph and "<Place> can see the wallet's balance", one line in the label type naming what there makes the description, then **Take it off** (secondary) and **Keep it** (primary). | Keep it closes it, the drop standing; Take it off undoes the drop. A press beside it is Keep it. |
| Partial descriptions | `docs/BACKUP-FLOW.md` §17.19: under the list's Whole, while the description splits, **Add partial descriptions to this backup** (or **Remove partial descriptions** while shares are kept), a secondary button a chip high, then **What is a share?**. No share chip is in the list and there is no keys-left-off stepper. | A press hands one share to every place (Faraday chooses which, and the keys each leaves off) and opens the **Partials sheet**; while on, a place added or taken off has them handed out again; Remove takes every share off. |
| Partials sheet | The Balance sheet's form: its head the file glyph and "Each place now holds a part of the description.", then, where the whole description is still kept, the line naming those places, then **Take it off every place** (secondary, only where taking it off leaves any M places able to rebuild it) and **Keep it** (primary). | Take it off every place removes the whole description from every place's paper and sticks; Keep it closes. |
| One place per key | The board's **+ place** is not drawn once the plan has one place per key of the wallet (`docs/BACKUP-FLOW.md` §17.19); a single key has one place, or two when its seed has a passphrase (the second for the passphrase alone). With a single place, "Any one place lost" reads "No: a single place" in `MUTED`, not as a warning, and the "every secret at one place" line is not shown for a single key. | None. |
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
| Its controls | Outlined, `HOST_CONTROL_H` (48 dp) tall, their press their whole box, labels in Material's title medium (16 sp) in the app's inverse on-surface, outlines in the host scheme's outline, Material's state layers hovered and pressed; a control whose menu is open filled in the app's inverse primary; one that takes no press now at the disabled opacities. | Left to right, the always-there controls: the stick pills, **Samples** (Practice), **Size · <size>**; then the screen's practice aids in a section of their own, a tertiary container of the host scheme with its on-tertiary-container text: Practice, the aids, those with no room in **More**, and the lines saying what Practice puts in place of the person's own, cut to one line; the step's main press (codex32's Next, Roll) filled in tertiary. |
| Its menus | Material's menu, `HOST_MENU_W` (280 dp), rows `HOST_MENU_ROW` (48 dp) with title-medium labels, on the host scheme's surface container with an outline-variant edge, the chosen row checked; opening up from its control. | In on the emphasized decelerate (medium 1), out on the emphasized accelerate (short 3), at once under Reduce motion; a choice acts and closes it; a press beside it closes it and does nothing else. Size, Sticks, Play options (codex32's walk: how far Play runs, how fast), Rolls per click, More. Narrow, the strip gives way: the sticks fold, Size loses its size's name, the aids go into More (the step's main press last), Samples joins More, and narrowest More reads "Practice" (`docs/DESKTOP-HOST.md` §3). |
| The dice roller | The die (§4.3) in the host scheme, first among the aids in their section, the strip growing to its height while it shows; then the strip's **Roll** and **Rolls per click · N**. | As in §4.3; Rolls per click opens its menu. |

## 5. The screens

A screen is one of the thirty-seven variants of `Screen` in
`faraday/faraday-core/src/lib.rs`, drawn in the file that file's code
map names: an area's `screen.rs`, or `screens/<name>.rs` for a screen
with no area. Each is drawn in one of three ways:

- **Step flow**: under the top app bar, which names the flow and carries
  its way back and **Cancel**, the step-card column (`flow::column`): its
  chip, then one card open at a time, the others closed to a line that says what
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
| `Home` | The lead tile while something is under way or waiting, then the tiles; on the small panel the menu of the sidebar's entries; **Scan** (`docs/FLOWS.md`, Home). One Home for every size, with each loaded wallet's Sign and Receive and the first-run tiles only while nothing is loaded, is specified and not built: `docs/HOME.md`. | Page |
| `Check` | Check, the place for checking what a hardware wallet says (F255): four groups of tiles in the grid Tools uses, each tile a label and one line saying what the person needs in hand, the tiles that take a seed tagged **Seed**, each opening the tool that does the check; below the table. Checked against Material 3's cards page and its navigation drawer and navigation rail pages. | Page |
| `WalletsStart` | The Wallets tab's first page: the wallets loaded, the keys without a wallet, and every way in by what the person has. | Page |
| `Wallets` | The wallets as a list and the chosen wallet's card: its name and shape, a `tr()` tree's key path line under the shape (the danger line, in the error colour, for a key not of the wallet; F132), the backup line, the signatures possible here, **Spend paths** (fact rows, §4.11) for a wallet with a relative wait, with **Coins last moved** and **I moved every coin**, **At a glance** (§4.16, its node for a wallet of several keys counting the keys not checked), the first receive address, over which a wallet with a key not checked carries the caution line, and the card's actions (`docs/WALLETS.md`; the key block, §4.4, is on the wallet's sheet on the chart). | Page |
| `Spend` | Signing a transaction: the review (what it sends, to whom, each recipient's whole address under its row in groups of four, the fee, change, a height lock as a plain **Locked until** row (the time-lock Caution below and this row both only while some input's sequence is not 0xFFFFFFFF), whether it is **Replaceable** (signalled by an input, BIP 125, or version 3, BIP 431) or **Not signalled**, the txid, warnings, among them a time lock as a Caution stated as a date, and each input's relative lock (BIP 68), worked out once at the inspection; on the small panel Continue shows once the review's end is in view, the foot reading **Scroll to the end** until then), the keys of the wallet spent from where it has several (the key block, §4.4, while any key is not checked, one **Every key checked** line once none is), the signers, **Sign with** (§4.4), and the signed PSBT and finished transaction. | Step flow |
| `SpendBackup` | The Spend tab: spending from a backup, step by step (`docs/SPEND-FROM-BACKUP.md`). The first page opens on the step list, each step's title wrapped rather than cut, to two lines or three at most, and every row the height of the longest title's (Material 3 Lists: items consistent in a list), and Bring the transaction here, Who signs and Put everything away open on their controls, with no paragraph or note, and Read the transaction on one label line; every other card's lead is one line (the action to take, in Sparrow or here), and what its other sentences said is under More about this. | Step flow |
| `Files` | The Inbox and the Outbox: a row per file with its kind, size and ⋮, and what each file can do (Sign, Decode, Nonce check, Load the wallet, Unlock, Import into the vault). While a stick is open, a checkbox on each row for writing, and the stick pane: what is on the stick, ticked to bring in; beside the lists from expanded windows up, below that as primary tabs **In Faraday · On {stick}** (F235). | Page |
| `Entry` | Add a key: a seed typed or scanned (§4.3). | Page |
| `Backup` | Backing up a wallet: the plan, the board (§4.17), each copy made and checked. A single key backed up as words opens on the quick paper backup (`docs/BACKUP-FLOW.md` §19): two item cards, Write the words down and Check my copy, with **Blank template** a text button whose sheet is a dialog with **Close** (`Sheet::QuickTemplate`); checked against Material 3's cards page and its dialogs page. | Step flow |
| `BackupFlow` | The shared backup flow for a key that is not a wallet's: where it will be kept, its sticks and vaults, the lock and the writes (`docs/BACKUP-FLOW.md`); a GPG key restored from its paperkey, with a head card of its public key and line 1 and the lines typed or a paperkey code scanned at step 4 (`docs/GPG-PAPERKEY.md` §4.2); also the setup of a new wallet's vaults, Bring in the sticks and Vaults. | Step flow |
| `Backups` | Every wallet known and where its backup is, a card each; on the small panel one wallet a page with **Previous · Next**. | Page |
| `Create` | Making a new wallet: Kind, for a single key its Backup card (Words or Shares), for a multisig the quorum (a multisig that is a descriptor with its path, BIP-48 by default or BIP-87 after Change, and the account), whether vaults are part of the backup, the keys (an empty slot of a single key backed up as words, or Key 1 of a multisig, holds the making of its key in place of New key: Material's segmented button **12 words** · **24 words**, neither selected until one is pressed, the dice pad under it once one is with the count still needed, **Make the words**, then the words masked with **Show words**, **Make again** and **Add a passphrase**, and one filled button, **Make the wallet** or **Lock in**, with **Other ways** a text button; F282), Check with **Back up now** at its foot (no Back up card). | Step flow |
| `Restore` | Restoring a wallet from its description and its seeds. | Step flow |
| `KeyGen` | Making a new key: Start (no choice ticked until one is pressed; SLIP-39 shares are chosen here, under Other forms), the length (BIP-39 words chosen, no count until one is pressed: 12 words and 24 words, the rest under **Other**, a menu of 15, 18 and 21; a count pressed is taken and Randomness opens, with no Continue), the source of randomness, its entries (§4.3) or, for This device, the Generator card with Add randomness's sources on it (Pointer and keys among them on the desktop app), Camera first and none chosen until one is pressed (no pad, key or camera live before that), each pad and its Add once chosen, "Randomness added to the pool" once any is added, and **Skip**, a text button at the start of the foot row while nothing is added, **Make the words** filled at its end, the passphrase, lock in; or, with Codex32 chosen, Make codex32 shares' cards (`docs/CODEX32.md` §6): Scheme, Identifier, Entropy, the entry with the worksheets filling live (§4.10 Worksheet), Worksheets, Strings and the Result **Strings made**. | Step flow |
| `Message` | Signing a message with a single-key wallet's key, or a loaded key's at an address Check an address found. | Step flow |
| `CheckMessage` | Checking a signed message against its address. | Page |
| `Decode` | A transaction taken apart, field by field. | Page |
| `OtsCheck` | A timestamp checked: four cards, **Proof**, **File**, **Block header** and **Result**, each closed to its name and what was taken; the proof and the file are chosen from Files (the file also as a typed SHA-256), the header typed or pasted as hex, scanned or chosen from Files, and **Result** lists each path with the block it reaches, or the one line saying why a take was refused. Checked against Material 3's cards page and its lists page. | Step flow |
| `CoreWallet` | A Bitcoin Core wallet file read: two cards, **File** and **Result**, each closed to its name and what was taken; the file is chosen from Files, a passphrase field (hidden, with an eye, for an encrypted wallet) and **Read**, and **Result** says whether the wallet is whole, its network, version, encryption, how many private keys were checked, each descriptor as Core lists it with its marks and range, the failed records one line each, and **Take to Wallets**, or the one line saying why the file was refused (`docs/decisions/F229.md`). Checked against Material 3's cards page and its text fields page. | Step flow |
| `Invoice` | A Lightning invoice taken apart: an outlined text field (Material's text field page, outlined) that always takes typing, **Decode**, **Scan** and **Clear**, then a row for each field the invoice or offer carries, name and value, or the one line saying why it was refused. Checked against Material 3's text fields page (outlined text field) and its lists page. | Page |
| `NonceCheck` | A signed PSBT no key here is left to sign, or a finished transaction, brought in by the camera, Files or Tools: the file's name, Send it's nonce check of it, whether it is finished, and **Save to a stick**, **Decode it**, **Show the raw hex** and **Show as QR** (`docs/FLOWS.md`, Nonce check). | Page |
| `SignerCheck` | Check a signer, from Tools: two PSBTs or finished transactions, chosen from Files or scanned, Sent and Returned, and one Result: the fee bump result where it applies, the verdict of what the signer changed where both are PSBTs, the facts, the findings and a line per difference; the Sent, Returned and Result cards, each file's name and its line "{n} signatures · {txid short form}" or "Finished · {txid short form}", **Scan**, **Change** and **Swap** (`docs/SIGNER-CHECK.md` §3, §4, §11; checked against Material 3's cards, lists and dividers pages). The seed step (SIGNER-CHECK §12): under the "Nonces recomputed" fact, a list row per key that signed and is not loaded (Material 3 Lists, one-line: the fingerprint, "Seed not loaded", a trailing text button **Load the seed**); the button opens a sheet titled "Seed of {fp}" (checked against Material 3's Dialogs page, a simple dialog of list rows) with **Type the words**, **Scan a SeedQR** and **Open a vault**; a seed whose fingerprint is another is refused with "Fingerprint {x}: not {fp}", and the right one brings the person back to the Result, checked again with the nonce lines. | Step flow |
| `Explore` | A loaded key at a path, opened from its row under Keys: the path field, the script, the address, public key and extended key; the account's addresses and Find an address (F76), and in the account's row **Make a wallet** (below, Make a wallet). Behind their eyes, on the stick and on mainnet too, each one press to show and one to hide and all closed when Explore opens: the key's secret (its words in their numbered grid and its passphrase, or its master seed in hex with its codex32 secret and the strings held), the root's and the account's extended private keys, and an address's private key (WIF) under the address and under each address on the Addresses page; leaving Explore, the idle warning and a lock hide them all (F125, `docs/KEYS.md` §3). | Page |
| `Bip85` | BIP-85's applications from a loaded key: a child seed's words, a private key, a password, at an index. | Step flow |
| `Convert` | Convert a seed: From a loaded key, an open vault's key, an LND aezeed or a typed SLIP-39 set, To a form that restores the same wallet (the words another way, SeedQR, hex entropy, master seed in hex, xprv, Seed XOR parts, codex32, SLIP-39) or, for a SLIP-39 set, Add a share, Replace a lost share and Rotate the set, with what is not offered dimmed and its line saying why; a Settings card for a split or a set action; Result with a masked value, its Facts, **Check my copy** and **Save as a file…** (`docs/SEED-CONVERSION.md` §6). | Step flow |
| `KefTool` | The Krux code tool in Tools: Contents (typed, or a loaded wallet's descriptor), the passphrase and Options, Make the code, and the Code card with Check my code, Copy by hand and Make another. | Step flow |
| `QrTool` | The QR vault tool in Tools: Contents (a typed note, or a loaded key), the passphrase with its memory step and name, and the Code card with Show as QR, PNG to the Outbox, To the Outbox and Make another. | Step flow |
| `Silent` | Silent payments: a loaded key's address, its labels and the payments checked. The Address card ends with a **DNS record** heading and two outlined text fields (Material 3 Text fields), **Name** and **Domain**, as Explore's fields are drawn; while either is empty nothing more shows. With both filled and valid (ASCII letters, digits and hyphens in labels of 1 to 63, dots between the domain's labels, the whole owner name within 253 characters) the card shows `₿name@domain` and the BIP-353 TXT record line in the address's monospace style, and one action, **DNS record as QR**, shown in the address's QR view as `silent-<fp>-dns`. A name or domain that is not valid shows a plain line under its field and no record. Nothing is resolved. | Step flow |
| `Vanity` | A vanity address for a loaded key: the kind (a script type or a silent payment code), the prefix, the dial, the grind and its find. | Step flow |
| `Lightning` | A Lightning node's key, from a loaded key (ldk-node's and Core Lightning's, one button each), an LND aezeed, Core Lightning's hsm_secret picked in Files (plain, encrypted or in its words form; a passphrase field only when the file asks, and empty is accepted) or English words typed; its node id with **Show as QR** and **PNG** on a row of their own (`docs/QR.md` §7). | Page |
| `Catalog` | Tools: every flow on one page, by group, each a name and one line (its standards are not shown, Find still finds a tool by them); the sidebar's Tools. Find a tool also finds the rows of Settings by their label and everyday words (Material 3's Lists): under a Settings heading each is an item with its section's name, and a press opens Settings at it, on a compact window the row's section as its own page, from medium up the page scrolled to its card; one search serves Tools and Home (`faraday/faraday-core/src/tools/search.rs`). Every flow a person starts a task from has a tile here, the Check place's rows included (`tools/lint-check-tiles.py`). | Page |
| `Tools` | The calculators: hashes, encodings, a descriptor's checksum, an xpub's spellings, units, the Miniscript compiler. | Page |
| `Vaults` | The vaults in Files and those open. | Page |
| `Unlock` | Unlocking one slot of a vault with its passphrase. | Page |
| `VaultContents` | An open vault: its kinds, the chosen kind's items (under a search bar for Wallets, Entries and GPG keys, §4.3; each title whole, wrapped where it does not fit), the chosen item, with the add actions; a wallet's item is its chart read parts first (§4.16); an item's secrets masked, each behind its eye (§4.10). On the small panel the kinds are a row of tabs over the list, and each keeps where its list was scrolled to (owner, 2026-10-26). | Page |
| `CreateVault` | Making a vault: where it will be opened, the cost of unlocking, the space per passphrase, its name and passphrases, its size. | Step flow |
| `Codes` | TOTP codes for the entries in open vaults, with the time they are computed for, under a search bar (§4.3, `docs/TOTP.md` §5). | Page |
| `SetTime` | Setting the time by hand: Date and Time (`docs/TOTP.md` §3). | Page |
| `Randomness` | Add randomness, from Settings or from the Randomness row at a press that draws the system's randomness (§4, Randomness row), its top app bar going back there: the status line, the label **Add randomness to the pool** over the source (Camera, Dice, Coins, Cards and, on the desktop app, Pointer and keys), none chosen and nothing started until one is pressed, and New key's pad for it, the batch and its Add, or Pointer and keys' area and its count; once a batch is added, a check mark and "Added to the pool 3 times · last: 6 dice rolls", a pressed state layer easing away behind it at each batch (`docs/OWN-RANDOMNESS.md` §2.1, §2.2). New key's Generator card draws the same sources on itself. | Page |
| `Settings` | Mode; Lock and power (the keys loaded, Lock, Lock after, Power off after); the time; Randomness (the status line and Add randomness, `docs/OWN-RANDOMNESS.md` §2.4); Appearance (Theme as filter chips, one per theme, Cobalt's marked Default; Mode, Light · Dark; Contrast, Standard · Medium · High; Text size, Standard · Large · Larger, `docs/TEXT-SIZE.md` §2; the Reduce motion switch, THEMES §12); Display (Scale, and on a 7-inch or 10-inch panel Rotation, Landscape · Portrait, `docs/ORIENTATION.md` §6); What is remembered (Amounts signed, In an open vault · Until power-off; Signatures made, for the nonce check, In an open vault · Off, `docs/FLOWS.md`, Nonce history); About. In About, Run again runs the self-test in the press; when it ends a snackbar (Material 3's Snackbar) says "Self-test passed: N checks" or "Self-test failed: <check>" and the result line shows the fresh result. On a compact window a list of the sections, each its own page; from medium up every card on one page (`docs/LAYOUT.md` §17). | Page |
| `Transfer` | The desktop app as the device's QR link: files and text out as codes, in by camera. Only in the desktop app. Send text is an outlined multi-line text field with its count or refusal under it and Paste, Clear and Show as QR (Material 3 Text fields, Cards, Buttons). | Page |

**Check** (F255; Material 3 Cards). A page under the top app bar named
**Check**, opened from its row right under Home on the expanded rail and
the navigation rail, from the Pi's Home menu, where it is the first
button, and from Home's Check tile (**Check a signer**, "Its
seed, addresses and signatures", the fourth tile, always). A row has
the name of the tool it opens, the same on its Tools tile and in the top
app bar of its page ("Check an address"). A tile with no **Seed** tag
keeps no row for tags: it is as tall as its name and its line.
Each row is a tile in the grid Tools draws (`screens/catalog.rs`, the
one drawing of a tile and its columns, `docs/LAYOUT.md` §17): the label
as its name, the line under it saying what the person needs in hand, and
nothing else on the page, no explainer. The tiles stand in as many
columns as fit, at least three beside the navigation and one on a small
panel, under a subheader per group with Tools' spacing (`section` in
`screens/catalog.rs`, both pages' one call) and no line between groups; a
name or line that does not fit in a tile wraps and the tile grows, never
an ellipsis; the page scrolls. Rows, in order (label, then its line, then what a press opens):

- **The wallet**
  1. **My seed makes my wallet**, "Its words, its passphrase and the
     wallet app": the flow that turns the words and passphrase into the
     fingerprint, the xpub and the first addresses, shown to be held
     against what the wallet app shows (`SeedCheck` below).
  2. **Check an address**, "The address and the wallet loaded
     here": Check an address.
- **Signing**
  3. **Decode a transaction**, "The PSBT, as a file or a QR
     code": Decode a transaction.
  4. **Check a signer**, "The PSBT sent and the one returned": Check a
     signer, which also compares two transactions
     (`docs/SIGNER-CHECK.md` §11).
  5. **Nonce check**, "A signed PSBT or transaction":
     Nonce check, for a signed PSBT alone.
- **Setup and backups**
  6. **Check a seed made from dice**, "The rolls and the words made from
     them": New key at its dice source, the method (Coldcard, SeedSigner,
     Keystone, BitBox) chosen there.
  7. **Check a multisig setup**, "The wallet's descriptor or setup file": Check
     a multisig setup, its quorum, checksum and cosigners
     (`MultisigCheck` below).
  8. **Check a backup in another form**, "Shares, parts, a SeedQR, codex32 or a
     Krux code": a menu (Material 3 Menus) of **SLIP-39 shares**, **Seed
     XOR parts**, **SeedQR**, **codex32** and **Krux code**, each opening
     its tool as Tools does, the Krux code the scanner.
- **Messages and keys**
  9. **Check a signed message**, "The message, its signature and the address":
     Check a signed message.
  10. **Child seeds and passwords**, "The parent seed and the child's index": Child
      seeds and passwords (BIP-85).
  11. **Convert an xpub**, "An extended public key in any
      spelling": Convert an xpub, in the calculators.

**Seed** tags rows 1, 6, 8 and 10, as a standard is tagged on a Tools tile: the rows keep the order of
checking a hardware wallet (its seed, what it shows, what it signs, its
setup and backups, the rest), which putting the seed-free ones first in
each group would break, and the mark lets a person who keeps the seed
away from this computer pass them over at a glance. A row whose tool is
not built is not drawn. A tool opened from Check has **Check** as its way
back ("Back to Check", F146). There is no row for a firmware file, a
timestamp or a Lightning invoice.

**Check an address** (the row 2 tool, also in Tools; Material 3 Text
fields, Buttons, Menus and Cards, checked against those pages). It is
a page of its own, under the top app bar
titled **Check an address**, in the reading column. At its top an outlined
text field labelled **Address** (the address keyboard of Explore's Find an
address, `faraday/faraday-core/src/explore/addresses.rs`, `KeyboardKind`,
under it on the small panel; a physical keyboard types into it as well),
and under the field a row of the other ways in, each its own button with
an icon and a label: **Scan** (Material's filled tonal button, the first
way, as today's scanner), **Paste** (a text button, Faraday Demo only, as
the scanner's Paste), and **From a file** (a text button, drawn only when
Files holds a text file). **From a file** opens a menu (Material 3 Menus,
a dropdown under its button, as New key's Other menu is drawn) of the
files in Files that are text, by name, newest first; a press reads that
file. A typed address is checked when Enter or the keyboard's Done is
pressed, and a pasted or file address at once; Scan reads at once as it
does. Whichever way in, the text goes through the pasted-text classifier
(`docs/SCANNING.md` §4.6), then to the same check the scanned code
reaches: `address_note`'s search of the first 100 receive and change
addresses of each loaded wallet and of each loaded key's standard
accounts that no loaded wallet is, and on a miss **Look further** to each
depth and **Stop** (`docs/QR.md` §2); what it finds is the result card
under the row (The result, below). A
`bitcoin:` link typed, pasted or in a file is read to BIP 21 first, as a
scanned one is (`core/faraday-codec/src/bip21.rs`), and its address is the
one checked. With no wallet or key loaded the page reads **Load a key or
a wallet first** in place of the answer and the ways in stay drawn.

What each refuses, in the card's place under the field, in the error
colour and as the field's supporting text (Material 3 Text fields, error
state), never a toast: text that is not an address or a `bitcoin:` link
("Not a Bitcoin address"), an address of another network than the loaded
wallets' ("A testnet address; this wallet is mainnet", by the loaded
wallet's network), a `bitcoin:` link with a `req-` parameter or not well
formed (the line `address_note` gives a scanned one), a secret (a seed, a
key, a descriptor holding one, as the scanner's Paste refuses it:
`explore_pasted` and `pasted_secret`, so nothing of it is kept or drawn,
and the field is cleared), and a Lightning invoice or any other kind of
code ("Not an address"). A pasted or file text that is a PSBT, a wallet
description or another kind that has a destination is named by the
classifier's line as a scan names it, not opened here. A file that is not
text, empty, or over 64 KiB is named in the menu's place ("Not a text
file"), nothing read.

A file holding several addresses (one a line, or a list after other
text) is read for its addresses: with one, it is checked as typed; with
more than one, the page lists them in a Material list (two-line items:
the address in short form, §4.5, then "Savings · receive 12" or **Not
found**), each checked against the first 100 on reading (a compute, `c:
&Compute`, as a scan's search is), and a press on a row shows that
address's result card in the list's place, Back returning to the list;
a miss's **Look further** is on its card, for that address alone. At most 100 addresses are read from a file; one past that
is "100 of {n} addresses read". A file that is a descriptor or holds a
secret is not read as addresses: a descriptor is named "A wallet
description, not an address" (it is not opened here), and a secret is refused as a paste is. There is no NFC.

**The result** (Material 3 Cards and Lists, checked against those pages:
an outlined card holding a list of facts). One card under the ways in
(§4.11, Card, `Ui::card`), in the reading column; on the small panel the
panel's width less its margin, the page scrolling under the bar. The card
holds a Result line (§4.11) and then Facts (§4.11, `kv`), one fact a row
with a rule between, nothing else: no explainer. The
address is always the second row: §4.5's Whole string with its label,
the one form a whole address takes wherever a person compares it with
another device's screen. It opens nothing here: it came from elsewhere to
be checked, not to be given out. A `bitcoin:` link shows the address it
carries. Indices count from 0, as the path's last step does.

A find in a single-key wallet, in order:

- the Result line in `OK` with its icon, **Found in Savings** (the
  wallet's name);
- **Address**: the address, whole, mono, in groups of four;
- **Place**: **Receive · index 12** or **Change · index 3**;
- **Wallet**: the wallet's shape, "Single key · native SegWit" (§4.6,
  Script type);
- **Path**: the key's fingerprint and the address's full path, mono,
  "73c5da0a · m/84h/0h/0h/0/12" (§4.6, Path), wrapping after the
  fingerprint where the line does not hold both.

Where the find is in a single-key wallet whose key is loaded (receive or
change), the card ends with **Sign a message with this address**, a text
button (Material 3 Buttons, Text), which opens Sign a message (§5,
`Message`) with its Address card done. A receive find opens it with that
wallet and index chosen. A change find opens it on the key's path: the
Address card shows the address, "Change address 1/3" and the Path line
(§4.6), with no **-** or **+**. A multisig or miniscript wallet, a
watch-only wallet and a wallet whose key is not loaded offer nothing.
A receive address of a key with no wallet opens on the path as well: the
Address card shows "Receive address 0/n" and the Path line, with no wallet
buttons.

A find in a multisig or miniscript wallet is the same card with the
wallet's shape ("2 of 3 · native SegWit multisig") and, in place of
**Path**, **Keys**: one line for each key of the descriptor, in its order,
each the key's fingerprint and its full path to this address, mono,
"73c5da0a · m/48h/0h/0h/2h/0/12", each wrapping after its fingerprint on
its own where it does not fit. A key with no origin is not in a loaded
wallet (a bare xpub is refused), so every line has both.

A find in a loaded key's standard account, where no wallet is made from
it: the Result line **Found in key Alice** (the key's name); **Address**;
**Place**; **Account** in place of **Wallet**, the account's script and
number, "Native SegWit · account 0"; and **Path** as a single-key find
has it. The card ends with **Sign a message with this address** (text)
and **Make a wallet**, filled tonal, which makes
the wallet of that key, path and script (Make a wallet, below; **Open
the wallet** where one is loaded).

A miss, in order:

- the Result line in `WARN` with the warning glyph, **Not found**: the
  address is not shown to be the person's, which is what the check is for;
- **Address**, as a find has it;
- **Searched**: how far it looked and in what, "The first 100 receive and
  change addresses of Savings" (the wallet's name where it is the only one
  searched), "… of 3 wallets", "… of 4 key accounts", "… of 2 wallets and
  4 key accounts";
- at the card's foot, **Look further: 1,000** and **Look further:
  10,000**, text buttons, each drawn while that depth is not yet
  searched (§4.13). While it looks, **Searched** reads "Looking: 340 of
  1,000" with Material's determinate linear indicator under it (§4.14,
  Progress), and **Stop** is the one button; a find while looking turns
  the card into the find's card, and a look that ends without one leaves
  the miss's card with **Searched** at the new depth.

With nothing loaded, or a refusal, there is no card: the page reads **Load
a key or a wallet first**, or the refusal under the field, as above.

Every scanner hands an address to this check: Home's Scan opens Check an
address with the address read checked, as it goes straight on for a seed,
and any other scanner's read card names it **An address** with **Check the
address** as its way over, which opens the page the same way; the read
card's one line for an address and its **Look further** go, so the result
and the search are on this page alone. When this is built,
`docs/SCANNING.md` §5.2, §5.4 and §7.3 and `docs/QR.md` §2's address row
say so, and the address lines of `address_note` that this card replaces
leave `faraday/faraday-strings/src/en/`.

The strings it adds (proposed, for `faraday/faraday-strings/src/en/`):

- `address_check_found`: "Found in {}" (a wallet's name)
- `address_check_found_key`: "Found in key {}" (a key's name)
- `address_check_missed`: "Not found"
- `address_check_load_first`: "Load a key or a wallet first" (Back up
  a wallet's "Load a wallet first" stays its own)
- `address_check_address`: "Address"
- `address_check_place`: "Place"
- `address_check_receive`: "Receive · index {}"
- `address_check_change`: "Change · index {}"
- `address_check_wallet`: "Wallet"
- `address_check_account`: "Account"
- `address_check_account_value`: "{} · account {}" (the script's name, the
  account number)
- `address_check_path`: "Path"
- `address_check_keys`: "Keys"
- `address_check_key_path`: "{} · {}" (a fingerprint, a path)
- `address_check_searched`: "Searched"
- `address_check_searched_value`: "The first {} receive and change
  addresses of {}" (the depth; a wallet's name or a count below)
- `address_check_wallets`: "{} wallets"
- `address_check_accounts`: "{} key accounts"
- `address_check_wallets_accounts`: "{} wallets and {} key accounts"
- `read_go_address`: "Check the address"

- `address_check_list_receive`: "{} · receive {}" and
  `address_check_list_change`: "{} · change {}" (a wallet's or key's name,
  the index), the list's second lines

The look's progress and buttons keep their strings (`read_look_progress`,
`read_look_further`, `read_look_stop`).

A typed address is never kept past leaving the page, a lock or the idle
lock, and an address is public, so Faraday Demo's paste and the device's
typing keep it in no log. A new way in for untrusted text has its parser
fuzzed (`faraday/fuzz/README.md`): the typed, pasted and file text reach
`faraday_codec::bip21::parse` and `faraday_bip::address::parse`, which the
`bip21` and `address` targets of `fuzz/` already cover, and the pasted-text
classifier, which `classify` covers; the file's finding of addresses in
text (several to a file, the limit) is new parsing and gets a target of
its own in `faraday/fuzz/`, with a seed corpus of files holding one
address, many, a descriptor and noise.

The guide's line (`docs/guide/catalog.md`, when this is built): "**Check
an address** takes an address scanned, typed on the address keyboard,
pasted on Faraday Demo or read from a text file in Files, and a `bitcoin:`
link the same way, and searches the loaded wallets and keys; it reads **Load a
key or a wallet first** until one is loaded."

**Make a wallet** (on Explore's path page and Find an address
at a BIP-44, BIP-49, BIP-84 or BIP-86 path on the network's coin type,
and the address check's find in a loaded key's account; Material 3
Buttons, "Common buttons", and Dialogs, the basic dialog, checked
against those pages). The steps, the default name, the duplicate case,
the first-address check and what is refused are `docs/FLOWS.md`
(Explore, Make a wallet at the path); this is how it is drawn.

- **Explore's path page.** **Make a wallet** is the third button of the
  account's row, after **Addresses** and **Find an address**, alike in
  style (`Style::Secondary`, `faraday_ui::parts::button_rows`, wrapping
  at the width on the small panel), drawn wherever that row is. The row
  stays where it is, at the page's foot: under the address, its private
  key's eye, the private key and the line under it at an unhardened last
  step ("With the account's extended public key, this key gives every key
  of the account"), the public key, the extended key and the extended
  private keys' eye. The new button moves none of them, hides none and
  opens no eye; that line shows whenever the private key does, as
  before.
- **Find an address.** Under a search's matches, once it has ended with
  at least one, a row of one button, **Make a wallet**, `Style::Secondary`
  (Search stays the page's filled button); nothing while it looks, and
  nothing on a miss.
- **The address check.** On Check an address's page the result card of
  a find in a key's account ends with **Make a wallet**, filled tonal
  (Check an address, The result); every scanner's read card hands an
  address to that page, so the read card has no such button.
- **Already loaded.** Each of these reads **Open the wallet** where a
  loaded wallet is that account, and opens its card.
- **Wallet made.** The Dialog (§4.13, `faraday_ui::sheet::dialog`):
  its head the wallet's name; its body the descriptor row
  (`descriptor_row`, opening the wallet's code) and the first addresses
  under the part's own heading (`first_addresses`, each whole in
  groups of four, §4.5, the first receive address opening its QR sheet);
  its actions **Done** (a text button) then **Open the wallet**
  (filled), dismissive first. A press beside it is Done.
- **Refused at the first-address check.** One line in the error colour
  under the button that was pressed, the way a step card's error line
  is drawn, and no dialog.

Strings (`faraday/faraday-strings/src/en/`; the address check's find uses the first two):

- `explore_make_wallet`: "Make a wallet"
- `explore_open_wallet`: "Open the wallet"
- `made_wallet_name`: "{} · {}" (the key's name, the account's path)
- `made_wallet_differs`: "Its first address is not the one this key
  gives at {}: not added" (the account's path)
- `made_wallet_done`: "Done"
- `made_wallet_open`: "Open the wallet"

The dialog's head is the wallet's name and its addresses' label is
Create's `create_first_addresses`, so they need no string of their own.

Tools (`faraday/faraday-core/src/catalog.rs`) stays the list of every
feature: each tool Check opens is in Tools as well, Check is a second
way in and never the only one, and a check added to Check is added to
Tools in the same change. Every label and line is in
`faraday/faraday-strings/src/en/`.

The sheets (`Sheet` in `faraday/faraday-core/src/lib.rs`, drawn in
`faraday/faraday-core/src/screens/sheets.rs` and the area that owns
them) include the QR sheet and the scanner (§4.9), the Guide sheet, the word list (a tab per list, Back, and at the start of the row under them a search field that finds a word by its word, number, bits or dice and lists only what it finds), a secret on its
way out (§4.10), the lock and its question, the idle warning and the
locked sheet (§4.14), power, a new input device (a held device's sheet shows only once the device is used, a toast naming it at plug-in, F202, none for what was plugged in at start, which one snackbar lists for 10 s over any screen and sheet, with Material's close button, a `TOUCH` square × at its end, and Escape to take it away early, nothing else dismissing it and a held device never; a dialog; for a new USB
touchscreen, and for a new pointer while nothing trusted can point, the whole
window instead: its title, name and **Tap the target** (**Press the
target** for a pointer), on the line under it **Target 1 of 3** at the
left and **Tries left** at the right once a press missed, the target
showing, a filled icon button, `ICON_BUTTON` round in a `TOUCH` target,
at the place drawn for it between them and **Ignore this device** at the
bottom end, and for a pointer a line saying Enter on a trusted keyboard also lets it in, F153, F202: how many targets in a row comes from the room
left and `TOUCH`, and a screen too small for any count shows none; the
toast that says a device is in is drawn over this sheet and the
dialog),
what is attached that
can capture the screen or the keys, a seed read by the camera, a chart
node's sheet (§4.16; a standard side sheet beside the chart on the wide
layout, the one sheet that leaves the page under it live), the board's list, its balance and partials sheets
and its ⋮ menu (§4.17), a file's ⋮ menu (§4.1), Rename and Details, New
key's Other menu of word counts (drawn as a ⋮ menu is, under its button), the
mode sheet, Load samples (§4.8), Restore a GPG key (the public keys in Files
that a GPG key is restored against, and **Scan the public key**, `docs/GPG-PAPERKEY.md` §4.1), the wallet confirmation sheet (a wallet
of several keys or one a key not of its own spends alone, its rows,
Cancel and **Hold to add this wallet**; F254), **Which account to load?** (`Account`: a Coldcard export
holding a single-sig account and a silent payments account, the file's
name, **Cancel**, **Single-sig · BIP-84** and **Silent payments ·
BIP-352**, the two alike; `docs/INTEROP.md` §5, BIP-392 sp()), the register sheet (the wallet a spend's PSBT defines,
its shape and name and the wallet confirmation sheet's rows, Cancel and **Hold to register**, which is the wallet's one confirmation; F135, F254),
the older-copy dialog an unlock stops at (the vault's name, what its seal
count found, **Keep locked** and **Open**; `docs/VAULT.md`
§11), **Remove codex32 set** and **Remove BIP-85 origin** (`Trim`: the
vault's name, that copies sealed before still hold it, **Cancel** and
**Remove**; `docs/VAULT.md` §7), **Record it as a child seed?** (`ChildRecord`: BIP-85 child words
held before Save into a vault or Load as a key, what each answer keeps,
**Don't record** and **Record**, alike; a press beside it or Escape puts
the words nowhere; F166), **Save into {vault}** (`SaveKeeps`: a seed with a
codex32 set or a BIP-85 origin about to be saved from Vaults or by a backup
flow's press (each such seed by its label when there are several), **Keep its
codex32 set** and **Keep its BIP-85 origin** ticked, **Cancel** and **Save**;
`docs/VAULT.md` §7), **A BIP-85 child seed** (`ChildNotice`: a
wallet just made with a key recorded as a child seed, that Faraday
records it only inside a vault and that nothing written for its backup
says so, **OK**; once per wallet made; F166), the Sweep sheet (a WIF
private key typed or scanned: the key's field, then its addresses with
their QR codes, then its PSBT's review and **Hold to sign**, **Done**
erasing the key; `docs/INTEROP.md` §5, Sweeping a private key, F171), **{file} reads as a seed or as text** (`TextSeed`: a picture on a stick whose code is 16 or 32 bytes of text that also read as a CompactSeedQR, the file's name, **Leave as a file** and **Load as a key**, a dialog, one for each such picture in turn; `docs/SCANNING.md` §1), and the question a
flow's Cancel asks (§4.13).

In Faraday (`faraday-ui`) every page's working area is one colour, the
theme's surface (`SURFACE`), and so is a sheet; the sidebar keeps its
own shade (`SIDEBAR`). A step flow's steps are flat sections divided by
a thin rule in `LINE`, with no fill and no box. The right-hand column
(Back up's list or Map, the side panel, `side_panel_w`) is the same
colour, divided from the page by the same rule. A card or tile on the
page is set apart by its edge alone, `BORDER` (`docs/DECISIONS.md` F39).

### My seed makes my wallet (`SeedCheck`)

A seed typed or scanned, and its passphrase, shown as what it
makes at the account picked, for the person to hold against the wallet
app and the hardware wallet; opened from its Tools tile and from the Check
screen's first row. Its steps, what it reads and what it decides
are `docs/FLOWS.md`, My seed makes my wallet.

| `Screen` | What it holds | Drawn as |
|---|---|---|
| `SeedCheck` | My seed makes my wallet: the cards **Seed**, **Passphrase**, **Account** and **Result**, under a top app bar titled "My seed makes my wallet" with its way back and **Cancel**. | Step flow |

| Card | Open | Closed |
|---|---|---|
| Seed | Three ways in, a list row each (§4.1): **Type the words**, which opens Add a key's BIP-39 form in the card (the count, the boxes, §4.3), **Scan a SeedQR**, and **A key loaded here** where any is, its keys as rows by fingerprint (§4.4) with their "passphrase" mark. | "{n} words", or the key's fingerprint, and **Change**. |
| Passphrase | **No passphrase** and **Add a passphrase**; then **Passphrase** and **Passphrase again**, outlined text fields each with its eye (§4.10), and **Use this passphrase** once they match. | "Fingerprint 73c5da0a", " · with a passphrase" where one was given; **Change**. |
| Account | A two-line list row each (§4.1): **Native SegWit**, **Taproot**, **Nested SegWit**, **Legacy** and **Multisig**, each with its path in §4.6's style and, as its supporting line, the BIP and "Addresses start bc1q" (bc1p, 3, 1; "A cosigner's key" for Multisig), at account 0 on the network in use; then **Another account or path**, which opens Explore's path field and script buttons (§4.6) in the card with **Use this path**. | "Native SegWit · m/84h/0h/0h" and **Change**. |
| Result | Fact rows (§4.11): **Fingerprint** (" · with a passphrase" where one was given), **Without the passphrase** where one was given, **Path** (§4.6), **Script** (§4.6's words), **Xpub** with its origin, a segmented button (§4.2) over it of the SLIP-132 spelling and **xpub** where the script has one, **First receive address** and **First change address** (none for Multisig), each string whole in groups of four (§4.5); the xpub and the receive address open their QR sheet with a chevron, the change address nothing. **Compare with the wallet app's**, a text button, opens under the rows one outlined text field with **Scan** and **From Files** beside it, and its answer as a result line (§4.11): "Matches" in `OK` (or "This seed is key 2 of this wallet · 2 of 3"), "Does not match" in `ERR` with its one line where one says more, the warning glyph where a file's origin names this seed and its key is not; a value refused shows its line under the field in `ERR`; an address search shows Explore's determinate progress (§4.14). The actions: **Another account** and **Another seed** (text buttons), **Keep this key** (outlined; not shown for a key loaded here; dimmed with "Remove the stick first" while a stick is in) and **Done** (filled). | — |

**Done** drops the key and **Keep this key** loads it as Add a key does,
each going back where the flow was opened. In Practice the Seed card and
the compare field hand their sample to the host strip (§4.19). Checked
against Material 3's Lists (the ways in, the key rows, the account rows
as two-line items and the fact rows), Text fields (outlined, the
passphrase and the compared value), Segmented buttons (the xpub's
spelling), Buttons (filled, outlined and text, one filled action per
card) and Progress indicators (linear, determinate) pages.

### A multisig setup (`MultisigCheck`)

Built. A wallet of several keys, read from its descriptor, a BIP-388
policy, a multisig setup file or a BIP 129 record, shown as its quorum,
its descriptor checksum and each cosigner, for the person to hold against
every other signer's device and the coordinator; with a key loaded here
or a seed given, which cosigner that key is. Opened from the Check
screen's row 7, from its Tools tile and from the ⋮ menu of a wallet file's row in Files (**Check this setup**, on a file Files reads as a wallet of more than one key, never on a single-key wallet file), which opens it on that file with the Setup card closed and the Result, or the refusal line, shown. Its steps, what it reads and what
it refuses are `docs/FLOWS.md`, Check a multisig setup.

| `Screen` | What it holds | Drawn as |
|---|---|---|
| `MultisigCheck` | Check a multisig setup: the cards **Setup** and **Result**, under a top app bar titled "Check a multisig setup" with its way back and **Cancel**. | Step flow |

| Card | Open | Closed |
|---|---|---|
| Setup | The ways in, a list row each (§4.1): **Scan** (the scanner titled "Scan the wallet's setup"), **From Files**, which opens the files Files reads as a wallet, a row each with its name and the shape it reads as ("2 of 3 · native SegWit multisig"), and **A wallet loaded here** where any of several keys is, its wallets as rows by name and shape. On Faraday Demo, **Paste** as well, in the Demo only card (`docs/DEMO.md` §14), its setup named "Pasted". A refusal stands under the rows as a result line (§4.11) in `ERR`. Opened from a file's ⋮ in Files it is closed from the start, on that file's name and shape. | The file's name ("Scanned" for a code, the wallet's name for a loaded one) and its shape, and **Change**. |
| Result | The rows of the wallet confirmation sheet (F254), drawn by the same code: the shape and the name the setup gives; then per cosigner, in the descriptor's order, the key block (§4.4): **Key 1**, **Key 2**, … with the fingerprint, the path (§4.6), the extended public key whole in groups of four (§4.5), and its state, "Yours, can sign" where a key loaded here gives that extended key at that path, **Checked by you** or **Not checked** with the tick, or the red mark and its line where the key claims a loaded key's fingerprint and is not that key; then the **Descriptor checksum**, and a `tr()` tree's key path line (`docs/WALLETS.md` §3.4). Under them, the page's own fact rows (§4.11): **Quorum** where the shape does not state it, **Ways to spend** for a script tree or a miniscript, and **First receive address**, whole, opening its QR sheet with a chevron. **Check a seed against it**, a text button, opens under the rows the Seed and Passphrase steps of My seed makes my wallet (`SeedCheck` above) and its answer as a result line: "This seed is key 2 of this wallet" in `OK`, the matched cosigner's row marked "This seed"; or Restore's "Not a key of this wallet: check the words or the passphrase" in `ERR`, with the warning glyph and its line where the fingerprint is a cosigner's and the key is not. Where the wallet is loaded (the policy, or the same keys in another order, at the place the Result matched, while the wallet there is still that one), the line "Loaded here as {name}" in `OK` stands above the Check a seed button. The actions, one under another: **Open the wallet** (`Primary`) while that wallet is still loaded, else **Add this wallet** (`Primary`); **Another setup** (`Ghost`); **Done** (`Ghost`). | — |

The key marks and ticks are one block drawn with `KeyBlockOf::Check`: for a wallet loaded here (matched by its policy, or by its keys in another order) they are that wallet's own, so a press ticks the wallet's key and the wallet's sheet shows it; for a setup not loaded the ticks are the Result's own, kept while the Result stands, and **Add this wallet** carries them to the confirmation sheet and into the wallet when it is added. The Result's top rows are the confirmation sheet's body (F254, the
sheet's Dialog in §4.13), so a person who has checked a setup here sees
the same rows in the same order when the wallet is added, and the other
way round. The sheet's last line, saying to compare with every other
signer, is left out here: comparing is what the page is for, and its
title says so. **Add this wallet** never adds the wallet from the page:
it opens that sheet over the page, and only the sheet's **Hold to add
this wallet** adds it, as every wallet of more than one key comes in;
the Result then reads "Loaded here as {name}" above **Open the wallet**. A
wallet already loaded (the same policy) reads so from the start; one taken out since the Result was made offers **Add this wallet** again. A refusal in the Setup card's place draws no
Result at all: nothing of a setup refused is shown as if it held. In
Practice the Setup card and the seed's steps hand their samples to the
host strip (§4.19). Checked against Material 3's Lists (the ways in, the
file and wallet rows, the cosigner and fact rows), Cards, Buttons
(filled, outlined and text, one filled action per card), Text fields
(the seed's steps, as `SeedCheck` uses them) and Dialogs (the
confirmation sheet Add this wallet opens) pages.

### The wallet confirmation sheet (`Sheet::WalletConfirm`)

Built. A wallet of more than one key comes in through this sheet,
however it arrives, unless every one of its keys is a key loaded here at
its path; so does a `tr()` wallet a key not of its own spends alone
(F254, F132). Its screen is `screens::sheets::wallet_confirm_sheet`,
the §4.13 Dialog: the head **Add this wallet** (with "{i} of {n}" while
several wait, one sheet at a time), the wallet's name, the rows Check a
multisig setup's Result shows (the shape; each cosigner's number,
fingerprint, path, the whole extended key in groups of four, "Yours" or
"Yours · your single-key account" or the wrong-key line; the descriptor
checksum; a tree's key path line), the line to compare them by, and
**Hold to add this wallet**, with **Cancel** the dialog's action. When a
key not of the wallet spends it alone the head, that line and the hold
are in the error tone. What is above the actions scrolls, so the hold is
reached after the keys. Cancel and Escape add nothing and open the next
wallet's sheet; there is no Cancel all. The rows are worked out when the
wallet is held back, never while drawing. Checked against Material 3's
Dialogs page.

## 9. Checklist for a change

Read by whoever makes or reviews a change in this area; no script holds these.

- The medium's words come from Medium; "card or stick" on the Pi, "SD card" only for the card it started from.
- A screen or component change names the Material 3 page it was checked against.
- None of F111's left-out components is added.
- Faraday Demo differs from the device only by a decision.
- The Starter's maker pages show several families at once, no model, no maker's screenshot or name.
