# Faraday's recipes, imported by the root justfile.

# release=1 makes a published release build: it refuses a dev image, a
# version override and a tree that differs from its commit
# (docs/DECISIONS.md F5). version=X.Y.Z makes a test or dev build claim
# another version, for trying the Starter's Keep data newer-Faraday
# warning between test builds (F5).
release := "0"
version := ""

# The containers' runtime: `docker`, or DOCKER=podman in the environment.
# Rootless is what the builds are written for: container root is you, so
# every file written into out/ belongs to you.
docker := env_var_or_default("DOCKER", "docker")
# The commit date, so what a builder gets depends on the commit and not
# on the day it was built.
source_date_epoch := `git log -1 --format=%ct 2>/dev/null || echo 0`
# Where the containers keep their caches: the cargo registry and the
# cargo target directories. Nothing here is an artifact.
build_cache := "out/build"

# Buildroot's long-term-support series, pinned to a release tag and to the
# commit that tag pointed at when it was taken. The tag is what a person
# reads; the commit is what the build checks, because a tag can be moved
# and a commit cannot. faraday/image/run-build.sh clones at the tag and
# then fails if HEAD is not this.
buildroot_tag := "2024.02.13"
buildroot_commit := "e687e3815f76c1c5ea9fb52b6558bedfe53ab117"

# Which image to build. These are `just` variables, so they are set on
# the command line before the recipe name (`just dev=1 faraday-pi-image`).
# board and panel name directories under faraday/image/boards and
# faraday/image/panels; dev=1 adds faraday/image/variants/dev, a console
# and a login, which never goes on a device that signs anything.
board := "pi3"
panel := "waveshare-28dpi"
dev := "0"
# The stick's board and panel.
stick_board := "x86_64-uefi"
stick_panel := "efi-framebuffer"

# The desktop app at out/linux/faraday, built in a Debian 11 container
# (faraday/build/linux) against glibc 2.31, so one file runs on Ubuntu
# 20.04 and later, Debian 11 and later, Fedora 32 and later and Arch
# (docs/STARTER.md §1.2). It is a developer's build: people run Faraday
# Demo inside the Starter, which carries it compiled in (F126). The
# Wayland and X11 libraries are loaded from the
# desktop it runs on. The Starter is built the same way with
# the arguments `out/linux out/build/run faraday-starter-shell faraday-starter`, against the payload a first
# `just faraday-release-stage` left: FARADAY_STARTER_PAYLOAD=out/payload/VERSION/linux
# in the environment is mounted into the container and built in (STARTER
# §1.3); without it the Starter carries no payload. Then
# `just faraday-release-stage-starter VERSION` adds it to the release.
#
#   FARADAY_STARTER_PAYLOAD=out/payload/0.2.0/linux just faraday-linux-bin out/linux out/build/run faraday-starter-shell faraday-starter
#
# Sticks are folders in ~/faraday-sticks: BOOTSTICK, plugged in at start
# as the stick the device booted from (renamed, the app starts with no
# stick in), and STICK-1 and on, one folder for each backup stick. The
# pills across the top of the window plug each stick in or pull it out.
# The boot stick starts empty, and every session starts on testnet. The
# stick image's app starts on mainnet.
[group('build')]
faraday-linux-bin out="out/linux" cache="out/build/run" package="faraday-desktop" bin="faraday":
    #!/usr/bin/env bash
    set -euo pipefail
    root="$(pwd)"
    mkdir -p "{{out}}" "{{cache}}/linux" "{{build_cache}}/cargo"
    # Not `export FARADAY_BUILD=$(...)`: with `set -e`, export's own exit
    # status hides a failure of the command inside it. The desktop build
    # has no dev variant (docs/DECISIONS.md F5), so FARADAY_DEV is fixed
    # at 0 rather than following a `dev` variable.
    FARADAY_BUILD="$(FARADAY_RELEASE={{release}} FARADAY_DEV=0 FARADAY_VERSION={{version}} faraday/image/build-id.sh)"
    export FARADAY_BUILD
    {{docker}} build -t faraday-build-linux -f faraday/build/linux/Containerfile faraday/build/linux
    # The Starter's payload, mounted where build.rs reads it.
    payload=()
    if [ -n "${FARADAY_STARTER_PAYLOAD:-}" ]; then
        [ -f "$FARADAY_STARTER_PAYLOAD/MANIFEST" ] || { echo "$FARADAY_STARTER_PAYLOAD/MANIFEST: not there" >&2; exit 1; }
        payload=(-v "$(realpath "$FARADAY_STARTER_PAYLOAD")":/payload:ro -e FARADAY_STARTER_PAYLOAD=/payload)
    fi
    {{docker}} run --rm \
        -e SOURCE_DATE_EPOCH="{{source_date_epoch}}" \
        -e FARADAY_BUILD="$FARADAY_BUILD" \
        ${payload[@]+"${payload[@]}"} \
        -v "$root":/src:ro \
        -v "$root/{{out}}":/out \
        -v "$root/{{cache}}/linux":/build \
        -v "$root/{{build_cache}}/cargo":/cargo \
        faraday-build-linux /src/faraday/build/linux/build.sh "{{package}}" "{{bin}}"
    file "{{out}}/{{bin}}"
    sha256sum "{{out}}/{{bin}}"

