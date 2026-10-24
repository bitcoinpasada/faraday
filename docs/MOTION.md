# Scrolling, motion and appearance

**Kind:** Spec
**Status:** Built; not yet checked by the owner on hardware
**Open:** the owner's check on real hardware (the stick's panels and the desktop app's compositor-paced frames)

The build log is `docs/history/MOTION-LOG.md`.

The owner's ask: scrolling should feel like any modern native app, the
interface should gain the shadows, fades and transparency a modern app
has, and Settings gains a light mode. Faraday draws every pixel in
software (`faraday-draw`'s tiny-skia canvas) and the stick writes them to the
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

Moved to `docs/history/MOTION-LOG.md`.

## 3. Design

### 3.1 Shell API (`core/faraday-shell-api`)

New variants only; `Event::Scroll` keeps its meaning (move the content
by `dy` pixels now), so the other cores and shells are unaffected
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
- **The desktop window on Wayland is paced by the compositor**
  (2026-10-12). A timer at 60 Hz beside a screen at 60 Hz drifts in and
  out of phase with it: every few seconds a frame is shown twice or not
  at all, and a step of the scroll is doubled or missed, which reads as
  judder however even the steps are. So the shell tells winit before
  each present (`Window::pre_present_notify`), winit holds the next
  `RedrawRequested` until the compositor reports the frame on the
  screen, and the shell sends the `Tick` that moves the content on each
  redraw, just before drawing. Every frame is then computed for the
  refresh that shows it, at the screen's own rate whatever it is, and a
  tick that asks for a frame asks for the next redraw, so the loop runs
  exactly while the core wants frames. The 50 ms timer stays as the idle
  tick, and keeps time (the clock, the idle lock, the physics) while the
  window is hidden and the compositor sends no refreshes. Under X11,
  which delivers a redraw at once, the timer keeps the 60 Hz grid.
- **The grid's step follows what a frame costs** (2026-10-12; the stick,
  and the desktop under X11). A grid finer than the machine can keep
  gives one step of one frame and the next of two, and the content
  stutters however exact its positions. The shell keeps a smoothed cost
  of the passes that drew (each pass moves it a fifth of the way, so the
  40 ms frame a sheet opens on changes nothing) and steps by the fewest
  whole refreshes that hold it with a quarter's room: one at 60 Hz on a
  machine that draws in a few milliseconds, two (30 Hz) on one that
  takes twenty, up to four. A step is shortened again only when the
  shorter one holds the cost with 60 % room, so it does not flip at the
  boundary (`faraday/shells/stick/src/pace.rs`, with tests;
  `Shell::drew` in the desktop shell, which also takes the refresh
  period from the monitor the window is on). On Wayland the compositor
  settles the rate the same way by itself: a frame that misses a refresh
  is shown at the next, and the frame after is asked for from there.
- **On the PC stick a fade's steps keep their own step.** A step that
  only blends the two kept frames is paced by the median of what the
  last five such steps cost, not by the frames drawn in full: the frame
  that shows a change is often the slowest there is, and one step held
  up does not stretch the ones after it
  (`Pace::stepped_fade`, `faraday/shells/stick/src/pace.rs`). The Pi
  keeps one grid for both.
- **On the PC stick the CPU's clock is up while something moves.** The
  stick kernel's governor is schedutil, which raises a CPU's clock only
  once work has run a while; a fade starts from idle. On a PC whose
  CPUs run schedutil, init lets the app write each CPU group's
  `scaling_min_freq`, and the shell sets it to the highest clock at
  input and while frames are drawn, and back a second after the last
  (`faraday/shells/stick/src/clock.rs`). A CPU whose driver sets its own
  clock (intel_pstate with HWP, amd-pstate) runs that driver's
  powersave and is left alone, and so is the Pi.
- **A shape wholly outside the clip is not drawn** (`Canvas::draw_path`,
  2026-10-12). A page's cards are all drawn every frame, inside the
  scroll region's clip; one scrolled off the page used to be filled in
  full and the rows outside the region put back. Now it costs its layout
  and nothing more, on every machine, so a long page scrolls at the cost
  of the cards in view. The pixel-equality test in `canvas.rs` covers
  shapes wholly above, below and beside the region.
