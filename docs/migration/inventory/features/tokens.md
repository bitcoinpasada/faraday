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

Callers at main `d89dde5de`: each token is read by the `faraday-core`, Starter and shell modules that import it from `faraday_draw::tokens` (or write it qualified); `git grep -n -w <token>` at that commit gives the lines. Of 729 tokens: 428 are read by exactly one app module (listed under it), 146 by several (listed with the number of modules), 112 only inside `faraday-ui` and 43 by no app module.

Read by one app module:
- `faraday/faraday-core/src/backup_flow`: `STICK_FIGURE_W`
- `faraday/faraday-core/src/board`: `BOARD_BOX_MIN`, `BOARD_CARD_MAX`, `BOARD_PLACE_GAP`, `BOARD_PLACE_W`, `BOARD_PLACE_STRIPE`, `BOARD_BOX_EDGE`, `BOARD_ADD_STICK`, `BOARD_TAG`, `BOARD_DASH`, `BOARD_HEAD`, `BOARD_PAD`, `BOARD_ZONE_LABEL`, `BOARD_CHIP`, `BOARD_CHECK_LABEL`, `BOARD_MARK_STROKE`, `BOARD_MENU_RULE`, `BOARD_STICK_EDGE`, `BOARD_VAULT_HEAD`, `BOARD_VAULT_PAD`, `BOARD_VAULT_STROKE`, `FOCUS_STROKE`, `ELEVATION_RAISED`
- `faraday/faraday-core/src/boot_import`: `IMPORT_TILE_ICON_BOX`, `IMPORT_TILE_ICON`, `IMPORT_TILE_TITLE_X`, `IMPORT_TILE_DOTS`, `IMPORT_TILE_MIN_ROWS`, `IMPORT_TILE_FOOT_GRIDS`
- `faraday/faraday-core/src/codex32`: `CODEX32_KEY_ROW`, `CODEX32_ENTRY_W`, `PAD_KEY_W`, `CODEX32_COLUMN_FIXED_H`, `CODEX32_TRACK_UNIT_MIN`, `CODEX32_VIEW_MIN_ROWS`, `INPUT_PAD_LABEL_W`, `PAD_KEYS_BESIDE_MIN`, `CODEX32_BOX_GAP`, `CODEX32_GROUP_GAP`, `CODEX32_ROW_GAP`, `CODEX32_NUMBER_ROW`, `CODEX32_RADIUS`, `CODEX32_GIVEN_STROKE`, `CODEX32_FRONT_STROKE`, `CODEX32_LABEL_W`, `CODEX32_PANEL_GAP`, `CODEX32_MAIN_MIN`, `CODEX32_CHOICE_H`, `CODEX32_TRACK_UNIT`, `CODEX32_TRACK_TOP`, `CODEX32_TRACK_GAP`, `CODEX32_FREE_X`, `CODEX32_FREE_Y`, `CODEX32_FREE_R`, `CODEX32_HEX_SIDE`, `CODEX32_HEX_W`, `CODEX32_HEX_X`, `CODEX32_HEX_Y`, `CODEX32_HEX_PITCH`, `CODEX32_HEX_RAISE`, `CODEX32_PADS_HEAD_X`, `CODEX32_PADS_LARGE_HEAD_X`, `CODEX32_MARKER`, `CODEX32_TRACK_DIE`, `CODEX32_MARKER_STROKE`, `CODEX32_TRACK_LABEL_W`, `CODEX32_TRACK_SPACE`, `CODEX32_TREE_MIN_W`, `CODEX32_TREE_MAX_W`, `CODEX32_TREE_LEVEL`, `CODEX32_TREE_NODE`, `CODEX32_TREE_LINE`, `CODEX32_PATH_STROKE`, `CODEX32_BAND_SIDE`, `CODEX32_TAB`, `CODEX32_CELL_FIT_MIN`, `CODEX32_CELL_FIT_MAX`, `CODEX32_CELL_FIT_STEP`, `CODEX32_CELL_CHAR_PART`, `CODEX32_STRING_BOX_MIN`, `CODEX32_STRING_BOX_MAX`, `CODEX32_STRING_ROWS`, `CODEX32_STRING_CHAR_PART`, `CODEX32_STRING_NUMBER_PART`, `CODEX32_SHEET_GAP`, `CODEX32_SHEET_BOLD`, `CODEX32_CELL_NOTE`, `CODEX32_CELL_NOTE_RISE`, `CODEX32_CELL_NOTE_INSET`, `CODEX32_MARK_GAP`, `CODEX32_STRING_NUMBER`, `CODEX32_SWATCH`, `CODEX32_CHIP_H`, `SHEET_FOLLOW_ENTRY_MAX`, `SHAPE_NONE`, `TEXT_MIN`, `codex32_box`, `codex32_char`, `codex32_cell`, `codex32_cell_char`, `codex32_sheet_view`
- `faraday/faraday-core/src/codex32_backup`: `C32_CHIP`, `C32_CHECK`
- `faraday/faraday-core/src/codex32_correct`: `CORRECT_BOX`, `CORRECT_BOX_GAP`, `CORRECT_GROUP`, `CORRECT_GROUP_GAP`, `CORRECT_BOX_RADIUS`, `CORRECT_NUMBER_ROW`, `CORRECT_ROW_GAP`, `CORRECT_PAIR_ROW`, `CORRECT_ICON`, `CORRECT_ICON_SIZE`, `CORRECT_TICK_ROW`, `CORRECT_TICK`, `CORRECT_TICK_LABEL`
- `faraday/faraday-core/src/compact`: `KEYBOARD_CLEAR`, `TILE_H_GRID`, `GRID_TILE_INSET`, `GRID_TILE_TEXT_X`, `TILE_PAD`, `PART_INDENT_COMPACT`
- `faraday/faraday-core/src/decode`: `DECODE_INDEX_W`, `DECODE_ROW_H`, `DECODE_VALUE_ROOM`, `DECODE_KIND_GAP`
- `faraday/faraday-core/src/drawing`: `WINDOW_DEFAULT_W`, `WINDOW_DEFAULT_H`, `WINDOW_HELD_WIDE`, `TOP_BAND`
- `faraday/faraday-core/src/explore`: `ENTRY_CELL_HEIGHT`, `ENTRY_ROW`, `ENTRY_CELL_GAP`, `ENTRY_CELL_NUMBER`, `EXPLORE_NOTE_GAP`, `PATH_FIELD_W`
- `faraday/faraday-core/src/flow`: `CHOICES_ROW`, `GUIDE_INDENT`, `GUIDE_BAR`, `FLOW_END_PAD`
- `faraday/faraday-core/src/fresh`: `INPUT_ROLLS_COL`, `INPUT_ROLLS_LINE`, `INPUT_ROLLS_STRUCK`, `ROLLER_DIE`, `ROLLER_DIE_RADIUS`, `ROLLER_PIP`, `ROLLER_PIP_OFFSET`, `ROLLER_BOUNCE`, `ROLLER_SWAY`, `ROLLER_TUMBLE_MS`, `ROLLER_ONE_TUMBLE_MS`, `ROLLER_PAUSE_MS`, `ROLLER_FACE_MS`, `ROLLER_COUNT_MAX`, `ROLLER_COUNT_DIGITS`, `ROLLER_FIVE_BATCH`
- `faraday/faraday-core/src/glance`: `CHART_NODE_PAD`, `CHART_NODE_MIN`, `CHART_NODE_MAX`, `CHART_NODE_GAP`, `CHART_BAND`, `CHART_LANE`, `CHART_STROKE`, `CHART_STROKE_BOLD`, `CHART_SHEET_ROW`, `CHART_BUTTON`, `CHART_DASHES`, `MOTION_SHORT1`, `EASE_EMPHASIZED_ACCELERATE`, `menu_row`
- `faraday/faraday-core/src/glide`: `EASE_EMPHASIZED`
- `faraday/faraday-core/src/invoice`: `INVOICE_FIELD_H`, `INVOICE_LABEL_W`, `INVOICE_PAGE_W`, `INVOICE_TOP`, `INVOICE_FIELD_PAD_X`, `INVOICE_FIELD_PAD_Y`, `INVOICE_GAP`, `INVOICE_CLEAR_ROOM`, `INVOICE_CLEAR_TOP`, `INVOICE_CLEAR_W`, `INVOICE_BLOCK_GAP`, `INVOICE_NAME_H`, `INVOICE_RULE_DROP`, `INVOICE_ROW_STEP`, `INVOICE_WIDE_ROW_H`, `INVOICE_WIDE_RULE_GAP`, `INVOICE_WIDE_STEP`, `INVOICE_NAME_DROP`
- `faraday/faraday-core/src/kef_form`: `KEF_HEAD_ADVANCE`, `KEF_CAPTION_ADVANCE`
- `faraday/faraday-core/src/keygen`: `TAG_PAD`, `DICE_STRIP_SLOT`, `DICE_STRIP_BIT_W`, `DICE_STRIP_BIT_H`, `DICE_STRIP_DROP`, `CODEX32_ENTER_SCROLL`
- `faraday/faraday-core/src/menu`: `BOARD_MENU_DOT`
- `faraday/faraday-core/src/modes`: `MODE_SHEET_WIDTH`
- `faraday/faraday-core/src/motion`: `MOTION_SHORT3`, `MOTION_SHORT4`, `MOTION_MEDIUM3`, `EASE_STANDARD`, `EASE_EMPHASIZED_DECELERATE`
- `faraday/faraday-core/src/nav`: `LOADED_LINE`
- `faraday/faraday-core/src/out`: `TOAST_SHOW_MS`, `EASE_STANDARD_DECELERATE`, `EASE_STANDARD_ACCELERATE`
- `faraday/faraday-core/src/picture`: `BODY`
- `faraday/faraday-core/src/scan_key`: `NOTE_FLASH_MS`, `POOR_PICTURE_MS`
- `faraday/faraday-core/src/screens/backup`: `END_TITLE_ICON`, `END_TITLE_INDENT`, `PART_NUMBER_INDENT`, `PART_MIN_H`, `TEXT_FIELD_TALL`, `ELEVATION_MENU`, `MENU_ITEM_H`, `MENU_ITEM_PAD`, `MENU_PAD_V`, `COPY_WRITE_GAP`
- `faraday/faraday-core/src/screens/catalog`: `CATALOG_TILE_H_PLAIN`, `FACE_INSET`, `FACE_TOP`, `FACE_LINE_Y`, `FACE_TAGS_Y`, `FACE_LINE_GAP`, `FACE_ABOVE_ICON`, `FACE_ABOVE_GLYPH`, `FACE_ABOVE_GAP`, `HOME_TILE_ICON`, `HOME_TILE_INSET`, `HOME_TILE_TEXT_X`, `HOME_RESULTS_LINE`
- `faraday/faraday-core/src/screens/create`: `CREATE_FIELD_LABEL`, `REFUSED_FILE_REASON_Y`, `REFUSED_FILE_NAME_Y`, `REFUSED_FILE_GAP`
- `faraday/faraday-core/src/screens/entry`: `KEY_FACE_INSET_Y`, `KEY_BOTTOM_INSET`, `WORD_KEYS_WIDE_FROM`, `WORD_COUNT_BESIDE_MIN`, `WIDE_FINISH_GAP`, `PASS_LABEL_W`, `PASS_BESIDE_GAP`, `WIDE_STICK_W`, `WIDE_STICK_GAP`, `FINISH_CLEAR_X`, `FINISH_BUTTON_GAP`, `FINISH_HINT_GAP`, `KEY_AREA_SLACK`, `PART_INDENT`
- `faraday/faraday-core/src/screens/files`: `SUMMARY_PAD`, `SUMMARY_HEAD_X`, `SUMMARY_HEAD_Y`, `SUMMARY_HEAD_H`, `VISIT_SUMMARY_FOLD_H`, `STATUS_CHIP_PAD`
- `faraday/faraday-core/src/screens/guide_pane`: `GUIDE_PANE_PAD`, `GUIDE_PANE_CLOSE_TOP`, `GUIDE_PANE_TABS_TOP`, `GUIDE_PANE_PARA`
- `faraday/faraday-core/src/screens/home`: `HOME_LEAD_H`, `HOME_LEAD_INSET`, `HOME_LEAD_ICON`, `HOME_LEAD_TEXT_X`, `HOME_LEAD_SUB_Y`, `HOME_RESULTS_ROWS`
- `faraday/faraday-core/src/screens/hub_layout`: `FLOW_CARD_MIN`, `CHOICES_PANE_MIN`, `CHOICES_PANE_MAX`, `CHOICES_PANE_SHARE`, `MENU_TILE_WIDE_MAX`, `SUPPORT_PANE`, `ELEVATION_FAB`
- `faraday/faraday-core/src/screens/menu_page`: `CATALOG_TILE_MIN`, `MENU_ROW_DIVIDERS`, `MENU_CARD_COLUMNS`, `MENU_ROW_ICON`, `MENU_TAG_H`, `MENU_TAG_PAD`, `MENU_TAG_GAP`
- `faraday/faraday-core/src/screens/message`: `MESSAGE_FIELD_PAD_X`, `MESSAGE_FIELD_PAD_Y`, `MESSAGE_FIELD_MIN_H`, `MESSAGE_SHOWN_GAP`
- `faraday/faraday-core/src/screens/mod`: `READING_PAGE_W`, `READING_TIME_W`, `READING_RANDOM_W`, `READING_EXPLORE_W`, `READING_TOOLS_W`, `READING_TRANSFER_W`, `PAGE_AREA_UNTIL_W`, `KEYGEN_COLUMN_IN`, `KEYGEN_COLUMN_MAX`, `TOP_ROW_QUESTION`, `HOME_RESULTS_FOOT`
- `faraday/faraday-core/src/screens/parts`: `PASS_CODE_INSET`, `PASS_CODE_TEXT`, `C32_GRID_LABEL`, `C32_GRID_MAX`, `C32_ZOOM_MAX`, `C32_ZOOM_MIN_CELL`
- `faraday/faraday-core/src/screens/randomness`: `OWN_AREA`
- `faraday/faraday-core/src/screens/scan_frame`: `VIEWFINDER_BRACKET_SHARE`, `VIEWFINDER_BRACKET_STROKE`, `VIEWFINDER_OUTLINE_STROKE`, `VIEWFINDER_OUTLINE_EDGE`, `VIEWFINDER_BRACKET_INSET`, `VIEWFINDER_BAND_PAD`, `VIEWFINDER_BAND_ALPHA`
- `faraday/faraday-core/src/screens/session_pane`: `SESSION_COL_MIN`, `SESSION_BAND_SHARE`, `SCAN_PANE_ASPECT`, `SCAN_PANE_MIN_H`, `SCAN_PANE_RADIUS`, `PICK_RADIO_DOT_INSET`, `HOME_BOX_KEYS`, `HOME_CARD_GAP`, `HOME_CARD_HEAD`, `HOME_BOX_MIN_ROWS`
- `faraday/faraday-core/src/screens/settings`: `APPEARANCE_LABEL_GAP`, `percent`
- `faraday/faraday-core/src/screens/sheets`: `CARD_FACT_LINE`, `QR_SHEET_MARGIN`, `GUIDE_SEARCH_TOP`, `GUIDE_SEARCH_AFTER`
- `faraday/faraday-core/src/screens/spend`: `REVIEW_ADDR_TOP_S`, `REVIEW_ADDR_TOP`, `REVIEW_ADDR_INSET_S`, `REVIEW_ADDR_INSET`, `REVIEW_ADDR_UNDER`, `REVIEW_ADDR_LINE_S`, `REVIEW_SCROLL_KEEP`, `TABLE_LABEL_W`, `TABLE_AMOUNT_W`, `FACT_INDENT`, `FINISH_CARD_GAP`, `FINISH_CARD_INSET`
- `faraday/faraday-core/src/screens/visit`: `VISIT_TICK_BOX_X`, `VISIT_NOTICE_H`, `VISIT_NOTICE_GAP`, `VISIT_NOTICE_BUTTON_PAD`, `VISIT_NAME_MIN`, `VISIT_ROW_BUTTON_H`, `VISIT_CARD_FOOT`, `VISIT_WRITE_FOOT`, `VISIT_BUTTON_PAD`, `VISIT_FOOT_ROW`, `VISIT_CHIP_PRESS_RISE`, `VISIT_SELECT_LABEL_X`, `VISIT_SELECT_END`, `VISIT_SELECT_MID`, `VISIT_GROUP_HEAD`, `VISIT_TEXT_X`, `VISIT_LOG_H`, `VISIT_COUNT_GAP`, `VISIT_CHIP_EXTRA`, `VISIT_CHIP_TOP`, `FAB_H`
- `faraday/faraday-core/src/screens/visit_same`: `VISIT_SAME_W`, `VISIT_SAME_LAYER`, `VISIT_SAME_ICON`, `VISIT_SAME_ICON_BOX`, `VISIT_ROW_BOX_X`, `VISIT_ROW_BOX_H`, `VISIT_SAME_TOP`, `alpha_of`
- `faraday/faraday-core/src/screens/wallets`: `WALLET_CARD_FOOT`
- `faraday/faraday-core/src/seed_check`: `CARD_ENTRY_ROOM`
- `faraday/faraday-core/src/seeds`: `SWEEP_QR`
- `faraday/faraday-core/src/sh:stick/src`: `PREVIEW_MAX`
- `faraday/faraday-core/src/silent`: `SILENT_LINE_GAP`
- `faraday/faraday-core/src/starter`: `IMAGE_LIST_GAP`, `STEP_SIDE`, `STEP_INDENT`, `CONTENT_MAX`, `CARD_MIN_W`, `KRUX_CELL_SMALL`, `KRUX_CELL_33`, `KRUX_CELL_37`, `KRUX_TITLE_SIZE`, `KRUX_TEXT_SIZE`, `KRUX_CAPTION_SIZE`, `KRUX_PASS_Y`, `KRUX_HAND_Y`, `KRUX_FIELDS_Y`, `KRUX_VERSION_END`, `KRUX_ROUNDS_X`, `KRUX_ROUNDS_END`, `KRUX_ID_X`, `KRUX_LABEL_W`, `KRUX_RULE_DROP`, `KRUX_RULE_GREY`, `KRUX_GRID_GAP`, `KRUX_CAPTION_GAP`, `TPL_OPTION_INDENT`, `TPL_OPTION_GAP`
- `faraday/faraday-core/src/stick`: `APP_BAR_PILLS_TITLE_ROOM`, `MOTION_LONG4`
- `faraday/faraday-core/src/tools`: `TOOLS_COLUMN_W`, `TOOLS_INSET`, `TAB_INDICATOR_PRIMARY`, `TAB_INDICATOR_MIN`
- `faraday/faraday-core/src/transfer`: `TRANSFER_W`, `FIELD_PAD_V`
- `faraday/faraday-core/src/ui`: `SLIDER_HALO`
- `faraday/faraday-core/src/vault`: `CHART_PANE_KINDS`, `CHART_PANE_ITEMS`, `CHECK_LABEL_INDENT`, `FILTER_CHIP_H`, `VAULT_ITEM_H`, `VAULT_ITEM_GAP`, `VAULT_ITEM_PAD`, `VAULT_ITEM_TOP`, `VAULT_ITEM_LINE`, `COPY_SHEET_INSET`, `COPY_SHEET_PAD`, `COPY_SHEET_SIDES`, `COPY_SHEET_TOP`, `COPY_SHEET_WIDE_PAD`, `COPY_HEAD_H`, `COPY_HEAD_TEXT_X`, `COPY_HEAD_BLOCK`
- `faraday/faraday-core/src/wallet_backup`: `KEF_DEFAULT_SIDE`
- `faraday/faraday-core/src/word_entry`: `CANDIDATE_CELLS_HANZI`, `CANDIDATE_ROWS_HANZI`
- `faraday/faraday-core/src/wordlist`: `SEARCH_FIELD_MAX`

