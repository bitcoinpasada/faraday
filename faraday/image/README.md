# The device images

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
| The machine | a Raspberry Pi with a 2.8" touch panel, or with an HDMI screen | any laptop or PC with UEFI firmware |
| The medium | a microSD card | a USB stick |
| What the firmware runs | `zImage`, named in `config.txt` | `EFI/BOOT/BOOTX64.EFI`, which is the kernel |
| The display | the panel the firmware sets up, 480×640 at 286 dpi; on the HDMI image, the screen vc4 drives at its EDID's mode | whatever the firmware left running, read from sysfs |
| Input | the Goodix touch controller; on a Compatible image also USB keyboards, mice and touchscreens | the built-in keyboard and touchpad, USB keyboards and mice, a HID touchscreen |
| Files | the card's `FARADAYDATA` partition, at `/mnt/microsd` | the stick's `FARADAYDATA` partition at `/mnt/microsd`, and every FAT partition on every other USB disk, at `/mnt/usb/<label>` |
| The image | `out/pi/faraday-pi3-waveshare-28dpi.img` (Strict), `-compatible.img`, `out/pi/faraday-pi3-hdmi-compatible.img` | `out/stick/faraday-x86_64-uefi-strict.img` |

Nothing here is shared with any other project.

## Boards, panels and variants

An image is one **board**, one **panel** and optionally the **dev**
variant. Each is a directory of fragments, and `build.sh` combines them:

```
common/     what every image has: the initramfs, BusyBox, the boot logo,
            the inittab, the account the app runs as
boards/     the architecture, the kernel, the firmware, the drivers, the
            layout of the medium the machine boots from
panels/     what the shell must be told about the display — which on a
            firmware framebuffer is almost nothing
variants/   dev only: a serial login and a talking console
external/   the Buildroot br2-external tree: the two packages
```

Each board and panel combination that is built has its entry in
`faraday/builds.toml`, the repository's list of builds; `just lint`
fails on a board or panel directory no entry builds.

The owner's Pi is `pi3` and `waveshare-28dpi`, which are the defaults:

```
just faraday-pi-image                                       # a test release
just dev=1 faraday-pi-image                                 # the same, with a login
just board=pi3 panel=waveshare-28dpi dev=0 faraday-pi-image # written out in full
just build=compatible faraday-pi-image                      # the same panel, Compatible build
just panel=hdmi build=compatible faraday-pi-image           # an HDMI screen, Compatible only
just faraday-stick-image                                    # the USB stick, Strict build
just build=compatible faraday-stick-image                   # the USB stick, Compatible build
just dev=1 faraday-stick-image                              # the same, with a login
```

The stick has two builds (`docs/IMAGES.md` §1): Strict, the default, has
`efi=disable_early_pci_dma` on its built-in command line, and Compatible
has not, for the PCs that do not start with it. They are directories under
`boards/x86_64-uefi/builds/`, each a part of the image after the panel,
with its own `kernel.cmdline` checked against the built kernel's
`CONFIG_CMDLINE`. The root filesystem and the app are the same; each build has
its own build tree, `out/pi/x86_64-uefi-efi-framebuffer-<build>[-dev]/output`,
and its own image, `out/stick/faraday-x86_64-uefi-<build>[-dev].img`.

The Pi has two builds too, under `boards/pi3/builds/` (IMAGES §1, §2).
Strict is the kernel the Pi has always had: its directory holds only a
`kernel.forbidden`, which keeps DRM, sound, HID, USB storage and SCSI
out, so a Strict card proves it carries none of them, and its image keeps
the name it had, `out/pi/faraday-pi3-<panel>.img`. Compatible adds, in
its `linux.fragment`, vc4's display side through DRM with its
framebuffer emulation, the sound core vc4 cannot build without (and no
sound driver), the generic USB HID drivers, and USB storage and SCSI;
its `kernel.required` and `kernel.forbidden` hold each of those to the
PC's rules. Both builds compile the board's kernel source as it is, with
no patch of Faraday's own (F153): vc4 binds the Pi 3's V3D and creates a
render node, which `common/rootfs-overlay/etc/mdev.conf` gives to root
alone with `/dev/dri/card0`, and DRM selects `KCMP`, which Compatible
allows while the Strict Pi's and the PC's lists forbid it. Its image is
`out/pi/faraday-pi3-<panel>-compatible.img`. A panel that needs it says
`BUILDS="compatible"` in its `panel.conf`, and build.sh refuses it in
another build.

