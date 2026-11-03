# Frames at the panel's rate

**Kind:** Spec
**Status:** Specified; the first step of §3 is built (`Ui::region`, the whole-pixel offset, the pages and panes laid out from offset 0 and the content layer, batch FR1; the face kept per size and `Ui::wrap_lines` linear in its words of §7, batch FR6a), the rest is not
**Open:** the rest of it: the moved frame itself, with the regions still laid out at their offset (the Board's list, the codex32 dice sheet and worksheet, Learn's body, the wide Family tab and the wide Files page's head, which draw in pixels of their own or in a scrolled place that depends on the offset; `Region::from_zero` is on every other), the content layer taken only when a frame asks for it (`Ui::keep_content`), the stretch moving the layer, a scroll step that moves the rows already drawn (§3), the frame drawn in bands across the cores (§4), the Pi's panel taking the core's frame without a conversion (§5), the framebuffer write (§6), what a frame does beside its pixels that §7 has not yet found or fixed (batch FR6b)

The Pi's frame grid runs at 15 Hz while a finger drags, and at 30 Hz
once its clock follows the work (`docs/MOTION.md` §3.2); the aim is
60 Hz, a frame and its way to the panel inside 16.7 ms. This spec
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
3.4 ms. The firmware's framebuffer there is 32 bits a pixel (the boot
report's `panel` line) and the firmware gives the kernel
`bcm2708_fb.fbswap=1`.

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
the scrollbar. Its cost is a copy of the view (about 20 µs of a 1.2 ms
frame on the build machine at the Pi's size). The copy is taken only
while `Ui::keep_content` is on, which the moved frame (FR2) turns on;
until then nothing reads the layer and a frame copies nothing.

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
  today's snapshots, once; the batch that makes the change (FR1)
  renders the tours again and says which changed and why.
- **A clip at a row cuts nothing inside it.** A shape cut at the clip's
  top or bottom is filled into the clip's rows (MOTION §3.4), and
  tiny-skia's coverage of a pixel depends on the path and the pixel's
  own rows only; text and icons blit whole glyphs cut at the clip. The
  existing pixel-equality test of clipped shapes in `canvas.rs` is
  extended to bands cut through a shape, a glyph and a shadow.
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
  step column. A screen that has not opted in draws whole, as today.

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
- The list is painted by `n` bands of rows, each into its own rows of
  the pixmap (disjoint `&mut [u8]`), each band's calls in the order
  recorded, each held to its rows as a clip. A pixel's value depends
  only on the calls that touch it, in their order, so each band paints
  the bytes one band would.
- **Barriers.** A call that reads pixels other rows wrote ends a run:
  `shift` (the stretch), `frost` (the blur under a sheet, once a
  sheet), `snapshot` and `copy_region`, a shadow blurred where it is
  drawn. At a barrier every band finishes the run before it, the call
  runs on the main thread over the whole canvas, and the bands go on.
  `restore`, `blend_*`, `fade_region` and `luma` read and write the same
  pixel only and stay banded.
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

- Made with the canvas in faraday-core (faraday-draw is `no_std`; the
  bands' painting is there and the threads are not): `n − 1` threads,
  parked between frames and woken once a frame, with the main thread
  as the `n`th. No thread is made per frame (a thread's start costs
  tens of microseconds on a Pi, a sizeable part of a strip).
- `n` from the machine: `std::thread::available_parallelism()`, at most
  four, one band for each 64 rows at least, and one (no pool, painted
  as now) when it says one or the canvas is under 64 rows a band.
- No new dependency: `std::thread`, a `Condvar` and a counter.

### 4.4 Beside the core's worker

The bands are not the worker of `docs/WORKERS.md`. The worker runs
costly work (a stretch, a signature, a picture) on its own thread while
frames go on; the bands are the drawing itself, started and joined
inside one frame, so the main thread still draws and nothing else
happens while it does (WORKERS §2 rule 1, which FR5 words to say the
bands are part of the drawing). They hold no `Faraday`, no `View` and
no token: a list of calls, the caches read-only and their own rows.
Drawing still reads (SPEED §3.3): what the screens do is the
recording, as now; the bands only paint.

### 4.5 The Pi's heat

The Pi 3B+'s firmware lowers its clock from 1400 to 1200 MHz above its
soft limit of 60 °C, and further near 85 °C. Four cores busy is a frame
drawn whole, a few milliseconds at a time: a change of screen, not a
scroll (§3). The dev card read 31 °C at rest. The batch that builds the
bands (FR5) reads the boot report's temperature line after two minutes
of dragging and changing screens on the dev card, with and without
bands, and says it in its report.

### 4.6 Held by

`list::tests::a_frame_painted_in_bands_is_the_frame_painted_once`:
every main screen and heavy sheet, the Pi's panel and a desktop, a
sheet over a page (a barrier), a stretch (a barrier), painted with 1,
2, 3, 4 and 7 bands, byte-equal. The build machine's snapshot tests
run with its own count of bands, so they check it too.

## 5. The Pi's panel takes the core's frame as it is

The core's frame is premultiplied RGBA, red first in memory; drawn over
an opaque page, its alpha is 255 everywhere. The Pi's firmware
framebuffer is 32 bits a pixel, and with `bcm2708_fb.fbswap=1` (which
the firmware puts on the kernel's command line) the kernel's driver
lays out its 24 and 32-bit modes red first too, at bit offset 0. So the
two already agree, and the conversion is a reordering that the image
then undoes:

- The shell reads 32 bits as `Depth::Bgra8888`, blue first, whatever the
  driver says (`fb::Depth::from_bits`; the bitfields are read only for
  24 bits, from sysfs files the fbdev driver does not publish).
- The colours' order is the pixel format of the panel's display overlay
  line (`faraday/image/panels/any/screens/waveshare-28dpi/display.line`,
  `rgb666-padhi`), on the kernel's DPI driver; it is worked out again on
  the panel (`docs/IMAGES.md` §3.3). On the firmware's framebuffer the
  image set `dpi_output_format=0x7F206` where Waveshare's documentation
  gives `0x7F216`: the two differ in the DPI output's colour order, and
  the firmware's DPI documentation names orders 1 to 4 (RGB, BGR, GRB,
  BRG), not 0.

The change (FR3):

1. `Depth::Rgba8888`, red first. The shell takes it for 32 bits when
   `/sys/module/bcm2708_fb/parameters/fbswap` reads 1 (the driver's
   parameter, readable in sysfs), and keeps `Bgra8888` otherwise, so a
   PC's firmware framebuffer (blue first) is unchanged. The dev card
   confirms both the file and the colours before the batch lands.
2. The panel's `config.txt` goes back to Waveshare's `0x7F216`, and
   `framebuffer_ignore_alpha=1` so the firmware never reads the alpha
   byte; `faraday/image/README.md`'s line on red and blue exchanged
   says so.
3. For `Rgba8888` at no turn, the `Presenter` keeps no converted copy:
   the panel is written from the kept frame itself, and the panel's own
   kept copy is that frame (§6), so one comparison and one copy stand
   where a comparison, a conversion and three copies stood.

On the build machine's figures a pass's `convert` falls from about 166
µs (comparison, conversion, copy) to about 37 (comparison, copy), so on
the Pi from about 5 ms to about 1.1 at its lowest clock and about half
that at full clock. A frame turned a quarter is still walked tile by
tile (`fb::convert_turned`), as now.

Not taken: a 16-bit framebuffer (`framebuffer_depth=16`) halves the
bytes written but needs a conversion again and shows five bits of red
and blue on a panel wired for six. The PC stick's conversion stays (its
firmware framebuffer is blue first and the canvas paints red first): a
word-at-a-time swap measured no quicker than the byte loop (134 µs
against 129); §3.7 is its gain.

## 6. The framebuffer write

What a write does now (`panel.rs`): for each band of 64 rows under the
lock, it compares the rows with its own kept copy (`shown`) a second
time, copies the rows into a band buffer, draws the arrow in where it
falls, and calls `pwrite`; the kernel's fbdev write copies the bytes
in from the program a page at a time and then into the framebuffer.
It writes only the rows from the first to the last that changed
(SPEED §11.1). After a scroll step that is the whole view, and must be
(§3.7).

The change (FR4):

1. **One comparison.** The presenter knows which rows changed; it hands
   the panel those bands, and the panel writes them without comparing
   again (`Panel::write_known`), keeping `shown` in step. The panel's
   own comparison stays for `write_rows`' other caller, the arrow.
2. **No band copy where the arrow is not.** A band the arrow does not
   cross is written from the frame directly; only the arrow's rows are
   copied to draw it in.
3. **Bands, not a span** (§3.7): a caret and the clock are two writes,
   not the rows between them.
4. With §5 the Pi's `shown` is the presenter's kept frame, one buffer
   for both, moved under the lock.

A write of the whole view then crosses the memory twice in the kernel,
where it crossed three times and was compared twice. The Pi's write is
bounded by its memory bus, which does not follow the CPU's clock, so
it is estimated, not run: about 2 ms for a whole 480 × 640 frame,
measured on the dev card by the batch.

Not taken: writing the framebuffer through `mmap` would save the
kernel's two copies, but needs `unsafe` (or a dependency that holds
it), which the workspace forbids but for one crate; it is the owner's
call (§9 question 2), and is not needed if §5 and points 1 to 3 bring
the pass inside a refresh.

## 7. What a frame does beside its pixels

§1's floor is most of a frame on the screens with long lists: 69 % of
Vault contents and 78 % of Files on the Pi's panel. None of §3 to §6
removes it. FR0 found what it is (a probe kept outside the repository,
the session of SPEED §7 at 480 × 640, release, counting the characters
measured and timing the parts of Files by hand; the build machine has no
sampling profiler).

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

Pixels are as before (the snapshot and budget tests pass).

**Not yet found or fixed.**

- Home (46 % floor), Wallets (58 %) and a wallet's page (55 %) cost
  about 1 ms on the build machine with 400 to 700 characters measured
  and 1 000 to 2 000 allocations: neither is most of it. The part is
  not yet named; it needs a sampling profile (`perf` on the build
  machine) or section timers as for Files.
- Files, what is left: the vault card is still measured and then drawn
  (two walks of its rows); its wallets with seeds ask the vault's rows a
  second time inside the accessor; the Inbox's card about a fifth.
- The signing page still wraps every text at every frame; the lines
  kept on the state, by file, would remove it.

## 8. Batches

Each about fifty agent turns. FR0, FR1 to FR3 and FR5 leave
`faraday/shells/stick/src/main.rs` alone; FR4 touches it in one place
(taking `Parts` for `frame`), after WORKERS' W2 has landed.

| Batch | What | Files | Review | Status |
| --- | --- | --- | --- | --- |
| FR0 | What the floor is (§7): the Vault contents and Files frames timed part by part at a one-row clip, with a probe kept outside the repository; what costs named here, with the step that removes it | `docs/FRAMES.md` | No | Built |
| FR1 | Content placed in whole pixels (`Ui::oy_px`), `Ui::region` in place of each clip and `report_scroll*` pair, the content layer taken and the stretch moving it; snapshots rendered again where an edge moved, each named in the report; the clipped-shapes test extended to bands through a shape, a glyph and a shadow | `faraday/faraday-ui/src/lib.rs`, `sheet.rs`, `menu.rs`, the 28 core files that report a region, `faraday-starter/src/draw.rs`, `core/faraday-draw/src/canvas.rs`, `docs/MOTION.md` §3.3 | No; a screen review of the tours that changed against Material's Scrolling | Built |
| FR2 | The moved frame: `moved.rs`, `out.redraw()` and `out.moved()`, `Moves::Yes` on the long screens, `draw_whole_next`, the two tests of §3.2 and §3.4; `Parts::moved` and its composition; timed as SPEED §7 says, a scroll step of each screen that opted in | `faraday-core/src/moved.rs` (new), `lib.rs`, `motion.rs`, the screens that opt in, `faraday-ui/src/lib.rs`, `core/faraday-shell-api/src/lib.rs`, `docs/MOTION.md`, `docs/SPEED.md` §7 | No | Open |
| FR3 | The Pi's panel in its own order (§5): `Depth::Rgba8888` from the driver's `fbswap`, the Presenter with no converted copy for it, the panel's `config.txt` and the README's colour line; the present tests at 32 bits in both orders; checked on the dev card | `faraday/shells/stick/src/fb.rs`, `present.rs`, `panel.rs`, `faraday/image/panels/any/screens/waveshare-28dpi/display.line`, `faraday/image/README.md`, `docs/HARDWARE.md` if it names the format | No | Open |
| FR4 | The shell after a moved frame (§3.7) and the write (§6): the Presenter moving its copies by `Parts::moved`, `changed_rows` as bands, `Panel::write_known`, no band copy outside the arrow, one buffer for `shown` and `last`; the stick takes `Parts`; `present::tests` for a moved frame (only the strip and the overlays converted, the panel showing the frame) and for a caret and a clock as two bands; measured on the dev card with `--timings` | `faraday/shells/stick/src/present.rs`, `panel.rs`, `fb.rs`, `main.rs` (one call), `docs/SPEED.md` §11.1 | No | Open |
| FR5 | The bands (§4): the recording mode, the barriers, the pool, the test of §4.6; WORKERS §2 rule 1 worded for the bands; the heat on the dev card (§4.5); timed as SPEED §7 says, a frame of each main screen drawn whole | `core/faraday-draw/src/canvas.rs`, `canvas/list.rs` (new), `faraday-core/src/bands.rs` (new), `lib.rs` (`render_frame`), `docs/WORKERS.md`, `docs/SPEED.md` | No | Open |
| FR6a | Two costs in the drawing path (§7): the face kept per size on the `Ui`, `Ui::wrap_lines` linear in its words | `faraday/faraday-ui/src/lib.rs`, `roles.rs`, `docs/FRAMES.md` | No | Built |
| FR6b | The rest of §7 (what is not yet found or fixed); this spec's plan moves to docs/history/FRAMES-LOG.md and it becomes a Reference | as FR0 says, `docs/FRAMES.md`, `docs/README.md` | No | Open |

FR1 comes before FR2; FR3 and FR0 can run at any time and in parallel
with FR1; FR4 needs FR2 and FR3; FR5 needs FR1 (the regions it bands
are drawn through `Ui::region`) and is best measured after FR2. None of
them touches keys, cryptography or what the device takes in.

## 9. For the owner

1. **The bands (§4) after the rest, on a measurement.** They give 1.2 to
   3 times on a frame drawn whole and next to nothing on a scroll step
   once §3 is in, at the cost of a recording mode for every painting
   call. Recommendation: build them, last (FR5), after FR2 and FR4 are
   measured on the Pi; if a change of screen on the Pi then comes in
   inside two refreshes, ask again before FR5.
2. **`mmap` for the framebuffer** (§6). Recommendation: no; the write
   after FR4 is the kernel's two copies of the rows that changed, and
   `unsafe` in the stick shell is a larger change than the saving.
3. **Snapshots moved by a pixel** (§3.2). Placing a region's content in
   whole pixels moves a few edges by one pixel, once. Recommendation:
   accept; FR1's report names each tour that changed.
