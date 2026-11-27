# Scrolled regions and shadows

The scroll containers of the component layer (`faraday-ui`): a scrolled
region keyed by a `Slot`, its overlay scroll bar and cross-pan bar, edge
fades, overscroll stretch, moved-frame scrolling, the sheet backdrop
(frost and scrim) and elevation shadows. Every page, pane, sheet and list
that scrolls opens a region; the app keeps the offsets.

## Screens and sheets

All `faraday/faraday-ui/src/`:
- `Slot` (17 variants: Page, VisitOut, VisitIn, Card, VaultItem, ListPane, BoardList, Worksheet, WorksheetWhole, DemoPane, InputPanel, GuidePane, Choices, Sidebar, HomeBox, HomeBand, DiceWork; `faraday/faraday-ui/src/region.rs:~13`).
- Types: `Scrolled` (slot, view rect, max scroll, own-bar flag, max pan, `faraday/faraday-ui/src/region.rs:~76`); `Moves` (No default, Yes, `faraday/faraday-ui/src/region.rs:~94`); `Region` and builders `page`, `beside`, `scrolled`, `on`, `fades_into`, `own_bar`, `bar_shown`, `panned`, `from_zero`, `moves` (`faraday/faraday-ui/src/region.rs:~105-213`); `Plan` (`faraday/faraday-ui/src/region.rs:~222`); `Extent` (down, across, `faraday/faraday-ui/src/region.rs:~234`); `Open` (`faraday/faraday-ui/src/region.rs:~251`); `Layer` (`faraday/faraday-ui/src/region.rs:~264`).
- `Ui::region`, `region_open`, `region_close`, `region_begin`, `region_end`, `region_abandon`, `region_reported`, `page_begin`, `moving_page_begin`, `moving_panel_page_begin`, `page_top` (`faraday/faraday-ui/src/region.rs:~278-431`); `region_finish` (`faraday/faraday-ui/src/region.rs:~439`).
- Scroll chrome (`faraday/faraday-ui/src/lib.rs`): `BarGeometry` (`faraday/faraday-ui/src/lib.rs:~56-87`); `scroll_bar` (overlay thumb at the right edge, shortened by stretch, `faraday/faraday-ui/src/lib.rs:~796-828`); `scroll_bar_across` (`faraday/faraday-ui/src/lib.rs:~765-782`); `edge_fades` (`faraday/faraday-ui/src/lib.rs:~851-907`); `stretch_view` (`faraday/faraday-ui/src/lib.rs:~966-999`).
- `backdrop` (page frosted at 8 units per px, blur 2, dimmed by the scrim, cached, `faraday/faraday-ui/src/lib.rs:~909-925`); `still` (keeps the un-dimmed page under a popup, `faraday/faraday-ui/src/lib.rs:~927-937`); `shadow` / `shadow_faded` (soft black shadow at elevation 1-5, `faraday/faraday-ui/src/lib.rs:~939-964`).

## Actions

- Scrolling (wheel, drag, keys) is the app's; the region reads the offset for its `Slot`, clamps it to `max` and records a `Scrolled` entry so the app can scroll it.
- `BAR_GRAB` 16 units is the grab width of the bar (`faraday/faraday-ui/src/lib.rs:~51`).

## Effects

None outside the app's state.

## State and lock

- Offsets per `Slot` are the app's. Between `region_begin` and `region_end` an `Open` holds the saved offset, moved rows and saved seen clip.
- `region_finish` pushes or replaces the `scrolled` list; `stretch_view` uses `Ui.stretch`.
- `backdrop` and `still` keep a `frost` snapshot of the page for the life of the sheet or popup: keyed by screen, sheet and theme (`faraday/faraday-core/src/drawing.rs:898`), taken each frame and kept only if the frame drew one (`faraday/faraday-core/src/drawing.rs:919-922`, `faraday/faraday-core/src/fade.rs:404-420`), so the first frame without it drops it; a display change drops it (`faraday/faraday-core/src/fade.rs:394-399`).
- No secrets of its own.

## Reach

- Every page (`page_begin`), pane, list, sheet body (`ui-sheets`), menu, guide pane, worksheet and the input panel.
- Moves hit and reach rects with the stretch (`ui-press`).
- Uses `Canvas::frost`, `snapshot`/`restore`, `elevation` (`draw-base`).

Callers at main `d89dde5de` in `faraday-core`, the shells and the Starter, by module with the screen it draws and the number of call sites (`x2`); `git grep -n -w <item>` at that commit gives the lines.