# The Windows Starter and desktop app (docs/STARTER.md §2), cross-compiled
# for x86_64-pc-windows-gnu in the pinned container (faraday/build/windows/),
# into OUT/BIN.exe, with its icon, version and asInvoker manifest, and
# checked: no library beside Windows' own, no local path in its strings.
# Parameters are positional, in this order, as faraday-linux-bin's are;
# the last two default to the Starter, which carries Faraday Demo compiled
# in (F126); the desktop app alone is a developer's build.
# FARADAY_STARTER_PAYLOAD=out/payload/VERSION/windows
# builds that payload into the Starter (the stick and Pi images); without
# it the Starter offers only --image. what
# `clippy` runs clippy for the Windows target on that package instead.
#
#   FARADAY_STARTER_PAYLOAD=out/payload/0.2.0/windows just faraday-windows-bin
#   just faraday-windows-bin build out/windows out/build/run faraday-desktop faraday
#   just faraday-windows-bin clippy out/windows out/build/run faraday-desktop faraday
[group('build')]
faraday-windows-bin what="build" out="out/windows" cache="out/build/run" package="faraday-starter-shell" bin="faraday-starter":
    #!/usr/bin/env bash
    set -euo pipefail
    root="$(pwd)"
    mkdir -p "{{out}}" "{{cache}}/windows" "{{build_cache}}/cargo"
    FARADAY_BUILD="$(FARADAY_RELEASE={{release}} FARADAY_DEV=0 FARADAY_VERSION={{version}} faraday/image/build-id.sh)"
    export FARADAY_BUILD
    {{docker}} build -t faraday-build-windows -f faraday/build/windows/Containerfile faraday/build/windows
    payload=()
    if [ -n "${FARADAY_STARTER_PAYLOAD:-}" ]; then
        [ -f "$FARADAY_STARTER_PAYLOAD/MANIFEST" ] || { echo "$FARADAY_STARTER_PAYLOAD/MANIFEST: not there" >&2; exit 1; }
        payload=(-v "$(realpath "$FARADAY_STARTER_PAYLOAD")":/payload:ro -e FARADAY_STARTER_PAYLOAD=/payload)
    fi
    {{docker}} run --rm \
        -e SOURCE_DATE_EPOCH="{{source_date_epoch}}" \
        -e FARADAY_BUILD="$FARADAY_BUILD" \
        ${payload[@]+"${payload[@]}"} \
        -v "$root":/src:ro \
        -v "$root/{{out}}":/out \
        -v "$root/{{cache}}/windows":/build \
        -v "$root/{{build_cache}}/cargo":/cargo \
        faraday-build-windows /src/faraday/build/windows/build.sh "{{what}}" "{{package}}" "{{bin}}"
    if [ "{{what}}" != clippy ]; then
        sha256sum "{{out}}/{{bin}}.exe"
    fi

