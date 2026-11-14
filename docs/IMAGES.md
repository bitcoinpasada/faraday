# Images for each piece of hardware

**Kind:** Spec
**Status:** Specified (F145, F185, F186), the PC's one build, the Pi's one build, the one Pi image for an HDMI screen and the 2.8-inch (its screen directories, overlays, boot script and kernel options) and the release's files and image list built
**Open:** each screen tried on it (§9.11), a full release's size and staging time measured, the Starter's finder tried on a computer with a real release's images (§9.6), the website's choice

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

The PC's kernel also sets, in its own fragment and lists
(`faraday/image/boards/x86_64-uefi/`), five options for how a process
sees memory and the kernel's interfaces: `DEFAULT_MMAP_MIN_ADDR` at
65536, `ARCH_MMAP_RND_BITS` at 32 (x86_64's maximum), `PROC_MEM_NO_FORCE`,
and `LDISC_AUTOLOAD` and `CACHESTAT_SYSCALL` off. `kernel.required` holds
the first three (a number after `=` is the value the check requires) and
`kernel.forbidden` the other two, so the build's kernel-config check fails
on any of them. They are not in the Pi's lists, whose kernel is not checked
for them.

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
  builds the board's kernel source with the patches
  `docs/KERNEL-PATCHES.md` lists: two of Faraday's own, one to the EDID
  read (owner, F153, F205) and one that makes vc4's framebuffer 32 bits with
  red first, so the shell writes its frame without a conversion
  (`docs/FRAMES.md` §5; owner, F221). vc4 also binds the GPU's 3D block,
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
  devices"); what is plugged in at start is trusted and listed in one
  notice; a held device asks when it is used, with a notice naming it
  at plug-in (F202):
  - interfaces are authorised one at a time;
  - a new keyboard counts once it types the code on the screen;
  - a new pointer counts once a trusted pointer approves it, or, with
    nothing trusted that can point, once it presses the targets on the
    screen in a row, or Enter on a trusted keyboard lets it in (F202);
  - a new touchscreen counts once it taps the targets on the screen in
    a row.
- **A new USB touchscreen approves itself; nothing is trusted by name**
  (owner, F153, replacing F145's touchscreen named by the profile). An
  HDMI panel's touch reaches the Pi over USB, and on such a panel it is
  the only input there is. It is held back as every plugged-in device
  is, and the app draws targets, one at a time, each at a place drawn
  from the system's randomness (F202: as many in a row as keep a blind
  device's chance no better than a keyboard's, which a smaller screen
  needs more of; a press on the one showing brings the next, the last
  lets the touchscreen in); three presses beside keep it out until it is
  unplugged and count toward the limit of devices kept out, as a
  keyboard's wrong codes do, and every press beside is counted for the
  session, whether or not the device is unplugged or ignored: nine close
  the door to new keyboards and touchscreens (and to new pointers while
  no input is trusted)
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
tried on each (§9.10 and §9.11). A screen not
yet tried is built, not tried, in `docs/HARDWARE.md` and on the
website.

### 3.1 The screens

Sizes in dp are at the panel's own density; classes are
`docs/LAYOUT.md` §2. A value marked "to confirm" is the one the
panel's vendor documents, built in as it is and checked on the panel
(§9.11). A wrong one fails safe: the probe that expects it finds
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
- **A window on the HDMI screen** is asked for by `faraday.window=WxH`
  on the kernel command line (`docs/FRAMES.md` §6): the app draws in a
  centred window of that size and the rest of the screen is black. A
  screen that cannot hold it is drawn whole. The dev Pi image's line
  carries `1366x768`; the regular image's does not.
- **Both screens fit every Pi the image starts.**
- **No DSI screen for now** (owner, F187). Raspberry Pi Touch Display 2
  and Waveshare's DSI (C) panels are not built: none would be tried, and
  a person with the money for one has a cheap laptop as a choice. One
  is added later as its own directory, its drivers and its probe
  (between HDMI and the 2.8-inch, since the DSI connector's I²C uses no
  header pin), when its panel is at hand to try. The original
  Raspberry Pi Touch Display (800 × 480) is not offered (owner, F145).
- **A screen whose check fails on its panel** (§9.11) and cannot
  be put right in its directory's values is not supported on the Pi: `docs/HARDWARE.md` §6 lists it with what failed,
  and it gets no image of its own. One case is
  kept apart: if the 2.8-inch's touch probe answers but its display
  overlay does not come up at run time, it takes the fallback in §3.3,
  which sets its pins at every boot; that is the owner's call.

### 3.2 The order of the probes

The boot script tries each in turn and stops at the first found. A
probe that finds nothing removes what it loaded before the next starts.

1. **HDMI connected.** `/sys/class/drm/card0-HDMI-A-1/status` reads
   `connected`, asked again for up to two seconds, since a screen can be
   slow to say so and the file holds the kernel's last probe (a boot with
   only a panel attached takes those two seconds longer): HDMI wins over
   any panel also attached, and no panel's overlay is loaded, so no
   panel's touch controller is bound or trusted. The Zero 2 W's mini-HDMI is the same connector. A monitor
   can read connected and give no modes (its EDID read failed), which
   leaves no `/dev/fb0`: when none comes within the wait, the script
   writes `on-digital` to that file, and the forced connector's next
   probe reads the EDID again. A patch in the board's `linux-patches/`
   keeps a failed read from leaving a screen with nothing: `i2c-bcm2835`
   cannot be relied on for the three messages an EDID's block 2 or later
   takes, so a read that fails keeps the blocks before it (F205). (The
   kernel's own fallback modes for an EDID with no block read at all are
   in the 6.18 tree.) The second patch there is not about
   the EDID: vc4's framebuffer emulation is 32 bits a pixel in XBGR8888,
   red first in memory, with the colours the console reports to match
   (`docs/FRAMES.md` §5). The rebind below leaves no output poll pending
   on the freed device: 6.18's vc4 stops it when the device is released.
2. **The 2.8-inch's GT911**, on the bit-banged I²C its touch overlay
   declares on the GPIO header: the touch overlay is loaded whenever
   HDMI does not read connected. It declares the GT911 at either of its
   two addresses, a gpio-backlight on GPIO 18, and the I²C pins and the
   GT911's reset and interrupt lines, so those are set up before the
   controller has answered. The probe passes when the controller's input
   device appears, with the name expected, on that bus and at one of the
   two addresses, within a set time (five seconds until §9.11
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
  and the boot logo up is the first question on each panel (§9.11). If the rebind leaves no `/dev/fb0` at the panel's mode, the
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
  - **What is to check** (§9.11): the ribbon camera beside every
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
  release; the Starter carries it, §9.6) is staged in
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
| `faraday/image/README.md` | the directory layout only; the rest is §10 |
| `README.md` | "Verifying a release": a card or stick checked by its boot partition |

## 9. Order of building

Each part below says what is built and what is left, in the order the parts
depend on one another.

### 9.1 One image for every supported Pi

