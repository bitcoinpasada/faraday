#!/usr/bin/env bash
# The crypto has not changed since the last passing cross-check (F79,
# docs/CROSSCHECK.md §10.2): the hash of the crypto sources, the
# cross-check and the crypto crates' Cargo.lock entries must match
# `tools/crosscheck/stamp`, which `just crosscheck` writes when every
# answer agrees. Fails with "crypto changed since the last cross-check:
# run `just crosscheck`".
#
# Run from the repository root; `just lint` does.
set -euo pipefail
cd "$(dirname "$0")/.."
exec python3 -I -B tools/crosscheck/stamp.py check
