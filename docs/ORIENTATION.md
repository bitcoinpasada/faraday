# Panel orientation

**Kind:** Spec
**Status:** Specified (F100), partly built: native orientation, the panel's density and the known list (O1)
**Open:** the shell turning its frame, touch and arrow (O2), the setting, its line and the turn without a restart (O3), the snapshot sizes, the guide and the docs that follow (O4).

A 7-inch or 10-inch panel starts in landscape. Settings › Display gains
**Rotation**: Landscape, Portrait. A panel that is portrait natively is
turned to landscape by the app, and its touch input with it. The choice
is kept on the stick. Changing it lays the screen out again at the
class its new width gives (`docs/LAYOUT.md` §2), with no restart. The
owner's choices are F100, which amends F92.

## 1. What the display path is today

- **The stick.** The firmware sets a mode and hands its framebuffer
  over; `sysfb` and `efifb` or `simplefb` publish it as `/dev/fb0`
  (`faraday/image/boards/x86_64-uefi/linux.fragment`, "display"). The
  kernel has no DRM: `# CONFIG_DRM is not set` in
  `faraday/image/common/linux.fragment`. The shell reads the size,
  depth and stride from sysfs (`Geometry::from_sysfs`,
  `faraday/shells/stick/src/fb.rs`), converts the core's RGBA frame to
  the panel's format row by row (`fb::convert_into`), compares rows with
  the last frame (`faraday/shells/stick/src/present.rs`) and writes the
  rows that differ with `pwrite` in bands of 64 rows
  (`faraday/shells/stick/src/panel.rs`). The mouse arrow is drawn into
  those bands in panel pixels (`faraday/shells/stick/src/cursor.rs`).
  Nothing is turned: the frame is the framebuffer's size and
  orientation.
- **Density on the stick.** `faraday.dpi=160` on the kernel command line
  (`faraday/image/boards/x86_64-uefi/cmdline.txt`), doubled on a
  framebuffer 2800 pixels wide or more (F101; `dpi` in
  `faraday/shells/stick/src/main.rs`). `docs/LAYOUT.md` §7 names EDID's
  physical size as the source; that is not read yet. The shell reads the
  EDID base block the EFI stub leaves in the boot parameters only for the
  device check (`faraday/faraday-core/src/edid.rs`, `from_boot_params`).
- **Touch on the stick.** Every `/dev/input/event*` is classified by
  what it reports (`faraday/shells/stick/src/evdev.rs`). A touchscreen's
  position is scaled from the controller's grid to panel pixels and
  clamped (`Parser::transform`, `faraday/shells/stick/src/touch.rs`); a
  HID touchscreen's grid is the ranges its report descriptor declares
  (`faraday/shells/stick/src/hid.rs`). There is no swap or inversion of
  axes.
- **The Pi.** The firmware drives the 2.8-inch DPI panel, 480 × 640
  portrait at 286 dpi, and hands Linux its framebuffer through
  `FB_BCM2708` (`faraday/image/panels/waveshare-28dpi/panel.conf`,
  `faraday/image/panels/waveshare-28dpi/config.txt`). The Goodix
  controller reports in a 640 × 480 grid, which the shell scales to the
  panel (`TOUCH_GRID`). There is no 7-inch or 10-inch panel directory
  under `faraday/image/panels/`.
- **Settings.** The stick keeps its settings in `faraday-settings.txt`,
  one `key=value` line each (`scale=`, `theme=`, `mode=`, `contrast=`,
  `motion=`, `guided=`, `qr-ms=`, `idle-lock=`, `idle-off=`, `tz=`;
  `Settings::body` and `Settings::apply`,
  `faraday/faraday-core/src/stick/settings.rs`). Settings' sections are
  Mode, Session, Time, Randomness, Appearance, Display scale, Signed
  amounts, Stick and About (`Part`,
  `faraday/faraday-core/src/screens/settings.rs`); Display scale is one
  row of choices. A scale chosen lays the screen out again at once
  (`layout_scale`: the last `DisplayInfo` given to `display` again).

So every 7-inch or 10-inch panel today is a stick on a PC: a small
tablet or panel PC with its own screen, or a portable monitor on HDMI or
USB-C. Such a panel that is portrait natively (common in small x86
tablets: 800 × 1280, 1200 × 1920) is drawn portrait.

