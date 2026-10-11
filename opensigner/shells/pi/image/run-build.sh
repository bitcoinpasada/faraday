#!/usr/bin/env bash
# Run one image build in the container, from the host.
#
#   run-build.sh <board> <panel> <dev> <buildroot tag> <buildroot commit>
#
# `just pi-image` and `just stick-image` are both this, with different
# arguments; the recipe that called it copies the finished image out. Run
# from the repository root, which is where out/ is.
#
# The tag is what the clone asks for and what a person reads. The commit
# is what the build checks: a tag is a name somebody can move, so a
# checkout whose HEAD is not that commit fails here, before anything is
# compiled, with both hashes printed.
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
set -euo pipefail

board="${1:?usage: run-build.sh <board> <panel> <dev> <buildroot tag> <buildroot commit>}"
panel="${2:?}"
dev="${3:?}"
tag="${4:?}"
commit="${5:?}"

root="$(pwd)"
image_dir="$root/opensigner/shells/pi/image"

[ -d "$image_dir/boards/$board" ] || { echo "no such board: $board" >&2; exit 1; }
[ -d "$image_dir/panels/$panel" ] || { echo "no such panel: $panel" >&2; exit 1; }

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
    echo "out/pi/buildroot is not the pinned Buildroot:" >&2
    echo "  pinned: $commit  ($tag)" >&2
    echo "  got:    $head" >&2
    exit 1
fi
echo "==> buildroot $tag $head"
docker build -t opensigner-pi-image "$image_dir"
docker run --rm \
    -e BOARD="$board" -e PANEL="$panel" -e DEV="$dev" \
    -v "$root/out/pi/buildroot":/buildroot \
    -v "$root/out/pi/$variant/output":/output \
    -v "$root/out/pi/dl":/dl \
    -v "$root/out/pi/ccache":/ccache \
    -v "$root/out/pi/$board/opensigner-pi":/binary/opensigner-pi:ro \
    -v "$image_dir":/image:ro \
    opensigner-pi-image /image/build.sh