One Pi image for every supported Pi: the 3B, 3B+ and Zero 2 W
device trees on one card (built: the Zero 2 W's board directory is
folded into the Pi 3's), tried on the 3B and the Zero 2 W.

### 9.2 The Pi 4

The Pi 4 on the same images: its 32-bit kernel, firmware and device
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

### 9.3 The Pi's kernel

The Pi's kernel: its fragment, its forbidden and required lists,
and the image that boots with it, starting with the monitor (HDMI
with a USB keyboard), which needs no new panel (built:
`faraday/image/boards/pi3/`; the 2.8-inch was tried on its own
image, which the one image replaces, §9.9). Left: trying an HDMI
monitor on a Pi 3B+, on the one image (§9.11).

### 9.4 The PC's one build

The PC's one build: the command line with
`efi=disable_early_pci_dma`, one image and one boot program (built:
`just faraday-stick-image`).

### 9.5 The release recipes

The release recipes, `SHA256SUMS` lines and the image list (built:
`just faraday-release-stage` stages every released image of
`faraday/builds.toml`, its boot partition's line and `IMAGES`, and
refuses images of two builds; its second pass adds the Starters,
built with `IMAGES`). Left: the
size and staging time of a full set, measured on the next release's
builds.

### 9.6 The Starter

The Starter: no images, finding and checking them, the offers.
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

### 9.7 Each screen's values

Each screen's values, from its documentation, and tried on the
panel to be marked tested: the 10-inch HDMI (with the USB
touchscreen rule, §2) on the HDMI image today, and the 2.8-inch and
an HDMI monitor on the one image (§9.9 to §9.11).

### 9.8 The website's download page

The website's download page (§7), built around the images released
today, with a tile for each platform as it is built.

### 9.9 The one Pi image, with both screens

The one Pi image, with both screens (§3, F186, F187; built):
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
been tried on it (§9.11).

### 9.10 Tests without a screen

Tests without a screen, run by the builder:
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
  `bcm2710-rpi-3-b.dtb`, kernel and command line given directly, and
  the card a copy of the image grown to a power of two in size:
  `raspi3b` shows nothing from this 32-bit kernel. QEMU does not run
  the Pi's firmware, so this does not test `bootcode.bin` reading
  the card.

### 9.11 Each screen tried on the one image

Each screen tried on the one image, the 2.8-inch and an HDMI
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

## 10. How an image is built and what is in it

What `faraday/image/` builds and what the images hold. The directory's own
README is only its layout.

### 10.1 What an image is

An image that boots a machine straight into Faraday's app (installed as
`/usr/bin/faraday`, the name the inittab starts). There is no
shell, no login, no network and no writable root: the firmware loads a
kernel with the whole root filesystem inside it, `init` runs one program,
and when that program exits the machine powers off. The kernel holds the
drivers that board, that display, the input devices, the camera and the
boot medium need, and nothing else (`check-kernel-config.sh`).

Two devices are built from this directory, and they are the same image
with different boards:

| | `just faraday-pi-image` | `just faraday-stick-image` |
|---|---|---|
| The machine | a Raspberry Pi with a 2.8" touch panel, or with an HDMI screen: one image, which finds the screen at boot | any laptop or PC with UEFI firmware |
| The medium | a microSD card | a USB stick |
| What the firmware runs | `zImage`, named in `config.txt` | `EFI/BOOT/BOOTX64.EFI`, which is the kernel |
| The display | the screen vc4 drives: an HDMI screen at its EDID's mode, or the 2.8" panel, 480×640 at 286 dpi, whose overlay the boot script loads | whatever the firmware left running, read from sysfs |
| Input | the Goodix touch controller; also USB keyboards, mice and touchscreens | the built-in keyboard and touchpad, USB keyboards and mice, a HID touchscreen |
| Files | the card's data partition, read and written through the disk process (`faraday-disk`) | the stick's data partition and every FAT partition on every other USB disk, through the disk process |
| The image | `out/pi/faraday-pi3-any.img` | `out/stick/faraday-x86_64-uefi.img` |

Nothing here is shared with any other project.