# The desktop app standing for a device's screen at its true size on
# this one, to try the small-panel layout by touch: 2.8 is the Pi's
# Waveshare panel (3:4), 5 a phone (9:16). Under Hyprland it opens on
# the laptop's own screen (eDP), whose EDID gives the pixels per inch;
# pass extra flags after the size, such as `--ppi 277` or `--aspect 9:19.5`.
[group('dev')]
faraday-panel inches="2.8" *flags="":
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build -q --release --locked -p faraday-desktop
    if command -v hyprctl >/dev/null && hyprctl monitors 2>/dev/null | grep -q '^Monitor eDP'; then
        output="$(hyprctl monitors | awk '/^Monitor eDP/ {print $2; exit}')"
        hyprctl dispatch "hl.dsp.focus({ monitor = \"$output\" })" >/dev/null 2>&1 \
            || hyprctl dispatch focusmonitor "$output" >/dev/null
    fi
    exec target/release/faraday --panel "{{inches}}" {{flags}}

# The test stick's files, all on testnet (every test wallet in each backup
# form with its spends, the test keys in each form, the vault), after
# checking every spend signs and finishes. `net` makes them on another
# network instead: `just faraday-testkit out/testkit-mainnet mainnet`.
[group('dev')]
faraday-testkit out="out/testkit" net="testnet":
    cargo run -q -p faraday-testkit -- --network "{{net}}" "{{out}}"

# The network faraday-snap's tour runs on: FARADAY_SNAP_NET=mainnet takes
# it as a Mainnet stick shows it, from its own test kit.
snap_net := env_var_or_default("FARADAY_SNAP_NET", "testnet")
snap_kit := if snap_net == "testnet" { "out/testkit" } else { "out/testkit-" + snap_net }
snap_suffix := if snap_net == "testnet" { "" } else { "-" + snap_net }

# Every screen along the snapshot tour, at each size, into
# out/snapshots/faraday/WxH/, and again with text at Larger into
# out/snapshots/faraday/WxH-larger/, with cut.txt there listing each
# frame with text cut off at a side or drawn over other text
# (docs/TEXT-SIZE.md §7). Desktop sizes first (160 dpi, the
# default), a window of each size class (docs/LAYOUT.md §2, §11): the
# desktop app's 1280x800 (large), 1920x1080, 2000x1090, 2560x1440 and
# 3840x2160 (extra-large), 1024x768 (expanded) and the half-screen
# 760x1000 (medium); then the two small-panel references: 480x640 at 286 dpi is
# the Pi's Waveshare panel, 240x320 at 143 is the smallest the design
# supports -- both `small` size class, so this
# is where a card or a flow that only fits `wide` would show up. Then
# the stick's 7-inch and 10-inch panels at both orientations
# (docs/ORIENTATION.md §10): 1024x600 and 600x1024 at 170 dpi, and
# 1280x800 and 800x1280 at 150, the landscape one into 1280x800-150
# beside the desktop window's 1280x800.
[group('test')]
faraday-snapshots: faraday-testkit
    #!/usr/bin/env bash
    set -euo pipefail
    for spec in "1280x800 1280x800 160" "1920x1080 1920x1080 160" "2000x1090 2000x1090 160" "2560x1440 2560x1440 160" "1024x768 1024x768 160" "760x1000 760x1000 160" "3840x2160 3840x2160 160" "480x640 480x640 286" "240x320 240x320 143" "1024x600 1024x600 170" "600x1024 600x1024 170" "1280x800-150 1280x800 150" "800x1280 800x1280 150"; do
        read -r name size dpi <<< "$spec"
        cargo run -q -p faraday-snapshot -- "$size@$dpi" out/testkit "out/snapshots/faraday/$name" > /dev/null
        echo "out/snapshots/faraday/$name"
        FARADAY_SNAP_TEXT=larger cargo run -q -p faraday-snapshot -- "$size@$dpi" out/testkit "out/snapshots/faraday/$name-larger" > /dev/null
        echo "out/snapshots/faraday/$name-larger"
    done

