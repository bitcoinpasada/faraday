# The kernel patches the images carry

**Kind:** Reference
**Status:** Built

Every change the images make to their kernel's source, on every board. The
patches are the files in each board's `linux-patches/`, applied in order by
the build. There are nine: four on the Pi, three of them Faraday's own, and
five on the PC, all from mainline. Nothing else changes the kernel's source;
what is built into it is the configuration, `docs/SECURITY-MODEL.md`, "What
the kernel can reach".

A patch of Faraday's own is code nobody else runs that way, carried by hand
through every kernel update. F153 set the rule against them, F205 made one
exception and F221 two more, and each says any other needs a decision of its
own.

| Board | Patch | What it does | From | Decision |
|---|---|---|---|---|
| Pi | 0001 | A failed EDID read gives fallback modes, not an error | Mainline | F205 names it |
| Pi | 0002 | A failed read of a later EDID block keeps the blocks before it | Faraday's own | F205 |
| Pi | 0003 | vc4's framebuffer is 32 bits with red first | Faraday's own | F221 |
| Pi | 0004 | vc4 stops output polling when it unbinds | Faraday's own | F221 |
| PC | 0001 to 0005 | The EFI stub puts the screen's EDID in `boot_params` | Mainline, Linux 6.19 | F23 |

## The Pi

The Raspberry Pi 6.1 tree, in `faraday/image/boards/pi3/linux-patches/`.
`docs/IMAGES.md` §3.2 says where each one matters at boot.

### 0001: a failed EDID read gives fallback modes

- **What it does.** One line in `vc4_hdmi.c`: when no EDID can be read,
  `.get_modes()` returns 0 where it returned `-ENODEV`, so the probe helper
  adds the kernel's fallback modes, up to 1024x768.
- **Why.** A monitor whose EDID cannot be read otherwise has no modes, so no
  `/dev/fb0` and no picture.
- **From.** Mainline commit abf493988e38, its one line applied to this tree.
- **Trade-off.** None for reach: no new interface, and no more of the
  monitor's data is read. An HDMI screen that gave no EDID now gets a picture
  at a low resolution where before it got none.
- **When it can go.** When the Pi moves to a kernel that has this commit.

### 0002: a failed read of a later EDID block keeps the blocks before it

- **What it does.** In `drm_edid.c`, when reading an extension block fails,
  that block and those after it are zeroed and counted invalid, where before
  the whole EDID was thrown away. The zeroing stays inside the blocks
  allocated. The checks on each block are unchanged, so nothing that failed
  them is passed on.
- **Why.** `i2c-bcm2835` cannot be relied on for the three messages a read of
  block 2 or later takes, so a monitor with three or more blocks gave no modes
  on most boots (F205).
- **From.** Faraday's own.
- **Trade-off.** It is shared DRM code, though vc4 is the only DRM driver this
  board builds. An EDID comes from the monitor, so it is outside data; the
  kernel now goes on to parse the blocks that passed their checks in a case
  where it used to stop. The parser is the one every other EDID reaches, and it
  sees no block it would have refused. A screen can show fewer modes than it
  has, since the lost blocks are not read again. It must be carried through
  every kernel update.
- **When it can go.** When a kernel the Pi moves to, or `i2c-bcm2835` itself,
  reads such an EDID reliably (F205).

### 0003: vc4's framebuffer is 32 bits with red first

- **What it does.** Four changes. vc4 asks for a 32-bit framebuffer emulation
  where it asked for 16. The generic emulation picks XBGR8888, red first in
  memory, where the legacy table gives XRGB8888. `drm_fb_helper_fill_pixel_fmt`
  reports red at bit 0 and blue at bit 16 for that format. And
  `drm_client_framebuffer_create` calls `drm_mode_addfb2`, which takes a
  format, where `drm_mode_addfb` worked XRGB8888 out again from bits and depth.
- **Why.** The app's frame is red first. At 16 bits the shell converted every
  changed pixel on the CPU; now it copies the frame as it is (`docs/FRAMES.md`
  §5), and the screen shows 8 bits a colour where it showed 5 or 6. Each pixel
  written is twice the size, so the write takes back most of the gain. Measured
  on a Pi 3B+, conversion and write together, in separate runs: 3.1 ms to 2.3 ms
  on the 2.8-inch panel, and about 21.9 ms to 18.7 ms at 1080p.
- **From.** Faraday's own.
- **Trade-off.** No new device node, command or permission: the app writes to
  `/dev/fb0` as before. The framebuffer takes twice the memory, about 1.2 MiB
  on the 2.8-inch panel and 7.9 MiB at 1080p, in place of half that. Two of the
  changes are in code shared by every driver using the generic emulation or a
  DRM client; vc4 is the only one this board builds. The `addfb2` change
  gives a format the legacy table gives the same either way; the generic
  emulation's pick changes XRGB8888 to XBGR8888 for any driver that asks
  for it, with no effect today. This is the largest of the
  patches, in code that changes between kernel versions, so it is the likeliest
  to need rewriting at an update.
- **When it can go.** When a kernel the Pi moves to lets vc4 ask for this
  format itself, or if the shell goes back to converting each frame and takes
  that cost.

### 0004: vc4 stops output polling when it unbinds

- **What it does.** One call in `vc4_drv.c`: `drm_kms_helper_poll_fini` when
  the driver unbinds.
- **Why.** vc4 starts the poll's delayed work and never stops it, so unbinding
  frees the device while the timer is pending, and the timer then fires from
  freed memory. The Pi's boot unbinds and binds vc4 again to bring up a DPI
  panel, so a Pi on the 2.8-inch panicked about 13 s after power-on.
- **From.** Faraday's own.
- **Trade-off.** It removes a use of freed memory and adds no reach. It only
  runs on an unbind, which HDMI's route does not do. The cost is one more
  patch to carry through an update.
- **When it can go.** When a kernel the Pi moves to stops the poll itself, or
  when the boot no longer unbinds vc4.

## The PC

Mainline 6.18 longterm, in `faraday/image/boards/x86_64-uefi/linux-patches/`.
All five are commits from Linux 6.19 with their diffs unchanged, and they are
one piece of work: from them the EFI stub puts the screen's EDID in
`boot_params`, and the stick reads its base block from
`/sys/kernel/boot_params/data` to name a screen (F23, `docs/STARTER.md` §3.1a).
DRM stays forbidden on this board.

| Patch | Mainline commit | What it does |
|---|---|---|
| 0001 | 9d805709d897 | Finds the display's handle where it found its protocol; no change in what is shown |
| 0002 | ae42b9c5ddda | Moves filling in `screen_info` to a helper; no change in what is shown |
| 0003 | 17029cdd8f9d | Reads the EDID from the firmware's protocols, active first, then discovered |
| 0004 | e41ef37ddfe7 | Stores it in `boot_params` |
| 0005 | 5688e977ed2b | Fixes reading it when a 64-bit kernel starts from 32-bit firmware |

- **Why.** Nothing else reads the screen's EDID on this board, since DRM is
  forbidden, and the stick needs it to name a screen.
- **Trade-off.** More of the firmware's data enters the kernel: the stub copies
  an EDID into `boot_params`, at most the size of the field and only if it is at
  least 128 bytes. The firmware is outside the security scope already. The copy
  is then readable at `/sys/kernel/boot_params/data`, and the stick parses it,
  so what a screen says about itself reaches `faraday-core`'s `edid` module,
  which treats it as a name to show. The code runs in the stub, before the
  kernel starts, and adds no interface after boot beyond that data.
- **When they can go.** All five, when the PC moves to Linux 6.19 or later.
