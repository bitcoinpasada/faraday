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

- Callers were not traced past the component crates (faraday-ui and faraday-draw); the Reach section rests on the component code and a few searches, not on a read of every screen.
- `Warning` appears in this slice in tests only.
