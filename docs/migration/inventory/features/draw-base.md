# The drawing base

The `no_std` drawing crate under every app (`core/faraday-draw`, with
`alloc`, `core/faraday-draw/src/lib.rs:~8-10`): the `Canvas` pixmap with its
clip stack, primitives (rects, rounded rects, lines, circles, contours,
cells, shadows, frost, camera luma, text, icons, kept shapes), the call
list that records and replays painting in bands, frame blends for motion,
geometry and display classes, text measure and wrap, the shape and glyph
caches, the embedded fonts and the icon set, and easing curves. The
component layer (`faraday-ui`) and the shells draw through it.

## Screens and sheets

None. Primitives (`core/faraday-draw/src/`):
- `Canvas` (`core/faraday-draw/src/canvas.rs:~31`): premultiplied RGBA pixmap, visible rows plus hidden rows (`HIDDEN_DP` 64 dp) below, clip stack, glyph cache, shape cache, scratch buffer, path pool, cut-string list (`cut`), ink list, line counter, kept-page marker, layer, damage box, work counter, mode, recorded list. `Canvas::part` / `new` (`core/faraday-draw/src/canvas.rs:~131`, `core/faraday-draw/src/canvas.rs:~142`).
- Painting: `clear` (`core/faraday-draw/src/canvas.rs:~344`), `push_clip`/`pop_clip`/`clip` (`core/faraday-draw/src/canvas.rs:~327-339`), `fill_rect` (`core/faraday-draw/src/canvas.rs:~443`), `fill_cells` (`core/faraday-draw/src/canvas.rs:~462`), `shadow` (`core/faraday-draw/src/canvas.rs:~495`), `elevation`/`elevation_faded` (`core/faraday-draw/src/canvas.rs:~532`, `core/faraday-draw/src/canvas.rs:~539`), `frost` (`core/faraday-draw/src/canvas.rs:~576`), `fill_rounded_rect` (`core/faraday-draw/src/canvas.rs:~1014`), `stroke_rounded_rect` (`core/faraday-draw/src/canvas.rs:~1054`), `line` (`core/faraday-draw/src/canvas.rs:~1072`), `stroke_closed` (`core/faraday-draw/src/canvas.rs:~1088`), `fill_circle` (`core/faraday-draw/src/canvas.rs:~1106`), `fill_contours` (`core/faraday-draw/src/canvas.rs:~1134`), `fill_kept` (`core/faraday-draw/src/canvas.rs:~1165`), `measure_text` (`core/faraday-draw/src/canvas.rs:~1215`), `text`/`text_with_face` (`core/faraday-draw/src/canvas.rs:~1221`, `core/faraday-draw/src/canvas.rs:~1227`), `icon` (`core/faraday-draw/src/canvas.rs:~1313`), `text_wrapped` (`core/faraday-draw/src/canvas.rs:~1342`), `place` (`core/faraday-draw/src/canvas.rs:~256`), `place_scaled` (`core/faraday-draw/src/canvas.rs:~281`), `forget_glyphs` (`core/faraday-draw/src/canvas.rs:407`), `Ink`/`InkKind` record (`core/faraday-draw/src/canvas.rs:~88`, `core/faraday-draw/src/canvas.rs:~97`).
- Surface (`core/faraday-draw/src/canvas/surface.rs`): `Surface` (`core/faraday-draw/src/canvas/surface.rs:~17-38`), `fill_rect` (`core/faraday-draw/src/canvas/surface.rs:~41-81`), `fill_cells` (`core/faraday-draw/src/canvas/surface.rs:~86-154`), `ShadowOf` and `reach` (`core/faraday-draw/src/canvas/surface.rs:~157-196`), `shadow` (`core/faraday-draw/src/canvas/surface.rs:~203-293`), `luma` (camera frame, NV12-style, `core/faraday-draw/src/canvas/surface.rs:~298-337`), `blit_glyph` (`core/faraday-draw/src/canvas/surface.rs:~341-384`), `ink_box` (`core/faraday-draw/src/canvas/surface.rs:~389-398`), `path` (`core/faraday-draw/src/canvas/surface.rs:~410-455`).
- Blend (`core/faraday-draw/src/canvas/blend.rs`): `blend_between`, `blend_noting`, `blend_noting_banded` (`core/faraday-draw/src/canvas/blend.rs:~98-188`); `blend_within`, `blend_within_banded`, `blend_all_within_banded`, `within_banded` (`core/faraday-draw/src/canvas/blend.rs:~196-282`); `blend_shifted` (`core/faraday-draw/src/canvas/blend.rs:~293-338`); `BlendBand` and `blend` (`core/faraday-draw/src/canvas/blend.rs:~26-53`, `core/faraday-draw/src/canvas/blend.rs:~341-398`); `split`, `each` (`core/faraday-draw/src/canvas/blend.rs:~55-90`); `blend_runs` (`core/faraday-draw/src/canvas/blend.rs:~404-443`); `Bands` (`core/faraday-draw/src/canvas/blend.rs:~445-491`).
- Call list (`core/faraday-draw/src/canvas/list.rs`): `Mode` Direct, Record, Replay (`core/faraday-draw/src/canvas/list.rs:~42-53`); `Op` (Clear, FillRect, Cells, Shadow, Luma, Rounded, StrokeRounded, Line, StrokeClosed, Circle, Contours, Kept, Text, Icon, Frost, Shift; `core/faraday-draw/src/canvas/list.rs:~56-98`); `Entry` (op, colour, clip, barrier, `core/faraday-draw/src/canvas/list.rs:~101-109`); `List` (`core/faraday-draw/src/canvas/list.rs:~113-124`); `List::clear` (`core/faraday-draw/src/canvas/list.rs:131-140`); `record_calls`, `trim_caches`, `waiting` (`core/faraday-draw/src/canvas/list.rs:~146-175`); `replay`, `sync`, `paint_entry` (`core/faraday-draw/src/canvas/list.rs:~190-284`); `push_call`, `push_plain` (`core/faraday-draw/src/canvas/list.rs:~287-306`); `record_*` hooks (`core/faraday-draw/src/canvas/list.rs:~310-487`).
- Bands (`core/faraday-draw/src/canvas/list/bands.rs`): `Room` (`core/faraday-draw/src/canvas/list/bands.rs:~27-36`, default `core/faraday-draw/src/canvas/list/bands.rs:~100-110`); `Band` (`core/faraday-draw/src/canvas/list/bands.rs:~38-58`); `edge`, `grown`, `bounds_of`, `path_box` (`core/faraday-draw/src/canvas/list/bands.rs:~61-98`); `barriers_in` (`core/faraday-draw/src/canvas/list/bands.rs:~124-126`); `replay_banded`, `paint_run` (`core/faraday-draw/src/canvas/list/bands.rs:~134-209`); `Band::paint`, `limits` (`core/faraday-draw/src/canvas/list/bands.rs:~215-231`); `Band::entry` (`core/faraday-draw/src/canvas/list/bands.rs:~233-332`); `path_op` (`core/faraday-draw/src/canvas/list/bands.rs:~335-386`); `Band::clear` (`core/faraday-draw/src/canvas/list/bands.rs:~389-399`); `Band::text` (`core/faraday-draw/src/canvas/list/bands.rs:~403-421`); `Band::rounded` (`core/faraday-draw/src/canvas/list/bands.rs:~425-464`); `Band::shaped` (`core/faraday-draw/src/canvas/list/bands.rs:~477-550`); `clear_scratch` (`core/faraday-draw/src/canvas/list/bands.rs:552-565`).
- Geometry (`core/faraday-draw/src/geom.rs`): `Dp` (`core/faraday-draw/src/geom.rs:~12`); `Scale` (dpi, type percent, narrow; `px`, `px_f`, `factor`, `type_factor`; `core/faraday-draw/src/geom.rs:~21`); `Scale::IDENTITY` (`core/faraday-draw/src/geom.rs:34`); `Scale::new` (`core/faraday-draw/src/geom.rs:~42`), `for_class` (`core/faraday-draw/src/geom.rs:~52`), `with_class` (`core/faraday-draw/src/geom.rs:~67`); `SizeClass` Small, Mobile, Wide and `of` (`core/faraday-draw/src/geom.rs:~113`, `core/faraday-draw/src/geom.rs:126`); `WindowClass` Compact, Medium, Expanded, Large, ExtraLarge (`core/faraday-draw/src/geom.rs:~146`); `Window` and `of` (`core/faraday-draw/src/geom.rs:~163`, `core/faraday-draw/src/geom.rs:~174`); `Size`, `Point`, `Rect`, `Edges`, `PxEdges` (`core/faraday-draw/src/geom.rs:~225-365`).
- `LayoutCtx` (scale, class, top and bottom insets; `core/faraday-draw/src/layout.rs:7`, `new` `core/faraday-draw/src/layout.rs:23`).
- Text (`core/faraday-draw/src/text.rs`): `Font` (`core/faraday-draw/src/text.rs:~16`), `Font::sized` (`core/faraday-draw/src/text.rs:~58`), `TextAlign` (`core/faraday-draw/src/text.rs:~78`), `TextMetrics` (`core/faraday-draw/src/text.rs:~90`), `width`, `measure` (`core/faraday-draw/src/text.rs:~100`, `core/faraday-draw/src/text.rs:~114`), `wrap` (`core/faraday-draw/src/text.rs:~128`), `wrapped_height` (`core/faraday-draw/src/text.rs:~279`).
- Shapes (`core/faraday-draw/src/shapes.rs`): `ShapeKey` (`core/faraday-draw/src/shapes.rs:~27`), `Shapes` cache (`core/faraday-draw/src/shapes.rs:~68`), `fill` (`core/faraday-draw/src/shapes.rs:~144`), `draw` (`core/faraday-draw/src/shapes.rs:~193`).
- Easing (`core/faraday-draw/src/easing.rs`): `ease` (`core/faraday-draw/src/easing.rs:~34`), `ease_path` (`core/faraday-draw/src/easing.rs:~40`), `segment`/`cubic` (`core/faraday-draw/src/easing.rs:~9`, `core/faraday-draw/src/easing.rs:~19`).
- Fonts (`core/faraday-draw/src/fonts/mod.rs`, `core/faraday-draw/src/fonts/generated.rs`): `Family` Regular, SemiBold, RegularNarrow, SemiBoldNarrow, Mono, Icon, Cjk (`core/faraday-draw/src/fonts/mod.rs:~20-47`); `Face`/`FACES` (one `include_bytes!` outline file per family from `assets/*.outl`, `core/faraday-draw/src/fonts/generated.rs:~8-37`, `core/faraday-draw/src/fonts/mod.rs:~50-56`); `Outlines` (FDOL v1 parser, `core/faraday-draw/src/fonts/mod.rs:~70-226`); `Outline`/`Resolved` (`core/faraday-draw/src/fonts/mod.rs:~87-95`, `core/faraday-draw/src/fonts/mod.rs:~237-244`); `SizedFace` (`core/faraday-draw/src/fonts/mod.rs:~259-333`); `Coverage`/`Glyph` (`core/faraday-draw/src/fonts/mod.rs:~337-373`); `GlyphCache` (`core/faraday-draw/src/fonts/mod.rs:~402-477`: `hold` `core/faraday-draw/src/fonts/mod.rs:~432`, `release` `core/faraday-draw/src/fonts/mod.rs:~439`, `peek` `core/faraday-draw/src/fonts/mod.rs:~448`, `glyph` `core/faraday-draw/src/fonts/mod.rs:~460`); `rasterise` (`core/faraday-draw/src/fonts/mod.rs:~498-579`).
- Icons (`core/faraday-draw/src/widgets/icon.rs`): `Icon`, 70 variants (`core/faraday-draw/src/widgets/icon.rs:~18-170`), `ALL` (`core/faraday-draw/src/widgets/icon.rs:~172-248`), `flipped` (`core/faraday-draw/src/widgets/icon.rs:~253`), `code_point` (`core/faraday-draw/src/widgets/icon.rs:~257-329`), `name` (`core/faraday-draw/src/widgets/icon.rs:~332`); `Icon::SEARCH` alias of `Icon::Verify` (`core/faraday-draw/src/widgets/icon.rs:~175`).

