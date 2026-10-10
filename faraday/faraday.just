# Faraday's recipes, imported by the root justfile.

# release=1 makes a published release build: it refuses a dev image, a
# version override and a tree that differs from its commit
# (docs/DECISIONS.md F5). version=X.Y.Z makes a test or dev build claim
# another version, for trying the upgrade's newer-Faraday warning between
# test builds (F5).
release := "0"
version := ""

# The desktop app at out/linux/faraday: glibc 2.36 or newer, which
# winit needs to load the Wayland and X11 libraries.
#
# Sticks are folders in ~/faraday-sticks: F2 plugs the test stick in
# or pulls it out, F3 the blank stick, F4 pulls every stick. The test
# stick is the backup test stick (a 2-of-3 Taproot multisig's public
# backup files, an unsigned spend, and vault.ofv, passphrase a, holding
# its three seeds), all testnet, and every session starts on testnet;
# run the app with --full-kit for the full test kit. The stick image's
# app starts on mainnet.
faraday-linux-bin out="out/linux" cache="out/build/run":
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
    {{docker}} build -t opensigner-build-linux tools/build/linux
    {{docker}} run --rm \
        -e SOURCE_DATE_EPOCH="{{source_date_epoch}}" \
        -e FARADAY_BUILD="$FARADAY_BUILD" \
        -v "$root":/src:ro \
        -v "$root/{{out}}":/out \
        -v "$root/{{cache}}/linux":/build \
        -v "$root/{{build_cache}}/cargo":/cargo \
        opensigner-build-linux /src/faraday/build/linux/build.sh
    file "{{out}}/faraday"
    echo "needs $(objdump -T "{{out}}/faraday" | grep -o 'GLIBC_[0-9.]*' | sort -t. -k2,2n -u | tail -1)"
    sha256sum "{{out}}/faraday"

# The desktop app standing for a device's screen at its true size on
# this one, to try the small-panel layout by touch: 2.8 is the Pi's
# Waveshare panel (3:4), 5 a phone (9:16). Under Hyprland it opens on
# the laptop's own screen (eDP), whose EDID gives the pixels per inch;
# pass extra flags after the size, such as `--ppi 277` or `--aspect 9:19.5`.
faraday-panel inches="2.8" *flags="":
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build -q --release -p faraday-desktop
    if command -v hyprctl >/dev/null && hyprctl monitors 2>/dev/null | grep -q '^Monitor eDP'; then
        output="$(hyprctl monitors | awk '/^Monitor eDP/ {print $2; exit}')"
        hyprctl dispatch "hl.dsp.focus({ monitor = \"$output\" })" >/dev/null 2>&1 \
            || hyprctl dispatch focusmonitor "$output" >/dev/null
    fi
    exec target/release/faraday --panel "{{inches}}" {{flags}}

# The test stick's files, all on testnet (every test wallet in each backup
# form with its spends, the test keys in each form, the vault), after
# checking every spend signs and finishes.
faraday-testkit out="out/testkit":
    cargo run -q -p faraday-testkit -- "{{out}}"

# Every screen along the snapshot tour, at each size, into
# out/snapshots/faraday/WxH/. Desktop sizes first (160 dpi, the
# default), then the two small-panel references: 480x640 at 286 dpi is
# the Pi's Waveshare panel, 240x320 at 143 is the smallest the design
# supports (docs/PLANNING.md §16.4) -- both `small` size class, so this
# is where a card or a flow that only fits `wide` would show up.
faraday-snapshots: faraday-testkit
    #!/usr/bin/env bash
    set -euo pipefail
    for size in 1280x800 1920x1080 1024x768 3840x2160; do
        cargo run -q -p faraday-snapshot -- "$size" out/testkit "out/snapshots/faraday/$size" > /dev/null
        echo "out/snapshots/faraday/$size"
    done
    for spec in "480x640 286" "240x320 143"; do
        read -r size dpi <<< "$spec"
        cargo run -q -p faraday-snapshot -- "$size@$dpi" out/testkit "out/snapshots/faraday/$size" > /dev/null
        echo "out/snapshots/faraday/$size"
    done

