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
# The disk process, the grant helper (PLAN.md §4.3) and the boot copier
# (§5.5) go in as files of the root filesystem overlay: upstream's
# Buildroot package installs the app alone.
mkdir -p "$merged/common/rootfs-overlay/usr/bin"
for b in faraday-disk faraday-grant faraday-boot; do
    install -m 0755 "$root/out/faraday/$board/$b" "$merged/common/rootfs-overlay/usr/bin/$b"
done

variant="$board-$panel"
if [ "$dev" != "0" ]; then
    variant="$variant-dev"
fi

# The kernel's release string carries the Faraday version and commit
# (PLAN.md §5.5): `6.6.84-faraday-0.1.0+4d0680b1a2b3`, with `.dirty` after
# the commit when tracked files differ from it. The upgrade takes a boot
# partition as the source only if it holds the running kernel's release
# string, and shows a target's version from the one it holds. It comes
# from the commit, never the clock, so two builders get the same bytes.
version="$(sed -n 's/^version = "\(.*\)"$/\1/p' "$root/faraday/faraday-core/Cargo.toml" | head -1)"
faraday_commit="$(git -C "$root" rev-parse --short=12 HEAD)"
dirty=""
git -C "$root" diff --quiet HEAD -- || dirty=".dirty"
localversion="-faraday-$version+$faraday_commit$dirty"
fragment="$merged/boards/$board/linux.fragment"
[ -f "$fragment" ] || { echo "$fragment: no board fragment to add the version to" >&2; exit 1; }
printf '\n# Faraday'"'"'s version and commit (run-build.sh).\nCONFIG_LOCALVERSION="%s"\n# CONFIG_LOCALVERSION_AUTO is not set\n' \
    "$localversion" >> "$fragment"
# Buildroot configures the kernel again when a fragment is newer than its
# .config. The fragment is dated by the newer of its source and the last
# change of version, so a build with nothing new configures nothing again.
stamp="$root/out/faraday/$board/localversion"
mkdir -p "$(dirname "$stamp")"
if [ "$(cat "$stamp" 2>/dev/null)" != "$localversion" ]; then
    printf '%s\n' "$localversion" > "$stamp"
fi
source_fragment="$root/faraday/image/overlay/boards/$board/linux.fragment"
if [ "$stamp" -nt "$source_fragment" ]; then
    touch -r "$stamp" "$fragment"
else
    touch -r "$source_fragment" "$fragment"
fi
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

# The PC's kernel carries its release string uncompressed in the bzImage
# setup code, which is what the upgrade reads (PLAN.md §5.5).
kernel="$root/out/pi/$variant/output/images/bzImage"
if [ -f "$kernel" ] && ! grep -qaF -- "$localversion " "$kernel"; then
    echo "$kernel does not carry the release string $localversion" >&2
    exit 1
fi