## Actions

None; it has no input.

## Effects

- Pixels of the canvas; the shell presents them. `luma` draws camera frames handed to it.
- `Ink` records (text with colour and cut flag, icon, progress, dot, wordmark, and box) when recording is on, for tests and audits.

## State and lock

- `Canvas` keeps between frames: the pixmap (last frame), the glyph cache, the shape cache (app name and mark), `cut` (`Vec<faraday_text::Text>`, strings that ran past their clip), `ink` and the call list.
- Lock: `Canvas::forget_glyphs` drops every cached glyph bitmap so nothing of a shown secret stays in the cache (`core/faraday-draw/src/canvas.rs:405-409`). It is called by `impl AtLock for Canvas` (`faraday/faraday-core/src/at_lock.rs:106`) and by `lock_secrets_off_screen` (`faraday/faraday-core/src/vault/seal.rs:197`). It clears glyph bitmaps only: the pixmap, the shape cache and `cut` are not touched.
- `cut` and `ink` are cleared at the next `Canvas::clear` (`core/faraday-draw/src/canvas.rs:344-350`), not at Lock. Whether a lock draws a blank page over the last frame is outside this crate.
- The call list's `text` (may hold a seed's words) is a `faraday_text::Text` and is zeroized on every `List::clear` (`core/faraday-draw/src/canvas/list.rs:131-140`, `core/faraday-draw/src/canvas/list.rs:136-138`; `Text::clear` at `core/faraday-text/src/text.rs:307-309`).
- Band scratch rows that were written are zeroed after each band paints (`core/faraday-draw/src/canvas/list/bands.rs:552-565`).
- `GlyphCache` holds bitmaps keyed by (family, px, code point), byte count and a held flag; `Shapes` holds coverage masks by key, byte count, filled counter and a held flag.
- The canvas belongs to the app. At a lock it ends with the process, or with the old app dropped in the not-kept rebuild. `cut`'s texts wipe as they drop (`core/faraday-text/src/text.rs:363`). The lock itself only forgets the glyphs (`faraday/faraday-core/src/at_lock.rs:101-108`, `faraday/faraday-core/src/vault/seal.rs:192-199`).

