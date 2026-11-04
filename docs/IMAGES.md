# Images for each piece of hardware

**Kind:** Spec
**Status:** Specified (F145, F185, F186), the PC's one build, the Pi's one build, the one Pi image for an HDMI screen and the 2.8-inch (its screen directories, overlays, boot script and kernel options) and the release's files and image list built
**Open:** the one Pi image built and booted under QEMU (§9 step 10), each screen tried on it (§9 step 11), a full release's size and staging time measured, the Starter's finder tried on a computer with a real release's images (§9 step 6), the website's choice

A person picks their hardware, and gets the one signed image built for
it: one for the PC stick, and one for the Pi that finds its screen at
boot (§3, F186, F187).
Each is its own release file. Nothing is edited after the build. The Starter carries no
images: the person downloads the Starter and the images they chose, and
the Starter offers what it finds. The owner's choices are F145, which
replaces this plan's earlier form in `docs/STARTER.md` §9.3 (one image,
edited for its screen by the Starter) and the Starter's earlier payload of
images (§1.3, removed).

## 1. One build on each platform

Each platform has one build (F185); there is no strict or loose one to
choose. What each carries beyond the common kernel, against
`docs/SECURITY-MODEL.md`:

| | The kernel |
|---|---|
| The Pi | DRM for HDMI and the 2.8-inch DPI panel; USB keyboards, mice and touchscreens; USB storage (§2; `faraday/image/boards/pi3/`) |
| The PC stick | `efi=disable_early_pci_dma` on its built-in command line, so no PCI device the firmware left running can write to memory before the kernel has set up the IOMMU (`faraday/image/boards/x86_64-uefi/cmdline.txt`); `kernel.cmdline` makes the kernel check require it |

The PC's kernel is the EFI program itself, with its root filesystem
inside it and the firmware's framebuffer, so nothing it runs needs the
devices that flag cuts off. `KCMP` is forbidden on the PC (its own
`kernel.forbidden`) and allowed on the Pi, where DRM selects it (F153).

A board's lists and fragments are the board's own, in its directory
under `faraday/image/boards/`; there is no layer between the board and
the panel. The root filesystem, the app and its checks are the same on
both platforms.

`just faraday-pi-image` builds the one Pi image into
`out/pi/faraday-pi3-any.img`, with the boot script that finds the
screen (§3.4); `just faraday-stick-image` builds the stick's,
`out/stick/faraday-x86_64-uefi.img`.

## 2. The Pi's kernel

What the Pi's kernel adds to the common kernel's reach, against `docs/SECURITY-MODEL.md`:

- **DRM, for the display only.** vc4's display side, the DPI panel's
  driver, and the 2.8-inch's I²C touch controller. The Pi
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

One Pi image serves every screen Faraday lists and every Pi it starts
(F186): an HDMI screen and the 2.8-inch, the two screens Faraday lists
on the Pi for now (F187). The image does not name its screen: a boot script, run as root
before the app, finds the screen attached, loads that screen's
device-tree overlay while the system runs, and gives the app the
screen's values. A screen with no touch controller cannot be found, and
the 2.8-inch has one. No screen found is HDMI. Nothing is written
to the card: the boot partition stays the release's, byte for byte
(F182), and a card is still checked against `SHA256SUMS` (§4).

The one image (`faraday/image/panels/any`) replaces the 2.8-inch's
image, on the firmware's framebuffer, and the HDMI screen's, which
earlier releases carry. It is built with both screens and is to be
tried on each (§9 steps 10 and 11). A screen not
yet tried is built, not tried, in `docs/HARDWARE.md` and on the
website.

### 3.1 The screens

Sizes in dp are at the panel's own density; classes are
`docs/LAYOUT.md` §2. A value marked "to confirm" is the one the
panel's vendor documents, built in as it is and checked on the panel
(§9 step 11). A wrong one fails safe: the probe that expects it finds
no screen, that screen's setup is not applied, and the Pi starts on
HDMI.

