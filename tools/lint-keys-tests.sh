#!/usr/bin/env bash
# A key's master key stays inside faraday-keys (docs/KEYS.md). The one
# accessor outside it, `Keys::master_for_tests`, exists only under
# faraday-keys' `master-for-tests` feature, which faraday-core's tests turn
# on as a dev-dependency; and the test keys, wallets and master-key helpers
# of `faraday_core::testkit` exist only under faraday-core's `testkit`
# feature, which lint-secrets' key-boundary rule exempts. This lint fails
# when either feature is on in the normal dependency graph of a binary
# Faraday ships, on any target: the desktop app, the stick's and the Pi's
# app, the disk and grant helpers, the boot report and the Starter.
#
# A third check: `faraday-samples` (the public samples Demo takes,
# docs/MODES.md §5.4) is public data, but only the desktop app carries it.
# The stick, the Pi, the disk and grant helpers, the boot report and the
# Starter's own app must not reach it by a normal dependency, on any target.
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

core=$(cargo tree -q --color never --target all -e features,normal \
    -p faraday-desktop -p faraday-stick -p faraday-disk -p faraday-grant \
    -p faraday-boot -p faraday-starter-shell -i faraday-core)

if grep -q 'faraday-core feature "testkit"' <<< "$core"; then
    echo "lint-keys-tests: a shipped binary enables faraday-core's testkit feature"
    echo "$core"
    exit 1
fi

# The tree of dependents of faraday-samples, over the five binaries; any of
# them in it is a path from that binary to the crate. A binary that does not
# reach the crate prints nothing of itself.
samples=$(cargo tree -q --color never --target all -e normal \
    -p faraday-stick -p faraday-disk -p faraday-grant \
    -p faraday-boot -p faraday-starter-shell -i faraday-samples 2>&1 || true)

if grep -qE '(faraday-stick|faraday-disk|faraday-grant|faraday-boot|faraday-starter-shell) v' <<< "$samples"; then
    echo "lint-keys-tests: a binary other than the desktop app depends on faraday-samples"
    echo "$samples"
    exit 1
fi
echo "lint-keys-tests: ok"
