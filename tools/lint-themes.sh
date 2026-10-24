#!/usr/bin/env bash
# The colour tables are the reference's (docs/THEMES.md §11.3): the
# vendored material-color-utilities matches its digests
# (tools/themes/reference.sha256), with no file added beside them, and
# the generator run on it now writes faraday/faraday-ui/src/pal/schemes.rs
# byte for byte. A hand edit to the table, or a seed changed in
# tools/themes/gen.mjs without `just themes`, fails. Offline; needs
# Node 22 or later.
#
# Run from the repository root; `just lint` does.
set -euo pipefail
cd "$(dirname "$0")/.."

ref=tools/reference/material-color-utilities
digests=tools/themes/reference.sha256
table=faraday/faraday-ui/src/pal/schemes.rs

if ! command -v node >/dev/null; then
    echo "lint-themes: node is not installed; the colour tables need Node 22 or later to check" >&2
    exit 1
fi
major="$(node -p 'process.versions.node.split(".")[0]')"
if [ "$major" -lt 22 ]; then
    echo "lint-themes: node $(node --version) is older than 22" >&2
    exit 1
fi

sha256sum --check --quiet --strict "$digests"
listed="$(sed 's/^[0-9a-f]*  //' "$digests" | LC_ALL=C sort)"
present="$(find "$ref" -type f ! -path "$ref/README.md" | LC_ALL=C sort)"
if [ "$listed" != "$present" ]; then
    echo "lint-themes: $ref holds other files than $digests lists:" >&2
    diff <(echo "$listed") <(echo "$present") >&2 || true
    exit 1
fi

fresh="$(mktemp)"
trap 'rm -f "$fresh"' EXIT
node tools/themes/gen.mjs "$fresh"
if ! cmp -s "$fresh" "$table"; then
    echo "lint-themes: $table differs from what tools/themes/gen.mjs makes; run \`just themes\`" >&2
    diff "$table" "$fresh" | head -20 >&2 || true
    exit 1
fi
