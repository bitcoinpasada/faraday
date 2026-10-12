#!/usr/bin/env bash
# Stages a Faraday release's plain files under out/release/VERSION/, named
# as docs/HUB.md §1.1 names them, with SHA256SUMS for the owner to sign.
#
#   faraday/tools/stage-release.sh VERSION [--any-build] [--out DIR]
#
# Takes what the builds left:
#   target/release/faraday                  -> faraday-desktop-VERSION-linux-x86_64
#   out/stick/faraday-x86_64-uefi.img       -> faraday-stick-VERSION-x86_64-uefi.img.xz
#   out/pi/faraday-pi3-waveshare-28dpi.img  -> faraday-pi-VERSION-pi3-waveshare-28dpi.img.xz
#
# Images are published as .xz only (F11). SHA256SUMS lists each file
# published and, under its .img name, the image inside each .xz, so
# `sha256sum -c --ignore-missing SHA256SUMS` checks a download and, once
# decompressed, the image. Its first line names the version and the
# commit, as earlier releases' did.
#
# The stick image must carry VERSION as its build ID (a release build,
# DECISIONS F5); --any-build skips that, for trying the script on a test
# build. The Pi image's build ID cannot be read from its bytes yet. xz runs single-threaded at -9e, so the same image always gives
# the same .xz.
set -euo pipefail

die() { printf 'stage-release: %s\n' "$1" >&2; exit 1; }

[ $# -ge 1 ] || die "usage: stage-release.sh VERSION [--any-build] [--out DIR]"
version="$1"
shift
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "VERSION must look like X.Y.Z, got '$version'"
any=0
out=""
while [ $# -gt 0 ]; do
    case "$1" in
        --any-build) any=1; shift ;;
        --out) [ $# -ge 2 ] || die "--out needs a directory"; out="$2"; shift 2 ;;
        *) die "unknown argument $1" ;;
    esac
done
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
out="${out:-out/release/$version}"

desktop=target/release/faraday
stick=out/stick/faraday-x86_64-uefi.img
pi=out/pi/faraday-pi3-waveshare-28dpi.img
for f in "$desktop" "$stick" "$pi"; do
    [ -f "$f" ] || die "$f: not built"
done

# The build ID an image carries, from its kernel's release string.
build_id() {
    # head ends the pipe at the first match, which grep sees as a broken
    # pipe; with pipefail that would end the script.
    { grep -a -o -m1 -E '[0-9]+\.[0-9]+\.[0-9]+-faraday-[0-9A-Za-z.+-]+' "$1" || true; } \
        | head -1 | sed 's/.*-faraday-//' || true
}
# The Pi's kernel is a compressed zImage, so its release string is not
# in the image's bytes and the Pi image's build cannot be read here; the
# stick image's is. HANDOFF item 70 has the Pi image carry its build ID
# where it can be read.
echo "$pi: build ID not readable (compressed kernel); check it was built from $(git rev-parse --short=12 HEAD)"
for img in "$stick"; do
    id="$(build_id "$img")"
    [ -n "$id" ] || die "$img carries no Faraday build ID"
    if [ "$any" = 0 ] && [ "$id" != "$version" ]; then
        die "$img is build $id, not the release $version (build it with release=1, or pass --any-build)"
    fi
    echo "$img: $id"
done

[ ! -e "$out" ] || die "$out exists; move it aside first"
mkdir -p "$out"

stick_name="faraday-stick-$version-x86_64-uefi.img"
pi_name="faraday-pi-$version-pi3-waveshare-28dpi.img"
desktop_name="faraday-desktop-$version-linux-x86_64"

cp "$desktop" "$out/$desktop_name"
for pair in "$stick:$stick_name" "$pi:$pi_name"; do
    src="${pair%%:*}"
    name="${pair#*:}"
    echo "Compressing $name.xz"
    xz -9e -T1 -c "$src" > "$out/$name.xz"
    xz -t "$out/$name.xz"
done

commit="$(git rev-parse HEAD)"
{
    echo "# Faraday $version, commit $commit"
    (cd "$out" && sha256sum "$desktop_name" "$stick_name.xz" "$pi_name.xz")
    printf '%s  %s\n' "$(sha256sum < "$stick" | cut -d' ' -f1)" "$stick_name"
    printf '%s  %s\n' "$(sha256sum < "$pi" | cut -d' ' -f1)" "$pi_name"
} > "$out/SHA256SUMS"

# What a person does with the files must check out before anything is
# signed: the sums of what is published, and of each image decompressed.
(cd "$out" && sha256sum -c --quiet --ignore-missing SHA256SUMS)
check="$(mktemp -d)"
trap 'rm -rf "$check"' EXIT
for name in "$stick_name" "$pi_name"; do
    xz -d -c "$out/$name.xz" > "$check/$name"
done
cp "$out/SHA256SUMS" "$check/"
(cd "$check" && sha256sum -c --quiet --ignore-missing SHA256SUMS)

echo
cat "$out/SHA256SUMS"
echo
ls -l "$out"
echo "Staged in $out. Sign SHA256SUMS in Faraday (GPG -> Sign a file)."