- **The stick's arrow moves at the mouse's own rate** (2026-10-12,
  owner: the mouse was jumpy, and still jumped when the main loop
  painted it as moves arrived, since the loop stalls for 5 to 15 ms
  whenever a pass renders, converts and writes a frame). A desktop's
  arrow is a hardware sprite that moves with every report from the
  mouse, whatever the program is doing; here the input thread that reads
  the mouse paints it. The framebuffer, a copy of the frame it shows
  without the arrow, and the arrow's place live in one `Panel` behind a
  lock (`faraday/shells/stick/src/panel.rs`), shared by the main loop
  and the input threads. A reader that decodes a move applies it to the
  shared cursor and calls `Panel::move_arrow`, which rewrites only the
  rows the arrow leaves (put back from the kept copy) and the rows it
  lands on, then sends the loop what the move meant (`Wake::Pointed`: a
  hover, a drag) for the frame's deadline, runs of them collapsed to the
  last. The loop writes a frame in bands of 64 rows, each under its own
  hold of the lock (`Panel::write_rows`, the changed rows of the band
  only, the arrow drawn in where it falls), so a move slips in between
  two bands and the arrow waits at most half a millisecond. A device
  held back until a person claims it moves the arrow through the loop
  until then, and from its own thread after (`evdev::spawn`'s believed
  flag). A finger on the touchscreen takes the arrow away at once. The
  panel's tests write to a file and read it back.
- **The stick writes only the rows that changed** since the last frame
  (`fb::changed_rows`, 2026-10-12): a caret, a toast or the fading bar
  is a strip, a scroll is the scrolled region without the bars around
  it. Fewer bytes cross to the framebuffer each frame, and the window in
  which the beam can catch a frame half written is shorter. The frame
  count and the 60 Hz grid still follow the core's draws, not the bytes.
- **The stick's cursor moves by the arrow's scale** (2026-10-16, F43;
  owner: the mouse was slow on a 3200×1800 screen and fine at 1920×1080
  and below). One
  mouse count moved the cursor one panel pixel, so a dense screen took
  up to three times the hand movement to cross. Now one count moves it
  by the arrow's own scale (`cursor::scale`, a whole number from the
  panel's short side: 1 below 1,800 lines, 2 at 1,800 to 2,999, 3 from
  3,000), times `--pointer-speed` as before: the cursor moves through
  the panel as a desktop's moves through its scaled pixels, and a
  3200×1800 panel crosses like a 1600×900 one. A touchpad's moves are
  scaled the same; the tap's travel limit (`TAP_TRAVEL_PX`) and a
  two-finger pan are measured in the same scaled steps, so a tap is as
  easy at any density. Still no acceleration. Built 2026-10-19. Tests
  (`cursor.rs`): 100 counts right move the cursor 100 pixels on a
  1920×1080 panel and 200 on a 3200×1800 one; the cursor stops at the
  panel's edge at either; a tap at 3200×1800 that travels 5 counts is
  still a tap.
- `faraday --timings` prints once a second what the frames cost on the
  machine it runs on: the render, the copy into the window, and the time
  from one frame to the next while something moved. A frame-to-frame
  mean near the screen's period with a worst case under twice it is a
  motion with no dropped frame.
- The dev stick's `--timings` (`faraday/image/README.md`) writes a
  block every five seconds to `faraday-timings.txt` on the boot stick:
  a frame drawn in full against a fade's blended step (`frame`,
  `blend`), how many frames went out at each step of the frame grid
  (`grid`), the first CPU's clock while drawing with its cpufreq driver
  and governor in the header (`clock`), and what fades did (`fade`: changes that faded, were drawn
  at once for slow steps or for reduce motion, the recent step gaps'
  median, and whether the next change fades; `Faraday::take_fade_counts`).

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
  Keys (Up, Down) use the same glide. Since 2026-10-12 the notch's
  distance is the layout's, not the screen's: the shells send 48 pixels
  a notch (`motion::SHELL_NOTCH_PX`), which the core reads as a count of
  notches and glides `motion::WHEEL_NOTCH` = 56 units each, the keys the
  same. Before, 48 device pixels a notch was 48 units on a 1× screen but
  24 on a scale-2 panel, half of a browser's notch (Chrome 53 CSS pixels,
  Firefox about 57); the owner asked for a shade over a browser's.
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

