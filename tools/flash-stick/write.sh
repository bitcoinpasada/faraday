#!/usr/bin/env bash
# Writes a Faraday stick image to a USB stick. The privileged half of
# tools/flash-stick/flash-stick.py, which runs it through pkexec; it can
# also be run by hand as root.
#
#   write.sh whole IMAGE.img  DEVICE [SERIAL SIZE]
#   write.sh boot  IMAGE.vfat DEVICE [SERIAL SIZE]
#
# whole  writes the stick image (out/stick/faraday-x86_64-uefi[-dev].img)
#        over the whole disk: everything on it is lost.
# boot   writes the image's boot partition
#        (out/stick/faraday-x86_64-uefi[-dev]-boot.vfat) over a Faraday
#        stick's OSKBOOT partition and keeps its OSKDATA partition, with
#        the vaults and settings on it. faraday/image/stick-boot.sh does
#        the writing and its own checks.
#
# DEVICE is a whole disk (/dev/sdX). It refuses unless the disk is on USB,
# removable or hot-pluggable, larger than the image, and not in use by the
# computer: a partition mounted anywhere but under /run/media, /media or
# /mnt, or used as swap, stops it. Partitions mounted there are unmounted
# first.
#
# SERIAL and SIZE (in bytes) are what the person confirmed, as lsblk
# reports them ("-" for a stick with no serial). Given, the disk must
# still match them, and nothing is asked. Left out, the stick is described
# and its path must be typed back.
#
# Each whole write is read back and compared with the image.
set -euo pipefail

die() { printf 'flash-stick: %s\n' "$1" >&2; exit 1; }

[ $# -eq 3 ] || [ $# -eq 5 ] || die "usage: write.sh whole|boot IMAGE DEVICE [SERIAL SIZE]"
mode="$1"
image="$2"
dev="$3"
want_serial="${4:-}"
want_size="${5:-}"

[ "$(id -u)" = 0 ] || die "needs root: run it with sudo, or through flash-stick.py"
case "$mode" in
    whole | boot) ;;
    *) die "the mode is whole or boot, not $mode" ;;
esac
[ -f "$image" ] || die "$image: no such file (build it with just faraday-stick-image)"
image_size="$(stat -c %s "$image")"
[ "$image_size" -gt 0 ] || die "$image is empty"

# ---- the stick --------------------------------------------------------------
[ -b "$dev" ] || die "$dev is not a block device"
dev="$(readlink -f "$dev")"
info() { lsblk -dnbo "$1" "$dev" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//'; }
[ "$(info TYPE)" = "disk" ] || die "$dev is not a whole disk; give /dev/sdX, not a partition"
[ "$(info TRAN)" = "usb" ] || die "$dev is not on USB"
[ "$(info RM)" = "1" ] || [ "$(info HOTPLUG)" = "1" ] \
    || die "$dev is neither removable nor hot-pluggable"
serial="$(info SERIAL)"
size="$(info SIZE)"
what="$(info VENDOR) $(info MODEL)"
what="$(echo "$what" | tr -s ' ' | sed 's/^ //; s/ $//')"
[ "$size" -ge "$image_size" ] || die "$dev ($size bytes) is smaller than $image ($image_size bytes)"

if [ -n "$want_size" ]; then
    [ "${serial:--}" = "$want_serial" ] && [ "$size" = "$want_size" ] \
        || die "$dev is no longer the stick that was chosen (serial ${serial:-none}, $size bytes); nothing written"
fi

# In use by the computer, or only mounted by the desktop when it was
# plugged in.
for part in $(lsblk -nrpo NAME "$dev"); do
    if grep -q "^$part " /proc/swaps; then
        die "$part is in use as swap"
    fi
    while read -r target; do
        [ -n "$target" ] || continue
        target="$(printf '%b' "$target")"
        case "$target" in
            /run/media/* | /media/* | /mnt/*) ;;
            *) die "$part is mounted at $target: this disk is in use by the computer" ;;
        esac
    done < <(findmnt -rn -S "$part" -o TARGET || true)
done

echo "Stick : $dev, ${what:-USB disk}, $size bytes, serial ${serial:-none}"
echo "Image : $image ($image_size bytes)"
if [ "$mode" = whole ]; then
    echo "Writes: the whole stick; everything on it is lost"
else
    echo "Writes: the OSKBOOT partition; OSKDATA is kept"
fi

if [ -z "$want_size" ]; then
    printf 'Type %s to write: ' "$dev"
    read -r answer || true
    [ "$answer" = "$dev" ] || die "not confirmed; nothing written"
fi

for part in $(lsblk -nrpo NAME "$dev"); do
    while findmnt -rn -S "$part" >/dev/null; do
        echo "Unmounting $part"
        umount "$part" || die "cannot unmount $part"
    done
done

# ---- the write --------------------------------------------------------------
if [ "$mode" = boot ]; then
    here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    printf '%s\n' "$dev" | "$here/../../faraday/image/stick-boot.sh" "$image" "$dev"
    echo "Done: Faraday on $dev is replaced and its data partition kept."
    exit 0
fi

echo "Writing..."
dd if="$image" of="$dev" bs=4M oflag=direct conv=fsync status=progress
sync
blockdev --flushbufs "$dev"

echo "Reading back..."
cmp -n "$image_size" "$image" \
    <(dd if="$dev" bs=4M count="$image_size" iflag=count_bytes,direct status=none) \
    || die "what was read back from $dev differs from $image"
blockdev --rereadpt "$dev" 2>/dev/null || true
echo "Done: $dev holds $image."
