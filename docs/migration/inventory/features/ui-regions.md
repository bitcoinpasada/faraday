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

- Callers were not traced past the component crates (faraday-ui and faraday-draw); the Reach section rests on the component code and a few searches, not on a read of every screen.
