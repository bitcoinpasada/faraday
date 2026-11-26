# Text and text fields

How the component layer (`faraday-ui`) draws text: single lines, wrapped
paragraphs, type roles, fitting with an ellipsis, typed lines with caret
and selection, outlined text fields with label and supporting text, the
address display, list items, dividers, icons and number formatting. Every
screen of every app reaches it; typing screens (Add a key, passphrase,
address and path fields, search bars) use its fields.

## Screens and sheets

None of its own. Components (all `faraday/faraday-ui/src/`):
- Plain text: `text`, `text_mid`, `text_right` (`faraday/faraday-ui/src/lib.rs:~1569-1628`); `wrap` (`faraday/faraday-ui/src/lib.rs:~1730-1756`); `wrap_marked` (a byte range drawn heavier: mono face drawn twice offset by `MARK_BOLD`, `faraday/faraday-ui/src/lib.rs:~1758-1801`); `wrap_lines` (count only, `faraday/faraday-ui/src/lib.rs:~1630-1658`); `fit` and `fit_middle` (trailing or middle ellipsis, `faraday/faraday-ui/src/lib.rs:~1660-1728`).
- Type-role text (`faraday/faraday-ui/src/roles.rs`): `text_as`, `text_mid_as`, `text_right_as`, `measure_as`, `fit_as`, `fit_middle_as`, `text_fit_as`, `text_fit_width` (`faraday/faraday-ui/src/roles.rs:~205-300`); wrapped paragraphs `wrap_as`, `wrap_seen`, `draw_wrapped`, `draw_wrapped_seen`, `wrapped_as`, `wrapped_with`, `wrap_ranges`, `wrap_basis`, `wrap_lines_as`, `wrap_height_as`, `laid` (`faraday/faraday-ui/src/roles.rs:~307-442`); "seen" variants draw only visible lines.
- Data types: `Wrapped` (text broken into lines, not drawn, `faraday/faraday-ui/src/roles.rs:~36`), `Lines` (owned or shared ranges, `faraday/faraday-ui/src/roles.rs:~54`), `WrapBasis` (font, scale, pixel width for reusing breaks, `faraday/faraday-ui/src/roles.rs:~73`), `Wrapped::first_line_end` (`faraday/faraday-ui/src/roles.rs:~1095`).
- Typed line: `typed` (one line of typing, masked dots applied by the caller, windowed to `room`, selection band, caret, char edges recorded as a `FieldAt`, `faraday/faraday-ui/src/lib.rs:~1039-1134`); multi-line `typed_lines`, `draw_typed`, `draw_typed_lines`, `caret_line` (`faraday/faraday-ui/src/roles.rs:~467-515`); `typed_address` (mono address in groups of four, grows with lines, `faraday/faraday-ui/src/roles.rs:~724`).
- Selection and caret: `selection` (band behind a whole string when all is selected in a focused field, `faraday/faraday-ui/src/lib.rs:~1001-1009`); `selected_band` / `selection_band` (`SELECTION_FILL`, `faraday/faraday-ui/src/lib.rs:~1011-1027`); `caret` (2-unit primary bar, drawn in the on-half of the blink, `faraday/faraday-ui/src/lib.rs:~1441-1451`); `caret_char` ("|" or "", `faraday/faraday-ui/src/lib.rs:~1464-1468`); `note_focus` (`faraday/faraday-ui/src/lib.rs:~1453-1462`).
- Fields (`faraday/faraday-ui/src/roles.rs`): `field_label` (Body small, OnSurfaceVariant, `faraday/faraday-ui/src/roles.rs:~818`); `field_box`, `field_box_sized` (outlined, `faraday/faraday-ui/src/roles.rs:~830`, `faraday/faraday-ui/src/roles.rs:~840`); `field_box_off` (`faraday/faraday-ui/src/roles.rs:~875`); `field_supporting` (`faraday/faraday-ui/src/roles.rs:~887`); consts `FIELD_LABEL` Body small and `FIELD_TEXT` Body large (`faraday/faraday-ui/src/roles.rs:~77`, `faraday/faraday-ui/src/roles.rs:~80`).
- Address display: `address_whole`, `address_height`, `address_lines` (`faraday/faraday-ui/src/roles.rs:~630-670`).
- List items: `ListItem` (headline, supporting text, trailing text with paint, action, `faraday/faraday-ui/src/roles.rs:~85`); `list_item`, `list_item_whole`, `list_item_whole_still`, `list_item_opens`, `list_item_still`, `list_item_as` (`faraday/faraday-ui/src/roles.rs:~984-1020`); measures `list_lines`, `list_head_room`, `head_line`, `list_head_lines`, `list_item_height` (`faraday/faraday-ui/src/roles.rs:~912-964`).
- Dividers `rule` and `vrule` (1 unit, at least 1 px, `faraday/faraday-ui/src/lib.rs:~1470-1485`); `icon` (`faraday/faraday-ui/src/lib.rs:~1803-1825`); `row_chevron` (`faraday/faraday-ui/src/lib.rs:~1827-1840`); `stick_glyph` (USB stick outline, `faraday/faraday-ui/src/lib.rs:~1842-1880`).
- `back` records the screen's back link and its hit index (`faraday/faraday-ui/src/lib.rs:~1136-1143`).

## Actions

- A press or drag on a typed line or field places the caret by the recorded char edges (`FieldAt`); the caller's edit handler moves it.
- A press on a list item gives its action unless it is a "still" item.