Read by several app modules (token and number of modules):
`HEADLINE_S` 3, `TITLE_L` 9, `TITLE_M` 34, `TITLE_S` 31, `BODY_L` 28, `BODY_M` 65, `BODY_S` 62, `LABEL_L` 44, `LABEL_M` 33, `LABEL_S` 11, `TITLE` 3, `LABEL` 5, `GRID` 30, `GAP` 60, `CHIP_LINE` 4, `GAP_SMALL` 53, `PAD` 44, `RADIUS` 2, `RADIUS_SMALL` 7, `TOUCH` 41, `ICON` 2, `ICON_SMALL` 13, `ROW` 9, `LIST_INDENT` 2, `FLOW_INSET` 2, `FLOW_MAX` 2, `STEPPER_ROW` 4, `SETUP_CONTINUE_PAD` 2, `TEXT_FIELD` 9, `FIELD` 5, `CHART_SHEET_WIDTH` 4, `SIDE_SHEET_W` 2, `BOARD_GAP` 2, `BOARD_CHIP_GAP` 2, `BOARD_MARK` 3, `BOARD_EDGE` 3, `BOARD_POPUP_WIDTH` 2, `BOARD_STICK_HEAD` 2, `BOARD_STICK_PAD` 2, `HOLD_MS` 2, `NO_PICTURE_MS` 3, `SIDE_PANEL_TOP` 2, `CHOICES_INSET` 2, `MODE_PILL_HEIGHT` 4, `MODE_PILL_PAD` 2, `MODE_ROW` 2, `FIELD_RADIUS` 7, `TEXT_FIELD_SHEET` 4, `SEARCH_BAR_H` 4, `HOME_SEARCH_MIN` 2, `SEARCH_ICON` 2, `SEARCH_GLYPH` 2, `SEARCH_INSET` 2, `SEGMENTED_H` 4, `APPEARANCE_LABEL_W` 2, `APPEARANCE_ROW_GAP` 2, `RAIL_W` 2, `NAV_BAR_H` 2, `APP_BAR_H` 10, `NAV_ICON` 3, `MARGIN_COMPACT` 5, `MARGIN_WIDE` 12, `COLUMN_MAX` 14, `EXPLORE_INSET` 2, `PAGE_INSET` 5, `VISIT_GAP` 2, `VISIT_CARD_HEAD` 2, `VISIT_COMPACT_HEAD` 2, `VISIT_PAD` 2, `VISIT_TITLE_X` 2, `READING_VISIT_W` 2, `CATALOG_TILE_H` 2, `MENU_GAP` 4, `MENU_PAGE_W` 3, `READING_MENU_W` 2, `ACTIONS_UNDER` 2, `SPACER` 6, `LIST_PANE` 2, `PANE_TOP_BAR` 4, `PAGE_TOP_BAR` 4, `BUTTON_H` 53, `LIST_1` 10, `LIST_2` 8, `LIST_PAD_V` 7, `FIELD_H` 24, `FIELD_EDGE` 2, `FIELD_EDGE_ACTIVE` 6, `FIELD_PAD` 15, `FIELD_GAP` 4, `TAG_H` 3, `CARET_MIN` 4, `TYPED_INSET` 6, `BUTTON_PAD` 5, `LIST_PAD` 7, `SUPPORT_PANE_TOP` 3, `INPUT_PANEL_MAX` 2, `DICE_PAD_KEY_H` 3, `LEAD_MAX` 2, `TILE_H` 2, `HOME_TILE_GLYPH` 2, `STATUS_CHIP_H` 2, `OWN_BUTTON` 2, `CORRECT_GAP` 4, `ROLLER_GAP` 2, `ROLLER_FIVE_GAP` 2, `ROLLER_PAIR_DIE` 2, `ROLLER_PAIR_COL` 2, `ROLLER_PAIR_LABEL_W` 2, `ELEVATION_SHEET` 8, `ICON_BUTTON` 8, `CODEX32_TAB_GLYPH` 3, `TAB_H` 2, `SHAPE_XS` 3, `SHAPE_S` 16, `SHAPE_M` 26, `SHAPE_L` 6, `STATE_PRESS` 4, `PRESS_EMPHASIS` 2, `PRESS_EMPHASIS_MS` 2, `DISABLED_CONTENT` 7, `DISABLED_CONTAINER` 5, `ICON_GLYPH` 9, `ICON_GLYPH_SMALL` 5, `DISCLOSE_GLYPH` 2, `DISCLOSE_ICON_Y` 2, `ICON_GLYPH_BUTTON` 7, `ICON_GAP` 2, `CHEVRON` 3, `ROW_CHEVRON_ROOM` 7, `DIALOG_MAX` 16, `SCRIM_ALPHA` 2, `PROGRESS_H` 11, `MOTION_MEDIUM1` 2, `MOTION_MEDIUM2` 2, `MOTION_MEDIUM4` 2, `INVOICE_REFUSAL_GAP` 2, `INVOICE_TAIL` 2, `KEF_BY_HAND_MAX_SIDE` 2, `sheet_head` 13, `touch` 35, `value_row` 2, `field` 57, `candidates` 5, `qr_side` 3, `key_label` 11, `shape_full` 11.

