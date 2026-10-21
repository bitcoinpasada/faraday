#!/usr/bin/env bash
# The volume serial of one of the image's FAT filesystems, as the eight
# hex digits `mkfs.vfat -i` takes. It is a function of the commit date
# (SOURCE_DATE_EPOCH) and the partition's role (boot or data), so two
# builds of one commit stamp the same serial and the two partitions of
# an image differ from each other. mkfs.vfat would otherwise stamp a
# random one, the only bytes in which two builds of the same source
# differ (README.md, "Reproducible builds").
#
#   fat-serial.sh <boot|data> <SOURCE_DATE_EPOCH>
set -euo pipefail
role="${1:?role: boot or data}"
epoch="${2:?SOURCE_DATE_EPOCH}"
case "$role" in
    boot | data) ;;
    *) echo "fat-serial.sh: role is boot or data, not '$role'" >&2; exit 1 ;;
esac
case "$epoch" in
    '' | *[!0-9]*) echo "fat-serial.sh: '$epoch' is not an epoch" >&2; exit 1 ;;
esac
printf 'faraday-fat-serial:%s:%s' "$role" "$epoch" | sha256sum | cut -c1-8
