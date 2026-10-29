#!/usr/bin/env bash
# Stages a Faraday release's plain files under out/release/VERSION/, named
# as docs/STARTER.md §1.1 names them, with SHA256SUMS for the owner to sign.
#
#   faraday/tools/stage-release.sh VERSION [--any-build] [--out DIR] [--payload DIR] [--builds DIR]
#   faraday/tools/stage-release.sh VERSION [--starter FILE] [--starter-windows FILE] [--out DIR] [--payload DIR]
#
# A release is staged in two passes (STARTER §1.3). The first, the form
# above without --starter, stages the plain files and the Starter's payload.
# The Starter is built against that payload (`FARADAY_STARTER_PAYLOAD=
# out/payload/VERSION/linux just faraday-linux-bin out/linux out/build/run
# faraday-starter-shell faraday-starter`, which leaves out/linux/faraday-starter;
# the Windows Starter against out/payload/VERSION/windows), and the second
# pass, `--starter out/linux/faraday-starter` (`just faraday-release-stage-starter
# VERSION`), adds it to the folder the first made: Faraday-Starter-VERSION-Linux.tar.gz
# holding one file, Faraday-Starter-VERSION-Linux, with its executable bit, made
# as STARTER §2 says (--owner=0 --group=0 --numeric-owner --sort=name
# --mtime=@$SOURCE_DATE_EPOCH, gzip -n; SOURCE_DATE_EPOCH is the last
# commit's time unless set), and two more lines in SHA256SUMS: the .tar.gz,
# and the file inside it under its inner name. The second pass refuses a
# Starter that does not carry this payload's MANIFEST, runs the
# published-file check below on the .tar.gz and the file inside it, and
# never touches what the first pass staged. `--starter-windows
# out/windows/faraday-starter.exe` adds the Windows Starter the same way,
# in the same pass or another: Faraday-Starter-VERSION-Windows.exe, as it
# is, and its line in SHA256SUMS, after the same two checks.
#
# Takes every image faraday/builds.toml lists as built with release files
# (docs/IMAGES.md §4), from where its build leaves it (its first outputs
# entry, under out/ or the folder --builds names), and publishes it under
# its release name: today the PC stick's Strict and Compatible, the Pi's
# 2.8-inch Strict and Compatible and the Pi's HDMI screen's Compatible,
#   out/stick/faraday-x86_64-uefi-strict.img -> faraday-stick-VERSION-x86_64-uefi-strict.img.xz
#   out/pi/faraday-pi3-hdmi-compatible.img   -> faraday-pi-VERSION-hdmi-compatible.img.xz
# and so on. An image that releases a boot program (.EFI) has a GPT whose
# first partition (FARADAYBOOT) holds EFI/BOOT/BOOTX64.EFI, published as
# its release name, faraday-VERSION-BOOTX64-strict.EFI and
# faraday-VERSION-BOOTX64-compatible.EFI: the boot programs as they are,
# unsigned, published so that a person with Secure Boot keys downloads one
# with SHA256SUMS and SHA256SUMS.asc and signs it on Faraday, which signs
# only a boot program those sums list (docs/decisions/F134.md). It must
# carry the images' build ID.
# Faraday Demo is neither a release file nor a payload file: it runs inside
# the Starter's window, compiled into the Starter (docs/decisions/F126.md),
# so no faraday-demo build is taken or staged.
#
# Images are published as .xz only (F11). SHA256SUMS lists each file
# published, the image inside each .xz under its .img name, and each
# image's boot partition (its first partition, GPT or MBR) under its name
# with -boot.vfat for .img, so `sha256sum -c --ignore-missing SHA256SUMS`
# checks a download, once decompressed the image, and a card or stick
# already written by its boot partition, which nothing writes to. Its
# first line names the version and the commit, as earlier releases' did.
#
# IMAGES, the release's image list (IMAGES §4), is staged beside them and
# listed in SHA256SUMS: a first line naming the version and the commit,
# then one line per image, in builds.toml's order, its fields separated by
# one tab: the .img.xz, its SHA-256, the .img's SHA-256, the platform (the
# entry's board), the screen (its panel), the build (strict or compatible)
# and the boards it starts (its starts, joined by ", ").
#
# The images must carry VERSION as their build ID (a release build,
# DECISIONS F5); --any-build skips that, for trying the script on a test
# build. The images must all carry the same build ID, --any-build or not,
# and a refusal names each image's. Each is read from the kernel release
# string: a GPT image's in its bytes, an MBR image's (the Pi's) in
# faraday-build.txt on its boot partition, which its image build writes:
# its kernel is compressed. xz runs single-threaded at -9e, so the same
# image always gives the same .xz.
#
# Second, the payload the Starter embeds (STARTER §1.3), in out/payload/VERSION/
# (with --out DIR, in DIR.payload; --payload DIR names it): one folder per
# system, linux/ and windows/, each with hard links to the Strict stick's and
# the 2.8-inch Strict Pi's image .xz (builds stick-strict and
# pi3-waveshare-28dpi; the other images are released, not embedded, until
# the Starter stops carrying images, IMAGES §9 step 6)
# and its own MANIFEST (the first line is the same in both). The two
# folders stay apart: each Starter embeds its own.
#
# Before it finishes, either pass refuses a release folder that holds a
# file whose name starts faraday-demo- (F83, F126), and names it.
#
# Last, the published-file check (STARTER §2): every file published, and every
# .img, binary and inner file inside one, goes through `strings`. The
# script refuses one carrying this account's home folder, its user name
# and an @, the computer's host name as a word, /home/<name>/,
# /Users/<name>/ or C:\Users\<name>\, and says which file and which
# string. The values are read here, when it runs; none is written in the
# repository.
set -euo pipefail

