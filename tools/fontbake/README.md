# osk-fontbake

Turns the vendored fonts in `tools/fonts/` into glyph outlines that
`osk-ui` embeds with `include_bytes!`. Runs on the build box only
(`just fonts`); the device reads the format below and fills a glyph with
tiny-skia at whatever pixel size a display asks for. No TTF or CFF
parser ships on a device.

## Why outlines rather than baked bitmaps

Until 2026-09-10 the baker rasterised each face at sixteen fixed pixel
sizes, 4.1 MB of atlases, and a runtime size snapped to the nearest baked
one. The CJK wordlists would have added 3,650 glyphs, 8 MB at nine sizes.
`osk-ui` already depends on `tiny-skia`, which fills anti-aliased paths
for every other shape on screen; only glyphs went around it. Storing each
glyph once as an outline and filling it on demand costs 140 KB for the
four faces, draws every size at its exact size instead of the nearest
baked one, and leaves the CJK faces affordable.

## Why `ttf-parser`

`ttf-parser` reads the TrueType outlines of the Noto faces and the CFF
outlines of the two icon OTFs through one `outline_glyph` call, and it is
already in the lock (`fontdue`, the previous baker's rasteriser, brought
it). The tool is pinned to that version, so it adds no crate. Kerning is
not read: UI text is short and the monospace face has none.

## What is written

- Faces: `regular` (Noto Sans Regular), `semibold` (Noto Sans SemiBold),
  `mono` (Iosevka Regular), `icon`, and `cjk`, the fallback face
  the three text faces read when their own file lacks a code point. One
  file each; no size list anywhere.
- Text glyph set: ASCII printable U+0020–U+007E, Latin-1 Supplement
  U+00A0–U+00FF (minus the invisible soft hyphen U+00AD), and the UI
  symbols `• ▾ ▸ ← → ↑ ↓ ✓ ✗ × … ° ₿`.
- CJK glyph set: computed, never listed by hand, so it cannot drift from
  the lists. Every character of every word of the ten BIP-39 lists, in
  both the published form (`WORDS`) and the composed form a reader sees
  (`DISPLAY`); every keycap of the three non-Latin keyboards — hiragana
  U+3041–U+3096 with the voicing marks U+309B and U+309C, the Hangul
  compatibility jamo U+3131–U+3163, bopomofo U+3105–U+3129 and the tone
  marks `ˇ ˊ ˋ ˙`; and the ideographic space U+3000. Anything the text
  faces already carry is left out. That is 3,723 code points. The baker
  depends on `osk-bip` by path to read the lists, which is why it is the
  one tool in `tools/` that does.
- Icon glyph set: the code points `ICONS` lists, which must match the
  `Icon` enum in `core/osk-ui/src/widgets/icon.rs`. `U+E9xx` comes from
  `seedsigner-icons.otf`, everything else from
  `Font_Awesome_6_Free-Solid-900.otf`. A crate test asserts that every
  `Icon` resolves in the outline file, so the two lists cannot drift.
- Glyph source, per code point: the face itself; else the fallback (Noto
  Sans Mono for the text faces, which has the arrows and triangles Noto
  Sans lacks; Font Awesome for the icon face); else a glyph
  synthesized from polygons in `main.rs` (`✓` and `✗`, which no vendored
  text face has, written as line contours). The baker aborts on anything
  else.
- Glyph source for the CJK face, in order: Noto Sans, then Noto Sans
  Mono — which between them have the combining marks the accented Latin
  lists decompose into, and which no CJK subset carries — then the region
  subset the script belongs to: hiragana and the voicing marks from JP,
  Hangul syllables and both jamo sets from KR, bopomofo and the tone
  marks from TC, and a Han character from TC when the Traditional list
  uses it and from SC otherwise, so a word is drawn in the shapes its own
  list is written in. The remaining subsets are tried after that, and the
  baker aborts on a code point none of them supplies. Every glyph is
  rescaled to the file's own em, so one file is one coordinate system.
- A glyph taken from another face is rescaled to the em of the file it is
  written into, so one file is one coordinate system.

Output: one `core/osk-ui/assets/<face>.outl` per face and
`core/osk-ui/src/fonts/generated.rs`, a table of `include_bytes!` entries.

## Where the CJK fonts come from

The four Noto Sans CJK region subsets are 23 MB together and are not
committed. The baker downloads each into `tools/fonts/cjk/` when the file
is missing, checks it against the SHA-256 pinned in `CJK_SOURCES` in
`main.rs`, and aborts on a mismatch. The digests are also in
`tools/fonts/README.md` with the licence. This is the only network access
in the build, it happens once, and it is the baker's alone: the `.outl`
files are committed, so `cargo build` never needs the fonts, the baker or
the network.

## Outline format, version 1

All integers little-endian. One file holds one face. Coordinates are in
font units with y up and the origin at the pen on the baseline, exactly
as the source font states them.

```
offset  size  field
0       4     magic   "OSKO"
4       2     version = 1
6       2     flags   none defined
8       2     units_per_em
10      2     ascent   font units above the baseline (hhea), signed
12      2     descent  font units below the baseline (hhea), signed and
                       negative, as ttf-parser reports it
14      2     line_gap extra font units between lines (hhea), signed
16      4     count    number of glyph records
20      20×count  glyph records, sorted by code point ascending
…       ∑ len  outline data, back to back
```

Glyph record (20 bytes):

```
0   4  code point (u32)
4   2  advance (u16)            horizontal pen advance, font units
6   2  x_min (i16)              bounding box, font units
8   2  y_min (i16)
10  2  x_max (i16)
12  2  y_max (i16)
14  4  offset (u32)             byte offset of the outline from the start
                                of the outline block
18  2  length (u16)             its length in bytes
```

Outline data is a command stream: one tag byte, then that command's
coordinates as `i16` pairs.

```
'M'  x y                  start a contour at (x, y)
'L'  x y                  straight line to (x, y)
'Q'  cx cy x y            quadratic curve
'C'  c1x c1y c2x c2y x y  cubic curve
'Z'                       close the contour
```

A glyph with no outline (space) has a zero-length stream and a zero
bounding box; its advance still applies. Contours are filled with the
non-zero winding rule.

To draw a glyph at pixel size `px`, scale font units by
`px / units_per_em`, flip y, and translate so that the bitmap's top-left
is at `(floor(x_min × s), ceil(y_max × s))` relative to the pen on the
baseline; that is what `core/osk-ui/src/fonts/` does, filling the path
into a `tiny_skia::Mask` and caching the bytes. An icon carries no
baseline: the icon face's ascent is a whole em, so its line box is the
square `Canvas::icon` centres the glyph in.

Deliberate simplifications, to revisit when size matters:

- Coordinates are `i16` per point, uncompressed; no delta coding, no
  shared points between contours. The four Latin and icon faces are
  140 KB together; `cjk.outl` is 1.4 MB for 3,723 glyphs.
- No kerning table. Adding one is a new block after the outlines; the
  parser ignores trailing bytes.
- No hinting, and no sub-pixel positioning: a glyph is filled once per
  (face, pixel size, code point) and placed at a whole pixel.
- A missing code point is looked for in `cjk.outl` and then falls back to
  `?` at draw time; the baker aborts if a font lacks a glyph in the set,
  so `?` only appears for text the set was never meant to cover.