# The default tour at one size, for looking at a change: `just faraday-snap
# 480x640`. Known sizes (480x640, 240x320, 1280x800, 1920x1080, 1024x768,
# 3840x2160) carry their own dpi; any other size needs `WxH@DPI` spelled
# out (`just faraday-snap 320x480@213`), or it renders at 160.
faraday-snap size: faraday-testkit
    #!/usr/bin/env bash
    set -euo pipefail
    arg="{{size}}"
    case "{{size}}" in
        480x640) arg="480x640@286" ;;
        240x320) arg="240x320@143" ;;
    esac
    cargo run -q -p faraday-snapshot -- "$arg" out/testkit "out/snapshots/faraday/{{size}}"
    echo "out/snapshots/faraday/{{size}}"

# Secure Boot in real UEFI firmware: OVMF in QEMU enrols the keys
# faraday-sb makes from their .auth updates, then runs only the image
# signed with the db key and refuses an unsigned or altered one, for both
# enrolment policies. Needs QEMU, OVMF's Secure Boot build, mtools and
# systemd-boot; serial logs in out/sb-ovmf-check/.
faraday-sb-ovmf out="out/sb-ovmf-check":
    python3 -B faraday/tools/sb-ovmf-check.py "{{out}}"

# Upgrading a stick in QEMU (PLAN.md §5.5): the dev stick image boots, an
# older stick image (OLD) is plugged in, and the boot copier writes the
# new boot partition over the older stick's; then the upgraded stick boots
# the new release, its data partition byte for byte what it was. Root on
# the dev console stands in for the app, which no input in QEMU can drive
# (faraday/tools/upgrade-check.py says what is and is not exercised).
# Needs QEMU, OVMF and mtools; console logs in out/upgrade-check/.
faraday-upgrade-check old out="out/upgrade-check":
    python3 -B faraday/tools/upgrade-check.py out/stick/faraday-x86_64-uefi-dev.img "{{old}}" "{{out}}"

# The stick shell for x86-64, static against musl, at
# out/faraday/x86_64-uefi/opensigner-pi: the name upstream's Buildroot
# package installs as /usr/bin/opensigner-pi. Beside it faraday-disk,
# faraday-grant and faraday-boot.
faraday-stick-bin:
    #!/usr/bin/env bash
    set -euo pipefail
    . opensigner/shells/pi/image/boards/{{stick_board}}/board.conf
    under="${RUST_TARGET//-/_}"
    upper="$(echo "$under" | tr 'a-z' 'A-Z')"
    export "CC_${under}=$CC"
    export "AR_${under}=${CC%gcc}ar"
    export "CARGO_TARGET_${upper}_LINKER=$CC"
    export "CFLAGS_${under}=$CFLAGS"
    export RUSTFLAGS="-C target-feature=+crt-static"
    # Not `export FARADAY_BUILD=$(...)`: with `set -e`, export's own exit
    # status hides a failure of the command inside it.
    FARADAY_BUILD="$(FARADAY_RELEASE={{release}} FARADAY_DEV={{dev}} FARADAY_VERSION={{version}} faraday/image/build-id.sh)"
    export FARADAY_BUILD
    cargo build --release --target "$RUST_TARGET" -p faraday-stick -p faraday-disk -p faraday-grant -p faraday-boot
    mkdir -p out/faraday/{{stick_board}}
    cp "target/$RUST_TARGET/release/faraday" out/faraday/{{stick_board}}/opensigner-pi
    # The disk process, the grant helper (PLAN.md §4.3) and the boot
    # copier (§5.5), which run-build.sh puts in the image's /usr/bin.
    cp "target/$RUST_TARGET/release/faraday-disk" "target/$RUST_TARGET/release/faraday-grant" \
        "target/$RUST_TARGET/release/faraday-boot" out/faraday/{{stick_board}}/
    file out/faraday/{{stick_board}}/opensigner-pi out/faraday/{{stick_board}}/faraday-disk \
        out/faraday/{{stick_board}}/faraday-grant out/faraday/{{stick_board}}/faraday-boot

