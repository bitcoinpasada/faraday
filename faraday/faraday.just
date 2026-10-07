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
# out/snapshots/faraday/WxH/.
faraday-snapshots: faraday-testkit
    #!/usr/bin/env bash
    set -euo pipefail
    for size in 1280x800 1920x1080 1024x768 3840x2160; do
        cargo run -q -p faraday-snapshot -- "$size" out/testkit "out/snapshots/faraday/$size" > /dev/null
        echo "out/snapshots/faraday/$size"
    done

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
