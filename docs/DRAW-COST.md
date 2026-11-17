# Cheaper drawing: what is left

**Kind:** Spec
**Status:** Specified; none of it is built and none of it is timed. Found by reading the code on 2026-11-17, after the work of `docs/SPEED.md` and `docs/FRAMES.md`.
**Open:** all of it: no change below is built, and every figure is a forecast reasoned from the code and from the measurements of `docs/FRAMES.md` §1 and §7, to be timed as `docs/SPEED.md` §7 says before a change is made

`docs/SPEED.md` holds what keeps costly work out of drawing and
`docs/FRAMES.md` how a frame reaches the panel at its refresh rate. This
document holds what a read of the drawing code found after both: where a
frame still spends time it need not, what each change would give, and what
was looked at and is not worth building. The rules of `docs/FRAMES.md` §2
hold for every change here: the same pixels, a fallback that is always
right, drawing reads, nothing tuned to one machine.

## 1. What to expect

No change left doubles the speed of every frame. A frame drawn whole can
become 1.1 to 1.5 times quicker, depending on the screen; a few kinds of
frame can become ten times cheaper or more.

- **A caret's blink, a clock's second, a tick while a job runs.** Each is a
  whole frame now: about 13 ms on Home and up to about 37 ms on Files on a
  Pi 3B+ at full clock (`docs/FRAMES.md` §1). Repainting only what changed
  (§3) would make each 1 to 2 ms.
- **A whole frame of a screen with long lists** (Files, Vault contents,
  Wallets): 1.2 to 1.5 times quicker, from §2.
- **A whole frame of any other screen:** 1.1 to 1.2 times quicker, from §5
  to §7.
- **A scroll step** costs what the frame does beside its pixels and a
  strip, so §2 is what shortens it, and §3 keeps it a moved frame in more
  cases.
- **A scroll step in Faraday Demo in a large window:** 1.2 to 1.4 times
  quicker, from §4.
- **A fade's step on the stick:** one comparison of the whole frame less,
  0.5 to 1 ms on the Pi, from §4.
- **A frame on a panel turned a quarter or a half:** its conversion 3 to 4
  times quicker, from §4.

For smooth scrolling on the Pi, §2 comes first: Files and Vault contents
miss a refresh on the work beside their pixels alone (`docs/FRAMES.md`
§3.5). Then §3's cases where a scroll step falls back to a whole frame,
and the refresh wait that does not follow the screen on the Pi
(`docs/FRAMES.md` §6, open there), which is what tearing and pacing depend
on. §5 to §7 shorten a frame drawn whole (a change of screen, a sheet's
first frame) and do nothing for a scroll step.

## 2. Lists worked out more than once a frame

What a frame does beside its pixels is 46 to 78 per cent of it
(`docs/FRAMES.md` §1). Most of what is left of that is a list built again
on every call, reached through `View`'s deref to `Faraday` and so past the
drawing memo (`docs/SPEED.md` §3.3). Each fits the memo as `backups()`
does (`faraday/faraday-core/src/drawing/memo.rs`): filled once in a frame,
gone when the frame ends, so nothing kept can go stale and a decision
still works from the state as it is.

- **`vault_files()`** (`faraday/faraday-core/src/vault/mod.rs`) builds its
  list on every call, copying every item's facts, and removes doubles by
  comparing each pair (`vault/area.rs`, `same_vault_files`,
  `summary_of_file`). It has 57 call sites; Home, Vault contents and Files
  call it several times a frame.
- **`vault_wallets_with_keys`** (`drawing/kept.rs`) keeps nothing and walks
  the vault's rows again (`vault_rows_now`) for each open vault, twice a
  frame on Files. This is the item `docs/FRAMES.md` §7 leaves open under
  Files.
- **`loose_keys()`** (`faraday/faraday-core/src/wallet.rs`) asks for a
  wallet's slots, a new list, for each key and each wallet; Wallets and
  three other screens call it. The fingerprints in use, gathered once,
  answer the same question.
- **Home** (`faraday/faraday-core/src/screens/home.rs`): `home_jobs` builds
  the lead of every rule and `home_job` keeps the first; `home_ways` makes
  new strings of subtitles that are fixed text; `tiles` measures each label
  twice. This is most of Home's allocations a frame. The rules are tried in
  the same order, stopping at the first that holds.
