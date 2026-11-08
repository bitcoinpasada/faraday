#!/bin/sh
# Faraday: writes faraday-build.txt for the boot partition, before
# common/post-image.sh runs genimage (genimage.cfg lists it). The Pi's
# kernel is a compressed zImage, so its release string is not in the
# image's bytes; this file holds it, a line, so faraday-boot's
# find_release (the Starter's), and the staging script read the Pi card's
# build ID as they read a stick's (docs/STARTER.md §1.3).
set -e

release=""
for f in "${BUILD_DIR}"/linux-custom/include/config/kernel.release; do
    [ -f "$f" ] && release="$(cat "$f")" && break
done
case "$release" in
    [0-9]*-faraday-*) ;;
    *) echo "post-image.sh: no Faraday kernel release string found (got '$release')" >&2; exit 1 ;;
esac
printf '%s\n' "$release" > "${BINARIES_DIR}/faraday-build.txt"