## 2. How the app knows a panel is portrait natively

Three things are read once at start, in this order:

1. **The framebuffer's size.** A UEFI firmware hands over the panel's
   native mode, so a framebuffer taller than wide is a panel portrait
   natively. This is always there and is what decides.
2. **The EDID base block** from the boot parameters, when the firmware
   passed one: the first detailed timing descriptor's active pixels
   (bytes 56 to 61) and image size in mm (bytes 66 to 68), or the
   screen's size in cm (bytes 21 and 22) where that descriptor carries
   none. It confirms the native size, gives the diagonal (§5: whether
   Rotation is offered) and the density (§2.1). A descriptor whose
   active size is not the framebuffer's (the firmware chose another
   mode) is not used for the density.
3. **The machine's known list**: which way the panel is mounted in its
   case, by the machine's DMI vendor and product name (the shell already
   reads `sys_vendor` and `product_name`). No signal reports this. The
   kernel keeps such a table (`drivers/gpu/drm/drm_panel_orientation_quirks.c`
   in its source); Faraday's list starts from that table's 7-inch to
   10-inch machines and is kept in the stick shell as data, one row per
   machine: vendor, product, and the quarter turn (90 or 270) that
   shows the frame upright in the orientation that is not the panel's
   native one. A machine not on the list takes 90. The native
   orientation is always shown at 0. The list is `MACHINES` in
   `faraday/shells/stick/src/orient.rs`: the table's machines of 6 to
   10 inches that it matches by `sys_vendor` and `product_name` alone,
   its "right side up" a turn of 90 and its "left side up" 270.

The touch device's axes are not used to decide. A HID touchscreen
reports in the panel's native axes; the shell compares the aspect of its
ranges with the framebuffer's, and a controller whose axes are swapped
or reversed against the panel is a known-list row (§4.2), never a guess.

A panel directory for a soldered panel states its native orientation
and mounting in `panel.conf`, as it states its size and density: a new
`NATIVE=portrait|landscape` and `MOUNT=90|270`, the quarter turn for the
other orientation. The image writes
them into the shell's arguments as it writes `--touch-grid`.

### 2.1 The density of a panel

A 7-inch panel at `faraday.dpi=160` lays out at its pixels: 1024 × 600
gives 1024 × 600 dp, and portrait 600 × 1024 dp is medium, not the
compact F100 names. At its own density (170 dpi) it is 964 × 565 dp,
expanded and low, and portrait 565 × 964 dp, compact. On a screen
whose EDID diagonal is 11 inches or less, the density is the EDID's
(pixels over inches across the diagonal), as `docs/LAYOUT.md` §7 says;
a larger screen and a screen with no EDID keep `faraday.dpi`. The
person's Display scale multiplies either.

The edges (`orient::edid_dpi`, `faraday_core::edid::size`): the
detailed timing's image size is read only where it is the shape of its
pixels, within a tenth (some panels report a placeholder, 60 × 60 mm
for an 800 × 1280 panel), and the screen's size in cm takes its place
where it is; a density outside 96 to 480 dpi is not believed; a
diagonal of exactly 11 inches (279.4 mm) is still small.

## 3. Where the frame is turned

| Way | What it needs | Turns 90 and 270 | Follows the setting at once | Touch | Cost |
|---|---|---|---|---|---|
| DRM plane rotation | `CONFIG_DRM`, a KMS driver for the GPU, a plane with a `rotation` property | only where the hardware has it: most PC GPUs offer 0 and 180 on the primary plane; the Pi's vc4 offers reflections, not quarter turns | yes | turned by the shell | brings the kernel's whole modesetting stack into an image that has kept it out; `efifb` and `simplefb` have no planes |
| Kernel console rotation (`fbcon=rotate:N`, `video=…,rotate=`) | the kernel command line, built into the stick's signed kernel | the console and boot logo only; `/dev/fb0` as the app sees it is not turned | no: fixed when the image is built | not turned | none, and none of the app's frame |
| The Pi firmware's `display_lcd_rotate` | `config.txt` | the legacy firmware framebuffer only | no: read before Linux starts | not for a DPI panel's Goodix | the Pi only |
| The shell turns its own frame | the shell | all four | yes | turned by the shell, the same map | a transposed walk of the frame when it is converted |

