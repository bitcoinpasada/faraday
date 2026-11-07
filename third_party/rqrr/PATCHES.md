# rqrr, vendored

Upstream: `rqrr` 0.11.0, <https://github.com/WanzenBug/rqrr>, copied from
the crates.io release. The version number stays upstream's. The root
`Cargo.toml`'s workspace dependency is a path to this directory, so
`faraday-codec` reaches the copy in every build: this workspace's, the fuzz
workspace's, and that of a project depending on `faraday-codec` by git, which
a `[patch.crates-io]` would not reach.

Why the copy exists: security review 2026-09-11 item M5a. The QR decoder
reads camera frames, which nobody controls, and the Pi shell is built
with `panic = "abort"` and is the only process on the board, so a panic
inside grid detection powers the device off while a person is signing.
Faraday's crate-reduction plan wants the crate in the tree anyway:
trimmed of `lru` and `g2p` it takes twelve crates and about 100,000 lines
off the device dependency graphs.

The copy now has no dependencies at all. `image` is still in the manifest
behind the `img` feature, still optional and still off.

## What was changed

**`Perspective::map` returns `Option<Point>`** (`src/geometry.rs`).
Upstream rounds the transformed point and asserts four times that it fits
in an `i32`. A frame with a near-degenerate capstone group drives the
denominator towards zero and the coordinate past the bounds, and the
assertion — an `assert!`, so release builds too — fires during grid
detection, before any payload is read. The minimized frames are
`fuzz/corpus/decode_luma/rqrr-perspective-map.bin` and
`rqrr-perspective-map-negative.bin`. The vendored version returns `None`
for a coordinate out of range or not a number, and every caller treats
that as "no such cell":

| Caller | What it does now |
|---|---|
| `create_capstone` (`src/detect.rs`) | `?`: no capstone at that position |
| `measure_timing_pattern` (`src/identify/grid.rs`) | returns `Option<usize>`; `from_group` rejects the group |
| `projected_module_area` (`src/identify/grid.rs`, for `find_alignment_pattern`) | `?`: no module size from the edges' point, so the second seed is tried; with that also failing, no alignment pattern and no grid |
| `fitness_cell`, `fitness_ring`, `fitness_apat`, `fitness_capstone`, `fitness_all` | return `Option<i32>`: a sample that cannot be placed makes the score meaningless |
| `jiggle_perspective` | returns `Option<Perspective>`; an adjustment whose score cannot be computed is not an improvement and the coefficient goes back |
| `RefGridImage::bit` | reads as white, which is what an off-frame cell already reads as; error correction then decides the grid's fate |
| `detect_grids` (`src/prepare.rs`) | a grid whose own four corners cannot be placed is not reported |

Two further changes on the same path:

- `measure_timing_pattern` returned `usize` after `assert!(scan >= 1)`.
  A frame whose timing scan finds no transition at all reaches that
  assertion. It now returns `None` instead, which the one caller already
  handles.
- The `image` dev-dependency and the tests in `src/detect.rs` that used
  it are gone, with `src/test_data/`. They are the only part of the crate
  that needed a crate outside the workspace lock file, and dropping them
  keeps the lock unchanged except for `rqrr`'s own source line. The
  `image` optional dependency behind the `img` feature stays in the
  manifest and stays off: `faraday-codec` takes the crate with
  `default-features = false`.

`Cargo.lock` and `Cargo.toml.orig` from the crates.io package are not
kept. The three licence files are.

**`lru` is gone** (`src/prepare.rs`). Upstream kept the flood fill's
regions in an `LruCache<u8, ColoredRegion>` of capacity 251. The key is a
`u8` and the index travels in the pixel values as
`PixelColor::Discarded(idx)` — written as `idx + 5`, which is what bounds
it to 251 — so the entries are now an array of that length with a
recency stamp per entry (`RegionTable` in `src/prepare.rs`).

