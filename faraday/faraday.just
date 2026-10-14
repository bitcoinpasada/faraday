# Faraday's recipes, imported by the root justfile.

# release=1 makes a published release build: it refuses a dev image, a
# version override and a tree that differs from its commit
# (docs/DECISIONS.md F5). version=X.Y.Z makes a test or dev build claim
# another version, for trying the upgrade's newer-Faraday warning between
# test builds (F5).
release := "0"
version := ""

# The desktop app at out/linux/faraday, built in a Debian 11 container
# (faraday/build/linux) against glibc 2.31, so one file runs on Ubuntu
# 20.04 and later, Debian 11 and later, Fedora 32 and later and Arch
# (docs/STARTER.md §1.2). The Wayland and X11 libraries are loaded from the
# desktop it runs on. The Starter is built the same way with
# the arguments `out/linux out/build/run faraday-starter-shell faraday-starter`, against the payload a first
# `just faraday-release-stage` left: FARADAY_STARTER_PAYLOAD=out/payload/VERSION
# in the environment is mounted into the container and built in (STARTER
# §1.3); without it the Starter carries no payload. Then
# `just faraday-release-stage-starter VERSION` adds it to the release.
#
#   FARADAY_STARTER_PAYLOAD=out/payload/0.2.0 just faraday-linux-bin out/linux out/build/run faraday-starter-shell faraday-starter
#
# Sticks are folders in ~/faraday-sticks: BOOTSTICK, plugged in at start
# as the stick the device booted from (renamed, the app starts with no
# stick in), and STICK-2. F2 plugs the boot
# stick in or pulls it out, F3 stick 2, F4 pulls every stick; the keys
# are pinned at the top of the window. The boot stick holds the backup
# test kit (a 2-of-3 Taproot multisig's public
# backup files, an unsigned spend, and vault.ofv, passphrase a, holding
# its three seeds), all testnet, and every session starts on testnet;
# run the app with --full-kit for the full test kit. The stick image's
# app starts on mainnet.
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

# The Windows Starter (docs/STARTER.md §2), cross-compiled for
# x86_64-pc-windows-gnu in the pinned container (faraday/build/windows/),
# into OUT/faraday-starter.exe, with its icon, version and asInvoker manifest,
# and checked: no library beside Windows' own, no local path in its
# strings. FARADAY_STARTER_PAYLOAD=out/payload/VERSION builds that payload in
# (the stick and Pi images; the desktop app from batch C); without it the
# Starter offers only --image. `just faraday-windows-bin clippy` runs
# clippy for the Windows target instead.
#
#   FARADAY_STARTER_PAYLOAD=out/payload/0.2.0 just faraday-windows-bin
faraday-windows-bin what="build" out="out/windows" cache="out/build/run":
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
    arg=()
    [ "{{what}}" = clippy ] && arg=(clippy)
    {{docker}} run --rm \
        -e SOURCE_DATE_EPOCH="{{source_date_epoch}}" \
        -e FARADAY_BUILD="$FARADAY_BUILD" \
        ${payload[@]+"${payload[@]}"} \
        -v "$root":/src:ro \
        -v "$root/{{out}}":/out \
        -v "$root/{{cache}}/windows":/build \
        -v "$root/{{build_cache}}/cargo":/cargo \
        faraday-build-windows /src/faraday/build/windows/build.sh ${arg[@]+"${arg[@]}"}
    if [ "{{what}}" != clippy ]; then
        sha256sum "{{out}}/faraday-starter.exe"
    fi

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

# The Starter (docs/STARTER.md), built and run here: `just faraday-starter`, or
# `just faraday-starter --image out/stick/faraday-x86_64-uefi-dev.img` to write
# a dev image, through the same screens and writer as a release's.
# FARADAY_STARTER_PAYLOAD=out/payload/VERSION builds that payload in; without
# it the Starter offers only --image. release=1 makes a release
# build with no local paths in it (faraday/image/path-remap.sh); the build
# for publishing is `just faraday-linux-bin` with the arguments
# `out/linux out/build/run faraday-starter-shell faraday-starter`, in the glibc 2.31 container (`just faraday-linux-bin`'s
# comment).
faraday-starter *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    profile=()
    if [ "{{release}}" = 1 ]; then
        . faraday/image/path-remap.sh
        profile=(--release)
    fi
    cargo build -q "${profile[@]}" -p faraday-starter-shell --bin faraday-starter
    dir=debug
    [ "{{release}}" = 1 ] && dir=release
    exec "target/$dir/faraday-starter" {{args}}