## Reach

- Everything drawn in every app goes through it: `faraday-ui` components, the shells' presenters, the snapshot tours (`Ink` records).
- Faraday Offline's lock (`faraday/faraday-core/src/at_lock.rs`, `faraday/faraday-core/src/vault/seal.rs`) calls `forget_glyphs`.
- Motion (`docs/MOTION.md` §3.5 per the blend module doc) uses the blends.
- Tokens (`tokens`) give the elevation layers, easing curves and size breakpoints.

Callers at main `d89dde5de`. The apps mostly draw through `faraday-ui`'s `Ui` (its `ui.c` is the `Canvas`); direct uses of the base are these (paths under `faraday/faraday-core/src/` unless a crate is named):
- Canvas made and owned: `drawing.rs:1011` (`Canvas::new` for the frame), `drawing.rs:782,790` (`canvas`, `canvas_mut`); `picture.rs:78` (a QR code as a PNG); `faraday/faraday-starter/src/lib.rs:2763` and `faraday/faraday-starter/examples/icon.rs:32`; in tests of screens: `codex32_correct/screen.rs:572`, `decode/screen.rs:643`, `fresh/roller/five_screen.rs:391`, `screens/session_pane/pick.rs:525,549,594`.
- Lock: `forget_glyphs` is called at `at_lock.rs:106` (`impl AtLock for Canvas`; the canvas is an area at `at_lock.rs:184`).
- Ink records: `drawn_ink` at `drawing.rs:655,673,683,758`; `faraday/faraday-starter/src/draw.rs:2664`, `faraday/faraday-starter/src/lib.rs:1651,1684` and the Starter's `examples/snap.rs:242-246`; `InkKind` is also read by `leaks.rs` and the snapshot tours (`faraday/shells/snapshot`, 7 mentions of `faraday_draw`).
- Frame blends (motion): `fade.rs:235,355,373,935,1004` call `blend_all_within`, `blend_within` and `blend_noting` of `fade/bands.rs:80-128`, which call `Canvas::blend_noting_banded`, `blend_within_banded` and `blend_all_within_banded` (`fade/bands.rs:90,109,128`); `fade/bands.rs:149,152` call `replay` and `replay_banded` (the call list).
- Easing: `motion.rs:184` (`easing::ease`), `glide.rs:13` (`ease_path` with `EASE_EMPHASIZED`).
- Camera luma: `screens/scan_frame.rs:21` (`ui.c.luma`).
- Geometry and display classes: `Rect` in `glide.rs`, `moved.rs`, `pointer.rs`, `drawing.rs`, `fade.rs`, `screens/parts.rs`, `screens/scan_frame.rs`, `faraday/faraday-starter/src/draw/home.rs`, `faraday/faraday-starter/src/scroll.rs`; `Window`/`WindowClass`/`SizeClass`/`Scale` in `codex32/`, `nav/`, `panes/`, `drawing.rs`, `guide.rs`, `keygen/`, `kef_form/`, `screens/parts.rs`, `faraday/faraday-starter/src/lib.rs:230`, `faraday/shells/stick/src/edid_screens.rs:15`; `LayoutCtx` in `screens/parts.rs`.
- Text: `wrapped_height` in `picture.rs`; the rest of text measure and wrap through `faraday-ui` (`ui-text`).
- Keyboard and icons: `keyboard` items by `screens/parts.rs`, `word_entry.rs`, `codex32/`, `codex32_backup/`, `compact/`, `screens/entry.rs` (see `keyboard`); `Icon` is imported from `faraday_draw::widgets` by about 70 modules (every area that draws an icon, and the Starter's `draw/*.rs`).
- Tokens: read by app modules per the `tokens` file; `faraday/shells/stick/src/camera.rs:353` reads `NO_PICTURE_MS`.
- The shells present the canvas through the presenter (`faraday/shells/stick/src/main.rs:1229-1349`: `settle`, `skipped`, `present_parts`).
- Not used by any app module: `Surface`, `GlyphCache`, `Shapes` and `ShapeKey` as names (reached through `Canvas`).

## Rules and refusals

Canvas and primitives:
- Every primitive honours the clip; a zero-alpha colour is skipped (in `core/faraday-draw/src/canvas.rs`: `fill_rect` `core/faraday-draw/src/canvas.rs:~445`, `fill_rounded_rect` `core/faraday-draw/src/canvas.rs:~1015`, `line` `core/faraday-draw/src/canvas.rs:~1073`, `fill_circle` `core/faraday-draw/src/canvas.rs:~1111`). `Canvas::part`/`new` panics on a zero size.
- Hidden rows below the visible ones keep coverage of shapes past the last row (`core/faraday-draw/src/canvas.rs:~32-36`, `core/faraday-draw/src/canvas.rs:~1754-1758`).
- An opaque rounded rect at least `SPLIT_ROWS` (64) rows tall is a plain middle fill with path corners; radius 0.5 or less is a plain rect (`core/faraday-draw/src/canvas.rs:~126`, `core/faraday-draw/src/canvas.rs:~1032`).
- Elevation levels are clamped to 1..5; key and ambient layers come from the tokens, scaled by the dp scale (`core/faraday-draw/src/canvas.rs:~547-567`).
- Frost samples every other pixel at scale 16 or more (`core/faraday-draw/src/canvas.rs:~592`); it is meant to be done once and kept by `snapshot`/`restore`.
- A text line wholly above or below the clip (with one line height spare) is not drawn or counted (`core/faraday-draw/src/canvas.rs:~1245-1250`).
Surface:
- Alpha 0 paints nothing; alpha 1-254 is blended by hand per pixel; 255 is opaque through tiny-skia with anti-aliasing off (`core/faraday-draw/src/canvas/surface.rs:~43`, `core/faraday-draw/src/canvas/surface.rs:~46`, `core/faraday-draw/src/canvas/surface.rs:~77`); the blend sums colour and pixel before one division so an alpha ramp does not step (`core/faraday-draw/src/canvas/surface.rs:~47-72`).
- Paths fill with the winding rule (`core/faraday-draw/src/canvas/surface.rs:~444`). A shape wholly outside the clip is skipped (`core/faraday-draw/src/canvas/surface.rs:~418-426`); a shape crossing the clip is drawn whole, then the pixels outside the clip are restored (`core/faraday-draw/src/canvas/surface.rs:~430-454`).
- Shadow: fall length `blur * 1.2` (`core/faraday-draw/src/canvas/surface.rs:~181`, `core/faraday-draw/src/canvas/surface.rs:~218`); fall table at 4 steps per px (`core/faraday-draw/src/canvas/surface.rs:~249`); curve `ca * (1-t)^2 * (1+2t)` (`core/faraday-draw/src/canvas/surface.rs:~252-253`); spread growth `spread - blur/2`, rounded away from zero (`core/faraday-draw/src/canvas/surface.rs:~179-180`); reach `fall_len + 1` px (`core/faraday-draw/src/canvas/surface.rs:~188`); radius clamped to half width and height (`core/faraday-draw/src/canvas/surface.rs:~220-223`, `core/faraday-draw/src/canvas/surface.rs:~236-239`); covered middle inset `radius + 1` and skipped (`core/faraday-draw/src/canvas/surface.rs:~224`).
- Luma: scale is the larger ratio, crop equal on both sides of the longer axis, nearest sample, chroma interleaved one pair per two luma pixels, output opaque (`core/faraday-draw/src/canvas/surface.rs:~309-334`).
- Ink box: half stroke width plus 1 for anti-aliasing plus 1 for truncation, then one pixel more each side (`core/faraday-draw/src/canvas/surface.rs:~389-397`).
Call list and bands:
- Calls that read or write the whole canvas replay the waiting list first (`core/faraday-draw/src/canvas/list.rs:~215-219`); turning recording off replays first (`core/faraday-draw/src/canvas/list.rs:~147-149`); while recording, glyph and shape caches are held.
- Barriers (`Frost`, `Shift`) end a run and are painted over the whole canvas (`core/faraday-draw/src/canvas/list.rs:~295`, `core/faraday-draw/src/canvas/list/bands.rs:~145-160`, `core/faraday-draw/src/canvas/list/bands.rs:~328-330`).
- Cells are packed one bit each (`core/faraday-draw/src/canvas/list.rs:~328-336`).
- Damage boxes: rounded rect grows 3 px; line and closed stroke `w + 3`; circle `r + 3`; contours 3 px (`core/faraday-draw/src/canvas/list/bands.rs:~85-97`); `grown` adds 1 px before and 2 after (`core/faraday-draw/src/canvas/list/bands.rs:~65-69`).
- Band rows: boundary `i * rows / n` (`core/faraday-draw/src/canvas/list/bands.rs:~61-63`); band count clamped to `1..=rows` (`core/faraday-draw/src/canvas/list/bands.rs:~138`); the last band holds the hidden rows (`core/faraday-draw/src/canvas/list/bands.rs:~185-189`, `core/faraday-draw/src/canvas/list/bands.rs:~229`); `clear` fills visible rows only (`core/faraday-draw/src/canvas/list/bands.rs:~394`).
- A path is never moved: drawn at canvas coordinates, into the band directly if its ink lies wholly in the first band, else through the room's scratch (`core/faraday-draw/src/canvas/list/bands.rs:~470-550`).
- Text in a band: pen in 26.6 fixed point, `+32` rounding, origin clamped to -32768, baseline `y + ascent` (`core/faraday-draw/src/canvas/list/bands.rs:~409-415`); skipped when wholly outside the band by two line heights above or one below (`core/faraday-draw/src/canvas/list/bands.rs:~305-307`).
- Rounded rect in a band: plain rect at radius 0.5 or less (`core/faraday-draw/src/canvas/list/bands.rs:~428`); the solid split only for `color.a == 255` and a middle of at least `SPLIT_ROWS` rows, cap `radius + 2` (`core/faraday-draw/src/canvas/list/bands.rs:~437-438`).
Blend:
- `t` stored as 0..256 (clamped 0..1); per byte `(was*(256-t) + will*t) >> 8`; `t == 256` copies the new run (`core/faraday-draw/src/canvas/blend.rs:~149`, `core/faraday-draw/src/canvas/blend.rs:~252`, `core/faraday-draw/src/canvas/blend.rs:~312`, `core/faraday-draw/src/canvas/blend.rs:~418`, `core/faraday-draw/src/canvas/blend.rs:~439`).
- Runs of 64 bytes (16 pixels); runs where the frames agree are copied only when `agreeing` (`core/faraday-draw/src/canvas/blend.rs:~404-443`).
- The excluded edge ring is clamped to half width and height (`core/faraday-draw/src/canvas/blend.rs:~152-155`); at most 256 difference rectangles, joined two by two beyond that, join slack 64 px (`core/faraday-draw/src/canvas/blend.rs:~453`, `core/faraday-draw/src/canvas/blend.rs:~456`, `core/faraday-draw/src/canvas/blend.rs:~476`).
Geometry and text:
- 1 dp = 1/160 inch; 160 dpi is identity (`core/faraday-draw/src/geom.rs:~10`, `core/faraday-draw/src/geom.rs:~35`, `core/faraday-draw/src/geom.rs:~84`, `core/faraday-draw/src/geom.rs:175`).
- Size class by diagonal inches: under 3.5 Small, under 7.0 Mobile, else Wide (`core/faraday-draw/src/geom.rs:126-136`).
- `Window::of` treats a text factor below 1 as 1 (`text_factor.max(1.0)`, `core/faraday-draw/src/geom.rs:175`); window class breakpoints are the `WINDOW_*` tokens (`core/faraday-draw/src/geom.rs:~184-191`); short under `WINDOW_SHORT`, low under `WINDOW_LOW` (`core/faraday-draw/src/geom.rs:~201`, `core/faraday-draw/src/geom.rs:~207`).
- `round` is half away from zero, by hand (`core/faraday-draw/src/geom.rs:~98-106`).
- Font size is dp times the type factor, rounded half up, at least 1 px, stored as u16 (`core/faraday-draw/src/text.rs:~72`); the icon font ignores the type scale (`core/faraday-draw/src/text.rs:~54-63`); Regular and SemiBold take their narrow cuts when the class is narrow, Mono and Icon do not (`core/faraday-draw/src/text.rs:~67-71`).
- Widths are summed in 1/64 px and rounded `+32 >> 6` (`core/faraday-draw/src/text.rs:~109-111`); the line-fit test compares rounded pixel widths so a line measured to fit does not wrap when drawn (`core/faraday-draw/src/text.rs:~189-191`); spaces count toward width (`core/faraday-draw/src/text.rs:~213-219`); `\n` forces a break; an over-wide word splits after the last hyphen that fits, else between characters (`core/faraday-draw/src/text.rs:~225-254`); always at least one line.
Caches:
- Shape cache: 1 MiB and 32 shapes; a miss beyond either clears all kept shapes (not LRU); no eviction while held (`core/faraday-draw/src/shapes.rs:43`, `core/faraday-draw/src/shapes.rs:46`, `core/faraday-draw/src/shapes.rs:96`, `core/faraday-draw/src/shapes.rs:129-133`). The key does not include colour, so a theme change needs no refill (`core/faraday-draw/src/shapes.rs:~20-25`).
- Shape blend: opaque uses tiny-skia's Source lerp, otherwise source-over (`core/faraday-draw/src/shapes.rs:~228-240`); low-precision /255 is `(v+255)>>8` (`core/faraday-draw/src/shapes.rs:~247-249`).
- Glyph cache: 256 KiB budget, then a whole clear (`core/faraday-draw/src/fonts/mod.rs:~375-381`, `core/faraday-draw/src/fonts/mod.rs:~467-471`); `hold()` keeps all glyphs while a recorded frame waits, `release()` trims.
Fonts and icons:
- Glyphs are stored once in font units and rasterised at the requested px (`core/faraday-draw/src/fonts/mod.rs:~1-8`) with scalar tiny-skia, so coverage bytes match on all architectures (`core/faraday-draw/src/fonts/mod.rs:~492-497`).
- Text faces fall back to the CJK face, then to `?`; Icon and Cjk do not fall back (`core/faraday-draw/src/fonts/mod.rs:~201-225`).
- `SizedFace::new` rounds ascent and descent up and the line gap to the nearest px (`core/faraday-draw/src/fonts/mod.rs:~287-303`).
- Each icon variant maps to one code point; `ChevronDown` reuses `ChevronUp`'s U+E90B drawn flipped (`core/faraday-draw/src/widgets/icon.rs:~253`, `core/faraday-draw/src/widgets/icon.rs:~257-329`); code points U+E9xx (SeedSigner) and U+F0xx-F7xx (Font Awesome) per the module doc (`core/faraday-draw/src/widgets/icon.rs:~3-6`).
Easing:
- Newton solve, 8 iterations, early stop when the slope is near zero (1e-6); output clamped to 0..1 (`core/faraday-draw/src/easing.rs:~21-27`, `core/faraday-draw/src/easing.rs:~34-35`, `core/faraday-draw/src/easing.rs:~41-42`); a path uses its first segment below `join[0]`, else the second (`core/faraday-draw/src/easing.rs:~42-46`).

## Domain calls

- `faraday_text::Text` (`core/faraday-text/src/text.rs`) for `cut` and the list's text buffer.
- tiny-skia (no-std, scalar, default features off) for paths and rasterising.
- `core/faraday-draw/src/tokens.rs`: `ELEVATION_KEY`, `ELEVATION_AMBIENT`, `type_scale_pct`, `narrow_text`, `WINDOW_*`, `Easing`, `EasingPath`.
- Public re-exports from the crate root (`core/faraday-draw/src/lib.rs:~23-26`).

## Fixtures

None.

## Fixes

None given to this feature.

## Doubts

- `Scale::IDENTITY` is test-only in effect: its other uses (`core/faraday-draw/src/text.rs:290`, `core/faraday-draw/src/widgets/keyboard.rs:930`) sit after `#[cfg(test)]` lines.
- One oversize shape can exceed the 1 MiB shape budget on its own (`core/faraday-draw/src/shapes.rs:129`).
- `ease_path` divides by `1 - join[0]`; if `join[0]` could be 1 this divides by zero (`core/faraday-draw/src/easing.rs:~44-45`).
- The source of the glyphs for Method, Pin, Pinned, Verified and Vault (U+E930-E933, U+E2C5) is not in the sets the icon module doc names.
- `SPLIT_ROWS` is defined in the canvas module, not read by the inventory in the band code.
