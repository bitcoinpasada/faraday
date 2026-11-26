# Menus, sheets, dialogs and snackbar

The surfaces that sit over a page in the component layer (`faraday-ui`):
popup menus (a sheet on the small panel), centred sheets, the side sheet,
dialogs and their action rows, key-value dialogs, the snackbar (toast),
tooltips, and small parts used inside them (titles, button rows, the sheet
box, the QR card, the step circle) and the stick pill. Screens of every app
open them; the app decides what a sheet holds.

## Screens and sheets

All `faraday/faraday-ui/src/`:
- Menu (`faraday/faraday-ui/src/menu.rs`): `MenuLine` Head, Item, Icon, Danger, Divider, Off (`faraday/faraday-ui/src/menu.rs:~22`); `Menu` (title, icon, lines, keyboard index, anchor rect, cancel button, `shown` 0..1, `faraday/faraday-ui/src/menu.rs:~44`); `menu` draws a popup with an anchor (SurfaceContainer, `ELEVATION_MENU` shadow, `SHAPE_XS` corners, scrolls within itself) or, on the compact panel, a `scroll_sheet` with a title head and Cancel (`faraday/faraday-ui/src/menu.rs:~247`); `lines` (`faraday/faraday-ui/src/menu.rs:~137`) and `item` (chosen fill, state layer, ring, optional icon, fitted label, Done tick, `faraday/faraday-ui/src/menu.rs:~203`); `menu_width` (`faraday/faraday-ui/src/menu.rs:~90`), `content_h` (`faraday/faraday-ui/src/menu.rs:~118`).
- Parts (`faraday/faraday-ui/src/parts.rs`): `title` (HEADLINE_M, `faraday/faraday-ui/src/parts.rs:~11`); `section_label` (TITLE_S, `faraday/faraday-ui/src/parts.rs:~17`); `chip_grid` (`faraday/faraday-ui/src/parts.rs:~27`); `button_rows`, `button_rows_ending` (`faraday/faraday-ui/src/parts.rs:~53`, `faraday/faraday-ui/src/parts.rs:~65`); `wrap_buttons`, `wrap_buttons_spaced`, `wrap_buttons_height` (`faraday/faraday-ui/src/parts.rs:~104`, `faraday/faraday-ui/src/parts.rs:~117`, `faraday/faraday-ui/src/parts.rs:~153`); `measured` (`faraday/faraday-ui/src/parts.rs:~178`); `sheet_box`, `sheet_box_in` (centred box, `SHAPE_XL`, `ELEVATION_DIALOG`, SurfaceContainerHigh, `faraday/faraday-ui/src/parts.rs:~191`, `faraday/faraday-ui/src/parts.rs:~203`); `qr_code` (white card radius 8.0, dark square modules, quiet zone 4 modules, `faraday/faraday-ui/src/parts.rs:~237`); `step_circle` (`faraday/faraday-ui/src/parts.rs:~269`).
- Sheets (`faraday/faraday-ui/src/sheet.rs`): `sheet` (panel `w - 2*GAP`, body measured then drawn, buttons pinned at the foot, `faraday/faraday-ui/src/sheet.rs:~20`); `scroll_sheet` (SurfaceContainerLow, `faraday/faraday-ui/src/sheet.rs:~28`); `framed` (two passes: measure with zero clip, then fill, shadow, outline at High contrast, scrolled body, footer, `faraday/faraday-ui/src/sheet.rs:~49`); `side_sheet` (docked at `right`, `shown` slide, Surface fill, 1 px OutlineVariant leading divider, no scrim, header with `sheet_head` and a Ghost Close text button, `faraday/faraday-ui/src/sheet.rs:~162`); `side_body` (`faraday/faraday-ui/src/sheet.rs:~202`); `dialog`, `dialog_in` (`faraday/faraday-ui/src/sheet.rs:~265`, `faraday/faraday-ui/src/sheet.rs:~277`); `end_buttons` (`faraday/faraday-ui/src/sheet.rs:~310`); `sheet_head` (`faraday/faraday-ui/src/sheet.rs:~342`); `kv` (key BODY_S, value BODY_M, rule under, `faraday/faraday-ui/src/sheet.rs:~363`); `buttons`, `buttons_by`, `buttons_with`, `buttons_height`, `stacked`, `buttons_fit` (`faraday/faraday-ui/src/sheet.rs:~381-457`); `kv_sheet` (dialog at `DIALOG_MAX` of key-value rows, `faraday/faraday-ui/src/sheet.rs:~509`).
- Snackbar (`faraday/faraday-ui/src/snackbar.rs`): `snackbar` (InverseSurface pill, InverseOnSurface text `TOAST_TEXT`, lines centred, `ELEVATION_FAB` shadow, fade by `shown`, rise offset, `faraday/faraday-ui/src/snackbar.rs:~39`); `snackbar_closable` (adds a `TOUCH`-square close button with U+00D7, `faraday/faraday-ui/src/snackbar.rs:~48`); `snackbar_with` (`faraday/faraday-ui/src/snackbar.rs:~58`); `snackbar_size` (`faraday/faraday-ui/src/snackbar.rs:~121`); `snackbar_room`, `snackbar_closable_room` (`faraday/faraday-ui/src/snackbar.rs:~145`, `faraday/faraday-ui/src/snackbar.rs:~152`); `above_fab` (`faraday/faraday-ui/src/snackbar.rs:~160`); `snackbar_line` (`faraday/faraday-ui/src/snackbar.rs:176`).
- Tooltip (`faraday/faraday-ui/src/tooltip.rs`): `TipKind` plain (inverse surface, short label) and rich (surface container, `TIP_RICH_LEVEL` shadow, supporting text, `faraday/faraday-ui/src/tooltip.rs:~42`); `TipAnchor` (`faraday/faraday-ui/src/tooltip.rs:~51`); `TipFrame` (`faraday/faraday-ui/src/tooltip.rs:64`); `TipBy` Hover, Focus, Touch, Press (`faraday/faraday-ui/src/tooltip.rs:~82`); `Tips` (`faraday/faraday-ui/src/tooltip.rs:~96`); `tip_anchor` (`faraday/faraday-ui/src/tooltip.rs:~290`), `tip_plain`, `hit_tip` (`faraday/faraday-ui/src/tooltip.rs:~311`, `faraday/faraday-ui/src/tooltip.rs:~317`); `draw_tip` (`faraday/faraday-ui/src/tooltip.rs:~326`).
- Stick pill (`faraday/faraday-ui/src/stick.rs`): `StickState` Quiet, Asked, Inserted (`faraday/faraday-ui/src/stick.rs:~15`); `PillAt` (`faraday/faraday-ui/src/stick.rs:~28`); `stick_paints` (`faraday/faraday-ui/src/stick.rs:~48`); `stick_pill` (round pill, optional edge, optional press, dot, `LABEL_M` label, `faraday/faraday-ui/src/stick.rs:~79`).

