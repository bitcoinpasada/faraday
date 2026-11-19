#!/usr/bin/env bash
# Writes a Faraday stick image's boot partition (the -boot.vfat that
# `just faraday-stick-image` leaves beside the .img) over the first
# partition of a Faraday stick, and nothing else: the data partition,
# with its vaults and settings file, keeps its bytes. For development
# only (docs/DEVELOPING.md); it puts the whole stick on the build computer.
#
#   stick-boot.sh [--dry-run] <boot.vfat> <target>
#
# The target is a whole USB disk (/dev/sdX) or, for testing, a regular
# file holding a stick image. It refuses unless:
#   - a disk is a whole disk on USB (its removable flag plays no part: many USB
#     sticks and SSDs report themselves fixed);
#   - no partition of it is mounted or used as swap (a file: it is not
#     attached to a loop device);
#   - its partition table has a Faraday stick's layout
#     (docs/STICK-IDENTITY.md §5.1): GPT with exactly two partitions, the
#     first an EFI system partition at sector 2048 of 64 MiB, at least as
#     large as the .vfat, the second a basic data partition at sector
#     133120 of at least 128 MiB, ending at or before the disk's last
#     usable sector (F182). Names, IDs and labels play no part (F174);
#   - its primary GPT header and entries are ones the Starter's Keep data
#     reads (faraday/shells/starter/src/gpt.rs): the signature, a header
#     size of 92 to a sector and its CRC, entries of 128 to 4096 bytes,
#     1 to 1024 of them, with their CRC, starting at sector 2 or later,
#     inside the disk and before the first usable sector; a last usable
#     sector that leaves the backup entries' room before the disk's last
#     sector; a backup header past that room and inside the disk; every
#     used entry inside the usable sectors; and no number that overflows;
#   - the .vfat is a FAT32 filesystem labelled EFI, as the image's is;
#   - the target's path is typed back.
# It then writes at the partition's offset, reads back and compares, and
# checks that the partition table is what it was. The stick keeps its own
# disk and partition GUIDs; the boot partition gets the image's FAT
# serial, where the Starter's keep data keeps the stick's (§4.4).
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

# A Faraday stick's layout (docs/STICK-IDENTITY.md §5.1, F80, F182): the two
# partitions' types, and their starts and least sizes in 512-byte sectors
# (the boot partition's size is exact).
esp_type="C12A7328-F81F-11D2-BA4B-00A0C93EC93B"
data_type="EBD0A0A2-B9E5-4433-87C0-68B6B72699C7"
layout_sectors="2048 131072 133120 262144"

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
[ "$sig" = "55aa" ] && [ "$fstype" = "FAT32   " ] && [ "$label" = "EFI        " ] \
    || die "$vfat: not a FAT32 filesystem labelled EFI"

# ---- the target ------------------------------------------------------------
sudo=""
if [ -b "$target" ]; then
    target="$(readlink -f "$target")"
    [ "$(lsblk -dno TYPE "$target")" = "disk" ] \
        || die "$target is not a whole disk; give /dev/sdX, not a partition"
    name="$(basename "$target")"
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

# "ok <offset> <size>" in bytes of the boot partition, or "refuse
# <reason>".
check="$(LAYOUT="$layout" python3 -I -B -c '
import json, os, sys
esp_type, data_type, sectors, vfat_size = sys.argv[1:5]
boot_start, boot_size, data_start, data_size = (int(n) for n in sectors.split())
t = json.loads(os.environ["LAYOUT"])["partitiontable"]
def refuse(why):
    print("refuse " + why)
    sys.exit(0)
if t.get("label") != "gpt":
    refuse("its partition table is not GPT")
parts = t.get("partitions", [])
if len(parts) != 2:
    refuse("it has %d partitions, not two" % len(parts))
boot, data = parts
ss = int(t.get("sectorsize", 512))
# Starts and sizes in 512-byte sectors, whatever the disk reports.
at = lambda p, k: p[k] * ss // 512
if boot.get("type", "").upper() != esp_type:
    refuse("its first partition is not an EFI system partition")
if (at(boot, "start"), at(boot, "size")) != (boot_start, boot_size):
    refuse("its first partition is not at sector %d of 64 MiB" % boot_start)
if data.get("type", "").upper() != data_type:
    refuse("its second partition is not a basic data partition")
if at(data, "start") != data_start or at(data, "size") < data_size:
    refuse("its second partition is not at sector %d of at least 128 MiB" % data_start)
