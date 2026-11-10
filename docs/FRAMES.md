# Frames at the panel's rate

**Kind:** Spec
**Status:** Specified; the first two steps of §3 are built (`Ui::region`, the whole-pixel offset, the pages and panes laid out from offset 0 and the content layer; the moved frame with `moved.rs`, `Out::redraw` and `Out::moved`, `Parts::moved` and the two tests of §3.2 and §3.4, opted in by the small panel's Settings, Wallets, a wallet's page, Files, Vault contents, Backups, the backup board and checklist and the step columns the test walks, and by the wide Wallets, Vault contents, Settings, Backups and those step-column pages), and so are the Pi's frame written without a conversion (§5, the kernel patch and the shell's red-first depth; its figure on a card is not yet taken), the framebuffer write's first three points (§6) and the face kept per size, `Ui::wrap_lines` linear in its words and Home's backup count and a wallet's backup line worked out without every wallet's map, the signing page's text lines kept by file, the Backups screen's list and the wallet chart kept across moved frames and the shell's calls to the disk process on a thread of their own, of §7; the stick shell takes the moved frame (§3.7: `Presenter::present_moved`, in ordinary memory, with the panel written bands rather than a span), and the frame drawn in bands across the cores (§4: `faraday-core`'s `bands.rs`, in every whole frame; a moved frame paints its strip as it is called); and the stick's centred window on a larger screen (§6, `faraday.window=`); the rest is not. Faraday Demo ignores `Parts::moved`, as §3.6 says
**Open:** the rest of it: `Moves::Yes` on the wide Files page and its Files-out pane, which the test drives to no moved frame (so they draw whole), `Moves::Yes` on the step columns the test does not walk (Convert, BIP-85, Silent, Keygen, Vanity, Message, Restore, Family, Signer check, Create vault: they draw whole), with the regions still laid out at their offset (the Board's list, the codex32 dice sheet and worksheet, Learn's body, the wide Family tab and the wide Files page's head, which draw in pixels of their own or in a scrolled place that depends on the offset; `Region::from_zero` is on every other), the stick's moved frame timed on the dev card with `--timings` (§3.5), the window's frame time on a card beside the whole screen's, the stick's look for input devices and its calls to the disk process, off the main loop, timed on the dev card, what a frame does beside its pixels that §7 has not yet found or fixed (among it, the Backups screen's list and the wallet chart, which a whole frame still works out, the first moved frame after it too). While the scanner sheet is open the stick asks its camera for 1920 × 1080 so that a dense QR reads, and the sheet's preview is the 640 × 360 reduction of it (three times the bytes of the 320 × 240 before), moved and wiped per frame; no other screen pays, and the cost of the sheet on a Pi is not measured The Pi's vsync wait does not follow the screen, since the first CRTC is the video core's writeback block and turning that block off left the Pi with no picture (§6).

The Pi's frame grid runs at 15 Hz while a finger drags, and at 30 Hz
once its clock follows the work (`docs/MOTION.md` §3.2); the aim is
60 Hz, a frame and its way to the panel inside 16.7 ms. With the
refresh wait working the grid stays on one refresh while frames mostly
fit and goes coarser only after repeated late frames, judged by the
vblank rather than the cost (`docs/MOTION.md` §3.2; its figure on a Pi
is not yet taken). This spec
designs four speed-ups the owner asked for (2026-11-02), measures what
each can give, and orders the work. Every shell gains from the parts in
the core: the stick on a PC and on the Pi (`faraday/shells/stick`, one
shell for both) and Faraday Demo (`faraday/shells/desktop` over
`faraday/shells/window`).

## 1. Where a frame's time goes

