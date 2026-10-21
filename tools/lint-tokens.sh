#!/usr/bin/env bash
# Design-token lint (docs/DESIGN.md §3).
#
# Every dimension, size, ratio and timing of the design system is a name
# in core/faraday-draw/src/tokens.rs, so that changing a number changes it
# everywhere and a review is settled by editing one file. This lint fails
# when a widget writes a layout number of its own instead.
#
# Checked: every file under core/faraday-draw/src/widgets/ (the icon and the
# keyboard). Not checked: tokens.rs itself, which is where the numbers
# live.
#
# A decimal literal is allowed when it is:
#   - 0.0, 0.5, 1.0 or 2.0 (an origin, a half, a whole, a halving)
#   - on a line that sets a flex weight (`.weight(`)
#   - inside a comment, a doc comment or a string literal
#   - in the file's `#[cfg(test)]` module, where a test names the
#     reference sizes it solves at
#   - between the markers `lint-tokens: sample data` and
#     `lint-tokens: end sample data`
#
# Run from the repository root; `just lint` does.
set -u
cd "$(dirname "$0")/.."

FILES=()
while IFS= read -r f; do
    FILES+=("$f")
done < <(find core/faraday-draw/src/widgets -name '*.rs' | sort)

TOKENS="core/faraday-draw/src/tokens.rs"

status=0

if [ ! -f "$TOKENS" ]; then
    echo "lint-tokens: missing $TOKENS"
    exit 1
fi

hits=$(
    for f in "${FILES[@]}"; do
        [ -f "$f" ] || continue
        awk -v file="$f" '
            # The test module names the reference sizes it solves at.
            /^#\[cfg\(test\)\]/ { exit }
            /lint-tokens: end sample data/ { sample = 0; next }
            /lint-tokens: sample data/ { sample = 1; next }
            sample { next }
            {
                line = $0
                sub(/\/\/.*/, "", line)
                gsub(/"[^"]*"/, "\"\"", line)
                if (line ~ /\.weight\(/) next
                while (match(line, /[0-9]+\.[0-9]+/)) {
                    v = substr(line, RSTART, RLENGTH)
                    if (v != "0.0" && v != "0.5" && v != "1.0" && v != "2.0")
                        printf "%s:%d: %s\n", file, NR, v
                    line = substr(line, RSTART + RLENGTH)
                }
            }
        ' "$f"
    done
)

if [ -n "$hits" ]; then
    echo "lint-tokens: layout number written in place; name it in $TOKENS"
    echo "$hits"
    status=1
fi

if [ "$status" -eq 0 ]; then
    echo "lint-tokens: ok"
fi
exit "$status"
