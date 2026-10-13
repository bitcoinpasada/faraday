#!/usr/bin/env bash
# The Companion's crate graph (docs/COMPANION.md §3).
#
# The phone app's core holds none of the code that loads, keeps or signs
# with a key: it depends on no crate in FORBIDDEN below. This lint checks
# every crate in CRATES, the ones the Companion's core is built from:
# `cargo tree` over its normal dependencies, on every target, must list
# none of the forbidden crates. A hit names the crate, the forbidden
# crate, and the path from one to the other, and fails.
#
# Run from the repository root; `just lint` does.
set -u
cd "$(dirname "$0")/.."

CRATES="
faraday-strings
faraday-learn
faraday-intake
faraday-qr
faraday-scanner
osk-ui
osk-learn
osk-codec
osk-bip
"

FORBIDDEN="
faraday-core
opensigner-core
faraday-vault
faraday-pgp
faraday-sb
osk-keep
osk-backup
osk-psbt
osk-entropy
"

status=0
for crate in $CRATES; do
    if ! names=$(cargo tree -q -e normal --target all -p "$crate" --prefix none --format '{p}' 2>&1); then
        echo "lint-companion-deps: cargo tree failed for $crate"
        echo "$names"
        status=1
        continue
    fi
    names=$(echo "$names" | awk '{print $1}' | sort -u)
    for bad in $FORBIDDEN; do
        if echo "$names" | grep -qx "$bad"; then
            echo "lint-companion-deps: $crate depends on $bad, which the Companion may not hold; the path:"
            cargo tree -q -e normal --target all -p "$crate" -i "$bad"
            status=1
        fi
    done
done

if [ "$status" -eq 0 ]; then
    echo "lint-companion-deps: ok"
fi
exit "$status"
