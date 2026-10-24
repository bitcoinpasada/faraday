# Text size

**Kind:** Spec
**Status:** Specified, not built (F85)
**Open:** all of it: the setting and its storage (T1), components that grow with their text (T2), the screens at Larger (T3), the Starter (T4), the tours and tests (T5); planned after the structure pass.

Settings › Appearance gains **Text size**: Standard, Large, Larger. It
makes text larger and lets the layout make room for it. It is kept as
the theme is kept. The Starter has the same setting. The owner's
choices are F85; §9 records the owner's answers (2026-10-23).

## 1. How type size works now

- **Design units.** A screen is laid out in dp. `Faraday::display`
  (`faraday/faraday-core/src/lib.rs`) works out `f`, the pixels per dp:
  the window's fit to 1280 × 800 dp on a wide display (no more than a
  quarter beyond the screen's density), the density itself on a small
  panel and inside the Starter, times the person's **Display scale**
  (`scale_pct`, Settings' 75 % to 200 %). Every length and every text
  size is multiplied by `f`.
- **The size class's type scale.** `faraday_draw::geom::Scale` carries
  `tokens::type_scale_pct` (Small 100, Mobile 110, Wide 130) and the
  narrow cut on Small (`tokens::narrow_text`). The class is
  `SizeClass::of` the canvas's display, by its diagonal in inches.
- **faraday-ui cancels the class's scale.** `Ui::font`
  (`faraday/faraday-ui/src/lib.rs`) divides a size by `tr`, the canvas's
  type factor over its length factor, so a size a Faraday or Starter
  screen asks for is that many dp at every class. The class's type scale
  reaches only what `faraday-draw` draws with `Font` directly. The narrow
  cut on Small stays.
- **The ramp.** `core/faraday-draw/src/tokens.rs` names TITLE 20, BODY
  16, LABEL 14, CAPTION 12 and a few sizes of their own
  (`FIELD_ACTION_TEXT`, `SCAN_TILE_TEXT`, `PASS_CODE_TEXT`,
  `VIEWFINDER_STATE_SIZE`). Many screens still write a size in place
  (`ui.text(x, y, 13.0, …)`) and a line's advance in place (`y += 24.0`);
  `tools/lint-tokens.counts` counts them.
- **Heights.** Rows, buttons and fields have heights named in tokens
  (`ROW` 56, `MENU_ROW` 72, `MENU_ROW_SMALL` 52, `FIELD` 52,
  `TEXT_FIELD` 40) or written in place (`wrap_buttons(…, 38.0, …)`).
  Text is placed in them by `text_mid`. Paragraphs wrap (`Ui::wrap`,
  `wrap_lines`, `parts::measure_wrap`) and their cards grow with them.
- **The layout.** `self.compact` (one column) is decided from the
  window before the Display scale is applied (§1 finding below).
- **Kept.** `stick/settings.rs`'s `settings_body` writes `scale=`,
  `theme=`, `motion=` and the rest; the kept set carries it across a
  lock, and `faraday-settings.txt` on the boot stick across a power-off.
  The desktop app's kept set is in memory (`Boxes::Memory`), so it lasts
  across a lock but not across quitting. The Starter keeps nothing
  (`docs/STARTER.md` §5.1: "there is no settings or state file") and
  has no theme of its own to choose.

**Finding.** The Display scale is applied after the one-column decision.
A 1280 × 800 window at 160 dpi with Display scale 200 % keeps the wide
layout and draws it into 640 × 400 dp, so the sidebar and the page are
cut off. T1 makes the Display scale count in the same decision as Text
size (§4), which removes this.

## 2. The setting

| Choice | Factor | Token |
|---|---|---|
| Standard | × 1.0 | none |
| Large | × 1.2 | `TEXT_LARGE_PCT` = 120 |
| Larger | × 1.4 | `TEXT_LARGER_PCT` = 140 |

- `faraday_ui::TextSize` (beside `Theme`): `Standard`, `Large`,
  `Larger`, with `id()` (`standard`, `large`, `larger`), `from_id` and
  `pct()`, read from the two tokens. One type for Faraday and the
  Starter.
- The factor multiplies the size a screen asks for, after the session's
  `f` and the Display scale, and leaves the class's own handling as it
  is: the narrow cut on Small stays, and `Ui::font` still cancels the
  class's ramp for faraday-ui text.
- Settings › Appearance: a **Text size** row under Contrast, three
  options as Contrast's are, the chosen one selected. Pressing one lays the
  screen out again at once (as `Action::Scale` does: `display` again
  with the last display) and saves the kept set.
- The strings, in `faraday/faraday-strings/src/en.rs`:
  `settings_text_size` "Text size", `settings_text_standard`
  "Standard", `settings_text_large` "Large", `settings_text_larger`
  "Larger".

## 3. What scales and what does not

The factor is applied in one place, `Ui::font`, so `measure`, `line`,
`wrap`, `wrap_lines`, `text` and everything built on them see the larger
size. What must keep its size draws inside `Ui::fixed_text`, which sets
the factor to 1 for what it draws and puts it back.

| Text | At Large and Larger |
|---|---|
| Paragraphs, notes, Learn and About pages, sheets' text, step text, results | Larger; wraps; the card or page grows and scrolls. |
| A row's label and value (list rows, menu rows, value rows, stacked rows) | Larger; wraps to a second line where it does not fit; the row grows to its lines (§5). |
| Titles: a page's title, a card's title | Larger; wraps. |
| A button's label, a choice's label | Larger; the button grows to its line (height) and `wrap_buttons` puts fewer on a line. |
| One-line slots that cannot grow: the small panel's bar title, a mode pill, a chip, a tag, a board chip, a chart node's name, a tab, the viewfinder's state line, the field above a keyboard | Fit-down: the largest size from the asked size × factor down to the asked size that fits the slot's width (`Ui::text_fit`). Never smaller than Standard. |
| Data in the mono face: fingerprints, hex, paths, addresses, xpubs, txids, descriptors | Larger; addresses and keys already break by character and wrap. Inside a chip or a node, fit-down. |
| Keyboards: keys, the candidate strip, the kana grid, the card pad, the dice pad | Unchanged (`fixed_text`). Their sizes are set by what must fit on the panel. |
| QR codes and SeedQR, the scanner's picture | Unchanged: they are pictures. Their captions are text and grow. |
| A row's icon (the leading icon of a list, menu or value row) | Grows with its label, by the same factor (§9 item 4); the row's height is §5's. |
| Other icons (`ICON`, `ICON_SMALL` elsewhere, glyphs, the wordmark) | Unchanged. |
| The structure chart's lines and the board's boxes | Lengths unchanged; their text as one-line slots or wrapping labels as above. |

No length grows with the text except a height that text sets (§5).

## 4. How each layout copes at Larger

The one-column decision counts the text factor and the Display scale. Let
`t` be the text factor (1.0, 1.2, 1.4) and `s` the Display scale
(0.75 to 2.0). The decision in `display` becomes:

- a panel is one-column when its width in dp at its density is under
  600 × `t` × `s` (today 600);
- a window is one-column when its width in dp, divided by `t`, is
  under the wide layout's class (today its width at its density times
  `s`, under 1200 dp: F101, which replaced the fit to 1280 × 800 and
  F78's floor);
- inside the Starter, the held app is one-column under 1280 × `t` × `s`
  dp (today 1280).

So Larger needs a window 1.4 times the size Standard needs to keep the
sidebar, the same step a narrow window takes. `f` itself is unchanged:
the wide layout at Larger draws at the same dp widths with larger text.
The factor's thresholds are names in `tokens.rs`
(`ONE_COLUMN_UNDER` 600, `HELD_ONE_COLUMN_UNDER` 1280; the two numbers
written in place in `display` today).

At 160 dpi (density 1), the smallest window that keeps the wide layout:

| | Standard | Large | Larger |
|---|---|---|---|
| Faraday Desktop | 960 × 600 | 1152 × 720 | 1344 × 840 |
| The Starter's held app | 1280 dp wide | 1536 dp wide | 1792 dp wide |
| The Starter's own pages, one column under | 600 dp | 720 dp | 840 dp |

So a 1280 × 800 desktop window at Larger takes the one-column layout;
a 1920 × 1080 window keeps the sidebar.

**The small panel** (the Pi's 480 × 640 at 286 dpi and the smallest,
240 × 320 at 143, both 268 × 358 dp) has no smaller layout to step down
to. There, at Larger:

- Home's menu: a label that no longer fits the 144 dp between icon and
  chevron wraps to two lines and its row grows; Home scrolls instead of
  showing every row at once.
- A step page: its text wraps and the page scrolls; the step's buttons
  stay at its foot.
- The bar across the top: its title fits down (§3).
- A screen with a keyboard (word entry, passphrase, the dice pad): the
  keyboard and the candidates are unchanged; the field above them fits
  down; the label above the field wraps, and when the label and field
  do not fit above the keyboard, the label's lines scroll inside the
  space left, as the candidates' strip does today.
- A sheet: its text wraps and the sheet scrolls; its buttons stay at
  its foot.

**Mobile** (the class a phone-sized display gets; only a saved QR's PNG
uses it, `docs/DESIGN.md` §3): no screen is laid out for it, so nothing
changes.

## 5. Heights that text sets

A height that holds text becomes the larger of its token and what its
text needs:

- `Ui::row_height(token, lines)`: `token`, or the lines' height
  (`Ui::line` per line) plus the row's padding, whichever is larger.
  List rows, menu rows (`menu_row`), value rows (`value_row`,
  `stacked_row`), sheet rows (`CHART_SHEET_ROW`, `MODE_ROW`,
  `BOARD_MENU_ROW`) take it; their touch target is the grown row.
- `wrap_buttons` and `wrap_buttons_height` take the button's height as
  a floor and grow it to the label's line plus padding.
- A line's advance written in place (`y += 24.0` after a 13 dp line) is
  replaced by `ui.line(size, w)` plus the gap the screen means. This is
  most of T3's work; each replacement lowers `tools/lint-tokens.counts`.
- `TOUCH` (48 dp) stays the floor: nothing becomes smaller than today.

Drawing reads: the row heights come from `measure` and `line`, which
are lookups in the glyph cache, so no new costly work enters drawing.
A screen that today works out a list's total height for its scroll
(`measured`, `measure_wrap`) does so with the grown heights.

## 6. Kept

- **Faraday.** `text=standard|large|larger`, a line of `settings_body`
  beside `theme=`, so it is in the kept set (across a lock) and in
  `faraday-settings.txt` on the boot stick (across a power-off, read
  before the passphrase as the theme is). `apply_setting` takes only
  those three values; anything else on the line is ignored and the
  setting stays as it was. The field lives on the `Settings` area
  struct (`docs/STRUCTURE-PASS.md` §7.4), not on `Faraday`, so the
  ratchet on Faraday's fields does not rise.
- **Faraday Desktop** keeps it as it keeps the theme: in memory, across
  a lock, not across quitting, as now (§9 item 6).
- **The Starter.** The Starter's own field, and its held app's: one
  choice, so Transfer and Learn inside the Starter follow the Starter's
  pages. Decided (§9 items 5 and 7): a **Text size** row on the Starter's About
  page (§3.7 of `docs/STARTER.md`), kept only once the Starter is
  installed, as `settings.txt` beside the installed Starter (Linux
  `~/.local/share/faraday-starter/settings.txt`, Windows
  `%LOCALAPPDATA%\Programs\Faraday Starter\settings.txt`) holding one
  `text=` line; Uninstall removes it; a Starter run without installing
  keeps it for that run only, so §5.1's "no settings or state file"
  holds for it.
- **Open Faraday Desktop** (`docs/STARTER.md` §5.3) starts the desktop
  app with the Starter's choice (`--text larger`), which the desktop
  app takes as its first setting.

## 7. Tests

Each states what a person would notice.

In `faraday/faraday-core/tests/all/` (a new `text_size.rs`, and
`settings_file.rs` for the kept line):

1. With Larger chosen, a label on Settings is drawn taller than at
   Standard, and Large's between the two (`drawn_text_boxes`: the
   label's ink height).
2. Larger is kept across a lock: chosen, the kept set saved, a new
   `Faraday` restored from it draws the label at Larger.
3. A stick's settings file with `text=larger` gives Larger before the
   passphrase; `text=huge` and `text=` leave Standard.
4. A 1280 × 800 window at 160 dpi shows the sidebar at Standard and not
   at Larger; a 1920 × 1080 window shows it at Larger.
5. At Larger, on 240 × 320 at 143 dpi and on 1280 × 800 at 160 dpi,
   Home, Settings, New key's first steps, word entry, a sheet and a
   Learn page draw no text cut at its right edge and no two lines of
   text over each other. The canvas notes a text whose ink crosses the
   right edge of its clip (`InkKind::Text` gains `cut`); a clip's top
   or bottom edge is a scroll and is not counted.
6. At Larger on 240 × 320, word entry's keys are all on the screen and
   a press on each types its letter.
7. The Display scale at 200 % on a 1280 × 800 window gives the
   one-column layout (the §1 finding).

In `faraday/faraday-starter/tests/all/`:

8. The Starter at Larger draws its Home label taller than at Standard,
   and the held Transfer page follows.
9. An installed Starter at Larger is at Larger when started again; one
   run without installing writes no file.

The snapshot shell takes `FARADAY_SNAP_TEXT=large|larger` as it takes
`FARADAY_SNAP_THEME`, and writes `cut.txt` beside the frames listing any
frame with text noted cut (test 5's check), for the review. The
Starter's `snap` example takes the same variable.

## 8. Batches

Each batch ends with `just` green; T3's areas may go to separate
agents.

| Batch | What | Size |
|---|---|---|
| T1 | `TextSize` and its tokens; `Ui` carries the factor (`Ui::new` takes it, `Ui::font` applies it), `fixed_text`, `text_fit`; Settings' row and `Action::TextSize`; the `text=` line; the one-column decision counts `t` and `s` (§4); tests 1–4, 7 | small |
| T2 | Components grow: `row_height`, menu, value and stacked rows and their icons, sheet rows, `wrap_buttons`; keyboards and the dice pad inside `fixed_text`; one-line slots on `text_fit`; the canvas's `cut` note; test 5's check as a helper | medium |
| T3 | The screens: each area's line advances written in place replaced by `ui.line`, its fixed heights by `row_height`, until the tours at Larger list nothing in `cut.txt`; by area, most self-contained first, as the structure pass orders them; tests 5–6 | large |
| T4 | The Starter: its pages on the same components, its row, its storage, `--text` to the desktop app; tests 8–9 | medium |
| T5 | `FARADAY_SNAP_TEXT`, `cut.txt`, the justfile; `faraday-snapshots` renders each size at Standard and Larger (`out/snapshots/faraday/WxH-larger/`); the docs of §10 | small |

T3 is most of the work: every screen that writes a line's advance in
place. It lowers the lint-tokens counts as it goes. A screen whose
heights are all from tokens and `ui.line` needs no change beyond T2.

Edit list:

- `core/faraday-draw/src/tokens.rs`: `TEXT_LARGE_PCT`,
  `TEXT_LARGER_PCT`, `ONE_COLUMN_UNDER`, `HELD_ONE_COLUMN_UNDER`.
- `core/faraday-draw/src/canvas.rs`: the `cut` note on recorded text
  ink.
- `faraday/faraday-ui/src/lib.rs`: `TextSize`; `Ui`'s factor, `font`,
  `fixed_text`, `text_fit`, `row_height`; `button` and `chip` heights;
  a row's icon size.
- `faraday/faraday-ui/src/parts.rs`, `faraday/faraday-ui/src/lists.rs`,
  `faraday/faraday-ui/src/sheet.rs`: rows and buttons grow.
- `faraday/faraday-ui/src/keyboard.rs`:
  inside `fixed_text`.
- `faraday/faraday-core/src/lib.rs`: `display` (§4), `Action::TextSize`,
  `Ui::new`'s factor.
- `faraday/faraday-core/src/stick/settings.rs`: the field on
  `Settings`, the `text=` line, `apply_setting`.
- `faraday/faraday-core/src/screens/settings.rs`: the row, wide and
  compact.
- `faraday/faraday-strings/src/en.rs`: the four strings.
- Each area's `screen.rs` and `faraday/faraday-core/src/screens/`: T3.
- `faraday/faraday-starter/src/lib.rs`, `faraday/faraday-starter/src/draw.rs`:
  the setting, the held app's factor, `display`.
- `faraday/shells/starter/src/main.rs`: the file when installed;
  `faraday/shells/starter/src/windows/install.rs` and the Linux
  uninstall: remove it.
- `faraday/shells/desktop/src/main.rs`: `--text`.
- `faraday/shells/snapshot/src/main.rs`, `faraday/faraday.just`: the
  variable, `cut.txt`, the Larger renders.
- Tests as §7.

## 9. Questions for the owner

Decided (owner, 2026-10-23), each as recommended:

1. **The factors.** Decided: ×1.2 and ×1.4 on every layout, the small
   panel included (a 268 dp panel at Larger sets body text at 22 dp;
   Home's menu scrolls, §4).
2. **Display scale.** Decided: kept beside Text size. Display scale
   makes everything larger and keeps the layout; Text size makes text
   larger and reflows. Both count in the one-column decision (§4).
3. **One-line slots.** Decided: a slot that cannot grow (bar title,
   pills, chips, board chips, chart nodes, the field over a keyboard)
   fits its text down, never below Standard (§3).
4. **Icons.** Decided: a row's icon grows with its label (§3); other
   icons keep their size.
5. **Where the Starter's Text size is chosen.** Decided: on its About
   page (§6).
6. **Faraday Desktop.** Decided: it keeps the theme and Text size in
   memory only, until it quits, as now (§6).
7. **The Starter's storage.** Decided: kept only once the Starter is
   installed, in one file that Uninstall removes; a Starter run without
   installing keeps it for that run (§6).
8. **The names.** Decided: Standard, Large, Larger (§2).

## 10. Docs that follow

- `docs/DESIGN.md` §3, after the paragraph on the size classes: "Text
  size (Settings › Appearance: Standard, Large, Larger, F85) multiplies
  every text size by 1, 1.2 or 1.4. Keyboards, QR codes
  and icons keep their sizes, except a row's icon, which grows
  with its label; a one-line slot that cannot grow
  fits its text down to Standard; a row or button grows to its text. The
  one-column layout's thresholds are multiplied by the text factor and
  the Display scale, so at Larger a window takes the one-column layout
  where it would be 1.4 times too small for Standard."
- `docs/DESIGN.md` §5, the `Settings` row: "Appearance (Theme, Mode,
  Contrast, Text size and the Reduce motion switch) and the display
  scale".
- `docs/FLOWS.md`, **Settings**: "keyboard layout (US first), display
  scale, text size (Standard, Large, Larger), idle lock …".
- `docs/STARTER.md` §3.7: "Text size: Standard, Large, Larger, as
  Faraday's (`docs/TEXT-SIZE.md`)." and §5.1: "there is no settings or
  state file; an installed Starter keeps one, its Text size
  (`docs/TEXT-SIZE.md` §6)."
- `docs/README.md`, the Spec list: "`docs/TEXT-SIZE.md`: Text size.
  Specified, not built (F85)." and a row in By question: "How does text
  size work, and what grows with it? | `docs/TEXT-SIZE.md`".