Home, menu pages, panes, sidebar and navigation:
- `faraday/faraday-core/src/compact/` (the small panel (no sidebar): Home as the menu and its pages): `Scrolled`, `Slot`, `moving_page_begin`, `page_begin`, `region_end`
- `faraday/faraday-core/src/flow/` (the flow container, Your choices sheet and step cards): `Moves` x2, `Region` x8, `Slot` x5, `region_close` x4, `region_open` x3
- `faraday/faraday-core/src/guide.rs` (the Guide button's pages): `Slot` x2
- `faraday/faraday-core/src/lib.rs` (the app's root): `Slot`
- `faraday/faraday-core/src/motion.rs` (scroll and toast motion): `Scrolled` x7, `Slot` x76
- `faraday/faraday-core/src/moved.rs` (the moved frame (scroll step)): `Plan` x2, `Slot` x7
- `faraday/faraday-core/src/panes/` (list and detail panes): `Moves` x6, `Open`, `Region` x2, `Slot`, `region_close`, `region_open`
- `faraday/faraday-core/src/pointer.rs` (the pointer, focus and what a press reaches): `BarGeometry` x3, `Scrolled`, `Slot`
- `faraday/faraday-core/src/screens/bar_search.rs` (the search in the app bar and Home): `shadow`
- `faraday/faraday-core/src/screens/guide_pane.rs` (the guide's supporting pane): `Region` x3, `Slot` x2, `region_reported`
- `faraday/faraday-core/src/screens/hub_layout/` (the pane container): `Region` x4, `region_close` x3, `region_open` x3, `shadow` x2
- `faraday/faraday-core/src/screens/menu_page.rs` (the menu page container): `Region` x2, `region_close`, `region_open`
- `faraday/faraday-core/src/screens/mod.rs` (the screen dispatch and sheet/frame composition): `backdrop`, `region_abandon`, `still`
- `faraday/faraday-core/src/screens/row_details.rs` (a row's details in the second pane): `Region` x2, `region_close`, `region_open`, `shadow`
- `faraday/faraday-core/src/screens/session_pane/` (the session pane): `Region` x8, `Slot` x8, `region_close` x5, `region_open` x5, `shadow`
- `faraday/faraday-core/src/screens/sheets.rs` (the sheets drawn over a screen): `Region` x3, `region_reported`
- `faraday/faraday-core/src/screens/sidebar.rs` (the sidebar): `shadow`
- `faraday/faraday-core/src/ui.rs` (drawing in design units (the app's `Ui` alias and helpers)): `BarGeometry`, `Extent`, `Layer`, `Moves`, `Plan`, `Region`, `Scrolled`, `Slot`

Settings, Files, Modes, sticks and boot import:
- `faraday/faraday-core/src/boot_import/` (the boot import list): `Region`, `Slot`, `region_close`, `region_open`
- `faraday/faraday-core/src/codes/` (the TOTP codes screen): `page_begin`
- `faraday/faraday-core/src/screens/set_time.rs` (Set the time by hand): `Region` x4, `region_close` x2, `region_open` x2
- `faraday/faraday-core/src/screens/settings.rs` (Settings and About): `moving_page_begin`, `moving_panel_page_begin`
- `faraday/faraday-core/src/screens/visit.rs` (Files: In Faraday and On the stick): `Moves` x3, `Region` x6, `Slot` x2, `region_close` x3, `region_open` x4
- `faraday/faraday-core/src/transfer/` (Transfer: the QR link to the online app): `Region` x3, `region_close`, `region_open`

Wallets, vaults and keys: create, add, restore, seeds:
- `faraday/faraday-core/src/bip85/` (BIP-85 derived children): `Moves`
- `faraday/faraday-core/src/codex32/` (Make codex32 shares (flow, dice screen, worksheet, input panel)): `Extent` x4, `Region` x10, `Slot` x14, `region_close` x3, `region_open` x3
- `faraday/faraday-core/src/core_wallet/` (Read a Bitcoin Core wallet): `Moves`, `Region` x2, `region_close`, `region_open`
- `faraday/faraday-core/src/kef_tool/` (Krux code in Tools): `Moves`
- `faraday/faraday-core/src/keygen/` (Making a new key (randomness, word count, words)): `Moves` x2, `Slot` x3
- `faraday/faraday-core/src/screens/create.rs` (Create a wallet): `Moves`
- `faraday/faraday-core/src/screens/entry.rs` (typing a seed): `Open`, `Region` x3, `region_close`, `region_open`
- `faraday/faraday-core/src/screens/randomness.rs` (Add randomness): `page_begin`
- `faraday/faraday-core/src/screens/restore.rs` (Restore a wallet): `Moves`
- `faraday/faraday-core/src/screens/vaults_pair.rs` (Vaults as the wide pair): `shadow`
- `faraday/faraday-core/src/screens/wallets.rs` (the Wallets tab and wallet card): `Moves` x3, `Region` x3, `Slot` x2, `region_close` x2, `region_open` x2
- `faraday/faraday-core/src/screens/wallets_pair.rs` (Wallets as the wide pair): `shadow`
- `faraday/faraday-core/src/seed_check/` (My seed makes my wallet): `Moves`
- `faraday/faraday-core/src/vault/` (Vaults: list, create, load, copy, dice, secrets): `Moves` x4, `Region` x5, `Slot`, `moving_page_begin`, `page_top` x2, `region_close` x2, `region_open` x2
- `faraday/faraday-core/src/wordlist/` (Word lists): `Region` x3, `region_close`, `region_open`

Back up and spend from a backup:
- `faraday/faraday-core/src/backup_flow/` (the shared guided backup flow): `Moves`
- `faraday/faraday-core/src/backups/` (the Backups screen): `Region` x3, `moving_page_begin`, `region_close`, `region_open`
- `faraday/faraday-core/src/board/` (the backup board (Back up's plan)): `Region` x3, `Slot`, `region_close`, `region_open`, `shadow` x2
- `faraday/faraday-core/src/screens/backup.rs` (Back up a wallet): `Moves` x2, `shadow`
- `faraday/faraday-core/src/spend_backup/` (the Spend tab (spend from a backup)): `Moves`, `Region` x3, `region_close`, `region_open`

Spend, sign and check:
- `faraday/faraday-core/src/address_check/` (Check an address): `page_begin`
- `faraday/faraday-core/src/convert/` (Convert a seed): `Moves`
- `faraday/faraday-core/src/decode/` (Decode a transaction (incl. the nonce pane)): `Moves` x2, `Region` x3, `region_close` x2, `region_open`
- `faraday/faraday-core/src/invoice/` (Decode a Lightning invoice): `Moves`, `Region` x3, `region_close`, `region_open`
- `faraday/faraday-core/src/multisig_check/` (Check a multisig setup): `Moves`
- `faraday/faraday-core/src/ots_check/` (Check a timestamp): `Moves`, `Region` x3, `region_close`, `region_open`
- `faraday/faraday-core/src/screens/check_message.rs` (Check a signed message): `Moves`, `page_begin`
- `faraday/faraday-core/src/screens/message.rs` (Sign a message): `Moves`
- `faraday/faraday-core/src/screens/nonce_check.rs` (the nonce check of a signed PSBT or finished transaction): `Moves`, `Region` x3, `region_close`, `region_open`
- `faraday/faraday-core/src/screens/spend.rs` (Spend: sign a transaction): `Moves`

Tools and explorers:
- `faraday/faraday-core/src/explore/` (Explore: a loaded key at any path): `page_begin`
- `faraday/faraday-core/src/lightning/` (A Lightning node's key): `page_begin`
- `faraday/faraday-core/src/silent/` (Silent payments, receiving): `Moves`
- `faraday/faraday-core/src/tools/` (Tools: the six calculators): `Moves`, `page_begin`
- `faraday/faraday-core/src/vanity/` (Vanity addresses): `Moves`

Drawing support: frame bands, picture, budget:
- `faraday/faraday-core/src/drawing.rs` (what a frame works out once): `Layer`, `Slot`
- `faraday/faraday-core/src/fade.rs` (cross-fade on a change of screen or sheet): `Layer`, `Plan` x2
- `faraday/faraday-core/src/work/` (the draw-budget and timing tests): `Slot`

Faraday Starter (`faraday/faraday-starter/src/`):
- `faraday/faraday-starter/src/draw.rs` (the Starter's pages): `Region` x3, `backdrop`, `region_close`, `region_open`
- `faraday/faraday-starter/src/scroll.rs` (the Starter's page and sheet under a finger): `Slot`


Named in this file but called by no app module (`ui.` method calls and imported items matched): bar_shown beside elevation elevation_faded fades_into from_zero frost max moves on own_bar page panned region region_begin restore scrolled shadow_faded snapshot

## Rules and refusals

- Page content gets `foot_room` added to its scroll max (clearance for a floating button), only outside a sheet (`faraday/faraday-ui/src/region.rs:~443-448`).
- A region in a sheet is stretched only in the matching stretch mode (`faraday/faraday-ui/src/region.rs:~460-462`).
- For the active slot: moved-frame paste, a kept layer for moving pages, and the view stretched past its ends (`faraday/faraday-ui/src/region.rs:~463-475`).
- A moved frame is used only when the region moves (`Moves::Yes`), content is kept, the plan matches and the offset change is less than the view height (`faraday/faraday-ui/src/region.rs:~340-351`).
- A page under the top bar fades into the bar's fill (`faraday/faraday-ui/src/region.rs:~477-481`); a page starting at the frame's top with no bar does not fade at the top (`faraday/faraday-ui/src/region.rs:~484`).
- `bar_shown` forces a full-strength bar; otherwise the bar is shown for the active slot, not on the compact panel and not for own-bar regions (`faraday/faraday-ui/src/region.rs:~487-499`); the overlay bar alpha there is 255 (`faraday/faraday-ui/src/region.rs:~488`).
- The cross-scroll bar is drawn only for the active slot, not on compact (`faraday/faraday-ui/src/region.rs:~500-507`).
- Scroll bar and fades are not drawn without overflow (`max <= 0` skips, `faraday/faraday-ui/src/lib.rs:~861`; track or thumb `<= 0` skips, `faraday/faraday-ui/src/lib.rs:~770`, `faraday/faraday-ui/src/lib.rs:~800`).
- Bar geometry: width `BAR_W` 6 units (`faraday/faraday-ui/src/lib.rs:~47`), inset 4 units (`faraday/faraday-ui/src/lib.rs:~69`), minimum thumb 30 units (`faraday/faraday-ui/src/lib.rs:~74`), minimum thickness 3 px (`faraday/faraday-ui/src/lib.rs:~91`); alpha 150/255 below High contrast, 255 at High (`faraday/faraday-ui/src/lib.rs:~785-790`).
- Fade band clamped to `view.h / 4 + 1` (`faraday/faraday-ui/src/lib.rs:~870`); fade sizes `EDGE_FADE`, `EDGE_FADE_COMPACT`, `EDGE_FADE_LOW_COMPACT`, alpha `EDGE_FADE_ALPHA`; the top bar tint uses `APP_BAR_TINT_SHARE` (`faraday/faraday-ui/src/lib.rs:~830-848`).
- Frost: scale 8 units, at least 4 px, blur 2 (`faraday/faraday-ui/src/lib.rs:~915-917`); dimmed by `Scrim` at `SCRIM_ALPHA` (`faraday/faraday-ui/src/lib.rs:~916`).
- High contrast: no shadows (`faraday/faraday-ui/src/lib.rs:~940`, `faraday/faraday-ui/src/lib.rs:~955`), no edge fades (`faraday/faraday-ui/src/lib.rs:~861`), scroll bar at full alpha (`faraday/faraday-ui/src/lib.rs:~785-788`).
- The top edge fade is drawn only when `fade_top = under_bar || slot != Slot::Page || in_sheet || view.y > 0` (`faraday/faraday-ui/src/region.rs:484`; `edge_fades` takes `(fade_top, top_into)`, `faraday/faraday-ui/src/lib.rs:856,882-886`).

## Domain calls

- `Canvas::frost`, `Canvas::elevation`, `Canvas::elevation_faded`, `Canvas::shadow`, clip push and pop (`core/faraday-draw/src/canvas.rs`).
- `faraday_draw::tokens::{EDGE_FADE, EDGE_FADE_COMPACT, EDGE_FADE_LOW_COMPACT, EDGE_FADE_ALPHA, APP_BAR_TINT_SHARE, SCRIM_ALPHA, SHADOW_REACH, ELEVATION_*}` (`core/faraday-draw/src/tokens.rs`).

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 74fc0bda0 | A page that begins at the top of the frame with no bar over it drew a top edge fade where there is nothing to fade under. | The top fade is drawn only when `fade_top = under_bar || slot != Page || in_sheet || view.y > 0` (`faraday/faraday-ui/src/region.rs:484`, `faraday/faraday-ui/src/lib.rs:882-886`). |

## Doubts

