#!/usr/bin/env python3
"""Leaves the stick image as Windows leaves it (docs/STICK-IDENTITY.md §4.1).

    stick-finish.py <image> [<boot.vfat>]

Run by common/post-image.sh after genimage, on the PC stick's image only.
It changes two things and nothing else, the same bytes on every builder:

  - both GPT headers give 34 as the first usable sector (genimage writes
    the first partition's start, 2048); the partitions do not move; the
    headers' CRCs and the entries' CRC are worked out again;
  - the boot FAT32 partition (the first one) has FAT[1] set to 0xFFFFFFFF
    in both FATs and FSInfo's next-free hint set to the first free
    cluster, read from the volume's FAT as it is.

The boot partition's file, <boot.vfat> (genimage's esp.vfat, which
`just faraday-stick-image` copies beside the image), is given the same
bytes, so the -boot.vfat that SHA256SUMS lists is the partition as it is
after this step. It then runs sgdisk -v, sfdisk --verify and fsck.fat -n,
which Faraday did not write, and fails if any reports a problem.
"""
import os
import struct
import subprocess
import sys
import zlib

SS = 512
FIRST_USABLE = 34


def die(msg):
    print(f"stick-finish: {msg}", file=sys.stderr)
    sys.exit(1)


def header_crc(h):
    size = struct.unpack_from("<I", h, 12)[0]
    h[16:20] = b"\0" * 4
    struct.pack_into("<I", h, 16, zlib.crc32(bytes(h[:size])))


def read_at(f, at, n):
    f.seek(at)
    b = f.read(n)
    if len(b) != n:
        die("the image ends inside its partition table")
    return b


def gpt(f):
    """Set the first usable sector in both headers; return the first partition."""
    primary = bytearray(read_at(f, SS, SS))
    if primary[:8] != b"EFI PART":
        die("no GPT header at sector 1")
    alt_lba = struct.unpack_from("<Q", primary, 32)[0]
    for lba in (1, alt_lba):
        h = bytearray(read_at(f, lba * SS, SS))
        if h[:8] != b"EFI PART":
            die(f"no GPT header at sector {lba}")
        ent_lba, n, size = struct.unpack_from("<QII", h, 72)
        entries = read_at(f, ent_lba * SS, n * size)
        if lba == 1:
            first = None
            for i in range(n):
                e = entries[i * size:(i + 1) * size]
                if e[:16] != b"\0" * 16:
                    start, end = struct.unpack_from("<QQ", e, 32)
                    if first is None or start < first[0]:
                        first = (start, end - start + 1)
            if first is None:
                die("the GPT has no partition")
        struct.pack_into("<Q", h, 40, FIRST_USABLE)
        struct.pack_into("<I", h, 88, zlib.crc32(entries))
        header_crc(h)
        f.seek(lba * SS)
        f.write(h)
    return first


def fat32(vol):
    """FAT[1] and FSInfo's hint in a FAT32 volume, as a bytearray."""
    bps, spc, reserved, nfats = struct.unpack_from("<HBHB", vol, 11)
    total = struct.unpack_from("<I", vol, 32)[0]
    fat_sectors = struct.unpack_from("<I", vol, 36)[0]
    fsinfo = struct.unpack_from("<H", vol, 48)[0]
    if vol[82:90] != b"FAT32   " or bps != SS or nfats != 2 or fsinfo == 0:
        die("the boot partition is not a FAT32 volume with two FATs and an FSInfo")
    fats = [(reserved + i * fat_sectors) * bps for i in range(nfats)]
    clusters = (total - reserved - nfats * fat_sectors) // spc
    for at in fats:
        vol[at + 4:at + 8] = b"\xff\xff\xff\xff"
    fat = vol[fats[0]:fats[0] + fat_sectors * bps]
    free = None
    for c in range(2, clusters + 2):
        if struct.unpack_from("<I", fat, c * 4)[0] & 0x0FFFFFFF == 0:
            free = c
            break
    info = fsinfo * bps
    if struct.unpack_from("<I", vol, info)[0] != 0x41615252 or struct.unpack_from("<I", vol, info + 484)[0] != 0x61417272:
        die("FSInfo has no signature")
    struct.pack_into("<I", vol, info + 492, 0xFFFFFFFF if free is None else free)
    return vol


def tool(argv, ok):
    r = subprocess.run(argv, capture_output=True, text=True)
    out = (r.stdout + r.stderr).strip()
    if r.returncode != 0 or not ok(out):
        die(f"{' '.join(argv)} reports a problem:\n{out}")


def main():
    args = sys.argv[1:]
    if len(args) not in (1, 2):
        die("usage: stick-finish.py <image> [<boot.vfat>]")
    image = args[0]
    with open(image, "r+b") as f:
        start, sectors = gpt(f)
        vol = bytearray(read_at(f, start * SS, sectors * SS))
        if len(args) == 2:
            with open(args[1], "rb") as v:
                if v.read() != bytes(vol):
                    die(f"{args[1]} is not the image's boot partition")
        fat32(vol)
        f.seek(start * SS)
        f.write(vol)
    if len(args) == 2:
        with open(args[1], "r+b") as v:
            v.write(vol)
    boot = args[1] if len(args) == 2 else None
    tool(["sgdisk", "-v", image], lambda o: "No problems found" in o)
    tool(["sfdisk", "--verify", image], lambda o: "No errors detected" in o)
    if boot is None:
        boot = image + ".boot.tmp"
        with open(boot, "wb") as t:
            t.write(vol)
    try:
        tool(["fsck.fat", "-n", boot], lambda o: True)
    finally:
        if len(args) == 1:
            os.unlink(boot)


main()
