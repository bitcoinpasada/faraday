# opensigner-pi

OpenSigner on a Raspberry Pi 3B+ with a Waveshare 2.8" DPI capacitive
touchscreen, running a minimal Linux from an initramfs with no console,
no SSH and no IP stack.

The shell is one static binary. It draws to `/dev/fb0`, reads
`/dev/input/eventN` and `/dev/video0`, and reads and writes files in
`/mnt/microsd`, the card's second partition; that is the whole of its
contact with the world. Apart from the standard library it uses
`osk-shell-api`, `opensigner-core` and `opensigner-v4l2` and nothing
else — no `mmap` of its own (the standard library has none;
`write_all_at` is one `pwrite` per frame), no `libc`, no `unsafe`, no
logging.

The camera is the one exception, and it is the crate's rather than this
one's. V4L2 capture is `ioctl`, `mmap` and `poll`, none of which the
standard library has, and `docs/PLANNING.md` §16.33 decided against a
camera library on every platform: a library is acceptable only if it is
minimal, vendored and reviewable in full, and no `v4l` crate is. So the
capture lives in `opensigner-v4l2`, a Linux-only crate the desktop shell
shares, which is the single exception to the workspace's
`unsafe_code = "forbid"` — its `Cargo.toml` says so, every `unsafe`
block carries a `// SAFETY:` comment, and its public API is entirely
safe: no raw pointer, file descriptor or mapped buffer crosses it. This
shell still has no `unsafe` of its own.

**Written, never run on the panel.** Everything below has been checked
against files on a build box.

## It does not run as root

On the device the binary is started by init as the `opensigner` account
(uid and gid 200), through `setpriv --nnp` and `setuidgid`; the image's
README has the inittab line and the account. What that account can open
is the panel, the touch controller, the capture device, `/dev/random`
and the files under `/mnt/microsd`, which init mounts `uid=200,gid=200`.
The card's raw block devices, the boot partition, `/dev/i2c-*` and
`/dev/mem` — which is not built at all — are not reachable from it.

Nothing the shell does needs more: `/sys/class/graphics/fb0` and
`/sys/class/input/*/name` are world-readable, `/proc/mounts` is
readable, and the one thing that did need root, `/sbin/poweroff` after
`Command::Exit`, is now init's line rather than the app's.

## What it does

- **Display.** Reads `virtual_size`, `bits_per_pixel` and `stride` from
  `/sys/class/graphics/fb0` at start-up and creates the core at that size,
  at the density `--dpi` or the kernel command line's `opensigner.dpi=N`
  gives (160 when neither does), Tier A. Every `Command::Draw` converts the core's premultiplied
  RGBA8888 frame and writes it with `write_all_at`: one call when the rows
  are packed, one per row when the kernel pads them. Both depths this
  panel has been seen in are supported and chosen at runtime — RGB565
  little-endian and BGRA8888 — matching what this panel's framebuffer reports on the DRM and the
  legacy firmware paths.
- **Input.** Every `/dev/input/event*` is opened at start and read on a
  thread of its own, which sends what it decodes over the loop's wake
  channel — one channel, carrying whatever woke the loop: a contact, a
  cursor motion, a key, or a note that a camera frame is ready. What each
  device is comes from its sysfs capability bitmaps
  (`capabilities/{ev,key,rel,abs}` and `properties`), read as text, so
  there is still no ioctl here: a keyboard has letter keys, a mouse has
  `REL_X`/`REL_Y`, a touchpad has absolute axes that move a pointer, a
  touchscreen names places on the screen. One device may be several.
  `--touch-name` still wins for touch, which is how the Pi's panel is
  found.
