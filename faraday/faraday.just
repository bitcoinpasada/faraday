# Faraday's recipes, imported by the root justfile. Each output lands
# where hp-pull collects it (~/Projects/hp-build-setup/README.md).

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
    {{docker}} build -t opensigner-build-linux tools/build/linux
    {{docker}} run --rm \
        -e SOURCE_DATE_EPOCH="{{source_date_epoch}}" \
        -v "$root":/src:ro \
        -v "$root/{{out}}":/out \
        -v "$root/{{cache}}/linux":/build \
        -v "$root/{{build_cache}}/cargo":/cargo \
        opensigner-build-linux /src/faraday/build/linux/build.sh
    file "{{out}}/faraday"
    echo "needs $(objdump -T "{{out}}/faraday" | grep -o 'GLIBC_[0-9.]*' | sort -t. -k2,2n -u | tail -1)"
    sha256sum "{{out}}/faraday"

# The test stick's files, all on testnet (every test wallet in each backup
# form with its spends, the test keys in each form, the vault; listed in
# its README.txt), after checking every spend signs and finishes.
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

# The stick shell for x86-64, static against musl, at
# out/faraday/x86_64-uefi/opensigner-pi: the name upstream's Buildroot
# package installs as /usr/bin/opensigner-pi. Beside it faraday-disk and
# faraday-grant.
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
    cargo build --release --target "$RUST_TARGET" -p faraday-stick -p faraday-disk -p faraday-grant
    mkdir -p out/faraday/{{stick_board}}
    cp "target/$RUST_TARGET/release/faraday" out/faraday/{{stick_board}}/opensigner-pi
    # The disk process and the grant helper (PLAN.md §4.3), which
    # run-build.sh puts in the image's /usr/bin.
    cp "target/$RUST_TARGET/release/faraday-disk" "target/$RUST_TARGET/release/faraday-grant" \
        out/faraday/{{stick_board}}/
    file out/faraday/{{stick_board}}/opensigner-pi out/faraday/{{stick_board}}/faraday-disk \
        out/faraday/{{stick_board}}/faraday-grant

# The Faraday USB stick image, at out/stick/faraday-x86_64-uefi.img
# (dev=1: -dev.img, with a console and a login on ttyS0). Flash it to a
# stick and boot the laptop from it.
faraday-stick-image: faraday-stick-bin
    #!/usr/bin/env bash
    set -euo pipefail
    variant="{{stick_board}}-{{stick_panel}}"
    name="faraday-x86_64-uefi"
    if [ "{{dev}}" != "0" ]; then
        variant="$variant-dev"
        name="$name-dev"
    fi
    DOCKER="{{docker}}" faraday/image/run-build.sh \
        "{{stick_board}}" "{{stick_panel}}" "{{dev}}" "{{buildroot_tag}}" \
        "{{buildroot_commit}}"
    mkdir -p out/stick
    cp "out/pi/$variant/output/images/opensigner-x86_64-uefi.img" "out/stick/$name.img"
    ls -l "out/stick/$name.img"

# The Pi shell, cross-compiled for the board the root justfile's `board`
# variable names (armv7, static against glibc, as upstream's own pi-bin
# does), at out/faraday/<board>/opensigner-pi: the name upstream's
# Buildroot package installs as /usr/bin/opensigner-pi. Beside it
# faraday-disk and faraday-grant, same as the stick.
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
    cargo build --release --target "$RUST_TARGET" -p faraday-stick -p faraday-disk -p faraday-grant
    mkdir -p out/faraday/{{board}}
    cp "target/$RUST_TARGET/release/faraday" out/faraday/{{board}}/opensigner-pi
    cp "target/$RUST_TARGET/release/faraday-disk" "target/$RUST_TARGET/release/faraday-grant" \
        out/faraday/{{board}}/
    file out/faraday/{{board}}/opensigner-pi out/faraday/{{board}}/faraday-disk \
        out/faraday/{{board}}/faraday-grant

# The Faraday microSD card image, at
# out/pi/faraday-<board>-<panel>.img (dev=1: -dev.img, with a console and
# a login on the board's serial port). Flash it to a card and boot the Pi
# from it.
#
#   just faraday-pi-image                        the owner's device (pi3,
#                                                  waveshare-28dpi)
#   just board=B panel=P dev=0|1 faraday-pi-image any other combination
faraday-pi-image: faraday-pi-bin
    #!/usr/bin/env bash
    set -euo pipefail
    variant="{{board}}-{{panel}}"
    name="faraday-{{board}}-{{panel}}"
    if [ "{{dev}}" != "0" ]; then
        variant="$variant-dev"
        name="$name-dev"
    fi
    DOCKER="{{docker}}" faraday/image/run-build.sh \
        "{{board}}" "{{panel}}" "{{dev}}" "{{buildroot_tag}}" \
        "{{buildroot_commit}}"
    mkdir -p out/pi
    cp "out/pi/$variant/output/images/opensigner-pi.img" "out/pi/$name.img"
    ls -l "out/pi/$name.img"
