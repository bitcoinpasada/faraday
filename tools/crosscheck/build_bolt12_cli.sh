#!/usr/bin/env bash
# Builds Core Lightning's `devtools/bolt12-cli` from the pinned source release
# (tools/reference/bolt12/release.pin; `fetch_cln.py` keeps the archive) into
# the folder given, as `<folder>/bolt12-cli`. Offline. The archive is checked
# against the pin, unpacked into `<folder>/cln-<hash>` and built there with
# Core Lightning's own `./configure` and `make`; a build already made from
# this archive is kept. Needs what Core Lightning's build needs
# (docs/DEVELOPING.md §1).
#
#   tools/crosscheck/build_bolt12_cli.sh out/crosscheck
set -euo pipefail
cd "$(dirname "$0")/../.."
root=$PWD
out=$(realpath -m "${1:?the folder to build in}")
pin=$root/tools/reference/bolt12/release.pin
name=$(awk '$1 == "sha256" { print $2 }' "$pin")
want=$(awk '$1 == "sha256" { print $3 }' "$pin")
version=$(awk '$1 == "version" { print $2 }' "$pin")
archive="${FARADAY_CLN_SOURCE:-$HOME/.cache/faraday/core-lightning}/$name"
if [ ! -f "$archive" ]; then
    echo "no $archive: python3 -I -B tools/crosscheck/fetch_cln.py (docs/DEVELOPING.md)" >&2
    exit 2
fi
got=$(sha256sum "$archive" | cut -d' ' -f1)
if [ "$got" != "$want" ]; then
    echo "$archive hashes to $got, tools/reference/bolt12/release.pin says $want" >&2
    exit 2
fi
for tool in unzip make cc python3 lowdown jq; do
    command -v "$tool" >/dev/null || { echo "no $tool (docs/DEVELOPING.md §1)" >&2; exit 2; }
done
python3 -c 'import mako' 2>/dev/null || { echo "no python3-mako (docs/DEVELOPING.md §1)" >&2; exit 2; }
mkdir -p "$out"
src="$out/cln-${want:0:12}"
if [ ! -x "$src/devtools/bolt12-cli" ]; then
    # A half-made folder from an earlier stop is moved aside, not reused.
    [ ! -e "$src" ] || mv "$src" "$src.old.$$"
    tmp=$(mktemp -d "$out/cln-unpack.XXXXXX")
    unzip -q "$archive" -d "$tmp"
    mv "$tmp"/clightning-* "$src"
    cd "$src"
    # The release has no git history: the version is the tag's.
    export BUILD_VERSION=$version
    ./configure --disable-rust --disable-valgrind >"$out/cln-configure.log" 2>&1 \
        || { tail -30 "$out/cln-configure.log" >&2; exit 2; }
    make -j"$(nproc)" devtools/bolt12-cli >"$out/cln-make.log" 2>&1 \
        || { tail -30 "$out/cln-make.log" >&2; exit 2; }
fi
cp "$src/devtools/bolt12-cli" "$out/bolt12-cli"