## Actions

- Menu: a press on an item gives its action; a press beside the popup goes to `outside` (`faraday/faraday-ui/src/menu.rs:~282-284`); Off items take no press; the keyboard index moves the ring.
- Sheet box and sheets: a press beside the box goes to `outside` (`faraday/faraday-ui/src/parts.rs:~191-203`).
- Side sheet: Close text button.
- Snackbar with close: the × press gives `close`; it is drawn last so its press is topmost (`faraday/faraday-ui/src/snackbar.rs:~58`).
- Tooltip: shown after hover, focus, a long press on touch, or a press (`Tips::hover`, `focus`, `press_down/moved/up`, `show`, `dismiss`, `tick`).
- `step_circle` is optionally pressable with a `STEP_TOUCH` target and ring.

## Effects

None outside the app's state. The QR card draws cells and keeps nothing.

## State and lock

- `Tips` (`faraday/faraday-ui/src/tooltip.rs:~96`) holds the last frame's anchors, `coming` (owner, `TipBy`, since ms) and `shown`; `Ui.tips: TipFrame<A>` is the live per-frame field (`faraday/faraday-ui/src/lib.rs:631`, `faraday/faraday-ui/src/lib.rs:744`). Tooltip text is static strings.
- `framed` and `side_body` write `ui.sheet_pin`, `sheet_pinning` and save and restore `focus_box`, and truncate the hit and reach lists back after measuring (`faraday/faraday-ui/src/sheet.rs:~49`, `faraday/faraday-ui/src/sheet.rs:~202`).
- `snackbar` reads `fab_at`, `ox`, `oy` (`faraday/faraday-ui/src/snackbar.rs:~160`).
- No menu, sheet or snackbar keeps state: `shown`, `rise`, `offset` and the menu's index are the caller's. No secrets kept here; a QR card of a secret is drawn from what the caller passes and nothing is kept (`faraday/faraday-ui/src/parts.rs:~244-259`).

## Reach

- Opened by every screen's sheets and menus; tooltips by icon buttons and app-bar actions (`ui-navigation`, `ui-controls`).
- Uses `ui-regions` (scrolled bodies, backdrop and shadow), `ui-press`, `ui-text`, `ui-controls` buttons.
- The stick pill is drawn by screens that show sticks; its tones are `ui-theme`'s stick tones.

