#!/usr/bin/env bash
# User-facing-strings lint (UX.md §6; opensigner-core/src/strings/).
#
# Every word a user can read lives in `strings/en.rs`, so that a second
# language is one more static and a wording change is a change to one
# file. This lint fails when a file under
# opensigner/opensigner-core/src/views/ holds a string literal whose first
# character is a letter: a label, a sentence, a badge.
#
# Not flagged, because they carry no wording:
#   - comments and doc comments
#   - panic and assertion messages (`expect`, `panic!`, `unreachable!`),
#     which a user never sees
#   - format templates and separators with no words in them: "{}",
#     "{} · {}", "#{}"
#   - the notation in ALLOW below
#
# Run from the repository root; `just lint` does.
set -u
cd "$(dirname "$0")/.."

VIEWS="opensigner/opensigner-core/src/views"
STRINGS="opensigner/opensigner-core/src/strings/en.rs"

# Lines whose literal is notation rather than wording, as one extended
# regular expression. `m/` is a derivation path; `BIP-` names a standard.
ALLOW_LINE='"(m/|BIP-)'

status=0

hits=$(
    for f in "$VIEWS"/*.rs; do
        # Drop comment lines and lines whose only literal is a developer
        # message, then look for a double quote followed by a letter.
        grep -nE '"[A-Za-z]' "$f" \
        | grep -vE ':[[:space:]]*//' \
        | grep -vE '\.expect\(|panic!\(|unreachable!\(|assert(_eq)?!\(|todo!\(' \
        | grep -vE "$ALLOW_LINE" \
        | sed "s,^,$f:,"
    done
)
if [ -n "$hits" ]; then
    echo "lint-strings: user-facing literal in a view; move it to $STRINGS"
    echo "$hits"
    status=1
fi

if [ ! -f "$STRINGS" ]; then
    echo "lint-strings: missing $STRINGS"
    status=1
else
    # Every string in en.rs is tidy: not empty, no double or edge spaces,
    # and none of the filler UX.md §6 forbids. The literals are what sits
    # between the quotes of a `name: "…"` line; a `\"` inside one is
    # notation and left alone.
    untidy=$(
        grep -nE '^[[:space:]]*[a-z_0-9]+: "' "$STRINGS" \
        | sed -E 's/^([0-9]+):[[:space:]]*([a-z_0-9]+): "(.*)",?[[:space:]]*$/\1\t\2\t\3/' \
        | awk -F'\t' '
            $3 == ""            { print $1 ": " $2 " is empty"; next }
            $3 ~ /  /           { print $1 ": " $2 " has a double space"; next }
            $3 ~ /^ / || $3 ~ / $/ { print $1 ": " $2 " starts or ends with a space"; next }
            $3 ~ / just /       { print $1 ": " $2 " says \"just\""; next }
            $3 ~ /don.t worry/  { print $1 ": " $2 " reassures"; next }
        '
    )
    if [ -n "$untidy" ]; then
        echo "lint-strings: untidy wording in $STRINGS"
        echo "$untidy" | sed "s,^,$STRINGS:,"
        status=1
    fi
fi

if [ "$status" -eq 0 ]; then
    echo "lint-strings: ok"
fi
exit "$status"
