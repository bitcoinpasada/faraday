#!/usr/bin/env bash
# Design-token lint (docs/DESIGN.md §3).
#
# A new dimension, size, ratio or timing of the design system is a name in
# core/faraday-draw/src/tokens.rs, so that changing a number changes it
# everywhere and a review is settled by editing one file, and a colour on
# a screen is one of Material 3's colour roles, from faraday-ui's palette
# (docs/THEMES.md). Three checks:
#
# 1. The widgets (every file under core/faraday-draw/src/widgets/, the icon
#    and the keyboard) write no layout number of their own. Zero allowed.
#
# 2. The screens still hold numbers written in place from before this rule
#    was checked there. tools/lint-tokens.counts holds, per crate, how many
#    decimal literals the rule below flags in faraday/faraday-core/src/,
#    faraday/faraday-ui/src/ and faraday/faraday-starter/src/ (the
#    Starter's screens), one `crate count` line each. A count above
#    the file fails: name the new numbers as tokens. A count below the
#    file also fails: lower the number in the file to the new count, so the
#    ratchet only ever tightens. A change that names numbers lowers the
#    count in the same commit.
#
# 3. The screens' colours: a colour written in place (`Color::rgb(` or
#    `Color::rgba(`) in faraday/faraday-core/src/ or
#    faraday/faraday-starter/src/, outside comments and the test module,
#    is counted the same way, one `crate:colour count` line each in
#    tools/lint-tokens.counts. The few there are stand for something
#    outside the theme (a physical die's colour); a new colour is a role,
#    or a named colour in faraday-ui's palette.
#
# Not checked: tokens.rs itself, which is where the numbers live, and
# faraday-ui's palette, which is where the colours live.
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
        ' "$f" 2>&1 || echo "$f: awk failed (exit $?)"
    done
}

# Prints "file:line" for each colour written in place in the files given.
scan_colours() {
    for f in "$@"; do
        [ -f "$f" ] || continue
        awk -v file="$f" '
            /^#\[cfg\(test\)\]/ { exit }
            {
                line = $0
                sub(/\/\/.*/, "", line)
                gsub(/"[^"]*"/, "\"\"", line)
                n = gsub(/Color::rgba?\(/, "", line)
                for (i = 0; i < n; i++) printf "%s:%d\n", file, NR
            }
        ' "$f" 2>&1 || echo "$f: awk failed (exit $?)"
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

# 2. The screens: no file above its count; a file below it fails until
# the count is lowered (`--write` lowers them).
if [ ! -f "$COUNTS" ]; then
    echo "lint-tokens: missing $COUNTS"
    exit 1
fi

# The files whose numbers are counted: faraday-core's screen files only.
number_files() {
    files_of faraday/faraday-ui/src
    files_of faraday/faraday-starter/src
    files_of faraday/faraday-core/src | grep -E '(/screen\.rs|/screens/.*)$'
}
mapfile -t NUMFILES < <(number_files | sort)
mapfile -t COREFILES < <(files_of faraday/faraday-core/src)
core_colours=$(scan_colours "${COREFILES[@]}" | wc -l)

# "path count" for every counted file that has a literal.
current_counts() {
    scan "${NUMFILES[@]}" | sed 's/:[0-9]*: .*//' | sort | uniq -c | awk '{ print $2, $1 }'
    echo "faraday-core:colour $core_colours"
}

if [ "${1:-}" = "--write" ]; then
    {
        echo "# Held by tools/lint-tokens.sh: per file, how many decimal literals it still"
        echo "# writes in place, and faraday-core's colours written in place. A count"
        echo "# only falls; tools/lint-tokens.sh --write lowers it."
        current_counts
    } > "$COUNTS"
    echo "lint-tokens: wrote $COUNTS"
    exit 0
fi

seen=" "
while read -r name want; do
    case "$name" in "" | "#"*) continue ;; esac
    if [ "$name" = "faraday-core:colour" ]; then
        have=$core_colours
        what="colours written in place"
        fix="use a colour role, or name the colour in faraday-ui's palette"
    else
        seen="$seen$name "
        if [ ! -f "$name" ]; then
            echo "lint-tokens: $COUNTS names a file that is not there: $name"
            status=1
            continue
        fi
        have=$(scan "$name" | wc -l)
        what="layout numbers written in place"
        fix="name the new numbers as tokens in $TOKENS"
    fi
    if [ "$have" -gt "$want" ]; then
        echo "lint-tokens: $name has $have $what, $COUNTS allows $want; $fix"
        status=1
    elif [ "$have" -lt "$want" ]; then
        echo "lint-tokens: $name has $have $what, $COUNTS says $want; lower it with tools/lint-tokens.sh --write"
        status=1
    fi
done < "$COUNTS"
if ! grep -q '^faraday-core:colour ' "$COUNTS"; then
    echo "lint-tokens: faraday-core:colour has no line in $COUNTS"
    status=1
fi
# A counted file with a literal and no line allows none.
hits=$(scan "${NUMFILES[@]}" | while IFS= read -r l; do
    f=${l%%:*}
    case "$seen" in *" $f "*) ;; *) echo "$l" ;; esac
done)
if [ -n "$hits" ]; then
    echo "lint-tokens: layout number written in a file with no count in $COUNTS; name it in $TOKENS"
    echo "$hits"
    status=1
fi

# 3. The screens' colours. faraday-core keeps a count (above): the few
# there stand for something outside the theme (a physical die's colour).
# faraday-starter: none.
mapfile -t STARTER < <(files_of faraday/faraday-starter/src)
hits=$(scan_colours "${STARTER[@]}")
if [ -n "$hits" ]; then
    echo "lint-tokens: colour written in place in the Starter; use a colour role"
    echo "$hits"
    status=1
fi

if [ "$status" -eq 0 ]; then
    echo "lint-tokens: ok"
fi
exit "$status"
