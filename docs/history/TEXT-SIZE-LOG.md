# TEXT-SIZE build log: the batches, the owner's answers and the docs that followed, as written at the time

**Kind:** History
**Status:** Kept as written; section numbers are the spec's when it was written. The current reference is `docs/TEXT-SIZE.md`.

## 8. Batches

Each batch ends with `just` green; T3's areas may go to separate
agents.

| Batch | What | Size |
|---|---|---|
| T1 | `TextSize` and its tokens; `Ui` carries the factor (`Ui::new` takes it, `Ui::font` applies it), `fixed_text`, `text_fit`; Settings' row and `Action::TextSize`; the `text=` line; the one-column decision counts `t` and `s` (§4); tests 1–4, 7 | small |
| T2 | Components grow: `row_height`, menu, value and stacked rows and their icons, sheet rows, `wrap_buttons`; keyboards and the dice pad inside `fixed_text`; one-line slots on `text_fit`; the canvas's `cut` note; test 5's check as a helper | medium |
| T3 | The screens: each area's line advances written in place replaced by `ui.line`, its fixed heights by `row_height`, until the tours at Larger list nothing in `cut.txt`; by area, most self-contained first, as the structure pass orders them; tests 5–6 | large |
| T4 | The Starter: its pages on the same components, its row, its storage, the size shared with the Demo it holds; tests 8–9 | medium |
| T5 | `FARADAY_SNAP_TEXT`, `cut.txt`, the justfile; `faraday-snapshots` renders each size at Standard and Larger (`out/snapshots/faraday/WxH-larger/`); the docs of §10 | small |

T3 is most of the work: every screen that writes a line's advance in
place. It lowers the lint-tokens counts as it goes. A screen whose
heights are all from tokens and `ui.line` needs no change beyond T2.

Edit list:

- `core/faraday-draw/src/tokens.rs`: `TEXT_LARGE_PCT`,
  `TEXT_LARGER_PCT`, `TEXT_FIT_STEP_PCT`, `LIST_PAD_V`, `BUTTON_PAD_V`,
  `FIELD_PAD_V` (the one-column thresholds are the window classes'
  tokens, §4).
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
6. **Faraday Demo.** Decided: it keeps the theme and Text size in
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

## T4 as built (2026-10-28)

The Starter's part, the last batch. The row is on About, as item 5
decided, though `docs/THEMES.md` §14 had expected it on the Starter's
Appearance page, which came after this plan; THEMES now says About. The
spec named no place on macOS: the file went to `~/Library/Application
Support/Faraday Starter/settings.txt`, outside the signed bundle.
Installed's record lists the file, and the uninstall sheet's "Received
files and settings stay." became "Received files and Faraday Demo's
sticks stay.", since Uninstall now removes the one settings file there
is. The Larger renders of the Starter found rows with a label that did
not wrap (a value row's label over its value, a mono row's label, a
radio row, the BitLocker key's tick box), buttons on a compact window
shortened at the larger size before they could fit down to Standard,
`Ui::choices` wrapping its buttons with the segmented button's padding
instead of a button's (so the chosen one's label was shortened), and
the Home wordmark under the Install button on a narrow window.
