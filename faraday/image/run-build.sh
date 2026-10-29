#!/usr/bin/env bash
# Builds a Faraday image, the stick's or a Pi card's: this directory's
# image tree (build.sh and the boards, panels and variants it combines)
# in the pinned Buildroot, in a container. faraday.just's
# faraday-stick-image and faraday-pi-image run it after building the
# binaries into out/faraday/<board>/. The Buildroot checkout, downloads,
# compiler cache and build trees are under out/pi/, so only what changed
# is rebuilt.
#
#   run-build.sh <board> <panel> <dev> <buildroot tag> <buildroot commit> [<build>]
#
# <build> is strict or compatible, a directory under the board's builds/; the
# build tree is out/pi/<board>-<panel>-<build>[-dev].
set -euo pipefail
board="${1:?}"
panel="${2:?}"
dev="${3:?}"
tag="${4:?}"
commit="${5:?}"
build="${6:-}"
root="$(pwd)"
image="$root/faraday/image"
binaries="$root/out/faraday/$board"
[ -d "$image/boards/$board" ] || { echo "no such board: $board" >&2; exit 1; }
for b in faraday faraday-disk faraday-grant; do
    [ -f "$binaries/$b" ] || { echo "$binaries/$b: not built" >&2; exit 1; }
done

# The FAT filesystems' volume serials, from the commit date and each
# partition's role, so two builds of one commit have the same ones
# (fat-serial.sh). build.sh puts them where the board's genimage.cfg
# says @FAT_SERIAL_BOOT@ and @FAT_SERIAL_DATA@.
epoch="${SOURCE_DATE_EPOCH:-$(git -C "$root" log -1 --format=%ct 2>/dev/null || echo 0)}"
serial_boot="$("$image/fat-serial.sh" boot "$epoch")"
serial_data="$("$image/fat-serial.sh" data "$epoch")"
echo "FAT serials: boot $serial_boot, data $serial_data (SOURCE_DATE_EPOCH $epoch)"

variant="$board-$panel"
if [ -n "$build" ]; then
    variant="$variant-$build"
fi
if [ "$dev" != "0" ]; then
    variant="$variant-dev"
fi

# The kernel's release string carries the Faraday build ID (docs/DECISIONS.md
# F5): a release's version alone, otherwise the version, the commit and the
# kind (test, the default, or dev), with `.dirty-XXXXXXXX` before the kind
# when the tree differs from the commit. The Starter reads a stick's
# version from the one its boot partition holds. It comes from the commit,
# never the clock, so two builders get the same bytes. build.sh writes it
# into a kernel fragment of its own.
localversion="-faraday-$(FARADAY_DEV="$dev" "$image/build-id.sh")"
echo "kernel release: <version>$localversion"
mkdir -p out/pi/dl out/pi/ccache "out/pi/$variant/output"
if [ ! -d out/pi/buildroot/.git ]; then
    git clone --depth 1 --branch "$tag" \
        https://gitlab.com/buildroot.org/buildroot.git out/pi/buildroot
fi
head="$(git -C out/pi/buildroot rev-parse HEAD)"
if [ "$head" != "$commit" ]; then
    echo "out/pi/buildroot is not the pinned Buildroot ($commit)" >&2
    exit 1
fi
# The pinned commit with something changed or added in the tree is not the
# pinned Buildroot either.
if ! tree_state="$(git -C out/pi/buildroot status --porcelain)"; then
    echo "out/pi/buildroot: git cannot read its state" >&2
    exit 1
fi
if [ -n "$tree_state" ]; then
    echo "out/pi/buildroot is the pinned Buildroot ($commit) with changes:" >&2
    echo "$tree_state" | head -20 >&2
    exit 1
fi
"${DOCKER:-docker}" build -t faraday-image "$image"
# /binary holds the app (faraday) and the disk process and the grant
# helper (docs/history/PLAN.md §4.3) beside it.
in_container() {
    "${DOCKER:-docker}" run --rm \
        -e BOARD="$board" -e PANEL="$panel" -e DEV="$dev" -e BUILD="$build" \
        -e FAT_SERIAL_BOOT="$serial_boot" -e FAT_SERIAL_DATA="$serial_data" \
        -e FARADAY_LOCALVERSION="$localversion" \
        -v "$root/out/pi/buildroot":/buildroot \
        -v "$root/out/pi/$variant/output":/output \
        -v "$root/out/pi/dl":/dl \
        -v "$root/out/pi/ccache":/ccache \
        -v "$binaries":/binary:ro \
        -v "$image":/image:ro \
        faraday-image "$@"
}

# Buildroot unpacks the kernel once per build tree, and builds the
# toolchain against that kernel's headers, and notices neither a new
# tarball nor new patches afterwards. The board's kernel source is its
# tarball, the tarball's hash and its patches (docs/DECISIONS.md F23);
# when that changes, the build tree is cleaned first. The downloads and
# the compiler cache stay. A tree built before this stamp existed is
# judged by the tarball and patches its .config names.
board_fragment="$image/boards/$board/buildroot.fragment"
kernel_lines() {
    grep -E '^(BR2_LINUX_KERNEL_CUSTOM_TARBALL_LOCATION|BR2_LINUX_KERNEL_PATCH)=' "$1" | sort || true
}
# A build (strict or compatible) may add patches of its own (none does:
# Faraday carries no kernel patch of its own on the Pi, F153); they would
# count too, and a build without any hashes as the board alone does.
build_dir="$image/boards/$board/builds/$build"
kernel_source="$({
    kernel_lines "$board_fragment"
    grep -E '^# KERNEL_SHA256 ' "$board_fragment" || true
    cat "$image/boards/$board/linux-patches/"*.patch 2>/dev/null || true
    if [ -n "$build" ] && [ -f "$build_dir/buildroot.fragment" ]; then
        kernel_lines "$build_dir/buildroot.fragment"
    fi
    if [ -n "$build" ]; then
        cat "$build_dir/linux-patches/"*.patch 2>/dev/null || true
    fi
} | sha256sum | cut -d' ' -f1)"
source_stamp="$root/out/pi/$variant/kernel-source"
config="$root/out/pi/$variant/output/.config"
changed=no
if [ -f "$source_stamp" ]; then
    [ "$(cat "$source_stamp")" = "$kernel_source" ] || changed=yes
elif [ -f "$config" ]; then
    # .config writes BR2_LINUX_KERNEL_PATCH="" where a fragment says
    # nothing.
    old="$(kernel_lines "$config" | grep -v '^BR2_LINUX_KERNEL_PATCH=""$' || true)"
    new="$({
        kernel_lines "$board_fragment"
        if [ -n "$build" ] && [ -f "$build_dir/buildroot.fragment" ]; then
            kernel_lines "$build_dir/buildroot.fragment"
        fi
    } | grep -v '^BR2_LINUX_KERNEL_PATCH=""$' | sort || true)"
    [ "$old" = "$new" ] || changed=yes
fi
if [ "$changed" = yes ] && [ -f "$root/out/pi/$variant/output/Makefile" ]; then
    echo "==> $variant: the kernel source changed; cleaning its build tree"
    in_container make -C /output clean
fi

in_container /image/build.sh
printf '%s\n' "$kernel_source" > "$source_stamp"

# The PC's kernel carries its release string uncompressed in the bzImage
# setup code, which is what the Starter reads (docs/STARTER.md §4.1).
kernel="$root/out/pi/$variant/output/images/bzImage"
if [ -f "$kernel" ] && ! grep -qaF -- "$localversion " "$kernel"; then
    echo "$kernel does not carry the release string $localversion" >&2
    exit 1
fi
