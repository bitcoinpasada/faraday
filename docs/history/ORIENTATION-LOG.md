# ORIENTATION build log: the display path before, the batches, the docs that followed and the owner's answers, as written at the time

**Kind:** History
**Status:** Kept as written; section numbers are the spec's when it was written. The current reference is `docs/ORIENTATION.md`.

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