# The Starter's pages in their states at one size, into
# out/snapshots/starter/WxH/, one named PNG each: `just faraday-starter-snap
# 1280x800`. 480x640 and 240x320 carry their panels' dpi; any other size
# renders at 160 unless spelled `WxH@DPI`.
faraday-starter-snap size:
    #!/usr/bin/env bash
    set -euo pipefail
    arg="{{size}}"
    case "{{size}}" in
        480x640) arg="480x640@286" ;;
        240x320) arg="240x320@143" ;;
    esac
    cargo run -q -p faraday-starter --example snap -- "$arg" "out/snapshots/starter/{{size}}" > /dev/null
    echo "out/snapshots/starter/{{size}}"

# Draws the Starter's menu icon again from the app's own mark
# (faraday-starter's examples/icon.rs) into shells/starter/assets/. Run it when
# the mark changes; the PNG is committed and the Starter embeds it.
faraday-starter-icon:
    cargo run -q -p faraday-starter --example icon -- faraday/shells/starter/assets/icon.png

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
    # No local path in the binaries (faraday/image/path-remap.sh).
    . faraday/image/path-remap.sh
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
    # No local path in the binaries (faraday/image/path-remap.sh).
    . faraday/image/path-remap.sh
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


# Stages a release's plain files under out/release/VERSION/ with the
# names docs/STARTER.md §1.1 gives them, images as .xz, and SHA256SUMS
# listing each file and each image inside its .xz, for the owner to
# sign, and the Starter's payload and MANIFEST under out/payload/VERSION/
# (STARTER §1.3); refuses files carrying a home folder, user name or host
# name. The stick image must be the release build (release=1); the stick,
# Pi and desktop builds (out/linux, from faraday-linux-bin) must carry
# one build ID; --any-build tries it on a test build.
#
#   just faraday-release-stage 0.2.0
#   just faraday-release-stage 0.2.0 --any-build --out DIR
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
faraday-release-stage-starter VERSION *args:
    faraday/tools/stage-release.sh {{VERSION}} --starter out/linux/faraday-starter --starter-windows out/windows/faraday-starter.exe {{args}}

# The Companion's screens (docs/COMPANION.md §4), each in its states, into
# out/snapshots/companion/WxH/, one named PNG each: `just companion-snap
# 1080x2340`, the phone `just snapshots` renders, at 420 dpi with its
# 63 px bottom inset. 480x640 and 240x320 carry their panels' dpi; any
# other size renders at 160 unless spelled `WxH@DPI`.
companion-snap size:
    #!/usr/bin/env bash
    set -euo pipefail
    arg="{{size}}"
    inset=0
    case "{{size}}" in
        1080x2340) arg="1080x2340@420"; inset=63 ;;
        480x640) arg="480x640@286" ;;
        240x320) arg="240x320@143" ;;
    esac
    cargo run -q -p faraday-companion --example snap -- "$arg" "out/snapshots/companion/{{size}}" "$inset" > /dev/null
    echo "out/snapshots/companion/{{size}}"

# --- Faraday Companion's Android app (faraday/shells/android) ------------
#
# As upstream's Android recipes in the root justfile: ANDROID_HOME (the
# SDK) and ANDROID_NDK_HOME (one NDK) from the environment, a JDK 21,
# `cargo ndk` and the Rust targets aarch64-linux-android and
# x86_64-linux-android; Gradle from the wrapper in the shell directory.
#
#   just companion-lib    the core for a phone (arm64-v8a) and the emulator
#                         (x86_64), into the app's jniLibs
#   just companion-apk    the debug APK, at out/android/faraday-companion-debug.apk,
#                         and its permissions checked
#   just companion-run    the emulator started if it is not up, the APK
#                         installed and launched, and its screen snapped
#                         into out/android/companion/
#   just companion-icon   the launcher icon drawn again from Faraday's mark
#
# The app sets FLAG_SECURE, so `adb exec-out screencap` returns black; the
# screenshot goes through the emulator console (tools/android-screenshot.py).