The Pi is `pi3` and the panel `any`, one image for every screen (`panels/any`,
whose README gives the format of a screen's directory); the stick is
`x86_64-uefi` and `efi-framebuffer`:

```
just faraday-pi-image                                       # a test release
just dev=1 faraday-pi-image                                 # the same, with a login
just board=pi3 dev=0 faraday-pi-image                       # written out in full
just faraday-stick-image                                    # the USB stick
just dev=1 faraday-stick-image                              # the same, with a login
```

The stick's built-in command line has `efi=disable_early_pci_dma`
(`docs/IMAGES.md` §1): the board's `cmdline.txt` carries it and its
`kernel.cmdline` checks it against the built kernel's `CONFIG_CMDLINE`.

The Pi's command line is a file, so its `boot.cmdline` checks the words
`cmdline.txt` ends up with (`rng_core.default_quality=0` among them).

The Pi's kernel carries, in the board's own `linux.fragment`, vc4's display
side through DRM with its framebuffer emulation, the sound core vc4 cannot
build without (and no sound driver), the generic USB HID drivers, and USB
storage and SCSI; the board's `kernel.required` and `kernel.forbidden`
hold each of those to the PC's rules. It compiles the board's kernel
source with the patches `docs/KERNEL-PATCHES.md` lists (F153, F205, F221): vc4 binds the
Pi 3's V3D and creates a render node, which
`common/rootfs-overlay/etc/mdev.conf` gives to root alone with
`/dev/dri/card0`, and DRM selects `KCMP`, which the Pi allows while the
PC's list forbids it.

Each combination has its own build tree at
`out/pi/<board>-<panel>[-dev]/output` and its own image. The Buildroot
checkout, the download cache and the compiler cache are shared, so the
second combination costs what differs and not the toolchain — and the
stick shares all three with the Pi, even though it is a different
architecture.

### 10.2 The boards

| Board | Directory | State |
|---|---|---|
| Raspberry Pi 3B / 3B+ / Zero 2 W | `boards/pi3` | 3B+ tested — the owner's device; the 3B's and Zero 2 W's device trees are on the card, built, not booted |
| x86_64 PC with UEFI firmware | `boards/x86_64-uefi` | untested on hardware — booted in QEMU with OVMF by `faraday/tools/qemu-check.py` |

The 3B, 3B+ and Zero 2 W have the same BCM2710, so they take the same Rust
target, the same kernel and the same drivers; only the device trees differ.
One card carries all of them (`DTBS` in `board.conf`) and the Pi's firmware
loads the one for the board it starts on, so there is no separate Zero 2 W
image (F147). Nobody has booted a Zero 2 W or a 3B on it.

An ARMv6 Zero — the Zero W and the original Zero — is **not** here. It
needs a second Rust target (`arm-unknown-linux-gnueabihf`) and C flags for
ARMv6 with VFPv2, which is a board directory plus a toolchain to install
and check. Nobody has done that.

### 10.3 What each part holds

| File | common | board | panel | dev |
|---|---|---|---|---|
| `buildroot.fragment` | the system's shape | arch, kernel, firmware | the panel's packages | the login |
| `linux.fragment` | initramfs, devtmpfs, console, logo, no SysRq, and the list of subsystems no image builds | this machine's drivers, its boot medium, its input devices, its camera | display and touch drivers | a virtual terminal |
| `busybox.fragment` | init, one shell, mdev, the uid drop | applets this board's `rcS` needs | — | getty, login, su, passwd |
| `kernel.forbidden` / `kernel.required` | what no image builds, and what every image needs | what this machine does not have, and what it does not work without | the display's and the touch controller's drivers | — |
| `genimage.cfg` | — | the layout of the medium | — | — |
| `post-image.sh` | the exchange partition's note, then genimage | anything the layout must find first | — | — |
| `config.txt` | — | the firmware's settings | the panel's part | — |
| `cmdline.txt` | `quiet` and the console's own settings | the console, the density | — | `-quiet` |
| `dev/` | — | the dev variant's part of this board: which console the probe lines go to | — | — |
| `rootfs-overlay/` | inittab, rcS, mdev.conf | — | — | — |
| `inittab.in` | — | — | — | the release inittab plus a getty on `@TTY@` |
| `users.table` | the `faraday` account | — | — | — |
| `board.conf` / `panel.conf` | — | the toolchain, the DTBs, the firmware, the exchange partition, the console tty | what the shell must be told about the display | — |

`build.sh` walks common, board, panel, dev, `boards/<board>/dev` in that
order. `config.txt` is
the parts concatenated. `cmdline.txt` is the parts' words joined, with
comments stripped and a word written `-quiet` removing the `quiet` an
earlier part contributed. The Buildroot fragments are merged with
Buildroot's own `merge_config.sh`, so a later part's value replaces an
earlier one's rather than sitting next to it. The kernel fragments, the
BusyBox fragments, the root filesystem overlays and the users tables
become space-separated lists in a generated fragment; nothing else in
the tree holds a path.

### 10.4 Adding a board

A directory under `boards/` with three fragments and a `board.conf`:

- `board.conf` — `NAME`, the toolchain
  `just faraday-pi-bin` builds the shell with (`RUST_TARGET`, `CC`, `CFLAGS`), and
  how the machine boots:

  | Key | Meaning |
  |---|---|
  | `FIRMWARE` | `rpi` if the firmware reads `config.txt` and `cmdline.txt` off the boot partition; `none` otherwise |
  | `CMDLINE` | `file` if the firmware passes the kernel its command line; `kernel` if the line is compiled in (`CONFIG_CMDLINE_OVERRIDE`) |
  | `DTBS` | the device trees that go on the boot partition and into `BR2_LINUX_KERNEL_INTREE_DTS_NAME`; absent on a machine described by ACPI |
  | `DTS_DIR` | the folder of the kernel's `arch/<arch>/boot/dts` the `DTBS` are in, if not the top (`broadcom` on the Pi); put before each name for `BR2_LINUX_KERNEL_INTREE_DTS_NAME`, not on the card |
  | `EXCHANGE` | the data partition of the medium the machine started from, written to `/etc/faraday/exchange`: a node, or `layout` for a stick, which `faraday-grant --find-boot` finds at start by its layout and the build it holds (`docs/STICK-IDENTITY.md` §5.2) |
  | `CONSOLE_TTY` | where the dev variant's getty runs |
  | `IMAGE` | what this board's `genimage.cfg` calls the finished image |

- `buildroot.fragment` — the architecture, the kernel tarball and
  defconfig, the firmware package, and a `# KERNEL_SHA256 <hex>` line,
  which `build.sh` checks the downloaded tarball against.
- `linux.fragment` — this machine's own drivers: the display, the input
  devices, the boot medium, the camera, the console. A board with the
  same SoC as one already here links to that board's file rather than
  copying it.
- `kernel.forbidden` and `kernel.required` — what this machine does not
  have and what it does not work without, each line a symbol and the
  reason. The stick and the Pi require `HID`, `SCSI` and
  `USB_STORAGE`. That is what these files are per board for.
- `genimage.cfg` — the layout of the medium, with `@DTBS@` where the
  device trees go.
- `post-image.sh` — optional, and run before the one that calls
  `genimage`: whatever the layout expects to find in `images/`.
- `config.txt` — only on a board whose `FIRMWARE` is `rpi`.
- `dev/` — optional: the parts that are this board's *and* the dev
  variant's, which today is the console the probe lines go to. Its
  `cmdline.txt` does not set `faraday.window=`, so the dev image draws
  an HDMI screen at the full mode the Pi sends; the window itself
  (`docs/FRAMES.md` §6) is kept, and a card's `cmdline.txt` can add it.

A board whose toolchain is not already installed is that much more work:
`rustup target add` its Rust target and a cross-compiler from the
distribution.

### 10.5 Adding a panel

A directory under `panels/` with four files:

- `panel.conf` — `NAME`, and then only what is true of this display:
  `WIDTH` and `HEIGHT`, `DPI`, `TOUCH_NAME` (a controller on the board's
  own bus, never USB, which needs `TOUCH_GRID` with it) and `TOUCH_GRID`, `FB`,
  `LOGO_WIDTH` and `LOGO_HEIGHT`, and the panel's orientation
  (`docs/ORIENTATION.md` §2, §4.2):

  - `NATIVE` — `portrait` or `landscape`, the way the panel is handed
    over (`--native`).
  - `MOUNT` — `90` or `270`, the quarter turn that shows the frame
    upright in the orientation that is not the native one (`--mount`).
    A panel that states it is offered Rotation in Settings; one that
    does not (the 2.8-inch panel) is not.
  - `TOUCH_SWAP=1`, `TOUCH_INVERT_X=1`, `TOUCH_INVERT_Y=1` — a touch
    controller whose x runs along the panel's height, or whose x or y
    runs the other way (`--touch-swap`, `--touch-invert-x`,
    `--touch-invert-y`). They describe the hardware and do not change
    with Rotation.

  Each becomes a flag in
  `/etc/faraday/args`, which the inittab passes to the shell, and the
  size pair also sets the size of the boot logo. No value may contain a
  space: the args file is word-split by the shell that starts the app.

  A soldered-on panel states all of it. A firmware framebuffer states
  almost none of it: it reports its own size in sysfs, no firmware
  reports a density, and there is no touch controller with a fixed name
  to look for. `panels/efi-framebuffer/panel.conf` is what that looks
  like — a name, `/dev/fb0`, and a logo size, because a logo is the one
  thing that has to be decided before the machine is on.
- `buildroot.fragment` — any package the panel needs, such as a vendor's
  device tree overlays.
- `linux.fragment` — the framebuffer driver and the touch driver.
- `config.txt` — the timings, the pin map and the overlays the firmware
  needs to bring the panel up.

The Pi's one panel, `panels/any`, is different: it names no screen, and
holds a directory per screen under `screens/` (`panels/any/README.md`
gives its format: `panel.conf`, `probe.conf`, `touch.line` and
`display.line`). A new Pi screen is a new directory there and a line in
`screens/order`; its kernel drivers are in the board's `linux.fragment`.

Nothing in the shell binary is panel-specific, so a new panel needs no
Rust. Bring it up with `just dev=1 faraday-pi-image`, read the probe lines on the panel
itself,
and only then build the release image.

### 10.6 The dev variant

`just dev=1 faraday-pi-image` is for bringing up a new board or a new panel
without a serial
adapter: the kernel console goes to `tty1` as well as the serial port and
`quiet` is dropped, so the probe lines scroll on the panel until the app
starts, when the console level drops to 1 (the dev inittab; not under QEMU,
whose serial console the checks read) so the kernel log stays off the screen
and the serial port, and comes back if the app ends with anything but a
lock; a getty runs on
the serial port and the root password is empty; and the app is started with
`--verbose --timings --boot-report`, so it says what it found and records
what its loop and its costly work cost and what the board reports.

`--timings` writes `faraday-timings.txt` onto the data partition,
through the disk process since the image mounts nothing: the whole file
every 30 seconds and once at exit, which a pulled stick may miss, so wait
half a minute after the last thing to measure. It holds every run
since boot, in order, each under a `run N` line (a lock ends the app and
init starts the next, which carries the earlier runs in
`/run/faraday/timings-carry` and so numbers its own; past 1 MiB the
oldest runs go, under a line saying so), and in each run one block for
every five seconds of the shell's clock, and one more at exit,
each holding the passes and touches of the interval and, per bucket, a
count, a mean and a worst in milliseconds. The buckets are `touch` (one
touch through the core, layout and paint included), `tick`, `camera` (one
camera frame through the core, which is the preview and nothing else),
`scanned` (routing a code the scanner's worker read), `convert` (the
frame packed for the panel), `write` (the `pwrite` to the
framebuffer), `frame` (the core's frame drawn in full) and `blend` (the
core's frame as a fade's step, two kept frames blended). After the
buckets, a `grid` line counts the passes that drew at each step of the
frame grid (16.7, 33.3, 50 and 66.7 ms) and gives the smoothed pass cost
the step was chosen from and how many times the step changed in the
block (`changes`), a `clock` line gives the first CPU's clock at
the passes that drew (the header names its cpufreq driver and governor),
a `steps` line lists each fade step that followed another as the gap
since it, what its pass cost and the step chosen after it, in ms, and a
`fade` line says how many changes faded,
were drawn at once because the last fades' steps came too far apart
(`skipped`) or because reduce motion is on (`reduced`), how many fade
steps were blended, how many recent step gaps are known and their median,
and whether the next change would fade (`docs/MOTION.md` §3.5); the line
also gives the longest of those gaps (`worst_ms`), the most bands a
blend was split in (`bands`, 0 when no step ran) and, after `on` or
`off`, `was on` or `was off` when the gate changed since the last
block, so a change is placed within five seconds. The header names the
panel's size and its pixel count. A fade's step is told apart in the
file by its `blend` bucket (the core's blend alone, the bands' threads
included), `convert` and `write` (the copy out), and the `steps` line's
pass cost, which holds all three; the wait for the buffer is the gap
less the pass cost. A
`part` row for each call the loop makes outside the buckets that ran in
the interval gives its count, mean and worst in ms and its name last:
`rescan` (the look for input devices plugged in), `sticks` (the look at
the storage plugged in), `report` and `bootreport` (those two files'
own calls, as a whole), and `clockwrite` (one write of the CPU's lowest
clock, made inside the pass). A `moved` and a `whole` line give, for the
passes that drew a moved or a whole frame, their count, the mean and
worst of their `frame` calls and the mean and worst of their `write`
calls, in ms (a fade's step is on neither). A `by_screen` line for each
screen the interval drew (at most 24, then one `other screens`) gives
the passes that drew it, their mean `frame` and mean `write` in ms, the
share of them that were moved frames, and the screen's name last. A `slowest`
line gives the slowest pass that drew, from the end of its wait to its
frame written, how much of it no bucket and no part explains, and what
it was: the screen shown (its name only), whether the frame was whole,
moved or a fade's step, whether the screen changed since the last pass
that drew, and the largest bucket or part in it with its ms. Then a
`work` row for each kind of costly work the interval did, with its
count, mean and worst in ms and its kind last: the shell reads the
core's counts of costly work (`faraday_core::work`) around each event it
hands the core and times the call, so an event goes on the row of the
costliest kind it did (`vault unlocked Light 64 MiB 3 passes`,
`vault made Standard 256 MiB 3 passes, failed: no memory`,
`vault cost measured`, `key made`, `transaction signed`, `qr made`, and
so on, with `(in a frame)` after work a frame did and `(stick files
served)` after work done answering the disk process); `qr decoded
(camera pass)` is one of the scanner worker's passes over a camera
frame, `file written` a file the app wrote to a stick or the card, and
`boot report written` the report's own write. A row's name is a kind
and a cost, never a name or anything a person typed. The shell's
README describes the format.

`--boot-report` writes `faraday-boot-report.txt` beside it, whole at
the same rate (`docs/DEVELOPING.md` §6.2): the kernel log, the panel and
devices found, a system block (the seconds to the first frame, when the
random generator was ready, the temperature, the first CPU's clock, the
undervoltage alarm where the kernel has the `rpi_volt` sensor (the Pi's
dev kernel builds both it and the temperature sensor:
`boards/pi3/dev/linux.fragment`), the
memory free and the app's peak, each now and at its worst), a radios
block (`/sys/class/bluetooth` and `/sys/class/net`, the device tree's
Bluetooth and Wi-Fi nodes with their status and their parent's, the
serial ports and what is bound to them) and a device nodes block (each
entry of `/dev/dri` with its mode, owner, group and major:minor, which
show that the screen's nodes are root's alone).

To collect a run: boot the card, scroll the Settings list for ten seconds,
open the scanner and point the camera at anything that is not a QR code for ten seconds (a code is decoded at once and the scanner stops),
open Learn and scroll a page, then go to Settings → Wipe and exit. The
device powers itself off after the last block is written. Put the card in
a computer and read `faraday-timings.txt` from the data
partition; it is plain text, one number per column, so it reads on a
phone. **It never goes on a card that signs
anything.** It exists because the release image has no way in at all, which
is the point of it and useless when the question is why the panel is blank.

### 10.7 What is in it

| Piece | Choice |
|---|---|
| Buildroot | 2024.02 LTS, pinned to a release tag in the `justfile` |
| Kernel | The board's, **with its defconfig's `=m` lines dropped**, plus the `linux.fragment`s: for `pi3`, the Raspberry Pi 6.18 tree (6.18.59) at the commit its `buildroot.fragment` pins, with the patches `docs/KERNEL-PATCHES.md` lists (`linux-patches/`, `docs/IMAGES.md` §3.2), and its `bcm2709` configuration; for `x86_64-uefi`, mainline 6.18.56 (6.18 longterm) from kernel.org with five commits from 6.19 that put the screen's EDID in `boot_params` (`linux-patches/`, `docs/KERNEL-PATCHES.md`, F23) and `x86_64_defconfig`. Each board's `buildroot.fragment` carries the tarball's sha256 and `build.sh` checks it |
| C library | musl, static only. BusyBox is the one package Buildroot builds for the target, so there is no shared library in the image at all |
| Firmware | On a Pi, `rpi-firmware`, the `PI_X` variant, so `start_x.elf` and `fixup_x.dat` are on the card. On the stick, none: the machine's own firmware runs the kernel, and no blob of any kind is in the image |
| Root filesystem | An initramfs (cpio) linked into the kernel image. BusyBox `init`, one shell, `mdev`, `setpriv`, `setuidgid`, `poweroff`, and `/usr/bin/faraday`; nothing that finds a filesystem by its label or UUID (`findfs`, `blkid`, `volume_id`). No `/lib/*.so` |
| The binary | Built on the host by `just faraday-pi-bin` with the board's toolchain, as a static executable, and installed by `package/faraday`. Buildroot never builds Rust |
| Overlays | The Pi's screens' overlays are merged at build time by Raspberry Pi's `ovmerge` (`utils`, at a pinned commit and sha256) from the pinned kernel's overlay sources, and compiled by Buildroot's `dtc`; the 2.8-inch's touch overlay is a source file in its screen's directory, compiled by the same `dtc`. `build.sh` builds that `dtc` first. Run on the panel with the pixel clock at 22 MHz: the wait held a measured 16.68 ms, and a run showed no tearing to the eye |

### 10.8 The kernel is the defconfig with its modules dropped

A board defconfig is a general-purpose kernel for a general-purpose
computer, and `bcm2709` has 1203 `=m` lines. Kconfig turns a `=m` into a
`=y` when `CONFIG_MODULES` is off, so `# CONFIG_MODULES is not set` on its
own did not drop those drivers — it built every one of them in.

`external/external.mk` therefore deletes every `=m` line from the
defconfig in the unpacked kernel tree, at `post-patch`, before Buildroot
reads it, and turns `CONFIG_MODULES` off there as well so an unnamed
tristate resolves to `n` rather than to `y`. What the device needs is then
named, with a reason, in the three `linux.fragment` files: the SoC, the
card, the panel, the touch controller, the camera, the framebuffer,
evdev, V4L2, the initramfs, devtmpfs and the serial console.

`CONFIG_NET` is out, so the image has no socket layer, no IPv4 or IPv6, no
netfilter, no wireless or Bluetooth stack, and no Ethernet, Wi-Fi or USB
network driver. So are sound, DRM, USB gadget, HID, mass storage, SCSI,
infrared, DVB, IIO, MD, six filesystems, and the tracing and profiling
machinery; the Pi alone brings back DRM for the
display, the sound core with no driver, HID and mass storage, each in
its board's lists. `check-kernel-config.sh` is the list, with a reason per line.

### 10.9 The kernel configuration is checked on every build

`check-kernel-config.sh` reads two lists out of each part directory:
`kernel.forbidden`, options that must not be built, and
`kernel.required`, options that must be, each line a symbol and the reason
it is in the list; a required line may be `SYMBOL=NUMBER`, for an option
whose value is a size or a bit count. `build.sh` runs it after `make linux-configure` has
generated the kernel's `.config` and before the kernel is built, so a
forbidden option fails the build in seconds and the offending line is
what it prints. A driver that quietly lost a dependency — because the
subsystem above it went out — fails the same way, rather than turning up
as a black panel on the owner's desk.

The lists are per part because the devices differ. `common/` holds what no
Faraday image builds and what every one needs; a board holds what that
machine does not have and does not work without.
`HID`, `SCSI` and `USB_STORAGE` are in the stick's and the Pi's required
lists and in no forbidden list, which is the whole reason these are files in board directories
rather than one list with exceptions in it. A symbol in both lists at
once fails the build too: two rules that cannot both hold would otherwise
be settled by whichever ran last.

### 10.10 The shell binary is reinstalled on every build

`package/faraday` installs a file it did not build, so once Buildroot
has stamped it installed it never copies it again: an image rebuilt after
`just faraday-pi-bin` would carry the previous binary. `build.sh` therefore runs
`make faraday-reinstall` before every build, and `rpi-firmware-
reinstall` with it, for the same reason — that package copies the assembled
`config.txt` and `cmdline.txt` onto the boot partition. Both are a handful
of file copies; the rootfs and the kernel's initramfs re-link follow.

After the build, `build.sh` checks that it worked: it pulls
`usr/bin/faraday` out of the rootfs cpio and compares the bytes of
its `.text` with the binary `just faraday-pi-bin` produced. The two files are not
identical — Buildroot strips the installed copy — but the code must be. A
stale binary fails the build.

### 10.11 The boot logo

The framebuffer console takes the display a second or two after power, well
before the app has a frame, and paints the kernel's logo on it.
`common/make-logo.py` generates that logo at the panel's exact size:
`common/faraday-mark.txt` is Faraday's mark as a pixel grid, scaled by a
whole number to about a quarter of the logo's width, with the word FARADAY
under it in a 5 × 7 pixel font the script holds. The kernel draws its logo
at the top left and does not scale it, so a full-panel image is how a mark
is centred; a PC's display is not known until it is on, so its logo is a
square, its side the mark and name's longer side times the golden ratio
(the pair drawn at the size a 320-wide logo gives it), that
`fbcon=logo-pos:center` puts in the middle. The logo
is drawn as the app's Light page: Cobalt Light's surface, the shield in
primary with its edge lit half with white, the name in primary (owner,
2026-10-27). On the Pi it is the whole screen; on a PC it is a light square
in the middle of a black one.

`external.mk` copies the generated file over the kernel's own
`drivers/video/logo/logo_linux_clut224.ppm` when the tree is unpacked, and
`CONFIG_LOGO` and `CONFIG_LOGO_LINUX_CLUT224` in `common/linux.fragment`
make the kernel compile and draw it. The console stays on the serial port,
so the panel gets the logo and never a line of text. The logo stays up
until the app's first frame paints over it.

Two details. The file is a plain (ASCII, `P3`) PPM because the kernel's
`pnmtologo` refuses binary PNM outright. And the copy happens once per
unpacked kernel tree, so `build.sh` compares an existing tree's logo
with the generated one on every build and, when they differ, copies the
new one in and has the kernel compiled again ("==> the boot logo
changed"). A stick or card already written keeps its old logo until it
is written again from a new image.

### 10.12 The app is not root

Init is root; the app is not. The inittab's app line is

```
::wait:/bin/sh -c 'while /bin/setpriv --nnp /usr/bin/setuidgid faraday /usr/bin/faraday $(cat /etc/faraday/args); ...'
```

`setpriv --nnp` sets `no_new_privs`, so nothing the app execs can gain
privileges. `setuidgid faraday` sets the group, drops the
supplementary groups and sets the uid, then execs. BusyBox's `setpriv`
cannot do that part: 1.36 accepts only `--dump`, `--nnp`,
`--inh-caps` and `--ambient-caps`, and has no `--reuid`, `--regid` or
`--clear-groups`.

`faraday` is uid and gid 200, from `common/users.table`
(`BR2_ROOTFS_USERS_TABLES`). Its password is `*`, its home is `/` and
its shell is `/bin/false`, so it cannot be logged into even on the dev
variant's serial console.

What that account can reach, and nothing else:

| Node | Owner | Mode | Why |
|---|---|---|---|
| `/dev/fb0` | `root:faraday` | 0660 | the panel, written once per frame |
| `/dev/input/event*` | `faraday:faraday` | 0640 | the touch controller, read only |
| `/dev/video*` | `root:faraday` | 0660 | the camera; V4L2 queues buffers through it |
| `/dev/random`, `/dev/urandom` | `root:root` | 0444 | the session's entropy |
| `/dev/null`, `/dev/zero`, `/dev/full`, `/dev/tty` | `root:root` | 0666 | what any program expects |
| everything else | `root:root` | 0660 | mdev's default, which an unprivileged process cannot open |

`common/rootfs-overlay/etc/mdev.conf` holds those rules and `mdev -s` in
`rcS` applies them. `/dev/mmcblk0` and `/dev/mmcblk0p1` — the card and
the boot partition — `/dev/i2c-*` and `/dev/vchiq` are covered by the
default and stay root's.

The run directory `/run/faraday` is in RAM and the app's alone. It holds
what a lock carries to the next process: the Inbox and Outbox, what the
app keeps for itself, and a file `mode`. Before a lock the shell writes
the mode in use there (`mainnet`, `testnet`, `signet` or `regtest`); the
next process reads it, removes it, and starts in that mode. No file, an
unreadable one, or any other text starts Mainnet, and a power-off leaves
none.

Powering the board off is init's: the app exits, and `::wait:/sbin/poweroff
-f` is the next line the inittab runs.

### 10.13 What the kernel does not build

The shell opens a framebuffer, an input device, a capture device and
files on the card's second partition. `common/linux.fragment` compiles
out every other way to reach memory, firmware or storage, so a
compromised shell — a decoder exploit, say — has nowhere to persist to
and nothing extra to read:

`BCM_VCIO` (`/dev/vcio`, the firmware mailbox, and with it `vcgencmd
otp_set`), `DEVMEM` (`/dev/mem`), `DEBUG_FS`, `SWAP`, `MTD`,
`EEPROM_AT24`, `EEPROM_AT25`, `EEPROM_93CX6`, `NVMEM_SYSFS`,
`BLK_DEV_RAM`, `PROC_KCORE`, `KEXEC`, `BLK_DEV_LOOP`, `BLK_DEV_NBD`,
`CRYPTO_USER_API`, `PROFILING`, `KPROBES`, `FTRACE` and `BPF_SYSCALL`.

`RASPBERRYPI_FIRMWARE` and `BCM2835_MBOX` stay: the framebuffer needs
the in-kernel mailbox. What goes is the userspace door to it.

Three things stay on that read as if they should not, each for one
driver. `CONFIG_PM`, because the firmware power-domain driver depends on
it and the USB controller and the camera sit behind those domains;
`SUSPEND` and `HIBERNATION` are out. `CONFIG_WATCHDOG`, because
`BCM2835_WDT` owns `pm_power_off` on a Pi and `poweroff -f` does nothing
without it; nothing opens `/dev/watchdog`. `CONFIG_STAGING`, because the
legacy camera lives there.

### 10.14 USB: a webcam, input and sticks

On the Pi, the board's host controller and `USB_VIDEO_CLASS` are built on
purpose, so a plain USB webcam appears as a `/dev/video*` and the shell —
which takes the first one — can scan with it. The Pi's USB began with that. Gadget mode and the USB network adapters
(which went with `CONFIG_NET`) are out. The Pi also carries USB
keyboards, mice, touchscreens and sticks, with the stick's drivers and
the stick's rules: `faraday-grant` authorises each interface, a new
keyboard types the code on the screen, a new pointer is approved with
trusted input, a new touchscreen taps the target the app draws at a
random place, and a stick's partitions reach `faraday-disk` one at a
time. No USB device is trusted without approval, whatever name it gives
(F153); the panel's own touch on I²C is.

On the stick, USB carries the camera, the keyboard, the mouse and the
disk the machine booted from. Gadget mode and the network adapters are
out there too, for the same reasons.

### 10.15 Writable non-volatile memory on a Pi

| Store | Size | How written | Wipeable | Reachable by the app |
|---|---|---|---|---|
| microSD boot partition | 64 MiB | raw `/dev/mmcblk0p1` | yes, by rewriting the card | no: never mounted, node is root's |
| microSD exchange partition | 128 MiB | files, through the disk process | yes | yes, and this is the only store it writes |
| SoC OTP (BCM2837) | 8 customer rows, 256 bits | firmware mailbox | **no**, one-time | no: `BCM_VCIO` is not built |
| GT911 touch controller | ~186 bytes of config flash | I2C register write | overwrite | no: `/dev/i2c-*` is root's, and the image ships no controller firmware |
| Camera module (OV5647, IMX219) | none | — | — | — |
| Wi-Fi/BT chip (CYW43455) | none of consequence | — | — | disabled by overlay, no driver built |
| Boot EEPROM | none on a Pi 3 | — | — | — |
| DRAM | — | — | power-off | the app zeroizes; `poweroff -f` after exit is what clears the rest |

The one store that cannot be wiped is the OTP, and it is exactly the size
of a 24-word seed's entropy. The answer is reachability, not wiping:
nothing in the image can write it. A factory wipe of a card is writing
the image to it again (Faraday Starter, or `dd`).

### 10.16 SysRq is off

`# CONFIG_MAGIC_SYSRQ is not set`. Nothing should be able to reboot the
device, kill the app or dump kernel state from an input device. The device
is meant to be unreachable except through the screen, and the magic keys
are a way around that.

### 10.17 The partition layout

#### The Pi's card

Two FAT partitions, both `0x0C`:

1. **boot**, 64 MiB, FAT16, labelled `bootfs` as Raspberry Pi OS's is, holding about 16 MB. What the Pi firmware reads: `bootcode.bin`,
   `start_x.elf`, `fixup_x.dat`, `config.txt`, `cmdline.txt`, `zImage`,
   `overlays/`, and the board's device trees — `bcm2710-rpi-3-b.dtb` and
   `bcm2710-rpi-3-b-plus.dtb` for `pi3`, from its `board.conf`.
   `boards/pi3/genimage.cfg` is the template; `build.sh` fills the device
   trees in.
2. **exchange**, 128 MiB, FAT32, with no label, holding one `README.txt` that
   says what it is for. Init mounts nothing (it mounts only `proc`,
   `sysfs` and `dev`, and the kernel has no FAT driver): the disk process
   reads and writes the partition, and it is the file channel. A card
   someone repartitioned still boots into the app.

There is no root filesystem partition, and nothing but this partition is
read or written after boot. The boot partition is never mounted. The
card's MBR disk signature is made from the commit date and its role
(`image-id.sh`), as the FAT serials are (`fat-serial.sh`); the Starter
gives each card it writes its own (`docs/STICK-IDENTITY.md` §4).

#### The stick

GPT, because that is what a UEFI firmware looks for on a removable disk,
and because the EFI system partition is a GPT partition type. Two
partitions, 1 MB-aligned, with the names an ordinary stick has (F174,
`docs/STICK-IDENTITY.md` §3) and given UUIDs, so that two builders get
the same bytes: `image-id.sh` makes the disk's and each partition's from
the commit date and its role, in the form of version 4 UUIDs, so they
spell nothing and differ from one release to the next. The Starter gives
each stick it writes its own (§4.2); a stick written by another tool
keeps these, and its data partition gets a FAT serial of its own at its
first start (§7). Nothing in Faraday knows the stick by its names or IDs:

1. **EFI system partition** (its GPT name), 64 MiB, FAT32, labelled `EFI`,
   bootable,
   holding exactly one file: `EFI/BOOT/BOOTX64.EFI`. That file is the
   kernel. `CONFIG_EFI_STUB` makes `bzImage` a PE/COFF application with
   its own loader, and the whole root filesystem is the initramfs inside
   it, so there is no bootloader, no shim, no configuration file and no
   blob anywhere on the partition. The firmware finds it because
   `EFI/BOOT/BOOTX64.EFI` is where a removable disk's default loader
   lives.
   The file is about 10 MB; 64 MiB holds a boot file of 30 MB twice
   over (F80). A FAT32 filesystem is only valid with at least 65525
   clusters: `mkfs.fat` makes a smaller one anyway, with a warning, and
   OVMF refuses to mount it — which shows up as "failed to load ... Not
   Found" and no boot at all. At 64 MiB it has 129022.
2. **Basic data partition** (its GPT name), 128 MiB, FAT32, with no
   label, the same partition and the same note as the Pi's. The stick is
   `sda` on one machine and `sdb` on the next, so at start
   `faraday-grant --find-boot` looks among the USB disks for the one with
   this layout (§5.1) whose boot partition holds the running kernel's
   release string (`faraday_boot::holds_release`), and writes its data
   partition's node to `/etc/faraday/exchange`. Where none or two are
   found it writes `none` or `two`: no data partition is the device's
   own, and Files says which. Until it has written, no partition is handed
   out.

The image is 193 MiB and 20 KiB: 1 MiB before the first partition, the
two partitions, and the backup GPT. A stick or card made before F80 has
a 48 MB (stick) or 32 MB (card) boot partition and a 32 MB FAT16 data
partition. Faraday reads and writes that data partition as any other;
the Starter's keep data refuses to write a
64 MiB boot partition onto it, so its files are moved to a new stick by
hand.

No boot partition is handed to the app on any medium
(`docs/STICK-IDENTITY.md` §5.2): `faraday-grant` keeps root's the first
partition of the medium the machine started from, the Pi's own card's
first, and every partition whose GPT name is `EFI system partition` (or
`FARADAYBOOT`, on a stick made before F174), and `faraday-disk` refuses
the same and any FAT volume labelled `EFI`, `bootfs` or `FARADAYBOOT`, in
its boot sector or its root directory (`docs/SECURITY-MODEL.md` "When root is used"). That
keeps back any stick's EFI system partition, which fails closed: Faraday
writes no boot partition (F128).

### 10.18 The USB stick on a laptop

#### What it needs from the machine

- **UEFI firmware.** A PC that boots only in legacy BIOS mode will not
  boot this stick, and nothing in the image can be made to. There is no
  MBR boot sector and no legacy bootloader.
- **Secure Boot off, or this image's hash enrolled.** The kernel is not
  signed by Microsoft's CA and carries no shim, so a machine that
  enforces Secure Boot rejects it. Two ways forward, both the firmware's
  and neither ours: turn Secure Boot off, or put the firmware into setup
  mode and enrol `BOOTX64.EFI`'s hash as an allowed image. Where that is
  is the firmware's own menu — usually Security, then Secure Boot — and
  the wording differs by vendor; the machine's manual is the reference.
  Signing releases with a key the owner holds, so that a person can
  enrol one key instead of a hash per release, is not built.
- **The boot menu.** Most firmware offers one on F12, F9, Esc or
  Option-at-power; pick the USB disk. Changing the permanent boot order
  is not necessary and not advised.
- **Nothing from its disk.** The machine's own disk is not read, not
  written and not seen: this kernel has no driver for it. `ATA`,
  `BLK_DEV_NVME`, `NVME_CORE`, `MMC` and `SCSI_LOWLEVEL` are all in
  `boards/x86_64-uefi/kernel.forbidden`, so a SATA, an NVMe or an eMMC
  disk never becomes a block device, by label or otherwise. `SCSI` and `BLK_DEV_SD` are built, because a USB
  mass-storage device is a SCSI disk: the stick itself is `sda` and the
  only disk in the machine.

#### What works

The firmware's framebuffer at whatever mode the firmware chose; the
built-in keyboard and touchpad, whether they are behind the i8042
controller, on an i2c bus, or speaking PS/2 or RMI over the SMBus; USB
keyboards, mice and HID touchscreens, including ones plugged in after the
app is up, which the shell finds within a couple of seconds; a USB
webcam, as a V4L2 device the scanner opens; and the file channel, which
is the data partition on the stick itself plus every FAT partition on
every other USB disk, plugged in before or after boot, through the disk
process.

#### What does not

- **A MIPI CSI-2 camera.** Most laptop webcams made after about 2020 are
  not USB devices: they are sensors on a CSI-2 lane behind an image
  signal processor, and they need a per-platform driver and a userspace
  that configures a media pipeline. Neither is in this image. Such a
  machine shows the scanner's file path and no viewfinder; a plain USB
  webcam in a port works.
- **Legacy BIOS machines**, as above.
- **Intel Macs and Apple silicon.** An Intel Mac's firmware is UEFI but
  not a PC's, and Apple silicon is not x86 at all.
- **Suspend and hibernate.** Both are compiled out. The machine is on or
  it is off.
- **A webcam plugged in after boot.** It is not picked up; a keyboard, a
  mouse, a touchscreen and a USB disk are.
- **Any filesystem but FAT.** A partition holding ext4, exFAT, NTFS or
  anything else is left alone, whatever is on it: this kernel has no
  filesystem driver at all (the disk process reads FAT itself), and the
  machine's own disk is not a block device here at all.

#### When it does not work

Build the dev variant (`just dev=1 faraday-stick-image`) and boot that. It puts
the kernel console on the screen as well as on `ttyS0`, so the boot is
visible on a machine with no serial port, and the app keeps
`faraday-boot-report.txt` on the stick's data partition: written
as soon as the stick is listed at boot, written again whole every 30
seconds while the app runs, and once more when it ends. The file is the
kernel log (`dmesg`, which includes the app's own error output), then the
panel the shell chose and every input and capture device it found with what
it made of each, headed with how many seconds after the shell started it
was written. Each write goes to a new file that is read back and renamed
over the old one, so a stick pulled mid-write holds the last whole report.
Pull the stick, plug it into any computer and read that file. It is what to
send with a report of a machine that does not work. The file holds the
kernel log, that inventory and the system, radios and device nodes blocks (above, "The
dev variant") and nothing else: the core prints nothing, the shell prints
sizes, names, counts and reasons, and the blocks read only the kernel's
files (`tools/lint-secrets.sh`, rules 1 and 5). The release image writes nothing: its app is started without
`--boot-report`, its kernel log stays closed to the app, and `build.sh`
refuses a release image whose arguments or inittab say otherwise.

### 10.19 Try it in QEMU first

```
just faraday-stick-image  # or: just dev=1 faraday-stick-image
faraday/tools/stick-qemu.py --image out/stick/faraday-x86_64-uefi.img --vnc 9 --serial stdio
```

`stick-qemu.py` gives the machine no display of its own and puts it on VNC,
on `127.0.0.1:5909`. To watch it from another machine, forward the port
over SSH and point a VNC client at the forwarded port:

```
ssh -N -L 5909:127.0.0.1:5909 <the machine running QEMU>
```

Nothing is opened on the network to do that, and no port is exposed
beyond loopback on either end. With the dev image
(`out/stick/faraday-x86_64-uefi-dev.img`) the kernel console and the
shell's `--verbose` output are on the terminal.

`faraday/tools/qemu-check.py [--no-kvm] IMAGE OUT_DIR TESTKIT_DIR` boots the same
image headless and saves the screen as a PNG under `OUT_DIR` at each
stage: the first screen, with the boot stick in; after the boot stick is
pulled; with a test stick holding `TESTKIT_DIR`'s files plugged in; and
after that stick is pulled. It checks nothing on its own: the frames are
for a person to look at. `--no-kvm` emulates the CPU for a machine
without `/dev/kvm`, and every wait is four times as long.

A webcam cannot be emulated: QEMU has no UVC device, so the scanner in a
QEMU run finds no `/dev/video*` and offers the file path instead. To try
it, pass a real webcam through to the guest. Find its ids with `lsusb`
and add one device to the command `faraday/tools/stick-qemu.py` builds:

```
-device usb-host,vendorid=0x046d,productid=0x0825,bus=xhci.0
```

Under rootless QEMU the host user needs read and write on the webcam's
`/dev/bus/usb/BBB/DDD` node, and the host must not already have it open.
The UVC path in the image is the same `faraday-v4l2` crate the Linux
desktop shell uses, so a webcam that works there works here.

**This has not been tried**: nothing here confirms the passthrough.

### 10.20 Build it

```
just faraday-pi-image
```

It needs Docker (or `DOCKER=podman`). Rootless is what the recipe is written for:

```
export DOCKER_HOST=unix:///run/user/$(id -u)/docker.sock
```

Under rootless Docker, container root is your own user on the host, so
everything the build writes into `out/pi/` belongs to you. With a system
daemon the build works but leaves root-owned files behind.

`just faraday-pi-image` runs `just faraday-pi-bin` first, then
`run-build.sh`, which clones Buildroot at the pinned tag into
`out/pi/buildroot`, builds the container image from the `Dockerfile`,
and runs `build.sh` inside it with the shared checkout and caches and this
combination's own output directory bind-mounted.

Inside, `build.sh` merges the Buildroot fragments, configures the kernel
(`make linux-configure`), runs `check-kernel-config.sh` against the
generated `.config` — which fails the build, naming the line, if a
forbidden option is on or a required one is off — and only then builds.

The first run takes 30 to 60 minutes and downloads a few hundred megabytes
of sources. Later runs reuse the checkout, the download cache, the compiler
cache and the build tree, and only redo what changed. To start a
combination over, delete its directory under `out/pi/`.

### 10.21 Write it to a card or a stick

Any card or stick sold as 256 MB or more holds the image: the stick's is
193 MiB and 20 KiB, the Pi card's 196 MiB (F80). The smallest sold are far
larger than that, and the unused space is simply unused.

**`dd` overwrites the whole device.** Everything on the card or stick you
name is gone: its partition table, every partition, every file. There is
no undo and no confirmation. Find the device first, and read the name
twice.

```
lsblk                                   # which one is it? check the size
sudo dd if=out/pi/faraday-pi3-any.img of=/dev/sdX \
    bs=4M conv=fsync status=progress    # the Pi's card

sudo dd if=out/stick/faraday-x86_64-uefi.img of=/dev/sdX \
    bs=4M conv=fsync status=progress    # the stick
```

A GUI writer (Raspberry Pi Imager's "use custom image", balenaEtcher, GNOME
Disks' "restore disk image") does the same thing and is harder to point at
the wrong disk.

### 10.22 The serial console

On the stick the release image names no console at all, so the kernel
prints nowhere; the dev variant's is `ttyS0` at 115200, which is what
QEMU gives it and the header a laptop hides under its bottom cover.

On a Pi the kernel console is `serial0` at 115200 baud, 8N1, and in the
release image nothing else. It is never `tty0`, so no kernel message can
appear on the panel. `disable-bt` in `config.txt` gives `serial0` back to
the PL011 UART on the GPIO header:

| Pin | Signal |
|---|---|
| 6 | ground |
| 8 (GPIO 14) | board transmits |
| 10 (GPIO 15) | board receives |

Connect a 3.3 V USB serial adapter — ground to ground, its RX to pin 8, its
TX to pin 10 — and read it with `picocom -b 115200 /dev/ttyUSB0` or any
terminal program. `cmdline.txt` says `quiet`, so a healthy boot prints
little: the firmware banner, a few kernel lines, and then silence, because
the shell writes nothing to stdout or stderr unless it is given `--verbose`,
which the release image does not. The dev variant does, and puts a login
here.

### 10.23 First boot

The logo comes up a second or two after power and the Home screen replaces
it a few seconds later. What to check on a Pi, and what to do when it does
not:

- **Panel black, serial silent.** The firmware never started. Check the card
  and re-write it.
- **Panel black, serial shows the kernel booting.** The display
  configuration did not take. On a panel, the boot script's lines in the
  kernel log say which probe answered; `config.txt` is on the FAT partition
  and editable from any computer.
- **The logo appears and the app never does.** The shell started and could
  not draw. Build the same combination with `dev=1` in front of the recipe
  name and read what `--verbose` says.
- **Red and blue exchanged on the 2.8-inch.** The colour order is the pixel
  format in `panels/any/screens/waveshare-28dpi/display.line`
  (`docs/IMAGES.md` §3.3); it is worked out again on the panel, and a
  change is a change to that line. The framebuffer itself is red first
  (`docs/FRAMES.md` §5).
- **Panel right, touch dead.** Check `/run/faraday-screen/args`: `--touch-name`
  must be a substring of what the controller calls itself and
  `--touch-grid` its reported range. On the Waveshare panel the Goodix
  controller answers on the bit-banged i2c bus its touch overlay
  (`panels/any/screens/waveshare-28dpi/touch.dts`) declares; watch the serial console for its probe.
- **Touch lands in the wrong place.** `--touch-grid` is wrong for this
  controller. It is the range the controller reports, not the panel size;
  the two differ whenever the panel is mounted rotated.
- **Camera scanning offers a file instead of a viewfinder.** The shell found
  no `/dev/video0`. The legacy camera stack needs `start_x=1`, `gpu_mem=128`
  and `start_file=start_x.elf`, which are all in `config.txt`, and a
  supported module: OV5647 (Camera Module v1, the Zero camera) or IMX219
  (v2). The v3 module needs libcamera and is not supported.

On the stick, the same three things go wrong in their own ways:

- **Nothing happens and the firmware shows its own menu.** The firmware
  did not run `BOOTX64.EFI`. Secure Boot is the usual reason; the
  machine's boot menu is the other.
- **The screen stays black after the firmware's logo.** The framebuffer
  hand-over did not take. Build `just dev=1 faraday-stick-image` and read `ttyS0`
  — or run the dev image under `faraday/tools/stick-qemu.py`, where the
  console is the terminal.
- **The screen is right and nothing responds.** No input device was
  classified. `--verbose` in the dev variant lists every
  `/dev/input/event*` it found and what it made of each.

Exiting from the app powers the machine off: the app exits and init's next
line is `poweroff -f`, which goes through the Pi's watchdog driver or
through ACPI. In the release image there is no other way out: no console,
no login, nothing listening.

### 10.24 Deliberately absent

- **No shell login.** The inittab has no getty and BusyBox is built without
  `login`, `getty`, `su` and `passwd`. The root password is locked, and so is
  the `faraday` account's. The dev variant undoes all four for root,
  which is why it is not a signing image.
- **No root for the app.** Init is the only thing on the device that runs
  as root, and it runs three lines: `rcS`, the app as `faraday`, and
  `poweroff -f`.
- **No network stack at all.** `CONFIG_NET` is compiled out, so there is no
  socket layer, no IPv4 or IPv6, no netfilter, no wireless or Bluetooth
  stack and no network driver of any kind — the board's own Wi-Fi chip
  included. There is also no SSH, no dropbear, no interface configuration
  and no BusyBox networking applet, but those are now beside the point.
- **No SysRq**, so no key combination reaches the kernel.
- **No `/dev/vcio`, no `/dev/mem`, no debugfs, no MTD, no EEPROM drivers,
  no `/proc/kcore`, no kexec**, so the one non-volatile store on the board
  that cannot be wiped — the SoC's OTP — has no write path in this image.
- **No swap, no writable root**, no persistent state of any kind. Every boot
  starts from the same bytes. The only thing the running device writes is
  the file a user asked to save, onto the second partition; a dev card
  also writes its timings there.
- **No sound, no DRM** (but on the Pi, for its
  display), **no USB gadget, no infrared, no DVB, no IIO** — and
  no `ext4`, `btrfs`, `f2fs`, `xfs`, `NFS`, `CIFS`, `squashfs`,
  `overlayfs` or `FUSE`. The rootfs is an initramfs and nothing is ever
  mounted. **HID, mass
  storage and SCSI are built** on the stick, because a laptop's keyboard and
  touchpad are HIDs and it boots from a USB disk, and on the Pi, for USB
  keyboards, mice, touchscreens and sticks.
- **On the stick: no microcode blob, no Thunderbolt, no efivarfs, no
  `/dev/cpu/*/msr`, no paravirtualised guest support**, and both IOMMUs
  on from boot in strict mode, because a laptop's USB4 port is a PCIe
  slot on the outside of the case.
- **No tracing and no profiling.** `FTRACE`, `KPROBES`, `PROFILING` and
  `BPF_SYSCALL` are out, so nothing can watch the app run.
- **No shared libraries.** The C library is musl, linked statically, and
  the shell binary was static already. There is no `/lib/*.so` and no
  dynamic loader in the image.
- **No Python, no package manager, no debug tools.** `make-logo.py` runs in
  the build container and nothing it produces needs an interpreter.