# One tour at one size, for looking at a change: `just faraday-snap
# 480x640` for the default tour, `just faraday-snap 480x640 spend` for one
# of the named tours faraday-snapshot's usage lists, into
# out/snapshots/faraday/WxH[/TOUR]. Known sizes (480x640, 240x320,
# 1280x800, 1920x1080, 2000x1090, 2560x1440, 1024x768, 760x1000,
# 3840x2160, and the stick's panels at
# both orientations: 1024x600 and 600x1024 at 170 dpi, 1280x800-150 and
# 800x1280 at 150) carry their own dpi; any other
# size needs `WxH@DPI` spelled out (`just faraday-snap 320x480@213`), or
# it renders at 160. The tour runs on testnet; `FARADAY_SNAP_NET=mainnet
# just faraday-snap 1280x800 spend` runs it as a Mainnet stick shows it
# (no network pill, bc1 addresses, xpubs; the same test seeds at coin
# type 0), from out/testkit-mainnet into
# out/snapshots/faraday/WxH-mainnet[/TOUR]. FARADAY_SNAP_TEXT=larger
# takes it with text at Larger, into out/snapshots/faraday/WxH-larger
# [/TOUR], with cut.txt (docs/TEXT-SIZE.md §7).
[group('test')]
faraday-snap size tour="": (faraday-testkit snap_kit snap_net)
    #!/usr/bin/env bash
    set -euo pipefail
    arg="{{size}}"
    case "{{size}}" in
        480x640) arg="480x640@286" ;;
        240x320) arg="240x320@143" ;;
        1024x600) arg="1024x600@170" ;;
        600x1024) arg="600x1024@170" ;;
        1280x800-150) arg="1280x800@150" ;;
        800x1280) arg="800x1280@150" ;;
    esac
    out="out/snapshots/faraday/{{size}}{{snap_suffix}}"
    case "${FARADAY_SNAP_TEXT:-standard}" in
        standard) ;;
        *) out="$out-$FARADAY_SNAP_TEXT" ;;
    esac
    tour=()
    if [ -n "{{tour}}" ]; then
        out="$out/{{tour}}"
        tour=("{{tour}}")
    fi
    cargo run -q -p faraday-snapshot -- --network "{{snap_net}}" "$arg" "{{snap_kit}}" "$out" ${tour[@]+"${tour[@]}"}
    echo "$out"

# The user guide's pictures (docs/guide/): the snapshot's guide and
# screens tours and the Starter's guide pictures, at 1280x800 and 160 dpi,
# Cobalt, Light, Standard, on Mainnet, each shot named as its picture's
# file, with `<name>.mark.json` beside a picture whose next control is
# marked. tools/guide-pictures.py copies into `out` every picture the
# guide's pages show or name as to come, removes the ones none shows,
# lists those no tour drew, and fails on a picture that carries text,
# EXIF, ICC or time chunks. The renders are left in a fresh temporary
# folder, whose path it prints. A tour that fails is named and fails the
# recipe once the pictures it drew are collected.
[group('dev')]
guide-pictures out="docs/guide/pictures": (faraday-testkit "out/testkit-mainnet" "mainnet")
    #!/usr/bin/env bash
    set -euo pipefail
    renders="$(mktemp -d -t faraday-guide-renders.XXXXXX)"
    failed=0
    for tour in guide screens; do
        cargo run -q --locked -p faraday-snapshot -- --network mainnet 1280x800@160 out/testkit-mainnet "$renders/$tour" "$tour" > /dev/null || failed=1
    done
    cargo run -q --locked -p faraday-starter --example snap -- --guide 1280x800@160 "$renders/starter" cobalt,light,standard > /dev/null || failed=1
    python3 -B tools/guide-pictures.py "{{out}}" "$renders/guide" "$renders/screens" "$renders/starter"
    echo "$renders"
    exit "$failed"

# The Starter (docs/STARTER.md), built and run here: `just faraday-starter`, or
# `just faraday-starter --image out/stick/faraday-x86_64-uefi-dev.img` to write
# a dev image, through the same screens and writer as a release's.
# FARADAY_STARTER_PAYLOAD=out/payload/VERSION builds that payload in; without
# it the Starter offers only --image. release=1 makes a release
# build with no local paths in it (faraday/image/path-remap.sh); the build
# for publishing is `just faraday-linux-bin` with the arguments
# `out/linux out/build/run faraday-starter-shell faraday-starter`, in the glibc 2.31 container (`just faraday-linux-bin`'s
# comment).
[group('dev')]
faraday-starter *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    profile=()
    if [ "{{release}}" = 1 ]; then
        . faraday/image/path-remap.sh
        profile=(--release)
    fi
    cargo build -q --locked "${profile[@]}" -p faraday-starter-shell --bin faraday-starter
    dir=debug
    [ "{{release}}" = 1 ] && dir=release
    exec "target/$dir/faraday-starter" {{args}}

