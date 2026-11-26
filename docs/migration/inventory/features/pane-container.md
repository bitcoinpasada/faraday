# The pane container

The pane container places a page's panes and times a change of page. A page or flow declares its panes; only `faraday/faraday-core/src/screens/hub_layout/` places them: the session pane and the page's tile (stacked under This session's card, or side by side), search results, row details, and the guide's pane at the right edge (`docs/HUB-LAYOUT.md`). It serves every wide page that has panes.

## Screens and sheets

- No screen of its own. Files: `faraday/faraday-core/src/screens/hub_layout/{mod,pair,page,lines,turn}.rs`.
- Places: the session pane and tile, search results, row details, the guide pane.
- Page turn (`faraday/faraday-core/src/screens/hub_layout/turn.rs`): Smooth = `Kind::Through`, Crossfade = `Kind::Crossfade`, Off = none; the duration comes from the Speed setting.
- Dividers between panes (`faraday/faraday-core/src/screens/hub_layout/lines.rs`).

## Actions

None of its own. The panes' own presses belong to their features.

## Effects

- None outward. Draw-time writes: `ui.spoiled` (`faraday/faraday-core/src/screens/hub_layout/lines.rs:~48-64`), `app.stood_apart()` (`faraday/faraday-core/src/screens/hub_layout/page.rs:~176`, body not read).

## State and lock

- Holds no state and no secrets. Every lock ends in a fresh process (`faraday/faraday-core/src/vault/seal.rs:933-946`).

## Reach

- Places the session pane (`session-pane`), search results and row details (`menu-pages`), the guide pane through `hub_layout::guide_place` (`guide`).
- Reads Settings' Motion and Speed (`settings-time`, `motion`).

## Rules and refusals

- The pane stands to the left only where the pair's narrowest fits; else the card narrows to `FLOW_CARD_MIN`, then the pane to `CHOICES_PANE_MIN` (`faraday/faraday-core/src/screens/hub_layout/pair.rs:~28-32`).
- Stacked under This session's card only when `stacks` and `under + tile_h + MARGIN_WIDE <= h` (`faraday/faraday-core/src/screens/hub_layout/mod.rs:137-140`).
- A tile fills only for two-column tiles (`faraday/faraday-core/src/screens/hub_layout/pair.rs:~56-57`).
- An equal-column pair is refused below its least width (`faraday/faraday-core/src/screens/hub_layout/pair.rs:~86-88`).
- A detail beside a list is refused where the room does not hold both (`faraday/faraday-core/src/screens/hub_layout/pair.rs:~109-111`, `faraday/faraday-core/src/screens/hub_layout/mod.rs:~190-192`).
- The guide pane needs `holds_guide` (`faraday/faraday-core/src/screens/hub_layout/mod.rs:~196-198`). Left of the guide, the pair moves only as needed, refused below `MARGIN_WIDE` (`faraday/faraday-core/src/screens/hub_layout/pair.rs:~73-80`).
- The side panel's width is clamped to `CHOICES_PANE_MIN..CHOICES_PANE_MAX` (`faraday/faraday-core/src/screens/hub_layout/mod.rs:~203-205`). Home's area is narrowed to `READING_MENU_W` (`faraday/faraday-core/src/screens/hub_layout/mod.rs:~209-215`).
- A divider is drawn only when the taller side exceeds `2 * PAD` (`faraday/faraday-core/src/screens/hub_layout/lines.rs:~78-90`).
- Motion Off gives no transition (`faraday/faraday-core/src/screens/hub_layout/turn.rs:25-31`).
- Since the history fixes: a side panel beside step columns is 31% of the content width, clamped 240 to 320; hub pages lay out to the window's width at the desktop's density, and a narrow window gets the one-column layout.

## Domain calls

None.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 1628a33c7 | The hub app's pages were laid out for a fixed 1280x800 design canvas and did not fit a window of another width; a narrow window got a broken wide layout. Demo's Transfer and Learn held in the hub had the same problem. | Hub pages lay out to the window's width at the desktop's density; a narrow window gets the one-column layout. The desktop app alone is unchanged. |
| 4dbfe8b0a | The 320-wide side panel beside step columns (Spend, Back up, Create a vault) left a narrow window's column little room. | The panel is 31% of the content width, clamped 240 to 320; a full-size window is unchanged. |

## Doubts

- `app.stood_apart()` (`faraday/faraday-core/src/screens/hub_layout/page.rs:~176`) writes during drawing; its body was not read.
- The tests in `faraday/faraday-core/src/screens/hub_layout/pair.rs` (`:~153-224`) were not read.
- 1628a33c7 names a hub crate that is not in the tree today (`faraday/faraday-hub`); the rule it restored is kept as stated.