Each combination has its own build tree at
`out/pi/<board>-<panel>[-<build>][-dev]/output` and its own image. The Buildroot
checkout, the download cache and the compiler cache are shared, so the
second combination costs what differs and not the toolchain — and the
stick shares all three with the Pi, even though it is a different
architecture.

### What each part holds

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

`build.sh` walks common, board, panel, the build
(`boards/<board>/builds/<build>`), dev, `boards/<board>/dev` in that
order. `config.txt` is
the parts concatenated. `cmdline.txt` is the parts' words joined, with
comments stripped and a word written `-quiet` removing the `quiet` an
earlier part contributed. The Buildroot fragments are merged with
Buildroot's own `merge_config.sh`, so a later part's value replaces an
earlier one's rather than sitting next to it. The kernel fragments, the
BusyBox fragments, the root filesystem overlays and the users tables
become space-separated lists in a generated fragment; nothing else in
the tree holds a path.

### The boards

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

### Adding a board

A directory under `boards/` with three fragments and a `board.conf`:

- `board.conf` — `NAME`, `STATUS` (`tested` or `untested`), the toolchain
  `just faraday-pi-bin` builds the shell with (`RUST_TARGET`, `CC`, `CFLAGS`), and
  how the machine boots:

  | Key | Meaning |
  |---|---|
  | `FIRMWARE` | `rpi` if the firmware reads `config.txt` and `cmdline.txt` off the boot partition; `none` otherwise |
  | `CMDLINE` | `file` if the firmware passes the kernel its command line; `kernel` if the line is compiled in (`CONFIG_CMDLINE_OVERRIDE`) |
  | `DTBS` | the device trees that go on the boot partition and into `BR2_LINUX_KERNEL_INTREE_DTS_NAME`; absent on a machine described by ACPI |
  | `EXCHANGE` | the partition `rcS` mounts at `/mnt/microsd`: a node, or `LABEL=FARADAYDATA` for a medium whose node name is not fixed |
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
  reason. The Pi's Strict build forbids `HID`, `SCSI` and
  `USB_STORAGE`; the stick and the Pi's Compatible build require all
  three. That is what these files are per board, and per build, for.
- `genimage.cfg` — the layout of the medium, with `@DTBS@` where the
  device trees go.
- `post-image.sh` — optional, and run before the one that calls
  `genimage`: whatever the layout expects to find in `images/`.
- `config.txt` — only on a board whose `FIRMWARE` is `rpi`.
- `dev/` — optional: the parts that are this board's *and* the dev
  variant's, which today is the console the probe lines go to.

A board whose toolchain is not already installed is that much more work:
`rustup target add` its Rust target and a cross-compiler from the
distribution.

### Adding a panel

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

  `BUILDS` names the builds the panel works in, where that is not all
  of them: `panels/hdmi` needs the Compatible kernel.

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

Nothing in the shell binary is panel-specific, so a new panel needs no
Rust. Bring it up with `just dev=1 faraday-pi-image`, read the probe lines on the panel
itself,
and only then build the release image.

### The dev variant

`just dev=1 faraday-pi-image` is for bringing up a new board or a new panel
without a serial
adapter: the kernel console goes to `tty1` as well as the serial port and
`quiet` is dropped, so the probe lines scroll on the panel; a getty runs on
the serial port and the root password is empty; and the app is started with
`--verbose --timings --boot-report`, so it says what it found and records
what its loop and its costly work cost and what the board reports.

`--timings` writes `faraday-timings.txt` onto the `FARADAYDATA` partition,
through the disk process since the image mounts nothing: the whole file
every 30 seconds and once at exit, which a pulled stick may miss, so wait
half a minute after the last thing to measure. It holds one block for
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
the step was chosen from, a `clock` line gives the first CPU's clock at
the passes that drew (the header names its cpufreq driver and governor),
a `steps` line lists each fade step that followed another as the gap
since it, what its pass cost and the step chosen after it, in ms, and a
`fade` line says how many changes faded,
were drawn at once because the last fades' steps came too far apart
(`skipped`) or because reduce motion is on (`reduced`), how many fade
steps were blended, how many recent step gaps are known and their median,
and whether the next change would fade (`docs/MOTION.md` §3.5). Then a
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
memory free and the app's peak, each now and at its worst) and a radios
block (`/sys/class/bluetooth` and `/sys/class/net`, the device tree's
Bluetooth and Wi-Fi nodes with their status and their parent's, the
serial ports and what is bound to them).

To collect a run: boot the card, scroll the Settings list for ten seconds,
open the scanner and point the camera at anything that is not a QR code for ten seconds (a code is decoded at once and the scanner stops),
open Learn and scroll a page, then go to Settings → Wipe and exit. The
device powers itself off after the last block is written. Put the card in
a computer and read `faraday-timings.txt` from the `FARADAYDATA`
partition; it is plain text, one number per column, so it reads on a
phone. **It never goes on a card that signs
anything.** It exists because the release image has no way in at all, which
is the point of it and useless when the question is why the panel is blank.

