# osk-ui

Layout engine, widgets and pixel renderer for OpenSignerKit. `no_std` +
`alloc`; the only dependencies are `tiny-skia` (shapes) and
`osk-shell-api` (the event and frame types). No flows, no wording of its
own: `opensigner-core` builds a widget tree from its state every frame and
this crate turns it into pixels and turns shell events into actions.

## Architecture

```
  DisplayInfo ──► Canvas (tiny_skia::Pixmap, premultiplied RGBA8888)
                    ▲ fill/stroke via tiny-skia, glyphs blended by hand
                    │
  Node tree ──► layout::solve ──► Layout (Rect + clip + HitTarget per node)
  (dp)          (px, two-pass)       │
                                     ├──► widgets::draw_tree ──► Canvas
                                     └──► UiState::event(Event) ──► Action
```

| Module | What it owns |
|---|---|
| `geom` | `Dp`, `Scale` (dpi → px, plus the class's type scale), `SizeClass`, `Metrics` (the area in dp and its class), integer `Rect`/`Size`/`Edges` |
| `color` | `Color`, `Theme` (dark default; every widget draws through theme roles) |
| `fonts` | outline-file parser, `SizedFace` (a family at a pixel size), the CJK fallback face, the `GlyphCache` a `Canvas` owns, `generated.rs` with the `include_bytes!` table |
| `text` | `Font { family, size dp }`, width measurement, word wrap; all in 1/64 px then rounded |
| `canvas` | `Canvas`: clear, rects, rounded rects, lines, circles, text, wrapped text, clip stack, `frame()` |
| `layout` | `Node` (Column/Row/Stack/Scroll/Widget) with weight, min/max/fixed, padding, gap, align, justify; `solve` → `Layout` |
| `widgets` | the `Widget` enum: `measure`, `hit_target`, `draw`; the `Icon` set; `keyboard` geometry; `chunked` planner; `scrollbar_thumb`; `draw_tree` |
| `state` | `UiState`: pressed target, hold progress, scroll offsets, keyboard modifiers, pager pages; `event` → `Action` |
| `tokens` | every dimension, size, ratio and timing the design system names (`docs/DESIGN.md` §3), grouped: type ramp, spacing, chrome, rows, entry, panels, codes, chips and badges, record, hub, timings |
| `components` | one function per content kind of `docs/DESIGN.md` §4, one file per group (`nav`, `choice`, `entry`, `identity`, `strings`, `paths`, `amounts`, `badges`, `codes`, `secrets`, `records`, `text`, `actions`, `progress`) |
| `organisms` | the frames the sixteen screens are built in — `screen_at` for one that scrolls, `action_screen_at` and `static_screen_at` for one that must not, the app bar, the `cta_at` action row and the `wide` `sidebar_frame` — and the few composites the screens module still calls: `tile_grid`, `secret_panel`, `field_at`, `descriptor`, `section`, `keyboard_slot`, `pad_slot` |
| `screens` | one function per reusable screen of `docs/DESIGN.md` §5, one file each: the sixteen screens every flow is built from |
| `gallery` | (feature `gallery`) an `App` with one page per §4 group and one per §5 screen, for the snapshot shell |

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

Every icon the application draws is a variant of `widgets::Icon`, which
maps to one code point in the icon face (SeedSigner's own icons at
`U+E9xx`, Font Awesome 6 Free Solid for the rest). Nothing pulls a symbol
out of a text face: text glyphs are the wrong shape and the wrong weight
at icon sizes. `Widget::Icon` draws one centred in a `size_dp` square;
`Widget::IconButton` puts one in a 48 dp target.

### Design tokens

`tokens` is the one place a dimension, a size, a ratio or a timing of the
design system lives (`docs/DESIGN.md` §3); `widgets::tokens` re-exports
it. It holds the ramp every screen draws with — display 26, title 20,
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

`tools/lint-tokens.sh`, which `just lint` runs, fails the build when
`organisms.rs`, `gallery.rs` or a file under `components/` writes a
layout number of its own instead of naming one in `tokens.rs`.

The ramp — and only the ramp — is scaled by the size class:
`tokens::type_scale_pct` is 100 % on `small`, 110 % on `mobile` and
130 % on `wide`. `Scale::for_class` carries that factor and `Font::sized`
applies it, so a screen keeps writing `tokens::BODY` and gets 16 dp on a
panel, 17.6 dp on a phone and 20.8 dp in a desktop window. Lengths do not
scale: a touch target is 48 dp everywhere and the grid is 8 dp
everywhere. `tokens::touch_floor` is the least height of a chip or a
candidate cell — 44 dp on `small` and `mobile`, 32 dp on `wide`, where a
pointer does the aiming.
`Theme::DARK` is black ground, `#1C1C1E` surface, `#FF9F0A` accent, with
success, caution, danger and info for the status colours.

### Components and organisms

`components` implements `docs/DESIGN.md` §4: one function per content
kind, one file per §4 group, each quoting the rule it implements. A
screen picks a component and never picks a font, a row height or a mask
policy of its own. Interaction reuses the hit targets that already exist,
so a component needs no state of its own.

`screens` implements `docs/DESIGN.md` §5. A flow builds a screen by
calling one of the sixteen with content — strings, values, ids, states —
and the screen decides everything else: the app bar, where the body sits
between the bar and the bottom action, the row heights, the fonts and the
`wide` sidebar, which every screen draws through `screens::wide_frame` so
its rectangle is the same on all of them. A screen spec is then three
lines: which screen, which content, what is left off.

One placement law puts everything on all sixteen, and `Chrome::place` is
the only place it is written down (`docs/DESIGN.md` §2.6): the actions
and the controls the thumb works — buttons, keyboards, pads, toggles,
pairs — are bottom-anchored; Menu, Addresses and Document start their
rows at the top and scroll; everything else is one block, vertically
centred in the space between the app bar and them. Nothing is placed by
how tall it happens to be, and the law is the same on all four sizes,
which is why a screen looks like itself on a 268 dp panel, a phone and a
desktop window; on `wide` the block is capped at `COLUMN_MAX_WIDTH` and
the action row is bottom-right at its natural width. A block taller than
its space gives height back first — a QR shrinks, a chunked string takes
a smaller size — and one that still does not fit starts at the top of a
screen that scrolls. `screens::SPACE`, `screens::BLOCK` and
`screens::GROUP` name the three rectangles, so a layout test can put one
centre against the other.

`organisms` is the frames and nothing else now. The composites the
design system replaced — `hero`, `choice_list`, `record`,
`warning_card`, `segmented_row`, `breadcrumb`, `using_chip`,
`cta_pager`, `key_strip` — went with the last of their callers when
Explore was rebuilt; their §4 components are in `components`, and a test
in `opensigner-core` (`no_view_uses_a_composite_the_design_system_replaced`)
walks the workspace's sources to keep it that way.

The §5 name for a screen that states an outcome is **Result**
(`screens::result`, `components::result`); "verdict" is gone. Addresses
and Address are two screens: a list of reference rows, and one address
with its QR.

### Layout

Everything an application writes is in dp; everything the solver emits is
in px. `measure` asks each node for its intrinsic size under a maximum
(text wraps to the offered width, a button reports its label width and the
44 dp touch height, a keyboard reports rows × key height). `place` then
gives weighted children the leftover main-axis space (respecting `max`),
shrinks the ones that can give height back when the sum overflows (down
to `min`), and aligns on the cross axis. A node that shrinks but never
grows says so with `shrink` instead of `weight`; a node with neither
never shrinks, so a screen that does not fit scrolls or clips, it never
silently reflows. `Scroll` nodes clip and offset their child; offsets
come from `UiState`. A `filling` scroll region gives its content the
whole viewport where the content fits in it, which is what lets the
placement law centre a body in the space it has and leaves a taller body
at the top.

Where a screen's body sits between the app bar and the bottom action is
one choice, `organisms::Body`: `Top` gives the body its own height under
the app bar and leaves the spare height below it, `Center` centres it in
the whole space, and `Fill` hands the body that space for a weighted
child — a code, a viewfinder, a hub grid — to absorb.

Composites branch on `geom::Metrics { width_dp, height_dp, class }`, not
on the class alone: the class swaps variants, the two lengths say how
much room the screen actually has. `OpenSigner::metrics` reports the
frame a screen is drawn in, which on `wide` is the content pane beside
the sidebar, capped to `WIZARD_MAX_WIDTH` for a wizard.

### Widgets and state

Widgets are plain data rebuilt every frame (immediate mode). Interactive
ones carry an application `Id`. `solve` records a `HitTarget` per
interactive widget, so `UiState::event` can resolve a touch without the
tree: tap (button, tile, chip, toggle, list row), hold (1500 ms, progress
from ticks), **reveal** (a secret panel's surface: no duration and no
completion, so nothing irreversible can hang off it), keyboard (key
geometry recomputed from the rect), candidate strip (equal cells), pager
(chunked strings that did not fit). Physical
keys are routed to the first keyboard on screen and filtered per keyboard
kind; PIN entry ignores them.

Anti-observation defaults (PLANNING §4.5): keyboards draw no popup preview
and no press state at all; only non-secret buttons, rows and tiles show a
pressed tint while held.

A secret is shown by one element and one gesture: `organisms::secret_panel`
is a tinted surface with an accent border, and `UiState::is_held` is true
while a finger is anywhere on it. It shows its content and nothing else;
there is no caption and no reveal button beside it. What a panel holds
fits it on one page, because the panel is the only touch target inside
it.

Revealing a secret moves nothing. The panel carries no caption in either
state, and a caller masks its content to the revealed line count and
column widths — bullets per character, the same monospace size — so the
panel and everything around it keep their rectangles when a finger lands
on them. Layout tests assert this at all four reference sizes. The app
bar's eye shows a secret for 30 s and is drawn as a `Widget::Ring` that
empties clockwise while it runs, which is the only countdown in the
interface.

A `Scroll` node draws its own scrollbar: a 3 dp thumb at the right edge of
the viewport, as tall as the visible fraction of the content and never
shorter than 24 dp, drawn only while the content overflows
(`widgets::scrollbar_thumb`). Screens are built to avoid scrolling; where
content is a fixed run (word pages, outputs) `components::pager` sits
under the block it pages (§4.1). `organisms::action_screen_at` is the
frame for a screen that must not scroll: the body takes the whole space
between the app bar and the buttons, so `screens::Chrome::place` can
centre a block in it. `organisms::screen_at` is the frame for one that
does scroll, and the Entry screen builds its own, because its keyboard
is full-bleed and reaches the bottom edge.

Text does not clip silently. A button label, a list-row title or a
chunked string drops a size before it runs out of room; only then is it
cut. `OpenSigner::overflow()` (in `opensigner-core`) reports anything the
last frame placed outside the region it is drawn in, which is what the
layout tests assert on.

## Size classes

From the physical diagonal (`px diagonal / dpi`): `Small` below 3.5",
`Mobile` below 7", `Wide` otherwise. Small panels are portrait: the
reference is 480×640 @ 286 dpi and the smallest supported panel is
240×320 @ 143 dpi, and both render the same 268×358 dp layout. The `Wide`
reference is 960×640 @ 160 dpi, which is also the desktop window's own
fixed size. Landscape still solves, but nothing is designed for it.

On `Wide` the sidebar is on every screen. A wizard, the scanner, the
lock screen, the four hold-to-confirm screens and the terminal screens
draw it dimmed and without hit targets, so the content origin never
moves between two screens of one flow and navigation never shares a
screen with a secret or a destructive step.

| Rule | Small | Mobile | Wide |
|---|---|---|---|
| Home | 2-column tile grid, filling the space | key rows and the square grid as one centred group | sidebar plus the key list; no grid |
| Type scale | 1.0 | 1.1 | 1.3 |
| Chip and candidate height | 44 dp | 44 dp | 32 dp |
| Words per backup page | 6 | 12 | 12 |
| A run of pages | one page and a pager | one page and a pager | the whole run side by side when it fits |
| Keyboard key height | 40 dp, down to 36 dp when squeezed | 9% of the height under the app bar, 36–56 dp | 9% of the height under the app bar, 36–56 dp |
| PIN pad key height | 44 dp | from the pad's width, cells no wider than 1.4:1 | from the pad's width, cells no wider than 1.4:1 |
| PIN, dice and coin pad width | the screen | 280 dp, centred | 280 dp, centred |
| Tile icon | 28 dp | 48 dp | 48 dp |
| Candidate font | 13 dp | 14 dp | 14 dp |

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

## Adding a widget

1. Add a variant to `Widget` in `widgets/mod.rs` with its data, and a
   constructor if it has defaults.
2. Implement it in `measure` (intrinsic size under a maximum), `id` /
   `hit_target` (if interactive), and `draw`.
3. If it needs cross-frame state, add it to `UiState` and an `Action`
   variant, and handle it in `UiState::touch`/`key`.
4. Add it to a gallery page and look at it at all four reference sizes
   (`just snapshots`).
5. Test geometry, not pixels: `measure` results, hit targets, any pure
   planning function (see `chunked.rs`, `keyboard.rs`).

Composite pieces that are just trees of existing widgets go in
`organisms.rs` as functions returning a `Node`.

## Reviewing screens

```
just snapshots                     # out/snapshots/<WxH>/ for 240×320, 480×640, 1080×2340, 960×640
cargo run -p opensigner-snapshot -- --size 480x640 --dpi 286 --out out/x --script events.txt
cargo run -p opensigner-snapshot -- --strip 4.1-navigation --out out/x   # one page, three sizes
```

Script commands: `tap X Y`, `down/move/up X Y`, `key K`, `text "abc"`,
`scroll X Y DY`, `tick MS`, `snap NAME`, `page N`, `#` comments. The
snapshot shell is a review tool; there are no image-comparison tests
(PLANNING §16.7).
