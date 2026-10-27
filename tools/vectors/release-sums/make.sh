#!/usr/bin/env bash
# Makes the release-sums fixtures in this directory with GnuPG 2.4
# (README.md names each file). Run from the repository root:
#
#   tools/vectors/release-sums/make.sh
#
# Every gpg call runs in a throwaway home with its clock set to
# 1,780,000,000 (--faked-system-time), so the keys' and signatures'
# dates are fixed; the keys themselves are random, so a new run makes new
# keys and new files. faraday-0.1.7/ is not made here: it is Faraday
# 0.1.7's own SHA256SUMS and SHA256SUMS.asc, as published.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
work=$(mktemp -d)
trap 'gpgconf --homedir "$work/gnupg" --kill all 2>/dev/null || true; rm -rf "$work"' EXIT
mkdir -m 700 "$work/gnupg"

g() {
    gpg --homedir "$work/gnupg" --batch --yes --no-tty --faked-system-time '1780000000!' \
        --pinentry-mode loopback --passphrase '' "$@" 2>/dev/null
}
fpr() {
    g --with-colons --list-keys "$1" | awk -F: '/^fpr/ { print $10; exit }'
}

# The boot program: the PE file faraday-core's testkit::test_efi() makes
# (a PE32+ EFI application whose one section holds a return), byte for
# byte; faraday-core's tests hold the two equal.
python3 -I -B - "$here/BOOTX64.EFI" <<'EOF'
import struct, sys
pe = bytearray(0x400)
def p16(at, v): pe[at:at + 2] = struct.pack("<H", v)
def p32(at, v): pe[at:at + 4] = struct.pack("<I", v)
pe[0:2] = b"MZ"
p32(0x3c, 0x40)
pe[0x40:0x44] = b"PE\0\0"
coff = 0x44
p16(coff, 0x8664); p16(coff + 2, 1); p16(coff + 16, 240); p16(coff + 18, 0x22)
opt = coff + 20
p16(opt, 0x20b); p32(opt + 16, 0x1000); p32(opt + 32, 0x1000); p32(opt + 36, 0x200)
p32(opt + 56, 0x2000); p32(opt + 60, 0x200); p16(opt + 68, 10); p32(opt + 108, 16)
sec = opt + 240
pe[sec:sec + 5] = b".text"
p32(sec + 8, 0x10); p32(sec + 12, 0x1000); p32(sec + 16, 0x200); p32(sec + 20, 0x200)
p32(sec + 36, 0x60000020)
pe[0x200] = 0xc3
open(sys.argv[1], "wb").write(pe)
EOF

# A release's SHA256SUMS, as faraday/tools/stage-release.sh writes one:
# the version and commit, the published files, and the boot program
# under the name the release publishes it as.
{
    echo "# Faraday 0.9.0, commit 0000000000000000000000000000000000000000"
    printf '%s  %s\n' "$(printf 'stick' | sha256sum | cut -d' ' -f1)" "faraday-stick-0.9.0-x86_64-uefi.img.xz"
    printf '%s  %s\n' "$(sha256sum < "$here/BOOTX64.EFI" | cut -d' ' -f1)" "faraday-0.9.0-BOOTX64.EFI"
} > "$here/SHA256SUMS"

# Two keys of the release key's shape: an Ed25519 primary that certifies
# and an Ed25519 subkey that signs, expiring in five years as the
# release key does.
for who in release other; do
    uid="${who^} <$who@example.com>"
    g --quick-generate-key "$uid" ed25519 cert 5y
    fp=$(fpr "$who@example.com")
    g --quick-add-key "$fp" ed25519 sign 5y
    g --armor --export "$fp" > "$here/$who-key.asc"
done

# The sums signed as the release is: a detached, armoured signature by
# the release key's signing subkey, over SHA-512 (GnuPG's choice for an
# Ed25519 key) and over SHA-256; and by the other key.
g --local-user release@example.com --armor --detach-sign \
    --output "$here/SHA256SUMS.asc" "$here/SHA256SUMS"
g --local-user release@example.com --armor --detach-sign --digest-algo SHA256 \
    --output "$here/SHA256SUMS.sha256.asc" "$here/SHA256SUMS"
g --local-user other@example.com --armor --detach-sign \
    --output "$here/SHA256SUMS.other.asc" "$here/SHA256SUMS"

# The same signed in text mode (signature type 0x01), which Faraday
# refuses: the release is signed as binary.
g --local-user release@example.com --armor --detach-sign --textmode \
    --output "$here/SHA256SUMS.textmode.asc" "$here/SHA256SUMS"

# Sums that each break one of the listing rules, or keep one in another
# allowed form, each signed by the release key as the release is.
boot=$(sha256sum < "$here/BOOTX64.EFI" | cut -d' ' -f1)
zero=$(printf '0%.0s' $(seq 64))
head="# Faraday 0.9.0, commit 0000000000000000000000000000000000000000"
mkdir -p "$here/variants"
variant() { # name, then the lines
    local name="$1"
    shift
    printf '%s\n' "$@" > "$here/variants/$name.sums"
    g --local-user release@example.com --armor --detach-sign \
        --output "$here/variants/$name.sums.asc" "$here/variants/$name.sums"
}
variant twice-different "$head" "$boot  faraday-0.9.0-BOOTX64.EFI" "$zero  faraday-0.9.0-BOOTX64.EFI"
variant twice-same "$head" "$boot  faraday-0.9.0-BOOTX64.EFI" "$boot  faraday-0.9.0-BOOTX64.EFI"
variant other-version "$head" "$boot  faraday-0.8.0-BOOTX64.EFI"
variant no-version "$boot  faraday-0.9.0-BOOTX64.EFI"
variant version-second "# sums" "$head" "$boot  faraday-0.9.0-BOOTX64.EFI"
variant older "# Faraday 0.1.0, commit 0000000000000000000000000000000000000000" "$boot  faraday-0.1.0-BOOTX64.EFI"
variant binary-mode "$head" "$boot *faraday-0.9.0-BOOTX64.EFI"
variant crlf "$head"$'\r' "$boot  faraday-0.9.0-BOOTX64.EFI"$'\r'

# GnuPG's own verdict on each, for the README.
g --import "$here/release-key.asc"
for sig in SHA256SUMS.asc SHA256SUMS.sha256.asc; do
    g --verify "$here/$sig" "$here/SHA256SUMS" && echo "$sig: good"
done

cd "$here"
sha256sum BOOTX64.EFI SHA256SUMS ./*.asc variants/* faraday-0.1.7/*
