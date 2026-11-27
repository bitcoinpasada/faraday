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

Callers at main `d89dde5de` in `faraday-core`, the shells and the Starter, by module with the screen it draws and the number of call sites (`x2`); `git grep -n -w <item>` at that commit gives the lines.

Home, menu pages, panes, sidebar and navigation:
- `faraday/faraday-core/src/action.rs` (the press vocabulary (`Action`)): `Ui`
- `faraday/faraday-core/src/compact/` (the small panel (no sidebar): Home as the menu and its pages): `Ui` x46, `caret`, `draw_typed`, `draw_wrapped`, `field_box` x2, `field_label`, `fit_as` x7, `icon` x7, `keyboard` x11, `measure_as` x3, `row_chevron` x3, `text_as` x7, `text_mid_as` x6, `text_right_as`, `typed` x2, `wrap_as` x6, `wrap_height_as`, `wrap_lines_as`, `wrapped_as`
- `faraday/faraday-core/src/demo.rs` (what Practice refuses): `Ui` x2
- `faraday/faraday-core/src/file_menu.rs` (the file row's menu): `Ui` x7, `field_box`, `fit_as` x2, `icon`, `measure_as`, `text_as` x3, `text_mid_as` x2, `typed`, `wrap_as` x3
- `faraday/faraday-core/src/flow.rs` (what every flow shares): `Ui` x7, `icon` x2, `rule`, `text_mid_as`, `wrap_as` x2
- `faraday/faraday-core/src/flow/` (the flow container, Your choices sheet and step cards): `Ui` x21, `address_height`, `address_whole`, `fit_as` x3, `measure_as` x5, `rule`, `text_as` x8, `text_right_as`, `wrap_as` x2
- `faraday/faraday-core/src/guide.rs` (the Guide button's pages): `WrapBasis` x4
- `faraday/faraday-core/src/held.rs` (the backup-before-spending hold): `grouped`
- `faraday/faraday-core/src/in_progress.rs` (flows in progress): `Ui` x2, `fit_as`, `icon`, `text_mid_as`, `wrap_as` x2
- `faraday/faraday-core/src/leaks.rs` (the tours' secret-leak check): `Ui` x3
- `faraday/faraday-core/src/menu.rs` (the ⋮ menu): `Ui` x7
- `faraday/faraday-core/src/moved.rs` (the moved frame (scroll step)): `Ui`
- `faraday/faraday-core/src/nav/` (navigation by window class (rail, bar, app bar)): `Ui` x17, `fit_as` x2, `icon`, `measure_as` x2, `row_height`, `text_mid_as` x2
- `faraday/faraday-core/src/panes/` (list and detail panes): `ListItem` x5, `Ui` x10, `list_item`, `list_item_opens` x2
- `faraday/faraday-core/src/pointer.rs` (the pointer, focus and what a press reaches): `Ui`
- `faraday/faraday-core/src/qr_view.rs` (the QR view and the camera's scan state): `grouped`
- `faraday/faraday-core/src/register.rs` (registering a multisig wallet from its spend): `Ui` x2, `field_box`, `typed`, `wrap_as` x5
- `faraday/faraday-core/src/screens/bar_search.rs` (the search in the app bar and Home): `Ui` x6, `measure_as` x3
- `faraday/faraday-core/src/screens/catalog.rs` (Tools, every flow on one page): `Ui` x12, `caret` x2, `fit_as` x2, `icon` x2, `measure_as` x2, `text_as`, `text_mid_as` x4, `typed` x2, `wrap_as` x2, `wrap_height_as` x2
- `faraday/faraday-core/src/screens/guide_pane.rs` (the guide's supporting pane): `Ui` x3
- `faraday/faraday-core/src/screens/home.rs` (Home): `Ui` x7, `fit_as` x2, `icon`, `text_as` x2, `wrap_lines_as`
- `faraday/faraday-core/src/screens/hub_layout/` (the pane container): `Ui` x19
- `faraday/faraday-core/src/screens/menu_page.rs` (the menu page container): `Ui` x17, `icon`, `measure_as` x2, `row_chevron`, `text_mid_as`, `wrap_as` x2, `wrap_height_as` x4
- `faraday/faraday-core/src/screens/mod.rs` (the screen dispatch and sheet/frame composition): `Ui` x15, `back` x2, `fit_as` x3, `icon`, `text_as`, `text_mid_as` x2, `wrap_as`
- `faraday/faraday-core/src/screens/parts.rs` (rows, buttons, fields and word cells two or more screens draw): `FieldAt` x2, `Ui` x63, `button_height` x3, `caret` x4, `draw_wrapped` x3, `fit_as` x13, `fixed_text` x5, `icon` x3, `keyboard` x2, `measure_as` x21, `row_chevron`, `rule` x5, `text_as` x18, `text_fit_as`, `text_fit_width`, `text_mid_as` x22, `text_right_as` x5, `wrap_as` x11, `wrap_height_as`, `wrapped_as` x3
- `faraday/faraday-core/src/screens/row_details.rs` (a row's details in the second pane): `Ui` x6, `wrap_as` x2
- `faraday/faraday-core/src/screens/scan_frame.rs` (the scanner's camera picture and overlay): `Ui` x5, `fit_as`, `measure_as`, `text_as`
- `faraday/faraday-core/src/screens/session_pane/` (the session pane): `Ui` x36, `button_height`, `draw_wrapped` x2, `fit_as` x3, `icon` x3, `measure_as` x2, `row_chevron`, `row_height` x3, `rule`, `text_as` x4, `text_mid_as` x3, `text_right_as`, `wrap_as` x2, `wrap_lines_as`, `wrapped_as` x2
- `faraday/faraday-core/src/screens/sheets.rs` (the sheets drawn over a screen): `ListItem` x2, `Ui` x41, `address_whole`, `button_height` x4, `draw_wrapped_seen`, `fit_as` x14, `icon`, `list_item` x2, `measure_as` x10, `rule` x2, `text_as` x19, `text_mid_as`, `wrap_as` x43, `wrap_basis` x2, `wrap_height_as` x7, `wrap_ranges`, `wrapped_with` x2
- `faraday/faraday-core/src/screens/sidebar.rs` (the sidebar): `Ui` x4, `fit_as`
- `faraday/faraday-core/src/ui.rs` (drawing in design units (the app's `Ui` alias and helpers)): `FieldAt`, `Ui` x2, `icon`
- `faraday/faraday-core/src/visible.rs` (text with hidden characters made visible): `WrapBasis` x2

Settings, Files, Modes, sticks and boot import:
- `faraday/faraday-core/src/boot_import/` (the boot import list): `Ui` x11, `fit_as` x5, `icon` x3, `measure_as` x4, `text_as` x3, `text_mid_as` x3, `wrap_as` x7, `wrap_height_as`
- `faraday/faraday-core/src/codes/` (the TOTP codes screen): `Ui` x7, `fit_as` x3, `measure_as` x6, `text_as` x7, `text_mid_as`, `text_right_as`
- `faraday/faraday-core/src/modes/` (Modes and the mode pill): `Ui` x15, `fit_as`, `icon` x3, `measure_as` x4, `row_chevron`, `rule`, `text_mid_as` x6
- `faraday/faraday-core/src/samples/` (Faraday's samples (Modes)): `Ui` x7, `measure_as`, `text_as`, `text_mid_as`, `wrap_as`
- `faraday/faraday-core/src/screens/files.rs` (Files: Inbox and Outbox): `Ui` x15, `draw_wrapped`, `fit_as` x17, `icon` x16, `measure_as` x12, `rule` x2, `text_as` x24, `text_mid_as` x23, `text_right_as` x3, `wrap_as` x5, `wrapped_as`
- `faraday/faraday-core/src/screens/set_time.rs` (Set the time by hand): `Ui` x7, `button_height`, `text_as` x2, `text_mid_as`, `text_right_as`, `wrap_as` x4, `wrap_height_as`
- `faraday/faraday-core/src/screens/settings.rs` (Settings and About): `Ui` x22, `button_height` x3, `measure_as` x2, `text_as` x10, `text_mid_as` x2, `wrap_as` x8
- `faraday/faraday-core/src/screens/visit.rs` (Files: In Faraday and On the stick): `Ui` x28, `draw_wrapped`, `fit_as` x7, `fit_middle_as`, `icon` x3, `measure_as` x16, `row_chevron`, `rule` x3, `text_as` x11, `text_mid_as` x12, `text_right_as` x3, `wrap_as` x4, `wrap_height_as` x2, `wrapped_as` x2
- `faraday/faraday-core/src/screens/visit_same.rs` (a file in both Faraday and on the stick): `Ui` x4, `icon` x2
- `faraday/faraday-core/src/stick/` (the stick area): `Ui` x13, `button_height` x2, `draw_wrapped` x3, `fit_as` x3, `icon` x3, `measure_as` x2, `text_as` x4, `text_mid_as` x3, `text_right_as`, `wrap_as` x10, `wrapped_as` x3
- `faraday/faraday-core/src/transfer/` (Transfer: the QR link to the online app): `Ui` x5, `caret`, `draw_typed`, `fit_as` x3, `measure_as`, `rule` x2, `text_as` x4, `text_mid_as`, `text_right_as`, `wrap_as` x4, `wrapped_as`

Wallets, vaults and keys: create, add, restore, seeds:
- `faraday/faraday-core/src/bip85/` (BIP-85 derived children): `Ui` x6, `text_as` x5, `text_mid_as` x2, `typed`, `wrap_as` x4
- `faraday/faraday-core/src/codex32/` (Make codex32 shares (flow, dice screen, worksheet, input panel)): `FieldAt`, `Ui` x75, `caret`, `fit_as` x2, `fixed_text` x6, `grouped` x4, `icon` x3, `keyboard` x21, `measure_as` x11, `rule` x6, `short`, `text` x3, `text_as` x10, `text_fit_as` x6, `text_mid` x3, `text_mid_as` x6, `text_right`, `vrule` x2, `wrap_as` x22, `wrap_lines_as`
- `faraday/faraday-core/src/codex32_correct/` (codex32 error correction): `FieldAt`, `Ui` x15, `caret`, `icon`, `measure_as` x4, `text_mid_as` x7, `wrap_as` x3
- `faraday/faraday-core/src/core_wallet/` (Read a Bitcoin Core wallet): `Ui` x10, `field_label`, `icon`, `measure_as`, `text_mid_as`, `wrap_as` x12
- `faraday/faraday-core/src/dice_entry/` (Generate with dice on Create a vault): `Ui` x6, `field_h` x2, `fit_as`, `icon`, `measure_as`, `text_as` x4, `wrap_as` x4
- `faraday/faraday-core/src/edit.rs` (caret and selection of the field typed into): `FieldAt` x6
- `faraday/faraday-core/src/fresh/` (Add randomness (roller pages)): `Ui` x13, `measure_as`, `text_as` x4, `text_fit_as` x3, `text_fit_width` x2, `text_mid_as` x2
- `faraday/faraday-core/src/kef_form/` (the Krux encrypted code form): `ListItem`, `Ui` x14, `field_label` x3, `icon`, `list_item`, `text_as` x5, `wrap_as` x12
- `faraday/faraday-core/src/kef_tool/` (Krux code in Tools): `Ui` x10, `field_label`, `wrap_as` x3
- `faraday/faraday-core/src/keygen/` (Making a new key (randomness, word count, words)): `Ui` x64, `fit_as` x6, `icon` x6, `keyboard` x4, `measure_as` x31, `rule`, `text_as` x35, `text_mid_as` x28, `text_right_as` x3, `typed`, `wrap_as` x21
- `faraday/faraday-core/src/pass_field/` (the passphrase field): `Ui` x11, `field_supporting`, `fit_as`, `text_as`, `text_mid_as` x2, `typed` x2
- `faraday/faraday-core/src/screens/create.rs` (Create a wallet): `Ui` x23, `fit_as` x4, `measure_as` x5, `rule` x2, `text_as` x8, `wrap_as` x9
- `faraday/faraday-core/src/screens/create_making.rs` (Make a new wallet's Keys card): `Ui` x2, `wrap_as` x2
- `faraday/faraday-core/src/screens/entry.rs` (typing a seed): `Ui` x22, `caret` x2, `field_supporting`, `fit_as`, `icon` x2, `keyboard` x2, `measure_as` x6, `text_as` x6, `text_mid_as` x4, `text_right_as`, `typed`, `wrap_as` x8
- `faraday/faraday-core/src/screens/randomness.rs` (Add randomness): `Ui` x12, `fit_as` x2, `icon`, `measure_as` x4, `text_mid_as` x4, `wrap_as` x10, `wrap_height_as` x3
- `faraday/faraday-core/src/screens/restore.rs` (Restore a wallet): `Ui` x11, `fit_as` x2, `measure_as` x7, `rule` x6, `text_as` x4, `text_mid_as` x8, `wrap_as` x12
- `faraday/faraday-core/src/screens/vaults_pair.rs` (Vaults as the wide pair): `Ui` x4, `text_as` x2
- `faraday/faraday-core/src/screens/wallets.rs` (the Wallets tab and wallet card): `Ui` x17, `fit_as` x4, `icon` x3, `measure_as` x7, `row_chevron`, `rule` x2, `text_as` x11, `text_mid_as` x7, `text_right_as`, `typed`, `wrap_as` x9
- `faraday/faraday-core/src/screens/wallets/` (Wallets actions): `Ui` x2, `first_line_end`, `fit_as`, `icon`, `measure_as`, `row_chevron`, `text_mid_as` x3
- `faraday/faraday-core/src/screens/wallets_pair.rs` (Wallets as the wide pair): `Ui` x6, `fit_as`, `icon`, `row_chevron`, `text_as` x3
- `faraday/faraday-core/src/screens/wallets_start.rs` (Wallets' first page): `Ui` x7, `fit_as` x3, `measure_as` x3, `text_mid_as` x7
- `faraday/faraday-core/src/seed_check/` (My seed makes my wallet): `Ui` x16, `draw_wrapped` x2, `field_box` x2, `grouped` x2, `text_mid_as`, `typed` x2, `wrap_as` x7, `wrapped_as` x2
- `faraday/faraday-core/src/seeds/` (a wallet made from the seeds in hand): `Ui` x21, `field_box`, `fit_as` x4, `grouped`, `measure_as` x4, `rule`, `text_as` x5, `text_mid_as` x11, `text_right_as` x2, `typed` x2, `wrap_as` x31
- `faraday/faraday-core/src/vault/` (Vaults: list, create, load, copy, dice, secrets): `Ui` x94, `draw_wrapped`, `field_box` x2, `field_box_off` x2, `field_supporting` x2, `fit_as` x50, `icon` x10, `measure_as` x40, `row_chevron`, `rule` x15, `text_as` x92, `text_mid_as` x43, `text_right_as` x9, `typed` x2, `typed_lines`, `wrap_as` x62, `wrap_basis`, `wrap_lines_as`, `wrap_ranges`, `wrapped_with`
- `faraday/faraday-core/src/wallet.rs` (the Wallets tab's session): `grouped`
- `faraday/faraday-core/src/wallet/` (Coldcard export of a single-sig account): `Ui` x12, `icon` x2, `wrap_as` x8
- `faraday/faraday-core/src/ways_in/` (the ways a key reaches a screen): `Ui` x7, `icon`
- `faraday/faraday-core/src/word_entry.rs` (a list's words typed on the on-screen keyboard): `keyboard` x10
- `faraday/faraday-core/src/wordlist/` (Word lists): `Ui` x3, `field_box`, `fit_as` x2, `measure_as` x3, `rule`, `text_as` x4, `text_fit_as`, `text_mid_as` x3, `text_right_as`, `typed`

Back up and spend from a backup:
- `faraday/faraday-core/src/backup_flow/` (the shared guided backup flow): `ListItem` x2, `Ui` x54, `draw_typed` x2, `field_box`, `field_label`, `fit_as` x7, `icon` x3, `list_item`, `list_item_still`, `measure_as` x10, `rule` x2, `text_as` x15, `text_mid_as` x4, `text_right_as` x2, `wrap_as` x31, `wrap_height_as` x2, `wrap_lines_as`, `wrapped_as`
- `faraday/faraday-core/src/backups/` (the Backups screen): `Ui` x8, `fit_as` x6, `measure_as` x5, `text_as` x6, `text_mid_as` x6, `text_right_as` x2, `wrap_as` x4
- `faraday/faraday-core/src/board/` (the backup board (Back up's plan)): `Ui` x54, `fit_as` x7, `icon` x10, `measure_as` x11, `rule`, `text_as` x3, `text_mid_as` x12, `text_right_as`, `typed`, `wrap_as` x6, `wrap_height_as`
- `faraday/faraday-core/src/codex32_backup/` (codex32 strings in a backup): `Ui` x14, `fit_as` x2, `grouped` x2, `icon`, `keyboard` x10, `measure_as` x2, `text_mid_as` x4, `text_right_as`, `wrap_as` x16, `wrap_lines_as`
- `faraday/faraday-core/src/glance/` (the wallet at a glance chart): `Ui` x25, `caret`, `field_box`, `fit_as` x5, `measure_as` x12, `row_chevron` x2, `rule` x2, `text_as` x2, `text_mid_as` x2, `text_right_as` x3, `typed`, `vrule`, `wrap_as` x14, `wrap_lines_as` x2
- `faraday/faraday-core/src/plan/` (the backup plan's lists): `Ui` x5, `fit_as` x2, `measure_as` x2, `text_as` x2, `text_mid_as`, `text_right_as` x2, `wrap_as` x5
- `faraday/faraday-core/src/screens/backup.rs` (Back up a wallet): `Ui` x41, `draw_wrapped`, `fit_as` x16, `icon` x2, `measure_as` x8, `rule` x3, `text_as` x19, `text_mid_as` x12, `text_right_as` x3, `wrap_as` x45, `wrapped_as`
- `faraday/faraday-core/src/screens/backup/` (Back up, quick backup): `fit_as`, `text_mid_as`, `wrap_as`
- `faraday/faraday-core/src/spend_backup/` (the Spend tab (spend from a backup)): `Ui` x22, `button_height`, `draw_wrapped`, `fit_as` x11, `icon` x5, `measure_as` x10, `rule` x3, `text_as` x16, `text_mid_as` x21, `text_right_as` x3, `wrap_as` x31, `wrapped_as`
- `faraday/faraday-core/src/wallet_backup.rs` (a wallet's backup, two parts): `keyboard`
- `faraday/faraday-core/src/wallet_backup/` (a wallet's backup checklist cards): `Ui`

Spend, sign and check:
- `faraday/faraday-core/src/address_check/` (Check an address): `ListItem` x3, `Ui` x8, `button_height`, `grouped`, `list_item_whole`, `list_item_whole_still`, `text_as`, `typed_address`, `wrap_as` x4
- `faraday/faraday-core/src/address_find/` (Find an address (Explore and the address check's Look further)): `Ui` x3, `grouped`, `icon`, `rule` x2, `wrap_as` x3, `wrap_height_as`
- `faraday/faraday-core/src/check/` (Check, the tile grid): `Ui` x4, `icon`, `text_mid_as`
- `faraday/faraday-core/src/convert/` (Convert a seed): `Ui` x24, `field_box` x2, `typed` x2, `wrap_as` x15
- `faraday/faraday-core/src/decode/` (Decode a transaction (incl. the nonce pane)): `Ui` x25, `WrapBasis` x2, `draw_wrapped`, `fit_as` x4, `grouped` x9, `icon` x2, `measure_as` x10, `rule` x8, `short` x3, `text_as` x7, `text_mid_as` x12, `text_right_as` x3, `wrap_as` x19, `wrap_basis`, `wrap_height_as`, `wrap_ranges`, `wrapped_as`
- `faraday/faraday-core/src/invoice/` (Decode a Lightning invoice): `Ui` x11, `caret`, `caret_line`, `draw_typed_lines`, `grouped`, `rule` x2, `text_as` x2, `wrap_as` x4, `wrapped_as`, `wrapped_with`
- `faraday/faraday-core/src/multisig_check/` (Check a multisig setup): `Ui` x11, `icon`, `wrap_as` x9
- `faraday/faraday-core/src/ots_check/` (Check a timestamp): `Ui` x9, `field_box`, `field_label`, `icon`, `typed`, `wrap_as` x5
- `faraday/faraday-core/src/screens/check_message.rs` (Check a signed message): `Ui` x7, `grouped`, `icon`, `text_as` x2, `text_mid_as`, `wrap_as` x6
- `faraday/faraday-core/src/screens/message.rs` (Sign a message): `Ui` x6, `Wrapped`, `caret`, `draw_typed`, `draw_wrapped`, `measure_as` x2, `text_as` x2, `text_mid_as` x2, `wrap_as` x7, `wrap_basis`, `wrap_ranges`, `wrapped_with`
- `faraday/faraday-core/src/screens/nonce_check.rs` (the nonce check of a signed PSBT or finished transaction): `Ui` x6, `wrap_as` x5
- `faraday/faraday-core/src/screens/spend.rs` (Spend: sign a transaction): `Ui` x39, `address_whole`, `btc` x12, `button_height` x3, `draw_wrapped`, `fit_as` x13, `grouped`, `icon` x3, `measure_as` x28, `rule` x9, `short` x6, `text_as` x18, `text_mid_as` x31, `text_right_as` x12, `thousands` x3, `wrap_as` x27, `wrap_lines_as`, `wrapped_as`

Tools and explorers:
- `faraday/faraday-core/src/explore/` (Explore: a loaded key at any path): `Ui` x27, `caret_char`, `draw_wrapped`, `field_box` x2, `fit_as`, `grouped` x4, `icon`, `measure_as`, `text_as` x8, `text_mid_as` x7, `thousands` x5, `typed` x2, `typed_address`, `wrap_as` x8, `wrap_marked`, `wrapped_as`
- `faraday/faraday-core/src/gpg.rs` (GPG keys in a vault): `WrapBasis` x3
- `faraday/faraday-core/src/lightning/` (A Lightning node's key): `Ui` x6, `caret`, `fit_as`, `text_as` x4, `text_mid_as` x2, `text_right_as`, `typed`, `wrap_as` x4
- `faraday/faraday-core/src/silent/` (Silent payments, receiving): `Ui` x10, `address_whole`, `btc`, `field_box`, `field_h`, `text_as` x6, `text_mid_as`, `typed`, `wrap_as` x9
- `faraday/faraday-core/src/tools/` (Tools: the six calculators): `Ui` x11, `fit_as` x2, `measure_as` x2, `rule`, `text_as`, `text_mid`, `text_mid_as` x2, `thousands`, `typed`, `wrap_as` x2
- `faraday/faraday-core/src/vanity/` (Vanity addresses): `Ui` x5, `grouped`, `measure_as`, `text_as` x6, `text_mid_as` x4, `thousands` x5, `typed`, `wrap_as` x4

Drawing support: frame bands, picture, budget:
- `faraday/faraday-core/src/drawing.rs` (what a frame works out once): `Ui`

Faraday Starter (`faraday/faraday-starter/src/`):
- `faraday/faraday-starter/src/draw` (the Starter's pages (name, row, nav, guide, home, boot block)): `field_box`, `field_h`, `field_label`, `field_supporting`, `fit_as`, `icon` x3, `measure_as` x3, `rule` x8, `text_as` x4, `text_mid_as`, `typed`, `wrap_as` x12, `wrap_height_as` x2
- `faraday/faraday-starter/src/draw.rs` (the Starter's pages): `Ui`, `button_height` x2, `fit_as` x5, `fixed_text`, `grouped`, `icon` x3, `measure_as` x6, `row_height` x2, `rule` x13, `text_as` x2, `text_mid_as` x7, `text_right_as` x2, `wrap_as` x15, `wrap_height_as` x2
- `faraday/faraday-starter/src/lib.rs` (the Starter's root): `FieldAt`, `Ui`
- `faraday/faraday-starter/src/name.rs` (What to write's Name field): `FieldAt` x2
- `faraday/faraday-starter/src/scroll.rs` (the Starter's page and sheet under a finger): `Ui` x2


Named in this file but called by no app module (`ui.` method calls and imported items matched): field_box_sized fit fit_factor fit_middle list_item_height measure_text note_focus selection selection_band stepped text_wrapped wrap wrap_lines wrap_seen

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

- No fix rows were given to this feature; fixes about carets in text fields (for example a word grid showing a caret in every empty cell) were filed under the features that own those fields.
- Whether `typed` masks a secret is the caller's: the component draws what it is given.
- `selection` discards the text it would draw on the band (`faraday/faraday-ui/src/lib.rs:~1001-1009`); the caller draws the text.