# The Starter's pages in their states at one size, into
# out/snapshots/starter/WxH/, one named PNG each: `just faraday-starter-snap
# 1280x800`. 480x640 and 240x320 carry their panels' dpi; any other size
# renders at 160 unless spelled `WxH@DPI`. FARADAY_SNAP_TEXT=larger takes
# them with text at Larger, into out/snapshots/starter/WxH-larger/, with
# cut.txt (docs/TEXT-SIZE.md §7).
[group('test')]
faraday-starter-snap size:
    #!/usr/bin/env bash
    set -euo pipefail
    arg="{{size}}"
    case "{{size}}" in
        480x640) arg="480x640@286" ;;
        240x320) arg="240x320@143" ;;
    esac
    out="out/snapshots/starter/{{size}}"
    case "${FARADAY_SNAP_TEXT:-standard}" in
        standard) ;;
        *) out="$out-$FARADAY_SNAP_TEXT" ;;
    esac
    cargo run -q -p faraday-starter --example snap -- "$arg" "$out" > /dev/null
    echo "$out"

# Draws the Starter's menu icon again from the app's own mark
# (faraday-starter's examples/icon.rs) into shells/starter/assets/. Run it when
# the mark changes; the PNG is committed and the Starter embeds it.
[group('build')]
faraday-starter-icon:
    cargo run -q -p faraday-starter --example icon -- faraday/shells/starter/assets/icon.png

# Secure Boot in real UEFI firmware: OVMF in QEMU enrols the keys
# faraday-sb makes from their .auth updates, then runs only the image
# signed with the db key and refuses an unsigned or altered one, for both
# enrolment policies. Needs QEMU, OVMF's Secure Boot build, mtools and
# systemd-boot; serial logs in out/sb-ovmf-check/.
[group('test')]
faraday-sb-ovmf out="out/sb-ovmf-check":
    python3 -B faraday/tools/sb-ovmf-check.py "{{out}}"

# The stick's app for x86-64, static against musl, at
# out/faraday/x86_64-uefi/faraday, and beside it faraday-disk
# and faraday-grant: what faraday/image/run-build.sh puts in
# the image. The board's board.conf (faraday/image/boards/) names the
# Rust target and the C compiler.
[group('build')]
faraday-stick-bin:
    #!/usr/bin/env bash
    set -euo pipefail
    . faraday/image/boards/{{stick_board}}/board.conf
    under="${RUST_TARGET//-/_}"
    upper="$(echo "$under" | tr 'a-z' 'A-Z')"
    export "CC_${under}=$CC"
    export "AR_${under}=${CC%gcc}ar"
    export "CARGO_TARGET_${upper}_LINKER=$CC"
    export "CFLAGS_${under}=$CFLAGS"
    export RUSTFLAGS="-C target-feature=+crt-static"
    # No local path in the binaries (faraday/image/path-remap.sh).
    . faraday/image/path-remap.sh
    # Not `export FARADAY_BUILD=$(...)`: with `set -e`, export's own exit
    # status hides a failure of the command inside it.
    FARADAY_BUILD="$(FARADAY_RELEASE={{release}} FARADAY_DEV={{dev}} FARADAY_VERSION={{version}} faraday/image/build-id.sh)"
    export FARADAY_BUILD
    cargo build --release --locked --target "$RUST_TARGET" -p faraday-stick -p faraday-disk -p faraday-grant
    mkdir -p out/faraday/{{stick_board}}
    cp "target/$RUST_TARGET/release/faraday" "target/$RUST_TARGET/release/faraday-disk" \
        "target/$RUST_TARGET/release/faraday-grant" out/faraday/{{stick_board}}/
    file out/faraday/{{stick_board}}/faraday out/faraday/{{stick_board}}/faraday-disk \
        out/faraday/{{stick_board}}/faraday-grant