- **Mouse, touchpad and keyboard** (`docs/PLANNING.md` §16.94). A
  laptop booted from the stick points and types. The cursor is the
  shell's: `BTN_LEFT` down, the moves while it is down, and `BTN_LEFT` up
  are `Event::Touch` Down, Move and Up at the cursor, and the wheel is
  `Event::Scroll` there, so the core needs no pointer and no screen is
  designed twice. A touchpad moves the cursor by its finger's travel; a
  pad with no button clicks when a finger lands. On a pad that has a
  button, a touch that lifts within 200 ms having carried the cursor no
  more than 6 pixels is a tap, which is a click where the cursor is. While
  `BTN_TOOL_DOUBLETAP` says two fingers are down, the finger's vertical
  travel scrolls at the cursor, moving the content with the fingers as a
  touchscreen does, and the cursor does not move. A device
  whose `abs` bitmap has `ABS_X` is followed by `ABS_X`/`ABS_Y`, the
  kernel's single-touch emulation, and its `ABS_MT_*` events are ignored;
  a device without `ABS_X` is followed by the `ABS_MT_POSITION_*` of slot
  0. Keys are a US layout in
  one table, with Shift for case and the shifted punctuation, and the
  kernel's auto-repeat repeats Backspace, Tab and the arrows and nothing
  else. The keyboard drives every screen and not only the ones that take
  text (`docs/DESIGN.md` §4.15), so a key coming up is reported as
  `Event::KeyUp` beside the key going down: that is what holds a hold
  under Enter. The arrow is
  drawn by the shell into its own converted copy of the frame, never into
  the core's, and only on a machine that has a pointing device: the Pi's
  frames are the bytes they always were.
- **Touch.** The device is found by name — `--touch-name`, "Goodix" by
  default — falling back to whatever the bitmaps say is a touchscreen. A controller reports in a grid of its own, given by
  `--touch-grid` and 640×480 by default, and a position is scaled from it
  to the panel and clamped, as the fork does: on the owner's panel that is
  `x·480/640`, `y·640/480`. A tracking id of zero or more opens or
  continues a contact, −1 (or `BTN_TOUCH` going to zero) ends it, and the
  touch is delivered at `EV_SYN`. `struct input_event` is 16 bytes on the
  device's 32-bit ARM kernel and 24 on a 64-bit build box; both are
  decoded.

  The main loop takes every waiting touch each pass, keeps only the last
  of a run of moves, and paints once at the end. Taking one touch per
  50 ms pass and painting the panel for each is what made a flick keep
  scrolling for seconds after the finger lifted, with later taps replaying
  behind it: the controller reports a moving finger a hundred times a
  second and the panel cannot be redrawn that often.
- **Camera.** `Command::CameraOn` opens the first `/dev/video*` that
  reports video capture and streaming and takes one of `GREY`, `YUYV`,
  `NV12` or `YU12` (`--camera PATH` names one instead), asks it for
  640 × 480, and streams 8-bit luma frames from a capture thread through
  a two-deep channel. Opening the device is part of what that thread
  does: finding a node, negotiating a format and mapping the buffers
  takes half a second here, and the loop that would have waited for it
  is the loop that draws, so `CameraOn` starts the thread and returns,
  and a device that will not open comes back as one
  `Event::CameraUnavailable`. The capture thread posts a note on the
  wake channel for each frame it queues, dropping the note when that
  channel is full, so a frame ends the loop's wait as a touch does and
  is drawn as soon as it arrives rather than up to a tick later. The
  frame itself stays on the camera's own two-deep channel, which drops
  a frame rather than queueing a stale one. Each pass takes one frame,
  the newest, and leaves the rest: the frames behind it picture where
  the camera was, not where it is. Sends are capped at
  fifteen a second: a 3B+ has better things to do than look at thirty
  frames a second, and a frame costs the loop a quarter of what it did
  now that the capture thread reduces it.

  Each frame goes twice: reduced to 320 × 240 for the core, which draws
  it in the viewfinder, and full-size to `opensigner-scanner`, whose
  worker thread reads the codes in it. The capture thread does that
  reduce, so the core reduces nothing and the loop moves a quarter of
  the bytes it used to. Each pass hands the core whatever the worker found, as
  `Event::Scanned`. A decode costs 235 ms of a pass here and 445 at
  worst (§16.36), which is why it is not the core's: with it there the
  Scan screen painted and answered at the decode rate. A mode wider
  than 800 pixels — the OV5647's 1296 × 972, say — is shrunk by an
  integer factor first. No device, or an open that fails, is one
  `Event::CameraUnavailable` and the scanner offers a file instead.
  On the device this needs the legacy camera stack: `start_x=1`,
  `gpu_mem=128`, the firmware's `start_x.elf` and the kernel's `bcm2835`
  driver (§16.33).
- **Clock.** The main loop waits 50 ms to be woken — 16 ms while a finger
  is down — and sends `Event::Tick` with milliseconds from a monotonic
  epoch either way. A wake ends the wait early; a pass with nothing to do
  costs nothing.
