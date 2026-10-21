# MOTION build log: the progress list and the measurements after, as written at the time

**Kind:** History
**Status:** Kept as written; section numbers are the spec's when it was written. The current spec is `docs/MOTION.md`.

## 5. Progress

Tick an item when it is built and its tests pass; note anything a later
session must know beside it.

Phase 1 — scrolling that is smooth with no change in feel elsewhere:

- [x] 1.1 Canvas: clipped path primitives without a mask (§3.4), with
      the pixel-equality test (`canvas::draw_path`). Backup at
      1920×1080: 11 ms → 3.9 ms a frame.
- [x] 1.2 Shell API: `Wheel`, `ScrollEnd`, `Hover`, `HoverEnd` (§3.1);
      a shell that does not use them has `faraday-draw` take `Wheel` as `Scroll` and ignore the rest.
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
      `faraday-draw`, unused here.

Phase 4 — pacing (2026-10-12, owner: scrolling still did not read as
smooth as a native window):

- [x] 4.1 Desktop shell: frames paced by the compositor on Wayland
      (§3.2), the 60 Hz grid kept under X11; the copy into the window
      goes a row at a time instead of a pixel at a time with a bounds
      test each; `--timings`.
- [x] 4.2 Stick shell: only the rows that changed are written (§3.2),
      with the test in `fb.rs`.
- [x] 4.3 The grid's step follows the frame cost on the stick and under
      X11 (§3.2, `pace.rs`); the desktop's timer takes the monitor's
      refresh period.
- [x] 4.4 A shape wholly outside the clip is not drawn (§3.2): off-page
      cards no longer cost a fill and a restore each frame.
- [x] 4.6 Cross-fades back, for a change of screen or sheet, gated on
      the frame cadence the core measures (§3.5); `tests/all/transitions.rs`
      covers the fade, reduce motion and the slow machine.
- [x] 4.7 The wheel notch and the arrow keys move 56 units (§3.3),
      the same amount of page on every screen; `tests/all/scroll.rs`.
- [x] 4.8 The stick's arrow is painted by the input thread that read
      the move, a few rows at a time, on a panel shared under a lock and
      written in bands (§3.2; `panel.rs` and its tests, the band test in
      `cursor.rs`). Owner's stick tests: scrolling good; the mouse jumpy,
      better once the loop painted it as moves arrived, still jumping
      whenever the loop drew a frame, hence the thread; the 150 ms fades
      not seen; the fade is 200 ms linear now. Then still no fade on
      the ? button: the gate is a median over nine gaps at 35 ms, and a
      slow first frame neither ends a fade nor uses it up.
- [ ] 4.5 Owner's check, with `--timings`: on a dense screen the
      window has about twice 1080p's pixels, so the render alone is
      near twice §6's 1080p figures. If the render is
      still the limit on a slow machine, the next step is §3.4's last
      item: scroll by copying the rows that stay and draw only the
      exposed strip.

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
- `faraday-draw` at opt-level 3 in release changed none of these numbers (`Cargo.toml`), so the profile is as it was.
- The frame a sheet opens on costs two frames at 1080p, once.
  Frosting by a coarser reduction would be cheaper still.
- Not measured here: the framebuffer write on the stick (`--timings`
  on a stick says how long a frame's `pwrite` takes).
