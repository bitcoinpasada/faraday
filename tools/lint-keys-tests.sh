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
# desktop app (`faraday-demo`) and Faraday Starter, in whose window
# Faraday Demo runs (docs/decisions/F126.md), so the Starter reaches it
# through `faraday-demo` alone. The samples are strings, bytes and
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
# A fifth: faraday-vault's `fuzzing` feature, which adds the entry the
# kdbx_body fuzz target feeds, is on in no shipped binary.
#
# A sixth: the token that counts nothing stays out of what Faraday ships
# (docs/COMPUTE.md §3). `faraday_crypto::work::Compute::unchecked` exists only
# under faraday-crypto's `unchecked` feature, which core's tests and the fuzz
# targets turn on as dev-dependencies; it is on in no shipped binary.
#
# A seventh: work kept out of the app by a build switch (F325). The
# handwriting reader (`faraday-handwriting`) is in the dependency graph of no
# shipped binary, and no feature named `handwriting`, of faraday-core or of
# faraday-demo, is on in one. Only Faraday Demo's dev build (F326) turns it on,
# and that build is never shipped. The binary is searched for the crate too,
# by the recipes that build the shipped ones (faraday/tools/binary-crate.py,
# F325 point 3).
#
# Every `cargo tree` here runs with --locked and fails the lint with its own
# error when it fails; a tree that prints nothing fails too, since each one
# asked for holds at least the package named. A renamed package therefore
# cannot pass.
#
# Run from the repository root; `just lint` does.
set -euo pipefail
cd "$(dirname "$0")/.."

shipped=(-p faraday-demo -p faraday-stick -p faraday-disk -p faraday-grant
    -p faraday-boot -p faraday-starter-shell)

# tree WHAT CARGO-TREE-ARGS...: runs cargo tree into $out. A failure of
# cargo (an unknown package, a lockfile out of date) ends the lint with
# cargo's own error; so does an empty tree. With ABSENT_OK=1, for a package
# that must not be in the binaries' trees (faraday-samples), a tree
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
# faraday-starter-shell to faraday-samples runs through faraday-demo.
tree "faraday-starter-shell" -e normal --depth 1 -p faraday-starter-shell
if grep -qE '^[├└]── faraday-samples v' <<< "$out"; then
    echo "lint-keys-tests: the Starter depends on faraday-samples other than through faraday-demo"
    echo "$out"
    exit 1
fi

# faraday-crypto's `unchecked` feature, which gives Compute::unchecked, is
# on in no shipped binary.
tree "faraday-crypto" -e features,normal "${shipped[@]}" -i faraday-crypto
if grep -q 'faraday-crypto feature "unchecked"' <<< "$out"; then
    echo "lint-keys-tests: a shipped binary enables faraday-crypto's unchecked feature"
    echo "$out"
    exit 1
fi
# Work kept out of the app by a build switch (F325): the crate is in no
# shipped binary's graph, with the default features a shipped build has.
ABSENT_OK=1 tree "faraday-handwriting" -e normal "${shipped[@]}" -i faraday-handwriting
if grep -q 'faraday-handwriting v' <<< "$out"; then
    echo "lint-keys-tests: a shipped binary depends on faraday-handwriting"
    echo "$out"
    exit 1
fi
for krate in faraday-core faraday-demo; do
    tree "$krate" -e features,normal "${shipped[@]}" -i "$krate"
    if grep -q "$krate feature \"handwriting\"" <<< "$out"; then
        echo "lint-keys-tests: a shipped binary enables $krate's handwriting feature"
        echo "$out"
        exit 1
    fi
done
echo "lint-keys-tests: ok"
