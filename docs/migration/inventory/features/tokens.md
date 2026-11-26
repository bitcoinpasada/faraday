# Design tokens

The token file `core/faraday-draw/src/tokens.rs`: every named dimension,
type role, opacity, elevation, duration and easing curve the apps draw
with, plus lookup helpers by size or window class. It draws nothing and
holds no state. A new dimension is a name here (held by `lint-tokens`,
whose counts only fall). All values are in dp unless marked; printed-sheet
values are in points.

## Screens and sheets

None.

## Actions

None.

## Effects

None.

## State and lock

None: constants and pure lookups. No secrets.

## Reach

- Read by every `faraday-ui` component, by faraday-core screens and by the shells.
- `SESSION_COL_MIN` is read by the session pane (`faraday/faraday-core/src/screens/session_pane/mod.rs:24`, columns that fit = floor((inner + SPACER) / (SESSION_COL_MIN + SPACER)), at least 1, at `:318`).
- `MENU_ROW_DIVIDERS` is read in one place, `faraday/faraday-core/src/screens/menu_page.rs:400`.
- The retired type sizes `TITLE`, `BODY`, `LABEL`, `CAPTION` (`core/faraday-draw/src/tokens.rs:123-129`) still have a user in `faraday/faraday-core/src/picture.rs`.

## Rules and refusals

Values as the code sets them (lines approximate unless written plainly).