Read only inside `faraday-ui`: `HEADLINE_M`, `WORDMARK_CAP_MIN`, `WORDMARK_STACK_WORD`, `WORDMARK_STACK_GAP`, `MARK_BOLD`, `LIST_RESERVE`, `OSK_KEY_ROW`, `SIDE_SHEET_PAD`, `FIELD_ACTION_HEIGHT`, `FIELD_ACTION_INSET`, `FIELD_ACTION_TEXT`, `FILTER_CHIP_LEAD`, `FILTER_CHIP_TRAIL`, `FILTER_CHIP_ICON`, `FILTER_CHIP_GAP`, `THEME_CHIP_DOT`, `THEME_CHIP_TICK`, `THEME_CHIP_RING`, `THEME_CHIP_RING_GAP`, `SWITCH_W`, `SWITCH_H`, `SWITCH_EDGE`, `SWITCH_HANDLE_OFF`, `SWITCH_HANDLE_ON`, `SWITCH_HANDLE_PRESSED`, `SWITCH_STATE`, `WINDOW_MEDIUM`, `NAV_PILL_W`, `NAV_PILL_H`, `NAV_ITEM_H`, `NAV_LABEL_GAP`, `RAIL_MARK_W`, `RAIL_MARK_H`, `NAV_RAIL_TOP`, `BADGE_SMALL`, `BADGE_LARGE`, `BADGE_PAD`, `BADGE_LABEL`, `LIST_3`, `LIST_3_TOP`, `BUTTON_PAD_V`, `MULTI_CHECK_X`, `TAG_DOT_R`, `TAG_DOT_TEXT_X`, `PIP_W`, `PIP_H`, `PIP_GAP`, `STEP_BADGE`, `STEP_BADGE_TICK`, `STEP_TOUCH`, `CHECK_TICK`, `FAB_H_COMPACT`, `FAB_PAD_COMPACT`, `FAB_PAD`, `CARET_OVER`, `NAV_ICON_GAP`, `SHADOW_REACH`, `ELEVATION_DIALOG`, `STEPPER_LABEL_TEXT`, `STEPPER_VALUE_TEXT`, `STEPPER_COUNT_W`, `STEPPER_LABEL_COL`, `CODEX32_TAB_ICON`, `TAB_INDICATOR_SECONDARY`, `TAB_PAD`, `SHAPE_XL`, `SHAPE_CHECKBOX`, `STATE_HOVER`, `STATE_FOCUS`, `HIGH_CONTRAST_TINT_MAX`, `FOCUS_RING`, `FOCUS_OFFSET`, `TEXT_LARGE_PCT`, `TEXT_LARGER_PCT`, `TEXT_FIT_STEP_PCT`, `SEGMENT_PAD`, `DIALOG_PAD`, `DIALOG_MIN`, `DIALOG_ACTION_GAP`, `TOAST_H`, `TOAST_PAD`, `TOAST_PAD_Y`, `TOAST_TEXT`, `TOAST_MARGIN`, `TOAST_MIN_W`, `TOAST_MAX_W`, `SELECTION_BAND`, `SELECTION_PAD`, `TOAST_STACK_GAP`, `MENU_MIN_W`, `MENU_MAX_W`, `MENU_HEAD_H`, `MENU_INDENT`, `TIP_PLAIN_MAX_W`, `TIP_PLAIN_MIN_H`, `TIP_PLAIN_PAD_X`, `TIP_PLAIN_PAD_Y`, `TIP_PLAIN_TEXT`, `TIP_RICH_MAX_W`, `TIP_RICH_PAD_X`, `TIP_RICH_PAD_Y`, `TIP_RICH_TEXT`, `TIP_RICH_LEVEL`, `TIP_GAP`, `TIP_DELAY_MS`, `TIP_LONG_PRESS_MS`, `EDGE_FADE_COMPACT`, `EDGE_FADE`, `EDGE_FADE_LOW_COMPACT`, `APP_BAR_TINT_SHARE`, `EDGE_FADE_ALPHA`, `PROGRESS_SPAN`.