| Screen | Panel | Pixels | dpi | Layout | Found by | Display overlay |
|---|---|---|---|---|---|---|
| An HDMI screen | any HDMI monitor with a USB keyboard and mouse, or Waveshare 10.1inch HDMI LCD (B) with its USB touch | the screen's mode, from its EDID | the EDID's on a screen of 11 inches or less; `faraday.dpi=160` otherwise | by its width | its connector reads `connected` | none beyond `vc4-kms-v3d` in `config.txt` |
| 2.8-inch | Waveshare 2.8inch DPI LCD, Goodix GT911 | 480 × 640, portrait | 286 | 268 × 358 dp: compact and short | its GT911 on the bit-banged I²C its touch overlay declares | the firmware's `vc4-kms-dpi-generic` with the panel's timings and pins |

- **What the probes expect** is fixed for each screen in its directory
  (§3.3): the controller's name, its bus and address. Only an exact
  match applies a screen.
- **The values each screen gives the app** are the ones `panel.conf`
  gives today, as the shell's arguments (§3.4): size (`--size`),
  density (`--dpi`), the touch controller's name and grid
  (`--touch-name`, `--touch-grid`) and the turn (`--native`, and
  `--mount` where a case turns the panel).
  - The 2.8-inch: 480 × 640, 286 dpi, `Goodix`, `portrait`; its grid
    and axes are worked out again on the kernel's DPI driver (§3.3).
  - An HDMI screen gives none: the shell reads its size from `/dev/fb0`
    and its density from the EDID (`docs/ORIENTATION.md` §2), and its
    USB touch is let in by tapping a target (§2, F153).
- **The HDMI screen** is driven by vc4's kernel mode setting
  (`dtoverlay=vc4-kms-v3d,noaudio,cma-64`, the firmware's overlay README,
  "vc4-kms-v3d"), which reads the screen's EDID over the HDMI bus and
  sets its preferred mode; no mode is named on the card (F150).
  `NATIVE` and `MOUNT` are not given: the screen's mode says which way
  it is.
- **Both screens fit every Pi the image starts.**
- **No DSI screen for now** (owner, F187). Raspberry Pi Touch Display 2
  and Waveshare's DSI (C) panels are not built: none would be tried, and
  a person with the money for one has a cheap laptop as a choice. One
  is added later as its own directory, its drivers and its probe
  (between HDMI and the 2.8-inch, since the DSI connector's I²C uses no
  header pin), when its panel is at hand to try. The original
  Raspberry Pi Touch Display (800 × 480) is not offered (owner, F145).
- **A screen whose check fails on its panel** (§9 step 11) and cannot
  be put right in its directory's values is not supported on the Pi: `docs/HARDWARE.md` §6 lists it with what failed,
  and it gets no image of its own. One case is
  kept apart: if the 2.8-inch's touch probe answers but its display
  overlay does not come up at run time, it takes the fallback in §3.3,
  which sets its pins at every boot; that is the owner's call.

### 3.2 The order of the probes

The boot script tries each in turn and stops at the first found. A
probe that finds nothing removes what it loaded before the next starts.

1. **HDMI connected.** `/sys/class/drm/card0-HDMI-A-1/status` reads
   `connected`: HDMI wins over any panel also attached, and no panel's
   overlay is loaded, so no panel's touch controller is bound or
   trusted. The Zero 2 W's mini-HDMI is the same connector. A monitor
   can read connected and give no modes (its EDID read failed), which
   leaves no `/dev/fb0`: when none comes within the wait, the script
   writes `on-digital` to that file, and the forced connector's next
   probe reads the EDID again.
2. **The 2.8-inch's GT911**, on the bit-banged I²C its touch overlay
   declares on the GPIO header: the touch overlay is loaded whenever
   HDMI does not read connected. It declares the GT911 at either of its
   two addresses, a gpio-backlight on GPIO 18, and the I²C pins and the
   GT911's reset and interrupt lines, so those are set up before the
   controller has answered. The probe passes when the controller's input
   device appears, with the name expected, on that bus and at one of the
   two addresses, within a set time (five seconds until §9 step 11
   measures it: the driver tries the two addresses in turn, and took
   about two seconds over a pair that did not answer, under QEMU). The DPI pin setup (GPIO 0-9, 12-17 and 20-25 to
   alternate function 2) is applied only once the GT911 has answered,
   by the display overlay.
