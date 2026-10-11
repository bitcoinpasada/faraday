# OpenSignerKit task runner. Everything runs locally; there is no hosted CI.

default: test lint

# Run every unit, integration and doc test in the workspace. nextest runs
# the 108 test binaries in parallel where `cargo test` runs them one after
# another (docs/PLANNING.md §16.119); it does not run doc tests, so those
# follow. `cargo install cargo-nextest --locked` puts it on a machine.
test:
    cargo nextest run --workspace
    cargo test --workspace --doc

# Clippy with warnings as errors, formatting check, the secret-hygiene
# lint, the user-facing-strings lint and the design-token lint.
lint:
    cargo clippy --workspace --all-targets -- -D warnings
    # Advisories, licences, duplicates and sources, against deny.toml.
    # Commented out until `cargo install cargo-deny` is part of the
    # documented setup, so that `just` still runs on a machine without it.
    # cargo deny check
    cargo fmt --all --check
    tools/lint-secrets.sh
    tools/lint-strings.sh
    tools/lint-tokens.sh
    python3 tools/learn/sync.py check

# BIP 445's reference implementation as the second share of the
# threshold fixture spend (docs/PLANNING.md §16.103). Not part of the
# default `just`, because it is Python and the default recipe is Rust;
# the reference and its curve library are vendored under
# tools/reference/bip445/ and nothing is fetched.
threshold-reference:
    python3 tools/scripts/threshold-reference.py

# Regenerate the embedded BIP-39 wordlists from the BIP repository and
# the SLIP-39 list from the SLIPs repository (needs network).
wordlists:
    python3 tools/wordlists/gen.py
    python3 tools/wordlists/gen-slip39.py
    cargo fmt --all

# Regenerate the embedded EFF diceware wordlists from the lists committed
# under tools/vectors/eff/ (offline).
diceware:
    python3 tools/diceware/gen.py
    cargo fmt --all

# Re-extract the glyph outlines from tools/fonts/ into core/osk-ui (offline).
fonts:
    cargo run -p osk-fontbake --release
    cargo fmt --all

# Regenerate the QR fixtures the scan-routing script reads (offline).
qr-fixtures:
    cargo run -p osk-qrgen --release -- tools/fixtures/qr

# Rewrite the one PSBT per policy warning, and the registered-wallet
# policy and transaction, in tools/vectors/psbt/ (offline).
psbt-fixtures:
    cargo run -p osk-psbt --release --example warnings -- tools/vectors/psbt
    cargo run -p osk-psbt --release --example wallet -- tools/vectors/psbt
    cargo run -p osk-psbt --release --example musig -- tools/vectors/psbt
    cargo run -p osk-psbt --release --example threshold -- tools/vectors/psbt
    cargo run -p osk-psbt --release --example tapmulti -- tools/vectors/psbt

# Render the OpenSigner review scripts and every gallery page at the four
# reference sizes into out/snapshots/<WxH>/: the smallest supported panel,
# the 2.8" reference panel, a phone, and the desktop window, whose size is
# fixed. Small panels are portrait (UX.md §2).
# The phone reports the 63 px (24 dp at 420 dpi) its gesture bar takes,
# which is `PHONE_INSET` in opensigner/shells/snapshot/src/main.rs; the
# panels and the window have no such strip and report none.
snapshots: (snapshot-size "240x320" "143") (snapshot-size "480x640" "286") (snapshot-size "1080x2340" "420" "63") (snapshot-size "960x640" "160")

# One script at one size, for looking at the screen a change touched:
# `just snap load-key 480x640`. The sizes are the four `snapshots` renders,
# named by their width: 240x320 @ 143 dpi, 480x640 @ 286, 1080x2340 @ 420
# with the phone's 63 px inset, 960x640 @ 160. Output goes to
# out/snapshots/<size>/ like the full run. The dev profile is used here and
# in `snapshots`, because it is the build the tests already made and the
# renderer is optimised in it (Cargo.toml's dev profile); the release
# profile's fat LTO cost 38 s of rebuild per change for nothing.
snap script size:
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi `case {{size}} in 240x320) echo 143;; 480x640) echo 286;; 1080x2340) echo 420;; 960x640) echo 160;; *) echo 160;; esac` --inset-bottom `case {{size}} in 1080x2340) echo 63;; *) echo 0;; esac` --out out/snapshots/{{size}} --script tools/scripts/{{script}}.txt

