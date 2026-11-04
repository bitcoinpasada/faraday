# Developing Faraday

**Kind:** Reference
**Status:** Built

How to build Faraday from a fresh clone, check it, reproduce a release,
and try it on your own hardware. Each section names the document that
says more. Everything here runs on your own machine; GitHub Actions runs
the check (§2) on every push to main and every pull request, and
nothing else.

## 1. What you need

- **Rust**, the version in `rust-toolchain.toml` (rustup installs it on
  the first `cargo` command), and a C compiler: `secp256k1-sys` builds
  libsecp256k1 from source. The cross-check's `codex32` area also builds
  two small Rust programs of their own, outside the workspace, from the
  vendored `tools/reference/rust-codex32/` (rust-codex32) and
  `tools/vectors/codex32-volvelle/harness/` (volvelle's worksheet), offline
  with `cargo build --frozen`: the toolchain above is all they need.
- **just** (the task runner) and, for the check, `cargo install --locked
  cargo-nextest cargo-deny cargo-vet`, and the two Mac targets, `rustup
  target add aarch64-apple-darwin x86_64-apple-darwin`
  (`tools/check-macos.sh` type-checks the Starter's macOS code with them).
- **Node 22 or later**, for the colour tables' lint
  (`tools/lint-themes.sh`).
- **Python 3.11 or later** and **git** for the lints (`tools/impact.py`
  reads TOML with `tomllib` and the history with git) and the
  cross-check. The cross-check's
  `vault` area needs the distribution's argon2-cffi (`python3-argon2`),
  pykeepass (`python3-pykeepass`) and KeePassXC's command line
  (`keepassxc-cli`, package `keepassxc`), PyCA cryptography (`python3-cryptography`) and the keepass-rs program
  (`tools/crosscheck/keepass-rs/`, built by `tools/crosscheck/run.sh` with the
  Rust toolchain from its committed `Cargo.lock`, offline), and its
  `sign`, `msg` and `musig` areas python-ecdsa (`python3-ecdsa`), its
  `ur` area cbor2 (`python3-cbor2`), its `totp` area pyotp
  (`python3-pyotp`), its
  `seedqr` area Pillow (`python3-pil`), zbar (`zbar-tools`), zxing-cpp
  (`python3-zxing-cpp`) and segno (`python3-segno`); PyPy is
  optional (`tools/crosscheck/README.md`).
- **Go** 1.27 or later, for the cross-check only: its `lightning` area
  builds LND's own aezeed code from `tools/reference/lnd/` and its `msg`
  area btcd's BIP-322 code from `tools/reference/btcd-bip322/`, and its
  `bip32` and `taproot` areas a released btcd's `hdkeychain` and
  control-block check from `tools/reference/btcd/` (all vendored, so
  offline, with `GOTOOLCHAIN=local`). Install it from
  <https://go.dev/dl/> (the tarball's SHA-256 is on that page) or the
  distribution's `golang` package; `go version` must say 1.27 or later,
  or `just crosscheck` stops.
  The `lightning` area's software-round pass reads the commands it builds
  with binutils' `objdump` (x86-64 only), which comes with the C toolchain
  Rust links with.
- The judges `docs/ESTABLISHED.md` §9.6 lists for the cross-check are
  distribution packages and vendored code to come; each is added here
  by the change that first needs it, and `just crosscheck` fails
  without it. It checks first that every tool it needs is there and
  names a missing one (`tools/crosscheck/judges.py`), runs the tests
  below that skip without a tool under plain `just` with
  `FARADAY_JUDGES_REQUIRED=1`, so a missing tool fails them, and runs
  the OVMF Secure Boot check (§6.3's tools and `systemd-boot-efi`).
- **Bitcoin Core v31.1**, which the cross-check requires: the official
  release tarball for x86_64 or aarch64 Linux, fetched and checked once
  by a script that needs **gpg** and git (§7). It is not in the
  repository.
- **Podman or Docker**, rootless, for the stick and Pi images and the
  Linux and Windows app builds (`DOCKER=podman` picks Podman).
- **GnuPG 2.4, paperkey 1.6, Sequoia's `sq` 1.3 built over Nettle, OpenSSL, sbsigntool** (`sbverify`), **GnuTLS's
  `certtool`, pesign** (`pesign`, `pesigcheck`) and **python3-pefile**: the
  GPG, Secure Boot and QR vault tests run these tools against Faraday's files. A
  missing GnuPG, paperkey, `sq`, OpenSSL or sbverify fails them, and so does
  an `sq` whose `sq version` does not say "cryptographic backend Nettle"; a missing
  `certtool`, pesign or pefile skips its test with a note in `just` and
  fails it in `just crosscheck`. On Debian or Ubuntu:
  `sudo apt install gnupg paperkey sq openssl sbsigntool gnutls-bin pesign
  python3-pefile`.
- **mtools, dosfstools, gdisk, fdisk** (`mkfs.fat`, `fsck.fat`, `fatlabel`,
  `mcopy`, `sgdisk`, `sfdisk`; on some systems `sfdisk` comes with
  util-linux): the FAT and stick-image tests skip with a note without
  them under `just`, and fail without them under `just crosscheck`.
- **xz-utils and 7-Zip** (`xz`, and `7zz` or `7z` from the `7zip` package,
  upstream 7-Zip; p7zip does not count): the Starter's tests decode good
  and damaged `.xz` streams with both beside Faraday's decoder. They skip
  with a note without them under `just`, and fail without them under `just
  crosscheck`. On Debian or Ubuntu: `sudo apt install xz-utils 7zip`.
- **QEMU and OVMF** (`qemu-system-x86_64`, OVMF's Secure Boot firmware
  (`OVMF_CODE.secboot.4m.fd`, or `OVMF_CODE_4M.secboot.fd` on Debian and
  Ubuntu), and `systemd-boot-efi`), to boot the stick
  image without a stick (§6.3); `just crosscheck` boots OVMF over
  Faraday's Secure Boot keys and fails without them. On Debian or
  Ubuntu: `sudo apt install qemu-system-x86 ovmf systemd-boot-efi`.
- On Linux, the X11 or Wayland client libraries at runtime, including
  `libxkbcommon-x11`, for the desktop app.

## 2. The check

```
just                      the tests, then every lint run to the end (lint-all)
just test                 the tests alone (nextest, then the doc tests)
just test-doc             the doc tests alone
just lint                 clippy with warnings as errors, fmt, and the repository's lints, in five parts:
just lint-clippy          clippy over every target
just lint-deps            cargo deny, cargo vet, the fuzz crates built
just lint-repo            fmt, the scripts in tools/, the registry
just lint-macos           the Starter's macOS code for both Mac targets
just lint-touch-targets   the touch-target ratchet: every snapshot tour at four sizes
just lint-all             every command of `lint`, run to the end; lists all that failed
cargo nextest run -p faraday-core -E 'test(/^keygen::/)'     one test module
```

Run the module you touched while working, and `just` once for a finished
body of work, not after each change or commit. When it fails, run again
only the command `lint-all` names.
`tools/README.md` lists every lint and what it holds; `CLAUDE.md` is the
rules a change follows. Builds and checks pass `--locked`.

GitHub Actions runs `just` on every push to main and every pull
request (`.github/workflows/check.yml`, F183), on Ubuntu 26.04 with
the tools above at the versions it names, spread over twenty-one jobs:
one compiles the tests once and archives them (`cargo nextest
archive`), fourteen each run a fourteenth of them (`--partition
hash:k/14`), and the doc tests and the five parts of `just lint` run
beside them. Its result is shown on the commit and the
pull request.

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
just faraday-stick-image                  out/stick/faraday-x86_64-uefi.img
just faraday-pi-image                     out/pi/faraday-pi3-any.img
just dev=1 faraday-stick-image            out/stick/faraday-x86_64-uefi-dev.img
just dev=1 faraday-pi-image               out/pi/faraday-pi3-any-dev.img
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
hardware it is offered under (`docs/IMAGES.md` §1, §2).

## 5. Reproducing a release

The images are built in a container with every compiler pinned, the
commit date as `SOURCE_DATE_EPOCH`, host paths remapped out of the
output, and the FAT serials derived from that date (README.md,
"Reproducible builds"). To check a release:

```
git tag -v vX.Y.Z                        the tag is signed by the release key
git checkout vX.Y.Z
just release=1 faraday-stick-image
just release=1 faraday-pi-image
sha256sum out/stick/faraday-x86_64-uefi.img out/pi/faraday-pi3-any.img
```

Compare each sum with the line for the `.img` (not the `.img.xz`) in the
release's signed `SHA256SUMS`, or with the third field of its line in
`IMAGES` (README.md, "Verifying a release"): the published files are
these images, renamed and compressed (`faraday/tools/stage-release.sh`,
which publishes every image `faraday/builds.toml` names release files
for, under those names: `out/pi/faraday-pi3-any.img` is
`faraday-pi-X.Y.Z.img.xz`). Same bytes or not, say what you got
in an issue, with the commit, your container engine and your host
system: no second builder has reproduced a Faraday image yet.

## 6. Trying it on your hardware

### 6.1. Write a stick

Faraday Starter writes a stick or a card and checks it by reading it
back. To write an image you built:

```
just faraday-starter --image out/stick/faraday-x86_64-uefi-dev.img
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
  `faraday-boot-report.txt` on the data partition (the stick's,
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
  (`faraday/image/README.md`, "The dev variant"). The file holds every
  run since boot, in order, each under a `run N` line: a lock ends the
  app and the next one carries on in the same file.

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
faraday/tools/stick-qemu.py --image out/stick/faraday-x86_64-uefi-dev.img --vnc 9 --serial stdio
just faraday-disk-test                    the stick's disk handling, end to end, under QEMU (add --no-kvm without KVM)
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

Faraday's Rust is checked against Python references, some written by
others and some second implementations Faraday's own authors wrote
(`docs/CROSSCHECK.md` §10, F160, F177),
over every published vector and a seeded random corpus (`docs/CROSSCHECK.md`,
`docs/decisions/F79.md`). It runs offline, in a network namespace of
its own.

**Bitcoin Core, once.** The cross-check runs Bitcoin Core v31.1
(`bitcoind` on regtest, no network), from the official release tarball
pinned in `tools/reference/core/release.pin`. Fetch it once, with the
network, from the repository root:

```
python3 -I -B tools/crosscheck/fetch_core.py
```

It checks the release's `SHA256SUMS` against the signatures of the
builders whose keys `guix.sigs` publishes (at least five good ones, no
bad one), and the tarball against `SHA256SUMS` and the pin. It keeps
the tarball in `~/.cache/faraday/bitcoin-core` (under `$XDG_CACHE_HOME`
when that is set). To keep it elsewhere, set `FARADAY_BITCOIN_CORE` to
a folder, for the fetch and for every run. Each run checks the tarball
against the pin again. Without it, or with another, the cross-check
fails and says so. `--machine aarch64` fetches the other tarball;
`--from DIR` checks files already downloaded (`SHA256SUMS`,
`SHA256SUMS.asc` and the tarball) instead of fetching them.

```
just crosscheck                 the stamp run: fixed seed, about 90 s on 12 threads
just crosscheck --fresh         before a release: a new seed, ten times the cases
just crosscheck --area musig    one area
just crosscheck --jobs 4        four workers (default: one a core)
```

The full cross-check runs before a release, and the tip between releases
is not kept cross-checked (F197): `just` does not check the stamp, and a
change to the crypto commits none. A change to an area's code runs that
area (`--area`, which writes no stamp). A passing full run writes
`tools/crosscheck/stamp`, committed with the release; `just lint-release`
fails when the crypto changed since it or an action's code changed since
its fingerprint.

`just crosscheck --fresh` also writes `out/crosscheck-record.txt`: the
commit, seed, scale and UTC date; each area's cases, answers, known
differences and result; each folder of `tools/reference/` with the commit
its README names and the SHA-256 of its files; Bitcoin Core's tarball with
its version and SHA-256; and "every answer agreed" or the first
disagreement. The release flow stages it as `crosscheck-<version>.txt`
beside `SHA256SUMS` and lists it there.

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
python3 -I -B docs/crypto/check.py check        coverage, truth and the places it checks
                                                (AUDIT §1, the Reveal count, COMPUTE §2.3,
                                                the Tools tags, the not_crypto count)
just lint-release               before a release: the stamp, and the same check with the
                                fingerprints (`check.py check --release`)
python3 -I -B docs/crypto/check.py --self-test  the checker over a small fixture
python3 -I -B docs/crypto/gen.py write          rewrite the generated places of STANDARDS and AUDIT
python3 -I -B docs/crypto/gen.py check          fail if one differs from the manifest
python3 -I -B docs/crypto/gen.py --self-test    the generator over a small fixture
```

`docs/STANDARDS.md` §1 to §7 and `docs/AUDIT.md` §3 are generated from the manifest
between `<!-- crypto-manifest: begin … -->` and `end` lines: change the manifest and
run `gen.py write`, never the text between the lines. `just crypto-review ID` runs
`gen.py write` itself. `just lint` runs `check.py check` and `gen.py check`, so an
uncovered function or a generated place out of date fails the build; an action whose
code changed since its review fails `just lint-release`, before a release (F197).

## 8. Fuzzing

```
just fuzz-list                  every target, one per parser of untrusted input
just fuzz psbt_parse 300        one target for 300 seconds
```

`just lint` checks that every fuzz target still compiles (`cargo check --locked`
on the stable toolchain, no nightly or cargo-fuzz needed).

The fuzz targets need the nightly in `fuzz/rust-toolchain.toml` and
cargo-fuzz. `fuzz/README.md` and `faraday/fuzz/README.md` say what each
target feeds; `tools/README.md`, "Before a release", is the release run.

## 9. Where to read next

- `docs/README.md`: every document, and which answers which question.
- `CLAUDE.md`: the rules every change follows.
- `tools/README.md`: every tool and lint.
- `docs/AUDIT.md`: where the cryptography is, for a reviewer.
- `SECURITY.md`: how to report a security problem.