3. **Nothing found:** HDMI, as the HDMI image starts today; a monitor
   plugged in later is set up by its hotplug.

The screen is chosen once per boot; a screen changed is a power cycle.

### 3.3 Each screen's overlay and where its settings live

- **The overlays are in the image's initramfs**, not read from the boot
  partition, since the kernel has no FAT driver. Each screen's touch
  overlay and display overlay is made at build time by Raspberry Pi's
  `ovmerge` from the firmware's overlay and the screen's parameters,
  into one overlay with no parameters, which the kernel's configfs
  interface takes as it is. They are part of `zImage`, so the release
  signs them.
- **A screen's directory** (`faraday/image/panels/any/screens/<screen>/`,
  in place of the earlier per-screen panel directories) holds its
  values (the `panel.conf` keys of today), its touch overlay's line and
  its display overlay's line, each a firmware overlay's name and its
  parameters. A DPI panel's display settings, its timings and
  output format included, are that display overlay line and nothing
  else: a change to the 2.8-inch's colour order is a change to that one
  line. The 2.8-inch's touch overlay has no source in the board's
  kernel, so it is a source file in the screen's directory with the
  values of Waveshare's overlay, laid out so configfs takes it: the
  kernel refuses Waveshare's compiled file, whose fragment aims at a
  node the same overlay creates. The display line is merged with
  `ovmerge` (`faraday/image/panels/any/README.md`).
- **Loading.** The display overlay is applied through configfs, then
  vc4's DRM driver is unbound and bound again
  (`/sys/bus/platform/drivers/vc4-drm`), so the DRM device it builds
  takes the new encoder; DRM's framebuffer emulation then makes
  `/dev/fb0` at the panel's mode. Whether this rebind brings the panel
  and the boot logo up is the first question on each panel (§9 step
  11). If the rebind leaves no `/dev/fb0` at the panel's mode, the
  script removes the overlay, binds vc4 again without it and starts on
  HDMI.
- **The 2.8-inch on the kernel's DPI driver.** The firmware's DPI lines
  (`enable_dpi_lcd`, `dpi_output_format`, `hdmi_timings`, the `gpio=`
  lines, `disable_hdmi`, `hdmi_ignore_hotplug`) and the firmware's
  framebuffer driver (`FB_BCM2708`) go: the panel is
  `vc4-kms-dpi-generic` with Waveshare's timings and its pins, through
  `DRM_PANEL_SIMPLE`. The colour order is that overlay's pixel format
  parameter, and the shell writes the format DRM's framebuffer reports,
  so the order is worked out again on the panel, as the touch grid and
  axes are. Waveshare's overlay pack also carries a KMS overlay for the
  panel; the firmware's generic one is used, with the timings from
  Waveshare's documentation.
- **The ribbon camera stays.** The one image keeps the camera the Pi
  images carry today: `start_x.elf`, `gpu_mem=128`, and `VIDEO_BCM2835`
  over `BCM2835_VCHIQ_MMAL` (`faraday/image/boards/pi3/linux.fragment`,
  "camera, legacy stack"), beside a USB webcam with uncompressed frames.
  - **What is known.** The camera runs today beside the 2.8-inch on the
    firmware's framebuffer. Nothing shows it failing beside the HDMI
    screen. Beside the 2.8-inch on the kernel's DPI
    driver it may not run, since the firmware's camera stack was used
    with the firmware's own display path.
  - **What is to check** (§9 step 11): the ribbon camera beside every
    screen, the 2.8-inch on the kernel's DPI driver included.
  - **If the firmware's stack fails beside the 2.8-inch**, the kernel's
    own camera drivers are the other path: unicam (`VIDEO_BCM2835_UNICAM`)
    and the sensor's V4L2 driver (`VIDEO_OV5647`, `VIDEO_IMX219`), with
    the sensor's overlay. That path gives raw Bayer frames, which need
    an image processor: the firmware's ISP, reached over VCHIQ
    (`VIDEO_ISP_BCM2835`), or a conversion in Faraday's own code.
    Whether it works beside the 2.8-inch on KMS, and what it adds to the
    kernel's reach, is checked and put to the owner before it is built.