## Effects

None outside the app's state.

## State and lock

- No component keeps state. `Edit<A>` (field action, caret, anchor in chars) lives in the `Ui` and is set by the caller each frame; `typed` pushes `fields` and sets `focus_action`; `caret` writes `caret_drawn` and `caret_foot`; `note_focus` writes `focus_box` and `label_at`.
- Typed text passed in may be a secret (a passphrase, seed words); the caller applies masking before `typed`. The text is drawn through `Canvas::text`, whose glyph cache is cleared at Lock (`core/faraday-draw/src/canvas.rs:405-409`, see `draw-base`).

## Reach

- Every screen; fields in Add a key, passphrases, addresses, paths, search bars and sheets.
- Uses `draw-base` text measure and wrap, `tokens` type roles, `ui-theme` paints.
- `field_box` notes the focused field so `ui-regions` can scroll it above the on-screen keyboard (`keyboard`).

## Rules and refusals

- Text size: `font()` multiplies size and divides by `tr` (`faraday/faraday-ui/src/lib.rs:~1495-1507`); `fixed_text` sets the factor to 1 (`faraday/faraday-ui/src/lib.rs:~1525-1530`); a font floor `TEXT_MIN` 11 (`faraday/faraday-ui/src/lib.rs:~1489-1491`, `core/faraday-draw/src/tokens.rs:2132`).
- `stepped`: Body small becomes Body medium where the note-up setting says so, same face unless Medium (`faraday/faraday-ui/src/roles.rs:~101-115`).
- `row_height`: the token at Standard; at larger text, the larger of the token and lines plus twice the pad (`faraday/faraday-ui/src/roles.rs:~150-155`).
- `button_height`: `LABEL_L` line plus `BUTTON_PAD_V` (`faraday/faraday-ui/src/roles.rs:~160-162`); `field_h`: `FIELD_H` or line plus `FIELD_PAD_V` (`faraday/faraday-ui/src/roles.rs:~166-168`).
- `fit_factor`: a one-line text steps down by `TEXT_FIT_STEP_PCT` (10) and never below Standard (`faraday/faraday-ui/src/roles.rs:~181-199`); `text_fit_as` fits at the text size, else steps down, else cuts with an ellipsis (`faraday/faraday-ui/src/roles.rs:~205-228`).
- Field edge: error is Error at 2 dp with an error line; focused is Primary at 2 dp; else Outline at 1 dp (`FIELD_EDGE`) (`faraday/faraday-ui/src/roles.rs:~840-851`); a disabled field is a 1 dp outline at on-surface 12% (`DISABLED_CONTAINER`, `faraday/faraday-ui/src/roles.rs:~875-877`).
- A focused whole field is noted for the keyboard view (`faraday/faraday-ui/src/roles.rs:~852-855`).
- List lines: 1 with no supporting text; 2 if the supporting text fits; else 3 (`faraday/faraday-ui/src/roles.rs:~912-919`). Headline: one line at Standard (cut); at larger text two lines if it does not fit (`faraday/faraday-ui/src/roles.rs:~945-959`). Heights `LIST_1`, `LIST_2` or `LIST_3` (with `LIST_3_TOP`), or lines plus padding where larger text needs more (`faraday/faraday-ui/src/roles.rs:~964-978`).
- A chevron reserves `ICON_SMALL + GAP_SMALL` at the end; the press covers the full width (`faraday/faraday-ui/src/roles.rs:~1033`, `faraday/faraday-ui/src/roles.rs:~1047`).
- Address: breaks only between groups of four (`faraday/faraday-ui/src/roles.rs:~630-657`); first two and last two groups in the `full` paint, the middle in `quiet` (`faraday/faraday-ui/src/roles.rs:~684-692`).
- `short` abbreviates to `head … tail` for 17 or more chars; `grouped` makes groups of four with spaces; `btc` gives 8 decimals; `thousands` adds separators (`faraday/faraday-ui/src/lib.rs:~2621-2663`).
- The caret is drawn only in the on-half of its blink (`faraday/faraday-ui/src/lib.rs:~1441-1451`).

## Domain calls

- `faraday_draw::text::{width, measure, wrap, wrapped_height}` (`core/faraday-draw/src/text.rs`).
- `Canvas::text`, `Canvas::measure_text`, `Canvas::text_wrapped`, `Canvas::icon` (`core/faraday-draw/src/canvas.rs`).
- `faraday_draw::tokens::{TypeRole, BODY_*, LABEL_*, TITLE_*, FIELD_*, LIST_*, SELECTION_BAND, SELECTION_PAD, CARET_MIN, CARET_OVER, MARK_BOLD, TEXT_MIN, TEXT_FIT_STEP_PCT, percent}` (`core/faraday-draw/src/tokens.rs`).
- `impl From<TypeFace> for W`: Regular to R, Medium to S, Mono to M (`faraday/faraday-ui/src/roles.rs:~23`).

## Fixtures

None.

## Fixes

None.

## Doubts

- Callers were not traced past the component crates (faraday-ui and faraday-draw); the Reach section rests on the component code and a few searches, not on a read of every screen.
- No fix rows were given to this feature; fixes about carets in text fields (for example a word grid showing a caret in every empty cell) were filed under the features that own those fields.
- Whether `typed` masks a secret is the caller's: the component draws what it is given.
- `selection` discards the text it would draw on the band (`faraday/faraday-ui/src/lib.rs:~1001-1009`); the caller draws the text.
