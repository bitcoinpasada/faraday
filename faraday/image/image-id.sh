#!/usr/bin/env bash
# One of the image's partition-table IDs (docs/STICK-IDENTITY.md §4.1): the
# stick's disk GUID or one of its two partition GUIDs, or the Pi card's MBR
# disk signature. Like fat-serial.sh's volume serials, each is a function
# of the commit date (SOURCE_DATE_EPOCH) and its role, so two builds of
# one commit write the same bytes, and the IDs differ from one release to
# the next. They spell nothing. A GUID is printed in the form of a
# version 4 one (RFC 9562: the version nibble 4, the variant bits 10),
# as genimage's disk-uuid and partition-uuid take it; the signature as
# the 0x-prefixed hex genimage's disk-signature takes.
#
# The Starter gives every stick and card it writes IDs of its own drawn
# from the system's randomness (§4.2); these are the ones a stick
# written by another tool keeps.
#
#   image-id.sh <disk|boot|data|signature> <SOURCE_DATE_EPOCH>
set -euo pipefail
role="${1:?role: disk, boot, data or signature}"
epoch="${2:?SOURCE_DATE_EPOCH}"
case "$role" in
    disk | boot | data | signature) ;;
    *) echo "image-id.sh: role is disk, boot, data or signature, not '$role'" >&2; exit 1 ;;
esac
case "$epoch" in
    '' | *[!0-9]*) echo "image-id.sh: '$epoch' is not an epoch" >&2; exit 1 ;;
esac
hex="$(printf 'faraday-image-id:%s:%s' "$role" "$epoch" | sha256sum | cut -c1-32)"
if [ "$role" = signature ]; then
    # A disk signature of zero reads as none to some tools; one build in
    # 2^32 would hash to it.
    [ "${hex:0:8}" != 00000000 ] || hex="00000001${hex:8}"
    printf '0x%s\n' "${hex:0:8}"
    exit 0
fi
# The variant's two high bits are 10: 8, 9, a or b.
variant="$(printf '%x' $(( (0x${hex:16:1} & 3) | 8 )))"
printf '%s-%s-4%s-%s%s-%s\n' "${hex:0:8}" "${hex:8:4}" "${hex:13:3}" \
    "$variant" "${hex:17:3}" "${hex:20:12}"