- **The 2.8-inch's fallback**, used only as §3.1 says: `config.txt`
  always carries `vc4-kms-dpi-generic` and the touch overlay, the
  command line starts the panel off with `video=DPI-1:d`, and the boot
  script writes `on` to `/sys/class/drm/card0-DPI-1/status` when HDMI
  is not connected, a hotplug that sets the framebuffer up at the
  panel's size. Its pins are then set up at every boot, whatever is on
  the header.

### 3.4 The boot script and what the shell reads

- **Where.** `/etc/init.d/screen` in the Pi panel directory's root
  filesystem overlay, run by
  `faraday/image/common/rootfs-overlay/etc/init.d/rcS` as root, after
  `/sys` is mounted and before `mdev -s`, so the framebuffer and the
  touch controller's event node it brings up get their owners from
  `mdev.conf`, and before the app's loop in
  `faraday/image/common/rootfs-overlay/etc/inittab` starts; the PC's
  image has none. It is BusyBox `sh`.
- **What it mounts.** configfs at `/sys/kernel/config`, for the probes
  and the screen's overlay, and unmounts it before it ends, so no
  overlay can be loaded once the app runs.
- **What it writes.** One line, `/run/faraday-screen/args`, root's and
  readable by all: the shell's arguments for the screen found (§3.1),
  HDMI's from its `panel.conf` (its touch grid). The inittab passes the shell `/etc/faraday/args`
  (what the build fixes, the dev flags among them) and then this file.
  It writes nothing to the card.
- **What the shell reads.** The same arguments as today, from that file
  as well; the shell gains no source of its own. It still opens
  `/dev/fb0` and never a DRM node (F153).
- **What it says.** Each probe's result goes to `/dev/kmsg`, so the dev
  image's boot report says which screen was found and why.

### 3.5 The kernel options added

Each is a line in the Pi board's
`faraday/image/boards/pi3/kernel.required`, with why:

| Option | Why |
|---|---|
| `OF_OVERLAY` | applying a device-tree overlay while the kernel runs |
| `OF_CONFIGFS` | the interface the boot script applies it through (Raspberry Pi's kernel), with `CONFIGFS_FS` |
| `DRM_PANEL_SIMPLE` | the DPI panel behind `vc4-kms-dpi-generic` |
| `GPIO_RASPBERRYPI_EXP` | the 3B+'s HDMI hotplug line, behind the firmware's GPIO expander |
| `BACKLIGHT_CLASS_DEVICE` | `DRM_PANEL_SIMPLE` depends on it |
| `BACKLIGHT_GPIO` | the backlight on GPIO 18 that the 2.8-inch's touch overlay declares |
| `TOUCHSCREEN_GOODIX`, `I2C_GPIO`, `I2C_BCM2835` | the 2.8-inch's touch controller and its bus, and the HDMI bus the EDID is read over, from the 2.8-inch's and the HDMI screen's lists today |

`FB_BCM2708` moves to the board's `kernel.forbidden`, so no firmware
framebuffer competes with DRM's for `/dev/fb0`. `VIDEO_BCM2835` and
`BCM2835_VCHIQ_MMAL` stay in `kernel.required`, for the ribbon camera
(§3.3).
`docs/SECURITY-MODEL.md`, "When root is used", gains that the boot
script loads overlays, from the initramfs only, through configfs, and
unmounts it before the app starts.

### 3.6 Every Pi, and screens listed before they are tried

- **The image starts every Pi Faraday supports** (F147). It carries the
  device trees of the 3B, 3B+ and Zero 2 W, and the firmware loads the
  one for the board it starts on. The Pi 4 joins them, planned (§9).
- **A screen may be listed before it is tried on its panel** (F147),
  marked on the download page as looking for testers. Once it has been
  tried there, its values become fact, not data from its documentation,
  its tour is rendered at its size and in both orientations
  (`docs/ORIENTATION.md` §6), and the page marks it tested. The
  documents cite each panel's documentation and never say whose
  hardware it was tried on.

## 4. Release files and checking them

- **One image per platform** (F186):
  - `faraday-pi-X.Y.Z.img.xz`, for every supported Pi and every screen
    (§3), so the name carries neither board nor screen; it replaces
    `faraday-pi-X.Y.Z-waveshare-28dpi.img.xz` and
    `faraday-pi-X.Y.Z-hdmi.img.xz`, which earlier releases keep;
  - `faraday-stick-X.Y.Z-x86_64-uefi.img.xz`;
  - the PC's boot program, `faraday-X.Y.Z-BOOTX64.EFI`.

  The Pi image is built from the board `pi3` and the panel directory
  `any`, into `out/pi/faraday-pi3-any.img`. Nothing reads a
  release file by the names of an earlier release: the website takes
  the names from `faraday/builds.toml`, and the Starter tells a stick
  image from a Pi image by `faraday-stick-` and `faraday-pi-` alone.
- **`SHA256SUMS`** lists each `.xz`, each boot program, `IMAGES`, the
  `.img` inside each `.xz` under its own name, and each image's boot
  partition, its first partition, under the image's name with
  `-boot.vfat` for `.img` (`faraday-pi-X.Y.Z-boot.vfat`). Nothing
  writes to a boot partition, the boot script included (§3.4), so a
  card or stick already in use can still be checked against the
  release's signature, whatever screen it ran on. The README's
  "Verifying a release" shows both checks: the downloaded file, and a
  card or stick.
- **The release's image list** (`IMAGES`, a plain-text file in the
  release; the Starter carries it, §9 step 6) is staged in
  the release folder and covered by `SHA256SUMS`; the Starters are built
  with it (`docs/STARTER.md` §1.3). Its
  first line is `# Faraday X.Y.Z images, commit <full hash>`; then one
  line per image, in `faraday/builds.toml`'s order, its nine fields
  separated by one tab:
  1. the release file, `….img.xz`;
  2. the `.xz`'s SHA-256;
  3. the SHA-256 of the `.img` inside it;
  4. the platform, the entry's `board` (`x86_64-uefi`, `pi3`);
  5. the screen, its `panel` (`efi-framebuffer`, `any`);
  6. the boards it starts, its `starts`, joined by `, ` (`Pi 3B, Pi 3B+,
     Pi Zero 2 W`);
  7. the `.img`'s size in bytes;
  8. the build ID the image carries;
  9. for a stick image, the SHA-256 of its boot partition (the
     `-boot.vfat` value of `SHA256SUMS`); `-` for the Pi image.

  Fields 2, 3, 7, 8 and 9 are what writing a file needs beyond its
  name; a list with fewer fields is refused with its line.

  The Pi image's line, its tabs shown as two spaces and its sums
  shortened:

  ```
  faraday-pi-X.Y.Z.img.xz  9f2c…  41ab…  pi3  any  Pi 3B, Pi 3B+, Pi Zero 2 W  209715200  X.Y.Z  -
  ```
- **Each image has its entry in `faraday/builds.toml`**, the
  repository's list of builds: its recipe, the files it leaves, its
  release file names and the boards it starts, and the `[[hardware]]`
  entry it is offered under (§7). `faraday/tools/stage-release.sh`
  publishes every built image that names its release files, and
  `tools/lint-builds.py` holds that each names its `.img.xz` first and,
  for the PC, its boot program.
- **The Starter offers one Pi image.** Make a Pi card offers the one Pi
  image it found, for a Pi 3B, 3B+ or Zero 2 W with an HDMI screen or
  the 2.8-inch.
- **The website has one Pi tile** (§7); the hardware page sets out the
  screens it runs on.
- **Each image is reproducible** from the release's commit, as
  `docs/DEVELOPING.md` §5 says. A recipe builds one image by its
  platform, and the release recipe builds them all.

## 5. The Starter

