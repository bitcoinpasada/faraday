# Developing Faraday

**Kind:** Reference
**Status:** Built

How to build Faraday from a fresh clone, check it, reproduce a release,
and try it on your own hardware. Each section names the document that
says more. Everything here runs on your own machine; there is no hosted
CI.

## 1. What you need

- **Rust**, the version in `rust-toolchain.toml` (rustup installs it on
  the first `cargo` command), and a C compiler: `secp256k1-sys` builds
  libsecp256k1 from source.
- **just** (the task runner) and, for the check, `cargo install --locked
  cargo-nextest cargo-deny cargo-vet`.
- **Python 3.11 or later** and **git** for the lints (`tools/impact.py`
  reads TOML with `tomllib` and the history with git) and the
  cross-check. The cross-check's
  `vault` and `kdbx` areas need the distribution's argon2-cffi, PyNaCl
  and pykeepass; PyPy is optional (`tools/crosscheck/README.md`).
- **Podman or Docker**, rootless, for the stick and Pi images and the
  Linux and Windows app builds (`DOCKER=podman` picks Podman).
- **mtools, dosfstools** (`mkfs.fat`, `fsck.fat`): the FAT tests skip
  with a note without them.
- **QEMU and OVMF**, to boot the stick image without a stick (§6.3).
- On Linux, the X11 or Wayland client libraries at runtime, including
  `libxkbcommon-x11`, for the desktop app.

## 2. The check

```
just                      the tests, then the lints: what every change passes
just test                 the tests alone (nextest, then the doc tests)
just lint                 clippy with warnings as errors, fmt, and the repository's lints
cargo nextest run -p faraday-core -E 'test(/^keygen::/)'     one test module
```

Run the module you touched while working and `just` once at the end.
`tools/README.md` lists every lint and what it holds; `CLAUDE.md` is the
rules a change follows. Builds and checks pass `--locked`.

## 3. The desktop app

```
cargo run -p faraday-desktop --release            Faraday in a window, in Practice on testnet
just faraday-panel 2.8                            the Pi's 2.8-inch panel at its true size
just faraday-testkit                              the test stick's files, into out/testkit
just faraday-snap 480x640 spend                   one tour at one size, into out/snapshots/faraday/480x640/spend/
```

The desktop app, Faraday Demo, has none of the stick's protections and
is never for a seed that holds funds. Its virtual sticks are folders in
`~/faraday-sticks` (`--sticks DIR` moves them): README.md, "Faraday
Demo", and `docs/DEMO.md`.

## 4. The images

```
just faraday-stick-image                  out/stick/faraday-x86_64-uefi-strict.img
just build=compatible faraday-stick-image out/stick/faraday-x86_64-uefi-compatible.img
just faraday-pi-image                     out/pi/faraday-pi3-waveshare-28dpi.img
just build=compatible faraday-pi-image    out/pi/faraday-pi3-waveshare-28dpi-compatible.img
just panel=hdmi build=compatible faraday-pi-image
                                          out/pi/faraday-pi3-hdmi-compatible.img
just dev=1 faraday-stick-image            out/stick/faraday-x86_64-uefi-strict-dev.img
just dev=1 faraday-pi-image               out/pi/faraday-pi3-waveshare-28dpi-dev.img
just faraday-starter                      the Starter, built and run here
just faraday-linux-bin                    one Linux file, out/linux/faraday (glibc 2.31 container)
```

The first image build clones Buildroot at its pinned tag and takes 30
to 60 minutes; later builds redo only what changed. A build is a test
release unless built with `release=1`, which refuses a dev image, a
version override and a tree that differs from its commit
(`docs/decisions/F5.md`). `faraday/image/README.md` describes the
image, its partitions, the boards and panels, and the build.

`faraday/builds.toml` lists every build the repository makes, each
with its recipe, its `just` variables and the files it leaves, and the
builds that are untried (a recipe, never built or booted) or planned
and not yet made. A tool that offers a choice of
builds reads it; a new build is added there, and `just lint` fails while
a board or panel directory has no entry. Its `[[hardware]]` entries are
the website's download tiles (`docs/IMAGES.md` §7): each image names the
hardware and the build (Strict or Compatible) it is offered as. On the
Pi, Strict is the kernel the Pi has always had and Compatible adds a
display through the kernel, USB input and USB storage (`docs/IMAGES.md`
§1, §2); the HDMI screen is built Compatible only.

