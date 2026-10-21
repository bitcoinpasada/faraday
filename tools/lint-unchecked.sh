#!/usr/bin/env bash
# The token that counts nothing stays out of what Faraday ships
# (docs/COMPUTE.md §3). `osk_crypto::work::Compute::unchecked` exists only
# under osk-crypto's `unchecked` feature, which core's tests and the fuzz
# targets turn on as dev-dependencies. This lint fails when the feature is
# on in the normal dependency graph of a binary Faraday ships, on any
# target: the desktop app, the stick's and the Pi's app, the disk and
# grant helpers, the boot report and the Starter.
#
# Run from the repository root; `just lint` does.
set -euo pipefail
cd "$(dirname "$0")/.."

tree=$(cargo tree -q --color never --target all -e features,normal \
    -p faraday-desktop -p faraday-stick -p faraday-disk -p faraday-grant \
    -p faraday-boot -p faraday-starter-shell -i osk-crypto)

if grep -q 'osk-crypto feature "unchecked"' <<< "$tree"; then
    echo "lint-unchecked: a shipped binary enables osk-crypto's unchecked feature"
    echo "$tree"
    exit 1
fi
echo "lint-unchecked: ok"