## Rules and refusals

Menu:
- Width is the widest line plus `2*MENU_ITEM_PAD`, clamped to `MENU_MIN_W..MENU_MAX_W` and at most frame width minus `2*GAP` (`faraday/faraday-ui/src/menu.rs:~113`, `faraday/faraday-ui/src/menu.rs:~262`); the measure counts tick room for items and icon room for icon items (`faraday/faraday-ui/src/menu.rs:~99-109`).
- Compact panel: a sheet with optional head, lines, then Cancel; wide: a popup (`faraday/faraday-ui/src/menu.rs:~248-260`).
- Popup placement: right edge at the anchor's right edge (clamped inside the margin); below the anchor if it fits, else above, else clamped to the frame (`faraday/faraday-ui/src/menu.rs:~266-281`).
- The popup unrolls from its top; nothing is drawn when `shown` is 0 (`faraday/faraday-ui/src/menu.rs:~286-290`).
- High contrast draws an Outline stroke in place of the shadow (`faraday/faraday-ui/src/menu.rs:~295-297`).
- Content is a page region on SurfaceContainer scrolled by `offset` (`faraday/faraday-ui/src/menu.rs:~300-309`); presses scrolled out of the popup are cut or dropped (`faraday/faraday-ui/src/menu.rs:~311-320`).
- A chosen item fills SecondaryContainer with a tick at the end (`faraday/faraday-ui/src/menu.rs:~210-212`, `faraday/faraday-ui/src/menu.rs:~237-241`); the focus ring is drawn inside the item bounds (`faraday/faraday-ui/src/menu.rs:~216-227`).
- A Danger item is in the Error role and never chosen; Off items take no press (`faraday/faraday-ui/src/menu.rs:~166`, `faraday/faraday-ui/src/menu.rs:~188-196`).

Parts:
- `chip_grid`: one row if every chip fits at its width, else rows of equal length (`faraday/faraday-ui/src/parts.rs:~40-46`).
- Button rows are at least `ui.touch` apart (`faraday/faraday-ui/src/parts.rs:~74`, `faraday/faraday-ui/src/parts.rs:~133`, `faraday/faraday-ui/src/parts.rs:~163`); a button is no wider than its row, label cut to fit (`faraday/faraday-ui/src/parts.rs:~79`, `faraday/faraday-ui/src/parts.rs:~136`); a Ghost button alone at a row's start is drawn as a text button (`faraday/faraday-ui/src/parts.rs:~85-95`, `faraday/faraday-ui/src/parts.rs:~142-146`).
- `measured` draws with a zero clip and drops its hits and reach (`faraday/faraday-ui/src/parts.rs:~176-186`).
- Sheet box: `SHAPE_XL`, dialog elevation, SurfaceContainerHigh; Outline at High contrast (`faraday/faraday-ui/src/parts.rs:~214-230`).
- QR: white card, whole-pixel modules, square with quiet zone (`faraday/faraday-ui/src/parts.rs:~244-259`).
- Step circle: filled Ok with OnOk number when ticked, outlined otherwise, target `STEP_TOUCH` (`faraday/faraday-ui/src/parts.rs:~278-301`); number in TITLE_S for one digit, LABEL_M for two (`faraday/faraday-ui/src/parts.rs:~280`).

