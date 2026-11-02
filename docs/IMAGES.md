# Images for each piece of hardware

**Kind:** Spec
**Status:** Specified (F145, F185), the PC's one build, the Pi's one build, the HDMI screen's image and the release's files and image list built
**Open:** the four new Pi screens with their own panels (5-inch, 7-inch, 10-inch DSI), a full release's size and staging time measured, the Starter finding images, the website's choice

A person picks their hardware, and gets the one signed image built for
it. Each Pi screen and the PC stick is its own image and its own
release file. Nothing is edited after the build. The Starter carries no
images: the person downloads the Starter and the images they chose, and
the Starter offers what it finds. The owner's choices are F145, which
replaces this plan's earlier form in `docs/STARTER.md` §9.3 (one image,
edited for its screen by the Starter) and the Starter's payload of
images (§1.3).

## 1. One build on each platform

Each platform has one build (F185); there is no strict or loose one to
choose. What each carries beyond the common kernel, against
`docs/SECURITY-MODEL.md`:

| | The kernel |
|---|---|
| The Pi | DRM for HDMI and the DSI panels; USB keyboards, mice and touchscreens; USB storage (§2; `faraday/image/boards/pi3/`) |
| The PC stick | `efi=disable_early_pci_dma` on its built-in command line, so no PCI device the firmware left running can write to memory before the kernel has set up the IOMMU (`faraday/image/boards/x86_64-uefi/cmdline.txt`); `kernel.cmdline` makes the kernel check require it |

The PC's kernel is the EFI program itself, with its root filesystem
inside it and the firmware's framebuffer, so nothing it runs needs the
devices that flag cuts off. `KCMP` is forbidden on the PC (its own
`kernel.forbidden`) and allowed on the Pi, where DRM selects it (F153).

A board's lists and fragments are the board's own, in its directory
under `faraday/image/boards/`; there is no layer between the board and
the panel. The root filesystem, the app and its checks are the same on
both platforms.

`just faraday-pi-image` builds the 2.8-inch Pi image into
`out/pi/faraday-pi3-waveshare-28dpi.img` and `just panel=hdmi
faraday-pi-image` the HDMI screen's, `out/pi/faraday-pi3-hdmi.img`;
`just faraday-stick-image` builds the stick's,
`out/stick/faraday-x86_64-uefi.img`.

## 2. The Pi's kernel

What the Pi's kernel adds to the common kernel's reach, against `docs/SECURITY-MODEL.md`:

- **DRM, for the display only.** vc4's display side, the DSI host and
  the panels' drivers, with their I²C touch controllers. The Pi
  builds the board's kernel source as it is: Faraday carries no kernel
  patch of its own (owner, F153). vc4 also binds the GPU's 3D block,
  V3D, and creates a render node, `/dev/dri/renderD128`; Kconfig has no
  switch for either. **The render node is root's alone, no patch:**
  `faraday/image/common/rootfs-overlay/etc/mdev.conf` gives it and
  `/dev/dri/card0` to root at 0600, and nothing that runs after start
  opens them: the app runs as its own user, and the only root processes
  after boot, init and its two shell loops, open no DRM node
  (`docs/SECURITY-MODEL.md`, "When root is used"). The shell keeps writing
  `/dev/fb0` through DRM's framebuffer emulation. HDMI-CEC is not
  built. `BCM_VCIO` stays forbidden.
- **`kcmp()`, on the Pi only.** DRM selects `KCMP` for Mesa's sake,
  and the Pi allows it (F153): it moved from
  `faraday/image/common/kernel.forbidden` to the PC's own list, which
  keeps forbidding it. It only says whether two
  processes share a kernel object, for processes that pass ptrace's read
  check against the caller, and each of the device's processes runs as
  a user of its own.
- **The sound core, and no sound driver.** vc4 does not build without
  the kernel's sound core and its SoC layer (`depends on SND && SND_SOC`
  in its Kconfig), since its HDMI encoder can carry audio. The core is
  built, no sound driver is (`SND_USB`, `SND_BCM2835` and the generic
  drivers are forbidden), and HDMI audio is off in the device tree
  (`vc4-kms-v3d,noaudio`), so no sound card is registered.
- **HDMI.** A monitor's EDID is read by the kernel. A monitor is a
  small new input; the owner accepts it (F145).
