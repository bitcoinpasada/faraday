#!/usr/bin/env bash
# Writes a Faraday stick image's boot partition (the -boot.vfat that
# `just faraday-stick-image` leaves beside the .img) over the partition
# named OSKBOOT on a Faraday stick, and nothing else: the data partition,
# with its vaults and settings file, keeps its bytes. For development
# only (PLAN.md §5.5); it puts the whole stick on the build computer.
#
#   stick-boot.sh [--dry-run] <boot.vfat> <target>
#
# The target is a whole USB disk (/dev/sdX) or, for testing, a regular
# file holding a stick image. It refuses unless:
#   - a disk is a whole disk, removable and on USB;
#   - no partition of it is mounted or used as swap (a file: it is not
#     attached to a loop device);
#   - its partition table is a Faraday stick's: GPT with Faraday's disk
#     UUID and exactly two partitions, the first named OSKBOOT, an EFI
#     system partition with Faraday's partition UUID and at least as large
#     as the .vfat, the second named OSKDATA with its own UUID;
#   - the .vfat is a FAT filesystem labelled OSKBOOT;
#   - the target's path is typed back.
# It then writes at the partition's offset, reads back and compares, and
# checks that the partition table is what it was.
#
# --dry-run runs every check, prints what it would write and where, and
# writes nothing.
set -euo pipefail

die() { printf 'faraday-stick-boot: %s\n' "$1" >&2; exit 1; }

dry=0
if [ "${1:-}" = "--dry-run" ]; then
    dry=1
    shift