die() { printf 'stage-release: %s\n' "$1" >&2; exit 1; }

[ $# -ge 1 ] || die "usage: stage-release.sh VERSION [--any-build] [--out DIR] [--payload DIR] [--builds DIR] | VERSION [--starter FILE] [--starter-windows FILE] [--out DIR] [--payload DIR]"
version="$1"
shift
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "VERSION must look like X.Y.Z, got '$version'"
any=0
out=""
payload=""
builds=""
starter=""
starter_win=""
while [ $# -gt 0 ]; do
    case "$1" in
        --any-build) any=1; shift ;;
        --out) [ $# -ge 2 ] || die "--out needs a directory"; out="$2"; shift 2 ;;
        --payload) [ $# -ge 2 ] || die "--payload needs a directory"; payload="$2"; shift 2 ;;
        --builds) [ $# -ge 2 ] || die "--builds needs a directory"; builds="$(realpath "$2")"; shift 2 ;;
        --starter) [ $# -ge 2 ] || die "--starter needs the Starter's file"; starter="$2"; shift 2 ;;
        --starter-windows) [ $# -ge 2 ] || die "--starter-windows needs the Starter's file"; starter_win="$2"; shift 2 ;;
        *) die "unknown argument $1" ;;
    esac
done
[ -z "$starter" ] || starter="$(realpath "$starter")"
[ -z "$starter_win" ] || starter_win="$(realpath "$starter_win")"
second=""
[ -z "$starter$starter_win" ] || second=1
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
if [ -z "$payload" ]; then
    if [ -n "$out" ]; then payload="$out.payload"; else payload="out/payload/$version"; fi
fi
out="${out:-out/release/$version}"

check="$(mktemp -d)"
trap 'rm -rf "$check"' EXIT
build=""
if [ -z "$second" ]; then
    builds="${builds:-out}"
    for tool in python3 xz strings mtype mcopy; do
        command -v "$tool" >/dev/null || die "$tool is not installed"
    done

    # The images a release publishes (IMAGES §4), read from builds.toml:
    # one line each, tab-separated: the build's id, its file under out/, its
    # .img.xz release name, its .EFI release name or -, its board, panel,
    # build, and the boards it starts.
    listed="$(python3 -I - "$version" faraday/builds.toml <<'PY'
import sys, tomllib
version, path = sys.argv[1], sys.argv[2]
with open(path, "rb") as f:
    builds = tomllib.load(f)["build"]
for b in builds:
    if b.get("group") != "image" or b.get("status") != "built" or not b.get("release"):
        continue
    names = [n.replace("{version}", version) for n in b["release"]]
    print("\t".join([b["id"], b["outputs"][0].replace("{dev}", "").removeprefix("out/"),
                     names[0], names[1] if len(names) > 1 else "-", b["board"], b["panel"],
                     b["build"], ", ".join(b["starts"])]))