## 5. Reproducing a release

The images are built in a container with every compiler pinned, the
commit date as `SOURCE_DATE_EPOCH`, host paths remapped out of the
output, and the FAT serials derived from that date (README.md,
"Reproducible builds"). To check a release:

```
git tag -v v0.1.8                         the tag is signed by the release key
git checkout v0.1.8
just release=1 faraday-stick-image
just release=1 build=compatible faraday-stick-image
just release=1 faraday-pi-image
sha256sum out/stick/faraday-x86_64-uefi-strict.img out/stick/faraday-x86_64-uefi-compatible.img out/pi/faraday-pi3-waveshare-28dpi.img
```

Compare each sum with the line for the `.img` (not the `.img.xz`) in the
release's signed `SHA256SUMS` (README.md, "Verifying a release"): the
published files are these images, renamed and compressed
(`faraday/tools/stage-release.sh`). Same bytes or not, say what you got
in an issue, with the commit, your container engine and your host
system: no second builder has reproduced a Faraday image yet.

## 6. Trying it on your hardware

### 6.1. Write a stick

Faraday Starter writes a stick or a card and checks it by reading it
back. To write an image you built:

```
just faraday-starter --image out/stick/faraday-x86_64-uefi-strict-dev.img
```

`--image` takes an `.img`, an `.img.xz`, or a `-boot.vfat`. **Write the
whole stick** erases it; **keep data** writes the boot partition alone
and leaves the data partition (vaults, settings, files) as it is.
`docs/STARTER.md` §4.1 says what the writer checks. `dd` works too and
overwrites the whole device (`faraday/image/README.md`, "Write it to a
card or a stick").

While working on the stick image, rewrite only its boot partition:

```
just dev=1 faraday-stick-boot --dry-run /dev/sdX     check only
just dev=1 faraday-stick-boot /dev/sdX               write the -dev image's boot partition
```

It refuses a disk that is not a Faraday stick on USB or that has
anything mounted, and asks for the device to be typed back.

### 6.2. The dev image

`dev=1` builds the image for finding out why a machine does not work.
Never use it for keys that matter.

- **The kernel console on the screen**, and on the serial port (`ttyS0`
  at 115200 on the stick, `serial0` on the Pi's GPIO header), with a
  root login there and no password.
- **`--verbose`**: the app says which panel it chose and every input and
  camera device it found, and what it made of each.
- **The boot report, every 30 seconds.** The app writes
  `faraday-boot-report.txt` on the `FARADAYDATA` partition (the stick's,
  or the Pi card's) as
  soon as the stick is listed at boot, again whole every 30 seconds
  while it runs, and once more when it ends. It holds the kernel log
  (the app's own errors included), then the panel and devices found,
  headed with the seconds since the app started, then two blocks
  refreshed at each write. **System**: the seconds from the kernel's
  start to the app's first frame, when the kernel's random generator was
  ready (its `crng init done` line), the temperature of each
  `/sys/class/thermal` zone now and its highest, the first CPU's clock
  now, lowest and highest, the Raspberry Pi's undervoltage alarm now and
  since start where the kernel has its `rpi_volt` sensor, with the kernel
  log's undervoltage warnings counted, `MemAvailable` now and lowest, and
  the app's peak and current memory, all read once a second; the
  firmware's throttling flags are "not available", since they are read
  only through `/dev/vcio`, which no Faraday kernel builds, and anything
  else the kernel does not expose says so too. The Pi's dev card builds
  the temperature sensor and the undervoltage alarm for this
  (`faraday/image/boards/pi3/dev/linux.fragment`); the release card has
  neither. **Radios**: what
  `/sys/class/bluetooth` and `/sys/class/net` hold, every device-tree
  node whose `compatible` names a Bluetooth or a Wi-Fi chip, with its
  path, its status and its parent's (the serial port or SDIO controller
  it sits on), and every serial port with its driver, the devices bound
  on one, and whether it is a console or has a getty. Each write goes to a
  new file that is read back and renamed over the old one, so a stick
  pulled mid-write keeps the last whole report. Wait half a minute
  after the thing you want recorded, pull the stick, and read the file
  on any computer.
- **The timings, every 30 seconds.** `faraday-timings.txt` beside it:
  per five seconds of the app's clock, each kind of work (a touch, a
  frame, a camera frame, a fade) counted with its mean and worst in
  milliseconds, and a `work` row for each kind of costly work done in
  that time: a vault made, unlocked or its cost measured, named by its
  strength, Argon2id memory and passes (and "failed: no memory" or "not
  tried: too little memory free" where it did not fit), a key made, a
  transaction signed, a camera frame read for a QR code, a file written
  to a stick or the card, and the rest of what the core counts
  (`faraday/image/README.md`, "The dev variant").

The release image writes neither file: its app is started without
those flags, its kernel log stays closed to the app, and
`faraday/image/build.sh` refuses a release image that says otherwise.
Neither file holds a secret or anything derived from one: no key,
words, passphrase, address, or name a person chose for a vault or a
file. The core prints nothing, and the writers read only the kernel's
files, the shell's clock and the core's counts of costly work
(`tools/lint-secrets.sh`, rules 1 and 5). The report is the file to
send with a report of a machine that does not work.

### 6.3. Without a stick: QEMU

```
faraday/tools/stick-qemu.py --image out/stick/faraday-x86_64-uefi-strict-dev.img --vnc 9 --serial stdio
just faraday-disk-test                    the stick's disk handling, end to end, under QEMU
```

The machine's screen is on VNC at `127.0.0.1:5909`, loopback only; with
the dev image the kernel console and `--verbose` are on the terminal.
`faraday/image/README.md`, "Try it in QEMU first", has the webcam
passthrough and `faraday/tools/qemu-check.py`.

### 6.4. What to report

An issue with the machine's make and model, the image's build ID (the
kernel's release string: `uname -r` on the dev image's console), what
happened, and the dev image's `faraday-boot-report.txt`. `faraday/image/README.md`, "The USB
stick on a laptop", lists what is known to work and what is not.

## 7. The crypto cross-check

Faraday's Rust is checked against independent Python references over
every published vector and a seeded random corpus (`docs/CROSSCHECK.md`,
`docs/decisions/F79.md`). It runs offline, in a network namespace of
its own.

```
just crosscheck                 the stamp run: fixed seed, about 90 s on 12 threads
just crosscheck --fresh         before a release: a new seed, ten times the cases
just crosscheck --area musig    one area
just crosscheck --jobs 4        four workers (default: one a core)
```

A passing full run writes `tools/crosscheck/stamp`; `just lint` fails
when the crypto changed since the stamp, until the cross-check passes
again. Commit the stamp with the change.

**Watching a run.** From another terminal, or over ssh:

```
just crosscheck-status              the state now
just crosscheck-status --follow     again every five seconds until the run ends
just crosscheck-status --log        the log so far
tail -f out/crosscheck/progress.log the same, as it is written
```

A run writes `out/crosscheck/progress.log` (one line at each event and
at least every ten seconds: cases done, cases a second, workers busy,
time left), `progress.json` and `report.txt`. A disagreement under
`--fresh` is kept in `found/` with its seed and inputs.

**The cryptographic actions' manifest.** `docs/crypto/actions.toml` lists
every cryptographic action with the functions it covers
(`docs/CRYPTO-LEARN.md`). Both recipes need `cargo` and Python 3.11 or
later:

```
just crypto-review ID           print the action's entry, what changed in its code and the
                                places to read with it; then write its fingerprint
just crypto-changes TAG         the actions whose fingerprint changed since a tag
python3 -I -B docs/crypto/check.py check        coverage, truth and fingerprints
python3 -I -B docs/crypto/check.py --self-test  the checker over a small fixture
```

## 8. Fuzzing

```
just fuzz-list                  every target, one per parser of untrusted input
just fuzz psbt_parse 300        one target for 300 seconds
```

The fuzz targets need the nightly in `fuzz/rust-toolchain.toml` and
cargo-fuzz. `fuzz/README.md` and `faraday/fuzz/README.md` say what each
target feeds; `tools/README.md`, "Before a release", is the release run.

## 9. Where to read next

- `docs/README.md`: every document, and which answers which question.
- `CLAUDE.md`: the rules every change follows.
- `tools/README.md`: every tool and lint.
- `docs/AUDIT.md`: where the cryptography is, for a reviewer.
- `SECURITY.md`: how to report a security problem.