- **USB HID.** The generic HID and multitouch drivers, the USB keyboard
  and mouse. No vendor-specific HID drivers, as on the PC. The stick's
  rules for new devices come across unchanged (SECURITY-MODEL, "USB
  devices"):
  - interfaces are authorised one at a time;
  - a new keyboard counts once it types the code on the screen;
  - a new pointer counts once trusted input approves it;
  - a new touchscreen counts once it taps the target on the screen.
- **A new USB touchscreen approves itself; nothing is trusted by name**
  (owner, F153, replacing F145's touchscreen named by the profile). An
  HDMI panel's touch reaches the Pi over USB, and on such a panel it is
  the only input there is. It is held back as every plugged-in device
  is, and the app draws a target at a place drawn from the system's
  randomness; a tap on it lets the touchscreen in, three taps beside it
  keep it out until it is unplugged and count toward the limit of
  devices kept out, as a keyboard's wrong codes do
  (`faraday/faraday-core/src/inputs.rs`). A touch controller on a bus
  that is part of the machine (the 2.8-inch's Goodix on I²C, a laptop's
  built-in touchscreen) stays trusted as before; `--touch-name` picks
  it out on that bus only (`faraday/shells/stick/src/evdev.rs`). The
  HDMI screen's profile gives no `TOUCH_NAME`, only the grid its touch
  reports in when its descriptor gives none.
- **USB storage and SCSI.** These sit behind `faraday-grant` and
  `faraday-disk`, as on the PC: one partition at a time, never the boot
  partition, read only while no secret is open.

## 3. Screens

Each screen is a panel directory under `faraday/image/panels/`, as the
2.8-inch's is. Its `panel.conf` gives the size, density, touch, `NATIVE`
and `MOUNT`, and its firmware lines go to `config.txt`. Sizes in dp are
at the panel's own density; classes are `docs/LAYOUT.md` §2.

| Screen | Panel | Pixels | dpi | Layout | Image |
|---|---|---|---|---|---|
| 2.8-inch | Waveshare 2.8inch DPI LCD, Goodix (today's) | 480 × 640, portrait | 286 | 268 × 358 dp: compact and short | Built |
| 5-inch | Raspberry Pi Touch Display 2, 5-inch, Goodix GT911 (to confirm) | 720 × 1280, portrait | 294 | 392 × 697 dp: compact; landscape 697 × 392: medium and short | Planned |
| 7-inch | Waveshare 7inch DSI LCD (C), Goodix | 1024 × 600, landscape | 170 | 964 × 565 dp: expanded and low; portrait: compact | Planned |
| 10-inch, DSI | Waveshare 10.1inch DSI LCD (C), Goodix | 1280 × 800, landscape | 149 | 1372 × 857 dp: large; portrait 857 × 1372: expanded | Planned |
| An HDMI screen | any HDMI monitor with a USB keyboard and mouse, or Waveshare 10.1inch HDMI LCD (B) with its USB touch | the screen's mode, from its EDID | the screen's, from its EDID, on a screen of 11 inches or less; `faraday.dpi=160` otherwise | by its width | Built (`faraday/image/panels/hdmi`) |

- **The HDMI screen** is driven by vc4's kernel mode setting
  (`dtoverlay=vc4-kms-v3d,noaudio,cma-64`, the firmware's overlay README,
  "vc4-kms-v3d"), which reads the screen's EDID over the HDMI bus and
  sets its preferred mode; no mode is named on the card, so one image
  serves every HDMI screen (F150). The shell reads the EDID from
  `/sys/class/drm` for the density and whether the screen is small, as
  the stick reads the firmware's (`docs/ORIENTATION.md` §2). `NATIVE`
  and `MOUNT` are not given: the screen's mode says which way it is.
- **No image for the original 7-inch screen.** The original Raspberry Pi
  Touch Display (800 × 480) may still run on the firmware's display path
  without DRM, but nothing current confirms it, so it is not offered
  (owner, F145).
- **Every Pi image starts every Pi Faraday supports** (F147). It
  carries the device trees of the 3B, 3B+ and Zero 2 W, and the firmware
  loads the one for the board it starts on. The Pi 4 joins them on the
  same images, planned (§9): its kernel, firmware and device tree beside
  the Pi 3's, the system the same. DSI screens fit the 3B, 3B+ and Pi 4:
  the Zero 2 W has no DSI connector.
- **An image may be published before it is tried on its panel**
  (F147), marked on the download page as looking for testers. Once it
  has been tried there, its `panel.conf` becomes fact, not data from its
  documentation, its tour is rendered at its size and in both
  orientations (`docs/ORIENTATION.md` §6), and the page marks it tested.
  The documents cite each panel's documentation and never say whose
  hardware it was tried on.

## 4. Release files and checking them

- **One image per platform and screen:**
  - `faraday-pi-X.Y.Z-<screen>.img.xz`, for every supported Pi (§3), so
    the name carries no board: today
    `faraday-pi-X.Y.Z-waveshare-28dpi.img.xz` and
    `faraday-pi-X.Y.Z-hdmi.img.xz`;
  - `faraday-stick-X.Y.Z-x86_64-uefi.img.xz`;
  - the PC's boot program, `faraday-X.Y.Z-BOOTX64.EFI`.

  `<screen>` is the panel directory's name. Nothing reads a release file
  by the names of an earlier release: the website takes the names from
  `faraday/builds.toml`, and the Starter tells a stick image from a Pi
  image by `faraday-stick-` and `faraday-pi-` alone. An image may be
  released before it is tried on its hardware (F147); the download page
  marks it.
- **`SHA256SUMS`** lists each `.xz`, each boot program, `IMAGES`, the
  `.img` inside each `.xz` under its own name, and each image's boot
  partition, its first partition, under the image's name with
  `-boot.vfat` for `.img` (`faraday-pi-X.Y.Z-hdmi-boot.vfat`).
  Nothing writes to a boot partition, so a card or stick already in use
  can still be checked against the release's signature. The README's
  "Verifying a release" shows both checks: the downloaded file, and a
  card or stick.
- **The release's image list** (`IMAGES`, a plain-text file in the
  release; the Starter carries it from §9 step 6) is staged with the
  manifest (`docs/STARTER.md` §1.3) and covered by `SHA256SUMS`. Its
  first line is `# Faraday X.Y.Z images, commit <full hash>`; then one
  line per image, in `faraday/builds.toml`'s order, its fields separated
  by one tab:
  1. the release file, `….img.xz`;
  2. the `.xz`'s SHA-256;
  3. the SHA-256 of the `.img` inside it;
  4. the platform, the entry's `board` (`x86_64-uefi`, `pi3`);
  5. the screen, its `panel` (`efi-framebuffer`, `waveshare-28dpi`, `hdmi`);
  6. the boards it starts, its `starts`, joined by `, ` (`Pi 3B, Pi 3B+,
     Pi Zero 2 W`).

  The HDMI screen's line, its tabs shown as two spaces and its sums
  shortened:

  ```
  faraday-pi-X.Y.Z-hdmi.img.xz  9f2c…  41ab…  pi3  hdmi  Pi 3B, Pi 3B+, Pi Zero 2 W
  ```
- **Each image has its entry in `faraday/builds.toml`**, the
  repository's list of builds: its recipe, the files it leaves, its
  release file names and the boards it starts.
  `faraday/tools/stage-release.sh` publishes every built image that
  names its release files, and `tools/lint-builds.py` holds that each
  names its `.img.xz` first and, for the PC, its boot program. The
  images this plan adds are listed there as planned, and an entry
  becomes built with its recipe.
- **Each image is reproducible** from the release's commit, as
  `docs/DEVELOPING.md` §5 says. A recipe builds one image by its
  platform and screen, and the release recipe builds them all.

## 5. The Starter

- **It carries no images.** It holds its own code, Faraday Demo (F126),
  and the release's image list. That is about 17 MB in place of about
  43 MB.
- **It finds images.** It looks in the Downloads folder Transfer uses
  (`--downloads`, as F133's signed boot file is found), and in files
  dropped on its window. Each file whose name is on the image list is
  hashed and checked against its line; a file that does not match is
  named and not offered. A file of another release is named with the
  version this Starter needs.
- **Make a Faraday stick and Make a Pi card offer what was found**, by
  its hardware: "Pi 3, Waveshare 7-inch DSI". Hardware without its image is
  listed with the exact file to download and where.
- **What it writes** to a Pi card is the image, decompressed while
  written and read back as `docs/STARTER.md` §4.1 says. To a stick it
  writes the image's boot partition and a table and data partition made
  for the stick's size (`docs/STICK-IDENTITY.md` §4.2), so the data
  partition has the rest of the stick; the image as published is
  unchanged, with its 128 MiB data partition, and is what `dd` writes.
- **Keep data and a boot file the person signed (F133)** take the
  stick image's boot partition and `BOOTX64.EFI`. The signed file is
  checked against the release's.
- **No network.** The Starter downloads nothing (`docs/STARTER.md`
  §10.1).
- **A developer's own image** (`--image`) is written as today, and
  Confirm says it is not a release image.

## 6. Without the Starter

The image for the hardware is downloaded and checked against
`SHA256SUMS` and its signature, then written with Raspberry Pi Imager,
balenaEtcher or `dd`. The README says so in a few lines per system.

## 7. The website

The download page asks one thing, the hardware, and leads to one image
per visit (F147):

1. **Tiles**, one per `[[hardware]]` entry of `faraday/builds.toml`, in
   their order: the PC, then each Pi screen by size, and a Pi with an
   HDMI screen; three columns from 840 px wide, fewer below. Each shows
   the hardware with Faraday's screen at that size, its name, the Pis
   it fits, and one button, Download or Coming soon; whether an image is
   tested is said on its page, not here (owner, 2026-10-30). The Pi
   model is not asked (§3).
2. **An image's page** offers the Starter for the visitor's system and
   the image, each as its own release file (no zip, so every
   file is one line of `SHA256SUMS`), and writing the image without the
   Starter. What the visitor needs, opening the Starter the first time
   and checking the files are on this page, not on the tiles' page. The
   image carries its state as `docs/HARDWARE.md` §1 gives it: tested,
   looking for testers, or coming soon; one looking for testers asks
   those who try it to say so, and so does a Pi the hardware fits that
   no one has started the image on yet.

The hardware page says what each image carries and links
Faraday's posts on hardware risk. Boards and screens under study are
marked so (`docs/HARDWARE.md` §6).

## 8. What else changes

| Document | Change |
|---|---|
| `docs/HARDWARE.md` | §1 and §2: a row per image and screen |
| `docs/STARTER.md` | §1.1 release files; §1.3 no payload of images, the image list; §3.2 and §3.3 what is offered; §9 points here |
| `docs/SECURITY-MODEL.md` | the Pi's table and "What the kernel can reach"; the PC's command line |
| `docs/ORIENTATION.md` | §1: the Pi's panel directories; §5 Rotation on them |
| `docs/DEVELOPING.md` | the recipes for one image and for all |
| `faraday/image/README.md` | the boards and panels |
| `README.md` | "Verifying a release": a card or stick checked by its boot partition |

## 9. Order of work

1. One Pi image for every supported Pi: the 3B, 3B+ and Zero 2 W
   device trees on one card (built: the Zero 2 W's board directory is
   folded into the Pi 3's), tried on the 3B and the Zero 2 W.
2. The Pi 4 on the same images: its 32-bit kernel, firmware and device
   tree on every Pi card, the system the same. First, for the owner:
   what it adds to the kernel's reach (USB behind PCIe, a boot loader in
   the board's EEPROM, against `docs/SECURITY-MODEL.md`), and whether
   the 2.8-inch image boots on the board with the Pi's kernel. Approved by the owner (F150) on these
   terms: no boot loader update is ever on a card; SPI_SPIDEV and
   SPI_BCM2835 are forbidden in the kernel; the Pi 4 is described as close
   to a PC, its boot code on the board, and its owner is told to restore
   Raspberry Pi's boot loader with Raspberry Pi Imager before first use
   (a wire to test point TP5 locks it in hardware).
3. The Pi's kernel: its fragment, its forbidden and required lists,
   and the image that boots with it, starting with the monitor (HDMI
   with a USB keyboard), which needs no new panel (built:
   `faraday/image/boards/pi3/`, `faraday/image/panels/hdmi`; `just
   panel=hdmi faraday-pi-image`, and the 2.8-inch with `just
   faraday-pi-image`; the 2.8-inch is tried on its panel, the HDMI image
   not yet on a Pi: no HDMI output in QEMU's raspi3b). Left: trying the
   HDMI image on a Pi 3B+ with a monitor, and the 10-inch HDMI panel's
   touch name read off the panel (step 7).
4. The PC's one build: the command line with
   `efi=disable_early_pci_dma`, one image and one boot program (built:
   `just faraday-stick-image`).
5. The release recipes, `SHA256SUMS` lines and the image list (built:
   `just faraday-release-stage` stages every released image of
   `faraday/builds.toml`, its boot partition's line and `IMAGES`, and
   refuses images of two builds; the Starter's payload is the stick
   and the 2.8-inch Pi image until step 6). Left: the
   size and staging time of a full set, measured on the next release's
   builds.
6. The Starter: no payload of images, finding and checking them, the
   offers.
7. Each panel's directory, from its documentation, and tried on the
   panel to be marked tested: the 7-inch, the 10-inch
   HDMI (with the USB touchscreen rule, §2), the 10-inch DSI, the
   5-inch.
8. The website's download page (§7), built around the images released
   today, with a tile for each image as it is built.