- **Files.** The boot medium's second partition, labelled `OSKDATA`, is
  mounted at `/mnt/microsd` by init, and on the stick every FAT partition
  on every other USB disk is mounted at `/mnt/usb/<label>` by init as the
  disk arrives (`--files DIR` points the channel at one directory on a
  build box). A `Command::RequestFile` is answered with every file in
  every one of those places that could be what was asked for — a name ending in `.psbt` for
  `FileKind::Psbt`, anything but the partition's own `README.txt` and a
  dot-file for `FileKind::Any` — newest first, up to 64 of them, as one
  `Event::FileList` carrying each name, its size and its date. A file's
  name is `OSKDATA/signed.psbt` when more than one place is mounted and
  `signed.psbt` when one is, and `Event::FileList::place` names the
  places it looked, joined with commas. The list is built when the
  request arrives, so a stick plugged in while the app was up is on it.
  The core draws the list and asks for one of them by name with
  `Command::ReadFile`, which is read whole, up to 4 MiB; a name that was
  not listed is never read and never a path this shell follows. A
  directory that is not a mount point, or a file too large, is one
  `Event::FileUnavailable`, with the reason only where `--verbose` can
  see it; the card is the picker, so there is nothing here for a person
  to close and this shell never sends `Event::FileCancelled`. A file dated before 2020 is a file the board could not date —
  no real-time clock means the clock starts at the epoch — so its row
  carries its size instead. `Command::WriteFile` saves the bytes under
  the core's name hint reduced to what FAT holds, onto `/mnt/microsd`
  when that is mounted and onto the first `/mnt/usb` partition otherwise,
  never over a file already there: `signed.psbt`, then `signed-2.psbt`,
  then `signed-3.psbt`. The settings and the timings stay on
  `/mnt/microsd` and go nowhere else. The write is flushed with `sync_all`, and the
  partition is mounted `flush`, so the file is on the card the moment
  the core's screen says it is saved. The shell reads no format and
  draws nothing: the core draws every screen, and this shell only says
  what is on the card.
- **Entropy.** `Command::RequestEntropy` is answered with 32 bytes from
  `/dev/random`, which since Linux 5.6 blocks only until the kernel's
  CSPRNG is initialised and never afterwards; the board seeds it from the
  SoC's hardware RNG. A read that fails is not answered at all, which the
  contract allows: the core then runs a weak session and says so.
- **Exit.** `Command::Exit` draws the last frame, waits two seconds so it
  can be read, and exits. The board is powered off by init, whose next
  inittab line is `poweroff -f`: the app runs as an unprivileged user and
  could not do it itself.
- **Silence.** Nothing is written to stdout or stderr, usage errors
  included, unless `--verbose` is given. The device pins its output
  channels and this shell has nothing to say into them.

## Timings on a dev card

The release card gives nothing back: no console, no network, no login, and
the owner has no serial adapter. `--timings` is the way a number gets off
it. The image adds the flag to `/etc/opensigner/args` only when `DEV=1`, so
a release card never records anything, and the clock is read only when the
flag is on.

With it on, the loop times every call it already makes into six buckets:

| Bucket | What is inside it |
|---|---|
| `touch` | one `Event::Touch` through the core, including the core's layout and paint when the touch redraws |
| `tick` | one `Event::Tick` |
| `camera` | one `Event::CameraFrame` through the core: the preview conversion, on every frame |
| `scanned` | one `Event::Scanned` through the core: routing a code the scanner's worker read |
| `convert` | the core's frame packed into the panel's pixel format |
| `write` | the `pwrite` of that frame to the framebuffer |

Every five seconds of the shell's clock, and once more at exit before the
machine powers off, one block is appended to `opensigner-timings.txt` in
the files directory and flushed: the seconds since start, the passes and
the touches in the interval, then a line per bucket with its count, its
mean in milliseconds and its worst in milliseconds. The first block of a
run is preceded by a line naming the binary version and the panel size. A
later run adds to the file rather than replacing it, and *Read a file*
never offers it, as it never offers `opensigner-settings.txt`.

```
opensigner-pi 0.1.0 panel 480x640 columns: bucket count mean_ms worst_ms
     5s passes     112 touches      31
touch         31      6.10     18.30
tick         112      0.20      1.10
camera         0      0.00      0.00
convert      143      1.50      2.30
write        143      4.00     12.00
```

To read it, power the device off from Settings, put the card in a
computer, and open `opensigner-timings.txt` on the `OSKDATA` partition.
There are no per-pass rows: the card is slow to write and the file has to
stay small.

## What it does not do yet