companion_dir := "faraday/shells/android"
companion_apk := "faraday/shells/android/app/build/outputs/apk/debug/app-debug.apk"
companion_avd := env_var_or_default("FARADAY_AVD", "headless")

# The core for both ABIs. The crate's library is libfaraday_companion_ffi.so
# (the core's own crate has the name faraday_companion); the app loads it
# as libfaraday_companion.so. Built with the path remaps, so the library
# carries no local path (docs/COMPANION.md §3).
companion-lib:
    #!/usr/bin/env bash
    set -euo pipefail
    . faraday/image/path-remap.sh
    cargo ndk -t arm64-v8a -t x86_64 -P 26 build --release -p faraday-companion-ffi
    for pair in arm64-v8a:aarch64-linux-android x86_64:x86_64-linux-android; do
        abi="${pair%%:*}"
        triple="${pair#*:}"
        mkdir -p "{{companion_dir}}/app/src/main/jniLibs/$abi"
        cp "target/$triple/release/libfaraday_companion_ffi.so" \
            "{{companion_dir}}/app/src/main/jniLibs/$abi/libfaraday_companion.so"
    done

# The debug APK, a clean Gradle build (the incremental packager can leave
# an old library as dead space in the archive), copied to a stable path.
# Then the permissions the APK asks for, as anyone checks them
# (docs/COMPANION.md §6.2): exactly CAMERA and USE_BIOMETRIC, or this fails.
companion-apk: companion-lib
    #!/usr/bin/env bash
    set -euo pipefail
    : "${ANDROID_HOME:?set ANDROID_HOME to the Android SDK root}"
    (cd {{companion_dir}} && ./gradlew --quiet clean assembleDebug)
    mkdir -p out/android
    cp {{companion_apk}} out/android/faraday-companion-debug.apk
    aapt2="$(ls -d "$ANDROID_HOME"/build-tools/*/ | sort -V | tail -1)aapt2"
    asked="$("$aapt2" dump permissions out/android/faraday-companion-debug.apk)"
    echo "$asked"
    got="$(echo "$asked" | sed -n "s/^uses-permission: name='\([^']*\)'.*/\1/p" | sort | tr '\n' ' ')"
    want="android.permission.CAMERA android.permission.USE_BIOMETRIC "
    if [ "$got" != "$want" ]; then
        echo "companion-apk: the APK asks for: ${got:-nothing}; it may ask for: $want" >&2
        exit 1
    fi
    if echo "$asked" | grep -qvE "^(package: |uses-permission: name='android\.permission\.(CAMERA|USE_BIOMETRIC)'$)"; then
        echo "companion-apk: aapt2 lists more than the two permissions" >&2
        exit 1
    fi
    ls -l out/android/faraday-companion-debug.apk

# Install on the emulator (starting it if needed), launch, and screenshot.
companion-run: companion-apk
    #!/usr/bin/env bash
    set -euo pipefail
    : "${ANDROID_HOME:?set ANDROID_HOME to the Android SDK root}"
    adb="$ANDROID_HOME/platform-tools/adb"
    if ! "$adb" devices | grep -qE '^emulator-[0-9]+[[:space:]]+device$'; then
        nohup "$ANDROID_HOME/emulator/emulator" -avd "{{companion_avd}}" \
            -no-window -gpu swiftshader_indirect -no-audio -no-boot-anim \
            >/dev/null 2>&1 &
        disown
        "$adb" wait-for-device
        until [ "$("$adb" shell getprop sys.boot_completed | tr -d '\r')" = "1" ]; do
            sleep 2
        done
    fi
    "$adb" install -r out/android/faraday-companion-debug.apk
    "$adb" shell am start -n app.faradayoffline.companion/.MainActivity
    sleep 5
    tools/android-screenshot.py out/android/companion/home.png

# The launcher icon's foreground, drawn from Faraday's mark
# (faraday-companion's examples/icon.rs). Run it when the mark changes;
# the PNG is committed.
companion-icon:
    cargo run -q -p faraday-companion --example launcher_icon -- {{companion_dir}}/app/src/main/res/drawable-nodpi/ic_launcher_foreground.png