Sheets and dialogs:
- Sheet panel width `w - 2*GAP` at x = GAP; margin GAP, padding PAD; height content plus footer, clamped to `h - 2*margin`, centred (`faraday/faraday-ui/src/sheet.rs:~21`, `faraday/faraday-ui/src/sheet.rs:~92-99`).
- Footer buttons are pinned at the foot with `pad` above and below; the body scrolls in the rest; offset clamped to `max` (`faraday/faraday-ui/src/sheet.rs:~86-128`).
- Sheet fill SurfaceContainerLow (`faraday/faraday-ui/src/sheet.rs:~40`); dialog fill SurfaceContainerHigh (`faraday/faraday-ui/src/sheet.rs:~295`); corners `SHAPE_XL` (`faraday/faraday-ui/src/sheet.rs:~56`, `faraday/faraday-ui/src/sheet.rs:~109`); shadow `ELEVATION_SHEET` (`faraday/faraday-ui/src/sheet.rs:~110`); Outline stroke at High contrast only (`faraday/faraday-ui/src/sheet.rs:~115-117`).
- Dialog, compact: x0 + GAP, width `w - 2*GAP`, margin GAP, pad PAD, actions share the width (`faraday/faraday-ui/src/sheet.rs:~285-286`). Wide: width `want` clamped to `DIALOG_MIN..DIALOG_MAX` and to `w - 2*PAD`, centred, margin PAD, pad `DIALOG_PAD`, actions end-aligned (`faraday/faraday-ui/src/sheet.rs:~288-289`), gap `DIALOG_ACTION_GAP` (`faraday/faraday-ui/src/sheet.rs:~317`); end buttons fall back to `buttons` when their total exceeds the width.
- Side sheet: `SIDE_SHEET_PAD` padding (`faraday/faraday-ui/src/sheet.rs:~170`); 1 px OutlineVariant leading divider (`faraday/faraday-ui/src/sheet.rs:~174`); close button width label + `2*PAD`, touch target offset `(TOUCH - BUTTON_H)/2` (`faraday/faraday-ui/src/sheet.rs:~186-194`); body top at least `TOUCH - PAD/2` below the top (`faraday/faraday-ui/src/sheet.rs:~195`); pinned buttons under an OutlineVariant divider when the body scrolls.
- Button row: equal share `(w - (n-1)*GAP)/n`; side by side when each label + `BUTTON_PAD` fits its share (`faraday/faraday-ui/src/sheet.rs:~416-425`, `faraday/faraday-ui/src/sheet.rs:~478-482`); else stacked full width, each at least `BUTTON_H`, labels wrapped at word breaks inside `BUTTON_PAD_V` (`faraday/faraday-ui/src/sheet.rs:~398-413`, `faraday/faraday-ui/src/sheet.rs:~488-503`). The one-line test differs at larger text: when `ui.text > 1.0`, `wide / ui.text <= w - 2*BUTTON_PAD`; else `wide + BUTTON_PAD <= w` (`faraday/faraday-ui/src/sheet.rs:402-406`). `buttons_by` decides stacking by `rule` words so a layout holds when label text changes. Inside a sheet the row is deferred to the pinned foot.
- Button height follows text size (`ui.button_height(BUTTON_H)`, `ui.line_as(LABEL_L)`, `faraday/faraday-ui/src/sheet.rs:~322`, `faraday/faraday-ui/src/sheet.rs:~385`, `faraday/faraday-ui/src/sheet.rs:~399`, `faraday/faraday-ui/src/sheet.rs:~411`).
- Key-value rows: key BODY_S, value BODY_M, OutlineVariant rule, gaps GAP_SMALL and GAP (`faraday/faraday-ui/src/sheet.rs:~363-377`).
- Headline role `tokens::sheet_head(ui.compact)` (TITLE_L compact, else HEADLINE_S), icon `ICON` with `ICON_GLYPH_SMALL` (`faraday/faraday-ui/src/sheet.rs:~351-360`).
- A pinned button keeps its whole touch target however the body scrolls: the button row is padded outside the scroll (history, 046a47e7f; a test at `faraday/faraday-ui/src/sheet.rs:~545-598` asserts it).

Snackbar:
- Width: below `WINDOW_MEDIUM` the window width less `2*TOAST_MARGIN`; wider, clamped to `TOAST_MIN_W..TOAST_MAX_W` (`faraday/faraday-ui/src/snackbar.rs:~122-127`); height at least `TOAST_H` (`faraday/faraday-ui/src/snackbar.rs:~138`); text padding `TOAST_PAD` (`faraday/faraday-ui/src/snackbar.rs:~71`, `faraday/faraday-ui/src/snackbar.rs:~84`); stack gap `TOAST_STACK_GAP` (`faraday/faraday-ui/src/snackbar.rs:~116`, `faraday/faraday-ui/src/snackbar.rs:~146`, `faraday/faraday-ui/src/snackbar.rs:~169`).
- Shadow strength is `shown` to the sixth power, so the shadow ring does not show through a thinning fill (`faraday/faraday-ui/src/snackbar.rs:78`, reason at `faraday/faraday-ui/src/snackbar.rs:~75-77`); fill and ink fade by `shown` (`faraday/faraday-ui/src/snackbar.rs:~68`, `faraday/faraday-ui/src/snackbar.rs:~79`, `faraday/faraday-ui/src/snackbar.rs:~85`); nothing is drawn when `shown <= 0`.
- A word longer than the line stands alone (`snackbar_line`, `faraday/faraday-ui/src/snackbar.rs:176`).
- A snackbar lifts `TOAST_STACK_GAP` above a FAB it overlaps (`faraday/faraday-ui/src/snackbar.rs:~160-170`).
- `TOAST_SHOW_MS` 4000 is the token for how long a toast shows (`core/faraday-draw/src/tokens.rs:2176`).

