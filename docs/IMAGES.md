# Images for each piece of hardware

**Kind:** Spec
**Status:** Specified (F145), the PC's two builds, the Pi's two builds and the HDMI screen's image built
**Open:** the four new Pi screens with their own panels (5-inch, 7-inch, 10-inch DSI) and the 10-inch HDMI panel's touch name confirmed, the release's image list, the Starter finding images, the website's choice

A person picks their hardware, and gets the one signed image built for
it. Each Pi screen and each PC build is its own image and its own
release file. Nothing is edited after the build. The Starter carries no
images: the person downloads the Starter and the images they chose, and
the Starter offers what it finds. The owner's choices are F145, which
replaces this plan's earlier form in `docs/STARTER.md` §9.3 (one image,
edited for its screen by the Starter) and the Starter's payload of
images (§1.3).

## 1. Two builds on each platform

| | Strict | Compatible |
|---|---|---|
| The Pi | today's kernel: no DRM, no HID, no USB storage (`faraday/image/boards/pi3/builds/strict/kernel.forbidden`); the DPI panel's touch is the only input | DRM for HDMI and the DSI panels; USB keyboards, mice and touchscreens; USB storage (`faraday/image/boards/pi3/builds/compatible/`) |
| The PC stick | today's kernel with `efi=disable_early_pci_dma` on its built-in command line (`faraday/image/boards/x86_64-uefi/builds/strict/kernel.cmdline`) | today's kernel as it is |

Strict is the most locked-down build a piece of hardware allows. Compatible
works with more hardware. Some older PCs do not start with
`efi=disable_early_pci_dma`. They take Compatible.

Each build has its own `kernel.forbidden` and `kernel.required`, in a
directory under the board's `builds/` that `build.sh` adds after the
panel. The Strict Pi list keeps DRM, sound, HID and USB storage
forbidden, so a Strict build proves it carries none of what Compatible
adds; Strict has no kernel fragment of its own, so its kernel is the one
the Pi had before it had two builds. The root filesystem, the app and
its checks are the same in both builds of a platform. The kernel and its
command line differ, and on the Pi the screen's lines.

`just faraday-pi-image` builds the 2.8-inch Strict, as it always has,
into `out/pi/faraday-pi3-waveshare-28dpi.img`; `just build=compatible
faraday-pi-image` the 2.8-inch Compatible, `…-compatible.img`, and
`just panel=hdmi build=compatible faraday-pi-image` the HDMI screen's,
`out/pi/faraday-pi3-hdmi-compatible.img`. A panel that needs the
Compatible kernel says so in its `panel.conf` (`BUILDS`), and the build
refuses it as Strict before anything is built.

## 2. The Pi's Compatible kernel

What Compatible adds to the kernel's reach, against `docs/SECURITY-MODEL.md`:

- **DRM, for the display only.** vc4's display side, the DSI host and
  the panels' drivers, with their I²C touch controllers. There is no
  V3D: no GPU render node. Kconfig has no switch for either, so the
  Compatible build carries two changes of Faraday's own to the kernel
  source, `faraday/image/boards/pi3/builds/compatible/linux-patches/`:
  vc4 registers no V3D component and creates no render node, and DRM no
  longer selects `KCMP`, the `kcmp()` system call Mesa wants and every
  Faraday image forbids. Strict builds the board's source as it is. The shell keeps writing `/dev/fb0`
  through DRM's framebuffer emulation; `/dev/dri/card0` is root's, as
  every node `mdev.conf` gives no owner is, and nothing runs as root
  after start. HDMI-CEC is not built. `BCM_VCIO` stays forbidden in both
  builds.
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
  - a new pointer counts once trusted input approves it.
- **A USB touchscreen named by the screen's profile is trusted input**
  (owner, F145). An HDMI panel's touch reaches the Pi over USB, and on
  such a panel it is the only input there is. The panel's profile
  names its touch controller as the 2.8-inch's names the Goodix
  (`TOUCH_NAME` in `faraday/image/panels/waveshare-28dpi/panel.conf`).
  The first USB touchscreen matching that name is the panel's own and
  needs no approval. Any other touchscreen, pointer or keyboard follows
  the rules above. The shell does this already for any panel's
  `--touch-name` (`faraday/shells/stick/src/evdev.rs`). The HDMI
  screen's profile names `WaveShare`, Waveshare's USB vendor string;
  what the 10.1inch HDMI LCD (B)'s touch calls itself is to be read off
  the panel when it is tried.
- **USB storage and SCSI.** These sit behind `faraday-grant` and
  `faraday-disk`, as on the PC: one partition at a time, never the boot
  partition, read only while no secret is open.

