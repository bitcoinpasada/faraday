#!/usr/bin/env bash
# A key's master key stays inside faraday-keys (docs/KEYS.md). The one
# accessor outside it, `Keys::master_for_tests`, exists only under
# faraday-keys' `master-for-tests` feature, which faraday-core's tests turn
# on as a dev-dependency. This lint fails when the feature is on in the
# normal dependency graph of a binary Faraday ships, on any target: the
# desktop app, the stick's and the Pi's app, the disk and grant helpers,
# the boot report and the Starter.
#
# Run from the repository root; `just lint` does.
set -euo pipefail
cd "$(dirname "$0")/.."

tree=$(cargo tree -q --color never --target all -e features,normal \
    -p faraday-desktop -p faraday-stick -p faraday-disk -p faraday-grant \
    -p faraday-boot -p faraday-starter-shell -i faraday-keys)

if grep -q 'faraday-keys feature "master-for-tests"' <<< "$tree"; then
    echo "lint-keys-tests: a shipped binary enables faraday-keys' master-for-tests feature"
    echo "$tree"
    exit 1
fi
echo "lint-keys-tests: ok"