### 3.4 Render cost (`core/faraday-draw/src/canvas.rs`)

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
  (`Canvas::shift`) and redraw the exposed strip, and
  convert and write only the rows that changed.

### 3.5 Visual polish

- **Hover**: rows, buttons, the sidebar items, the step cards' headers
  and the ? button gain a hover look (a fill of `INNER` at 60 %, a
  secondary button's edge one step brighter). `Ui::hovered` is found from
  the last frame's hits, like `pressed`; a frame is drawn only when the
  hovered action changes.
- **Motion** (all time-based, all under a reduce-motion setting later):
  - a sheet and its scrim appear at once (cross-fades removed
    2026-10-07, `docs/history/MOTION-LOG.md` §5 item 3.9);
  - a toast rises in over 200 ms on Material's standard decelerate
    curve and fades on its standard accelerate (`motion::TOAST_MS`,
    `Out::toast_shown`), shown 4 s (`tokens::TOAST_SHOW_MS`);
  - a step card opens with its height growing over 300 ms, Material's
    medium 2, on its emphasized decelerate curve (`motion::ease_open`,
    F108), and the jump that brings the open card into view
    (`Scroll::follow`) is a glide;
  - a switch's fill slides in 150 ms on Material's standard curve
    (`motion::SWITCH_MS`, `motion::ease_move`);
  - a change of theme cross-fades the whole frame the same way, over
    350 ms (`motion::THEME_FADE_MS`), on Material's standard curve
    (`motion::ease_move`),
    since every pixel changes colour: it is a change like any other to
    `Faraday::fade_begin` (the theme is part of what a frame drew), so
    it is at once with reduce motion and on a machine whose fades are
    gated off, runs to its end, and a second theme pressed mid-fade
    starts from the frame on screen;
  - a change of screen or sheet, or of the Learn sheet's tab,
    cross-fades from the frame before it over 200 ms, linear
    (`motion::FADE_MS`; 2026-10-12, phase 4.6; an eased 150 ms was over
    in three frames and read as a flash): the frame that shows the
    change draws the new screen in full, keeps it aside
    (`Canvas::swap_pixels`, no copy) and shows all the old one; the
    fade's clock starts at the tick after it, backdated one frame, so a
    slow first frame does not use the fade up. Each step after that
    nothing else asks a frame for blends the two kept frames
    (`Canvas::blend_between`, `Faraday::fade_step`) and draws nothing
    again, so a screen slow to draw fades as smoothly as any other; a
    frame something else asks for (a hover, a toast, a scroll) is drawn
    in full and kept aside again. A step part of the way asks for the
    next itself, so the shell keeps drawing from the first one (the
    desktop shell answers a frame's own asks after presenting it). At
    once with reduce motion, and at once on a machine whose fades come
    in steps too far apart to read as one: the core times the gap
    between each two steps of a fade that follow one another (not the
    frame that shows the change, not a frame drawn in full, not any
    other motion), keeps the last nine with when each was taken, and
    fades while the median of those under 30 s old
    (`motion::FADE_GAPS_FRESH_MS`) is 35 ms or under, 30 Hz or better
    (`motion::FADE_GAP_MS`, `Faraday::fades`); with fewer than five
    known it fades, and a new display size forgets them. A fade under
    way runs to its end.

    Why (2026-10-22, owner: fades on the sidebar's items, none on
    opening Learn, and none on switching tabs after). Each frame of a
    fade used to draw the new screen in full and blend it; the Learn
    sheet's frame draws the page under it, puts back the frosted copy
    and lays out a page of text, and its frosted page differs from the
    page before at every pixel, so its fade's frames cost about 2.3
    times a change of screen's (14 ms against 6 on the build machine
    in a 2560 × 1600 window; on a slower machine past two refreshes,
    three steps in 200 ms). The gate then timed every tick that asked
    for a frame, so those gaps, a scroll of a page slow to draw, and the
    gap from a motion's last frame to the 50 ms idle tick all counted
    against the machine; five slow gaps of nine turned fades off, and
    nothing timed a gap again until something else moved, so they
    stayed off. The Learn sheet's tabs were not a change the fade knew.
    Now a fade's step costs a blend alone (5 ms for Learn, under 3 for
    a change of screen, same window), the blend runs a byte at a time so
    the compiler takes many at once, only a fade's own steps are timed,
    and old gaps are forgotten (`tests/all/transitions.rs`,
    `faraday-core/src/work/budget/fades.rs`).

    The median and the late clock are there because the frame a sheet
    opens on frosts the page under it, the slowest frame the app draws
    (40 ms at 1080p, about twice that in a window with twice 1080p's
    pixels): a smoothed mean counted it, judged the machine too slow and
    cancelled the very fade the sheet had started, which is why the
    owner saw none on the ? button (2026-10-12). `Faraday::settle` ends a
    fade and the next frame is drawn at rest, so the snapshot tool and a
    test that reads pixels see the screen itself;

    The 2026-10-07 removal (item 3.9) judged fades under the timer
    pacing, which doubled or dropped a frame every few seconds: a fade
    shows every such slip as a flicker in brightness. With frames paced
    by the compositor (§3.2) they are tried again;
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