- **A list row out of sight** is measured about four times before it is
  passed over (`faraday/faraday-ui/src/roles.rs`, `list_item_as` and
  `list_item_height`, which each ask for the row's lines). The height stays
  exact, since the scroll's extent is worked out from it.
- **A file card** is measured and then drawn, two walks of its wraps and
  two lists made (`faraday/faraday-core/src/screens/files.rs`); `docs/FRAMES.md`
  §7 names it.
- **Small things made each frame:** the date as text for the wallet chart,
  the lists of hits, reach and tips in `render_frame`.

The budget tests count a frame's allocations (`docs/SPEED.md` §8.3); the
ceilings there fall with these changes.

## 3. A whole frame drawn where a rectangle changed

A tick sets the frame to be drawn whole (`Out::redraw`) when the caret
blinks, while a job runs and when a clock's second passes
(`faraday/faraday-core/src/lib.rs`, the tick). So a focused field draws the
whole screen twice a second, a running job at every tick, and Settings, Set
the time, Codes and a vault entry with a one-time code once a second.

A scroll step is a moved frame only when nothing else asked for the frame
(`docs/FRAMES.md` §3.3). It is drawn whole while a sheet is open, a toast
shows, a clock is in view, a job runs, or something is pressed or under
the pointer (`faraday/faraday-core/src/fade.rs`, where the moved frame is
refused). A scroll on a vault entry's page with its code showing is
therefore never a moved frame.

The design: a frame asked for by one of these alone repaints the
rectangle that changed (the field's caret, the clock's text, the busy
mark) over the frame kept from before, and tells the shell that rectangle
in `Parts::changed`. A scroll step with one of them in view is a moved
frame and that rectangle. The caret already tells the moved frame it was
drawn (`moved_drew`). It is held as the moved frame is: a test that draws
each such frame both ways on every screen and compares the bytes, with a
whole frame wherever the rectangle cannot be told. This is new design, not
a tuning; it comes after §2 and is timed first.

## 4. What the core knows about a frame and the shells do not use

- **Faraday Demo's host** gives the window no moved region and no changed
  bands but a fade's (`faraday/faraday-host/src/lib.rs`), so every other
  frame hashes every row of the window (`faraday/shells/window/src/view.rs`,
  `Shown::copy`): at 3840 × 2160 about 33 MB read for a scroll step. With
  the moved region passed on, the window puts the region's rows without
  hashing them and hashes only the rows outside it. `docs/FRAMES.md` §3.6
  says the Demo ignores the moved region; this uses it for the hash alone
  and still moves no rows inside the window's buffer.
- **The stick is given no bands for a fade's step**
  (`faraday/faraday-core/src/fade.rs`, `parts_moved`, which leaves
  `changed` empty; only the host reads `take_changed`). And where the frame
  is written unconverted, on the Pi, the presenter returns before it reads
  the bands (`faraday/shells/stick/src/present.rs`, `present_unconverted`,
  `present_staged`) and compares the whole frame with the panel's kept
  copy. Taking the bands there, as `Panel::write_known` does on the
  converting path, removes that comparison on each step.
- **Bands given beside a moved region are not used** (`present.rs`): the
  frame is then compared whole. The bands serve for the rows the move did
  not bring.
- **A turned frame is converted on the calling thread**, a pixel at a time
  in tiles (`faraday/shells/stick/src/fb.rs`). The panel's rows are disjoint
  slices, so they split across the cores as the unturned conversion does.
- **On a PC the converted frame is copied once more** into the panel's kept
  copy (`faraday/shells/stick/src/panel.rs`).
- **The conversion starts its threads at every frame** (`fb.rs`,
  `convert_keep`, `compare_convert_keep`); see §6.
- **A window buffer of unknown age** takes every row at every frame
  (`view.rs`). `--timings` on a Wayland desktop says whether the age is
  given.

## 5. Shadows

A pixel of a shadow's ring is worked out in floating point, and a corner's
pixel takes a square root in software besides
(`core/faraday-draw/src/canvas/surface.rs`, `canvas.rs`). An elevated card
draws two shadows, and a frame painted in bands draws them in each band
they cross. The part that depends on the column alone and the part that
depends on the row alone can each be worked out once per shadow, so a side
pixel is a lookup and only a corner's pixel takes the root: the same bytes,
and 30 to 50 per cent of a shadow's time. A square root computed another
way would move some pixels and is not proposed. Each shadow also makes a
new empty list for its fall; a kept one serves.

## 6. The bands

- **Bands hold equal rows**, so the band with the most to paint sets the
  frame's time (`core/faraday-draw/src/canvas/list/bands.rs`, `edge`). The
  list holds each call's box when the bands are made; rows shared out by
  what falls in them give 10 to 25 per cent of a whole frame's painting
  where a screen is uneven. Any split gives the same bytes
  (`docs/FRAMES.md` §4.6).
- **Threads are started for each run of calls between barriers and for
  each step of a fade** (`faraday/faraday-core/src/fade/bands.rs`,
  `std::thread::scope`). Threads kept from frame to frame, given the same
  rows, save their starting: 0.1 to 0.3 ms a whole frame on the Pi, a
  guess. `docs/FRAMES.md` §7 measured what a round of threads costs when it
  tried the frost in bands.
- **A band's scratch is zeroed over the canvas's whole width** for the rows
  a shape touched, where only the shape's columns were written (`bands.rs`,
  `clear_scratch`).