## 3. Screens

Each screen is a panel directory under `faraday/image/panels/`, as the
2.8-inch's is. Its `panel.conf` gives the size, density, touch, `NATIVE`
and `MOUNT`, and its firmware lines go to `config.txt`. Sizes in dp are
at the panel's own density; classes are `docs/LAYOUT.md` §2.

| Screen | Panel | Pixels | dpi | Layout | Build |
|---|---|---|---|---|---|
| 2.8-inch | Waveshare 2.8inch DPI LCD, Goodix (today's) | 480 × 640, portrait | 286 | 268 × 358 dp: compact and short | Strict or Compatible (both built) |
| 5-inch | Raspberry Pi Touch Display 2, 5-inch, Goodix GT911 (to confirm) | 720 × 1280, portrait | 294 | 392 × 697 dp: compact; landscape 697 × 392: medium and short | Compatible |
| 7-inch | Waveshare 7inch DSI LCD (C), Goodix | 1024 × 600, landscape | 170 | 964 × 565 dp: expanded and low; portrait: compact | Compatible |
| 10-inch, DSI | Waveshare 10.1inch DSI LCD (C), Goodix | 1280 × 800, landscape | 149 | 1372 × 857 dp: large; portrait 857 × 1372: expanded | Compatible |
| An HDMI screen | any HDMI monitor with a USB keyboard and mouse, or Waveshare 10.1inch HDMI LCD (B) with its USB touch | the screen's mode, from its EDID | the screen's, from its EDID, on a screen of 11 inches or less; `faraday.dpi=160` otherwise | by its width | Compatible (built: `faraday/image/panels/hdmi`) |

- **The HDMI screen** is driven by vc4's kernel mode setting
  (`dtoverlay=vc4-kms-v3d,noaudio,cma-64`, the firmware's overlay README,
  "vc4-kms-v3d"), which reads the screen's EDID over the HDMI bus and
  sets its preferred mode; no mode is named on the card, so one image
  serves every HDMI screen (F150). The shell reads the EDID from
  `/sys/class/drm` for the density and whether the screen is small, as
  the stick reads the firmware's (`docs/ORIENTATION.md` §2). `NATIVE`
  and `MOUNT` are not given: the screen's mode says which way it is.
- **No 7-inch Strict image.** The original Raspberry Pi Touch Display
  (800 × 480) may still run on the firmware's display path without DRM,
  but nothing current confirms it, so it is not offered (owner, F145).
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

- **One image per platform, screen and build:**
  - `faraday-pi-X.Y.Z-<screen>-strict.img.xz` and `…-compatible.img.xz`, each
    for every supported Pi (§3);
  - `faraday-stick-X.Y.Z-x86_64-uefi-strict.img.xz` and `…-compatible.img.xz`;
  - each PC build's boot program, `faraday-X.Y.Z-BOOTX64-strict.EFI`
    and `…-compatible.EFI`.

  `<screen>` is the panel directory's name.
- **`SHA256SUMS`** lists each `.xz`, the `.img` inside it, and each
  image's boot partition (`-boot.vfat`). Nothing writes to a boot
  partition, so a card or stick already in use can still be checked
  against the release's signature. The README's "Verifying a release"
  shows both checks: the downloaded file, and a card or stick.
- **The release's image list** (`IMAGES`, a plain-text file in the
  release and built into the Starter) has one line per image: file
  name, the two SHA-256 sums, platform, screen, build, and the boards
  it starts. It is staged with the manifest (`docs/STARTER.md` §1.3)
  and covered by `SHA256SUMS`.
- **Each image has its entry in `faraday/builds.toml`**, the
  repository's list of builds: its recipe, the files it leaves and
  its release file names. The images this plan adds are listed there
  as planned, and an entry becomes built with its recipe.
- **Each image is reproducible** from the release's commit, as
  `docs/DEVELOPING.md` §5 says. A recipe builds one image by its
  platform, screen and build, and the release recipe builds them all.

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
  its hardware: "Pi 3, Waveshare 7-inch DSI, Compatible". For the PC stick,
  Strict is offered first, saying that some older PCs do not start with
  it, and that Compatible is what to write then. Hardware without its image is
  listed with the exact file to download and where.
- **What it writes** is the image unchanged, decompressed while written
  and read back as `docs/STARTER.md` §4.1 says.
- **Keep data and a boot file the person signed (F133)** take the
  chosen build's boot partition and `BOOTX64.EFI`. The signed file is
  checked against that build's.
- **No network.** The Starter downloads nothing (`docs/STARTER.md`
  §10.1).
