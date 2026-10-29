#!/usr/bin/env bash
# The Starter's macOS code, checked without a Mac (HANDOFF item 73, A3a):
# clippy with warnings as errors (tools/clippy.py, which also fails on a
# warning about clippy's configuration) over faraday-starter-shell and
# its tests, for both Mac targets, aarch64-apple-darwin and x86_64-apple-darwin.
#
# Why no Mac is needed: a check type-checks and lints Rust for the target
# from the target's standard library, which rustup installs as it does any
# other; it links nothing, so no Apple linker or SDK is asked for. The one
# thing it cannot do without a Mac's tools is a C dependency's build
# script (secp256k1-sys compiles C with the target's compiler). Here that
# compiler and its archiver are stand-ins this script writes: the
# compiler writes an empty file where `-o` names one, the archiver an
# empty archive, so the build script succeeds and nothing it made is ever
# linked. Nothing here runs on macOS; batch A3c builds and runs it on a
# Mac.
#
# Run from the repository root; `just lint` does. Its build output stays
# in target/check-macos/.
set -euo pipefail
cd "$(dirname "$0")/.."

targets=(aarch64-apple-darwin x86_64-apple-darwin)

installed=$(rustup target list --installed)
missing=()
for t in "${targets[@]}"; do
    grep -qx "$t" <<<"$installed" || missing+=("$t")
done
if [ ${#missing[@]} -gt 0 ]; then
    echo "check-macos: the Mac targets are not installed; install them with:"
    echo "  rustup target add ${missing[*]}"
    exit 1
fi

dir=target/check-macos
mkdir -p "$dir"
cat >"$dir/cc" <<'EOF'
#!/bin/sh
# A stand-in C compiler for a check: an empty file where -o says.
while [ $# -gt 0 ]; do
    if [ "$1" = "-o" ] && [ $# -gt 1 ]; then
        : >"$2"
        shift
    fi
    shift
done
exit 0
EOF
cat >"$dir/ar" <<'EOF'
#!/bin/sh
# A stand-in archiver for a check: an empty archive at its second argument.
[ $# -ge 2 ] && printf '!<arch>\n' >"$2"
exit 0
EOF
chmod +x "$dir/cc" "$dir/ar"
cc=$(pwd)/$dir/cc
ar=$(pwd)/$dir/ar

# Both targets in one cargo: the build scripts and proc macros, which run on
# the host, are built once for the two.
stand_ins=()
target_args=()
for t in "${targets[@]}"; do
    var=${t//-/_}
    stand_ins+=("CC_$var=$cc" "AR_$var=$ar")
    target_args+=(--target "$t")
done
echo "check-macos: ${targets[*]}"
env "${stand_ins[@]}" CARGO_TARGET_DIR="$dir" \
    python3 tools/clippy.py --locked -p faraday-starter-shell --all-targets "${target_args[@]}"