# Every review script at the four reference sizes, with the snapshot
# tool's --audit on: after each script line it asks the app what the frame
# on screen does wrong a person would see — text or icons drawn over each
# other, a fingerprint, checksum, address, path or descriptor in a text
# face rather than the mono one, text cut at the side of its clip — and
# prints each fault once per script and size. Renders go to out/audit/.
# Not part of `just`: it renders every script four times.
audit:
    #!/usr/bin/env bash
    set -uo pipefail
    cargo build -q -p opensigner-snapshot
    mkdir -p out/audit
    : > out/audit/screens.txt
    status=0
    for spec in "240x320 143 0" "480x640 286 0" "1080x2340 420 63" "960x640 160 0"; do
        read -r size dpi inset <<< "$spec"
        for script in tools/scripts/*.txt; do
            name=$(basename "$script" .txt)
            out=$(target/debug/opensigner-snapshot --size "$size" --dpi "$dpi" --inset-bottom "$inset" --out "out/audit/$size" --script "$script" --audit 2>&1)
            if [ $? -ne 0 ]; then
                echo "$size $name: script failed: $(echo "$out" | grep '^error' | head -1)"
                status=1
            fi
            echo "$out" | grep '^audit: ' | sed "s|^audit: |$size $name: |"
            echo "$out" | grep '^audited: ' | sed 's|^audited: ||' >> out/audit/screens.txt
        done
    done
    # The kinds of screen no script reached, which the audit has not seen.
    all=$(awk '/^pub enum ScreenKind/,/^}/' opensigner/opensigner-core/src/lib.rs | grep -oE '^    [A-Z][A-Za-z0-9]*' | tr -d ' ')
    missed=$(comm -23 <(echo "$all" | sort -u) <(sort -u out/audit/screens.txt) | tr '\n' ' ')
    echo "not reached by any script: ${missed:-none}"
    exit $status

snapshot-size size dpi inset_bottom="0":
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/load-key.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/load-key-ja.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/load-key-ko.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/load-key-zh-hans.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/load-key-zh-hant.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/load-numbers.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/load-hex.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/key-detail.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/sign-psbt.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/sign-warnings.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/message.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/create-key.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/scan-routing.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/files.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/wallet.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/wallet-xpub.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/wallet-config.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/wallet-musig.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/wallet-miniscript.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/wallet-build.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/export-qr.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/session.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/explore.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/tools-wordlist.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/tools-dice.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/tools-more.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/learn.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/first-run.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/key-more.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/keep.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/notes.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/silent.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/compare-tx.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}} --script tools/scripts/boot.txt
    cargo run -p opensigner-snapshot -- --size {{size}} --dpi {{dpi}} --inset-bottom {{inset_bottom}} --out out/snapshots/{{size}}/gallery --app gallery

# --- Android (opensigner/shells/android) ------------------------------------
#
# Two environment variables, and nothing hard-coded:
#
#   ANDROID_HOME       the SDK root (platform-tools, platforms, build-tools,
#                      emulator)
#   ANDROID_NDK_HOME   one NDK, usually $ANDROID_HOME/ndk/<version>
#
# Also needed: a JDK 21, `cargo ndk` (`cargo install cargo-ndk`) and the
# Rust targets `aarch64-linux-android` and `x86_64-linux-android`. Gradle
# itself comes from the wrapper committed in the shell directory.
#
#   just android-lib   cross-compile libopensigner.so into the app's jniLibs
#   just android-apk   the debug APK (rebuilds the library first); a copy
#                      always lands at out/android/opensigner-debug.apk
#   just android-release-apk  the release APK, unsigned, built in the pinned
#                      container instead of on this machine (tools/build/)
#   just android-run   boot the emulator if it is not up, install, launch,
#                      and snap the screen into out/android/
#
# The app sets FLAG_SECURE, so `adb exec-out screencap` returns black. The
# screenshot goes through the emulator console instead, which reads the
# host framebuffer (tools/android-screenshot.py).

android_dir := "opensigner/shells/android"
android_apk := "opensigner/shells/android/app/build/outputs/apk/debug/app-debug.apk"
android_avd := env_var_or_default("OPENSIGNER_AVD", "headless")

# The core, cross-compiled for a phone (arm64-v8a) and the emulator (x86_64).
android-lib:
    cargo ndk -t arm64-v8a -t x86_64 -P 26 \
        -o {{android_dir}}/app/src/main/jniLibs \
        build --release -p opensigner-ffi

# The debug APK, built with whatever toolchain this machine has, which is
# what development wants; `just android-release-apk` is the pinned build a
# second builder reproduces. Always a clean Gradle build: the incremental packager can leave the old
# native library as dead space inside the archive (6.8 MB once), and the APK
# is copied to a stable path the owner fetches by size.
android-apk: android-lib
    cd {{android_dir}} && ./gradlew clean assembleDebug
    mkdir -p out/android
    cp {{android_dir}}/app/build/outputs/apk/debug/app-debug.apk out/android/opensigner-debug.apk

# Install on the emulator (starting it if needed), launch, and screenshot.
android-run: android-apk
    #!/usr/bin/env bash
    set -euo pipefail
    : "${ANDROID_HOME:?set ANDROID_HOME to the Android SDK root}"
    adb="$ANDROID_HOME/platform-tools/adb"
    if ! "$adb" devices | grep -qE '^emulator-[0-9]+[[:space:]]+device$'; then
        nohup "$ANDROID_HOME/emulator/emulator" -avd "{{android_avd}}" \
            -no-window -gpu swiftshader_indirect -no-audio -no-boot-anim \
            >/dev/null 2>&1 &
        disown
        "$adb" wait-for-device
        until [ "$("$adb" shell getprop sys.boot_completed | tr -d '\r')" = "1" ]; do
            sleep 2
        done
    fi
    "$adb" install -r "{{android_apk}}"
    "$adb" shell am start -n app.opensigner/.MainActivity
    sleep 5
    tools/android-screenshot.py out/android/home.png

# --- Raspberry Pi (opensigner/shells/pi) ----------------------------------
#
# The shell is built as a static 32-bit ARM binary. Each board directory's
# board.conf names the Rust target and the C cross-compiler it wants; the
# two boards here both want `armv7-unknown-linux-gnueabihf` (`rustup target
# add ...`) and `gcc-arm-linux-gnueabihf` from the distribution, which the
# libsecp256k1 sources `secp256k1-sys` compiles need. Set OPENSIGNER_PI_CC
# to override the compiler.
#
#   just pi-bin     the static ARM binary, at out/pi/<board>/opensigner-pi
#   just pi-image   a bootable card image for one board, panel and variant
#
# The image build lives in this repository and depends on nothing outside
# it: opensigner/shells/pi/image, described in that directory's README.

# Buildroot's long-term-support series, pinned to a release tag and to the
# commit that tag pointed at when it was taken. The tag is what a person
# reads; the commit is what the build checks, because a tag can be moved
# and a commit cannot. run-build.sh clones at the tag and then fails if
# HEAD is not this, printing both hashes.
buildroot_tag := "2024.02.13"
buildroot_commit := "e687e3815f76c1c5ea9fb52b6558bedfe53ab117"

# Which image to build. These are `just` variables, so they are set on the
# command line before the recipe name:
#
#   just pi-image                              the owner's device
#   just dev=1 pi-image                        the same, with a serial login
#   just board=pi3 panel=waveshare-28dpi dev=0 pi-image
#
# board and panel name directories under opensigner/shells/pi/image.
board := "pi3"
panel := "waveshare-28dpi"
dev := "0"

# opensigner-pi for one board, statically linked against glibc so the
# initramfs needs no libraries at all. The board directory's board.conf
# carries the toolchain — RUST_TARGET, CC and the CFLAGS the libsecp256k1
# sources want — so a new board is a directory and not a change here.
#
#   just pi-bin             the owner's board
#   just board=NAME pi-bin  any other, from image/boards/
#
# The binary lands at out/pi/<board>/opensigner-pi.
pi-bin:
    #!/usr/bin/env bash
    set -euo pipefail
    conf="opensigner/shells/pi/image/boards/{{board}}/board.conf"
    if [ ! -f "$conf" ]; then
        echo "no such board: {{board}}" >&2
        exit 1
    fi
    . "$conf"
    cc="${OPENSIGNER_PI_CC:-$CC}"
    # cargo and the cc crate name their per-target variables after the
    # target triple: underscores for the environment, upper case for
    # cargo's own.
    under="${RUST_TARGET//-/_}"
    upper="$(echo "$under" | tr 'a-z' 'A-Z')"
    export "CC_${under}=$cc"
    export "AR_${under}=${cc%gcc}ar"
    export "CARGO_TARGET_${upper}_LINKER=$cc"
    if [ -n "${CFLAGS:-}" ]; then
        export "CFLAGS_${under}=$CFLAGS"
    fi
    export RUSTFLAGS="-C target-feature=+crt-static"
    cargo build --release --target "$RUST_TARGET" -p opensigner-pi
    mkdir -p "out/pi/{{board}}"
    cp "target/$RUST_TARGET/release/opensigner-pi" "out/pi/{{board}}/opensigner-pi"
    file "out/pi/{{board}}/opensigner-pi"
    "${cc%gcc}readelf" -hd "out/pi/{{board}}/opensigner-pi" | \
        grep -E 'Class|Machine|Type:|NEEDED|Flags' || true

# The bootable microSD image: Buildroot in a container, the binary from
# pi-bin installed into an initramfs, one board and one panel configured.
# Everything lands under out/pi/.
#
#   just pi-image                              the owner's device
#   just dev=1 pi-image                        the same, with a serial login
#   just board=B panel=P dev=0|1 pi-image      any other combination
#
# The three are `just` variables, so the assignments come before the
# recipe name. dev=1 adds variants/dev, which is for bringing a new board
# or panel up and never goes on a card that signs anything. Each combination has its own build
# tree at out/pi/<board>-<panel>[-dev]/output and its own image at
# out/pi/opensigner-pi-<board>-<panel>[-dev].img; the Buildroot checkout,
# the download cache and the compiler cache are shared.
#
# It needs Docker. Rootless is what this is written for: point DOCKER_HOST
# at the socket in your own user's runtime directory, and container root is
# you, so every file in out/pi/ comes out owned by you. With a system Docker
# daemon the build works but leaves root-owned files behind.
#
# The first run clones Buildroot at the pinned tag and downloads a few
# hundred megabytes of sources; it takes 30 to 60 minutes. Later runs reuse
# the checkout and the caches and are incremental. Delete a combination's
# output directory to start its build over.
pi-image: pi-bin
    #!/usr/bin/env bash
    set -euo pipefail
    variant="{{board}}-{{panel}}"
    if [ "{{dev}}" != "0" ]; then
        variant="$variant-dev"
    fi
    opensigner/shells/pi/image/run-build.sh \
        "{{board}}" "{{panel}}" "{{dev}}" "{{buildroot_tag}}" "{{buildroot_commit}}"
    cp "out/pi/$variant/output/images/opensigner-pi.img" \
        "out/pi/opensigner-pi-$variant.img"
    ls -l "out/pi/opensigner-pi-$variant.img"

# --- The USB stick (opensigner/shells/pi, board x86_64-uefi) --------------
#
# The same shell, the same image tree and the same Buildroot, built for a
# laptop instead of a single-board computer: a GPT stick whose EFI system
# partition holds one file, the kernel, which is its own EFI application
# with the root filesystem inside it. No bootloader, no firmware blob, no
# modules, no network stack. `opensigner/shells/pi/image/README.md`
# describes the board; `docs/PLANNING.md` §16.95 says why each choice.
#
#   just stick-bin    the static x86_64 binary
#   just stick-image  out/stick/opensigner-x86_64-uefi.img
#   just stick-qemu   boot it in QEMU with OVMF and KVM, on VNC :9
#   just stick-test   boot it headless and prove it reaches the app
#
# dev=1 on any of them is the dev variant: a console and a login on
# ttyS0, and the shell started with --verbose --timings.

# The board and the panel the stick recipes build. They are `just`
# variables so that a second x86 board or a second way of finding the
# display does not need a new recipe.
stick_board := "x86_64-uefi"
stick_panel := "efi-framebuffer"

# opensigner-pi for x86_64, statically linked against musl. This is
# `just board=x86_64-uefi pi-bin` and nothing more: the board directory's
# board.conf carries the Rust target and the C compiler, so the recipe
# that builds the ARM binary builds this one too.
#
# The binary lands at out/pi/x86_64-uefi/opensigner-pi. It needs
# `rustup target add x86_64-unknown-linux-musl`; the host's own gcc
# compiles the libsecp256k1 sources.
stick-bin:
    just board="{{stick_board}}" pi-bin

# The bootable USB stick image, at out/stick/opensigner-x86_64-uefi.img.
#
#   just stick-image          the release image
#   just dev=1 stick-image    the same, with a console and a login on ttyS0
#
# Everything under out/pi/ is shared with the Pi images: the Buildroot
# checkout, the download cache, the compiler cache and one build tree per
# combination. Only the finished image is copied to out/stick/.
stick-image: stick-bin
    #!/usr/bin/env bash
    set -euo pipefail
    variant="{{stick_board}}-{{stick_panel}}"
    name="opensigner-x86_64-uefi"
    if [ "{{dev}}" != "0" ]; then
        variant="$variant-dev"
        name="$name-dev"
    fi
    opensigner/shells/pi/image/run-build.sh \
        "{{stick_board}}" "{{stick_panel}}" "{{dev}}" "{{buildroot_tag}}" \
        "{{buildroot_commit}}"
    mkdir -p out/stick
    cp "out/pi/$variant/output/images/opensigner-x86_64-uefi.img" \
        "out/stick/$name.img"
    ls -l "out/stick/$name.img"

# Boot the stick image in QEMU, as a USB disk on an xHCI controller with
# OVMF as the firmware — the same path a laptop takes, rather than a
# virtual hard disk the firmware finds another way.
#
#   just stick-qemu           the release image
#   just dev=1 stick-qemu     the dev image, whose console is on stdio
#
# The display is on VNC, port 5909. To see it from another machine on the
# tailnet, forward the port over SSH; the image README says how.
# Ctrl-A x, or closing the VNC client and killing the process, ends it.
#
# To try the file channel with a second stick, run tools/stick-qemu.py
# directly and give it `--usb IMAGE`: any raw disk image with a FAT
# partition on it is attached as a second USB disk.
stick-qemu:
    #!/usr/bin/env bash
    set -euo pipefail
    name="opensigner-x86_64-uefi"
    if [ "{{dev}}" != "0" ]; then
        name="$name-dev"
    fi
    tools/stick-qemu.py --image "out/stick/$name.img" --vnc 9 --serial stdio

# Boot the stick image headless and prove it reaches the app and takes
# input: a screen that is not black and carries the action bar's orange, a
# mouse click on the first Home row that changes the frame, and Esc that
# changes it back. Then a second USB stick is plugged in while the app
# runs. The screendumps are written under out/stick/test/.
#
#   just stick-test           the release image
#   just dev=1 stick-test     the dev image, which says on ttyS0 when the
#                             shell starts, so the test waits for that
#                             line instead of a fixed time, and which says
#                             on the same console what init mounted when
#                             the second stick arrived — the release image
#                             prints nothing, so only this run checks the
#                             mount
stick-test:
    #!/usr/bin/env bash
    set -euo pipefail
    name="opensigner-x86_64-uefi"
    out="out/stick/test"
    dev_arg=""
    if [ "{{dev}}" != "0" ]; then
        name="$name-dev"
        out="$out-dev"
        dev_arg="--dev"
    fi
    tools/stick-test.py --image "out/stick/$name.img" --out "$out" $dev_arg

# --- macOS (opensigner/shells/desktop + opensigner/shells/avfoundation) -----
#
# The camera needs an application bundle: macOS grants a camera permission
# to a bundle with an NSCameraUsageDescription and to nothing else, so a
# bare binary run from Terminal inherits Terminal's permission instead,
# which may or may not have been granted.
#
#   just mac-app   out/mac/OpenSigner.app, ad-hoc signed
#
# The bundle is ad-hoc signed and not notarised (§16.33: no Apple Developer
# Program, since it publishes a legal identity and this project is
# anonymous), so Gatekeeper refuses a double-click. Open it the documented
# way instead: right-click the app, choose Open, then Open again in the
# dialog. That is needed once per copy.
#
# Runs on a Mac only: the Objective-C in `opensigner-avfoundation` needs
# the macOS SDK, and there is no cross-build.

mac_app := "out/mac/OpenSigner.app"

# The desktop shell as a macOS application bundle, ad-hoc signed.
mac-app:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ "$(uname)" != "Darwin" ]; then
        echo "just mac-app builds a macOS bundle and runs on macOS only" >&2
        exit 1
    fi
    version=$(grep -m1 '^version = ' opensigner/shells/desktop/Cargo.toml | cut -d'"' -f2)
    cargo build --release -p opensigner-desktop
    mkdir -p "{{mac_app}}/Contents/MacOS" "{{mac_app}}/Contents/Resources"
    cp -f target/release/opensigner-desktop "{{mac_app}}/Contents/MacOS/opensigner-desktop"
    cat > "{{mac_app}}/Contents/Info.plist" <<PLIST
    <?xml version="1.0" encoding="UTF-8"?>
    <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
    <plist version="1.0">
    <dict>
    	<key>CFBundleExecutable</key>
    	<string>opensigner-desktop</string>
    	<key>CFBundleIdentifier</key>
    	<string>app.opensigner.desktop</string>
    	<key>CFBundleName</key>
    	<string>OpenSigner</string>
    	<key>CFBundlePackageType</key>
    	<string>APPL</string>
    	<key>CFBundleShortVersionString</key>
    	<string>$version</string>
    	<key>LSMinimumSystemVersion</key>
    	<string>11.0</string>
    	<key>NSHighResolutionCapable</key>
    	<true/>
    	<key>NSCameraUsageDescription</key>
    	<string>Reads QR codes: transactions, seeds and addresses. Nothing leaves the device.</string>
    </dict>
    </plist>
    PLIST
    codesign --force --sign - --deep "{{mac_app}}"
    echo "{{mac_app}}"

# --- Reproducible builds (tools/build/) -------------------------------------
#
# A release is blocked unless two independent builders reproduce every
# artifact (docs/PLANNING.md §11.1), which they can only do if the build
# depends on the source and on nothing else about the machine. So every
# release artifact comes out of a container whose base image is pinned by
# digest and whose toolchains are pinned by version:
#
#   just linux-bin            out/linux/opensigner-desktop, static musl
#   just android-release-apk  out/android/release/, the unsigned APK
#   just reproduce            both of those twice, from clean, and diffed
#
# The Pi image already builds this way (`just pi-image`, Buildroot at a
# pinned tag in a pinned container) and `just reproduce pi=1` covers it.
#
# The same rootless Docker: point DOCKER_HOST at your own runtime
# directory's socket, and every file written into out/ belongs to you.
# The first run of each pulls a base image and installs a toolchain.
#
# SOURCE_DATE_EPOCH is the commit date, so what a builder gets depends on
# the commit they checked out and not on the day they built it.

docker := env_var_or_default("DOCKER", "docker")
source_date_epoch := `git log -1 --format=%ct 2>/dev/null || echo 0`

# Where the containers keep their caches: the cargo registry, the cargo
# target directories and Gradle's home. Nothing here is an artifact.
build_cache := "out/build"

# The Linux desktop binary, static against musl, in the container.
#
# OUT is the directory the binary lands in and CACHE the build tree, both
# named so that `just reproduce` can give each of its two runs its own.
linux-bin out="out/linux" cache="out/build/run":
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
        opensigner-build-linux /src/tools/build/linux/build.sh
    file "{{out}}/opensigner-desktop"
    sha256sum "{{out}}/opensigner-desktop"

# The release APK, unsigned, in the container, with the two libraries it
# packages beside it.
#
# Signing is the owner's key story and is not done here (§15 item 33):
# what this produces is the file a second builder compares against the
# published APK with its signing block stripped (docs/VERIFY.md).
android-release-apk out="out/android/release" cache="out/build/run":
    #!/usr/bin/env bash
    set -euo pipefail
    root="$(pwd)"
    mkdir -p "{{out}}" "{{cache}}/android" "{{cache}}/work" \
        "{{build_cache}}/cargo" "{{build_cache}}/gradle"
    {{docker}} build -t opensigner-build-android tools/build/android
    {{docker}} run --rm \
        -e SOURCE_DATE_EPOCH="{{source_date_epoch}}" \
        -v "$root":/src:ro \
        -v "$root/{{out}}":/out \
        -v "$root/{{cache}}/android":/build \
        -v "$root/{{cache}}/work":/work \
        -v "$root/{{build_cache}}/cargo":/cargo \
        -v "$root/{{build_cache}}/gradle":/gradle \
        opensigner-build-android /src/tools/build/android/build.sh
    sha256sum "{{out}}/opensigner-release-unsigned.apk" \
        "{{out}}"/jniLibs/*/libopensigner.so