The eviction stays. A version 30 symbol at three pixels a module has
thousands of black regions, so a table that refused the 252nd would stop
finding capstones part-way down the frame and large symbols would not
decode at all (`faraday-codec`'s `large_codes_decode_too` covers this). As
upstream did, the entry with the oldest stamp is dropped, its pixels are
repainted plain black so it can be found again under another index, and
its index is reused. `get` stamps the entry it returns, as `LruCache::get`
promoted it. The one visible difference is in `Clone`: upstream's rebuilt
the cache by iterating it most-recently-used first and putting each entry
back, which reversed the recency order; the array copies it as it is.

`get_region` returns `Option<ColoredRegion>`, which also answers the
`panic!("Tried to color white patch")` and the `.unwrap()` on a missing
cache entry: `None` at a pixel means "nothing of interest here", and
`is_capstone` and `find_alignment_pattern` already had that case.

**`g2p` is gone** (`src/decode.rs`, `src/galois.rs`). Upstream generated
two fields with the `g2gen` proc-macro.

- GF(2^8), for Reed-Solomon over the codewords, is now `src/galois.rs`:
  the same field, polynomials over GF(2) reduced by
  x^8 + x^4 + x^3 + x^2 + 1 (`0b1_0001_1101`) with 2 as the generator —
  the element `g2gen`'s `find_generator` picks for that modulus — and the
  same exp and log tables, built in a `const` block instead of by a
  macro. Division by zero returns zero where upstream's panicked; both
  call sites test the divisor first, so the value is never used.
- GF(2^4), used only to BCH-decode the 15-bit format information, is
  gone entirely. `correct_format` now takes the nearest of the 32 valid
  format codewords, which the same file generates in a `const` block, and
  fails when the nearest is more than three bits away. The code's minimum
  distance is 7, so a word within three bits of a codeword is within
  three bits of exactly one of them, and this is the same answer
  Berlekamp-Massey gave. It is also what quirc, which this crate is a
  port of, does. `format_syndromes` is gone with it, and
  `berlekamp_massey`, `poly_eval` and `poly_add` are no longer generic
  over a field.

**`#![forbid(unsafe_code)]`** is on `src/lib.rs`. The crate had no
`unsafe` and now cannot gain any.

**Panic sites a frame can reach are rejections now.** Each of these was
on the list below; each is now "no capstone", "no grid" or a decode
error.

| Site | Was | Is |
|---|---|---|
| `rotate_capstone` (`src/identify/grid.rs`) | `.expect("rotated perspective can't fail")` on corners taken from the frame | returns `Option<()>`; `from_group` rejects the group |
| `repaint_and_apply` (`src/prepare.rs`) | `panic!("Cannot repaint with white or same color")` — an alignment pattern already claimed by another candidate group reaches the second half | returns `Option<F>`; `create_capstone` and `from_group` reject the candidate |
| `get_region` (`src/prepare.rs`) | `panic!("Tried to color white patch")`, and `.unwrap()` on a cache entry | returns `Option<ColoredRegion>` |
| `flood_fill` (`src/prepare.rs`) | `assert_ne!(from, to)` | returns the filler untouched |
| `find_and_rank_possible_neighbors` (`src/identify/match_capstones.rs`) | `.expect(...)` on a `partial_cmp` that is `None` for a NaN score | NaN sorts last |
| `RawData::push` (`src/decode.rs`) | `assert!((self.len / 8) < MAX_PAYLOAD_SIZE)` | a bit past the buffer is dropped, and the truncated stream fails error correction |
| `bits_remaining` (`src/decode.rs`) | `assert!(self.bit_len >= self.ptr)` | saturating subtraction |
| `take_bits` (`src/decode.rs`) | `assert!(max_len <= 64)` | the read is clamped to the accumulator's width |
| `correct_block` (`src/decode.rs`) | `assert!(ecc.bs > ecc.dw)` | `DeQRError::DataEcc` |
| `mask_bit` (`src/decode.rs`) | `panic!("Unknown mask value")` | the match is on `mask & 7`, which the format information's three mask bits are |

## Finding the grid of a dense code

A version 23 symbol (109 modules a side) at 6 to 9 pixels a module in a
1920x1080 frame found its three capstones every time and a grid in about
half the frames. Both causes are distances read off a capstone's own
geometry, which is fitted to corners a few dozen pixels apart and rounded
to whole pixels, and which a dense code asks to be extrapolated a hundred
modules.

- **Neighbour test** (`src/identify/match_capstones.rs`). Whether another
  capstone lies along a capstone's rows or columns, and how far, was read
  through the capstone's perspective transform (`unmap`). Two corners a
  pixel off make that transform's projective terms wrong enough that a
  capstone a hundred modules away mapped to 80 or 160 modules and the two
  distances of a corner capstone differed by more than the quarter the
  grouping allows, so no group formed. The test now takes the capstone's
  ring as a parallelogram (`affine_offsets`: each axis the mean of the two
  opposite edges) and uses the perspective where that is degenerate and
  where the parallelogram test gives a capstone no viable pair, so a code
  tilted 35 degrees or more and held close, whose parallelogram distances
  disagree, forms the groups it formed before. The test only chooses which capstones are tried together; the timing
  pattern, the alignment pattern, the sampling and the error correction
  are as before.
- **Alignment pattern search** (`src/identify/grid.rs`). The search
  spiralled out from the point where the edges of capstones A and C
  intersect, and on `continue` for a region that was not a plain black one
  (a capstone's own ring, say) it did not step, so the spiral stood still
  for the whole search and looked only at its first pixel. It
  steps on every pixel now. Where the search from the edges' point finds
  nothing, it is repeated from the point that completes the parallelogram
  of the three capstones' centres, with the module area the capstones
  show; the area of a module used to come only from the capstones'
  perspective too.

Tested against generated frames in `core/faraday-codec/tests/dense_codes.rs`.

## Crafted frames: bounded work and checked arithmetic

A camera frame is the untrusted side's. The fault to prevent is a scanner
that stalls or ends, never a wrong payload; each change below makes a
frame over a bound give no read, as any unreadable frame does.

- **Capstone count** (`src/detect.rs`, `MAX_CAPSTONES` = 256). A lattice of
  seven-pixel corner marks gives about 32,000 capstones at 1920x1080 and
  the grouping then did not end (measured: still running after 60 s in a
  release build; now 16 ms). More than 256 capstones in a frame is no
  read. A code has three.
- **Group tries** (`src/prepare.rs`, `MAX_PAIRS_PER_CAPSTONE` = 8,
  `MAX_GROUP_TRIES` = 128). Each try copies the whole image. Pairs are
  taken best-ranked first; a genuine code's pair ranks among the first
  few. Past the budget the groups found so far are returned.
- **Alignment search** (`src/identify/grid.rs`,
  `MIN_MODULES_ACROSS_FRAME` = 17). The spiral's bound grew with the
  module area read off the capstones' perspective, up to 2^31 pixels and
  about 10^11 steps. An estimate over (longer side / 17) squared, 12,700
  pixels at 1920x1080, is refused. That bounds the search to about 1.3
  million steps. No frame reaching this was found: 4,000 warped version 2
  codes at 1920x1080 all ended in under 150 ms. Each step also counts
  against the work budget below.
- **Work budget** (`src/prepare.rs`, `WORK_PER_PIXEL` = 64, `MIN_WORK` =
  2^26). The bounds above stop only frames that make capstones. Full-height
  stripes repeating black 1, white 1, black 3, white 1, black 1, white 1 make
  none: each row touches 480 stripes against the 251 entries of the region
  table, so every touch evicts one stripe and repaints another, 1080 rows
  each, about 1.1 x 10^9 rows for the frame. Black 2, white 2, black 5,
  white 2, black 2, white 1 does the same, and so does the alignment search
  crossing radial spokes. So each frame has one budget of work, shared with
  the copies each group try makes, and past it the search stops and the
  frame is no read, whatever it found first. A unit is about one pixel of a
  flood fill: every route through `flood_fill` is charged per row (its width
  plus `WORK_PER_ROW` = 8), and so are the things a frame can repeat that are
  not fills: finding a region to evict (251), a copy of the frame (a quarter
  of a unit a pixel), a step of the alignment search (1) and fitting a grid
  (60,000 a module of its side). At 1920x1080 the budget is 132.7 million,
  and at 960x540 or smaller 67 million.

  Spent on generated frames (the search alone, of the budget):

  | Frame | Spent, millions | Budget over that |
  |---|---|---|
  | Version 23 at 1920x1080, 6 to 9 pixels a module, blurred, noisy | 14.0 to 14.3 | 9.3 times |
  | The same on a textured background | 16.1 | 8.2 times |
  | Six version 2 codes on one sheet | 25.1 | 5.3 times |
  | Fine noise at 2 pixels, nothing to read | 24.3 | 5.5 times |
  | Version 23 reduced to 960x540 | 13.4 | 5.0 times |
  | Version 40 alone at 2, 3 pixels a module (370x370, 555x555) | 21.4, 21.6 | 3.1 times |
  | Version 40 alone at 6 pixels a module (1110x1110) | 22.5 | 3.5 times |
  | Version 1 alone, any size | 2.5 to 2.8 | over 20 times |

  A code's fit takes nearly all of it and is fixed by its version, so these
  move little from frame to frame; none of them was a camera's own picture.
  A code in the same frame as stripes of this kind is not read while they
  are in the frame.

  In a release build on an x86 desktop core a frame that spends all of it
  takes 120 to 150 ms, at about 1 ns a unit for each kind of frame tried
  (stripes, spokes, lattices of 60 to 220 capstones, codes); before, the
  stripes' rows alone came to about 10 s there, by arithmetic. Not measured
  on a Pi 3B+. If a row costs one 55 to 165 ns (7 to 20 times the desktop's,
  an estimate) a frame that spends all of it takes 1 to 3 s there.
  `Scanner::stop` waits for the pass it interrupts, and a pass in the reduced
  mode reads at most the reduced copy and then the full frame, so closing the
  scan sheet on such a frame holds the loop for 1.5 to 4 s at most, by the
  same estimate.
- **Capstone centre** (`src/detect.rs`). A capstone whose centre lies more
  than a frame's width or height outside the frame is no capstone. Later
  scans (`BresenhamScan`) run between points derived from it. Not shown
  to be reachable as a frame.
- **Arithmetic** (`src/geometry.rs`, `src/identify/grid.rs`).
  `line_intersect` works in `i128` and returns `None` for a crossing that
  is not an `i32` (the `i32` products wrapped in release and panicked in
  debug). `BresenhamScan::new` returns `None` for a span over 2^30 or a
  difference that overflows (`i32::MIN` has no absolute value);
  `timing_scan` passes that on and the group is rejected. The side-of-line
  tests and `LeftMostFinder`'s score are in `i64`. Nothing is saturated: a
  value out of range rejects the group.

Tested in `core/faraday-codec/tests/crafted_frames.rs` (the lattice frame is
generated) and in `src/geometry.rs`.

## Other panic sites, not yet changed

Found by reading for `assert!`, `unwrap`, `expect`, `panic!` and indexing
on frame-derived values. None has a reproducer; they are listed so the
next pass over this crate has somewhere to start.

- `src/prepare.rs`, `prepare_from_greyscale` and `prepare_from_bitmap`:
  `w.checked_mul(h).expect("Image dimensions caused overflow")`.
  `faraday_codec::decode_luma` requires `luma.len() == width * height`
  before it calls either, so our caller cannot reach it.
- `src/prepare.rs`, `without_preparation`: `assert!(buf.get_pixel(x, y) <
  2)`. It is reachable only through `prepare_from_bitmap`, whose own
  pixels come from a `bool`, or by a caller passing its own
  `ImageBuffer`. `faraday-codec` uses neither.
- `src/prepare.rs`, `get_pixel_at_point`: `self.width() - 1` underflows
  on a zero-width image. `faraday_codec::decode_luma` rejects a frame whose
  length is not `width * height`, and the only zero-width frame that
  passes that has no pixels to scan.

## Next steps from the plan

That plan's §5.1 asked for three removals. `lru` and
`g2p` are done. The third was a correction rather than a removal: `log`
is not a dependency of this crate and never was — it is in the graph
through `tiny-skia`.

What that section asked for and this copy still does not have: zeroizing
scratch buffers. The prepared image, the corrected data stream and the
decoded payload are ordinary allocations. That is an accepted gap.