# The Faraday USB stick image, at out/stick/faraday-x86_64-uefi.img
# (dev=1: -dev.img, with a console and a login on ttyS0), and beside it
# -boot.vfat, its boot partition's exact bytes, for faraday-stick-boot.
# Flash it to a stick and boot the laptop from it. A test release unless
# release=1 (docs/DECISIONS.md F5):
#
#   just faraday-stick-image              a test release
#   just release=1 faraday-stick-image    a published release
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
    cp "out/pi/$variant/output/images/opensigner-x86_64-uefi.img" "out/stick/$name.img"
    cp "out/pi/$variant/output/images/esp.vfat" "out/stick/$name-boot.vfat"
    ls -l "out/stick/$name.img" "out/stick/$name-boot.vfat"

# For development, not offered to users (PLAN.md §5.5): writes the stick
# image's boot partition over the partition named OSKBOOT on a Faraday
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

# The Pi shell, cross-compiled for the board the root justfile's `board`
# variable names (armv7, static against glibc, as upstream's own pi-bin
# does), at out/faraday/<board>/opensigner-pi: the name upstream's
# Buildroot package installs as /usr/bin/opensigner-pi. Beside it
# faraday-disk, faraday-grant and faraday-boot, same as the stick.
#
#   just faraday-pi-bin                  the owner's board (pi3)
#   just board=pi02w faraday-pi-bin      the Zero 2 W
faraday-pi-bin:
    #!/usr/bin/env bash
    set -euo pipefail
    . opensigner/shells/pi/image/boards/{{board}}/board.conf
    under="${RUST_TARGET//-/_}"
    upper="$(echo "$under" | tr 'a-z' 'A-Z')"
    export "CC_${under}=$CC"
    export "AR_${under}=${CC%gcc}ar"
    export "CARGO_TARGET_${upper}_LINKER=$CC"
    export "CFLAGS_${under}=$CFLAGS"
    export RUSTFLAGS="-C target-feature=+crt-static"
    # Not `export FARADAY_BUILD=$(...)`: with `set -e`, export's own exit
    # status hides a failure of the command inside it.
    FARADAY_BUILD="$(FARADAY_RELEASE={{release}} FARADAY_DEV={{dev}} FARADAY_VERSION={{version}} faraday/image/build-id.sh)"
    export FARADAY_BUILD
    cargo build --release --target "$RUST_TARGET" -p faraday-stick -p faraday-disk -p faraday-grant -p faraday-boot
    mkdir -p out/faraday/{{board}}
    cp "target/$RUST_TARGET/release/faraday" out/faraday/{{board}}/opensigner-pi
    cp "target/$RUST_TARGET/release/faraday-disk" "target/$RUST_TARGET/release/faraday-grant" \
        "target/$RUST_TARGET/release/faraday-boot" out/faraday/{{board}}/
    file out/faraday/{{board}}/opensigner-pi out/faraday/{{board}}/faraday-disk \
        out/faraday/{{board}}/faraday-grant out/faraday/{{board}}/faraday-boot

# The Faraday microSD card image, at
# out/pi/faraday-<board>-<panel>.img (dev=1: -dev.img, with a console and
# a login on the board's serial port). Flash it to a card and boot the Pi
# from it. A test release unless release=1 (docs/DECISIONS.md F5):
#
#   just faraday-pi-image                        the owner's device (pi3,
#                                                  waveshare-28dpi)
#   just board=B panel=P dev=0|1 faraday-pi-image any other combination
#   just release=1 faraday-pi-image              a published release
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
    cp "out/pi/$variant/output/images/opensigner-pi.img" "out/pi/$name.img"
    ls -l "out/pi/$name.img"