# The Faraday USB stick image, at out/stick/faraday-x86_64-uefi.img
# (dev=1: -dev.img, with a console and a login on ttyS0), and beside it
# -boot.vfat, its boot partition's exact bytes, for faraday-stick-boot.
# Flash it to a stick and boot the laptop from it. A test release unless
# release=1 (docs/DECISIONS.md F5):
#
#   just faraday-stick-image              a test release
#   just release=1 faraday-stick-image    a published release
[group('build')]
faraday-stick-image: faraday-stick-bin
    #!/usr/bin/env bash
    set -euo pipefail
    variant="{{stick_board}}-{{stick_panel}}"
    name="faraday-x86_64-uefi"
    if [ "{{dev}}" != "0" ]; then
        variant="$variant-dev"
        name="$name-dev"
    fi
    DOCKER="{{docker}}" FARADAY_RELEASE="{{release}}" FARADAY_VERSION="{{version}}" \
        faraday/image/run-build.sh \
        "{{stick_board}}" "{{stick_panel}}" "{{dev}}" "{{buildroot_tag}}" \
        "{{buildroot_commit}}"
    mkdir -p out/stick
    cp "out/pi/$variant/output/images/faraday-x86_64-uefi.img" "out/stick/$name.img"
    cp "out/pi/$variant/output/images/esp.vfat" "out/stick/$name-boot.vfat"
    ls -l "out/stick/$name.img" "out/stick/$name-boot.vfat"

