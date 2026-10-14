#!/usr/bin/env bash
# User-facing-strings lint (UX.md §6; opensigner-core/src/strings/).
#
# Every word a user can read lives in a strings file, so that a second
# language is one more static and a wording change is a change to one
# file: OpenSigner's words in opensigner-core's `strings/en.rs`,
# Faraday's in faraday-strings' `en.rs` (CLAUDE.md). Both files are
# held to the same tidiness below. This lint fails when a file under
# opensigner/opensigner-core/src/views/ holds a string literal whose first
# character is a letter: a label, a sentence, a badge; and when Faraday's
# screen code holds a literal with wording in it (the second check below,
# whose kinds of allowed literal are listed there).
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
FARADAY_STRINGS="faraday/faraday-strings/src/en.rs"

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

# Faraday's screen code: faraday-core (every module but the test kit:
# the screens, and the modules whose toasts, errors and labels they
# show), the two crates split from it that hold no wording
# (faraday-learn, faraday-intake), the Starter's faraday-starter, the
# Companion's faraday-companion, Faraday's drawing in faraday-ui, and the two
# shells' main files, which title their windows. The shells' other files
# and the stick's shell are platform code: system calls, the writer's
# pipe, command lines and stderr; the one label each puts on screen is in
# the strings file too.
#
# A literal fails when, with its `{…}` holes taken out, it still has a
# letter and is none of the kinds below. Test modules (from the first
# `#[cfg(test)]` line on) and comments are not read. Allowed, because
# none of it is wording a translation would change:
#   - developer messages: `expect`, `panic!`, `assert!`, `eprintln!` and
#     the like, to the end of the call
#   - tokens read out of data or handed to the system: arguments of
#     `starts_with`, `strip_prefix`, `contains`, `split_once` and their
#     kin, of `join`, `PathBuf::from`, `Command::new`, `.arg`, environment
#     variables and `include_str!`; byte strings (`b"…"`), which are
#     bytes such as a hash's domain label
#   - identifiers: a lowercase code word (`vault`, `idle-off`,
#     `seed{i}`, `checked {} {}`) is a key of a record, a file format or
#     an enum's code. A single lowercase word passes as one, so a word
#     like that, when it is wording, has to be caught by review
#   - names with a capital inside or a digit: PSBT, QEMU, VMware, Ed25519,
#     P2WPKH, BIP-39, SLIP-39, SHA-256, RFC 6979 and the extended keys'
#     prefixes (xpub, Yprv); a catalog entry's `tags`, which are such
#     names; and the makers and models a firmware names (`vendors`,
#     `families`), which are matched, not shown
#   - paths, file names and command-line flags: anything with a slash,
#     starting with `.` or `-`, or a name with an extension
#   - a line of a file or record format (a literal with `\n` or `\t` in
#     it, or a format's header such as `faraday-sheet 1`) and BIP 129's
#     key record, which other software reads
#   - units, which are symbols: s, h, B, sat, BTC, mBTC, bits, KB to TiB
#   - notation: derivation paths (`m`, `m/…`), descriptors, codex32's
#     `ms1`, hex (`0x…`, a fingerprint), PDF operators, and a type as
#     Debug prints it
# The Learn pages (docs/learn, osk-learn) and the self-test's check names
# are outside this lint, as CLAUDE.md says, and so are the maker pages'
# firmware sketches (`sketch.rs` in faraday-learn, the shape, and
# faraday-guides, the data; docs/STARTER.md §7.8, F18): their words are a
# firmware's own, the screen a sketch depicts, not Faraday's wording.
FARADAY_SCREENS="$(ls faraday/faraday-core/src/*.rs | grep -v '/testkit\.rs$\|/names\.rs$')
faraday/faraday-starter/src/*.rs
faraday/faraday-companion/src/*.rs
faraday/faraday-ui/src/*.rs
$(ls faraday/faraday-learn/src/*.rs | grep -v '/sketch\.rs$')
$(ls faraday/faraday-guides/src/*.rs | grep -v '/sketch\.rs$')
faraday/faraday-intake/src/*.rs
faraday/shells/desktop/src/main.rs
faraday/shells/starter/src/main.rs"

# shellcheck disable=SC2086
hits=$(awk '
FNR == 1 { intest = 0; dev = 0 }
/^#\[cfg\(test\)\]/ { intest = 1 }
intest { next }
/^[[:space:]]*\/\// { next }
dev > 0 { dev += gsub(/\(/, "(") - gsub(/\)/, ")"); next }
/\.expect\(|panic!\(|unreachable!\(|assert(_eq|_ne)?!\(|todo!\(|eprintln!\(|eprint!\(|println!\(|print!\(|debug_assert/ {
    dev = gsub(/\(/, "(") - gsub(/\)/, ")")
    if (dev < 0) dev = 0
    next
}
/\.(starts_with|ends_with|strip_prefix|strip_suffix|trim_start_matches|trim_end_matches|split_once|contains|contains_key)\(|std::env::var|\.join\(|PathBuf::from\(|Path::new\(|Command::new\(|\.args?\(|include_str!|include_bytes!|json_strings?\(/ { next }
/tags: &\[|(vendors|families): &\[/ { next }
/bsms_record\(/ { next }
{
    line = $0
    sub(/[[:space:]]\/\/ .*$/, "", line)
    gsub(/(^|[^A-Za-z0-9_])br?#*"([^"\\]|\\.)*"#*/, " ", line)
    while (match(line, /r#*"([^"\\]|\\.)*"#*|"([^"\\]|\\.)*"/)) {
        lit = substr(line, RSTART, RLENGTH)
        line = substr(line, RSTART + RLENGTH)
        sub(/^r?#*"/, "", lit); sub(/"#*$/, "", lit)
        bare = lit
        gsub(/\{[^{}]*\}/, "", bare)
        if (bare !~ /[A-Za-z]/ || allowed(lit, bare)) continue
        printf "%s:%d: %s\n", FILENAME, FNR, lit
    }
    # a string that goes on to the next line
    if (line ~ /"[A-Za-z][^"]*\\$/) printf "%s:%d: %s\n", FILENAME, FNR, line
}
function allowed(s, b) {
    if (b ~ /^[a-z0-9][a-z0-9_.:+-]* *$/) return 1
    if (b ~ /^[A-Za-z0-9_:+-]+$/ && (b ~ /^[A-Z0-9_:+-]+$/ || b ~ /[A-Za-z-][A-Z]/ || b ~ /[0-9]/)) return 1
    if (s ~ /^(BIP|SLIP|RFC|SHA|HASH|KDBX|BC-UR|ur:)/ || s ~ /^[A-Za-z](prv|pub)$/) return 1
    if (s ~ /[\/\\]/ || b ~ /^[.-]/ || s ~ /^[A-Za-z0-9_{}-]+\.([a-z0-9]{2,6}|\{[a-z]*\})$/) return 1
    if (s ~ /\\[nt]/ || s ~ /^[a-z0-9-]+ [0-9]+$/) return 1
    if (b ~ /^ *([0-9.]+ )?(s|h|B|sat|sats|BTC|mBTC|bits|[KMGT]i?B)$/) return 1
    if (s ~ /^m$/ || s ~ /^(wpkh|wsh|sh|tr|pkh|musig)\(/ || s ~ /^ms1/ || s ~ /^0x/) return 1
    if (b ~ /^( +[A-Za-z0-9]{1,2}){2,} *$/ || s ~ /^[0-9A-F ]+$/ || s ~ /^[A-Z][A-Za-z]+\(.*\)$/) return 1
    return 0
}
' $FARADAY_SCREENS)
if [ -n "$hits" ]; then
    echo "lint-strings: user-facing literal in Faraday's screen code; move it to $FARADAY_STRINGS"
    echo "$hits"
    status=1
fi

for file in "$STRINGS" "$FARADAY_STRINGS"; do
    if [ ! -f "$file" ]; then
        echo "lint-strings: missing $file"
        status=1
    else
        # Every string in en.rs is tidy: not empty, no double or edge spaces,
        # and none of the filler UX.md §6 forbids. The literals are what sits
        # between the quotes of a `name: "…"` line; a `\"` inside one is
        # notation and left alone.
        untidy=$(
            grep -nE '^[[:space:]]*[a-z_0-9]+: "' "$file" \
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
            echo "lint-strings: untidy wording in $file"
            echo "$untidy" | sed "s,^,$file:,"
            status=1
        fi
    fi
done

if [ "$status" -eq 0 ]; then
    echo "lint-strings: ok"
fi
exit "$status"
