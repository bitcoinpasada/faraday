# faraday-draw

The drawing base of Faraday's screens. It holds
the pixel canvas, text and the fonts, geometry, the design tokens, the
layout context, and the icon and keyboard widgets. `no_std` + `alloc`; the
dependencies are `tiny-skia` (shapes), `faraday-shell-api` (the display and
frame types), `faraday-codec` (the QR quiet zone) and `faraday-crypto` (the work
counter). It has no flows and no wording of its own.

## Architecture

```
  DisplayInfo ──► Canvas (tiny_skia::Pixmap, premultiplied RGBA8888)
                    ▲ fill/stroke via tiny-skia, glyphs blended by hand
                    │
  fonts, text, geom, tokens, LayoutCtx ──► sizes and positions
  widgets::Icon, widgets::keyboard     ──► icon code points, key rows and key cells
```

| Module | What it owns |
|---|---|
| `geom` | `Dp`, `Scale` (dpi to px, plus the class's type scale), `SizeClass`, integer `Rect`/`Size`/`Edges` |
| `color` | `Color` |
| `fonts` | outline-file parser, `SizedFace` (a family at a pixel size), the CJK fallback face, the `GlyphCache` a `Canvas` owns, `generated.rs` with the `include_bytes!` table |
| `text` | `Font { family, size dp }`, width measurement, word wrap; all in 1/64 px then rounded |
| `canvas` | `Canvas`: clear, rects, rounded rects, lines, circles, text, wrapped text, clip stack, `frame()` |
| `layout` | `LayoutCtx`: the scale, the size class and the shell's insets |
| `widgets` | the `Icon` set; `keyboard`: the key rows of every layout, key cells and key heights |
| `tokens` | every dimension, size, ratio and timing the design system names (`docs/DESIGN.md` §3) |

### Fonts and icons

Each glyph is stored once as an outline, in font units, by
`tools/fontbake` on the build box: one `assets/<face>.outl` per face,
140 KB for the three text faces and the icon face and 1.4 MB for `cjk`
(format in `tools/fontbake/README.md`). `cjk` is the fallback face, never
asked for by name: a code point a text face lacks — a hanzi, a kana, a
jamo, a bopomofo keycap — is looked up there and drawn at the size the
text face asked for, scaled by that file's own units per em. At runtime a
`Font` size in dp is multiplied by dpi/160 and rounded to a whole pixel,
and `Font::sized` returns a `SizedFace`: the outlines plus that size's
ascent, descent and line gap. Measurement (`text::width`, `wrap`,
`measure`) reads advances and never rasterises. The first time a glyph is
drawn at a size, tiny-skia fills its path into a coverage bitmap that the
`Canvas`'s `GlyphCache` keeps, keyed by (family, pixel size, code point)
and bounded by a byte budget in `fonts/`; the canvas blends the coverage
by hand. tiny-skia is built without SIMD (`no-std-float`), so the
coverage is the same bytes on every architecture. The size class's type
scale is applied before the rounding (see **Design tokens**); the icon
face is not scaled that way, because a screen asks for an icon in the box
it drew for it. Text is placed at whole pixels with advances accumulated
in 1/64 px, so a frame is deterministic for a given
`(width, height, dpi)`.

Every icon Faraday draws is a variant of `widgets::Icon`, which
maps to one code point in the icon face (SeedSigner's own icons at
`U+E9xx`, Font Awesome 6 Free Solid for the rest). Nothing pulls a symbol
out of a text face: text glyphs are the wrong shape and the wrong weight
at icon sizes. 
### Design tokens

`tokens` is the one place a dimension, a size, a ratio or a timing of the
design system lives (`docs/DESIGN.md` §3). It holds the ramp every screen draws with — display 26, title 20,
body 16, label 14, caption 12 and mono 16 dp, and the 1.43 line box —
plus an 8 dp spacing grid, 16 dp screen padding, 48 dp touch targets, 12
dp corner radius, a 56 dp app bar and status line, 56 dp list rows, 72 dp
menu rows (52 dp on `small`, where five rows and the app bar share 358
dp) and a 52 dp bottom action and text field. A value that varies with
the class is a `const fn name(class)` rather than three constants:
`menu_row`, `stacked_row`, `pager_row`, `touch_floor`, `key_height`,
`candidates`, `words_per_page`, `qr_side`, `tile_icon`, `hub_columns`.
The `wide` width caps are constants: a form or list column 480 dp
(`COLUMN_MAX_WIDTH`), a button 160–320 dp (`BUTTON_MIN_WIDTH`,
`BUTTON_MAX_WIDTH`), a QR 240 dp (`qr_side`) and a keyboard 720 dp
(`KEYBOARD_MAX_WIDTH`).

`tools/lint-tokens.sh`, which `just lint` runs, fails the build when a
file under `widgets/` writes a layout number of its own instead of naming
one in `tokens.rs`.

The ramp — and only the ramp — is scaled by the size class:
`tokens::type_scale_pct` is 100 % on `small`, 110 % on `mobile` and
130 % on `wide`. `Scale::for_class` carries that factor and `Font::sized`
applies it, so a screen keeps writing `tokens::BODY` and gets 16 dp on a
panel, 17.6 dp on a phone and 20.8 dp in a desktop window. Lengths do not
scale: a touch target is 48 dp everywhere and the grid is 8 dp
everywhere. `tokens::touch_floor` is the least height of a chip or a
candidate cell — 44 dp on `small` and `mobile`, 32 dp on `wide`, where a
pointer does the aiming.

## Size classes

From the physical diagonal (`px diagonal / dpi`): `Small` below 3.5",
`Mobile` below 7", `Wide` otherwise. Small panels are portrait: the
reference is 480×640 @ 286 dpi and the smallest supported panel is
240×320 @ 143 dpi, and both render the same 268×358 dp layout. The `Wide`
reference is 960×640 @ 160 dpi, which is also the desktop window's own
fixed size. Landscape still solves, but nothing is designed for it.

Key cells tile the keyboard region with no gaps between hit areas, out to
the region's edges: the bottom row's cells reach the bottom of the
screen and the outer columns the left and right edges. Each key paints a
face inset by `keyboard::KEY_FACE_INSET` (3 dp) at the sides and
`keyboard::KEY_FACE_INSET_Y` (2 dp) above and below, and the bottom row's
face is inset by `keyboard::KEY_BOTTOM_INSET` (16 dp), so the keys people
see stand clear of the edge while every pixel of the region still types.
Key width is whatever the columns divide the width into: about 27 dp of
pitch and 24 dp of face on the reference panel, about 41 dp of pitch on a
phone. Touch targets are 48 dp (`tokens::TOUCH`). Size classes only swap
variants; they never reorder or hide anything.

A screen asks for its keyboard's height with `keyboard::node_heights`,
which adds `keyboard::BOTTOM_RESERVE` to the row heights. `keyboard::
height` takes that reserve back off whatever it is offered, so a screen
that caps the node at `rows × key height` alone loses
`BOTTOM_RESERVE / rows` dp from every key.
