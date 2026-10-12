#!/usr/bin/env bash
# Stages a Faraday release's plain files under out/release/VERSION/, named
# as docs/HUB.md §1.1 names them, with SHA256SUMS for the owner to sign.
#
#   faraday/tools/stage-release.sh VERSION [--any-build] [--out DIR] [--payload DIR]
#   faraday/tools/stage-release.sh VERSION --hub FILE [--out DIR] [--payload DIR]
#
# A release is staged in two passes (HUB §1.3). The first, the form
# above without --hub, stages the plain files and the hub app's payload.
# The hub app is built against that payload (`FARADAY_HUB_PAYLOAD=
# out/payload/VERSION just faraday-linux-bin out/linux out/build/run
# faraday-hub-shell faraday-hub`, which leaves out/linux/faraday-hub), and the second
# pass, `--hub out/linux/faraday-hub` (`just faraday-release-stage-hub
# VERSION`), adds it to the folder the first made: Faraday-VERSION-Linux.tar.gz
# holding one file, Faraday-VERSION-Linux, with its executable bit, made
# as HUB §2 says (--owner=0 --group=0 --numeric-owner --sort=name
# --mtime=@$SOURCE_DATE_EPOCH, gzip -n; SOURCE_DATE_EPOCH is the last
# commit's time unless set), and two more lines in SHA256SUMS: the .tar.gz,
# and the file inside it under its inner name. The second pass refuses a
# hub app that does not carry this payload's MANIFEST, runs the
# published-file check below on the .tar.gz and the file inside it, and
# never touches what the first pass staged.
#
# Takes what the builds left:
#   out/linux/faraday                       -> faraday-desktop-VERSION-linux-x86_64
#       (`just faraday-linux-bin`: the glibc 2.31 container build, HUB §1.2;
#       target/release/faraday needs the build machine's newer glibc)
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
# build. The three builds (stick, Pi, desktop app) must carry the same
# build ID, --any-build or not. Each is read from the file it describes:
# the stick's and the Pi's from the kernel release string on their boot
# partitions (the Pi's in faraday-build.txt, which its image build
# writes: its kernel is compressed), the desktop app's from the build
# string compiled into it. xz runs single-threaded at -9e, so the same
# image always gives the same .xz.
#
# Second, the payload the hub app embeds (HUB §1.3), in out/payload/VERSION/
# (with --out DIR, in DIR.payload; --payload DIR names it): hard links to
# the two image .xz, the desktop app as .xz, and MANIFEST.
#
# Last, the published-file check (HUB §2): every file published, and every
# .img, binary and inner file inside one, goes through `strings`. The
# script refuses one carrying this account's home folder, its user name
# and an @, the computer's host name as a word, /home/<name>/,
# /Users/<name>/ or C:\Users\<name>\, and says which file and which
# string. The values are read here, when it runs; none is written in the
# repository.
set -euo pipefail

die() { printf 'stage-release: %s\n' "$1" >&2; exit 1; }

[ $# -ge 1 ] || die "usage: stage-release.sh VERSION [--any-build] [--out DIR] [--payload DIR] | VERSION --hub FILE [--out DIR] [--payload DIR]"
version="$1"
shift
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "VERSION must look like X.Y.Z, got '$version'"
any=0
out=""
payload=""
hub=""
while [ $# -gt 0 ]; do
    case "$1" in
        --any-build) any=1; shift ;;
        --out) [ $# -ge 2 ] || die "--out needs a directory"; out="$2"; shift 2 ;;
        --payload) [ $# -ge 2 ] || die "--payload needs a directory"; payload="$2"; shift 2 ;;
        --hub) [ $# -ge 2 ] || die "--hub needs the hub app's file"; hub="$2"; shift 2 ;;
        *) die "unknown argument $1" ;;
    esac
done
[ -z "$hub" ] || hub="$(realpath "$hub")"
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
if [ -z "$payload" ]; then
    if [ -n "$out" ]; then payload="$out.payload"; else payload="out/payload/$version"; fi