**The shell turns its own frame.** It is the one way that works on
every machine the stick starts, on the firmware framebuffer the image
uses today, at all four turns, and at a press in Settings. It keeps DRM
out. The cost is in conversion, which already touches every pixel of
every changed row:

- The core draws a frame at the turned size: on a 1280 × 800 panel
  turned a quarter, 800 × 1280. `DisplayInfo` carries the turned width
  and height; the core knows nothing else of the turn.
- `fb.rs` gains a conversion of a band of the core's rows into the
  panel buffer at the turn: rows as now at 0, rows reversed at 180,
  columns at 90 and 270, walked in tiles of 16 × 16 pixels so a column's
  writes stay in cache.
- `present.rs` compares the core's rows with its last frame as now. At
  0 and 180 the rows that changed are panel rows; at 90 and 270 they are
  a band of panel columns, and `Panel` writes, for each panel row, only
  the bytes of that band (one `pwrite` per row). A caret's blink at 90
  is about 800 short writes on a 1280 × 800 panel; the timing of §9's
  late-flow session decides whether that stands or whole rows are
  written instead.
- The arrow is drawn at its turned place with its bitmap turned
  (`cursor.rs`, the four bitmaps made once at start).

The boot logo and anything the kernel draws are not turned: they are
drawn before the app runs, at the panel's native orientation (§8).

## 4. Touch and the pointer

### 4.1 The map for each turn

The panel is `W × H` pixels natively. The turn `r` is how far the
core's frame is turned clockwise to sit on the panel. The core's frame
is `W × H` at 0 and 180, `H × W` at 90 and 270. A touch is first scaled
from the controller's grid to a panel pixel `(x, y)`, as
`Parser::transform` does now, then mapped to the core's `(u, v)`:

| Turn | Panel pixel of the core's `(u, v)` | Core's `(u, v)` of a touch at `(x, y)` |
|---|---|---|
| 0 | `(u, v)` | `(x, y)` |
| 90 | `(H − 1 − v, u)` | `(y, W − 1 − x)` |
| 180 | `(W − 1 − u, H − 1 − v)` | `(W − 1 − x, H − 1 − y)` |
| 270 | `(v, W − 1 − u)` | `(H − 1 − y, x)` |

The same map, one function, serves the conversion (left column) and the
touch (right column), so a touch lands where the frame was drawn. It
lives in a new module of the stick shell with its tests; `touch.rs`
calls it after its scaling and clamping, and `present.rs` and
`cursor.rs` call it for the frame and the arrow.

### 4.2 Calibration

Before the turn, a controller's own axes are put to the panel's:

- **Grid.** As now: `--touch-grid` from `panel.conf`, or the HID
  descriptor's ranges.
- **Swap and reversal.** A controller whose x runs along the panel's
  height, or runs right to left, is corrected by `TOUCH_SWAP=1`,
  `TOUCH_INVERT_X=1`, `TOUCH_INVERT_Y=1` in `panel.conf`, or by the
  machine's row on the known list. These describe the hardware and do
  not change with the setting.
- **The turn.** §4.1, from the setting and the mounting.

So the calibration for each of the four turns is the same grid, swap and
reversal, then that turn's row of §4.1. A test (§9) checks each.

### 4.3 The mouse and the keys

A mouse's motion is the person's: right is right on the screen at
every turn, so the arrow moves in the core's coordinates and is drawn
through the map. A touchpad's relative motion is the same. The keyboard
is not affected; the arrow keys move as the screen shows.

## 5. Which screens it applies to

| Screen | Rotation |
|---|---|
| A stick on a panel whose EDID diagonal is 6 to 11 inches, or whose framebuffer is taller than wide | offered: the shell sends `Event::RotationOffered` (§6) |
| A stick on a laptop's screen or a monitor (wider than tall, over 11 inches or no EDID) | not offered; drawn as now |
| The Pi's 2.8-inch panel | not offered: it is a handheld portrait panel, compact either way (480 × 640 at 286 dpi is 268 × 358 dp; turned, 358 × 268, compact and short). It stays as it is |
| A Pi panel directory for a 7-inch or 10-inch panel, when one is added (`docs/STARTER.md` §9) | offered, by its `panel.conf`; the same shell code |
| The desktop app (Faraday Demo, a test build) | not offered: it is a window, and the window's shape is its own. Faraday Demo's Size menu shows the panels at both orientations (§10) |
| The Starter | not affected |