# For development, not offered to users (docs/history/PLAN.md §5.5): writes the stick
# image's boot partition over the partition named FARADAYBOOT on a Faraday
# stick, leaving its data partition, vaults and settings as they are.
# Refuses a disk that is not removable on USB, has anything mounted, or
# whose partition table is not a Faraday stick's; the device is typed
# back; it reads back and compares. dev=1 writes the -dev image's.
#
#   just faraday-stick-boot /dev/sdX
#   just dev=1 faraday-stick-boot /dev/sdX
#   just faraday-stick-boot --dry-run /dev/sdX    checks, writes nothing
#
# The target may be a regular file holding a stick image, for testing.
# Writes a stick's boot partition alone, for development.
[group('dev')]
faraday-stick-boot +args:
    #!/usr/bin/env bash
    set -euo pipefail
    name="faraday-x86_64-uefi"
    if [ "{{dev}}" != "0" ]; then
        name="$name-dev"
    fi
    dry=""
    set -- {{args}}
    if [ "${1:-}" = "--dry-run" ]; then
        dry="--dry-run"
        shift
    fi
    [ $# -eq 1 ] || { echo "usage: just [dev=1] faraday-stick-boot [--dry-run] /dev/sdX" >&2; exit 1; }
    exec faraday/image/stick-boot.sh $dry "out/stick/$name-boot.vfat" "$1"

# The Pi's app, cross-compiled for the board the `board` variable names
# (armv7, static against glibc), at out/faraday/<board>/faraday, and
# beside it faraday-disk and faraday-grant, as for the
# stick.
#
#   just faraday-pi-bin                  the owner's board (pi3)
[group('build')]
faraday-pi-bin:
    #!/usr/bin/env bash
    set -euo pipefail
    . faraday/image/boards/{{board}}/board.conf
    under="${RUST_TARGET//-/_}"
    upper="$(echo "$under" | tr 'a-z' 'A-Z')"
    export "CC_${under}=$CC"
    export "AR_${under}=${CC%gcc}ar"
    export "CARGO_TARGET_${upper}_LINKER=$CC"
    export "CFLAGS_${under}=$CFLAGS"
    export RUSTFLAGS="-C target-feature=+crt-static"
    # No local path in the binaries (faraday/image/path-remap.sh).
    . faraday/image/path-remap.sh
    # Not `export FARADAY_BUILD=$(...)`: with `set -e`, export's own exit
    # status hides a failure of the command inside it.
    FARADAY_BUILD="$(FARADAY_RELEASE={{release}} FARADAY_DEV={{dev}} FARADAY_VERSION={{version}} faraday/image/build-id.sh)"
    export FARADAY_BUILD
    cargo build --release --locked --target "$RUST_TARGET" -p faraday-stick -p faraday-disk -p faraday-grant
    mkdir -p out/faraday/{{board}}
    cp "target/$RUST_TARGET/release/faraday" "target/$RUST_TARGET/release/faraday-disk" \
        "target/$RUST_TARGET/release/faraday-grant" out/faraday/{{board}}/
    file out/faraday/{{board}}/faraday out/faraday/{{board}}/faraday-disk \
        out/faraday/{{board}}/faraday-grant

# The Faraday microSD card image, at
# out/pi/faraday-<board>-<panel>.img (dev=1: -dev.img, with a console and
# a login on the board's serial port). Flash it to a card and boot the Pi
# from it. A test release unless release=1 (docs/DECISIONS.md F5):
#
#   just faraday-pi-image                        the owner's device (pi3,
#                                                  waveshare-28dpi)
#   just board=B panel=P dev=0|1 faraday-pi-image any other combination
#   just release=1 faraday-pi-image              a published release
[group('build')]
faraday-pi-image: faraday-pi-bin
    #!/usr/bin/env bash
    set -euo pipefail
    variant="{{board}}-{{panel}}"
    name="faraday-{{board}}-{{panel}}"
    if [ "{{dev}}" != "0" ]; then
        variant="$variant-dev"
        name="$name-dev"
    fi
    DOCKER="{{docker}}" FARADAY_RELEASE="{{release}}" FARADAY_VERSION="{{version}}" \
        faraday/image/run-build.sh \
        "{{board}}" "{{panel}}" "{{dev}}" "{{buildroot_tag}}" \
        "{{buildroot_commit}}"
    mkdir -p out/pi
    cp "out/pi/$variant/output/images/faraday-pi.img" "out/pi/$name.img"
    ls -l "out/pi/$name.img"


# Stages a release's plain files under out/release/VERSION/ with the
# names docs/STARTER.md §1.1 gives them, images as .xz, and SHA256SUMS
# listing each file and each image inside its .xz, for the owner to
# sign, and the Starter's payload and MANIFEST under out/payload/VERSION/
# (STARTER §1.3); refuses files carrying a home folder, user name or host
# name. The stick image must be the release build (release=1); the stick
# and Pi builds must carry one build ID; --any-build tries it on a test
# build. Faraday Demo is compiled into the Starter (F126) and staged
# nowhere: no faraday-demo file is in the payload or the release folder.
# The payload has linux/ and windows/, one for each Starter, each holding
# the two images and MANIFEST.
#
#   just faraday-release-stage 0.2.0
#   just faraday-release-stage 0.2.0 --any-build --out DIR
[group('release')]
faraday-release-stage VERSION *args:
    faraday/tools/stage-release.sh {{VERSION}} {{args}}

# The second pass of staging (STARTER §1.3): adds the Linux Starter,
# out/linux/faraday-starter, and the Windows Starter,
# out/windows/faraday-starter.exe, both built against the payload the first
# pass wrote, to the folder the first pass staged:
# Faraday-Starter-VERSION-Linux.tar.gz holding Faraday-Starter-VERSION-Linux,
# and Faraday-Starter-VERSION-Windows.exe, all three in SHA256SUMS. Refuses a
# Starter that does not carry that payload. With --out DIR it adds to DIR,
# as the first pass was given.
#
#   just faraday-release-stage-starter 0.2.0
#   just faraday-release-stage-starter 0.2.0 --out DIR
[group('release')]
faraday-release-stage-starter VERSION *args:
    faraday/tools/stage-release.sh {{VERSION}} --starter out/linux/faraday-starter --starter-windows out/windows/faraday-starter.exe {{args}}

# The stick's disk process end to end (faraday/tools/disk-test.py): QEMU
# boots the dev stick image, and the script plugs sticks in and out through
# QMP and checks what is mounted, who owns each partition and which USB
# devices are authorised. Needs `just dev=1 faraday-stick-image` first, and
# QEMU, OVMF and mtools; screenshots go to out/stick/disk-test/.
[group('test')]
faraday-disk-test out="out/stick/disk-test": faraday-testkit
    python3 -B faraday/tools/disk-test.py out/stick/faraday-x86_64-uefi-dev.img "{{out}}" out/testkit