## What is in it

| Piece | Choice |
|---|---|
| Buildroot | 2024.02 LTS, pinned to a release tag in the `justfile` |
| Kernel | The board's, **with its defconfig's `=m` lines dropped**, plus the `linux.fragment`s: for `pi3`, the Raspberry Pi 6.1 tree at the commit Buildroot 2024.02 pins and its `bcm2709` configuration; for `x86_64-uefi`, mainline 6.18.56 (6.18 longterm) from kernel.org with five commits from 6.19 that put the screen's EDID in `boot_params` (`linux-patches/`, `docs/DECISIONS.md` F23) and `x86_64_defconfig`. Each board's `buildroot.fragment` carries the tarball's sha256 and `build.sh` checks it |
| C library | musl, static only. BusyBox is the one package Buildroot builds for the target, so there is no shared library in the image at all |
| Firmware | On a Pi, `rpi-firmware`, the `PI_X` variant, so `start_x.elf` and `fixup_x.dat` are on the card. On the stick, none: the machine's own firmware runs the kernel, and no blob of any kind is in the image |
| Root filesystem | An initramfs (cpio) linked into the kernel image. BusyBox `init`, one shell, `mdev`, `setpriv`, `setuidgid`, `poweroff`, `findfs` on the stick, and `/usr/bin/faraday`. No `/lib/*.so` |
| The binary | Built on the host by `just faraday-pi-bin` with the board's toolchain, as a static executable, and installed by `package/faraday`. Buildroot never builds Rust |
| Overlays | For `waveshare-28dpi`, Waveshare's own `28DPI-DTBO` pack, downloaded at a pinned sha256 |

### The kernel is the defconfig with its modules dropped

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
evdev, V4L2, the initramfs, devtmpfs, FAT and the serial console.

`CONFIG_NET` is out, so the image has no socket layer, no IPv4 or IPv6, no
netfilter, no wireless or Bluetooth stack, and no Ethernet, Wi-Fi or USB
network driver. So are sound, DRM, USB gadget, HID, mass storage, SCSI,
infrared, DVB, IIO, MD, six filesystems, and the tracing and profiling
machinery; the Pi's Compatible build alone brings back DRM for the
display, the sound core with no driver, HID and mass storage, each in
its build directory's lists. `check-kernel-config.sh` is the list, with a reason per line.

### The kernel configuration is checked on every build

`check-kernel-config.sh` reads two lists out of each part directory:
`kernel.forbidden`, options that must not be built, and
`kernel.required`, options that must be, each line a symbol and the reason
it is in the list. `build.sh` runs it after `make linux-configure` has
generated the kernel's `.config` and before the kernel is built, so a
forbidden option fails the build in seconds and the offending line is
what it prints. A driver that quietly lost a dependency — because the
subsystem above it went out — fails the same way, rather than turning up
as a black panel on the owner's desk.

The lists are per part because the devices differ. `common/` holds what no
Faraday image builds and what every one needs; a board holds what that
machine does not have and does not work without, and a build
(`boards/<board>/builds/<build>`) what that build adds or keeps out.
`HID`, `SCSI` and `USB_STORAGE` are in the Pi's Strict forbidden list and
in the stick's and the Pi Compatible's required lists, which is the whole reason these are files in board directories
rather than one list with exceptions in it. A symbol in both lists at
once fails the build too: two rules that cannot both hold would otherwise
be settled by whichever ran last.

### The shell binary is reinstalled on every build

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

### The boot logo

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

### The app is not root

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
| `/mnt/microsd` | mounted `uid=200,gid=200,umask=077` | — | the file channel; init still mounts it as root |
| `/mnt/usb/*` | mounted `uid=200,gid=200,umask=077` | — | a FAT partition on a plugged-in USB disk, mounted the same way and by the same init |
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

### What the kernel does not build

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

### USB is kept for a webcam

On the Pi, the board's host controller and `USB_VIDEO_CLASS` are built on
purpose, so a plain USB webcam appears as a `/dev/video*` and the shell —
which takes the first one — can scan with it. That is the whole reason
USB is on there. Gadget mode, HID, mass storage and SCSI are out, and the
USB network adapters went with `CONFIG_NET`, so the port carries a camera
or nothing.

That is the Pi's Strict build. Its Compatible build also carries USB
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

### Writable non-volatile memory on a Pi