Read by no app module (`HOST_*` and `PREVIEW_*` are read by `faraday-host`, `SHEET_*`, `PATHS_*`, `MOVE_TABLE_*` and `SHARE_*` by `faraday-paper`; `CAPTION`, `KEY_FACE_INSET` and `WIDE_STICK_ROW` by nothing): `CAPTION`, `KEY_FACE_INSET`, `WIDE_STICK_ROW`, `PREVIEW_SIZES`, `HOST_STRIP_H`, `HOST_CONTROL_H`, `HOST_LINE`, `HOST_INDICATOR`, `HOST_MENU_W`, `PREVIEW_SCALE_MIN`, `HOST_MENU_ROW`, `HOST_SHEET_W`, `HOST_FOCUS_EDGE`, `HOST_FOCUS_FADE_MS`, `HOST_MARK_EDGE`, `HOST_PULSE_MS`, `HOST_PULSES`, `HOST_FLASH_MS`, `HOST_PULSE_LAYER`, `HOST_MARK_LAYER`, `SHEET_TEXT_SIZE`, `SHEET_LABEL_SIZE`, `SHEET_FOOT_SIZE`, `SHEET_RULE_SIZE`, `SHEET_TITLE_SIZE`, `SHEET_RULE_GREY`, `PATHS_HEAD_GAP`, `PATHS_LINE`, `PATHS_WAIT_X`, `PATHS_GAP`, `PATHS_PARA_GAP`, `PATHS_END`, `PATHS_TITLE_GAP`, `PATHS_FIELDS_GAP`, `PATHS_RULE_DROP`, `PATHS_FIELD_ROW`, `MOVE_TABLE_COLS`, `MOVE_TABLE_ROWS`, `MOVE_TABLE_INSET`, `MOVE_TABLE_RULE_GAP`, `MOVE_TABLE_ROW`, `SHARE_QUORUM_GAP`, `SHARE_LINES_GAP`.

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

- The doc of `SESSION_COL_MIN` says "no code reads it yet"; it is read (see Reach). The doc is stale.
- Several text field heights coexist (`FIELD_H` 56, `FIELD` 52, `TEXT_FIELD` 40, `TEXT_FIELD_TALL` 42, `TEXT_FIELD_SHEET` 44, `field(Small)` 40); `LIST_1` equals `ROW` (56) and `LIST_2` equals `MENU_ROW` (72).
- `EDGE_FADE_ALPHA` is an f32 while other alphas are u8.
- The doc of `FAB_H_COMPACT` contradicts itself.
- `KEY_BOTTOM_RESERVE` is named in a doc comment; only the function `key_bottom_reserve` exists.
- `MENU_ROW_DIVIDERS` is never false.
- Values of the area tokens were read once by the inventory; only the plainly written line numbers were checked.