# Build every release artifact twice, from clean, and diff the results.
#
# This is the box checking itself: two clean builds of one commit that
# come out bit for bit the same are the precondition for two people on two
# machines reproducing a release, not a substitute for it (§11.2).
#
# The Pi image is off by default because Buildroot builds it from source:
# each of the two runs is 30 to 60 minutes and a few gigabytes, against a
# couple of minutes for the other two. Ask for it before a release:
#
#   just reproduce
#   just pi=1 reproduce
pi := "0"
reproduce:
    #!/usr/bin/env bash
    set -euo pipefail
    root="$(pwd)"
    status=0

    # Each run gets its own output directory and its own build tree, so
    # the second build shares nothing with the first except the caches
    # that only hold downloads.
    for run in 1 2; do
        echo "== run $run =================================================="
        just linux-bin "out/reproduce/$run/linux" "out/build/reproduce-$run"
        just android-release-apk "out/reproduce/$run/android" \
            "out/build/reproduce-$run"
        if [ "{{pi}}" != "0" ]; then
            variant="{{board}}-{{panel}}"
            just board="{{board}}" panel="{{panel}}" dev=0 pi-image
            mkdir -p "out/reproduce/$run/pi"
            cp -f "out/pi/opensigner-pi-$variant.img" \
                "out/reproduce/$run/pi/opensigner-pi-$variant.img"
            # Moved aside rather than reused: a Buildroot tree left in
            # place would make run 2 an incremental build of run 1, which
            # proves nothing.
            mv "out/pi/$variant" "out/build/reproduce-$run-pi-$variant"
        fi
    done

    echo "== compared ==================================================="
    compare() {
        local name="$1" a b
        a="$(sha256sum "out/reproduce/1/$name" | cut -d' ' -f1)"
        b="$(sha256sum "out/reproduce/2/$name" | cut -d' ' -f1)"
        if [ "$a" = "$b" ]; then
            printf '%s\n  %s  both runs\n' "$name" "$a"
        else
            printf '%s\n  %s  run 1\n  %s  run 2  DIFFERENT\n' "$name" "$a" "$b"
            status=1
        fi
    }

    compare linux/opensigner-desktop
    compare android/opensigner-release-unsigned.apk
    for abi in arm64-v8a x86_64; do
        compare "android/jniLibs/$abi/libopensigner.so"
    done
    if [ "{{pi}}" != "0" ]; then
        compare "pi/opensigner-pi-{{board}}-{{panel}}.img"
    fi

    if [ "$status" -ne 0 ]; then
        echo "just reproduce: an artifact did not reproduce" >&2
        exit 1
    fi
    echo "every artifact reproduced"

