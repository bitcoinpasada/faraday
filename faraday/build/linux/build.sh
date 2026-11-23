#!/usr/bin/env bash
# Builds one Linux binary of the workspace inside the container
# (faraday/build/linux/Containerfile) against glibc 2.31, and checks it.
# `just faraday-linux-bin` starts it; there is nothing here to run on the
# host.
#
#   build.sh [PACKAGE [BINARY [FEATURES]]]   default: faraday-demo faraday
#
# FEATURES (cargo's --features list) is only Faraday Demo's dev build's
# (F326): the work kept out of the app.
#
# The binary lands in /out/BINARY. Container paths: /src is the checkout
# (read-only), /out and /build are bind mounts, /cargo the registry cache.
# FARADAY_BUILD comes from the host (faraday/image/build-id.sh).
set -euo pipefail

package="${1:-faraday-demo}"
binary="${2:-faraday}"
features="${3:-}"

die() { printf 'build.sh: %s\n' "$1" >&2; exit 1; }

cd /src
want="$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)"
have="$(rustc --version | cut -d' ' -f2)"
[ "$want" = "$have" ] || die "rust-toolchain.toml says $want, the container has $have"
: "${FARADAY_BUILD:?FARADAY_BUILD is not set}"
export FARADAY_BUILD

# No local path in the binary: the same rewriting as every other build of
# something published (faraday/image/path-remap.sh), with this container's
# home (/root), cargo home and source tree.
export HOME=/root
. /src/faraday/image/path-remap.sh

# A release's build ID is no dev build's, and faraday-core's version.rs
# refuses the features under it.
cargo build --release --locked -p "$package" --bin "$binary" ${features:+--features "$features"}
install -m 0755 "$CARGO_TARGET_DIR/release/$binary" "/out/$binary"

# Nothing may need a newer glibc than the container's, and nothing of the
# container's own paths may be left in the binary.
need="$(objdump -T "/out/$binary" | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_//' | sort -t. -k1,1n -k2,2n -k3,3n -u | tail -1)"
echo "$binary needs GLIBC_$need"
[ "$(printf '%s\n2.31\n' "$need" | sort -V | tail -1)" = "2.31" ] || die "$binary needs GLIBC_$need, more than 2.31"
# (grep -m1 would end the pipe early, and pipefail would take that for a
# failure.)
hit="$(strings -a "/out/$binary" | grep -E '(^|[^[:alnum:]_.-])/(src|root)/|/cargo/registry|/home/|/Users/' | head -1 || true)"
[ -z "$hit" ] || die "$binary carries a path: $hit"