Tooltip:
- Delay `TIP_LONG_PRESS_MS` for touch, `TIP_DELAY_MS` otherwise (500 ms each, `faraday/faraday-ui/src/tooltip.rs:~273-277`).
- Plain is centred on its anchor, rich starts at the anchor's x; clamped inside the canvas with a PAD margin (`faraday/faraday-ui/src/tooltip.rs:~352`, `faraday/faraday-ui/src/tooltip.rs:~371-378`); above with `TIP_GAP`, or below when there is no room (`faraday/faraday-ui/src/tooltip.rs:~379-384`); plain min height `TIP_PLAIN_MIN_H` (`faraday/faraday-ui/src/tooltip.rs:~365`); rich shadow `TIP_RICH_LEVEL` with `SHAPE_M` (`faraday/faraday-ui/src/tooltip.rs:~393`); plain `SHAPE_XS` (`faraday/faraday-ui/src/tooltip.rs:~387`).
- Palette: plain InverseSurface/InverseOnSurface (`faraday/faraday-ui/src/tooltip.rs:~30-32`); rich SurfaceContainer/OnSurfaceVariant (`faraday/faraday-ui/src/tooltip.rs:~34-36`); rich edge Outline at High contrast only (`faraday/faraday-ui/src/tooltip.rs:~37-38`, `faraday/faraday-ui/src/tooltip.rs:~395-397`).
- An anchor is clipped to the seen clip and dropped if empty (`tip_anchor`, `faraday/faraday-ui/src/tooltip.rs:~290`).

Stick pill:
- Radius `h/2`, dot at `pad`, label LABEL_M at `dot_room` (`faraday/faraday-ui/src/stick.rs:~88-98`).
- Inserted: filled in the stick tone with on-stick ink; Asked: OnSurface text, tone edge, SurfaceContainerHighest body; Quiet: OnSurfaceVariant text, OutlineVariant edge, SurfaceContainerHighest body; the dot is always the tone; nothing see-through (`faraday/faraday-ui/src/stick.rs:~5-7`, `faraday/faraday-ui/src/stick.rs:~48-72`).
The stick tones' ink contrast holds by computation: white on the light Standard and Medium tones 5.67:1 or more, white on the light High tones 9.08:1 or more, black on the dark High tones 8.82:1 or more; each tone stands out 3:1 or more from every surface in every scheme (faraday/faraday-ui/src/pal/mod.rs:175-213, faraday/faraday-ui/src/read_tests.rs:988-1012).

## Domain calls

- `faraday_draw::tokens` (`MENU_*`, `DIALOG_*`, `TOAST_*`, `TIP_*`, `SIDE_SHEET_PAD`, `ELEVATION_MENU`, `ELEVATION_SHEET`, `ELEVATION_DIALOG`, `ELEVATION_FAB`, `SHAPE_XS`, `SHAPE_M`, `SHAPE_XL`, `sheet_head`, `WINDOW_MEDIUM`, `STEP_*`) in `core/faraday-draw/src/tokens.rs`.
- `Canvas::fill_cells` for the QR (`core/faraday-draw/src/canvas.rs:~462`), `Canvas::elevation` for shadows (`core/faraday-draw/src/canvas.rs:~532`).
- `faraday/faraday-ui/src/pal/mod.rs` `stick`, `on_stick`, `stick_in` for the stick pill.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| c1798e8b5 | On the 7-inch panel a scrolling sheet put a field (Options' Rounds) under the on-screen keyboard: the sheet's measuring pass joined the box the keyboard keeps in view. | A scrolling sheet's field is measured where it is drawn: `framed` restores `focus_box` after its measuring pass (`faraday/faraday-ui/src/sheet.rs:71-74`). |
| 046a47e7f | A pinned button of a dialog or sheet lost part of its touch target at the scroll edge (Cancel 79 px under the 86 px floor at 480 wide, 40 of 43 at 240 x 320), because the bottom pad was inside the scroll view. | The button row is held outside the scroll with a pad above and below, the divider at the scroll's edge; unscrolled, the sheet is laid out as before (`faraday/faraday-ui/src/sheet.rs:92-98,121-145`). |

## Doubts

- Callers were not traced past the component crates (faraday-ui and faraday-draw); the Reach section rests on the component code and a few searches, not on a read of every screen.
- `Tips` method bodies and how `TipFrame` and `Tips` hand over between frames were not traced beyond the field.