Helpers:
- `TypeRole` (size, line, face), `role()`, `.mono()`, `.medium()`, `.regular()` (`core/faraday-draw/src/tokens.rs:~44`, `core/faraday-draw/src/tokens.rs:~54`, `core/faraday-draw/src/tokens.rs:~82`).
- `touch(window)`: `TOUCH` 48 (`core/faraday-draw/src/tokens.rs:215`), or `TOUCH_COMPACT_SHORT` 44 on a Compact and short window (`core/faraday-draw/src/tokens.rs:~218-228`).
- `menu_row(class)` 52 on Small, else 72; `stacked_row` ROW on Small, else MENU_ROW; `value_row` (`core/faraday-draw/src/tokens.rs:~273-293`).
- `field(class)` 40 on Small, else 52 (`core/faraday-draw/src/tokens.rs:~306-311`).
- `key_height`, `key_bottom_inset` (adds the shell's bottom inset), `key_bottom_reserve` (`core/faraday-draw/src/tokens.rs:~328-357`).
- `candidate_rows`: 2 if one char, else 1 on Wide, 2 otherwise; `candidates_per_row`: 10 if one char, else 3 on Small, 4 otherwise; `candidates` = rows x per row (`core/faraday-draw/src/tokens.rs:~443-467`).
- `qr_side`: 198 Small, 260 Mobile, 240 Wide (`core/faraday-draw/src/tokens.rs:~486-492`).
- `codex32_box` 16/18/20, `codex32_cell` 20/24/28, `codex32_cell_char` 12/14/16 by class, `codex32_char`, `codex32_sheet_view` (`core/faraday-draw/src/tokens.rs:~1702-1863`).
- `key_label(h)` 15/14/13 by key height (`core/faraday-draw/src/tokens.rs:~1420`); `shape_full(h)` = h/2 (`core/faraday-draw/src/tokens.rs:~2060`); `alpha_of` 0..1 to 0..255 clamped and rounded (`core/faraday-draw/src/tokens.rs:~2095`); `percent(p)` u16 to f32 (`core/faraday-draw/src/tokens.rs:~2143`).
- `sheet_head(compact)`: TITLE_L if compact, else HEADLINE_S (`core/faraday-draw/src/tokens.rs:~116`); `type_scale_pct`: Small 100, Mobile 110, Wide 130 (`core/faraday-draw/src/tokens.rs:~149-155`); `narrow_text`: true only for Small (`core/faraday-draw/src/tokens.rs:~163-165`).

Type ramp (`core/faraday-draw/src/tokens.rs:~87-112`): HEADLINE_M 28/36, HEADLINE_S 24/32, TITLE_L 22/28, TITLE_M 16/24 medium, TITLE_S 14/20 medium, BODY_L 16/24, BODY_M 14/20, BODY_S 12/16, LABEL_L 14/20 medium, LABEL_M 12/16 medium, LABEL_S 11/16 medium. Medium face is Noto Sans SemiBold, Mono is Noto Sans Mono (`core/faraday-draw/src/tokens.rs:~30-37`). `TEXT_MIN` 11 (`core/faraday-draw/src/tokens.rs:2132`); `TEXT_LARGE_PCT` 120, `TEXT_LARGER_PCT` 140 (`core/faraday-draw/src/tokens.rs:~2136`, `core/faraday-draw/src/tokens.rs:~2140`); `TEXT_FIT_STEP_PCT` 10 (`core/faraday-draw/src/tokens.rs:~2150`).

Spacing and widths: `GRID` 8 (`core/faraday-draw/src/tokens.rs:172`), `GAP` = GRID, `GAP_SMALL` 4, `PAD` 16, `MARGIN_COMPACT` 16, `MARGIN_WIDE` 24; `SPACER` 24; `MENU_GAP` 16; `COLUMN_MAX` 720; `PAGE_INSET` 48; `READING_PAGE_W` 1000; `MENU_PAGE_W` 1112; `MENU_TILE_WIDE_MAX` 1248; `LIST_PANE` 360; `TRANSFER_W` 1100; `TOOLS_COLUMN_W` 900; `CONTENT_MAX` 1040; `FLOW_MAX` 900; `KEYGEN_COLUMN_MAX` 900; `SESSION_COL_MIN` 336; reading widths `READING_*_W` (632 to 1556); `PAGE_AREA_UNTIL_W` 1360; `KEYBOARD_CLEAR` 16.

Touch and chrome: `BUTTON_H` 40 (`core/faraday-draw/src/tokens.rs:1293`), `BUTTON_PAD` 16, `BUTTON_PAD_V` 8; `ICON` 28, `ICON_SMALL` 20, `ICON_GLYPH` 24, `ICON_GLYPH_SMALL` 20, `ICON_GLYPH_BUTTON` 18, `CHEVRON` 20, `ICON_BUTTON` 40; `NAV_BAR_H` 80, `APP_BAR_H` 64, `TOP_BAND` 56, `RAIL_W` 80, `NAV_ITEM_H` 56, `NAV_PILL_W` 56, `NAV_PILL_H` 32; `BADGE_SMALL` 6, `BADGE_LARGE` 16, `BADGE_PAD` 4, `BADGE_LABEL` 11.

Rows and lists: `ROW` 56; `MENU_ROW` 72, `MENU_ROW_SMALL` 52; `LIST_1` 56, `LIST_2` 72, `LIST_3` 88; `LIST_PAD` 16, `LIST_PAD_V` 8; `LIST_INDENT` 40; `LIST_RESERVE` 50; `MENU_ITEM_H` 48; `MENU_HEAD_H` 32; `MENU_ROW_ICON` 40; `MENU_CARD_COLUMNS` 3; `MENU_ROW_DIVIDERS` true (`core/faraday-draw/src/tokens.rs:1227`).

Fields: `FIELD_H` 56; `FIELD` 52; `TEXT_FIELD` 40; `TEXT_FIELD_TALL` 42; `TEXT_FIELD_SHEET` 44; `FIELD_EDGE` 1, `FIELD_EDGE_ACTIVE` 2; `FIELD_PAD` 16, `FIELD_PAD_V` 16; `FIELD_GAP` 4; `FIELD_RADIUS` = SHAPE_XS 4; `SEARCH_FIELD_MAX` 320; `SEARCH_BAR_H` 48; `HOME_SEARCH_MIN` 200; `CARET_OVER` 4, `CARET_MIN` 18, `TYPED_INSET` 12; `SELECTION_BAND` 0.6, `SELECTION_PAD` 2.

Keyboard: `KEY_GAP` 0; `KEY_FACE_INSET` 3, `KEY_FACE_INSET_Y` 2; `KEY_BOTTOM_INSET` 16; `OSK_KEY_ROW` = TOUCH + GAP = 56; `HEX_KEY_ROW` 56; `CODEX32_KEY_ROW` = TOUCH 48; `KANA_COLUMNS` 10; `KEY_UNITS_EDGE` 1.5, `KEY_UNITS_DONE` 3, `KEY_UNITS_SPACE` 6; `PAD_COLUMNS` 4; `KEY_AREA_SLACK` 0.01; `CANDIDATE_CELLS_HANZI` 10, `CANDIDATE_ROWS_HANZI` 2.

Scanner and codes: `SWEEP_QR` 160; `PREVIEW_MAX` 400 px; `VIEWFINDER_*` (bracket share 1/6, strokes 3, outline edge 1.5, insets GAP, band alpha 180); `SCAN_PANE_ASPECT` 0.75, `SCAN_PANE_MIN_H` 96, `SCAN_PANE_RADIUS` 8.

Elevation, state, focus, shapes:
- Levels: sheet 1, menu 2, FAB/toast/snackbar 3, dialog 3, raised FAB and drag 4; `TIP_RICH_LEVEL` 2. `ELEVATION_KEY_OPACITY` 77 (30%), `ELEVATION_AMBIENT_OPACITY` 38 (15%); `ElevationLayer`, `ELEVATION_KEY`, `ELEVATION_AMBIENT` per level 1-5 (y, blur, spread) (`core/faraday-draw/src/tokens.rs:~1923-1971`). `SHADOW_REACH` 24.
- `STATE_HOVER` 0.08, `STATE_FOCUS` 0.10, `STATE_PRESS` 0.10, `STATE_DRAG` 0.16, `PRESS_EMPHASIS` 0.22 (`core/faraday-draw/src/tokens.rs:~2068-2080`); `DISABLED_CONTENT` 0.38, `DISABLED_CONTAINER` 0.12; `HIGH_CONTRAST_TINT_MAX` 96; `FOCUS_RING` 3, `FOCUS_OFFSET` 2, `FOCUS_STROKE` 3; `SCRIM_ALPHA` 82.
- Shapes: none 0, XS 4, S 8, M 12, L 16, XL 28 (`SHAPE_XL` at `core/faraday-draw/src/tokens.rs:2057`); `SHAPE_CHECKBOX` 2; `RADIUS` = M 12; `RADIUS_SMALL` = S 8.
- Dialog: `DIALOG_PAD` 24, `DIALOG_MIN` 280, `DIALOG_MAX` 560, `DIALOG_ACTION_GAP` = GAP.
- Toast: `TOAST_H` 48, `TOAST_PAD` 16, `TOAST_PAD_Y` 14, `TOAST_MARGIN` 16, `TOAST_SHOW_MS` 4000 (`core/faraday-draw/src/tokens.rs:2176`), `TOAST_MIN_W` 344, `TOAST_MAX_W` 672, `TOAST_STACK_GAP` 8.
- Menu: `MENU_PAD_V` 8, `MENU_MIN_W` 112, `MENU_MAX_W` 280, `MENU_INDENT` 12.
- Tooltips: plain max W 200, min H 24, pad 8 x 4; rich max W 320, pad 16 x 12; `TIP_GAP` 4; show delay 500 ms; long press 500 ms.
- Progress: `PROGRESS_H` 4, `PROGRESS_SPAN` 0.3.
- Edge fades: `EDGE_FADE_COMPACT` 44, `EDGE_FADE` 24, `EDGE_FADE_LOW_COMPACT` 18, `APP_BAR_TINT_SHARE` 0.25, `EDGE_FADE_ALPHA` 230.0.

Controls: `TAB_H` 48, `TAB_INDICATOR_PRIMARY` 3, `TAB_INDICATOR_MIN` 24, `TAB_INDICATOR_SECONDARY` 2, `TAB_PAD` 16; switch `SWITCH_W` 52, `SWITCH_H` 32, `SWITCH_EDGE` 2, handle off 16 / on 24 / pressed 28, state 40; `SEGMENTED_H` 40, `SEGMENT_PAD` 12; stepper `STEPPER_COUNT_W` 44, `STEPPER_LABEL_COL` 180, label BODY_M, value TITLE_M; `SLIDER_HALO` 40; filter chips H 32, lead 8, trail 16, icon 18, gap 8; tags `TAG_H` 26, `TAG_PAD` 12, `TAG_DOT_R` 3, `TAG_DOT_TEXT_X` 22; `STATUS_CHIP_H` 32, pad 12; `PIP_W` 28, `PIP_H` 10, `PIP_GAP` 6; `STEP_BADGE` 26, `STEP_BADGE_TICK` 12, `STEP_SIDE` 30, `STEP_INDENT` 42, `STEP_TOUCH` 44; `CHECK_TICK` 11, `CHECK_LABEL_INDENT` 32, `MULTI_CHECK_X` 10; `FACT_INDENT` 14.

Motion: durations SHORT1 50, SHORT3 150, SHORT4 200, MEDIUM1 250, MEDIUM2 300, MEDIUM3 350, MEDIUM4 400, LONG4 600, EXTRA_LONG4 1000 ms; `Easing` = `[f32; 4]` cubic Bezier (`core/faraday-draw/src/tokens.rs:~2278`); STANDARD, STANDARD_DECELERATE, STANDARD_ACCELERATE, EMPHASIZED_DECELERATE, EMPHASIZED_ACCELERATE (`core/faraday-draw/src/tokens.rs:~2280-2289`); `EasingPath` and `EASE_EMPHASIZED` (`core/faraday-draw/src/tokens.rs:~2294`, `core/faraday-draw/src/tokens.rs:~2305-2309`). Other timings: `HOLD_MS` 1500 (`core/faraday-draw/src/tokens.rs:700`), `NOTE_FLASH_MS` 2000, `NO_PICTURE_MS` 9000, `POOR_PICTURE_MS` 4000, `HOST_FLASH_MS` 1600, `HOST_PULSE_MS` 1000, `HOST_PULSES` 3, `HOST_FOCUS_FADE_MS` 200, `ROLLER_TUMBLE_MS` 700, `ROLLER_ONE_TUMBLE_MS` 200, `ROLLER_PAUSE_MS` 350, `ROLLER_FACE_MS` 90, `PRESS_EMPHASIS_MS` 400.

Window classes: `WINDOW_MEDIUM` 600 (`core/faraday-draw/src/tokens.rs:900`), `WINDOW_EXPANDED` 840, `WINDOW_LARGE` 1200, `WINDOW_EXTRA_LARGE` 1600; `WINDOW_SHORT` 480 tall; `WINDOW_LOW` 600 tall; `WINDOW_DEFAULT_W` 1280, `WINDOW_DEFAULT_H` 800; `WINDOW_HELD_WIDE` 1280. `PREVIEW_SIZES`: nine (w, h) dp pairs (`core/faraday-draw/src/tokens.rs:~935`); the doc gives physical pixels at a dpi and the table dp, which agree (1024 x 600 at 170 dpi is 964 x 565 dp; 1280 x 800 at 150 dpi is 1365 x 853 dp).

Area tokens (named for the record; each area's feature uses them): structure chart `CHART_*` (node 112..240, pad and gap 12, band 12, lane 4, strokes 1 and 2, sheet 560 x 44 rows, panes 128 and 176, button 32, `CHART_DASHES` five patterns `core/faraday-draw/src/tokens.rs:~588-594`); `SIDE_SHEET_W` 400, `SIDE_SHEET_PAD` 24; backup board `BOARD_*` (`core/faraday-draw/src/tokens.rs:~604-688`); panels and flows (`SIDE_PANEL_TOP` 48, `CHOICES_*`, `FLOW_INSET` 40, `FLOW_CARD_MIN` 560, `FLOW_END_PAD` 24, `CHIP_LINE` 36, `SUPPORT_PANE` 400, `GUIDE_PANE_*`, `GUIDE_INDENT` 14, `GUIDE_BAR` 3); host strip of Faraday Demo (`HOST_STRIP_H` 64, `HOST_CONTROL_H` = TOUCH, `HOST_MENU_W` 280, `HOST_MENU_ROW` 48, `HOST_SHEET_W` 360, edges 2, `HOST_PULSE_LAYER` 0.24, `HOST_MARK_LAYER` = STATE_DRAG, `PREVIEW_SCALE_MIN` 0.05); codex32 worksheet, strings, dice tracks, tree, correction boxes and rollers (`CODEX32_*`, `CORRECT_*`, `ROLLER_*`, `C32_*`, `CODEX32_ENTER_SCROLL` 140); Home (`LEAD_MAX` 560, `TILE_H` 84, `TILE_H_GRID` 60, `HOME_*`); catalogue tiles (`CATALOG_TILE_MIN` 360, `CATALOG_TILE_H` 82, `CATALOG_TILE_H_PLAIN` 58); stick visit (`VISIT_*`, `IMPORT_TILE_*`); Starter cards (`CARD_MIN_W` 300); printed sheets in points (`KRUX_CELL_*`, `KEF_BY_HAND_MAX_SIDE` 37, `KEF_DEFAULT_SIDE` 33, `SHEET_*`, `PATHS_*`, `MOVE_TABLE_*`); Decode and Sign (`INVOICE_*`, `MESSAGE_FIELD_MIN_H` 120, `DECODE_*`); passphrase (`PASS_LABEL_W` 150, `PASS_BESIDE_GAP` 12, `PASS_CODE_INSET` 12, `PASS_CODE_TEXT` 14); Add a key (`WORD_KEYS_WIDE_FROM` 600, `WORD_COUNT_BESIDE_MIN` 340, `ENTRY_*`, `FINISH_*`); input panel (`CODEX32_ENTRY_W` 424, `PAD_KEY_W` = TOUCH, `INPUT_PANEL_MAX` 0.4, `DICE_PAD_KEY_H` 56, `DICE_STRIP_SLOT` 64, `INPUT_ROLLS_COL` 76); refused file row (`REFUSED_FILE_*`); copy sheet (`COPY_SHEET_*`, `COPY_HEAD_*`); setup (`SETUP_CONTINUE_PAD` 36, `END_TITLE_ICON` 24, `END_TITLE_INDENT` 32); `TOP_ROW_QUESTION` 52; tile faces `FACE_*`.

Which part uses which token (from the component files):
- Appearance: `FILTER_CHIP_*`, `SHAPE_S`, `THEME_CHIP_RING`, `THEME_CHIP_RING_GAP`, `THEME_CHIP_DOT`, `THEME_CHIP_TICK`, `TOUCH`, `APPEARANCE_ROW_GAP`, `APPEARANCE_LABEL_GAP`, `SEGMENTED_H`, `LABEL_L`, `BODY_M`, `BODY_L`, `LIST_PAD_V`, `SWITCH_*`, `STATE_*` (`faraday/faraday-ui/src/appearance.rs`).
- Controls: `BUTTON_PAD`, `DISABLED_*`, `FOCUS_OFFSET`, `FOCUS_RING`, `ICON_BUTTON`, `ICON_GAP`, `ICON_GLYPH`, `ICON_GLYPH_BUTTON`, `SEGMENT_PAD`, `PROGRESS_*`, `STEPPER_*`, `HIGH_CONTRAST_TINT_MAX`, `GAP`, `STATE_*` (`faraday/faraday-ui/src/controls.rs`).
- Hold: `BUTTON_H`, `BUTTON_PAD`, `BUTTON_PAD_V`, `DISABLED_*`, `LABEL_L` (`faraday/faraday-ui/src/hold.rs`).
- Guide: `LIST_PAD_V`, `BODY_L`, `BODY_M`, `TITLE_S`, `TITLE_M`, `PAD`, `GAP`, `GAP_SMALL`, `ICON_GLYPH_SMALL` (`faraday/faraday-ui/src/how.rs`).
- Keyboard panel: `OSK_KEY_ROW`, `RADIUS_SMALL`, plus private `KEY_ROW_COMPACT` 32 and `KEYS_MAX_W` 720 (`faraday/faraday-ui/src/keyboard.rs:48`, `faraday/faraday-ui/src/keyboard.rs:52`) and literals 8.0, 4.0, 6.0, 3.0 key inset, icon sizes 9, 18, 12.
- Lists, menu, parts, roles, press: as named in their files' imports (`faraday/faraday-ui/src/lists.rs:~8-9`, `faraday/faraday-ui/src/menu.rs:~14-16`, `faraday/faraday-ui/src/parts.rs:~7`, `faraday/faraday-ui/src/parts.rs:~215`, `faraday/faraday-ui/src/parts.rs:~222`, `faraday/faraday-ui/src/parts.rs:~262`, `faraday/faraday-ui/src/roles.rs:~15-20`, `faraday/faraday-ui/src/press.rs:~13`).
- lib.rs: `SHAPE_XS` (selection band), `SHAPE_S` (state layers, ring, pills), `SHAPE_M` (card), `SHAPE_L` (FAB), `SHAPE_CHECKBOX`, `TOUCH`, `TEXT_MIN`, `MARK_BOLD`, `SHADOW_REACH`, `WORDMARK_*`, `ELEVATION_RAISED`, `ELEVATION_FAB`, `EDGE_FADE*`, `APP_BAR_TINT_SHARE`, `SCRIM_ALPHA`, `DISABLED_*`; local constants `BAR_W` 6, `BAR_GRAB` 16, `SUFFIX_GAP` 364 and `WORDMARK_CAP` 687 font units (`faraday/faraday-ui/src/lib.rs`).
- Literals not tokens: QR card radius 8.0 and quiet zone 4 modules (`faraday/faraday-ui/src/parts.rs:~244`, `faraday/faraday-ui/src/parts.rs:~246`); overlay bar alpha 255 (`faraday/faraday-ui/src/region.rs:~488`); menu ring inset `FOCUS_RING / 2` (`faraday/faraday-ui/src/menu.rs:~217-220`).

## Domain calls

None; the file is a leaf of `core/faraday-draw`.

## Fixtures

None.

## Fixes

None given to this feature.

## Doubts

- Callers were not traced past the component crates (faraday-ui and faraday-draw); the Reach section rests on the component code and a few searches, not on a read of every screen.
- The doc of `SESSION_COL_MIN` says "no code reads it yet"; it is read (see Reach). The doc is stale.
- Several text field heights coexist (`FIELD_H` 56, `FIELD` 52, `TEXT_FIELD` 40, `TEXT_FIELD_TALL` 42, `TEXT_FIELD_SHEET` 44, `field(Small)` 40); `LIST_1` equals `ROW` (56) and `LIST_2` equals `MENU_ROW` (72).
- `EDGE_FADE_ALPHA` is an f32 while other alphas are u8.
- The doc of `FAB_H_COMPACT` contradicts itself.
- `KEY_BOTTOM_RESERVE` is named in a doc comment; only the function `key_bottom_reserve` exists.
- `MENU_ROW_DIVIDERS` is never false.
- Values of the area tokens were read once by the inventory; only the plainly written line numbers were checked.
