#!/usr/bin/env bash
# Builds the Windows hub app inside the container
# (faraday/build/windows/Containerfile) for x86_64-pc-windows-gnu, and
# checks it. `just faraday-windows-bin` starts it; there is nothing here
# to run on the host.
#
#   build.sh            the release build, into /out/Faraday.exe
#   build.sh clippy     clippy for the Windows target, warnings as errors
#
# Container paths: /src is the checkout (read-only), /out and /build are
# bind mounts, /cargo the registry cache. FARADAY_BUILD comes from the
# host (faraday/image/build-id.sh).
set -euo pipefail

target=x86_64-pc-windows-gnu
die() { printf 'build.sh: %s\n' "$1" >&2; exit 1; }

cd /src
want="$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)"
have="$(rustc --version | cut -d' ' -f2)"
[ "$want" = "$have" ] || die "rust-toolchain.toml says $want, the container has $have"

if [ "${1:-}" = clippy ]; then
    exec cargo clippy --locked --target "$target" -p faraday-hub-shell -- -D warnings
fi

: "${FARADAY_BUILD:?FARADAY_BUILD is not set}"
export FARADAY_BUILD

# No local path in the binary: the same rewriting as every other build of
# something published (faraday/image/path-remap.sh), with this container's
# home (/root), cargo home and source tree; and the target directory,
# where build scripts write what is compiled in.
export HOME=/root RUST_TARGET="$target"
. /src/faraday/image/path-remap.sh
export RUSTFLAGS="$RUSTFLAGS --remap-path-prefix=$CARGO_TARGET_DIR=/target"
# The symbols are not needed to run it, and are most of its size.
export CARGO_PROFILE_RELEASE_STRIP=symbols

cargo build --release --locked --target "$target" -p faraday-hub-shell --bin faraday-hub
install -m 0644 "$CARGO_TARGET_DIR/$target/release/faraday-hub.exe" /out/Faraday.exe
exe=/out/Faraday.exe

# Only Windows' own libraries: nothing of mingw-w64's (libgcc_s,
# libwinpthread, libstdc++) may be needed beside it.
dlls="$(x86_64-w64-mingw32-objdump -p "$exe" | sed -n 's/^\s*DLL Name: //p' | sort -u)"
echo "Faraday.exe imports: $(echo "$dlls" | tr '\n' ' ')"
bad="$(echo "$dlls" | grep -iE '^(lib|mingw)' || true)"
[ -z "$bad" ] || die "Faraday.exe needs a library Windows does not have: $bad"

# Its resources: the manifest asks for no elevation.
# (Not grep -q: it would end the pipe early, and pipefail would take that
# for a failure.)
[ "$(strings -a "$exe" | grep -c 'level="asInvoker"' || true)" -gt 0 ] || die "Faraday.exe carries no asInvoker manifest"

# Nothing of the container's own paths, nor a home folder, may be left in
# it. (grep -m1 would end the pipe early, and pipefail would take that for
# a failure.) Both 8-bit and 16-bit little-endian strings.
hit="$( { strings -a "$exe"; strings -a -el "$exe"; } \
    | grep -E '(^|[^[:alnum:]_.-])/(src|root|payload)/|/build/target|/cargo/registry|/home/|/Users/|[A-Za-z]:\\Users\\' \
    | head -1 || true)"
[ -z "$hit" ] || die "Faraday.exe carries a path: $hit"
echo "Faraday.exe: $(stat -c %s "$exe") bytes, no path in its strings"
