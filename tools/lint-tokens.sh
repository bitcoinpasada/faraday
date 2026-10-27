#!/usr/bin/env bash
# Design-token lint (docs/DESIGN.md §3).
#
# A new dimension, size, ratio or timing of the design system is a name in
# core/faraday-draw/src/tokens.rs, so that changing a number changes it
# everywhere and a review is settled by editing one file. Two checks:
#
# 1. The widgets (every file under core/faraday-draw/src/widgets/, the icon
#    and the keyboard) write no layout number of their own. Zero allowed.
#
# 2. The screens still hold numbers written in place from before this rule
#    was checked there. tools/lint-tokens.counts holds, per crate, how many
#    decimal literals the rule below flags in faraday/faraday-core/src/ and
#    faraday/faraday-ui/src/, one `crate count` line each. A count above
#    the file fails: name the new numbers as tokens. A count below the
#    file also fails: lower the number in the file to the new count, so the
#    ratchet only ever tightens. A change that names numbers lowers the
#    count in the same commit.
#
# Not checked: tokens.rs itself, which is where the numbers live.
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

TOKENS="core/faraday-draw/src/tokens.rs"
COUNTS="tools/lint-tokens.counts"

status=0

if [ ! -f "$TOKENS" ]; then
    echo "lint-tokens: missing $TOKENS"
    exit 1
fi

# Prints "file:line: literal" for each flagged literal in the files given.
scan() {
    for f in "$@"; do
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
}

files_of() {
    find "$1" -name '*.rs' | sort
}

# 1. The widgets: none.
mapfile -t WIDGETS < <(files_of core/faraday-draw/src/widgets)
hits=$(scan "${WIDGETS[@]}")
if [ -n "$hits" ]; then
    echo "lint-tokens: layout number written in place; name it in $TOKENS"
    echo "$hits"
    status=1
fi

# 2. The screens: no more than the committed counts, and no fewer.
if [ ! -f "$COUNTS" ]; then
    echo "lint-tokens: missing $COUNTS"
    exit 1
fi
# Every crate measured below must have a line: an empty or short counts
# file fails closed.
MEASURED="faraday-core faraday-ui"
seen=" "
while read -r crate want; do
    [ -n "$crate" ] || continue
    seen="$seen$crate "
    case "$crate" in
        faraday-core) dir=faraday/faraday-core/src ;;
        faraday-ui) dir=faraday/faraday-ui/src ;;
        *)
            echo "lint-tokens: $COUNTS names an unknown crate: $crate"
            status=1
            continue
            ;;
    esac
    mapfile -t SRC < <(files_of "$dir")
    have=$(scan "${SRC[@]}" | wc -l)
    if [ "$have" -gt "$want" ]; then
        echo "lint-tokens: $crate has $have layout numbers written in place, $COUNTS allows $want; name the new numbers as tokens in $TOKENS"
        status=1
    elif [ "$have" -lt "$want" ]; then
        echo "lint-tokens: $crate has $have layout numbers written in place, $COUNTS says $want; lower the number in $COUNTS to $have"
        status=1
    fi
done < "$COUNTS"
for crate in $MEASURED; do
    case "$seen" in
        *" $crate "*) ;;
        *)
            echo "lint-tokens: $crate has no line in $COUNTS"
            status=1
            ;;
    esac
done

if [ "$status" -eq 0 ]; then
    echo "lint-tokens: ok"
fi
exit "$status"
