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
# A third check: `faraday-samples` (the public samples Practice takes,
# docs/MODES.md §5.4) is public data, carried only by Faraday Demo: the
# desktop app (`faraday-desktop`) and Faraday Starter, in whose window
# Faraday Demo runs (docs/decisions/F126.md), so the Starter reaches it
# through `faraday-desktop` alone. The samples are strings, bytes and
# fingerprints that anyone may read, and Faraday Demo starts in Practice
# on testnet, where they are its only keys (F81); the Starter already
# carried faraday-core and faraday-keys for Transfer and Learn, so no key
# path is new to it. The stick, the Pi, the disk and grant helpers and the
# boot report hold real keys or run beside them and must not reach it by a
# normal dependency, on any target; nor may the Starter's own crate
# (`faraday-starter`, its pages and decisions), which takes the Demo's
# app from the shell.
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

# The tree of dependents of faraday-samples, over the four binaries that
# must not reach it and the Starter's own crate; any of them in it is a path
# to the crate. One that does not reach the crate prints nothing of itself.
samples=$(cargo tree -q --color never --target all -e normal \
    -p faraday-stick -p faraday-disk -p faraday-grant \
    -p faraday-boot -p faraday-starter -i faraday-samples 2>&1 || true)

if grep -qE '(faraday-stick|faraday-disk|faraday-grant|faraday-boot|faraday-starter) v' <<< "$samples"; then
    echo "lint-keys-tests: a binary other than Faraday Demo's (the desktop app, the Starter) depends on faraday-samples"
    echo "$samples"
    exit 1
fi

# The Starter reaches it only through Faraday Demo's crate: every path from
# faraday-starter-shell to faraday-samples runs through faraday-desktop.
direct=$(cargo tree -q --color never --target all -e normal --depth 1 \
    -p faraday-starter-shell 2>&1 || true)
if grep -qE '^[├└]── faraday-samples v' <<< "$direct"; then
    echo "lint-keys-tests: the Starter depends on faraday-samples other than through faraday-desktop"
    echo "$direct"
    exit 1
fi
echo "lint-keys-tests: ok"
