# Layout rules

The layout rules the component layer (`faraday-ui`) and the drawing base
(`core/faraday-draw`) enforce in code for every screen of every app: the
touch floor, focus order, the dimmed-while-busy rule, High contrast,
text-size growth, clipping, fit and wrap, and the per-component placement
rules. They are not a screen; each rule is applied by the component that
draws. This file gathers them across components; each component's own file
(`ui-press`, `ui-controls`, `ui-text`, `ui-sheets`, `ui-navigation`,
`ui-regions`, `keyboard`, `draw-base`) has the full list.

## Screens and sheets

None.

## Actions

None of its own.

## Effects

None.

## State and lock

None. No secrets.

## Reach

- Every screen of Faraday Offline, Faraday Demo and the Starter, through the components.
- Lints that hold related rules (`lint-tokens`, `lint-components`, `lint-touch-targets` with `tools/touch-targets.counts`) are outside this code.

## Rules and refusals

Touch and press:
- Touch floor: a press target is at least `Ui.touch` px (`TOUCH` 48 dp, or 44 on a Compact short window) inside its clip (`faraday/faraday-ui/src/controls.rs:~239-268`; `core/faraday-draw/src/tokens.rs:215`).
- Overlapping targets share the gap at its middle (`faraday/faraday-ui/src/controls.rs:~80-115`, `faraday/faraday-ui/src/controls.rs:~252-259`).
- A target never leaves its clip except as it is cut; a cut press is recorded (`faraday/faraday-ui/src/controls.rs:~249-250`; `faraday/faraday-ui/src/lib.rs:~1948-1952`).
- A state layer is drawn only on controls under a quarter of the screen (`faraday/faraday-ui/src/lib.rs:~1959-1961`).
- Button rows are at least a touch target apart (`faraday/faraday-ui/src/parts.rs:~74`, `faraday/faraday-ui/src/parts.rs:~133`, `faraday/faraday-ui/src/parts.rs:~163`).
- A pinned sheet button keeps its whole touch target however the body scrolls (`faraday/faraday-ui/src/sheet.rs`, history 046a47e7f).
- On the compact small panel the FAB and a field action take the touch-floor height as target (`faraday/faraday-ui/src/lib.rs:~2368-2371`, `faraday/faraday-ui/src/lib.rs:~2394-2396`).
Focus:
- Tab goes through controls in the order drawn and wraps (`faraday/faraday-ui/src/controls.rs:~46-65`).
- One focus ring per control, `FOCUS_RING` thick, `FOCUS_OFFSET` outside, in Secondary; inside the item bounds in a menu (`faraday/faraday-ui/src/controls.rs:~300-310`; `faraday/faraday-ui/src/menu.rs:~216-227`).
Busy and disabled:
- While a job runs, `keep_live` closes every action not kept live; dimmed actions draw faded and take no press unless the app answers them (F292) (`faraday/faraday-ui/src/lib.rs:~1912-1936`).
- An off button style takes no press (`faraday/faraday-ui/src/lib.rs:2300`); a disabled segment, disabled icon button, disabled list row, Off menu item and dimmed hold take no press (`faraday/faraday-ui/src/controls.rs:~426-428`, `faraday/faraday-ui/src/controls.rs:~571-584`; `faraday/faraday-ui/src/lists.rs:~97-99`; `faraday/faraday-ui/src/menu.rs:~188-196`; `faraday/faraday-ui/src/hold.rs:~101-112`).
High contrast:
- No shadows and no edge fades; an Outline stroke in place of the shadow on menus, sheets, dialogs and rich tooltips; scroll bar at full alpha; tonal and selected buttons outlined; see-through role fills become full-strength edges (`faraday/faraday-ui/src/lib.rs:~785-788`, `faraday/faraday-ui/src/lib.rs:~861`, `faraday/faraday-ui/src/lib.rs:~940`, `faraday/faraday-ui/src/lib.rs:~955`, `faraday/faraday-ui/src/lib.rs:~1218-1220`, `faraday/faraday-ui/src/lib.rs:~2284-2286`; `faraday/faraday-ui/src/controls.rs:~206-221`; `faraday/faraday-ui/src/menu.rs:~295-297`; `faraday/faraday-ui/src/sheet.rs:~115-117`; `faraday/faraday-ui/src/tooltip.rs:~395-397`).
Text size:
- Rows, buttons, fields and list items grow with larger text; at Standard they keep the token height (`faraday/faraday-ui/src/roles.rs:~150-168`, `faraday/faraday-ui/src/roles.rs:~964-978`).
- A one-line text steps down by 10% to fit and never below Standard, then is cut with an ellipsis (`faraday/faraday-ui/src/roles.rs:~181-228`).
- Keyboard keys and labels keep their size (`fixed_text`, `faraday/faraday-ui/src/lib.rs:~2183-2185`, `faraday/faraday-ui/src/lib.rs:~2203-2205`).
- `Window::of` divides the window's dp by the text factor (at least 1), so larger text gives a narrower window class (`core/faraday-draw/src/geom.rs:175`).
Wrap and fit:
- A line measured to fit does not wrap when drawn (rounded pixel widths, `core/faraday-draw/src/text.rs:~189-191`).
- Addresses break only between groups of four (`faraday/faraday-ui/src/roles.rs:~630-657`).
- A hold label wraps and the hold grows (`faraday/faraday-ui/src/hold.rs:~53-64`); a tab wider than the row wraps (`faraday/faraday-ui/src/tabs.rs:~74-86`); stacked button labels wrap at word breaks (`faraday/faraday-ui/src/sheet.rs:~398-413`).
- Segment widths are constant so a choice moving changes no width (`faraday/faraday-ui/src/controls.rs:~355-368`); choices fall back from a segmented button to wrapped buttons (`faraday/faraday-ui/src/controls.rs:~500-531`).
- The wordmark is beside, else stacked and shrunk to `WORDMARK_CAP_MIN`, else dropped (`faraday/faraday-ui/src/lib.rs:~1412-1439`).
Placement:
- Menus: popup on wide, sheet on compact; placed below the anchor, else above, else clamped (`faraday/faraday-ui/src/menu.rs:~248-281`).
- Dialogs: compact take the panel width and share action widths; wide clamp to `DIALOG_MIN..DIALOG_MAX` and end-align actions (`faraday/faraday-ui/src/sheet.rs:~285-289`).
- Snackbar: full width under `WINDOW_MEDIUM`, else clamped; lifts above an overlapping FAB (`faraday/faraday-ui/src/snackbar.rs:~122-127`, `faraday/faraday-ui/src/snackbar.rs:~160-170`).
- Tooltip: above the anchor, else below; clamped inside the canvas (`faraday/faraday-ui/src/tooltip.rs:~371-384`).
- Tabs: equal share, else natural widths, else whole tabs with chevrons (`faraday/faraday-ui/src/tabs.rs:67-72`).
- Keyboard panel width capped at 720 dp and centred on wide (`faraday/faraday-ui/src/keyboard.rs:52`, `faraday/faraday-ui/src/keyboard.rs:~171-172`).
- Page content's scroll max includes room for a floating button, outside sheets (`faraday/faraday-ui/src/region.rs:~443-448`).
- A focused field is noted so the page scrolls it above the on-screen keyboard (`faraday/faraday-ui/src/lib.rs:~1453-1462`; `faraday/faraday-ui/src/roles.rs:~852-855`).
Display classes:
- Size class by diagonal: under 3.5 inches Small, under 7.0 Mobile, else Wide (`core/faraday-draw/src/geom.rs:126-136`); type ramp 100/110/130% by class; narrow faces on Small only (`core/faraday-draw/src/tokens.rs:~149-165`).
- Window classes at 600, 840, 1200, 1600 dp wide; short under 480 tall; low under 600 tall (`core/faraday-draw/src/tokens.rs:900`, `core/faraday-draw/src/tokens.rs:~903-915`).
Clip:
- Every primitive honours the clip; a shape crossing it is drawn whole and the outside restored (`core/faraday-draw/src/canvas/surface.rs:~419-454`); a text line wholly outside is skipped (`core/faraday-draw/src/canvas.rs:~1245-1250`).
- `measured` draws with a zero clip and drops its presses (`faraday/faraday-ui/src/parts.rs:~176-186`).

## Domain calls

- `core/faraday-draw/src/tokens.rs`, `core/faraday-draw/src/geom.rs`, `core/faraday-draw/src/text.rs` (see `tokens`, `draw-base`).

## Fixtures

None.

## Fixes

None given to this feature. Fixes that restored layout rules are listed under the components they touch (`ui-controls`, `ui-press`, `keyboard`).

## Doubts

- Callers were not traced past the component crates (faraday-ui and faraday-draw); the Reach section rests on the component code and a few searches, not on a read of every screen.
- Rules here are those the inventory read in the component and drawing code; layout rules enforced in faraday-core screens, the pane container and the menu page container are outside this cluster.
- Whether `choices` and `choices_h` agree was raised; they make the same segmented-or-wrapped decision from the same widths (`faraday/faraday-ui/src/controls.rs:496-552`).
