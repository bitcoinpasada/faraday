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
# A fourth: no shipped binary turns on faraday-sb's `sign-any` feature,
# which signs a boot program with no checked release behind it (F134).
#
# A fifth: the Argon QR Vault (`aqrv`, AQRV specification, https://github.com/bitcoinpasada/aqrv, F245) is Faraday's own
# construction, not reviewed outside it, and a test tool: no shipped binary
# depends on any of its crates, and none turns on faraday-strings' `aqrv`
# feature, which holds the wording of its app.
#
# A sixth: faraday-vault's `fuzzing` feature, which adds the entry the
# kdbx_body fuzz target feeds, is on in no shipped binary.
#
# Every `cargo tree` here runs with --locked and fails the lint with its own
# error when it fails; a tree that prints nothing fails too, since each one
# asked for holds at least the package named. A renamed package therefore
# cannot pass.
#
# Run from the repository root; `just lint` does.
set -euo pipefail
cd "$(dirname "$0")/.."

shipped=(-p faraday-desktop -p faraday-stick -p faraday-disk -p faraday-grant
    -p faraday-boot -p faraday-starter-shell)

# tree WHAT CARGO-TREE-ARGS...: runs cargo tree into $out. A failure of
# cargo (an unknown package, a lockfile out of date) ends the lint with
# cargo's own error; so does an empty tree. With ABSENT_OK=1, for a package
# that must not be in the binaries' trees (faraday-samples, aqrv), a tree
# that is empty, or that cargo refuses with "did not match any packages"
# (the package is not in their graph, which is the pass), is accepted; the
# package must still exist in the workspace's lockfile (cargo pkgid), so a
# renamed one fails.
out=""
ABSENT_OK=0
tree() {
    local what=$1 pkg=""
    shift
    if [ "$ABSENT_OK" = 1 ]; then
        pkg=${*: -1}
        if ! cargo pkgid --locked "$pkg" > /dev/null 2>&1; then
            echo "lint-keys-tests: $pkg is no package of the workspace; a renamed or removed package?"
            cargo pkgid --locked "$pkg" 2>&1 || true
            exit 1
        fi
    fi
    if ! out=$(cargo tree --locked -q --color never --target all "$@" 2>&1); then
        if [ "$ABSENT_OK" = 1 ] && grep -q 'did not match any packages' <<< "$out"; then
            out=""
            return
        fi
        echo "lint-keys-tests: cargo tree failed for $what:"
        echo "$out"
        exit 1
    fi
    if [ -z "$out" ] && [ "$ABSENT_OK" != 1 ]; then
        echo "lint-keys-tests: cargo tree printed nothing for $what; a renamed or removed package?"
        exit 1
    fi
}

tree "faraday-keys" -e features,normal "${shipped[@]}" -i faraday-keys
if grep -q 'faraday-keys feature "master-for-tests"' <<< "$out"; then
    echo "lint-keys-tests: a shipped binary enables faraday-keys' master-for-tests feature"
    echo "$out"
    exit 1
fi

tree "faraday-core" -e features,normal "${shipped[@]}" -i faraday-core
if grep -q 'faraday-core feature "testkit"' <<< "$out"; then
    echo "lint-keys-tests: a shipped binary enables faraday-core's testkit feature"
    echo "$out"
    exit 1
fi

# The tree of dependents of faraday-samples, over the four binaries that
# must not reach it and the Starter's own crate; any of them in it is a path
# to the crate. One that does not reach the crate prints nothing of itself,
# but a name that is no package fails.
ABSENT_OK=1 tree "faraday-samples" -e normal \
    -p faraday-stick -p faraday-disk -p faraday-grant \
    -p faraday-boot -p faraday-starter -i faraday-samples
if grep -qE '(faraday-stick|faraday-disk|faraday-grant|faraday-boot|faraday-starter) v' <<< "$out"; then
    echo "lint-keys-tests: a binary other than Faraday Demo's (the desktop app, the Starter) depends on faraday-samples"
    echo "$out"
    exit 1
fi

# Faraday signs a boot program only through a checked release (F134):
# faraday-sb's `sign-any` feature, which signs any image, is on in no
# shipped binary.
tree "faraday-sb" -e features,normal "${shipped[@]}" -i faraday-sb
if grep -q 'faraday-sb feature "sign-any"' <<< "$out"; then
    echo "lint-keys-tests: a shipped binary enables faraday-sb's sign-any feature"
    echo "$out"
    exit 1
fi

# faraday-vault's `fuzzing` feature is turned on only by the fuzz crate.
tree "faraday-vault" -e features,normal "${shipped[@]}" -i faraday-vault
if grep -q 'faraday-vault feature "fuzzing"' <<< "$out"; then
    echo "lint-keys-tests: a shipped binary enables faraday-vault's fuzzing feature"
    echo "$out"
    exit 1
fi

# The Starter reaches it only through Faraday Demo's crate: every path from
# faraday-starter-shell to faraday-samples runs through faraday-desktop.
tree "faraday-starter-shell" -e normal --depth 1 -p faraday-starter-shell
if grep -qE '^[├└]── faraday-samples v' <<< "$out"; then
    echo "lint-keys-tests: the Starter depends on faraday-samples other than through faraday-desktop"
    echo "$out"
    exit 1
fi
# The Argon QR Vault: the tree of dependents of aqrv over every shipped
# binary names none of them, and faraday-strings' `aqrv` feature is on in
# none. A binary that does not reach the crate prints nothing of itself,
# but a name that is no package fails.
ABSENT_OK=1 tree "aqrv" -e normal "${shipped[@]}" -i aqrv
if grep -qE '(faraday-desktop|faraday-stick|faraday-disk|faraday-grant|faraday-boot|faraday-starter-shell) v' <<< "$out"; then
    echo "lint-keys-tests: a shipped binary depends on the Argon QR Vault (aqrv)"
    echo "$out"
    exit 1
fi
tree "faraday-strings" -e features,normal "${shipped[@]}" -i faraday-strings
if grep -q 'faraday-strings feature "aqrv"' <<< "$out"; then
    echo "lint-keys-tests: a shipped binary enables faraday-strings' aqrv feature"
    echo "$out"
    exit 1
fi
echo "lint-keys-tests: ok"