| Store | Size | How written | Wipeable | Reachable by the app |
|---|---|---|---|---|
| microSD boot partition | 64 MiB | raw `/dev/mmcblk0p1` | yes, by rewriting the card | no: never mounted, node is root's |
| microSD exchange partition | 128 MiB | files at `/mnt/microsd` | yes | yes, and this is the only store it writes |
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

### SysRq is off

`# CONFIG_MAGIC_SYSRQ is not set`. Nothing should be able to reboot the
device, kill the app or dump kernel state from an input device. The device
is meant to be unreachable except through the screen, and the magic keys
are a way around that.

## Partition layout

### The Pi's card

Two FAT partitions, both `0x0C`:

1. **boot**, 64 MiB, FAT16, holding about 16 MB. What the Pi firmware reads: `bootcode.bin`,
   `start_x.elf`, `fixup_x.dat`, `config.txt`, `cmdline.txt`, `zImage`,
   `overlays/`, and the board's device trees — `bcm2710-rpi-3-b.dtb` and
   `bcm2710-rpi-3-b-plus.dtb` for `pi3`, from its `board.conf`.
   `boards/pi3/genimage.cfg` is the template; `build.sh` fills the device
   trees in.
2. **exchange**, 128 MiB, FAT32, labelled `FARADAYDATA`, holding one `README.txt` that
   says what it is for. Init mounts it at `/mnt/microsd`
   (`flush,noexec,nosuid,nodev,uid=200,gid=200,umask=077`), and it is the
   file channel: a `.psbt` put
   here from a computer is what *Read a file* offers, and *Save to file*
   writes back beside it. The mount is allowed to fail, so a card someone
   repartitioned still boots into the app.

There is no root filesystem partition, and nothing but this partition is
read or written after boot. The boot partition is never mounted.

### The stick

GPT, because that is what a UEFI firmware looks for on a removable disk,
and because the EFI system partition is a GPT partition type. Two
partitions, 1 MB-aligned, with fixed UUIDs so that two builders get the
same bytes. The UUIDs spell "Faraday-Stick" in ASCII hex and end in a
number: the disk `46617261-6461-792D-5374-69636B000000`, the boot
partition `…000001` and the data partition `…000002`. The partitions'
GPT names are their FAT labels, `FARADAYBOOT` and `FARADAYDATA`:

1. **FARADAYBOOT**, 64 MiB, the EFI system partition, FAT32, labelled `FARADAYBOOT`,
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
2. **FARADAYDATA**, 128 MiB, FAT32, labelled `FARADAYDATA`, the same partition and
   the same note as the Pi's. `rcS` finds it with `findfs LABEL=FARADAYDATA`, because
   the stick is `sda` on one machine and `sdb` on the next.

The image is 193 MiB and 20 KiB: 1 MiB before the first partition, the
two partitions, and the backup GPT. A stick or card made before F80 has
a 48 MB (stick) or 32 MB (card) boot partition and a 32 MB FAT16 data
partition. Faraday reads and writes that data partition as any other;
the Starter's keep data refuses to write a
64 MiB boot partition onto it, so its files are moved to a new stick by
hand.

No partition labelled `FARADAYBOOT` is handed to the app on any
medium: that is a Faraday stick's EFI system partition, which holds the
kernel and is the firmware's. `faraday-grant` keeps it root's and
`faraday-disk` refuses it by its FAT label (`docs/history/PLAN.md` §4.3).

## The USB stick on a laptop

### What it needs from the machine

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
  disk never becomes a block device and there is nothing to mount, by
  label or otherwise. `SCSI` and `BLK_DEV_SD` are built, because a USB
  mass-storage device is a SCSI disk: the stick itself is `sda` and the
  only disk in the machine.

### What works

The firmware's framebuffer at whatever mode the firmware chose; the
built-in keyboard and touchpad, whether they are behind the i8042
controller, on an i2c bus, or speaking PS/2 or RMI over the SMBus; USB
keyboards, mice and HID touchscreens, including ones plugged in after the
app is up, which the shell finds within a couple of seconds; a USB
webcam, as a V4L2 device the scanner opens; and the file channel, which
is the `FARADAYDATA` partition on the stick itself at `/mnt/microsd` plus
every FAT partition on every other USB disk, plugged in before or after
boot, at `/mnt/usb/<label>`.

### What does not

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
  anything else is left alone, whatever is on it: `vfat` is the only
  filesystem in this kernel, and the machine's own disk is not a block
  device here at all.

### When it does not work