## 7. Text and small loops

- **A glyph found in the cache is searched for twice**
  (`core/faraday-draw/src/fonts/mod.rs`, `GlyphCache::glyph`: `contains_key`,
  then `get`). One search, or for ASCII a table by character for each face
  and size, is the largest saving left for each character drawn.
- **A wrapped line's advances are added three times:** by the wrap, by
  `text::width`, and as the line is drawn (`canvas.rs`, `text_wrapped`;
  `Ui::wrap_marked`). The wrap holds the width when it ends a line and can
  return it.
- **`Ui::fit` and `Ui::wrap_lines`** do not use the table of ASCII advances
  that `text::wrap` builds for long text, and `fit` adds the whole string
  before it adds the part that fits.
- **The glyph blit** works a pixel at a time and blends a fully covered
  pixel of an opaque colour as any other (`surface.rs`, `blit_glyph`).
  Writing that pixel as it is gives the same bytes; with the row taken as
  one slice, 10 to 25 per cent of the blit.
- **A frame painted in bands looks each glyph up when the call is recorded
  and again when it is painted** (`canvas.rs`, `text_with_face`). The cache
  must be filled before the bands paint, so only a glyph wholly inside the
  clip could give its advance without the lookup: a small gain for a change
  that is easy to get wrong.
- **`Canvas::text` and `Canvas::icon`** find the face at every call; `Ui`
  keeps it (`docs/FRAMES.md` §7).
- **The clear** writes a pixel at a time (`canvas.rs`); the camera preview
  divides in floating point for each pixel (`surface.rs`, `luma`), on the
  scanner sheet alone; `fade_to_background` blends a channel at a time, on
  a disabled control alone.

## 8. Looked at and not proposed

- **A cache of stroked outlines**, the candidate `docs/SPEED.md` §3.3
  leaves open. It would keep the outline's making and not its filling,
  which is most of a stroke: a few per cent of a frame at most. The
  stroker's own share is timed before anything is built.
- **The core's canvas handed to the panel** to save the Pi's last copy
  (`docs/FRAMES.md` §6 point 4). The core needs its canvas for the next
  moved frame, so the copy would move into the core; 0.1 to 0.3 ms.
- **A second buffer for the write's thread**, so a frame is not held for
  the write before it: another copy of the frame, and not before §4.
- **Least-recently-used eviction in the glyph cache** removes one slow
  frame in a session at the largest sizes (`docs/SPEED.md` §3.8) and
  nothing from any other.

Found tight, with nothing to change: a rectangle's fill, opaque and
translucent; the fade's blend; a QR code's modules; the rounded fill; a
path's put-back outside the clip; the conversion loops; the Pi's
unconverted write; the arrow; the host's hand-over of the app's frame;
glyphs placed on whole pixels with icons kept as glyphs; lines out of sight
measured and not drawn.

## 9. What comes first

§2 needs nothing before it and is the least risk. §4's items are small
and separate from each other. §3 is the largest change and the largest
gain for typing, waiting and a clock in view; it follows §2, so that the
whole frame it falls back to is already cheaper. §5 to §7 are each on
their own. Each is timed before and after as `docs/SPEED.md` §7 says, and
on a Pi's dev card with `--timings` where the gain is the Pi's.