# --- Releases (tools/release/, docs/VERIFY.md) ------------------------------
#
# A release is a set of artifacts in one directory, a manifest of their
# SHA-256 sums, and a detached GPG signature over that manifest. Users
# check it with the steps in docs/VERIFY.md; the artifacts and the
# manifest with its signature go on the GitHub release page.
#
#   just release 0.1.0        build this box's artifacts and write the manifest
#   just release-sign 0.1.0   sign the manifest and verify the signature
#
# What this box builds, each of them in a pinned container so that a
# second builder can reproduce it: the Pi card image for the release board
# and panel, the unsigned Android APK and the Linux desktop binary. The
# macOS bundle is built on a Mac with `just mac-app`, zipped, and copied
# into the release directory by hand; `just release` includes any file
# already sitting there, so build it, drop the zip in, and run
# `just release` again to remake the manifest.

release_root := "out/release"
release_board := "pi3"
release_panel := "waveshare-28dpi"

# Build the release artifacts for VERSION and write the manifest.
release VERSION:
    #!/usr/bin/env bash
    set -euo pipefail
    version="{{VERSION}}"
    dir="{{release_root}}/$version"

    # A release is built from a committed tree, so that the commit in the
    # manifest names the source the artifacts came from.
    if [ -n "$(git status --porcelain)" ]; then
        echo "just release: the tree is dirty; commit or stash first" >&2
        git status --short >&2
        exit 1
    fi

    # The version is the one every crate in the workspace carries.
    workspace_version="$(git ls-files '*Cargo.toml' | xargs grep -h -m1 '^version = ' | cut -d'"' -f2 | sort -u)"
    if [ "$(printf '%s\n' "$workspace_version" | wc -l)" -ne 1 ]; then
        echo "just release: the workspace crates disagree on a version:" >&2
        printf '%s\n' "$workspace_version" >&2
        exit 1
    fi
    if [ "$version" != "$workspace_version" ]; then
        echo "just release: $version is not the workspace version ($workspace_version)" >&2
        exit 1
    fi

    commit="$(git rev-parse HEAD)"
    mkdir -p "$dir"

    just board="{{release_board}}" panel="{{release_panel}}" dev=0 pi-image
    just android-release-apk
    just linux-bin

    cp -f "out/pi/opensigner-pi-{{release_board}}-{{release_panel}}.img" \
        "$dir/opensigner-$version-{{release_board}}-{{release_panel}}.img"
    cp -f out/android/release/opensigner-release-unsigned.apk \
        "$dir/opensigner-$version-android-unsigned.apk"
    cp -f out/linux/opensigner-desktop \
        "$dir/opensigner-$version-linux-x86_64"

    # One line per file in the directory, in name order, so that anything
    # added by hand -- the macOS bundle built on a Mac -- is covered too.
    (
        cd "$dir"
        {
            echo "# OpenSigner $version, commit $commit"
            find . -maxdepth 1 -type f \
                ! -name manifest.txt ! -name manifest.txt.asc \
                -printf '%P\n' | LC_ALL=C sort | xargs -r sha256sum
        } > manifest.txt
        cat manifest.txt
        echo
        sha256sum -c manifest.txt
    )
    echo "$dir"