PY
)" || die "cannot read the released images from faraday/builds.toml"
    [ -n "$listed" ] || die "faraday/builds.toml lists no built image with release files"
    ids=() src=() img=() efi=() board=() panel=() kind=() starts=() part=()
    while IFS=$'\t' read -r i s n e bo pa k st; do
        ids+=("$i") src+=("$builds/$s") img+=("${n%.xz}") efi+=("$e")
        board+=("$bo") panel+=("$pa") kind+=("$k") starts+=("$st")
    done <<< "$listed"

    is_gpt() { [ "$(dd if="$1" bs=1 skip=512 count=8 2>/dev/null)" = "EFI PART" ]; }
    # An image's first partition, its boot partition: prints its first
    # sector and its length in sectors, from the GPT's first entry (the
    # header at LBA 1 names where the entries start; an entry's first and
    # last LBA are 32 and 40 bytes into it) or the MBR's.
    first_partition() {
        local img="$1" entries start end count size
        if is_gpt "$img"; then
            entries="$(od -An -t u8 --endian=little -j 584 -N 8 "$img" | tr -d ' ')"
            [ -n "$entries" ] || return 1
            start="$(od -An -t u8 --endian=little -j $((entries * 512 + 32)) -N 8 "$img" | tr -d ' ')"
            end="$(od -An -t u8 --endian=little -j $((entries * 512 + 40)) -N 8 "$img" | tr -d ' ')"
            [ -n "$start" ] && [ -n "$end" ] && [ "$end" -ge "$start" ] || return 1
            count=$((end - start + 1))
        else
            start="$(od -An -t u4 --endian=little -j 454 -N 4 "$img" | tr -d ' ')"
            count="$(od -An -t u4 --endian=little -j 458 -N 4 "$img" | tr -d ' ')"
            [ -n "$start" ] && [ -n "$count" ] || return 1
        fi
        size="$(stat -c %s "$img")"
        [ "$start" -gt 0 ] && [ "$count" -gt 0 ] && [ $(((start + count) * 512)) -le "$size" ] || return 1
        echo "$start $count"
    }
    # The bytes of an image's boot partition, given as first_partition prints it.
    boot_bytes() { # the image, "start count"
        dd if="$1" bs=1M iflag=skip_bytes,count_bytes skip=$((${2% *} * 512)) \
            count=$((${2#* } * 512)) status=none
    }
    # The build ID an image carries, from its kernel's release string.
    build_id() {
        # head ends the pipe at the first match, which grep sees as a broken
        # pipe; with pipefail that would end the script.
        { grep -a -o -m1 -E '[0-9]+\.[0-9]+\.[0-9]+-faraday-[0-9A-Za-z.+-]+' "$1" || true; } \
            | head -1 | sed 's/.*-faraday-//' || true
    }
    # A GPT image carries it in its bytes; an MBR image (the Pi's, whose
    # kernel is compressed) in faraday-build.txt on its boot partition.
    image_build_id() { # the image, its boot partition
        if is_gpt "$1"; then
            build_id "$1"
        else
            mtype -i "$1@@$((${2% *} * 512))" ::faraday-build.txt 2>/dev/null | build_id /dev/stdin
        fi
    }

    every=""
    for k in "${!ids[@]}"; do
        f="${src[k]}"
        [ -f "$f" ] || die "$f: not built (${ids[k]} in faraday/builds.toml)"
        [ "${efi[k]}" = - ] || is_gpt "$f" || die "$f has no GPT"
        part[k]="$(first_partition "$f")" || die "$f has no boot partition"
        b="$(image_build_id "$f" "${part[k]}")"
        [ -n "$b" ] || die "$f carries no Faraday build ID"
        echo "$f: $b"
        every="${every:+$every, }${ids[k]} $b"
        if [ -z "$build" ]; then build="$b"; elif [ "$b" != "$build" ]; then build="-"; fi
    done
    [ "$build" != - ] || die "the builds are not one build: $every"
    if [ "$any" = 0 ] && [ "$build" != "$version" ]; then
        die "the builds are $build, not the release $version (build them with release=1, or pass --any-build)"
    fi

    [ ! -e "$out" ] || die "$out exists; move it aside first"
    [ ! -e "$payload" ] || die "$payload exists; move it aside first"
    mkdir -p "$out" "$payload/linux" "$payload/windows"

    # The boot program off a GPT image's boot partition. A refusal here
    # takes back the folders just made.
    unstage() { rm -rf "$out" "$payload"; die "$1 (nothing staged)"; }
    for k in "${!ids[@]}"; do
        [ "${efi[k]}" != - ] || continue
        dest="$out/${efi[k]}"
        mcopy -i "${src[k]}@@$((${part[k]% *} * 512))" ::EFI/BOOT/BOOTX64.EFI "$dest" 2>/dev/null \
            || unstage "${src[k]} has no EFI/BOOT/BOOTX64.EFI on its boot partition"
        [ "$(build_id "$dest")" = "$build" ] \
            || unstage "$(basename "$dest") does not carry the build ID $build"
    done

    xz_sum=() img_sum=() boot_sum=()
    for k in "${!ids[@]}"; do
        echo "Compressing ${img[k]}.xz"
        xz -9e -T1 -c "${src[k]}" > "$out/${img[k]}.xz"
        xz -t "$out/${img[k]}.xz"
        xz_sum[k]="$(sha256sum < "$out/${img[k]}.xz" | cut -d' ' -f1)"
        img_sum[k]="$(sha256sum < "${src[k]}" | cut -d' ' -f1)"
        boot_sum[k]="$(boot_bytes "${src[k]}" "${part[k]}" | sha256sum | cut -d' ' -f1)"
    done

    commit="$(git rev-parse HEAD)"
    # The release's image list (IMAGES §4).
    {
        echo "# Faraday $version images, commit $commit"
        for k in "${!ids[@]}"; do
            printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "${img[k]}.xz" "${xz_sum[k]}" "${img_sum[k]}" \
                "${board[k]}" "${panel[k]}" "${kind[k]}" "${starts[k]}"
        done
    } > "$out/IMAGES"
    {
        echo "# Faraday $version, commit $commit"
        for k in "${!ids[@]}"; do printf '%s  %s\n' "${xz_sum[k]}" "${img[k]}.xz"; done
        for k in "${!ids[@]}"; do
            [ "${efi[k]}" = - ] || (cd "$out" && sha256sum "${efi[k]}")
        done
        (cd "$out" && sha256sum IMAGES)
        for k in "${!ids[@]}"; do printf '%s  %s\n' "${img_sum[k]}" "${img[k]}"; done
        for k in "${!ids[@]}"; do printf '%s  %s\n' "${boot_sum[k]}" "${img[k]%.img}-boot.vfat"; done
    } > "$out/SHA256SUMS"

    # What a person does with the files must check out before anything is
    # signed: the sums of what is published, and of each image decompressed
    # and its boot partition, one image at a time.
    (cd "$out" && sha256sum -c --quiet --ignore-missing SHA256SUMS)
    cp "$out/SHA256SUMS" "$check/"
    for k in "${!ids[@]}"; do
        xz -d -c "$out/${img[k]}.xz" > "$check/${img[k]}"
        boot_bytes "$check/${img[k]}" "${part[k]}" > "$check/${img[k]%.img}-boot.vfat"
        (cd "$check" && sha256sum -c --quiet --ignore-missing SHA256SUMS)
        rm -f "$check/${img[k]}" "$check/${img[k]%.img}-boot.vfat"
    done

    # The payload the Starter embeds (STARTER §1.3), one folder per system:
    # the images' .xz as hard links to the staged ones, and MANIFEST. One
    # line per file, tab-separated: the payload file, its SHA-256, what is
    # inside it, that file's SHA-256 and size, the build ID it carries.
    # Both MANIFESTs start with the same line. The Strict stick and the
    # 2.8-inch Strict Pi, until the Starter carries no images (IMAGES §9
    # step 6).
    carried=()
    for want in stick-strict pi3-waveshare-28dpi; do
        found=""
        for k in "${!ids[@]}"; do [ "${ids[k]}" != "$want" ] || found="$k"; done
        [ -n "$found" ] || unstage "faraday/builds.toml releases no $want, which the Starter carries"
        carried+=("$found")
    done
    for system in linux windows; do
        dir="$payload/$system"
        {
            echo "# Faraday Starter payload $version, commit $commit"
            for k in "${carried[@]}"; do
                ln "$out/${img[k]}.xz" "$dir/${img[k]}.xz"
                printf '%s\t%s\t%s\t%s\t%s\t%s\n' "${img[k]}.xz" "${xz_sum[k]}" "${img[k]}" \
                    "${img_sum[k]}" "$(stat -c %s "${src[k]}")" "$build"
            done
        } > "$dir/MANIFEST"
    done
fi

# The published-file check (STARTER §2). The values come from the account and
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
# second only the files it adds (the first pass's files stay).
refuse() {
    if [ -n "$second" ]; then
        rm -f "$out/$tar_name" "$out/$exe_name"
        die "$1 (nothing added)"
    fi
    rm -rf "$out" "$payload"
    die "$1 (nothing staged)"
}
starter_release_name="Faraday-Starter" # the one place the release files are named (F16)
tar_name="$starter_release_name-$version-Linux.tar.gz"
inner_name="$starter_release_name-$version-Linux"
exe_name="$starter_release_name-$version-Windows.exe"
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
# A Starter the second pass adds must sit on a first pass's output and
# carry its payload: the MANIFEST's first line names the release and the
# commit, and is in the binary as a string. Prints that line.
starter_payload() { # the Starter's file, the name it is published as, its system's folder
    [ -f "$out/SHA256SUMS" ] && [ -f "$payload/$3/MANIFEST" ] \
        || die "$out and $payload/$3 are not a first pass's output: stage the plain files first"
    [ ! -e "$out/$2" ] || die "$out/$2 exists; move it aside first"
    local first
    first="$(head -1 "$payload/$3/MANIFEST")"
    grep -aqF -- "$first" "$1" || die "$1 does not carry the payload in $payload/$3 (its MANIFEST's first line is not in it); build it against that payload"
    printf '%s' "$first"
}
# The second pass: the Linux Starter (STARTER §1.1, §1.2).
add_starter() {
    local first
    [ -f "$starter" ] || die "$starter: not built (just faraday-linux-bin out/linux out/build/run faraday-starter-shell faraday-starter)"
    first="$(starter_payload "$starter" "$tar_name" linux)"
    # The tar holds one file, with its executable bit and no owner,
    # time or name of the building computer; gzip -n leaves out the
    # name and time of its own.
    local epoch="${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct 2>/dev/null || echo 0)}"
    mkdir "$check/starter"
    install -m 0755 "$starter" "$check/starter/$inner_name"
    tar --format=gnu --owner=0 --group=0 --numeric-owner --sort=name \
        --mtime="@$epoch" -C "$check/starter" -cf - "$inner_name" \
        | gzip -n -9 > "$out/$tar_name"
    # What a person gets by extracting it: one file, executable, equal
    # to the Starter.
    mkdir "$check/starter-out"
    tar -C "$check/starter-out" -xzf "$out/$tar_name"
    [ "$(ls -A "$check/starter-out")" = "$inner_name" ] || refuse "$tar_name does not hold exactly $inner_name"
    [ -x "$check/starter-out/$inner_name" ] || refuse "$inner_name has no executable bit in $tar_name"
    cmp -s "$starter" "$check/starter-out/$inner_name" || refuse "$inner_name in $tar_name differs from $starter"
    # The published-file check covers the .tar.gz and the file inside it.
    scan_tar "$out/$tar_name"
    # SHA256SUMS lists the .tar.gz and the file inside it under its inner
    # name, so either can be checked.
    {
        (cd "$out" && sha256sum "$tar_name")
        printf '%s  %s\n' "$(sha256sum < "$check/starter-out/$inner_name" | cut -d' ' -f1)" "$inner_name"
    } >> "$out/SHA256SUMS"
    (cd "$out" && sha256sum -c --quiet --ignore-missing SHA256SUMS)
    cp "$out/SHA256SUMS" "$check/starter-out/"
    (cd "$check/starter-out" && sha256sum -c --quiet --ignore-missing SHA256SUMS)
    echo "Added $tar_name ($(stat -c %s "$out/$tar_name") bytes) holding $inner_name ($(stat -c %s "$starter") bytes), carrying: $first"
}
# The Windows Starter (STARTER §1.1): published as it is.
add_starter_windows() {
    local first
    [ -f "$starter_win" ] || die "$starter_win: not built (just faraday-windows-bin)"
    first="$(starter_payload "$starter_win" "$exe_name" windows)"
    cp "$starter_win" "$out/$exe_name"
    scan "$out/$exe_name" "$out/$exe_name" 0
    (cd "$out" && sha256sum "$exe_name") >> "$out/SHA256SUMS"
    (cd "$out" && sha256sum -c --quiet --ignore-missing SHA256SUMS)
    echo "Added $exe_name ($(stat -c %s "$out/$exe_name") bytes), carrying: $first"
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
# Faraday Demo is not published on its own (F83) and runs inside the
# Starter (F126): a release folder holding a desktop app is refused,
# whichever pass put it there, before the second pass adds anything.
for f in "$out"/faraday-demo-*; do
    [ -e "$f" ] || continue
    refuse "$f is a desktop app, which a release does not publish (F83): Faraday Demo is inside the Starter (F126)"
done
if [ -n "$second" ]; then
    [ -z "$starter" ] || add_starter
    [ -z "$starter_win" ] || add_starter_windows
else
for f in "$out"/* "$payload"/*/*; do
    # The payload's image .xz are the release's own files.
    case "$f" in "$payload"/*/*) [ ! -e "$out/$(basename "$f")" ] || continue ;; esac
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