fi
out="${out:-out/release/$version}"

check="$(mktemp -d)"
trap 'rm -rf "$check"' EXIT
build=""
if [ -z "$hub" ]; then
    desktop=out/linux/faraday
    stick=out/stick/faraday-x86_64-uefi.img
    pi=out/pi/faraday-pi3-waveshare-28dpi.img
    for f in "$desktop" "$stick" "$pi"; do
        [ -f "$f" ] || die "$f: not built"
    done
    for tool in xz strings mtype; do
        command -v "$tool" >/dev/null || die "$tool is not installed"
    done

    # The build ID an image carries, from its kernel's release string.
    build_id() {
        # head ends the pipe at the first match, which grep sees as a broken
        # pipe; with pipefail that would end the script.
        { grep -a -o -m1 -E '[0-9]+\.[0-9]+\.[0-9]+-faraday-[0-9A-Za-z.+-]+' "$1" || true; } \
            | head -1 | sed 's/.*-faraday-//' || true
    }
    # The Pi image's build ID: the release string in faraday-build.txt on
    # its boot partition (the first MBR partition), read as the stick's is.
    pi_build_id() {
        local lba
        lba="$(od -An -t u4 --endian=little -j 454 -N 4 "$1" | tr -d ' ')"
        [ -n "$lba" ] && [ "$lba" -gt 0 ] || die "$1: no boot partition"
        mtype -i "$1@@$((lba * 512))" ::faraday-build.txt 2>/dev/null | build_id /dev/stdin
    }
    # The desktop app's: the build string compiled into it. A release's is
    # the version alone, found by the version being there and no other
    # form; a plain cargo build has none (it carries "(local build)").
    desktop_build_id() {
        local id
        id="$(grep -a -o -m1 -E '[0-9]+\.[0-9]+\.[0-9]+\+[0-9a-f]{12}(\.dirty-[0-9a-f]{8})?\.(test|dev)' "$1" || true)"
        if [ -z "$id" ]; then
            grep -aqF '(local build)' "$1" && return 0
            grep -aqF "$version" "$1" && id="$version"
        fi
        printf '%s' "$id"
    }

    stick_id="$(build_id "$stick")"
    pi_id="$(pi_build_id "$pi")"
    desktop_id="$(desktop_build_id "$desktop")"
    [ -n "$stick_id" ] || die "$stick carries no Faraday build ID"
    [ -n "$pi_id" ] || die "$pi carries no Faraday build ID (faraday-build.txt on its boot partition)"
    [ -n "$desktop_id" ] || die "$desktop carries no Faraday build ID (a plain cargo build, or not this version)"
    echo "$stick: $stick_id"
    echo "$pi: $pi_id"
    echo "$desktop: $desktop_id"
    if [ "$stick_id" != "$pi_id" ] || [ "$stick_id" != "$desktop_id" ]; then
        die "the builds are not one build: stick $stick_id, Pi $pi_id, desktop app $desktop_id"
    fi
    if [ "$any" = 0 ] && [ "$stick_id" != "$version" ]; then
        die "the builds are $stick_id, not the release $version (build them with release=1, or pass --any-build)"
    fi
    build="$stick_id"

    [ ! -e "$out" ] || die "$out exists; move it aside first"
    [ ! -e "$payload" ] || die "$payload exists; move it aside first"
    mkdir -p "$out" "$payload"

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
    for name in "$stick_name" "$pi_name"; do
        xz -d -c "$out/$name.xz" > "$check/$name"
    done
    cp "$out/SHA256SUMS" "$check/"
    (cd "$check" && sha256sum -c --quiet --ignore-missing SHA256SUMS)

    # The payload the hub app embeds (HUB §1.3): the images' .xz as hard
    # links to the staged ones, the desktop app as .xz, and MANIFEST. One
    # line per file, tab-separated: the payload file, its SHA-256, what is
    # inside it, that file's SHA-256 and size, the build ID it carries.
    desktop_xz="$desktop_name.xz"
    echo "Compressing $desktop_xz"
    xz -9e -T1 -c "$desktop" > "$payload/$desktop_xz"
    xz -t "$payload/$desktop_xz"
    ln "$out/$stick_name.xz" "$payload/$stick_name.xz"
    ln "$out/$pi_name.xz" "$payload/$pi_name.xz"
    manifest_line() { # payload file, inner name, inner file
        printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$1" "$(sha256sum < "$payload/$1" | cut -d' ' -f1)" \
            "$2" "$(sha256sum < "$3" | cut -d' ' -f1)" "$(stat -c %s "$3")" "$build"
    }
    {
        echo "# Faraday hub payload $version, commit $commit"
        manifest_line "$stick_name.xz" "$stick_name" "$stick"
        manifest_line "$pi_name.xz" "$pi_name" "$pi"
        manifest_line "$desktop_xz" "$desktop_name" "$desktop"
    } > "$payload/MANIFEST"
    # The payload files decompress to what the manifest says.
    xz -d -c "$payload/$desktop_xz" | cmp - "$desktop"
