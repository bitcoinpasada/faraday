# Scrolling, motion and appearance

**Status: approved by the owner 2026-10-06; built 2026-10-07, waiting
for the owner's check on the Dell.** The progress list at the end says
what was done and how; §6 lists what was measured after.

The owner's ask: scrolling should feel like any modern native app, the
interface should gain the shadows, fades and transparency a modern app
has, and Settings gains a light mode. Faraday draws every pixel in
software (`osk-ui`'s tiny-skia canvas) and the stick writes them to the
firmware framebuffer, which has no vsync; everything below is designed
for that.

## 1. Owner's decisions (2026-10-06)

1. **Momentum lives in the core.** The shell says when the fingers that
   were scrolling lift (`Event::ScrollEnd`), and the core coasts. One
   implementation for the stick, the desktop window and a touchscreen.
2. **Hover is added to the shell API** (`Event::Hover`, `Event::HoverEnd`).
3. **Rubber-band at the ends** for touchpad, touchscreen and momentum.
   The wheel and keys stop hard at the ends.
4. **Light mode**, chosen in Settings (Appearance: Dark, Light).

## 2. What was wrong (measured 2026-10-06)

Measured with a harness outside the repo on an i7-7560U, release
profile, driving `Faraday` through `App::event` and `App::frame`.

| Per scroll frame | 1366×768 | 1920×1080 |
|---|---|---|
| Backup (a step column) render | 6 ms (max 10) | 11 ms (max 19) |
| Files, Start render | 1–1.5 ms | 2–3 ms |
| RGBA → panel format convert | 1.7 ms | 3.3 ms |

Primitive costs on a 1920×1080 canvas, six 1130×120 shapes per frame:

| Primitive | Unclipped | Inside a clip |
|---|---|---|
| `fill_rounded_rect` | 0.05 ms each | 1.6 ms each |
| `stroke_rounded_rect` | 0.09 ms each | 0.19 ms each |
| `fill_rect`, text | no difference | no difference |

Causes, in order of how much they are felt:

1. **The wheel jumps.** A notch is an instant 48 px move
   (`faraday/shells/stick/src/cursor.rs`, `WHEEL_NOTCH_PX`). A hi-res
   wheel is quantised to whole notches (`pointer.rs`, `HI_RES_PER_NOTCH`)
   and a touchpad pan loses its rounding remainder each pass.
2. **Frames are slow and unevenly paced.** A rounded rectangle drawn
   inside a clip goes through a panel-sized tiny-skia `Mask`
   (`core/osk-ui/src/canvas.rs`, `fill_rounded_rect`), about 30 times
   slower than without one, and every card, button and pill on a
   scrolled page is drawn inside the scroll region's clip. The stick
   shell draws whenever input arrives, so frames land at irregular
   intervals with irregular steps, and it ticks every 50 ms when no
   finger is down (`main.rs`, `TICK`), so an animation would run at
   20 fps.
3. **No momentum, no edge feedback, no touchscreen drag.** A two-finger
   flick stops dead when the fingers lift; nothing shows the end of a
   page; and a finger dragged on a touchscreen does not scroll Faraday
   at all (`Faraday::touch`, `TouchPhase::Move` only drives the Visit
   scrollbar).
4. **Items jostle while scrolling.** Scroll offsets are fractional
   design units and every rectangle and string rounds its own position
   to pixels (`ui.rs`, `Ui::px`), so items move by a pixel at different
   moments instead of as one sheet.
5. **Tearing.** The firmware framebuffer has no vsync. It cannot be
   removed without a GPU driver in the kernel, which the image does not
   carry on purpose; small, even steps make a tear a few pixels instead
   of 48, and writing less of the panel per frame narrows the window.

## 3. Design

### 3.1 Shell API (`core/osk-shell-api`)

New variants only; `Event::Scroll` keeps its meaning (move the content
by `dy` pixels now), so upstream's cores and shells are unaffected
beyond ignoring the new events.

- `Event::Wheel { x, y, dy }`: a wheel turned. `dy` is pixels the
  content should end up moved, positive up; the core glides there. The
  stick shell sends `notches × 48` and passes hi-res wheel fractions as
  fractional notches (carrying the sub-pixel remainder).
- `Event::ScrollEnd { x, y }`: the gesture that was sending
  `Event::Scroll` ended (two fingers lifted from a touchpad). The core
  coasts with the velocity the gesture had.
- `Event::Hover { x, y }`: the pointer moved with no button down.
- `Event::HoverEnd`: there is no pointer on the panel any more (the
  arrow hid because a finger touched the screen, or the window lost the
  pointer).

The stick shell (`faraday/shells/stick`) sends all four; the desktop
shell sends `Wheel` for `LineDelta`, `Scroll` for `PixelDelta`,
`ScrollEnd` for a touch phase of `Ended`, and `Hover`/`HoverEnd` from
`CursorMoved`/`CursorLeft`. The snapshot shell needs nothing.

### 3.2 Frame pacing (the shells)

- While anything moves the shell draws on a steady 60 Hz grid: a frame
  deadline every 16.67 ms from a fixed epoch, not "16 ms after the last
  pass". The core asks for a frame on every `Tick` while it animates, so
  "the last pass drew" is the shell's signal to keep the grid; a pass
  that drew nothing returns to the 50 ms idle tick.
- Input that arrives between deadlines is collected and goes in at the
  deadline, so each frame carries one even step. A tap is not delayed by
  more than one frame.
- Animation positions are computed from `now_ms`, so a slow frame does
  not slow the motion, it only skips a step.
- The pan's sub-pixel remainder is carried (`cursor.rs`).

### 3.3 Scroll physics (`faraday-core/src/motion.rs`)

One `Motion` state in `Faraday`. `Faraday::scroll_slot()` returns the
active region's offset (the Learn or word-list sheet's, or the
screen's), and `Faraday::with_region` hands it to the physics with its
extent; a motion left from another screen or sheet is stopped.

- **Pixel-rigid offsets.** `Motion` keeps the exact offset and writes
  the region's offset rounded to whole device pixels, carrying the
  remainder. Every item is then translated by the same whole number of
  pixels and the page moves as one sheet.
- **Pan (`Scroll`)**: moves 1:1 at once; the velocity is tracked as an
  exponentially weighted average of `dy / dt` between ticks (about a
  50 ms window).
- **Coast (`ScrollEnd`, touchscreen lift)**: velocity decays as
  `v ← v · e^(−dt/τ)`, τ = 325 ms (Android's and iOS's "normal" feel
  sit between 300 and 500 ms), and stops below 0.02 px/ms. A coast
  is cancelled by any new touch, pan or wheel.
- **Wheel**: each `Wheel` adds to a remaining distance; each tick moves
  `remaining · (1 − e^(−dt/τ))`, τ = 45 ms, so a notch settles in about
  140 ms, and notches that arrive while it glides add up and speed it.
  Keys (Up, Down) use the same glide.
- **Rubber band**: a pan or coast that pushes past an end stretches
  instead: the displayed overscroll for a pushed distance `x` is
  `(1 − 1 / (x · 0.55 / d + 1)) · d` with `d` the viewport height (the
  iOS curve). On release, or when a coast reaches the end, the
  overscroll returns with a critically damped spring of about 300 ms
  (a coast's remaining velocity is carried into the stretch first).
  The wheel and keys clamp and never stretch.
  - Ends come from the last frame: each scrolled screen and sheet calls
    `Ui::report_scroll(view, max)` once its content is drawn, and
    `Faraday::extent` keeps that, with the screen and sheet it was drawn
    for. A move is clamped to it, and what a pan pushes past it becomes
    stretch.
  - The stretch is drawn inside `report_scroll`: the viewport's pixels
    shift (`Canvas::shift`), the gap is filled with the page colour, and
    hits inside the viewport shift with them. Toasts, the ? button and
    sheets are drawn after, so they stay put.
- **Touchscreen drag**: a press that moves past the slop on a scrolled
  region becomes a pan (the press is cancelled, as now), and the lift is
  a `ScrollEnd`.
- **Overlay scrollbar**: a 3-unit thumb at the viewport's right edge,
  shown while the region moves and fading out over 300 ms after 800 ms
  of rest. Regions that do not overflow show none.

### 3.4 Render cost (`core/osk-ui/src/canvas.rs`)

- **Clipped shapes without a mask.** A shape whose bounds lie inside the
  clip is filled with no mask. A shape whose bounds lie inside the clip's
  columns but cross its top or bottom edge, the scroll case, is filled
  into the band of rows the clip covers, which is a contiguous slice of
  the pixmap (`PixmapMut::from_bytes` over those rows), with no mask.
  Only a shape crossing a side of the clip still takes the mask. Applies
  to every path primitive (rounded rect fill and stroke, circle, line,
  arc).
- **No second frame for a clamp.** A column that finds its offset past
  the end clamps and draws again at once (`flow::column_foot` returns
  `again`); with `Motion` doing the clamp readback, the clamped frame is
  the next animation frame instead of a second full render.
- Measured again after these; only if a column page is still over 4 ms
  at 1920×1080: skip cards wholly outside the viewport, and cache an open
  card's measured height instead of measuring it by drawing it.
- Later, for slow boards only: scroll by copying the rows that stay
  (`Canvas::shift`, as upstream does) and redraw the exposed strip, and
  convert and write only the rows that changed.

### 3.5 Visual polish

- **Hover**: rows, buttons, the sidebar items, the step cards' headers
  and the ? button gain a hover look (a fill of `INNER` at 60 %, a
  secondary button's edge one step brighter). `Ui::hovered` is found from
  the last frame's hits, like `pressed`; a frame is drawn only when the
  hovered action changes.
- **Motion** (all time-based, all under a reduce-motion setting later):
  - a sheet and its scrim appear at once (cross-fades removed
    2026-10-07, §5 item 3.9);
  - a toast rises and fades in over 160 ms;
  - a step card opens with its height growing over 180 ms (ease-out
    cubic), and the jump that brings the open card into view
    (`Scroll::follow`) is a glide;
  - changing screen shows the new one at once;
  - the Guided / Steps only switch's pill slides between its options.
- **Depth**: soft shadows only under what floats (sheets, toasts, the
  network menu), pre-blurred once per radius and blur and drawn as nine
  pieces, so a shadow costs a few blits. Cards keep a flat surface with
  a 1-unit edge.
- **Scroll edges**: when a column is scrolled away from its top, a
  hairline and a short shadow fade in under the page's top edge.
- **Frosted sheets** (last, optional): when a sheet opens, the page
  under it is blurred once (downscaled, box-blurred, upscaled) and kept
  until the sheet closes; each frame copies it instead of the scrim.

### 3.6 Light mode

- `Faraday::theme` (`Theme::Dark`, `Theme::Light`), chosen on Settings
  under **Appearance**. Kept across a lock with the other settings
  (`theme=` in the kept `settings`, `memory.rs`), until power-off.
- The screens keep drawing with `ui::pal`'s names; `Ui` maps each colour
  to the active theme at the draw boundary (`Ui::fill`, `stroke`, `text`,
  `wrap`, `icon`, `dot`, `rule`, and the frame's clear), matching on the
  colour's RGB and keeping its alpha, so `ACCENT.with_alpha(140)` becomes
  the light accent at the same alpha. Colours outside the palette (a
  QR's black and white, the camera image) pass unchanged. The one
  `mix` (`Ui::button`'s pressed primary) maps before it mixes.
  `ON_ACCENT` moves one step (`#0d1219`) so it no longer shares
  `SIDEBAR`'s RGB.
- The light palette:

| Name | Dark | Light |
|---|---|---|
| BG | `#11161c` | `#f4f6f8` |
| SIDEBAR | `#0d1218` | `#e9edf1` |
| SURFACE | `#19212a` | `#ffffff` |
| LINE | `#1f2933` | `#dce2e8` |
| INNER | `#222c37` | `#e8edf1` |
| BORDER | `#303d49` | `#c6d0d8` |
| TEXT | `#e0e9ef` | `#151c23` |
| MUTED | `#9aacb9` | `#4f5d69` |
| DIM | `#6f8291` | `#7a8894` |
| ACCENT | `#83d8ef` | `#0b7a9e` |
| ON_ACCENT | `#0d1219` | `#ffffff` |
| OK | `#8bd4b2` | `#1d8556` |
| WARN | `#f0c077` | `#9a5d00` |
| ERR | `#ef9b9b` | `#bf3434` |

### 3.7 Named themes (2026-10-07)

Five more palettes beside Dark and Light, each a published theme mapped
onto the same fourteen names (`ui::pal::NORD`, `CATPPUCCIN`,
`TOKYO_NIGHT`, `GRUVBOX`, `ROSE_PINE`): Nord, Catppuccin Mocha, Tokyo
Night, Gruvbox dark and Rosé Pine Dawn. Where a theme's own red, gold or
green reads under 4.5:1 on its cards it is lightened or darkened until it
does (Nord's and Gruvbox's red, Rosé Pine's love, gold and a green it
lacks). Rosé Pine and Light are light themes (`Theme::is_light`), with
the lighter shadow. Settings › Appearance shows each as a tile drawn in
its own colours (`Ui::theme_tile`); the theme is kept as `theme=<id>`.
`faraday-snapshot WxH KIT OUT themes` renders three screens in each.
Tokyo Night is the theme a first start is in, on every shell (owner,
2026-10-08; it was Nord from 2026-10-07; `Theme`'s default).
Lamplight (2026-10-09, `ui::pal::LAMPLIGHT`) is Faraday's own light
theme: warm cream page, espresso text, a terracotta accent (#A34E26,
5.2:1 on the page) and a sage, amber and brick at 4.5:1 or more.

### 3.8 The caret and the scrollbar (2026-10-07)

- The caret of the field typing goes to blinks: on 530 ms, off 530 ms
  (`motion::CARET_HALF_MS`), back on at every key or press; steady with
  reduce motion. `Ui::caret` and `Ui::caret_char` draw it; a frame is
  asked for only when a caret was drawn and its half changes.
- The overlay scrollbar is 6 units wide and can be held: a press within
  16 units of a scrolled region's right edge (`ui::BAR_GRAB`) takes it,
  on the thumb from where it was taken, elsewhere on the track with the
  thumb's middle brought to the press, and the page follows the pointer
  or finger until it lifts. With the pointer over that edge the bar
  shows in full. `ui::BarGeometry` is where both drawing and holding
  find it.
- Every scrolled region uses it (2026-10-08): the word-list sheet and
  the Learn sheet, which drew a thin bar of their own that could not be
  held, now report their region with `Ui::report_scroll` like a page.
  The stick visit's file list keeps its own bar, which is held and
  dragged by itself (`Action::VisitBar`).

## 4. Tests

User-facing behaviour only (`CLAUDE.md`):

- `faraday-core/tests/scroll.rs`, rewritten to the new rules: a wheel
  notch glides and ends exactly 48 pixels on; notches in quick
  succession add up; a pan moves at once; a pan then `ScrollEnd` keeps
  moving and comes to rest; a pan past the top stretches and springs
  back to the top; the wheel at an end does not stretch; offsets are
  whole pixels after any of these; a touchscreen drag scrolls a page.
- `faraday-core/tests/hover.rs`: hovering a button changes its pixels
  and `HoverEnd` restores them; hovering does not press.
- `faraday-core/tests/appearance.rs`: Settings › Light turns the page's
  background light and Dark turns it back; a QR stays black on white.
- `osk-ui` canvas: a rounded card cut by a scroll region's top and
  bottom draws the same pixels on the fast path as through the mask.
- Stick shell: two fingers lifting send `ScrollEnd`; a hi-res wheel's
  half notch scrolls half a notch; the pan carries its remainder.

## 5. Progress

Tick an item when it is built and its tests pass; note anything a later
session must know beside it.

Phase 1 — scrolling that is smooth with no change in feel elsewhere:

- [x] 1.1 Canvas: clipped path primitives without a mask (§3.4), with
      the pixel-equality test (`canvas::draw_path`). Backup at
      1920×1080: 11 ms → 3.9 ms a frame.
- [x] 1.2 Shell API: `Wheel`, `ScrollEnd`, `Hover`, `HoverEnd` (§3.1);
      upstream's `osk-ui` takes `Wheel` as `Scroll` and ignores the rest.
- [x] 1.3 `motion.rs`: `scroll_slot()`, pixel-rigid offsets, pan, wheel
      glide, keys through the glide, ticks while moving (§3.3). Each
      scrolled screen reports its viewport and extent with
      `Ui::report_scroll` once its content is drawn, which also fixed
      Files, Visit and Wallets letting the offset run on past the end.
- [x] 1.4 Stick shell: 60 Hz frame grid while drawing, input collected
      to the deadline, `Wheel` with hi-res fractions, pan remainder,
      `ScrollEnd`, `Hover`/`HoverEnd` (§3.1, §3.2).
- [x] 1.5 Desktop shell: the same events from winit, 16 ms ticks while
      the core draws on ticks. A touchpad under X11 reports lines and so
      glides like a wheel; under Wayland it pans and coasts.
- [x] 1.6 Remove the second render on a clamp (§3.4); measure again.
      Moves clamp to the last frame's extent, so no frame is drawn past
      the end. Backup at 1920×1080 is 4.5 ms a frame through the whole
      path; culling was not needed.

Phase 2 — touch physics:

- [x] 2.1 Coast after `ScrollEnd` (§3.3). A touch on content that is
      still flying only stops it.
- [x] 2.2 Rubber band at both ends, viewport shift (§3.3), drawn in
      `Ui::report_scroll`.
- [x] 2.3 Touchscreen drag scrolling with coast (`Faraday::drag`). A
      mouse drag on a page scrolls it too, since the shells send a mouse
      as a touch.
- [x] 2.4 Overlay scrollbar (`Ui::scroll_bar`), not on regions that
      draw their own (`report_scroll_own_bar`: Visit, Learn, word list).
      A scroll that stops with no `ScrollEnd` (a shell that never says)
      ends after 300 ms, so a stretch cannot stay.

Phase 3 — appearance:

- [x] 3.1 Light mode (§3.6): `ui::Theme`, `pal::LIGHT`, Settings ›
      Appearance. Settings now scrolls, being taller than 800 units.
- [x] 3.2 Hover looks (§3.5): buttons draw their own; every other hit
      area gets a faint wash from `Ui::hit`. A frame only when what is
      under the pointer changes.
- [x] 3.3 Sheet, toast and scrim motion: a toast rises in and fades
      out. The cross-fade on a change of screen or sheet, and a sheet's
      rise, were removed by item 3.9.
      `Faraday::settle` brings everything to rest for the snapshot tool,
      whose pictures match the last commit's but for 1 px snapping.
- [x] 3.4 Card disclosure height animation and gliding follow: the
      column reports its open card (`Ui::column`); a change is drawn
      again at once growing from closed, the card closing shrinks, and
      hits keep their full size while a body is uncovered
      (`Ui::reveal`). The follow glides (`Ui::follow_to`). Motion belongs
      to the scrolling region (`Faraday::region_key`), so a sheet that
      does not scroll leaves what moves under it moving.
- [x] 3.5 Guided switch slide. The page and a theme change no longer
      cross-fade (item 3.9).
- [x] 3.6 Shadows under floating things (`Canvas::shadow`, worked out
      per pixel from the distance to the shape, so nothing is cached):
      sheets, the toast, the stick banner. Instead of a hairline, the
      scrolled content fades into the page at an edge it continues past
      (`Ui::edge_fades`).
- [x] 3.7 Frosted sheets: the page under a sheet is reduced eight units
      to a pixel, blurred, tinted with the dim and enlarged once
      (`Canvas::frost`), kept (`Faraday::frost`) and copied each frame
      after (`Canvas::restore`); the page is not drawn again while the
      copy stands for it.
- [x] 3.8 Reduce motion in Settings (Motion: Full, Reduced), kept
      across a lock: nothing glides, coasts, stretches, grows, rises or
      slides.
- [x] 3.9 No cross-fades (owner, 2026-10-07: they did not look smooth
      enough). A change of screen, sheet, Guided or theme is drawn at
      once; a sheet no longer rises. `Canvas::blend_from` is left in
      `osk-ui`, unused here.

## 6. After (measured 2026-10-07)

Same harness and machine as §2, release profile.

| Per frame | 1366×768 | 1920×1080 |
|---|---|---|
| Backup (a step column), scrolling | 2.2 ms | 3.9–4.5 ms |
| Files, Start, scrolling | 1.2–1.6 ms | 2.1–3.0 ms |
| A sheet open (frosted, kept) | 2.5 ms | 4.5 ms |
| The frame a sheet opens on (frosting it) | 21 ms | 40 ms |

- A translucent `fill_rect` over the whole panel took 20 ms at 1080p in
  tiny-skia; a sheet's dim paid it every frame before this work. It is
  blended in `Canvas::fill_rect` now (6 ms), and the dim itself is baked
  into the kept frosted copy, so a sheet's frames pay neither.
- `osk-ui` at opt-level 3 in release changed none of these numbers, as
  upstream found (`Cargo.toml`), so the profile is as it was.
- The frame a sheet opens on costs two frames at 1080p, once.
  Frosting by a coarser reduction would be cheaper still.
- Not measured here: the framebuffer write on the stick (`--timings`
  on the Dell says how long a frame's `pwrite` takes).

## 7. Notes for whoever changes this next

- Every scrolled screen calls `Ui::report_scroll` (or
  `report_scroll_own_bar`) once its content is drawn; a new scrolled
  screen must too, or it has no ends, no stretch, no fades and no bar,
  and must be added to `Faraday::scroll_slot`.
- A driver that takes pictures of the screens calls `Faraday::settle`
  first (the snapshot tool does), and draws a frame after a press before
  scrolling, as a shell does: a scroll stops where the last frame said
  the page ends.
- Tests that read pixels after a change tick at 16 ms first, as a shell
  does (`tests/transitions.rs`, `tests/scroll.rs`).
- Settings scrolls now: it outgrew 800 units with Appearance and Motion.