## 6. The setting

- **Type.** `faraday_shell_api::Orientation`: `Landscape`, `Portrait`,
  with `id()` (`landscape`, `portrait`) and `from_id`. The default is
  `Landscape` (F100).
- **Shell to core.** `Event::RotationOffered { native: Orientation }`,
  sent after `Event::Display` by a shell that can turn its frame (§5).
  A shell that cannot never sends it, and Settings shows no Rotation
  row.
- **Core to shell.** `Command::Orientation(Orientation)`, sent when the
  setting changes and once after `RotationOffered` with the kept
  choice. The shell works out the turn from the native orientation and
  the mounting: the chosen orientation equal to the native one is 0,
  the other the mounting's quarter turn. 180 and the quarter turn
  opposite the mounting's are drawn by the same code and tested, and
  are not offered (an upside-down choice is not in F100).
- **Settings.** The section **Display scale** becomes **Display**,
  holding two rows: **Scale**, the choices it has now, and, where
  offered, **Rotation**, a Material 3 segmented button of two segments,
  Landscape and Portrait, the chosen one selected, as the Contrast and
  Scale rows are drawn (`Ui::choices`). On a compact window the Display
  section opens as its own page as every section does
  (`docs/LAYOUT.md` §17). Each segment is a 48 dp touch target.
- **Strings**, in `faraday/faraday-strings/src/en.rs`:
  `settings_display` "Display", `settings_scale` "Scale",
  `settings_rotation` "Rotation", `settings_landscape` "Landscape",
  `settings_portrait` "Portrait". Guided mode's explainer:
  `settings_guide_rotation` "Landscape suits the backup board and lists
  beside their detail. Portrait suits typing words in."
- **Kept.** One line in `faraday-settings.txt`, `rotation=landscape` or
  `rotation=portrait`, written by `Settings::body` after `scale=` and
  taken by `Settings::apply` only when its value is one of the two; any
  other value leaves Landscape. It is read before the passphrase screen
  with the theme and scale. The kept set the other shells store
  (`Command::StoreSettings`) carries the same line. The file's header
  stays `faraday-settings 1`: a line an older build does not know is
  ignored by it.

## 7. On change

A press on a segment:

1. sets the setting and sends `Command::Orientation`;
2. the shell sets its turn, marks the panel stale (the next frame is
   written whole), sets the touch parser's and the arrow's bounds to the
   turned size, and sends `Event::Display` with the turned width and
   height and the same density;
3. the core lays out at the new size as for any `Event::Display`: the
   class its new width gives (`docs/LAYOUT.md` §2), the navigation that
   class has (§3 there), Settings with the Display section in view;
4. the kept settings are marked changed, so the next stick visit offers
   the file as it offers a changed theme.

Nothing restarts. The session, its keys and the screen stay. A touch
in progress at the press ends with the press.

## 8. At start

- The shell starts at the panel's landscape turn (0 on a panel
  landscape natively, the mounting's quarter turn on one portrait
  natively), so the first frame is landscape (F100).
- The settings file is read with the theme and scale, before the
  passphrase screen. A kept `rotation=portrait` turns the screen once
  there, as §7 does.
- The kernel's boot logo is drawn by the framebuffer console before the
  app, at the panel's native orientation: on a panel portrait natively
  its centred mark shows a quarter turned until the app's first frame.
  Turning it would take `fbcon=rotate:N` on the stick's built-in
  command line, fixed when the image is built and the same for every
  machine, or the console's rotation built in
  (`CONFIG_FRAMEBUFFER_CONSOLE_ROTATION`), which turns it only for the
  machines the kernel's own table lists.

## 9. Tests

Each states what a person would notice.

In the stick shell (`faraday/shells/stick/src/`, beside the touch and
present tests):