A pass on the stick is the core's frame (`App::frame`), then the
shell's comparison and conversion (`present.rs`, the `convert` bucket of
`--timings`) and the write to the framebuffer (`panel.rs`, `write`).
The Pi's dev card, measured at its lowest clock while a finger moved:
frame 27 to 32 ms (worst 53), convert 4.8 to 5.1 ms, write 3.1 to
3.4 ms, on the firmware's 32-bit framebuffer. The kernel's vc4 emulation
the image has now is 16 bits (the boot report's `panel` line) until the
patch of §5 makes it 32: the panel is written as the core's frame is,
with no conversion (§5).

At no turn the shell converts the changed rows and copies them into the
kept frame in one pass per row (`fb::convert_keep`), the rows in bands
over up to eight threads, the first on the calling thread, each depth a
loop over whole pixels the compiler vectorises. At 3200 × 1800 with
every row changed, as a fade step changes them, `present` took 12.4 ms
(BGRA) and 11.5 ms (24-bit BGR) before and 5.7 ms and 4.6 ms after, on
the build machine in release (the ignored test
`convert_step_cost_at_3200_by_1800_with_every_row_changed`). A frame
turned a half or a quarter is converted tile by tile on the calling thread.

**Measured** (the build machine, release, the heaviest session of
`docs/SPEED.md` §7, the median of 41 frames each drawn in full; a probe
kept outside the repository). Each frame was drawn whole, then with the
canvas's clip held to a band of rows: each quarter of the panel in turn,
a strip of 40 rows, and one row. A frame held to one row paints almost
nothing, so what it still costs is what a frame does beside its pixels:
the screen's own walk through what it shows, which no clip removes
("floor" below).

| Screen, the Pi's 480 × 640 | Whole frame | Floor (one row) | Floor share | 40-row strip | Slowest quarter |
| --- | --- | --- | --- | --- | --- |
| Home | 1.23 ms | 0.57 | 46 % | 0.57 | 0.83 |
| Wallets | 0.99 | 0.57 | 58 % | 0.59 | 0.79 |
| A wallet's page | 0.99 | 0.55 | 55 % | 0.56 | 0.80 |
| Backup board | 0.56 | 0.12 | 21 % | 0.16 | 0.30 |
| Backup checklist | 0.81 | 0.27 | 33 % | 0.28 | 0.47 |
| Backup words | 0.79 | 0.27 | 34 % | 0.28 | 0.46 |
| Vault contents | 1.99 | 1.38 | 69 % | 1.40 | 1.85 |
| Settings | 0.58 | 0.08 | 13 % | 0.08 | 0.39 |
| Files | 3.42 | 2.67 | 78 % | 2.72 | 3.17 |

At 1366 × 768 the floor's share is about the same: Home 54 %, Wallets
53 %, Settings 30 %, Vault contents 67 %, Files 76 %. The strip costs
the same whether the screen lays out only what meets the clip or
everything in view (within 5 %), so a frame that paints a strip but lays
out the whole view, as §3 does, costs its floor and the strip.

The ratio between the two machines comes from Home on the Pi's panel:
the dev card's frame at its lowest clock is about 25 times the build
machine's, and at its full clock it should be about 11 times. So at full
clock a frame of Home is about 13 ms on the Pi, Vault contents about 22,
and Files about 37: the last two miss 60 Hz on their floor alone.

The shell's side, on the build machine, a whole 480 × 640 frame of
32 bits: the conversion to the Pi's byte order 129 µs, a plain copy
19 µs, a comparison of two equal frames 19 µs. The conversion is about
seven copies' worth, which is why `convert` is the Pi's second cost.

## 2. Rules

1. **The same pixels.** Each speed-up gives, byte for byte, the frame a
   whole draw gives; the snapshot tests, which compare pixels, pass
   unchanged but where §3.2 says.
2. **A fallback that is always right.** Where a speed-up cannot be sure
   of rule 1 it draws the frame whole, as today.
3. **Drawing reads** (`docs/SPEED.md` §3.3, `docs/COMPUTE.md` §4). No
   part works out more while drawing; what a frame keeps between frames
   is pixels and rectangles, public by nature (SPEED §2 rule 1: a
   region's pixels are what the panel already shows).
4. **The repository holds nothing tuned to one machine.** Thread counts
   come from the machine the app runs on; no count, timing or size of
   a particular machine is written in code, config or tests.

## 3. A scroll step moves the rows already drawn

### 3.1 What a moved frame is

A scroll step moves a region's content by a whole number of pixels: the
offset is held to whole device pixels (`whole_offset`, MOTION §3.3). A
**moved frame** keeps the region's content from the frame before,
moved by that number, and paints only the rows the move brings into
view; everything outside the region and every overlay on it is drawn
as today.

The region's **content layer** is the region's pixels as its content
left them, before any overlay: `Ui` copies the region's view into it at
the start of `Ui::region`'s close, before the stretch, the edge fades and
the scrollbar. Its cost is a copy of the view (about 85 µs on the build machine at the
Pi's size, measured on Settings; §3.5). The copy is taken only for a
region that opts in (`Moves::Yes`), while `Ui::keep_content` is on,
which the frame sets; any other region copies nothing. A secret on show
is in the layer while it shows, as it is on the canvas. Hiding it is a
whole frame, which takes the layer again, and a frame in which no region
took one overwrites and empties it (`Canvas::drop_layer`), so its pixels
do not outlast the page or a lock (F195).

A moved frame, for the active region of the last frame, moving it `d`
pixels:

1. Everything is drawn as today, but in the region's view the canvas's
   clip is the strip `d` rows deep at the edge the move uncovers
   (`Ui::region`, §3.3). The screen lays out the whole view as it does
   now, so its presses, hits, fields, tip anchors and the step column's
   answers are the whole view's (`Ui::hit` takes `hit_clip`, the view,
   not the strip); only the pixels are held to the strip.
2. At `Ui::region`'s close, before any overlay, the content layer moved by
   `d` is copied into the view but for the strip, then the view is
   copied into the layer again.
3. The overlays are drawn as today: the stretch (`stretch_view`, which
   now moves the layer's copy, so a frame that only stretches paints no
   strip at all), the edge fades, the scrollbar, sticky headers, the app
   bar's scrolled state, and whatever is drawn after the region (a
   toast, a sheet over the page, the FAB).

What is outside the view is drawn whole each frame, as now: the app
bar, the navigation, a pane beside the region. Their cost is part of
the floor of §1.

### 3.2 Why the pixels are the same

A row of the region's content depends only on the content and the row's
place in it, if three things hold:

- **Content placed in whole pixels.** Today a row's place is
  `round((y − offset) × f)`, which in floating point can round a
  half-pixel the other way from `round(y × f) − offset × f`. A region's
  content is laid out at offset 0 and moved by a whole number of pixels
  (`Ui::oy_px`, applied after `Ui::rect` rounds), so a moved row and a
  drawn row are the same row. This moves a few edges by a pixel against
  today's snapshots, once; the tours are rendered again
  and the change says which changed and why.
- **A clip at a row cuts nothing inside it.** A shape cut at the clip's
  top or bottom is filled into the clip's rows (MOTION §3.4), and
  tiny-skia's coverage of a pixel depends on the path and the pixel's
  own rows only; text and icons blit whole glyphs cut at the clip. The
  existing pixel-equality test of clipped shapes in `canvas.rs` is
  extended to bands cut through a shape, a glyph and a shadow. A rounded
  shape is made about the origin, each point cut to the 1/256 pixel the
  rasterizer cuts it to and moved by whole pixels as it is added to the
  path (`canvas.rs`, `Builder`; a stroke's outline is cut and moved
  after it is made), so it is the same pixels wherever on a long
  page it stands (a circle likewise); made at its place, floating-point
  rounding differed with how large the place's numbers were. The paths
  are made in builders the canvas keeps from one shape to the next. The
  canvas has hidden rows below the visible ones (`hidden_rows`, 64
  units: a corner's 28 and a shadow's reach), which nothing reads and
  a clear does not fill, so a path that
  runs past the last visible row is drawn whole once, and its curves
  there have the coverage they have anywhere else; tiny-skia cuts a
  path at the end of its pixmap, and a curve crossing the cut comes out
  differently. A shadow reaches past its
  shape, so a moved frame paints `SHADOW_REACH` units of rows beside its
  strip as well.
- **Nothing in the content changed but the offset.** §3.4.

**Held by** a test, `moved::tests::every_scroll_step_shows_what_a_whole_frame_shows`:
on every main screen and the heavy sheets of the budget session, on the
Pi's panel and a desktop, scrolls in steps of 1, 7, 40 and a third of
the view, to each end and past it (a stretch), and back; after each
step the moved frame is compared byte for byte with the same state
drawn whole (`Faraday::draw_whole_next`, a test hook). The snapshot
tool draws through the same path, so its tours that scroll check it
too.

### 3.3 Where it is decided

- **`Ui::region(slot, view, ground, |ui| …)`** (faraday-ui) replaces each
  pair of a clip pushed for a region's view and `report_scroll*` after
  it (40 calls in 31 files). Inside it `Ui` knows the region before its
  content is drawn: it pushes the view's clip, or the strip's on a moved
  frame, draws the content, pastes the layer, takes the layer again and
  draws the overlays.
- **The core decides** (`faraday-core/src/moved.rs`, new): a frame is a
  moved frame when all of these hold, and drawn whole otherwise.
  1. The last frame drew the same region (screen and sheet, slot,
     view, the theme, scale, text size and panel size) and kept its
     layer.
  2. Nothing asked for the frame but the region's motion: a pan, a
     wheel's glide, a coast, the stretch and its spring. Every other
     reason to draw is a whole frame: `out.dirty = true` (54 places)
     becomes `out.redraw()`, a whole frame, and the motion's ticks call
     `out.moved()` (§3.4).
  3. `|d|` is less than the view's height less one row, and the stretch
     before and after is within the view.
  4. Nothing in the region moves of itself: no fade or step-card
     disclosure under way, no caret in a field in the view, no state
     layer changing (a hover moved onto another item, a press), no
     camera frame, no Learn picture loading.
  5. The snapshot tool's ink audit is off (`Canvas::record_ink`): the
     audit reads every string's ink, which a strip does not draw.
- **A screen opts in.** `Ui::region` takes a moved frame only for a
  region whose screen says its content is a function of its state and
  offset alone (`Moves::Yes`); the test of §3.2 runs over every screen
  that says so. The long ones first: Files, Wallets, a wallet's page,
  Vault contents, Settings, Learn, the backup checklist and board, the
  step column. A screen that has not opted in draws whole, as today. A
  step column says so per flow (`flow::Column::moves`): Create, Spend,
  the backup checklist and the backup flows, the ones the test walks.

### 3.4 What a frame is asked for

`Out` gains a reason: `redraw()` (whole) and `moved()` (only the active
region's offset or stretch changed). A frame asked for by both is
whole. The motion code (`motion.rs`, the pan, glide, coast and spring
steps, `with_region`) is the only caller of `moved()`; a test drives
each other event in the shell API and checks the next frame is drawn
whole (`moved::tests::every_other_event_draws_the_frame_whole`).

### 3.5 What it gives

A moved frame costs the floor of §1, the strip and two copies of the
view. From §1's figures on the Pi's panel: Home from 1.23 ms to about
0.61 (2.0 times quicker), Wallets 0.99 to 0.63 (1.6), the backup
checklist 0.81 to 0.32 (2.5), Settings 0.58 to 0.12 (4.8), Vault
contents 1.99 to 1.46 (1.4), Files 3.42 to 2.76 (1.2). On the Pi at
full clock a scroll step of Home would take about 7 ms in the core, and
with §5 and §6 a pass about 10: 60 Hz while dragging Home, Wallets,
the backup flow and Settings. Files and Vault contents stay over a
refresh on their floor, which is §7's.

Those figures are a forecast and do not hold on the build machine. Measured
there on compact Settings, three runs, a step of 40 pixels, every step a
moved frame: a scroll step 0.37 ms before the moved frame and 0.38 after, a frame drawn
whole 0.32 before and 0.40 after. The two copies of the view cost about what
the painting they save. What a Pi gains is not yet measured; it is read on
the dev card. Screens are opted in on the owner's decision without that figure.

Measured on the build machine at 480x640 once Wallets, a wallet's page,
Files and Vault contents were opted in: a frame drawn whole is about 7%
slower on Wallets and Files, about 2% on Vault contents and no slower on
Settings; a scroll step on them is about 20 to 30% under a whole frame.

### 3.6 Per shell

- **The stick on a PC.** The core's moved frame as above. The shell's
  comparison then finds the strip and the overlays changed, and every
  other row of the view moved: §3.7 has the shell move its own copies
  the same way, so it converts only the strip and the overlays. The
  framebuffer still takes every row of the view (§3.7).
- **The Pi.** The same, and with §5 nothing is converted at all.
- **Faraday Demo.** The core's moved frame as above. The window copies
  what changed into the window's buffer and gives the compositor that
  much (`Shown`, `Parts::changed`, SPEED §13): after a scroll step that
  is the view, as now, since every row in it shows another row. Moving
  rows inside the window's buffer is not done: softbuffer hands back
  one of two or three buffers, each holding a frame one, two or three
  presents old, so a move would have to compose the steps since that
  buffer was shown, and the copy it saves is a few hundred
  microseconds at 4K against a frame of several milliseconds.
- **The Starter** draws with faraday-ui's regions too; it gains a moved
  frame when its pages opt in, later, after its own test.

### 3.7 The shell after a moved frame

The core says how the frame moved: `Parts` gains `moved:
Option<Moved>` (`faraday-shell-api`), the region's columns and rows and
`d`, set only when the frame is a moved frame and relative to the frame
the shell last took (two moved frames between two takes compose when
they move the same region; any other mix is `None`). A shell that
ignores it is right, as `changed` is today.

The stick's `Presenter` (`present.rs`), given `moved`, moves the rows of
its kept frame (`last`) and of its converted copy by `d` inside the
region's columns, a `copy_within` in ordinary memory, before it
compares. The comparison then finds only the strip, the overlays and
whatever is outside the region changed, and only those are converted.
`fb::changed_rows` returns bands rather than the span from the first to
the last row that differs (`fb::merge_bands` exists), so a strip at the
foot and a fade at the head are two bands, not the whole view. A frame
turned a quarter onto the panel (ORIENTATION §3) moves columns of the
panel instead; the presenter takes it whole, as now.

The framebuffer itself takes every row of the view: each of its rows
now shows another row. Moving rows inside the framebuffer is not done.
Reading framebuffer memory is uncached on the Pi and write-combined on
a PC, many times slower than writing it; the fbdev interface gives a
program no copy within the framebuffer (the kernel's `copyarea` is for
its console), and panning moves the whole screen, not a region. §6 makes
the write itself cheaper.

## 4. The frame drawn in bands across the cores

### 4.1 Why not each band drawing the screen

The core draws through `View`, which reads `Faraday` and fills the
drawing memo (`Cell`, `RefCell`) as it goes; neither is shared across
threads, and the token is not `Send` (COMPUTE §2.1). And §1 shows what
re-drawing the screen once a band would cost: the four quarters drawn
one by one each pay the floor, so they sum to 2.3 to 3.4 times the
whole frame, and the slowest quarter is 0.68 to 0.93 of it. Four
threads each drawing a quarter would give at best 1.1 to 1.5 times.

### 4.2 Recorded once, painted in bands

So the frame is drawn once, on the main thread, into a **list**, and
the list is painted in bands:

- `Canvas` gains a recording mode (`faraday-draw`, `canvas/list.rs`,
  new): each painting call (`fill_rect`, a path, a rounded rectangle,
  text, an icon, a kept shape, a shadow, `fill_cells`, `luma`, a blend)
  appends what it is, its colour and its clip to the list instead of
  painting. Glyphs and kept shapes are looked up as the call is
  recorded (the lookup already happens for a line's advance), so the
  glyph cache and the shape cache are complete before any band paints
  and are only read while bands paint; nothing is evicted during a
  frame (SPEED §3.8).
- The list is painted by `n` bands of rows, each band's calls in the
  order recorded, and a band writes only its own rows (disjoint
  `&mut [u8]`). A pixel's value depends only on the calls that touch it,
  in their order, but a path is not painted by clipping alone:
  `canvas.rs` draws a path on the whole canvas and puts back the rows
  outside the clip, because tiny-skia cuts a path at its pixmap's edge
  and a curve crossing the cut has other coverage. So a band paints a
  path at its own canvas coordinates, never moved, because tiny-skia
  does not rasterize a path identically when it is moved by whole rows
  (a stroked rounded card differs by a few anti-aliased pixels):
  - A path wholly inside the first band, whose rows start at the
    canvas's, is painted straight into the band's rows.
  - Any other path is painted into the band's scratch, a buffer of the
    whole canvas's width and rows, hidden rows included, that the band's
    room keeps between frames. The band copies its own rows of the
    path's ink (and the ink's columns) into the same rows of the
    scratch, paints the path there through the call the direct canvas
    makes, with no transform, and copies the same rows and columns back.
    Painting a row reads only that row, so the scratch's other rows may
    hold anything. A path wider than a band is rasterized by each band
    it touches and written back for that band's rows only; the last
    band holds the hidden rows. The scratch is allocated once and a
    band allocates nothing after.
  - A pixel of a secret on show must not stay in the scratch: the rows
    a band wrote in it are zeroed when it has painted its run.
  Calls are recorded with caches held (the damage of a call is noted
  when it is recorded, and a path's may be a few pixels larger than its
  ink).
  Painting with a clip, the put-back of the rows outside the clip, stays,
  inside the band. §4.6 is the proof: the bytes are equal for every band
  count.
- **Barriers.** A call that reads pixels other rows wrote ends a run:
  `shift` (the stretch) and `frost` (the blur under a sheet, once a
  sheet) are recorded as barriers. At a barrier every band finishes the
  run before it, the call runs on the main thread over the whole canvas,
  and the bands go on. A shadow is worked out per pixel from its distance
  to the shape and reads no other pixel, so it is banded, not a barrier.
  `luma` reads and writes the same pixel only and stays banded.
- **Calls that take pixels the caller holds** (`restore`, `blend_*`,
  `swap_pixels`, `fade_region`, `fade_to_background`, `place`,
  `place_scaled`, `snapshot`, `snapshot_into`, `keep_layer`,
  `paste_layer`) are not recorded: each paints the calls waiting first
  and then runs on the whole canvas, which is a barrier of its own. The
  two readers that take `&self` (`copy_region`, `frame`) read a canvas
  with nothing waiting, and the caller paints it first.
- **Counts.** What the canvas counts (`Work::Blur`, `Work::FullCanvas`,
  the lines of text) is counted as the call is recorded, on the main
  thread, so the budget tests (SPEED §7) count what they count now. A
  band allocates nothing.

What the bands give is the part of the frame that is pixels, divided
by the bands; the floor stays on the main thread. From §1, with four
bands and memory as quick as for one: Home 1.23 to about 0.73 ms (1.7
times), the backup checklist 0.81 to 0.40 (2.0), Settings 0.58 to 0.20
(2.9), Files 3.42 to 2.86 (1.2). It speeds up a frame drawn whole: a
change of screen, the first frame of a sheet, a theme, a fade's first
frame. A moved frame (§3) paints a strip, which bands barely help.

### 4.3 The pool

- Made with the canvas in faraday-core (`bands.rs`; faraday-draw is
  `no_std`; the bands' painting is there and the threads are not):
  `n − 1` threads, with the main thread as the `n`th, started with the
  whole frame and joined before it returns (`std::thread::scope`, since
  a band borrows the frame's rows and the workspace forbids `unsafe`).
  A moved frame (§3) makes none: it paints its strip as it is called.
- `n` from the machine: `std::thread::available_parallelism()`, at most
  four, one band for each 64 rows at least, and one (no threads, painted
  as now) when it says one or the canvas is under 64 rows a band.
- A fade's blend is split in bands of rows the same way
  (`faraday-draw`, `canvas/blend.rs`; `fade/bands.rs`): the screen it
  fades to is drawn in bands as above, and each step's blend of the two
  kept frames (`Canvas::blend_noting`, `Canvas::blend_within`, which cost
  a pass over the canvas or over the bands noted) runs in `n` bands with
  the first on the main thread, joined before the step returns. `n`
  is from the pixels to blend, not the rows: one band for each 256 Ki
  pixels at least, at most the cores (the cap above), so a window under
  that blends on the main thread alone. A pixel is blended from the same
  pixel of the two frames alone, so any count of bands gives the same
  pixels and the same changed rectangles as one. This keeps a step of a
  fade in a large window under the fade gate's limit (`docs/MOTION.md`
  §3.5), which is not changed.
- No new dependency: `std::thread` alone.

### 4.4 Beside the core's worker

The bands are not the worker of `docs/WORKERS.md`. The worker runs
costly work (a stretch, a signature, a picture) on its own thread while
frames go on; the bands are the drawing itself, started and joined
inside one frame, so the main thread still draws and nothing else
happens while it does (WORKERS §2 rule 1, to be worded to say the
bands are part of the drawing). They hold no `Faraday`, no `View` and
no token: a list of calls, the caches read-only and their own rows.
Drawing still reads (SPEED §3.3): what the screens do is the
recording, as now; the bands only paint.

### 4.5 The Pi's heat

The Pi 3B+'s firmware lowers its clock from 1400 to 1200 MHz above its
soft limit of 60 °C, and further near 85 °C. Four cores busy is a frame
drawn whole, a few milliseconds at a time: a change of screen, not a
scroll (§3). The dev card read 31 °C at rest. In one pair of runs of
dragging and changing screens, bands read about 2.7 °C more than no
bands (42.9 °C), and about 9 MiB more of the app's memory; the runs were
not identical, so the figure is thin. A later run of about four minutes
read at most 46.6 °C, with no undervoltage: well under the soft limit.

### 4.6 Held by

`list::tests::a_frame_painted_in_bands_is_the_frame_painted_once`:
every main screen and heavy sheet, the Pi's panel and a desktop, a
sheet over a page (a barrier), a stretch (a barrier), painted with 1,
2, 3, 4 and 7 bands, byte-equal. The build machine's snapshot tests
run with its own count of bands, so they check it too.

## 5. The Pi's panel takes the core's frame as it is

The core's frame is premultiplied RGBA, red first in memory; drawn over
an opaque page, its alpha is 255 everywhere.

**What the Pi's framebuffer was.** The Pi's panel is driven by the
kernel's vc4 with the generic framebuffer emulation (`FB_BCM2708`, the
firmware's driver, is not in the image: `faraday/image/boards/pi3/kernel.forbidden`).
vc4 sets that emulation up at 16 bits a pixel
(`drm_client_setup_with_fourcc(drm, DRM_FORMAT_RGB565)` in 6.18), which the boot report shows as
`panel 1920x1080 16bpp` on an HDMI screen, so the shell converted every
pixel of a changed row to RGB565. On a Pi 3B+ at 1400 MHz at 1080p that
`convert` bucket averaged 19.6 ms of a 25.9 ms frame, and the write
2.3 ms. The emulation's 32-bit format is XRGB8888, blue first in memory,
which would still have been a reordering.

**What is built.** Faraday's own kernel patch,
`faraday/image/boards/pi3/linux-patches/0003-vc4-fbdev-is-32-bits-with-red-first-so-the-shell-writes-the-frame-unconverted.patch`
(not from mainline), makes three changes:

- vc4 asks for a 32-bit emulation
  (`drm_client_setup_with_fourcc(drm, DRM_FORMAT_XBGR8888)`).
- The generic DMA emulation picks XBGR8888, red first in memory, where the
  legacy table gives XRGB8888. The display engine reads XBGR8888 as it
  is (it is among `vc4_plane.c`'s formats).
- `drm_fb_helper_fill_pixel_fmt` reports red at bit 0 and blue at bit 16
  for XBGR8888, so the console and the boot logo keep their colours.

The shell takes it as `fb::Depth::Rgba8888`, red first: `Geometry::from_sysfs`
reads 32 bits as that when the directory's `name` file reads `vc4drmfb`,
and as `Bgra8888` (blue first) for any other name, so a PC's firmware
framebuffer is unchanged. For `Rgba8888` at no turn the `Presenter`
keeps no converted copy and no last frame: the core's frame goes to
`Panel::write_rows` in bands of `CHUNK_ROWS`, each under its own hold of
the lock, which compares it with the panel's kept copy and writes what
differs (§6). One comparison and one copy stand where a comparison, a
conversion and three copies stood, and the time counts in the `write`
bucket. A frame turned a quarter or a half is still converted and walked
tile by tile (`fb::convert_turned`), as is every other depth; turning
and turning back leaves the panel right, because `Panel::set_turn` marks
it stale and the `Presenter` drops what it kept for a turned frame.

**What it costs.** 32 bits double the bytes a full write moves and the
size of the panel's kept copy, against 16. The colours gain: the panel
shows the frame's eight bits of each of red, green and blue, not 5-6-5.
The figure on a card is taken later with `--timings`; none is claimed
here.

What the earlier plan (the firmware's `bcm2708_fb.fbswap` and a
`dpi_output_format` change in the panel's `config.txt`) assumed is not
in the image, so neither is used. The PC stick's conversion stays (its
firmware framebuffer is blue first and the canvas paints red first): a
word-at-a-time swap measured no quicker than the byte loop (134 µs
against 129); §3.7 is its gain.

## 6. The framebuffer write

What a write does (`panel.rs`, `writer.rs`): the rows of a frame that
differ from the panel's own kept copy (`shown`) are found, and for each
band of 64 rows under the lock the arrow is drawn in where it falls and
`pwrite` is called. On vc4 the kernel's write is one `copy_from_user`
straight into the write-combined framebuffer (`drm_fbdev_fb_write`, which
calls `fb_write_screen_buffer`, in kernel 6.1's `drm_fb_helper.c`): no
shadow buffer and no copy a page at a time. It is bounded by the memory
bus and cannot be made much quicker. It writes only the bands of rows
that changed (SPEED §11.1). After a scroll step that is the whole view,
and must be (§3.7).

The change (all five points are built):

1. **One comparison.** *(Built.)* The presenter knows which rows changed; it hands
   the panel those bands, and the panel writes them without comparing
   again (`Panel::write_known`), keeping `shown` in step. The panel's
   own comparison stays for `write_rows`' other caller, the arrow.
2. **No band copy where the arrow is not.** *(Built.)* A band the arrow does not
   cross is written from the frame directly; only the arrow's rows are
   copied to draw it in.
3. **Bands, not a span** *(Built.)* (§3.7): a caret and the clock are two writes,
   not the rows between them.
4. **One buffer on the Pi.** *(Built with §5.)* On the path where the frame is
   written as it is, the presenter keeps no `last` and no converted copy
   (it drops both), so `shown` is the only kept frame: a pass that
   changes the whole view makes one copy into it (the rows written, from
   the core's frame, which the core owns), and a moved frame is taken
   whole there, as the framebuffer takes every row anyway (§3.7), so
   there is no second buffer to move. Moving `shown` itself would copy
   the same bytes the copy from the frame does. The copy that is left
   (about 1.2 MB at 480 × 640 and 4.2 MB at 1366 × 768) goes only if the core
   hands its frame over as a buffer the panel may keep. The converting
   path (a PC's other byte orders or depth, a turned frame) keeps `last`,
   the converted copy and `shown`, and the converted copy is copied into
   `shown` row by row as written. Nothing wipes `shown` or `last`
   (`Panel::blank` writes black to the framebuffer and marks the panel
   stale).
5. **The write on a thread of its own.** *(Built; its figure on a card is
   not yet taken.)* On the path of point 4, on a machine with more than
   one core, the main thread compares the frame with `shown` and copies
   the changed rows into it (`Panel::changed_in`, `Panel::keep_rows`),
   and one long-lived thread (`writer.rs`) writes those rows from
   `shown`, a band of 64 rows at a time under the panel's lock, as the
   main thread did: the arrow's thread slips in between two bands and the
   rows it draws are the same ones. The thread holds the panel and
   nothing of the app (WORKERS §2 rule 1 is about the main thread's
   work; this is a write, not a job). There is no second buffer: at most
   one write is out, and the next frame waits for it before it copies
   into `shown`, so the thread only ever reads `shown` and nothing
   queues. A frame with no changed row waits for nothing. The loop pays
   the draw, the compare and the copy, and any wait for the write
   before; a pass is about the longer of the draw and the write. The
   panel is blanked, locked, powered off or left only after the write
   out has ended (`Presenter::settle`), so no row of a hidden screen
   reaches the panel after the black, and the thread is joined when the
   shell is let go. A write that fails is told by the next frame (a frame
   late), the panel is stale, and that frame goes out whole; a thread that
   ends without answering, or panics, ends the loop as a panic on the
   main thread does. A machine with one core writes on the main thread as
   before. `--timings` gives `write` as the `pwrite`s' own time (on the
   thread, so not in the pass), `wait` as the loop's wait for it, and
   `stage` as the compare and copy.

A write of the whole view is then one copy in the kernel, from the
frame's memory into the framebuffer's, and on the Pi it runs beside the
next draw. The write is bounded by the memory bus, which does not follow
the CPU's clock: about 2.3 ms for a whole 480 × 640 frame and 18.4 ms
at 1920 × 1080 were measured before the thread, on the dev card.

Not taken, by the owner's decision: writing the framebuffer through
`mmap`. The mapped buffer is the same write-combined memory the write
copies into, so the gain over the one copy is expected to be small. It
needs `unsafe` (or a dependency that holds it), which the workspace
forbids but for one crate.

**The write after the vblank** *(Built; its figure on a card is not yet
taken)*. A frame written at a moment of its own can tear, and a frame
that misses a refresh shows a refresh late. On the Pi the framebuffer
(vc4's fbdev) implements `FBIO_WAITFORVSYNC`: it waits for the next
vertical blank of the first CRTC, gives up after 100 ms, and returns at
once when the screen is off. The `fb-write` thread calls it
(`faraday_v4l2::wait_vblank`, in the one crate that may use `unsafe`, on
a handle of its own from `try_clone`) before it writes a frame's bands,
outside the panel's lock: the main loop keeps drawing, the arrow's
thread is not held, and a blank, a lock, a power-off or an exit waits for
at most one wait. The thread tells `refresh.rs` when each wait returned;
that gives the last vblank, which the loop's grid is tied to; the
refresh length is not smoothed from them but measured once, by sixteen
waits back to back after the first write, between writes (before it the
call returned at once on the panel),
and is the median gap; the first frame is written with no wait. Sixteen
waits that all returned at once (a median under 2 ms) are no
measurement: they are tried again after a later write, and the third
such set turns the wait off for the run, saying so. A
median outside 8 to 34 ms, or gaps with no steady beat (fewer than half
within a quarter of the median), turn the wait off for the run; later
waits only count how many refreshes passed, so a late wake-up never moves
the length (`docs/MOTION.md` §3.2). Any error from the call (`ENOTTY` on a
framebuffer without it, a PC's firmware framebuffer; `EBUSY` while
something holds the display), or eight waits in a row that do not fall on
a grid of refreshes (the call returning at once), turns the wait off for
the run and the shell goes on as before on its own timer. A write that
takes longer than a refresh, as the whole of 1920 × 1080 does, still tears
below the line the display is at; the wait times the start of the write,
not its end. `--timings` gives, per block, a `vblank` line (the wait on or
off and why, refreshes seen while frames were written, those that carried
a new frame, those a frame was due on and none carried, and the
percentage that carried), a `refresh` line (the measured length in ms, and the
least and most of the gaps it is the median of) and a `vb_write` line (count, mean and worst
of the time from the vblank to the write's end). "X% of refreshes during
motion carried a new frame" is `carried_pct` when the grid's step is one
frame; at a step of two it is about 50 by design, and `missed` is the
number to read.

**Which CRTC the wait follows** *(Open: the wait does not follow the screen on the Pi)*. `drm_fb_helper_ioctl` waits on the first CRTC of the device
only, and when that CRTC is not enabled it returns 0 at once; the first
CRTC is the first one vc4 created (`drm_for_each_crtc`). vc4 binds the
TXP, the video core's writeback block, before the pixelvalves, and
`vc4-kms-v3d` turns it on, so the first CRTC is the writeback's and a
wait on the panel returns at once, which is what the `vblank` line
reported (the waits returned at once, three times). The image leaves the
writeback block on: an overlay that turned it off was tried, and the Pi
with it showed no picture on HDMI or on the panel and wrote no boot
report, while the same card without it booted to the app on both. The
cause is not known. So the first CRTC on the Pi is the TXP, the vsync
wait does not follow the screen, and the shell's own timer paces the
frames; this is open. Nothing in Faraday uses writeback.

An earlier run with the writeback off, a pixelvalve as the first CRTC, held a
measured 16.68 ms on the 2.8-inch panel at the 22 MHz pixel clock and
showed no tearing to the eye; that is the result to reach again.

**A window on a larger screen** *(Built; its figure on a card is not yet
taken)*. `faraday.window=WxH` on the kernel command line draws in a
centred window of that size on a larger screen, so fewer pixels are
converted and written. The black around it is written once at the start
(`Panel::window_in`) and never again; every frame, band, moved frame and
the arrow is written at the window's byte offset in the framebuffer, with
the screen's stride, so a frame in a window is written a row at a time. A
window that is empty or does not fit is ignored and the whole screen is
drawn, so the 480 × 640 panel is as before; it is never used with
`--size` or a file standing in for a framebuffer. The window is the
screen as far as the page is concerned: its pixels decide the layout, the
orientation and the density (the screen's EDID size is not used, since it
is the whole screen's), so the page lays out as on a 1366 × 768 screen.
A touchscreen covers the whole screen: its touch is placed on the screen
and then in the window, a press in the black is no touch, and a finger
dragged out of the window is held at its edge. A mouse's arrow stays in
the window. The border is not repainted after a console draws on it.

**What a look for input devices costs.** On a Pi 3B+ with 8 input nodes,
opening and closing each node at each 2-second look stalled the main loop
450 to 630 ms. Not reopening a node a reader holds brought the worst to
240 to 330 ms per look, and opening only the nodes of devices that are
candidates is what the stick's look now adds. The cause, reasoned and not measured separately, is that
closing an evdev node waits for an RCU grace period in the kernel.
With it, the worst look in a five-second block has a median of 7 ms.

The look is then moved off the main loop: after the first one at start
(done in the loop, so a device plugged in at boot is found, and the
notice listing it written, as before) the later looks run on a thread
started by the first, which hands what it found back for the loop to
take on a later pass, with the same rules for what it opens and the
same trust decision, made when the loop takes the result. At most one
look is out at a time. A thread that ends without answering panics the
loop. The `rescan` part in `--timings` is then the cost of asking and
taking; a card with it is not yet taken. The readers close their own
nodes on their own threads, so the loop closes none.
`--timings` also gives a `moved` and a `whole` line and a `by_screen`
table per block, so a frame mean can be told by what was drawn.

**The calls to the disk process.** Where the image runs a disk process,
the loop made three kinds of call to it, each waiting on a stick: the
look for plugged-in sticks (`sticks` in `--timings`), the timings file's
write (`report`) and the boot report's composing and write (`bootreport`).
On a Pi 3B+ (a card of the panel run, release) their worst figures in a
block were 17, 31 and 50 ms, each held once in a while a frame of the
loop. They now run on one thread of its own (`diskwork.rs`), which the
first of them starts and `Work::finish` ends and joins. The disk
process's handle is shared under a lock, held by the thread for one call
at a time. The loop's pass over the app's file requests takes the same
lock: with a request waiting it waits for a call that is under way and,
the lock not being fair, for the jobs already queued for it, three
calls at most; with none waiting it only tries the lock, and a
busy disk skips that pass's serving, which the next pass makes.
The thread holds the list of sticks and the text of the two diagnostic
files, and nothing of the app, a key or a vault. A look is asked on
the pass it is due and its answer taken on a later pass, then told to the
app as it was (a stick plugged in shows on the next frame after the
look finds it); the first look at start is made in the loop. The boot
report is composed from a copy of what the loop has seen (the kernel log
is read on the thread) and the timings' text is cut by the loop; a write
asked for while the last of its kind is out is made at the next pass.
At exit the loop ends the thread, which finishes the write in progress,
before it makes its own last writes, so the files end as the last text
the loop made them. The panic chord does not wait for it: a write cut off
there is the disk process's, which writes a new file and renames it over
the old. A thread that ends without answering panics the loop. The three
parts keep their names and now count what the loop still spends: asking
and taking. A card with the figures is not yet taken; the writes and the
look need a disk process, so they were not run off a Pi.

## 7. What a frame does beside its pixels

§1's floor is most of a frame on the screens with long lists: 69 % of
Vault contents and 78 % of Files on the Pi's panel. None of §3 to §6
removes it. It was found with a probe kept outside the repository (the session of
SPEED §7 at 480 × 640, release, counting the characters measured and
timing the parts of Files by hand; the build machine has no sampling
profiler).

**Found and fixed.**

- **Files: the open vault's card, 90 % of the frame.** The card asked for
  the vault's rows and for its wallets with seeds, and each wrote the
  descriptor of every wallet of the session out again to compare it with
  each wallet record (`same_wallet`, a descriptor and its checksum): the
  records times the session's wallets, twice a frame. A wallet now keeps
  that descriptor (`Wallet::same`, dropped with its policy) and the
  comparisons in the vault code read it. Files drew in about a third of
  its time; the card's two asks went from 1.35 and 1.40 ms to 0.01 and
  0.12 ms.
- **Vault contents: the signing page's file texts.** In the heaviest
  session this screen is the page that signs the Inbox's files, which
  wraps each small text file (up to 2 KiB) at every frame: 64 000
  characters measured, each by a binary search of the outline file.
  `Outlines::glyph_in_file` now finds the characters of the first block
  by their offset (one read; same glyph), and `text::wrap` looks the
  printable ASCII advances up once for a text of 160 bytes or more.
  Characters measured fell to 43 000 a frame and the frame to about
  half; the advance of a character fell from 28 to 16 ns.
- **The signing page's lines kept by file.** The page wrapped each small
  text file at every frame: 335 µs of Vault contents' 885 µs floor on the
  build machine. The lines are now kept on the vault state with the
  text, the font, the scale and the width in pixels they were broken
  from (`gpg::SignWrap`, `Ui::wrap_basis`), used only while all four are
  the same and dropped when the files on the page no longer hold the
  text; the floor fell to 802 µs and the wrapping to 3 µs, and 48 fewer
  allocations a frame.
- **A face kept per size.** Each `Font::sized` looked the face up by a
  search of the font list and parsed its header, 100 to 1 100 times a
  frame. The `Ui` now keeps the face of each font at the canvas's scale
  the first time a frame asks for it (`Ui::face`); the key holds the
  font and the scale, so a change of either reaches another entry.
  A probe outside the repository (release, 480 × 640 class,
  160 dpi): a face went from 32 to 8 ns a call, the lookup now a short
  search of the faces kept.
- **`Ui::wrap_lines` linear in the words.** It rebuilt and measured the
  whole candidate line for each word, so a long label cost the square of
  its words. It now adds the advances of the words and the spaces,
  which is all the measure does (no kerning), and rounds the sum once
  as the measure does; the count is the same, held by a test against a
  measure of each whole line. The same probe, a label of 130 words
  at 300 units: 115 µs a call before, 15 µs after.
- **Home and Wallets: the entries of every wallet worked out to count
  or to say one line.** Home's Backups tile counts the wallets known and
  those with no map, and a wallet's card says one line of its own map;
  both asked `View::backups()`, a line per place for every wallet of the
  session, of each open vault and of each locked vault's summary (the
  session's 12 wallets and more: about 700 of Home's 877 allocations and
  377 µs of its 486 µs floor on a Pi 3B+, in a probe outside the
  repository with each frame under a one-row clip). The count is now
  `Faraday::backup_counts_now`, which asks of each wallet only whether it
  has a map (a record of its backup, or a plan an open vault keeps for
  it, or a locked vault's summary naming it as planned) and never builds
  the lines; the card's line is worked out from that wallet's map
  alone (`backup_map_of`). Both are worked out from the state as it
  is, so nothing is kept and nothing can be stale; a test holds the
  count equal to the entries of `backups_now` without lines, and Home
  and the Wallets card are read after a wallet is loaded, a vault
  opens, a plan is made and a vault is locked. At 480 x 640 a frame
  drawn again made 908 allocations on Home and 997 on Wallets before,
  and 670 and 391 after (the budget test's walk).

- **The Backups screen's list and the wallet chart kept across moved
  frames.** Both were worked out at every frame (`backups_now`, a line per
  place for every wallet, and `glance::build`). They are now kept on the
  state (`drawing/kept.rs`, `Kept::charts_frame`) only while the frames
  drawn are moved ones: a frame that is asked for whole empties them
  first, and a moved frame (a scroll or a wheel step, which write only the
  region's offset) reads what the last frame worked out. The inputs
  (the session's wallets and keys, the flows, the backup under way, the
  backups area, open vaults' records, locked vaults' summaries, the
  Outbox, the last visit's receipt, the medium and time zone) are plain
  fields written from many places, and none of them is watched one by
  one: any change to them comes from an event, a tick that does more than
  move the region, a storage answer or a job's answer, each of which asks
  for a whole frame (`Out::redraw`), so the first whole frame after it
  empties what was kept. A moved frame never writes any of them. The
  lock empties them as well (`lock_finish`). Nothing is kept outside a
  frame being drawn (a view made by a test or by a decision reads
  nothing kept). A debug build works the value out again at every hit and
  compares (a panic names the list that went stale), so the whole test
  suite checks it at every moved frame it draws; two tests scroll the
  Backups screen and a wallet's page, then plan a backup and check a copy,
  and show the screen follows. What is newly kept: the wallet chart
  (names, shape, descriptor checksum, key fingerprints and labels, the
  plan's answers and places' names) and the backup list (names, shapes,
  checksums, the lines' texts); the same public text the frame already
  held until it ended, now until the next whole frame or a lock. No key,
  seed, passphrase, descriptor text or file path. Measured on the build
  machine, release, 480 x 640 (the Pi's panel class, 286 dpi), the heavy
  session of SPEED §7, the median of 41 moved frames (a wheel step and the
  frame): Wallets 1 998 µs before, 1 394 µs after; Backups 1 548 µs
  before, 1 057 µs after; Home, which uses neither, 1 631 and 1 632 µs.
  The chart's work moved off no worker: it is under the 100 ms line and
  needs none.

Pixels are as before (the snapshot and budget tests pass).

**Not yet found or fixed.**

- The signing page (the spend of the session's test PSBT) has a floor of
  125 µs, 38 µs of it in its page, against 1.2 ms with its pixels: its
  wrapping is not what the floor holds here, so keeping its lines by file
  would save at most that 38 µs and is left open.
- Files, what is left: the vault card is still measured and then drawn
  (two walks of its rows); its wallets with seeds ask the vault's rows a
  second time inside the accessor; the Inbox's card about a fifth.
- The wallet's page's chart, 387 of its 657 µs floor on a whole frame,
  is built once per run of moved frames (above); a whole frame still
  builds it. Wallets: 274 µs, of which the chart 52 and the backup line 26.
  Files: the head 117 µs (73 of it the vault's wallets with seeds), the
  Inbox's card 34, the vault card measured and drawn 31. Vault contents:
  the file rows' text drawn outside the one-row clip is still most of
  what is left after the wrapping. Measured on the build machine,
  release, 480 x 640, a one-row clip, the median of 41 frames, with
  section timers in a probe kept outside the repository.
- **The first whole frame of a screen change** (read from the Pi logs of
  the panel run, then timed on the build machine, release, 480 x 640 at
  286 dpi, the app driven through `App::frame` after the press that opens
  the screen, with a probe kept outside the repository). The slowest passes
  on the panel were Home's Lock sheet (98 ms), BackupFlow (up to 62),
  Home's Power sheet (58), Pull (55), Key lengths (49), Cancel flow (47),
  and Settings, Wallets start and Visit (31 to 35). A screen's first frame
  is no dearer than a later whole frame of it: Settings 35 ms on the Pi
  with the screen unchanged, 32 ms when changed, and on the build machine
  0.8 ms for Settings, 0.7 for Wallets start, 0.3 to 0.4 for Wallets, Visit
  and Backups, 1.0 to 1.2 for BackupFlow, the same on the 2nd to 4th time
  the screen is opened (only the first open in a process is dearer, 2.3
  ms for Settings, from the glyphs and faces not yet kept). Nothing was
  found that a screen works out again on its first frame, so no work is
  kept or prepared ahead for these. A sheet is 2.0 to 3.5 ms where its
  page is 0.3 to 0.9: 1.0 ms of it is the frost under the sheet
  (`Canvas::frost`, a barrier in the recording, so it runs after the page is
  painted and before the sheet is, on the main thread and not in bands), 0.9
  ms the page painted under it. The frost is kept after the first frame
  (`Ui::backdrop`), so it is one hitch at the sheet's opening. Its cost is
  in proportion to the pixels at a block under 16 (scale 14 on this panel:
  977 µs; scale 28, which reads a quarter of the pixels: 374 µs); the sum
  of each block in two-lane words did not change it (1 039 against 1 040 µs
  a frost), and a quarter-reading at a block under 16 would change the
  pixels, so neither was taken. The frost run in bands across the cores
  was built and measured and not kept: its two stages that touch every
  pixel (the blocks' sums, the enlarging) split over the bands' rows with
  bytes equal to the whole frost at 2, 3 and 4 bands, on opaque and
  translucent pages, at block scales 1 to 33, but each stage is a round
  of threads started and joined, 75 µs for 2 and 156 µs for 4 on the
  build machine (release), against a frost of 565 µs. Frost alone, median
  of 41: 565 µs whole, 508 at 2 bands, 623 at 4. The first frame of a
  sheet at 480 x 640, median of 21: Lock 941 µs whole, 958 at 2 bands,
  1 262 at 4; Power 5 963, 5 487, 5 328; Pull 4 929, 4 726, 5 086 (the
  frame's other costs move more than the frost's share). Doing both
  stages in one round of threads would save at most one such round, 75 to
  156 µs, of a frame of 1 to 6 ms, and a Pi starts threads at a higher
  share of its frame, so it was left. What a Pi does with it can only be
  read on a Pi.

## 8. Order of building

The content placed in whole pixels and `Ui::region` (§3.2, built) come
before the moved frame, and the bands (§4) need the regions drawn through
`Ui::region`. They are best measured after the moved frame is in. The
Pi's frame written as it is (§5, built) and the framebuffer write (§6
points 1 to 3, built) need nothing before them. The shell after a moved
frame (§3.7) and §6 point 4 need both the moved frame and the Pi's frame
written as it is. Opting
more screens in to the moved frame does not wait for a figure on a Pi: the
owner decides, and the gain on a Pi is still to be read on a dev card (§3.5).
None of this touches keys, cryptography or what the device takes in.

What each piece changes:

- **Whole-pixel content and regions** (built): `faraday/faraday-ui/src/lib.rs`,
  `sheet.rs`, `menu.rs`, the core files that report a region,
  `faraday-starter/src/draw.rs`, `core/faraday-draw/src/canvas.rs` and
  `docs/MOTION.md` §3.3. The snapshots are rendered again where an edge
  moved, each named; the clipped-shapes test covers bands through a shape,
  a glyph and a shadow.
- **The moved frame** (built for the compact Settings page): `moved.rs`,
  `lib.rs`, `motion.rs`, the screens that opt in, `faraday-ui/src/lib.rs`,
  `core/faraday-shell-api/src/lib.rs`, `docs/MOTION.md`, `docs/SPEED.md`
  §7; the two tests of §3.2 and §3.4; a scroll step of each screen that
  opts in is timed as SPEED §7 says (§3.5).
- **The Pi's frame written as it is** (§5, built):
  `faraday/image/boards/pi3/linux-patches/` (the third patch),
  `faraday/shells/stick/src/fb.rs`, `present.rs`, `cursor.rs`,
  `docs/IMAGES.md` §3.2 and the stick shell's module docs; the present
  tests at 32 bits red first and the framebuffer test for `vc4drmfb`; the
  figure on a card is taken later with `--timings`.
- **The framebuffer write** (§6 points 1 to 3 built; point 4 not):
  `present.rs`, `panel.rs`, `fb.rs`, `docs/SPEED.md` §11.1; the
  `present::tests` for a caret and a clock as two bands, the panel after
  each change with the arrow across a band and moved away, and turned
  frames. The shell taking the moved frame (built) added the stick's `main.rs`
  (one call, taking `Parts` for `frame`) and `present::tests` for a moved
  frame (only the strip and the overlays converted, the panel showing the
  frame); its figure on the dev card with `--timings` is not yet taken.
  Point 4 is not built.
- **The bands** (§4, not built): `core/faraday-draw/src/canvas.rs`,
  `canvas/list.rs` (new), `faraday-core/src/bands.rs` (new), `lib.rs`
  (`render_frame`), `docs/WORKERS.md` (rule 1 of its §2 worded for the
  bands), `docs/SPEED.md`; the test of §4.6, the heat on the dev card
  (§4.5), and a frame of each main screen drawn whole timed as SPEED §7
  says.
- **Beside the pixels** (§7): the face kept per size and `Ui::wrap_lines`
  (built) in `faraday/faraday-ui/src/lib.rs` and `roles.rs`; the rest of
  §7 is not built.

## 9. For the owner

1. **The bands (§4) are built, on a measurement.** On a machine whose
   whole frame takes about a millisecond they cost more than they save,
   5 to 30 percent, about the same time on every screen. On the Pi 3B+,
   whose whole frame takes tens of milliseconds, one pair of runs read a
   mean frame of 9.5 ms against 13.1 and a slowest five seconds of 16.5 ms
   against 36.0. A scroll step gains nothing: it makes no threads.
2. **`mmap` for the framebuffer** (§6) is decided, not taken: it is the
   owner's decision, for the reason in §6. It is no longer open.
3. **Snapshots moved by a pixel** (§3.2). Placing a region's content in
   whole pixels moves a few edges by one pixel, once. Recommendation:
   accept; the tours that changed were named when it was built.