- **It carries no images.** It holds its own code, Faraday Demo (F126),
  and the release's image list, built in from the release's `IMAGES`
  file (`docs/STARTER.md` §1.3).
- **It finds images.** It looks in the Downloads folder Transfer uses
  (`--downloads`, as F133's signed boot file is found), and in files
  dropped on its window. Each file whose name is on the image list is
  hashed and checked against its line in the built-in list, and only
  against it; a file that does not match is named and not offered. A file of another release is named with the
  version this Starter needs.
- **Make a Faraday stick and Make a Pi card offer what was found**, by
  its hardware: "Pi 3, Waveshare 2.8-inch". Hardware without its image is
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
per visit (F147): a person is getting the stick, the Pi image or the
Starter, and it asks nothing about screens (owner, 2026-11-03).

1. **Tiles**, one per `[[hardware]]` entry of `faraday/builds.toml`, in
   their order: the PC, then the Pi; side by side from 840 px wide, one
   above the other below. Each shows the hardware with Faraday's screen,
   its name, the Pis it fits, and one button, Download or Coming soon; whether an image is
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

The hardware page sets out the setups each image runs on, the Pi's
screens among them, says what each image carries and links
Faraday's posts on hardware risk. Boards and screens under study are
marked so (`docs/HARDWARE.md` §6).

## 8. What else changes

| Document | Change |
|---|---|
| `docs/HARDWARE.md` | §1 and §2: a row per image and screen; for the one Pi image (§3), each of its two screens built, not tried until tried, §4 the ribbon camera beside each screen as tried, §6 a screen whose check failed |
| `docs/STARTER.md` | §1.1 release files; §1.3 no images, the image list; §3.2 and §3.3 what is offered; §9 points here |
| `docs/SECURITY-MODEL.md` | the Pi's table and "What the kernel can reach"; the PC's command line; "When root is used": the boot script's probes and overlays, configfs unmounted before the app (§3.4) |
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
   Paused by the owner, as macOS is: nothing is built for it until
   the owner takes it up again.
3. The Pi's kernel: its fragment, its forbidden and required lists,
   and the image that boots with it, starting with the monitor (HDMI
   with a USB keyboard), which needs no new panel (built:
   `faraday/image/boards/pi3/`; the 2.8-inch was tried on its own
   image, which the one image replaces, step 9). Left: trying an HDMI
   monitor on a Pi 3B+, on the one image (step 11).
4. The PC's one build: the command line with
   `efi=disable_early_pci_dma`, one image and one boot program (built:
   `just faraday-stick-image`).
5. The release recipes, `SHA256SUMS` lines and the image list (built:
   `just faraday-release-stage` stages every released image of
   `faraday/builds.toml`, its boot partition's line and `IMAGES`, and
   refuses images of two builds; its second pass adds the Starters,
   built with `IMAGES`). Left: the
   size and staging time of a full set, measured on the next release's
   builds.
6. The Starter: no images, finding and checking them, the offers.
   Built: the Starter carries no payload; the release's image list
   (`IMAGES`, named by `FARADAY_STARTER_IMAGES` at build) is embedded and
   is all a found file is checked against; the finder
   (`faraday/shells/starter/src/finder.rs`): Downloads
   and dropped files are looked at, and each file whose name is on the
   list is hashed off the window's thread and found, named as not
   matching, or named as another release's with the version needed; the
   Starter keeps the answer (`Starter::image_findings`). The writer
   writes from a found file (`Source::Found`, from `Starter::image`): the `.xz`'s sum
   is checked before a byte is written, and the size and SHA-256 of
   what is written against the list's line; a
   file changed since it was found is refused with its path and sum. The
   offers (Make a Faraday stick and Make a Pi card, For which computer?,
   Not found) are built. Left: the finder tried by a person on a
   computer with a real release's images, which a release brings.
7. Each screen's values, from its documentation, and tried on the
   panel to be marked tested: the 10-inch HDMI (with the USB
   touchscreen rule, §2) on the HDMI image today, and the 2.8-inch and
   an HDMI monitor on the one image (steps 9 to 11).
8. The website's download page (§7), built around the images released
   today, with a tile for each platform as it is built.
9. **The one Pi image, with both screens** (§3, F186, F187; built):
   - the panel directory `any` with a directory per screen (the 2.8-inch
     and HDMI),
     each with its values, what its probe expects, and its touch and
     display overlay lines, a vendor's value marked to confirm (§3.1);
   - the overlays merged at build time with `ovmerge` and placed in the
     initramfs (§3.3);
   - the boot script, `/run/faraday-screen/args`
     and the inittab reading it (§3.4);
   - the kernel options and their `kernel.required` and
     `kernel.forbidden` lines (§3.5), the ribbon camera's kept;
   - `faraday/builds.toml` (one Pi image, under the Pi's one
     tile), `faraday/tools/stage-release.sh`, the Starter's one
     Pi offer, `docs/HARDWARE.md` (both screens built, not tried on the
     one image; §4's camera), `docs/STARTER.md`, `docs/ORIENTATION.md`,
     `docs/DEVELOPING.md`, `docs/SECURITY-MODEL.md`;
   - the per-screen panel directories `waveshare-28dpi` and `hdmi`, and
     their release files, go.

   No Pi image is released from the one image until the 2.8-inch has
   been tried on it (step 11).
10. **Tests without a screen**, run by the builder:
    - the boot script's choices against recorded and faked probe
      answers: HDMI connected with the 2.8-inch attached; its GT911
      answering; a controller that answers late, or under another name or
      address; a rebind that leaves no `/dev/fb0`; nothing found. Each
      gives the screen expected, or HDMI, and the arguments the shell
      is given;
    - the image under QEMU, which has no HDMI output and no panel:
      every probe finds nothing, the Pi falls back to HDMI, configfs is
      unmounted, and the app starts (and exits, with no framebuffer to
      draw on). The machine is `raspi2b` with the card's
      `bcm2710-rpi-3-b.dtb`, kernel and command line given directly:
      `raspi3b` shows nothing from this 32-bit kernel. QEMU does not run
      the Pi's firmware, so this does not test `bootcode.bin` reading
      the card.
11. **Each screen tried on the one image**, the 2.8-inch and an HDMI
    monitor (the owner, with the dev image, whose boot report carries each
    probe's lines, §3.4). A screen tried becomes tested in
    `docs/HARDWARE.md` and on the website, and its values fact; a value
    found wrong is put right in its directory. What each panel answers:
    - **Every panel.** Whether the display overlay applied through
      configfs and vc4's unbind and bind bring the panel up with
      `/dev/fb0` at its mode and the boot logo (§3.3); how long its
      probe takes when the panel is there and when it is not, which
      sets the probe's wait; the touch grid and axes the shell needs
      (`--touch-grid`, and swap or invert if any).
    - **The 2.8-inch.** Whether its GT911 answers on the touch overlay's
      bus before the DPI overlay is applied (whether the controller is
      powered and out of reset without the panel set up); the pixel
      format parameter of `vc4-kms-dpi-generic` that gives the right
      colours with the format the shell writes; that `DRM_PANEL_SIMPLE`
      is in the built configuration; Waveshare's timings on the
      kernel's DPI driver; the panel on a Zero 2 W. If the GT911
      answers but the run-time overlay does not come up, the owner
      decides on the fallback (§3.3).
    - **HDMI.** Whether `/sys/class/drm/card0-HDMI-A-1/status` reads
      `connected` by the time the boot script runs, on the 3B+ (with
      `GPIO_RASPBERRYPI_EXP`), the 3B and the Zero 2 W; that a monitor
      attached beside the 2.8-inch wins and the panel stays dark with its
      touch unbound; that a monitor plugged in after a boot with nothing
      found is set up by its hotplug and the app draws on it.
    - **The camera beside each screen**, the 2.8-inch included: the
      ribbon camera (Camera Module v1 or v2) on the firmware's stack
      with `gpu_mem=128`, and a USB webcam's frames. Beside the 2.8-inch on the kernel's DPI driver, if the firmware's
      stack does not run: whether unicam and the sensor's V4L2 driver
      do, with the ISP or without it (§3.3).
