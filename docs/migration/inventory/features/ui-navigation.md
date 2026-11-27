# Navigation rail, bar and tabs

The navigation components of the component layer (`faraday-ui`): the
navigation rail (with its expanded row form and Scan button), the bottom
navigation bar, destination badges and flow-count pills, the status chip,
the small top app bar with back, title, actions and close, and tabs. App
shells and pages draw them; which destinations exist is the app's.

## Screens and sheets

All `faraday/faraday-ui/src/`:
- Tabs (`faraday/faraday-ui/src/tabs.rs`): `TabKind` Primary (indicator 3 dp, as wide as content) and Secondary (full-width 2 dp) (`faraday/faraday-ui/src/tabs.rs:~23`); `Tab` (label, on, done, action, `faraday/faraday-ui/src/tabs.rs:~34`); `tabs` (a row over an OutlineVariant rule, `faraday/faraday-ui/src/tabs.rs:~49`); `tab_chevron` (`faraday/faraday-ui/src/tabs.rs:~111`); `tab` (label TITLE_S, done check icon in `Role::Ok`, indicator, `faraday/faraday-ui/src/tabs.rs:~124`); `whole_tabs` (`faraday/faraday-ui/src/tabs.rs:~185`).
- Destinations (`faraday/faraday-ui/src/nav/mod.rs`): `Dest` (icon, label, action, current, badge, flows count, `faraday/faraday-ui/src/nav/mod.rs:~39`); `Badge` None, Dot, Count(n) (`faraday/faraday-ui/src/nav/mod.rs:~57`); `badge` (dot at the glyph's top right, or a large count pill in `BADGE` fill with `ON_BADGE` ink, `faraday/faraday-ui/src/nav/mod.rs:~71`); `flow_pill` (count pill in Primary/OnPrimary at the icon corner, `faraday/faraday-ui/src/nav/mod.rs:~130`); `item` (icon on a `NAV_PILL` pill, SecondaryContainer when current; whole box pressable, `faraday/faraday-ui/src/nav/mod.rs:~158`).
- `bar` (SurfaceContainer strip `NAV_BAR_H` tall, destinations evenly spaced, `faraday/faraday-ui/src/nav/bar.rs:~12`).
- `Chip` and `status_chip` (outlined assist chip, head OnSurface, tail OnSurfaceVariant at LABEL_L, press ringed by a `TOUCH`-high target, `faraday/faraday-ui/src/nav/chip.rs:~14`, `faraday/faraday-ui/src/nav/chip.rs:~29`, `faraday/faraday-ui/src/nav/chip.rs:~52`).
- Rail (`faraday/faraday-ui/src/nav/rail.rs`): `RailButton` (tonal icon button, Scan, `faraday/faraday-ui/src/nav/rail.rs:~19`); `Rail` (button, top, under after a divider, foot, mark flag, `faraday/faraday-ui/src/nav/rail.rs:~29`); `rail` (Surface column `RAIL_W` wide, OutlineVariant edge; destinations at top, optional divider and under group, Scan tonal button, foot pinned to the bottom, shield mark, `faraday/faraday-ui/src/nav/rail.rs:~47`); `destination_row` (expanded rail row, `faraday/faraday-ui/src/nav/rail.rs:~110`).
- App bar (`faraday/faraday-ui/src/nav/app_bar.rs`): `AppBar` (back label and action, title, back tip, `faraday/faraday-ui/src/nav/app_bar.rs:~21`); `fill_behind` (`faraday/faraday-ui/src/nav/app_bar.rs:~34`); `app_bar` (`APP_BAR_H`, optional round back with chevron and tooltip, "Back to X" in LABEL_M, title, `faraday/faraday-ui/src/nav/app_bar.rs:~50`); `two_lines` (`faraday/faraday-ui/src/nav/app_bar.rs:~125`); `pill_top`, `pill_top_at` (mode pill placement, `faraday/faraday-ui/src/nav/app_bar.rs:~134`, `faraday/faraday-ui/src/nav/app_bar.rs:~140`); `app_bar_fill` (`faraday/faraday-ui/src/nav/app_bar.rs:~147`); `bar_title_whole`, `bar_title_one_line` (`faraday/faraday-ui/src/nav/app_bar.rs:~158`, `faraday/faraday-ui/src/nav/app_bar.rs:~169`); `Name` and `bar_name` (title ladder, `faraday/faraday-ui/src/nav/app_bar.rs:~178`, `faraday/faraday-ui/src/nav/app_bar.rs:~195`); `icon_action`, `icon_action_at` (`faraday/faraday-ui/src/nav/app_bar.rs:~235`, `faraday/faraday-ui/src/nav/app_bar.rs:~248`); `help` (`faraday/faraday-ui/src/nav/app_bar.rs:~284`); `more` (three dots drawn as circles, `faraday/faraday-ui/src/nav/app_bar.rs:~295`); `close_button`, `close_button_in` (× at `ICON_GLYPH` in an `ICON_BUTTON` circle inside a `TOUCH` square, `faraday/faraday-ui/src/nav/app_bar.rs:~330`, `faraday/faraday-ui/src/nav/app_bar.rs:~342`).

## Actions

- A press on a destination, rail row, bar item, Scan button, status chip, tab, back, help, more, close or an icon action gives its action.
- A tab chevron at a scrolled row's end shows the next tab (`faraday/faraday-ui/src/tabs.rs:~111`).

## Effects

None outside the app's state.

## State and lock

- No component keeps state. `app_bar` writes `ui.under_bar` and `bar_pill_top`; `fill_behind` reads `ui.offset` and `in_sheet`.
- No secrets.

## Reach

- Drawn by the app shells and the pane container (rail, bar, app bar) and by pages with tabs.
- Uses `ui-press`, `ui-text` (fit, wrap), `ui-sheets` tooltips, `ui-theme` (`BADGE`, `ON_BADGE` fixed colours, F311).
- The mode pill placed by `pill_top` is drawn by the app.

Callers at main `d89dde5de` in `faraday-core`, the shells and the Starter, by module with the screen it draws and the number of call sites (`x2`); `git grep -n -w <item>` at that commit gives the lines.

Home, menu pages, panes, sidebar and navigation:
- `faraday/faraday-core/src/compact/` (the small panel (no sidebar): Home as the menu and its pages): `AppBar` x3, `Icon` x26, `app_bar` x2, `fill_behind`, `help`, `icon_action` x2, `icon_action_at`, `more` x2, `pill_top`
- `faraday/faraday-core/src/file_menu.rs` (the file row's menu): `Icon` x5
- `faraday/faraday-core/src/flow.rs` (what every flow shares): `Icon` x5
- `faraday/faraday-core/src/flow/` (the flow container, Your choices sheet and step cards): `AppBar`, `Icon`, `app_bar`
- `faraday/faraday-core/src/in_progress.rs` (flows in progress): `Icon` x7
- `faraday/faraday-core/src/medium.rs` (what the removable medium is called): `Icon` x4
- `faraday/faraday-core/src/menu.rs` (the ⋮ menu): `Icon` x8
- `faraday/faraday-core/src/nav/` (navigation by window class (rail, bar, app bar)): `AppBar` x3, `Badge` x9, `Chip` x2, `Dest` x5, `Icon` x32, `Rail` x2, `RailButton`, `app_bar` x2, `bar`, `help`, `icon_action` x2, `more`, `rail`, `status_chip`
- `faraday/faraday-core/src/register.rs` (registering a multisig wallet from its spend): `Icon` x3
- `faraday/faraday-core/src/screens/bar_search.rs` (the search in the app bar and Home): `Icon` x3
- `faraday/faraday-core/src/screens/catalog.rs` (Tools, every flow on one page): `Icon`
- `faraday/faraday-core/src/screens/guide_pane.rs` (the guide's supporting pane): `close_button` x2
- `faraday/faraday-core/src/screens/home.rs` (Home): `Icon` x33
- `faraday/faraday-core/src/screens/mod.rs` (the screen dispatch and sheet/frame composition): `Icon` x4
- `faraday/faraday-core/src/screens/parts.rs` (rows, buttons, fields and word cells two or more screens draw): `Icon` x5
- `faraday/faraday-core/src/screens/row_details.rs` (a row's details in the second pane): `Icon` x17
- `faraday/faraday-core/src/screens/session_pane/` (the session pane): `Icon` x46
- `faraday/faraday-core/src/screens/sheets.rs` (the sheets drawn over a screen): `Icon` x20, `Tab` x3, `TabKind`, `tabs`
- `faraday/faraday-core/src/screens/sidebar.rs` (the sidebar): `Badge`, `Dest`, `Icon` x19, `destination_row`

Settings, Files, Modes, sticks and boot import:
- `faraday/faraday-core/src/boot_import/` (the boot import list): `Icon` x7
- `faraday/faraday-core/src/modes/` (Modes and the mode pill): `Icon` x6
- `faraday/faraday-core/src/samples/` (Faraday's samples (Modes)): `Icon` x3
- `faraday/faraday-core/src/screens/files.rs` (Files: Inbox and Outbox): `Icon` x20
- `faraday/faraday-core/src/screens/visit.rs` (Files: In Faraday and On the stick): `Icon` x5, `Tab` x2, `TabKind`, `tabs`
- `faraday/faraday-core/src/screens/visit_same.rs` (a file in both Faraday and on the stick): `Icon` x4
- `faraday/faraday-core/src/stick/` (the stick area): `Icon` x11

Wallets, vaults and keys: create, add, restore, seeds:
- `faraday/faraday-core/src/bip85/` (BIP-85 derived children): `Icon` x4
- `faraday/faraday-core/src/codex32/` (Make codex32 shares (flow, dice screen, worksheet, input panel)): `Icon` x11, `Tab` x8, `TabKind` x2, `tabs`
- `faraday/faraday-core/src/codex32_correct/` (codex32 error correction): `Icon` x9
- `faraday/faraday-core/src/core_wallet/` (Read a Bitcoin Core wallet): `Icon` x4
- `faraday/faraday-core/src/create_pick.rs` (Create's Keys step as a pick): `Icon` x7
- `faraday/faraday-core/src/dice_entry/` (Generate with dice on Create a vault): `Icon` x3
- `faraday/faraday-core/src/kef_form/` (the Krux encrypted code form): `Icon` x4
- `faraday/faraday-core/src/kef_tool/` (Krux code in Tools): `Icon` x3
- `faraday/faraday-core/src/keygen/` (Making a new key (randomness, word count, words)): `Icon` x18, `bar_title_one_line` x2, `bar_title_whole`
- `faraday/faraday-core/src/screens/create.rs` (Create a wallet): `Icon` x3
- `faraday/faraday-core/src/screens/entry.rs` (typing a seed): `Icon` x9
- `faraday/faraday-core/src/screens/randomness.rs` (Add randomness): `Icon` x3
- `faraday/faraday-core/src/screens/vaults_pair.rs` (Vaults as the wide pair): `Icon` x3
- `faraday/faraday-core/src/screens/wallets.rs` (the Wallets tab and wallet card): `Icon` x7
- `faraday/faraday-core/src/screens/wallets/` (Wallets actions): `Icon` x16
- `faraday/faraday-core/src/screens/wallets_pair.rs` (Wallets as the wide pair): `Icon` x4
- `faraday/faraday-core/src/screens/wallets_start.rs` (Wallets' first page): `Icon` x9
- `faraday/faraday-core/src/seeds/` (a wallet made from the seeds in hand): `Icon` x9
- `faraday/faraday-core/src/vault/` (Vaults: list, create, load, copy, dice, secrets): `Icon` x48
- `faraday/faraday-core/src/wallet/` (Coldcard export of a single-sig account): `Icon` x4
- `faraday/faraday-core/src/ways_in/` (the ways a key reaches a screen): `Icon` x7

Back up and spend from a backup:
- `faraday/faraday-core/src/backup_flow/` (the shared guided backup flow): `Icon` x18
- `faraday/faraday-core/src/board/` (the backup board (Back up's plan)): `Icon` x20
- `faraday/faraday-core/src/codex32_backup/` (codex32 strings in a backup): `Icon` x3
- `faraday/faraday-core/src/glance/` (the wallet at a glance chart): `Icon` x21
- `faraday/faraday-core/src/screens/backup.rs` (Back up a wallet): `AppBar`, `Icon` x4, `app_bar`, `badge`
- `faraday/faraday-core/src/spend_backup/` (the Spend tab (spend from a backup)): `Icon` x8

Spend, sign and check:
- `faraday/faraday-core/src/address_check/` (Check an address): `Icon` x8
- `faraday/faraday-core/src/address_find/` (Find an address (Explore and the address check's Look further)): `Icon` x4
- `faraday/faraday-core/src/check/` (Check, the tile grid): `Icon` x5
- `faraday/faraday-core/src/convert/` (Convert a seed): `Icon` x7
- `faraday/faraday-core/src/decode/` (Decode a transaction (incl. the nonce pane)): `Icon` x18
- `faraday/faraday-core/src/invoice/` (Decode a Lightning invoice): `Icon` x4
- `faraday/faraday-core/src/multisig_check/` (Check a multisig setup): `Icon` x3
- `faraday/faraday-core/src/ots_check/` (Check a timestamp): `Icon` x4
- `faraday/faraday-core/src/screens/check_message.rs` (Check a signed message): `Icon` x6
- `faraday/faraday-core/src/screens/spend.rs` (Spend: sign a transaction): `AppBar`, `Icon` x7, `app_bar`

Tools and explorers:
- `faraday/faraday-core/src/explore/` (Explore: a loaded key at any path): `Icon` x5
- `faraday/faraday-core/src/silent/` (Silent payments, receiving): `Icon` x3
- `faraday/faraday-core/src/tools/` (Tools: the six calculators): `Icon` x4

Faraday Starter (`faraday/faraday-starter/src/`):
- `faraday/faraday-starter/src/draw` (the Starter's pages (name, row, nav, guide, home, boot block)): `AppBar` x2, `Icon` x13, `Rail` x2, `badge`
- `faraday/faraday-starter/src/draw.rs` (the Starter's pages): `Icon` x15, `badge`


Named in this file but called by no app module (`ui.` method calls and imported items matched): app_bar_fill close_button_in

## Rules and refusals

Tabs:
- Equal share when every label fits its share; natural widths when their total fits; else whole tabs around the shown one with a `TOUCH` chevron at each scrolled end, in that order (`faraday/faraday-ui/src/tabs.rs:67-72`, `faraday/faraday-ui/src/tabs.rs:~101-103`, `faraday/faraday-ui/src/tabs.rs:~185-206`). Padding `TAB_PAD` (`faraday/faraday-ui/src/tabs.rs:~61`); label TITLE_S (`faraday/faraday-ui/src/tabs.rs:~59`).
- A single tab wider than the row wraps onto more lines: lines times the TITLE_S line plus `2*GAP_SMALL` (`faraday/faraday-ui/src/tabs.rs:~74-86`).
- Indicator: Primary is `TAB_INDICATOR_PRIMARY`, width the larger of content and `TAB_INDICATOR_MIN`, round at the top (`faraday/faraday-ui/src/tabs.rs:~161-168`); Secondary is `TAB_INDICATOR_SECONDARY` across the whole tab (`faraday/faraday-ui/src/tabs.rs:~170-171`); an OutlineVariant rule along the row's bottom (`faraday/faraday-ui/src/tabs.rs:~87`).
- Check icon `CODEX32_TAB_ICON`, `CODEX32_TAB_GLYPH`, `Role::Ok` (`faraday/faraday-ui/src/tabs.rs:~139-147`); chevron `TOUCH` wide, `ICON_BUTTON`, `ICON_GLYPH` (`faraday/faraday-ui/src/tabs.rs:~112-119`).

Rail and bar:
- Rail width `RAIL_W`; destination height `clamp(room / n, TOUCH, NAV_ITEM_H + line_grown_as(LABEL_M))` (`faraday/faraday-ui/src/nav/rail.rs:~65`); the mark gap shrinks first when short (`faraday/faraday-ui/src/nav/rail.rs:~57-61`); a `2*GAP` divider when an under group exists (`faraday/faraday-ui/src/nav/rail.rs:~51`); rule at PAD inset (`faraday/faraday-ui/src/nav/rail.rs:~82`); Scan button height `TOUCH + GRID` (`faraday/faraday-ui/src/nav/rail.rs:~52-54`, `faraday/faraday-ui/src/nav/rail.rs:~89-96`); foot pinned from the bottom (`faraday/faraday-ui/src/nav/rail.rs:~97-101`); Surface fill, OutlineVariant edge (`faraday/faraday-ui/src/nav/rail.rs:~48-49`).
- Expanded rail row: `TOUCH` tall, pill radius `TOUCH/2`, icon `ICON_SMALL` with `NAV_ICON` glyph, label fitted in the room (`faraday/faraday-ui/src/nav/rail.rs:~117-158`).
- Nav item: `NAV_PILL_W` x `NAV_PILL_H` pill, `NAV_LABEL_GAP`, label LABEL_M (regular unless current) fitted to `w - 2*GAP_SMALL` (`faraday/faraday-ui/src/nav/mod.rs:~149-203`).
- Nav bar: `NAV_BAR_H`, SurfaceContainer, equal share per destination (`faraday/faraday-ui/src/nav/bar.rs:~13-16`).
- Badge: `BADGE_SMALL` dot; large count pill `BADGE_LARGE` high, `BADGE_LABEL` text, `BADGE_PAD` each side, minimum `BADGE_LARGE` (`faraday/faraday-ui/src/nav/mod.rs:~71-125`); count as its number up to 999, "999+" above (`faraday/faraday-ui/src/nav/mod.rs:~91-97`).
- Flows count: in a bar item it is a Primary/OnPrimary pill (`flow_pill`, `faraday/faraday-ui/src/nav/mod.rs:146`); in a rail row it is drawn as `badge(Badge::Count(flows))` in badge colours (`faraday/faraday-ui/src/nav/rail.rs:136-145`). A Dot badge also counts as the row's own badge (`faraday/faraday-ui/src/nav/rail.rs:139`), so the count moves down by `BADGE_LARGE + 2` for a dot too.

Status chip:
- `STATUS_CHIP_H`, `STATUS_CHIP_PAD`, `GRID` corner radius and gap, `ICON_SMALL` with a `GAP_SMALL` lead, LABEL_L label; Outline stroke always; text fitted down by the fit factor (`faraday/faraday-ui/src/nav/chip.rs:~7-9`, `faraday/faraday-ui/src/nav/chip.rs:~34-48`, `faraday/faraday-ui/src/nav/chip.rs:~77-113`).

App bar:
- `APP_BAR_H` tall; left margin `MARGIN_COMPACT` (`faraday/faraday-ui/src/nav/app_bar.rs:~55`); back at `GAP_SMALL` inset in a `TOUCH` square (`faraday/faraday-ui/src/nav/app_bar.rs:~58-59`).
- Title ladder: whole in TITLE_L, then TITLE_M; two lines in TITLE_M or TITLE_S; else shortened with an ellipsis; an item name is cut inside its word when `bar_item_name` (`faraday/faraday-ui/src/nav/app_bar.rs:~195-203`); a role whose two lines exceed `APP_BAR_H` is skipped (`faraday/faraday-ui/src/nav/app_bar.rs:~203`); two lines centred (`faraday/faraday-ui/src/nav/app_bar.rs:~127-130`).
- Mode pill `MODE_PILL_HEIGHT` centred on the title line and clamped inside the bar (`faraday/faraday-ui/src/nav/app_bar.rs:~135`).
- Actions: icon in a `TOUCH` target at the right end (`faraday/faraday-ui/src/nav/app_bar.rs:~257-277`); on = SecondaryContainer fill and OnSecondaryContainer icon; `more` dots at `GAP_SMALL` radius over 2 steps (`faraday/faraday-ui/src/nav/app_bar.rs:~309-318`).
- Fill: Surface, or SurfaceContainer when scrolled (`faraday/faraday-ui/src/nav/app_bar.rs:~147-152`); at High contrast `app_bar_fill`, else a bar-tone wash by the scroll offset (`fill_behind`, `faraday/faraday-ui/src/nav/app_bar.rs:~34`); no rule below the bar (`faraday/faraday-ui/src/nav/app_bar.rs:~49`, `faraday/faraday-ui/src/nav/mod.rs:~9-12`).
The flows-in-progress mark is a Primary pill with OnPrimary ink on the bar's and the collapsed rail's icons (faraday/faraday-ui/src/nav/mod.rs:127-147), and Bitcoin orange with a white count in the expanded rail's row (faraday/faraday-ui/src/nav/rail.rs:104-146).

## Domain calls

- `faraday_draw::tokens` (`RAIL_W`, `NAV_*`, `APP_BAR_H`, `TAB_*`, `BADGE_*`, `STATUS_CHIP_*`, `MODE_PILL_HEIGHT`, `MARGIN_COMPACT`, `TOUCH`, `ICON_BUTTON`) in `core/faraday-draw/src/tokens.rs`.
- `faraday_draw::widgets::icon::Icon` (`core/faraday-draw/src/widgets/icon.rs`).
- `faraday/faraday-ui/src/pal/mod.rs` `BADGE`, `ON_BADGE` (`faraday/faraday-ui/src/pal/mod.rs:~157`, `faraday/faraday-ui/src/pal/mod.rs:~159`).

## Fixtures

None.

## Fixes

None given to this feature.

## Doubts

- `Warning` appears in this slice in tests only.