fi

# The published-file check (HUB §2). The values come from the account and
# the computer this runs on.
home_dir="${HOME:-}"
user_name="$(id -un)"
host_name="$(hostname)"
patterns="$check/patterns"
{
    [ -z "$home_dir" ] || [ "$home_dir" = / ] || printf '%s\n' "$home_dir"
    printf '%s\n' "$user_name@" "/home/$user_name/" "/Users/$user_name/" "C:\\Users\\$user_name\\"
} > "$patterns"
# Prints each forbidden string found in FILE's printable strings.
forbidden_in() {
    local strs="$check/strings"
    strings -a "$1" > "$strs"
    grep -a -i -F -f "$patterns" "$strs" | head -3 || true
    [ -z "$host_name" ] || grep -a -i -w -F "$host_name" "$strs" | head -3 || true
}
# What a refusal takes back: the first pass stages the folders, the
# second only the .tar.gz it adds (the first pass's files stay).
refuse() {
    if [ -n "$hub" ]; then
        rm -f "$out/$tar_name"
        die "$1 (nothing added)"
    fi
    rm -rf "$out" "$payload"
    die "$1 (nothing staged)"
}
tar_name="Faraday-$version-Linux.tar.gz"
inner_name="Faraday-$version-Linux"
# FILE and, inside it, every gzip or xz stream found by its magic bytes
# (a kernel image holds its kernel and the initramfs that way), to a depth
# of three. LABEL names it in what is printed.
scanned=0
scan() {
    local file="$1" label="$2" depth="$3" hit magic off tmp n
    scanned=$((scanned + 1))
    [ -z "${STAGE_RELEASE_VERBOSE:-}" ] || echo "  checking $label"
    hit="$(forbidden_in "$file")"
    [ -z "$hit" ] || refuse "$label carries: $(printf '%s' "$hit" | head -1 | cut -c1-200)"
    [ "$depth" -lt 3 ] || return 0
    for magic in 'gzip:\x1f\x8b\x08' 'xz:\xfd7zXZ\x00'; do
        n=0
        for off in $(LC_ALL=C grep -obUaP "${magic#*:}" "$file" | cut -d: -f1 | head -16); do
            tmp="$(mktemp -p "$check")"
            tail -c +$((off + 1)) "$file" | "${magic%%:*}" -dc > "$tmp" 2>/dev/null || true
            if [ "$(stat -c %s "$tmp")" -gt 65536 ]; then
                scan "$tmp" "$label (${magic%%:*} stream at byte $off)" $((depth + 1))
            fi
            rm -f "$tmp"
        done
    done
}
# The second pass: the Linux hub app (HUB §1.1, §1.2).
add_hub() {
    local first
    [ -f "$hub" ] || die "$hub: not built (just faraday-linux-bin out/linux out/build/run faraday-hub-shell faraday-hub)"
    [ -f "$out/SHA256SUMS" ] && [ -f "$payload/MANIFEST" ] \
        || die "$out and $payload are not a first pass's output: stage the plain files first"
    [ ! -e "$out/$tar_name" ] || die "$out/$tar_name exists; move it aside first"
    # It carries this payload: the MANIFEST's first line names the
    # release and the commit, and is in the binary as a string.
    first="$(head -1 "$payload/MANIFEST")"
    grep -aqF -- "$first" "$hub" || die "$hub does not carry the payload in $payload (its MANIFEST's first line is not in it); build it against that payload"
    # The tar holds one file, with its executable bit and no owner,
    # time or name of the building computer; gzip -n leaves out the
    # name and time of its own.
    local epoch="${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct 2>/dev/null || echo 0)}"
    mkdir "$check/hub"
    install -m 0755 "$hub" "$check/hub/$inner_name"
    tar --format=gnu --owner=0 --group=0 --numeric-owner --sort=name \
        --mtime="@$epoch" -C "$check/hub" -cf - "$inner_name" \
        | gzip -n -9 > "$out/$tar_name"
    # What a person gets by extracting it: one file, executable, equal
    # to the hub app.
    mkdir "$check/hub-out"
    tar -C "$check/hub-out" -xzf "$out/$tar_name"
    [ "$(ls -A "$check/hub-out")" = "$inner_name" ] || refuse "$tar_name does not hold exactly $inner_name"
    [ -x "$check/hub-out/$inner_name" ] || refuse "$inner_name has no executable bit in $tar_name"
    cmp -s "$hub" "$check/hub-out/$inner_name" || refuse "$inner_name in $tar_name differs from $hub"
    # The published-file check covers the .tar.gz and the file inside it.
    scan_tar "$out/$tar_name"
    # SHA256SUMS lists the .tar.gz and the file inside it under its inner
    # name, so either can be checked.
    {
        (cd "$out" && sha256sum "$tar_name")
        printf '%s  %s\n' "$(sha256sum < "$check/hub-out/$inner_name" | cut -d' ' -f1)" "$inner_name"
    } >> "$out/SHA256SUMS"
    (cd "$out" && sha256sum -c --quiet --ignore-missing SHA256SUMS)
    cp "$out/SHA256SUMS" "$check/hub-out/"
    (cd "$check/hub-out" && sha256sum -c --quiet --ignore-missing SHA256SUMS)
    echo "Added $tar_name ($(stat -c %s "$out/$tar_name") bytes) holding $inner_name ($(stat -c %s "$hub") bytes), carrying: $first"
}
# A .tar.gz, its tar and each file extracted from it, scanned.
scan_tar() {
    local f="$1" g
    gzip -dc "$f" > "$check/inner"
    scan "$check/inner" "$f" 0
    mkdir "$check/untar"
    tar -C "$check/untar" -xf "$f"
    for g in "$check"/untar/*; do scan "$g" "$f: ${g#"$check"/untar/}" 0; done
    rm -rf "$check/untar"
}
if [ -n "$hub" ]; then
    add_hub
else
for f in "$out"/* "$payload"/*; do
    # The payload's image .xz are the release's own files.
    if [ "$(dirname "$f")" = "$payload" ] && [ -e "$out/$(basename "$f")" ]; then continue; fi
    case "$f" in
        *.xz)
            xz -d -c "$f" > "$check/inner"
            scan "$check/inner" "$f" 0 ;;
        *.tar.gz) scan_tar "$f" ;;
        *) scan "$f" "$f" 0 ;;
    esac
done
fi
echo "Checked $scanned files and streams for the home folder, user name, host name and /home, /Users and C:\\Users paths: none found."

echo
cat "$out/SHA256SUMS"
echo
ls -l "$out"
echo "Staged in $out. Sign SHA256SUMS in Faraday (GPG -> Sign a file)."
