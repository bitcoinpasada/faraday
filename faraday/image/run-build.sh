#!/usr/bin/env bash
# Builds the Faraday stick image: upstream's image tree
# (opensigner/shells/pi/image) with faraday/image/overlay copied over
# it, and the Faraday binary where upstream's Buildroot package looks
# for opensigner-pi. The Buildroot checkout, downloads, compiler cache and
# build tree are upstream's own under out/pi/, so only what changed is
# rebuilt.
#
#   run-build.sh <board> <panel> <dev> <buildroot tag> <buildroot commit>
set -euo pipefail
board="${1:?}"
panel="${2:?}"
dev="${3:?}"
tag="${4:?}"
commit="${5:?}"
root="$(pwd)"
merged="$root/out/faraday/image-src"
mkdir -p "$merged"
rsync -a --delete "$root/opensigner/shells/pi/image/" "$merged/"
cp -a "$root/faraday/image/overlay/." "$merged/"
# The disk process and the grant helper (PLAN.md §4.3) go in as files of
# the root filesystem overlay: upstream's Buildroot package installs the
# app alone.
mkdir -p "$merged/common/rootfs-overlay/usr/bin"
for b in faraday-disk faraday-grant; do
    install -m 0755 "$root/out/faraday/$board/$b" "$merged/common/rootfs-overlay/usr/bin/$b"
done

variant="$board-$panel"
if [ "$dev" != "0" ]; then
    variant="$variant-dev"
fi
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
"${DOCKER:-docker}" build -t opensigner-pi-image "$merged"
"${DOCKER:-docker}" run --rm \
    -e BOARD="$board" -e PANEL="$panel" -e DEV="$dev" \
    -v "$root/out/pi/buildroot":/buildroot \
    -v "$root/out/pi/$variant/output":/output \
    -v "$root/out/pi/dl":/dl \
    -v "$root/out/pi/ccache":/ccache \
    -v "$root/out/faraday/$board/opensigner-pi":/binary/opensigner-pi:ro \
    -v "$merged":/image:ro \
    opensigner-pi-image /image/build.sh