- **A developer's own image** (`--image`) is written as today, and
  Confirm says it is not a release image.

## 6. Without the Starter

The image for the hardware is downloaded and checked against
`SHA256SUMS` and its signature, then written with Raspberry Pi Imager,
balenaEtcher or `dd`. The README says so in a few lines per system.

## 7. The website

The download page asks one thing, the hardware, and leads to one build
per visit (F147):

1. **Tiles**, one per piece of hardware, from `faraday/builds.toml`'s
   `[[hardware]]` entries, in their order, each image naming the
   hardware and build it is offered as: the PC, then each Pi screen by
   size, and an HDMI monitor. Each shows the hardware with Faraday's
   screen at that size, its name, the Pis it fits, and whether it is
   tested, looking for testers, or coming soon. The Pi model is not
   asked (§3).
2. **A build's page** offers the Starter for the visitor's system and
   the build's image, each as its own release file (no zip, so every
   file is one line of `SHA256SUMS`), and writing the image without the
   Starter. Strict is offered where the hardware has it, and the page
   offers the other build: Compatible for a PC that does not start with Strict,
   or for a USB keyboard or stick on the 2.8-inch. What the visitor needs,
   opening the Starter the first time and checking the files are on this
   page, not on the tiles' page. A build looking for testers asks those
   who try it to say so.

The hardware page says what each build carries and that every one is
safe in practice, Strict being the most the hardware allows, and links
Faraday's posts on hardware risk. Boards and screens under study are
marked so (`docs/HARDWARE.md` §6).

## 8. What else changes

| Document | Change |
|---|---|
| `docs/HARDWARE.md` | §1 and §2: a row per image and screen |
| `docs/STARTER.md` | §1.1 release files; §1.3 no payload of images, the image list; §3.2 and §3.3 what is offered; §9 points here |
| `docs/SECURITY-MODEL.md` | the Pi's table and "What the kernel can reach" for Strict and Compatible; the PC's command line |
| `docs/ORIENTATION.md` | §1: the Pi's panel directories; §5 Rotation on them |
| `docs/DEVELOPING.md` | the recipes for one image and for all |
| `faraday/image/README.md` | the boards and panels, Strict and Compatible |
| `README.md` | "Verifying a release": a card or stick checked by its boot partition |

## 9. Order of work

1. One Pi image for every supported Pi: the 3B, 3B+ and Zero 2 W
   device trees on one card (built: the Zero 2 W's board directory is
   folded into the Pi 3's), tried on the 3B and the Zero 2 W.
2. The Pi 4 on the same images: its 32-bit kernel, firmware and device
   tree on every Pi card, the system the same. First, for the owner:
   what it adds to the kernel's reach (USB behind PCIe, a boot loader in
   the board's EEPROM, against `docs/SECURITY-MODEL.md`), and whether
   the 2.8-inch runs on its firmware display path without DRM, as Strict
   needs, tried on the board. Approved by the owner (F150) on these
   terms: no boot loader update is ever on a card; SPI_SPIDEV and
   SPI_BCM2835 are forbidden in the kernel; the Pi 4 is described as close
   to a PC, its boot code on the board, and its owner is told to restore
   Raspberry Pi's boot loader with Raspberry Pi Imager before first use
   (a wire to test point TP5 locks it in hardware).
3. The Pi's Compatible kernel: its fragment, its forbidden and required
   lists, and the image that boots with it, starting with the monitor
   (HDMI with a USB keyboard), which needs no new panel (built:
   `faraday/image/boards/pi3/builds/`, `faraday/image/panels/hdmi`;
   `just panel=hdmi build=compatible faraday-pi-image`, and the 2.8-inch
   Compatible with `just build=compatible faraday-pi-image`; not yet tried on
   a Pi: no HDMI output in QEMU's raspi3b). Left: trying both on a Pi 3B+
   with a monitor, and the 10-inch HDMI panel's touch name read off the
   panel (step 7).
4. The PC's Strict and Compatible: the command line in each, two
   images, two boot programs (built: `just build=compatible
   faraday-stick-image`; not yet tried on a PC).
5. The release recipes, `SHA256SUMS` lines and the image list; the
   size and build time measured.
6. The Starter: no payload of images, finding and checking them, the
   offers.
7. Each panel's directory, from its documentation, and tried on the
   panel to be marked tested: the 7-inch, the 10-inch
   HDMI (with the USB touchscreen rule, §2), the 10-inch DSI, the
   5-inch.
8. The website's download page (§7), built around the images released
   today, with a tile for each image as it is built.