# Sign the manifest for VERSION and verify the signature that comes back.
#
# `opensigner-release-sign` is a command on PATH and not part of this
# repository. Its contract: given the path of a manifest, it writes a
# detached ASCII-armoured signature `manifest.txt.asc` beside it, made
# with the release key, and exits non-zero if it cannot. How it reaches
# the key is the machine's business; no session holds it.
release-sign VERSION:
    #!/usr/bin/env bash
    set -euo pipefail
    manifest="{{release_root}}/{{VERSION}}/manifest.txt"
    pubkey="tools/release/pubkey.asc"

    if [ ! -f "$manifest" ]; then
        echo "just release-sign: no $manifest; run just release {{VERSION}} first" >&2
        exit 1
    fi
    if [ ! -f "$pubkey" ]; then
        echo "just release-sign: no $pubkey; export the release public key there (tools/release/README.md)" >&2
        exit 1
    fi
    if ! command -v opensigner-release-sign >/dev/null; then
        echo "just release-sign: opensigner-release-sign is not on PATH" >&2
        exit 1
    fi

    opensigner-release-sign "$manifest"
    if [ ! -f "$manifest.asc" ]; then
        echo "just release-sign: opensigner-release-sign wrote no $manifest.asc" >&2
        exit 1
    fi

    gpg --import "$pubkey"
    gpg --verify "$manifest.asc" "$manifest"
    echo "$manifest.asc"

