# The kernel patches the images carry

**Kind:** Reference
**Status:** Built

Every change the images make to their kernel's source, on every board. The
patches are the files in each board's `linux-patches/`, applied in order by
the build. There are eight: three on the Pi, all Faraday's own, and
five on the PC, all from mainline. Nothing else changes the kernel's source;
what is built into it is the configuration, `docs/SECURITY-MODEL.md`, "What
the kernel can reach".

A patch of Faraday's own is code nobody else runs that way, carried by hand
through every kernel update. F153 set the rule against them, F205 made one
exception, F221 two more (one of which went with 6.18) and F224 one, and each says any other needs a decision of its
own.

| Board | Patch | What it does | From | Decision |
|---|---|---|---|---|
| Pi | 0002 | A failed read of a later EDID block keeps the blocks before it | Faraday's own | F205 |
| Pi | 0003 | vc4's framebuffer is 32 bits with red first | Faraday's own | F221 |
| Pi | 0004 | The bootloader's seed is not credited unless the command line says to | Faraday's own | F224 |
| PC | 0001 to 0005 | The EFI stub puts the screen's EDID in `boot_params` | Mainline, Linux 6.19 | F23 |

## The Pi

The Raspberry Pi 6.18 tree (6.18.59), in `faraday/image/boards/pi3/linux-patches/`.
`docs/IMAGES.md` §3.2 says where each one matters at boot.

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

- **What it does.** Three changes. vc4 asks for an XBGR8888 framebuffer
  emulation, red first in memory, where it asked for RGB565
  (`drm_client_setup_with_fourcc`). The generic DMA emulation picks XBGR8888
  where the legacy table gives XRGB8888: the emulation keeps only the bits a
  pixel of the format it is given, so the format itself has to be chosen
  there. And `drm_fb_helper_fill_pixel_fmt` reports red at bit 0 and blue at
  bit 16 for that format, so the console and the boot logo keep their colours.
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
  changes are in code shared by every driver using the generic DMA emulation;
  vc4 is the only one this board builds. The emulation's pick changes XRGB8888
  to XBGR8888 for any driver that asks for it, with no effect today. This is the largest of the
  patches, in code that changes between kernel versions, so it is the likeliest
  to need rewriting at an update.
- **When it can go.** When a kernel the Pi moves to lets vc4 ask for this
  format itself, or if the shell goes back to converting each frame and takes
  that cost.

### 0004: the bootloader's seed is not credited unless the command line says to

- **What it does.** In `drivers/char/random.c`, `trust_bootloader` starts
  false where it started true. The seed is still mixed into the pool; it is
  credited only by `random.trust_bootloader=on`. One line.
- **Why.** The Pi's firmware writes `/chosen/rng-seed`, and on 32-bit ARM
  `setup_machine_fdt` credits it before `parse_early_param` has read the command
  line, so `random.trust_bootloader=off` comes too late and the pool counts as
  ready at the first line of the log. The firmware has no setting that leaves the
  seed out (`docs/OWN-RANDOMNESS.md` §3.2). With the seed uncounted, F102's
  rule that only the kernel's jitter makes the pool ready can hold.
- **From.** Faraday's own.
- **Trade-off.** It credits less, never more: no new reach, no new code path,
  no kept copy of anything. The pool is ready later, on bcm2835-rng until
  that is uncounted too, then on jitter. It is carried through every kernel update.
- **When it can go.** When the Pi's kernel reads the command line before the
  device tree's seed, or the firmware can leave the seed out.

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
