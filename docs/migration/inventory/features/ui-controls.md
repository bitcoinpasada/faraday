# Buttons and choice controls

The Material 3 controls of the component layer (`faraday-ui`): buttons in
every style, the FAB, chips, pills, tags, badges, checkboxes, switches,
segmented buttons and their wrapped fallback, icon buttons, the stepper, the
progress bar, hold-to-confirm, multi-choice list rows, the code box and the
Settings appearance rows. Every screen of every app draws them; each is a
function on `Ui` that draws and registers its press in one call.

## Screens and sheets

None of its own. Components (all `faraday/faraday-ui/src/`):
- Buttons: `button` (label, full round ends, `LABEL_L` medium, height grown with text size, `faraday/faraday-ui/src/lib.rs:~2051-2080`); `button_wrapped` (multi-line label, `faraday/faraday-ui/src/lib.rs:~2082-2100`); `button_icon` (leading icon, `faraday/faraday-ui/src/lib.rs:~2102-2127`); `button_icon_after` (trailing icon, menu opener, `faraday/faraday-ui/src/lib.rs:~2129-2154`); `text_button` (label starts at `x`, container padding reaches past it, height `BUTTON_H`, `faraday/faraday-ui/src/lib.rs:~2167-2177`); `button_in` and `key_in` (keyboard keys, square corners `r`, `faraday/faraday-ui/src/lib.rs:~2179-2221`); the core `button_drawn` (`faraday/faraday-ui/src/lib.rs:~2240-2352`).
- `Style` (`faraday/faraday-ui/src/controls.rs:~120-198`): Primary (filled), Tonal, Secondary (outlined), Ghost (text), Danger (outlined error), Selected (tonal with tick), and the off kinds Disabled, PrimaryOff, TonalOff, GhostOff.
- `field_action` (outlined primary-text chip inside an entry field, `faraday/faraday-ui/src/lib.rs:~2354-2382`).
- `fab` and `fab_pill` (pill FAB at bottom right, primary fill, on-primary icon and word, shadow raised when hovered, records `fab_at`, `faraday/faraday-ui/src/lib.rs:~2384-2439`).
- `tag` with `Status` Ok, Warning, Error, Neutral mapped to container and on-container roles (`faraday/faraday-ui/src/lib.rs:~2441-2446`, `faraday/faraday-ui/src/lib.rs:~2597-2618`).
- `chip` / `chip_press` (dot plus text, `faraday/faraday-ui/src/lib.rs:~2448-2481`); `word_pill` (mono word in outlined pill, press opens a list, `faraday/faraday-ui/src/lib.rs:~2490-2504`); `pill` / `pill_in` (outlined assist chip, `faraday/faraday-ui/src/lib.rs:~2506-2520`); `pips` (`n` pills, `filled` in ok colour, `faraday/faraday-ui/src/lib.rs:~2522-2535`); `badge` (step circle: done, open, idle, tick or number, `faraday/faraday-ui/src/lib.rs:~2537-2556`); `checkbox` (`faraday/faraday-ui/src/lib.rs:~2558-2595`).
- `segmented` / `segmented_with` (options side by side in an outlined container, chosen filled `SecondaryContainer` with tick, `slide` moves the fill, dividers in Outline, `faraday/faraday-ui/src/controls.rs:~375-470`); `choices` / `choices_with` (segmented when it fits, else wrapped buttons, `faraday/faraday-ui/src/controls.rs:~476-532`); `choices_h` (`faraday/faraday-ui/src/controls.rs:536-552`).
- `outlined_icon_button` (`faraday/faraday-ui/src/controls.rs:~560-585`) and `tonal_icon_button` (`faraday/faraday-ui/src/controls.rs:591-606`), both `ICON_BUTTON` circles with a plain tooltip.
- `stepper` (label, minus, count, plus; `faraday/faraday-ui/src/controls.rs:~615-643`).
- `progress` (linear indicator, determinate or moving by `phase`, `faraday/faraday-ui/src/controls.rs:~650-679`).
- `hold_button` (outlined; fills from the left in the tone's container by `held` 0..1; label split between held and rest; `HoldTone` Error, Warning, Primary; `faraday/faraday-ui/src/hold.rs:~17-45`, `faraday/faraday-ui/src/hold.rs:~81-146`).
- `multi_list`, `multi_list_valued` (one row per option with checkbox, optional muted value line, `faraday/faraday-ui/src/lists.rs:~18`, `faraday/faraday-ui/src/lists.rs:~33`, `faraday/faraday-ui/src/lists.rs:~46`); `code_box`, `code_height`, `code_line` (`faraday/faraday-ui/src/lists.rs:~108`, `faraday/faraday-ui/src/lists.rs:~117`, `faraday/faraday-ui/src/lists.rs:~124`).
- Appearance: `theme_chip` and `theme_chips` (ten themes as filter chips, filled in that theme's PrimaryContainer, dot in Primary, tick and ring when chosen, "Default" mark on the default theme, `faraday/faraday-ui/src/appearance.rs:~23-120`); `appearance` (rows Theme, Mode, Contrast, `faraday/faraday-ui/src/appearance.rs:~126-183`); `text_size_row` (segmented over `TextSize::OFFERED`, `faraday/faraday-ui/src/appearance.rs:~190-213`); `switch` and `switch_row` (`faraday/faraday-ui/src/appearance.rs:~220-296`); `held_ink` (white or black content by luminance, `faraday/faraday-ui/src/appearance.rs:~304-311`).

## Actions

- A press on a button, chip, segment, icon button, checkbox, switch, list row or stepper button gives the caller's action.
- A hold button reports no action of its own; the caller supplies `held` (0..1) and decides when a hold completes (`HOLD_MS` 1500 is the token, `core/faraday-draw/src/tokens.rs:700`).
- A press on a `word_pill` opens a list (the caller's action).

## Effects

None outside the app's state.

## State and lock

- No control keeps state between frames: ticks, chosen index, `slide`, `held`, `phase` and the stepper count are the caller's.
- `Ui` fields written: `primaries` counter (`button_drawn`), `fab_at` (`fab`).
- No secrets.

## Reach

- Every screen of Faraday Offline, Faraday Demo and the Starter.
- Uses presses (`ui-press`), text measure and fit (`ui-text`), tooltips (`ui-sheets`), theme and palette (`ui-theme`), tokens (`tokens`).
- Appearance rows are drawn in Settings; theme chips map each chip to a `Theme` in the current mode and contrast (`faraday/faraday-ui/src/appearance.rs:~102-120`).
- `button_rows`, `wrap_buttons` and sheet button rows that use these buttons are in `ui-sheets`.

## Rules and refusals

- An off style (Disabled, PrimaryOff, TonalOff, GhostOff) registers no press: `button_drawn` presses only when `!style.is_off()` (`faraday/faraday-ui/src/lib.rs:2300`; `Style::is_off` at `faraday/faraday-ui/src/controls.rs:177-182`). So a disabled option in `choices` takes no press.
- A dimmed action draws its style through `style.off()` (`faraday/faraday-ui/src/lib.rs:~2247`).
- Button height grows with text size (`button_height`, `faraday/faraday-ui/src/lib.rs:~2057`, `faraday/faraday-ui/src/lib.rs:~2108`, `faraday/faraday-ui/src/lib.rs:~2136`); a label is fitted down toward Standard only when text size is over 1 (`faraday/faraday-ui/src/lib.rs:~2309-2321`).
- Key labels and key boxes keep their size under `fixed_text` (`faraday/faraday-ui/src/lib.rs:~2183-2185`, `faraday/faraday-ui/src/lib.rs:~2203-2205`); `key_in` draws hover and focus only, no pressed layer (`Unpressed::Keyboard`).
- On the compact small panel the FAB and a field action take the touch-floor height as target (`faraday/faraday-ui/src/lib.rs:~2368-2371`, `faraday/faraday-ui/src/lib.rs:~2394-2396`).
- High contrast: tonal and selected buttons get an outline (`faraday/faraday-ui/src/lib.rs:~2284-2286`); role colours with alpha below full become full-strength edges (`tint_edge`, `solid`, `faraday/faraday-ui/src/controls.rs:~206-221`).
- Choices: when every segment fits `w` they form a segmented button, else each option is a `Ui::button` wrapped onto rows `h + GAP` apart, chosen = `Style::Selected`, off = `Style::Disabled` (`faraday/faraday-ui/src/controls.rs:~500-531`). `choices_h` makes the same segmented-or-wrapped decision from the same widths with nothing drawn (`faraday/faraday-ui/src/controls.rs:536-552`).
- Segment width is constant: widest label plus `2*SEGMENT_PAD`, tick glyph and `ICON_GAP`, so no segment changes width as the choice moves (`faraday/faraday-ui/src/controls.rs:~355-368`).
- A disabled segment takes no press (`faraday/faraday-ui/src/controls.rs:~426-428`); a disabled outlined icon button draws a faint edge and icon and takes no press (`faraday/faraday-ui/src/controls.rs:~571`, `faraday/faraday-ui/src/controls.rs:~577-584`).
- `tonal_icon_button` has no disabled branch (no `is_dimmed` check, `faraday/faraday-ui/src/controls.rs:591-606`).
- Stepper: compact puts the buttons at the right end (`faraday/faraday-ui/src/controls.rs:~625-626`); wide puts them `STEPPER_LABEL_COL` in from the label start, or past a longer label (`faraday/faraday-ui/src/controls.rs:~628`).
- Progress: reduce motion (phase `None`) draws the whole track Primary at half strength (`faraday/faraday-ui/src/controls.rs:~648-649`, `faraday/faraday-ui/src/controls.rs:~675-677`).
- Hold label: one line if it fits `room`, else wrapped lines and the hold grows (`faraday/faraday-ui/src/hold.rs:~53-64`); the held fill starts at the left edge (`faraday/faraday-ui/src/hold.rs:~114-121`); a dimmed hold is faint, returns its width and takes no press (`faraday/faraday-ui/src/hold.rs:~101-112`). Hold-to-confirm is the F136 hold.
- Multi-choice rows: row height at least the token and taller for larger text (`faraday/faraday-ui/src/lists.rs:~55-61`); a rule between rows, not after the last (`faraday/faraday-ui/src/lists.rs:~89-96`); a disabled row takes no press, its checkbox is drawn dimmed (`faraday/faraday-ui/src/lists.rs:~97-99`).
- Code box: height mono line plus `2*GAP_SMALL` (`faraday/faraday-ui/src/lists.rs:~118`); `code_line` puts the box beside its lead if it fits, else on the next line (`faraday/faraday-ui/src/lists.rs:~136-143`).
- Theme chips wrap in width `w`, a new row when a chip would pass the right edge, rows `TOUCH` apart (`faraday/faraday-ui/src/appearance.rs:~111-114`).
- Appearance rows: label in a column of `label_w` beside the control, or above it (`faraday/faraday-ui/src/appearance.rs:~149-163`, `faraday/faraday-ui/src/appearance.rs:~199-209`).
- Switch: on is a Primary track with OnPrimary handle at right; off is a SurfaceContainerHighest track with Outline edge and Outline handle at left; the handle grows when pressed (`faraday/faraday-ui/src/appearance.rs:~220-278`).
- Chosen chip keeps its check and shows whole; card ranks wrap into equal rows wide enough for a chosen rank, so choosing moves none (history, 92de21c46).

## Domain calls

- `faraday_draw::tokens` sizes and roles: `BUTTON_H`, `BUTTON_PAD`, `BUTTON_PAD_V`, `ICON_BUTTON`, `ICON_GLYPH`, `ICON_GLYPH_BUTTON`, `ICON_GAP`, `SEGMENT_PAD`, `SEGMENTED_H`, `STEPPER_*`, `PROGRESS_H`, `PROGRESS_SPAN`, `FILTER_CHIP_*`, `THEME_CHIP_*`, `SWITCH_*`, `FAB_*`, `TAG_*`, `PIP_*`, `STEP_BADGE*`, `SHAPE_*`, `DISABLED_CONTENT`, `DISABLED_CONTAINER`, `HIGH_CONTRAST_TINT_MAX`, `ELEVATION_FAB`, `ELEVATION_RAISED`, `LABEL_L` (`core/faraday-draw/src/tokens.rs`).
- `faraday_draw::widgets::icon::Icon` (`core/faraday-draw/src/widgets/icon.rs`).
- `Canvas` fill, stroke, shadow and text (`core/faraday-draw/src/canvas.rs`).

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| b2b22ee33 | On the small panel text and controls sharing a line ran into each other (vault choice, Search again, Open this wallet, scan sheet camera buttons, randomness). | A layout step stacks the other choice under the vault when the line is too narrow; buttons that do not fit stack; the scan sheet leaves room for camera buttons. |
| 92de21c46 | On the small panel a chosen chip lost its check or was cut; choosing a rank or seed-word count moved its neighbours. | A chosen chip keeps its check and shows whole; card ranks wrap into rows wide enough for a chosen rank, all alike, so choosing moves none. |
| e2486a62a | On the small panel a hold label that did not fit was cut ("Hold to save without encryption"); the Delete files warning, option lines and file names did not wrap. | A hold label wraps onto as many lines as it needs and the container grows; a label that fits is drawn as before. |
| 9de67eb58 | On the 7-inch panel a Learn link (Delete files sheet, backup's last card, Extra copies) was cut short and its target could not reach the touch floor. | A full-size touch target with room beside the control below. |
| adfea1356 | A text button alone under body text, a field, or first in a row had its label and touch target misplaced (Spend's Not sure what you have?, More about this, Start over, unlock keys). | The label sits on the content's start edge; the state layer and touch target reach past it. |
| 3900d1045 | On 1024 x 600 notes were too small, and two rows of buttons or chips shared their touch overlap. | Notes one step larger; rows a touch target apart. |

## Doubts

- Callers were not traced past the component crates (faraday-ui and faraday-draw); the Reach section rests on the component code and a few searches, not on a read of every screen.
- Of the fix rows, only 7ac5cd6a0 and 10a89d54d were checked against git by the inventory; the others are as given.

- Negative wrap width in `hold_layout` when `room < 2*BUTTON_PAD` was raised and not settled (`faraday/faraday-ui/src/hold.rs:~53-64`).
- `button_icon_width`, `button_width`, `chip_width` are measures only (`faraday/faraday-ui/src/lib.rs:~2156-2165`, `faraday/faraday-ui/src/lib.rs:~2223-2238`, `faraday/faraday-ui/src/lib.rs:~2483-2488`).
- `pill` is described as a Material assist chip in its own comment; `chip`, `tag`, `pips`, `badge`, `word_pill`, the stepper and the hold are Faraday's own, not Material kinds.