# --- Fuzzing (fuzz/, docs/PLANNING.md §12) --------------------------------
#
# Local only: no hosted CI runs this, and no release depends on it. It
# needs a nightly toolchain and cargo-fuzz, both at user level:
#
#   rustup toolchain install nightly --profile minimal
#   cargo install cargo-fuzz --locked
#
#   just fuzz-list             the targets
#   just fuzz classify         one target for the default time
#   just fuzz decode_luma 300  one target for 300 seconds
#
# The fuzz crate is its own workspace (the root Cargo.toml excludes it),
# so a sanitizer build never shares this workspace's target directory or
# lock file.

fuzz_seconds := "60"

# The fuzz targets, one per untrusted-input parser.
fuzz-list:
    cd fuzz && cargo +nightly fuzz list

# Run one target. SECONDS defaults to fuzz_seconds; give decode_luma and
# psbt_parse longer, since they are the widest surfaces.
fuzz TARGET seconds=fuzz_seconds:
    cd fuzz && cargo +nightly fuzz run {{TARGET}} -- -max_total_time={{seconds}}

# The Learn pages as Markdown: docs/learn/ is where the text is edited,
# en.rs is what the device reads; `lint` fails while the two differ.
learn-export:
    python3 tools/learn/sync.py export

learn-import:
    python3 tools/learn/sync.py import
    cargo fmt --all

# Faraday's own recipes: faraday/faraday.just.
import? 'faraday/faraday.just'
