#!/usr/bin/env bash
# Builds the device's binaries for one board inside the container
# (faraday/build/device/Containerfile). `just faraday-stick-bin` and `just
# faraday-pi-bin` start it; there is nothing here to run on the host.
#
#   build.sh BOARD
#
# The binaries land in /out. Container paths: /src is the checkout
# (read-only), /out and /build are bind mounts, /cargo the registry cache.
# FARADAY_BUILD comes from the host (faraday/image/build-id.sh).
set -euo pipefail

board="${1:?build.sh BOARD}"

die() { printf 'build.sh: %s\n' "$1" >&2; exit 1; }

cd /src
want="$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)"
have="$(rustc --version | cut -d' ' -f2)"
[ "$want" = "$have" ] || die "rust-toolchain.toml says $want, the container has $have"
: "${FARADAY_BUILD:?FARADAY_BUILD is not set}"
export FARADAY_BUILD

# The board names the Rust target and the C compiler.
. "faraday/image/boards/$board/board.conf"
under="${RUST_TARGET//-/_}"
upper="$(echo "$under" | tr 'a-z' 'A-Z')"
export "CC_${under}=$CC"
export "AR_${under}=${CC%gcc}ar"
export "CARGO_TARGET_${upper}_LINKER=$CC"
export "CFLAGS_${under}=$CFLAGS"
export RUSTFLAGS="-C target-feature=+crt-static"

# No local path in the binaries: this container's home (/root), cargo home
# and source tree (faraday/image/path-remap.sh).
export HOME=/root
. /src/faraday/image/path-remap.sh

cargo build --release --locked --target "$RUST_TARGET" -p faraday-stick -p faraday-disk -p faraday-grant
for bin in faraday faraday-disk faraday-grant; do
    install -m 0755 "$CARGO_TARGET_DIR/$RUST_TARGET/release/$bin" "/out/$bin"
    file "/out/$bin"
    # (grep -m1 would end the pipe early, and pipefail would take that for
    # a failure.)
    hit="$(strings -a "/out/$bin" | grep -E '(^|[^[:alnum:]_.-])/(src|root)/|/cargo/registry|/home/|/Users/' | head -1 || true)"
    [ -z "$hit" ] || die "$bin carries a path: $hit"
done