fi
[ $# -eq 2 ] || die "usage: stick-boot.sh [--dry-run] <boot.vfat> <target>"
vfat="$1"
target="$2"

# genimage.cfg's fixed UUIDs, and the EFI system partition's type.
disk_uuid="4F70656E-5369-676E-6572-4B6974000000"
boot_uuid="4F70656E-5369-676E-6572-4B6974000001"
data_uuid="4F70656E-5369-676E-6572-4B6974000002"
esp_type="C12A7328-F81F-11D2-BA4B-00A0C93EC93B"

# ---- the source ------------------------------------------------------------
[ -f "$vfat" ] || die "$vfat: no such file (build it with just faraday-stick-image)"
vfat_size="$(stat -c %s "$vfat")"
[ "$vfat_size" -gt 0 ] && [ $((vfat_size % 512)) -eq 0 ] \
    || die "$vfat: size $vfat_size is not a whole number of sectors"
# FAT32's boot sector: the 0x55AA signature at 510, the volume label at
# 71 and "FAT32   " at 82.
sig="$(od -An -tx1 -j510 -N2 "$vfat" | tr -d ' \n')"
label="$(dd if="$vfat" bs=1 skip=71 count=11 status=none | tr -d '\0')"
fstype="$(dd if="$vfat" bs=1 skip=82 count=8 status=none | tr -d '\0')"
[ "$sig" = "55aa" ] && [ "$fstype" = "FAT32   " ] && [ "$label" = "OSKBOOT    " ] \
    || die "$vfat: not a FAT32 filesystem labelled OSKBOOT"

# ---- the target ------------------------------------------------------------
sudo=""
if [ -b "$target" ]; then
    target="$(readlink -f "$target")"
    [ "$(lsblk -dno TYPE "$target")" = "disk" ] \
        || die "$target is not a whole disk; give /dev/sdX, not a partition"
    name="$(basename "$target")"
    [ "$(cat "/sys/block/$name/removable" 2>/dev/null)" = "1" ] \
        || die "$target is not removable"
    [ "$(lsblk -dno TRAN "$target")" = "usb" ] || die "$target is not on USB"
    if lsblk -no MOUNTPOINTS "$target" | grep -q .; then
        die "$target has a mounted partition ($(lsblk -no MOUNTPOINTS "$target" | grep . | tr '\n' ' ')); unmount it first"
    fi
    if grep -q "^/dev/$name" /proc/swaps; then
        die "$target is in use as swap"
    fi
    what="$(lsblk -dno VENDOR,MODEL,SIZE "$target" | tr -s ' ')"
    [ "$(id -u)" = 0 ] || sudo="sudo"
elif [ -f "$target" ]; then
    if command -v losetup >/dev/null && [ -n "$(losetup -j "$target" 2>/dev/null)" ]; then
        die "$target is attached to a loop device; detach it first"
    fi
    what="regular file, $(stat -c %s "$target") bytes"
else
    die "$target is neither a block device nor a regular file"
fi

table() { $sudo sfdisk -J "$target"; }
layout="$(table)" || die "$target: no partition table sfdisk can read"

# "ok <offset> <size>" in bytes of the OSKBOOT partition, or "refuse
# <reason>".
check="$(LAYOUT="$layout" python3 -B -c '
import json, os, sys
disk_uuid, boot_uuid, data_uuid, esp_type, vfat_size = sys.argv[1:6]
t = json.loads(os.environ["LAYOUT"])["partitiontable"]
def refuse(why):
    print("refuse " + why)
    sys.exit(0)
if t.get("label") != "gpt":
    refuse("its partition table is not GPT")
if t.get("id", "").upper() != disk_uuid:
    refuse("its disk UUID is not a Faraday stick'"'"'s")
parts = t.get("partitions", [])
if len(parts) != 2:
    refuse("it has %d partitions, not two" % len(parts))
boot, data = parts
ss = int(t.get("sectorsize", 512))
if boot.get("name") != "OSKBOOT":
    refuse("its first partition is not named OSKBOOT")
if boot.get("uuid", "").upper() != boot_uuid or boot.get("type", "").upper() != esp_type:
    refuse("its first partition is not a Faraday EFI system partition")
if data.get("name") != "OSKDATA" or data.get("uuid", "").upper() != data_uuid:
    refuse("its second partition is not a Faraday OSKDATA partition")
size = boot["size"] * ss
if size < int(vfat_size):
    refuse("its OSKBOOT partition (%d bytes) is smaller than the image (%s bytes)" % (size, vfat_size))
if boot["start"] + boot["size"] > data["start"]:
    refuse("its partitions overlap")
print("ok %d %d" % (boot["start"] * ss, size))
' "$disk_uuid" "$boot_uuid" "$data_uuid" "$esp_type" "$vfat_size")"
case "$check" in
    "ok "*) read -r _ offset part_size <<<"$check" ;;
    "refuse "*) die "$target is not a Faraday stick: ${check#refuse }" ;;
    *) die "$target: could not read its partition table" ;;
esac

echo "Target : $target ($what)"
echo "Source : $vfat ($vfat_size bytes)"
echo "Writes : bytes $offset to $((offset + vfat_size - 1)) of $target, the OSKBOOT partition ($part_size bytes)"
echo "Keeps  : the partition table and the OSKDATA partition"
if [ "$dry" = 1 ]; then
    echo "Dry run: nothing written."
    exit 0
fi

printf 'This replaces the OSKBOOT partition on %s. Type %s to write: ' "$target" "$target"
read -r answer || true
[ "$answer" = "$target" ] || die "not confirmed; nothing written"

direct=""
if [ -b "$target" ]; then
    direct=",direct"
fi
$sudo dd if="$vfat" of="$target" bs=4M seek="$offset" \
    oflag="seek_bytes$direct" conv=notrunc,fsync status=progress
sync
if [ -b "$target" ]; then
    $sudo blockdev --flushbufs "$target"
fi

echo "Reading back..."
cmp -n "$vfat_size" "$vfat" \
    <($sudo dd if="$target" bs=4M skip="$offset" count="$vfat_size" \
        iflag="skip_bytes,count_bytes$direct" status=none) \
    || die "read-back differs from $vfat"
[ "$(table)" = "$layout" ] || die "the partition table changed"
echo "OK: the OSKBOOT partition on $target matches $vfat"