Build the dev variant (`just dev=1 faraday-stick-image`) and boot that. It puts
the kernel console on the screen as well as on `ttyS0`, so the boot is
visible on a machine with no serial port, and the app keeps
`faraday-boot-report.txt` on the stick's `FARADAYDATA` partition: written
as soon as the stick is listed at boot, written again whole every 30
seconds while the app runs, and once more when it ends. The file is the
kernel log (`dmesg`, which includes the app's own error output), then the
panel the shell chose and every input and capture device it found with what
it made of each, headed with how many seconds after the shell started it
was written. Each write goes to a new file that is read back and renamed
over the old one, so a stick pulled mid-write holds the last whole report.
Pull the stick, plug it into any computer and read that file. It is what to
send with a report of a machine that does not work. The file holds the
kernel log, that inventory and the system and radios blocks (above, "The
dev variant") and nothing else: the core prints nothing, the shell prints
sizes, names, counts and reasons, and the blocks read only the kernel's
files (`tools/lint-secrets.sh`, rules 1 and 5). The release image writes nothing: its app is started without
`--boot-report`, its kernel log stays closed to the app, and `build.sh`
refuses a release image whose arguments or inittab say otherwise.

### Try it in QEMU first

```
just faraday-stick-image  # or: just dev=1 faraday-stick-image, or build=compatible
faraday/tools/stick-qemu.py --image out/stick/faraday-x86_64-uefi-strict.img --vnc 9 --serial stdio
```

`stick-qemu.py` gives the machine no display of its own and puts it on VNC,
on `127.0.0.1:5909`. To watch it from another machine, forward the port
over SSH and point a VNC client at the forwarded port:

```
ssh -N -L 5909:127.0.0.1:5909 <the machine running QEMU>
```

Nothing is opened on the network to do that, and no port is exposed
beyond loopback on either end. With the dev image
(`out/stick/faraday-x86_64-uefi-strict-dev.img`) the kernel console and the
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

## Build it

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

## Write it to a card or a stick

Any card or stick sold as 256 MB or more holds the image: the stick's is
193 MiB and 20 KiB, the Pi card's 196 MiB (F80). The smallest sold are far
larger than that, and the unused space is simply unused.

**`dd` overwrites the whole device.** Everything on the card or stick you
name is gone: its partition table, every partition, every file. There is
no undo and no confirmation. Find the device first, and read the name
twice.

```
lsblk                                   # which one is it? check the size
sudo dd if=out/pi/faraday-pi3-waveshare-28dpi.img of=/dev/sdX \
    bs=4M conv=fsync status=progress    # the Pi's card

sudo dd if=out/stick/faraday-x86_64-uefi-strict.img of=/dev/sdX \
    bs=4M conv=fsync status=progress    # the stick
```

A GUI writer (Raspberry Pi Imager's "use custom image", balenaEtcher, GNOME
Disks' "restore disk image") does the same thing and is harder to point at
the wrong disk.

## The serial console

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

## First boot

The logo comes up a second or two after power and the Home screen replaces
it a few seconds later. What to check on a Pi, and what to do when it does
not:

- **Panel black, serial silent.** The firmware never started. Check the card
  and re-write it.
- **Panel black, serial shows the kernel booting.** The display
  configuration did not take. `config.txt` is on the FAT partition and
  editable from any computer.
- **The logo appears and the app never does.** The shell started and could
  not draw. Build the same combination with `dev=1` in front of the recipe
  name and read what `--verbose` says.
- **Red and blue exchanged.** Change `dpi_output_format=0x7F206` to
  `0x7F216` in `config.txt`. The two values differ only in the colour order
  the firmware hands the framebuffer; this image ships the one that matches
  what the shell writes.
- **Panel right, touch dead.** Check `/etc/faraday/args`: `--touch-name`
  must be a substring of what the controller calls itself and
  `--touch-grid` its reported range. On the Waveshare panel the Goodix
  controller answers on the bit-banged i2c bus the `waveshare-touch-28dpi`
  overlay declares; watch the serial console for its probe.
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

## Deliberately absent

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
- **No sound, no DRM** (but on the Pi's Compatible build, for its
  display), **no USB gadget, no infrared, no DVB, no IIO** — and
  no `ext4`, `btrfs`, `f2fs`, `xfs`, `NFS`, `CIFS`, `squashfs`,
  `overlayfs` or `FUSE`. The rootfs is an initramfs and the exchange
  partition is FAT; nothing else is ever mounted. **No HID, no mass
  storage and no SCSI on the Pi's Strict build**, whose touch controller is its whole
  input device and whose only storage is the card the firmware booted
  from; the stick needs all three, because a laptop's keyboard and
  touchpad are HIDs and it boots from a USB disk.
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