1. **A touch lands where it is drawn, at each turn.** On a 1024 × 600
   panel and an 800 × 1280 panel, at each of 0, 90, 180 and 270, a frame
   with a block drawn at a point of the core's frame is presented to a
   panel in memory; a touch reported by the controller at the panel
   pixel where the block shows (in the controller's grid) reaches the
   core at the block.
2. The same with a HID touchscreen's ranges, and with a controller whose
   axes are swapped and reversed (`TOUCH_SWAP`, `TOUCH_INVERT_X`).
3. A mouse moved right moves the arrow right on the panel at each turn,
   and the arrow is drawn where the core's hover is.
4. After `Command::Orientation`, the shell reports the turned size and
   writes the whole panel once; a frame drawn again with nothing changed
   writes nothing.
5. A caret's blink at 90 writes only the band of columns it covers.

In `faraday/faraday-core/tests/all/` (a new `orientation.rs`, and
`settings_file.rs` for the kept line):

6. **The setting survives a restart.** Portrait chosen, the settings
   file written; a new `Faraday` given that file asks the shell for
   Portrait before the passphrase screen. `rotation=sideways` and
   `rotation=` leave Landscape.
7. **The layout follows the width.** A 1024 × 600 panel at 170 dpi
   shows the rail in Landscape and the navigation bar at the foot in
   Portrait; a 1280 × 800 panel at 150 dpi shows the rail in both.
8. Rotation is on Settings only where the shell offers it: not on the
   desktop app, not on the Pi's panel.
9. A turn keeps the session: with a key loaded, choosing Portrait and
   back leaves the key loaded and Settings on screen.

The core's work budget (`faraday/faraday-core/src/work/budget/`) names the
relayout for a press of Rotation as it names one for Scale. The shell's
turned conversion is work done after every frame drawn: it is timed in a
late-flow session on a 1280 × 800 panel turned a quarter before O2 is
committed (`docs/SPEED.md` §7).

## 10. Snapshots

The snapshot shell renders the core's frame, which is the turned size,
so each orientation is its own size:

| Panel | Landscape | Portrait | Classes |
|---|---|---|---|
| 7-inch, 1024 × 600 | `1024x600@170` | `600x1024@170` | expanded and low; compact |
| 10-inch, 1280 × 800 | `1280x800@150` | `800x1280@150` | large; expanded |

`faraday/faraday.just`'s `faraday-snap` knows the four sizes by name
with their density, as it knows `480x640`, and `faraday-snapshots`
renders all four. `1280x800` already names the desktop window at 160
dpi; the 10-inch panel's landscape render is `1280x800@150`, written to
`out/snapshots/faraday/1280x800-150/`.

Faraday Demo's Size menu (`docs/PREVIEW-SIZE.md` §1) gains **7-inch
panel, portrait** (565 × 964 dp) and **10-inch panel, portrait**
(853 × 1365 dp), and its 7-inch and 10-inch rows take the densities
above (964 × 565 and 1365 × 853), in `PREVIEW_SIZES`
(`core/faraday-draw/src/tokens.rs`).

The 10-inch panel in portrait at its own density is 853 dp wide,
expanded, not the medium of F100 and `docs/LAYOUT.md` §2's table, which
count it at 160 dpi (800 dp). Both classes have the rail beside one
column (`docs/LAYOUT.md` §3), so what F100 describes holds; the table's
row is corrected with O4.

## 11. The guide

`docs/GUIDE.md` gains, after §2.4:

> ### 2.5 Settings → Display
>
> On a 7-inch or 10-inch panel, **Settings → Display → Rotation** turns
> the screen: **Landscape** or **Portrait**. Faraday starts in
> landscape. Landscape suits the backup board and a list beside its
> detail. Portrait suits typing words in: the keyboard leaves more of
> the screen above it. The choice is kept on the stick. **Scale** makes
> text and controls larger or smaller.
>
> Picture: Settings → Display on the 7-inch panel, landscape.

## 12. Batches

Each batch ends with `just` green.

| Batch | What | Model | Size |
|---|---|---|---|
| O1 | Native orientation and density: the EDID detailed timing's size and mm read (`faraday/faraday-core/src/edid.rs`), the diagonal and density rule of §2.1 in the stick shell's `dpi`; `NATIVE`, `MOUNT`, `TOUCH_SWAP`, `TOUCH_INVERT_X`, `TOUCH_INVERT_Y` in `panel.conf` and the shell's arguments (`faraday/image/README.md`, "Adding a panel"); the known list, its first rows from the kernel's table; tests of the EDID read against a captured base block | Opus: which machines go on the list, the density rule's edges | small |
| O2 | The shell turns: the map module, the turned conversion and column-band writes (`fb.rs`, `present.rs`, `panel.rs`), the arrow (`cursor.rs`), touch (`touch.rs`), `Event::RotationOffered`, `Command::Orientation` (`core/faraday-shell-api/src/lib.rs`); tests 1–5; the late-flow timing | Opus: the band writes and the timing call | medium |
| O3 | The setting: `Orientation`, Settings' Display section and its Rotation row, the strings, the `rotation=` line, the turn on change and at start; tests 6–9; the budget's named relayout | Sonnet: specified here | small |
| O4 | The snapshot sizes in `faraday/faraday.just`, the Size menu's portrait rows and densities, the guide's §2.5 and its picture, the docs of §13 | Sonnet | small |

O2 needs O1's arguments; O3 needs O2's event and command; O4 needs O3.

Edit list:

- `faraday/faraday-core/src/edid.rs`: the detailed timing's active size
  and image size.
- `faraday/shells/stick/src/main.rs`: `dpi` (§2.1), the arguments, the
  turn at start, `Command::Orientation`.
- `faraday/shells/stick/src/fb.rs`, `faraday/shells/stick/src/present.rs`,
  `faraday/shells/stick/src/panel.rs`, `faraday/shells/stick/src/cursor.rs`,
  `faraday/shells/stick/src/touch.rs`: §3, §4; a new module for the map
  and the known list.
- `faraday/image/panels/efi-framebuffer/panel.conf`,
  `faraday/image/panels/waveshare-28dpi/panel.conf`, the image's build
  script and `faraday/image/README.md`: the new `panel.conf` keys (the
  2.8-inch panel states `NATIVE=portrait` and offers no rotation).
- `core/faraday-shell-api/src/lib.rs`: `Orientation`,
  `Event::RotationOffered`, `Command::Orientation`.
- `faraday/faraday-core/src/stick/settings.rs`: the field, the
  `rotation=` line.
- `faraday/faraday-core/src/screens/settings.rs`: the Display section and
  its rows.
- `faraday/faraday-strings/src/en.rs`: §6's strings.
- `core/faraday-draw/src/tokens.rs`: `PREVIEW_SIZES`.
- `faraday/faraday.just`: §10's sizes.
- Tests as §9.

## 13. Docs that follow

- `docs/LAYOUT.md` §2: the 10-inch portrait row (§10); §7: the density
  rule of §2.1.
- `docs/PREVIEW-SIZE.md` §1: the portrait rows and densities.
- `docs/DESIGN.md` §5 Settings: the Display section.
- `docs/GUIDE.md` §2.5 (§11).
- `faraday/image/README.md`: the `panel.conf` keys.
- This document becomes a Reference and its plan moves to
  ORIENTATION-LOG.md in `docs/history/` with the last batch.

## 14. Questions for the owner

Asked 2026-10-26; answered 2026-10-27, each as recommended (F100).

1. The density from the EDID on screens of 11 inches or less, and
   `faraday.dpi` kept elsewhere (§2.1, amending F101)? Yes: at 160 dpi a
   7-inch panel in portrait is 600 dp, medium, not the compact F100
   counts.
2. Which panels first? The stick: a 7-inch 1024 × 600 panel, landscape
   natively, and a 10-inch 800 × 1280 panel, portrait natively. The Pi's
   7-inch and 10-inch panels later, as their own work.
3. Portrait on a panel that is landscape natively? Yes, on panels of 6
   to 11 inches; never on a laptop's screen or a monitor.
4. The half turn and the opposite quarter turn offered? Not now; the
   map draws and is tested at all four.
5. The boot logo and the kernel console turned too? No: they stay as the
   firmware draws them.
6. Faraday Demo's Size menu gains the two portrait rows, and its 7-inch
   and 10-inch rows their real densities? Yes.
