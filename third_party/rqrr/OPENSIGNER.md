# rqrr, vendored

Upstream: `rqrr` 0.11.0, <https://github.com/WanzenBug/rqrr>, copied from
the crates.io release. The version number stays upstream's. The root
`Cargo.toml`'s workspace dependency is a path to this directory, so
`osk-codec` reaches the copy in every build: this workspace's, the fuzz
workspace's, and that of a project depending on `osk-codec` by git, which
a `[patch.crates-io]` would not reach.

Why the copy exists: security review 2026-09-11 item M5a. The QR decoder
reads camera frames, which nobody controls, and `opensigner-pi` is built
with `panic = "abort"` and is the only process on the board, so a panic
inside grid detection powers the device off while a person is signing.
`local/crate-reduction-plan.md` §5 wants the crate in the tree anyway:
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
`fuzz/artifacts/decode_luma/rqrr-perspective-map.bin` and
`rqrr-perspective-map-negative.bin`. The vendored version returns `None`
for a coordinate out of range or not a number, and every caller treats
that as "no such cell":

| Caller | What it does now |
|---|---|
| `create_capstone` (`src/detect.rs`) | `?`: no capstone at that position |
| `measure_timing_pattern` (`src/identify/grid.rs`) | returns `Option<usize>`; `from_group` rejects the group |
| `find_alignment_pattern` | `?`: no alignment pattern, so no grid |
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
  manifest and stays off: `osk-codec` takes the crate with
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
decode at all (`osk-codec`'s `large_codes_decode_too` covers this). As
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

## Other panic sites, not yet changed

Found by reading for `assert!`, `unwrap`, `expect`, `panic!` and indexing
on frame-derived values. None has a reproducer; they are listed so the
next pass over this crate has somewhere to start.

- `src/prepare.rs`, `prepare_from_greyscale` and `prepare_from_bitmap`:
  `w.checked_mul(h).expect("Image dimensions caused overflow")`.
  `osk_codec::decode_luma` requires `luma.len() == width * height`
  before it calls either, so our caller cannot reach it.
- `src/prepare.rs`, `without_preparation`: `assert!(buf.get_pixel(x, y) <
  2)`. It is reachable only through `prepare_from_bitmap`, whose own
  pixels come from a `bool`, or by a caller passing its own
  `ImageBuffer`. `osk-codec` uses neither.
- `src/prepare.rs`, `get_pixel_at_point`: `self.width() - 1` underflows
  on a zero-width image. `osk_codec::decode_luma` rejects a frame whose
  length is not `width * height`, and the only zero-width frame that
  passes that has no pixels to scan.

## Next steps from the plan

`local/crate-reduction-plan.md` §5.1 asked for three removals. `lru` and
`g2p` are done. The third was a correction rather than a removal: `log`
is not a dependency of this crate and never was — it is in the graph
through `tiny-skia`.

What that section asked for and this copy still does not have: zeroizing
scratch buffers. The prepared image, the corrected data stream and the
decoded payload are ordinary allocations, and `docs/PLANNING.md` §16.20
records that as an accepted gap.