- **No word from a failed save.** The contract has no event for a write
  that did not happen, so a card that is full or absent leaves the core's
  "saved as" row saying a file was written. The reason reaches `--verbose`
  and nothing else.
- **No physical buttons.** `DisplayInfo::buttons` is zero; every flow is
  reachable by touch.
- **The console is not put into graphics mode.** The fork's driver issues
  a `KDSETMODE` ioctl on `/dev/tty0` so kernel text cannot appear over the
  UI. This crate has no `unsafe` and so no ioctl of its own; the camera's
  live in `opensigner-v4l2`, which is scoped to V4L2 and is not the place
  for a console ioctl. Boot messages are painted over by the first frame,
  but a kernel message printed later would sit on top of the UI until the
  next draw.
- **No netlink.** `/dev/input` is listed again every two seconds, so a
  keyboard or a mouse plugged in after OpenSigner is up is found within
  that, and one unplugged is dropped when its reader ends. That is a
  `read_dir` and the sysfs text the classification already reads: no
  netlink socket, no hotplug helper in the image, no ioctl. Anything that
  is not an input device still has to be in before the machine boots.
- **No `EVIOCGABS`.** Same bar: the fork asks the driver for its real axis
  ranges; this shell is told them instead, by `--touch-grid`, which the
  image writes from the panel directory's `panel.conf`. A different panel
  is a different flag, not different code.

## Flags

`--size`, `--dpi`, `--touch-name` and `--touch-grid` describe the panel:
the image writes them into `/etc/opensigner/args` from the panel
directory's `panel.conf` and the inittab passes them on, so one binary
serves every panel. The rest exist so the shell can be exercised on a
build box.

| Flag | Meaning |
|---|---|
| `--fb PATH` | write frames here instead of `/dev/fb0`; also turns off the sysfs probe |
| `--input [KIND:]PATH` | read a file or FIFO of recorded packets instead of searching for devices; `KIND` is `touch` (the default), `mouse`, `pad` or `keys`, and the flag may be given once per device |
| `--camera PATH` | capture from this device instead of searching `/dev/video*` |
| `--no-camera` | answer every camera request "unavailable", so the scanner offers a file |
| `--files DIR` | read and write files here instead of `/mnt/microsd`; also drops the check that the directory is a mount point |
| `--size WxH` | panel size, overriding sysfs; without it and without a readable sysfs, 480x640 |
| `--depth 16\|32` | RGB565 or BGRA8888, overriding sysfs (default 16) |
| `--dpi N` | pixel density reported to the core; without it, `opensigner.dpi=N` on the kernel command line, and failing that 160 |
| `--pointer-speed F` | device pixels the cursor moves per unit of mouse or touchpad motion (default 1) |
| `--touch-name NAME` | the input device name to look for; any substring of what the controller calls itself (default `Goodix`). No match falls back to the first multitouch device |
| `--touch-grid WxH` | the range the touch controller reports in, scaled to the panel (default 640x480) |
| `--frames N` | exit after N frames have been drawn |
| `--verbose` | allow diagnostics on stderr |
| `--timings` | record what each part of a loop pass costs into `opensigner-timings.txt` on the card |

```
# One frame of the Home screen into a file, as raw RGB565.
cargo run -p opensigner-pi -- --fb /tmp/panel.fb --input /dev/null --no-camera \
    --size 480x640 --depth 16 --frames 1
```

`tests/framebuffer.rs` does exactly that and compares the result with the
frame the core renders in-process, pixel for pixel, at both depths.

## Building for the device

`just pi-bin` in the repository root builds the binary; `just pi-image`
puts it on a card image for one board and one panel, and
`just dev=1 pi-image` builds the bring-up variant of the same combination
(`image/README.md`). `board`, `panel` and `dev` are `just` variables, set
before the recipe name; the board directory's
`board.conf` carries the toolchain. For the two boards there today that is
the `armv7-unknown-linux-gnueabihf`
Rust target and a C cross-compiler, `gcc-arm-linux-gnueabihf` from the
distribution (`OPENSIGNER_PI_CC` overrides it), because `secp256k1-sys`
compiles libsecp256k1 from C. The binary lands at
`out/pi/<board>/opensigner-pi`, statically linked against glibc.

`just pi-image` puts that binary into a bootable microSD image: Buildroot in
a container, the panel and the legacy camera stack configured, the shell as
the only program on the device. It is described in
[`image/README.md`](image/README.md).