- Changing the theme cross-fades from the old palette to the new
  (§3.5).
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
| OK | `#8bd4b2` | `#1c8053` |
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
Since 2026-10-11 (owner) a first start is in **Cobalt**
(`ui::pal::COBALT`): a cool blue-white page with a deep cobalt accent,
#0062A3 (6.0:1 on the page). The same day Bitcoin Orange Light took
Bitcoin's orange (#F7931A) at full strength, with dark text on its
buttons, since white on it is 2.1:1; its accent text on the page is
2.1:1 too: the owner asked for the orange itself.
Lamplight (2026-10-09, `ui::pal::LAMPLIGHT`) is Faraday's own light
theme: warm cream page, espresso text, a terracotta accent (#A34E26,
5.2:1 on the page) and a sage, amber and brick at 4.5:1 or more.
High contrast dark and High contrast light (`ui::pal::HIGH_CONTRAST_DARK`,
`HIGH_CONTRAST_LIGHT`; light is `Theme::is_light`) are pure black and
pure white pages with white and black text and one strong accent, yellow
and deep blue (#0000CC): every text pair is 7:1 or more on the page and
the card, and edges, rules and focus rings 3:1 or more. Mars
(`ui::pal::MARS`, id `mars`) is a warm near-black page with butterscotch
text and Mars's rust red-orange accent, #D9603B (5.2:1 on the page).
The palettes are in `faraday/faraday-ui/src/pal/`: the colours and
Dark in `mod.rs`, the other themes' tables in `light.rs`, `dark.rs` and
`high_contrast.rs`.
Since 2026-10-23 (owner) every theme's text, accent, positive and
warning read at 4.5:1 or more on its page and its cards, and its dimmed
text at 3:1 or more, Bitcoin Orange Light's accent alone excepted: the
positive green of Light (#1C8053), Cobalt (#1C7F52) and Bitcoin Orange
Light (#1C7E52), and Rosé Pine's warning (#A26111) and dimmed text
(#918B9F), each darkened by the least that reaches it.

Settings › Appearance (since 2026-10-23) groups the tiles under three
small labels, Light, Dark and High contrast (`ThemeGroup`, worked out
from `Theme::is_high_contrast` and `Theme::is_light`), each group's
tiles in order of their names (`Ui::theme_groups`), as many to a row
as `Ui::theme_grid` fits so the groups' tiles line up. The tile of the
theme a first start is in, Cobalt (`Theme`'s default), carries a small
Default mark after its name.

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

### 3.9 The stick's touchpad: click-and-drag and tap-and-drag (2026-10-19)

Owner-asked 2026-10-19: dragging a scrollbar with a
laptop's touchpad did not work on the stick. The pad should do what a
person's laptop does under Linux (libinput) and Windows (Precision
Touchpad), so the core sees what a mouse sends: Down, Moves, Up.

**What was wrong.** Two gestures failed, for two reasons
(`faraday/shells/stick/src/pointer.rs`, `cursor.rs`):

- *A held click and a second finger.* On a clickpad the finger that
  presses rests on the pad; the drag is made with a second finger. The
  pad was followed by `ABS_X`/`ABS_Y`, the kernel's single-touch
  emulation, which tracks the oldest contact, so the pointer followed
  the resting finger and the moving one moved nothing. And any two
  fingers on the pad (`BTN_TOOL_DOUBLETAP`) were a two-finger scroll,
  held button or not, so the core got scrolls between the Down and the
  Up instead of Moves.
- *Tap-and-drag.* A tap was sent whole on the lift, Down and Up
  together, so there was nothing held for a second touch to drag.

**The standard behaviour.** From libinput's documentation (Tapping,
Clickpad software buttons), its API header (`libinput.h`, drag lock)
and its source (`evdev-mt-touchpad-tap.c`, `-gestures.c`),
and Windows' Settings > Touchpad defaults:

- Tap-to-click: a touch that lifts within the tap timeout (libinput
  180 ms) having moved less than the tap motion threshold (libinput
  1.3 mm) is a tap. libinput presses the button when the finger lifts
  and releases it when the tap timeout runs out with no new touch.
  Windows has tap-to-click on by default and publishes no timings.
- Tap-and-drag: a touch that lands within the tap timeout of a tap's
  lift keeps the button held; the finger then drags, and lifting it
  releases the button. A touch that lifts again at once is a double
  tap: release, press, and the wait again. On in libinput where tapping
  is on, and on in Windows ("Tap twice and drag to multi-select").
- Drag lock: libinput's default is off (lifting ends the drag at once;
  it has an optional timeout or sticky mode, and a short automatic lock
  only when the finger lifts at the pad's very edge). Windows ends the
  drag on lift. Chosen: **no drag lock**.
- A held click and a second finger: the pressing finger is ignored and
  the pointer follows the finger that moves; no finger scrolls while
  the button is held (libinput, `evdev-mt-touchpad-gestures.c`: while
  a clickpad is clicked, or a tap-and-drag holds the button, it forces
  one-finger mode, so the fingers move the pointer and do not scroll;
  a finger in the button area does not move the pointer. Windows: click
  with one finger, drag with another).
- Software button areas (libinput, about 10 mm at the bottom of a
  clickpad, the default on PC clickpads; clickfinger on Apple's) decide
  which button a click is. The core has one button, so every click is
  the left one and the areas are not needed for that. libinput also
  stops a thumb resting in that strip from moving the pointer or
  counting as a scroll's second finger.

**Chosen rules, with numbers.**

- **Contacts by slot.** A pad that reports multitouch axes
  (`ABS_MT_SLOT`, `ABS_MT_TRACKING_ID`, `ABS_MT_POSITION_*`) is followed
  contact by contact. In each packet the contact that travelled
  furthest (|dx| + |dy|) is the one that moves the pointer; a contact's
  first position after it lands moves nothing, so landing or lifting a
  finger never makes the cursor jump. A pad with no multitouch axes is
  followed by `ABS_X`/`ABS_Y` as before.
- **Held click.** While `BTN_LEFT` is held, every finger's travel is a
  move and none is a scroll; two fingers that were scrolling when the
  button went down end their scroll first. So a press is Down at the
  cursor, the moving finger's travel is Moves, the release is Up.
- **Two fingers, no click:** a scroll, as before (natural direction,
  the pan's carry kept).
- **Tap:** a touch that lifts within **180 ms** (`TAP_MS`, libinput's
  tap timeout; 200 ms before) and carried the cursor no further than
  `TAP_TRAVEL_PX` steps (§3.2) presses at the cursor on the lift (Down)
  and holds it for 180 ms. If no finger lands in that time it comes up
  (Up): a click. The main loop asks the cursor for the release on every
  pass (`Cursor::due`); while it is held the loop is on the frame grid,
  so it is at most a frame late.
- **Tap-and-drag:** a finger that lands within 180 ms of the tap's lift
  (measured on the kernel's timestamps) keeps the press held; its
  travel is Moves, and lifting it is Up, whenever that is. Lifted again
  within 180 ms without travel, it is a second tap: Up, Down, and the
  180 ms wait again. A second finger joining the drag moves the pointer
  with it rather than scrolling. A finger that lands later than 180 ms
  after the lift ends the click (Up) first and is a plain touch: it
  moves the pointer, and may be a tap of its own.
- **No drag lock**, as libinput's and Windows' defaults. libinput's
  automatic lock at the pad's edge is not copied: it needs the pad's
  size, which the shell does not read (it uses no ioctls).
- **No acceleration** (F43 stands): one count moves the cursor by the
  arrow's scale times `--pointer-speed`.
- **Not done:** the software button areas and the bottom-strip thumb
  rule, which need the pad's size and resolution in millimetres
  (`EVIOCGABS`). Until then a thumb resting on the pad while another
  finger moves counts as a second finger, and so scrolls unless the
  button is held.

Tests (`faraday/shells/stick/src/pointer.rs`, `pointer::tests::pad`,
synthetic multitouch packets through the pointer and the cursor to the
core's events): a held click and a second finger moving give Down,
Moves following the moving finger, Up; tap, quick touch and move give
Down, Moves, Up on the lift; a single tap is Down then Up after the
wait; two fingers with no click still scroll; a touch 300 ms after a
tap is a plain move; and a held click and a second finger dragging the
scrollbar of Settings at 1280×800 scroll the page to the top and press
nothing. `cursor.rs` tests the tap states directly: the wait, the
drag, the double tap, the slow touch, two fingers during a drag.

## 4. Tests

User-facing behaviour only (`CLAUDE.md`):

- `faraday-core/tests/all/scroll.rs`, rewritten to the new rules: a wheel
  notch glides and ends exactly 48 pixels on; notches in quick
  succession add up; a pan moves at once; a pan then `ScrollEnd` keeps
  moving and comes to rest; a pan past the top stretches and springs
  back to the top; the wheel at an end does not stretch; offsets are
  whole pixels after any of these; a touchscreen drag scrolls a page.
- `faraday-core/tests/all/hover.rs`: hovering a button changes its pixels
  and `HoverEnd` restores them; hovering does not press.
- `faraday-core/tests/all/appearance.rs`: Settings › Light turns the page's
  background light and Dark turns it back; a QR stays black on white.
- `faraday-draw` canvas: a rounded card cut by a scroll region's top and
  bottom draws the same pixels on the fast path as through the mask.
- Stick shell: two fingers lifting send `ScrollEnd`; a hi-res wheel's
  half notch scrolls half a notch; the pan carries its remainder.

## 5. Notes for whoever changes this next

- Every scrolled screen calls `Ui::report_scroll` (or
  `report_scroll_own_bar`) once its content is drawn; a new scrolled
  screen must too, or it has no ends, no stretch, no fades and no bar,
  and must be added to `Faraday::scroll_slot`.
- A driver that takes pictures of the screens calls `Faraday::settle`
  first (the snapshot tool does), and draws a frame after a press before
  scrolling, as a shell does: a scroll stops where the last frame said
  the page ends.
- A fade's steps blend the kept frames only while `dirty` is not set;
  anything that changes what a frame shows sets `dirty`, as it always
  has, and is drawn in full. A change that alters the screen without
  setting `dirty` would be shown only when the fade ends. A tick asks
  for the fade's next step through `fade_due`, not `dirty`.
- Tests that read pixels after a change tick at 16 ms first, as a shell
  does (`tests/all/transitions.rs`, `tests/all/scroll.rs`).
- Settings scrolls now: it outgrew 800 units with Appearance and Motion.
- On the desktop, a `Tick` is sent on every `RedrawRequested` when the
  window is Wayland's (`Shell::paced`); a change that needs a tick before
  a frame gets one there and must not add a second. A shell on a
  compositor that does not send frame callbacks to a hidden window
  relies on the 50 ms timer for its ticks meanwhile; do not remove it.
- The stick compares the converted frame with the one on the panel
  before writing; anything that writes to the framebuffer around the
  shell's `draw` (as `blank` does) clears `previous`, or the next frame
  is written in part over something else. A console sharing the
  framebuffer on a developer's machine is not noticed: its marks stay
  until the rows under them change. The image has no console.

