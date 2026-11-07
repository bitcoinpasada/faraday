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

# The firmware reads config.txt's `dtoverlay=faraday-txp-off` from the
# overlays directory of the boot partition, which genimage copies from
# rpi-firmware/overlays. The overlay is Faraday's own source, compiled by
# Buildroot's dtc with symbols (-@) like the firmware's own.
dtc="${HOST_DIR}/bin/dtc"
[ -x "$dtc" ] || { echo "post-image.sh: no dtc at $dtc" >&2; exit 1; }
[ -d "${BINARIES_DIR}/rpi-firmware/overlays" ] || {
    echo "post-image.sh: no rpi-firmware/overlays in ${BINARIES_DIR}" >&2; exit 1; }
"$dtc" -q -@ -I dts -O dtb \
    -o "${BINARIES_DIR}/rpi-firmware/overlays/faraday-txp-off.dtbo" \
    "$(dirname "$0")/faraday-txp-off.dts"
# The compiled overlay must still name the writeback block.
"$dtc" -q -I dtb -O dts "${BINARIES_DIR}/rpi-firmware/overlays/faraday-txp-off.dtbo" \
    | grep -q 'status = "disabled"' || {
    echo "post-image.sh: faraday-txp-off.dtbo does not disable anything" >&2; exit 1; }