if "lastlba" not in t:
    refuse("its table does not say its last usable sector")
if data["start"] + data["size"] - 1 > t["lastlba"]:
    refuse("its second partition ends past the last usable sector of the disk")
size = boot["size"] * ss
if size < int(vfat_size):
    refuse("its boot partition (%d bytes) is smaller than the image (%s bytes)" % (size, vfat_size))
print("ok %d %d %d" % (boot["start"] * ss, size, ss))
' "$esp_type" "$data_type" "$layout_sectors" "$vfat_size")"
case "$check" in
    "ok "*) read -r _ offset part_size ss <<<"$check" ;;
    "refuse "*) die "$target is not a Faraday stick: ${check#refuse }" ;;
    *) die "$target: could not read its partition table" ;;
esac

# The primary GPT header (the sector after the first) and its entries, read
# from the target as they are, must be ones the Starter's Keep data reads
# (faraday/shells/starter/src/gpt.rs, read_with), numbers checked and never
# wrapped. "ok" or "refuse <reason>".
case "$check" in "ok "*)
gpt="$($sudo python3 -I -B -c '
import struct, sys, zlib
path, ss = sys.argv[1], int(sys.argv[2])
M = (1 << 64) - 1
def refuse(why):
    print("refuse " + why)
    sys.exit(0)
with open(path, "rb") as f:
    f.seek(0, 2)
    length = f.tell()
    f.seek(ss)
    h = f.read(ss)
    if len(h) != ss:
        refuse("its disk is too short for a GPT header")
    if h[:8] != b"EFI PART":
        refuse("its GPT header has no EFI PART signature")
    size = struct.unpack_from("<I", h, 12)[0]
    if not 92 <= size <= ss:
        refuse("its GPT header size is not 92 to a sector")
    c = bytearray(h[:size])
    c[16:20] = bytes(4)
    if zlib.crc32(bytes(c)) != struct.unpack_from("<I", h, 16)[0]:
        refuse("its GPT header checksum is wrong")
    alt, first, last, elba, count, esize, ecrc = (
        struct.unpack_from("<Q", h, 32)[0], struct.unpack_from("<Q", h, 40)[0],
        struct.unpack_from("<Q", h, 48)[0], struct.unpack_from("<Q", h, 72)[0],
        struct.unpack_from("<I", h, 80)[0], struct.unpack_from("<I", h, 84)[0],
        struct.unpack_from("<I", h, 88)[0])
    if not 128 <= esize <= 4096 or count == 0 or count > 1024:
        refuse("its GPT entries are not 1 to 1024 of 128 to 4096 bytes")
    elen = count * esize
    esec = -(-elen // ss)
    if elba * ss > M or (elba + esec) * ss > M or (alt + 1) * ss > M:
        refuse("its GPT header has numbers past what a disk can address")
    if elba * ss + elen > length:
        refuse("its GPT entries run past the end of the disk")
    dsec = length // ss
    if elba < 2 or first < elba + esec or last < first:
        refuse("its GPT entries are not between the header and the first usable sector")
    if last + esec >= max(dsec - 1, 0) or alt <= last + esec or alt >= dsec:
        refuse("its GPT backup header and entries do not fit past the last usable sector, inside the disk")
    f.seek(elba * ss)
    e = f.read(elen)
    if len(e) != elen:
        refuse("its GPT entries could not be read")
    if zlib.crc32(e) != ecrc:
        refuse("its GPT entries checksum is wrong")
    for i in range(0, elen, esize):
        en = e[i:i + esize]
        if not any(en[:16]):
            continue
        a, b = struct.unpack_from("<QQ", en, 32)
        if a < first or b < a or b > last:
            refuse("a GPT entry lies outside the usable sectors")
print("ok")
' "$target" "$ss")" || die "$target: could not read its GPT header"
    [ "$gpt" = "ok" ] || die "$target is not a Faraday stick: ${gpt#refuse }" ;;
esac

echo "Target : $target ($what)"
echo "Source : $vfat ($vfat_size bytes)"
echo "Writes : bytes $offset to $((offset + vfat_size - 1)) of $target, the boot partition ($part_size bytes)"
echo "Keeps  : the partition table and the data partition"
if [ "$dry" = 1 ]; then
    echo "Dry run: nothing written."
    exit 0
fi

printf 'This replaces the boot partition on %s. Type %s to write: ' "$target" "$target"
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
echo "OK: the boot partition on $target matches $vfat"
