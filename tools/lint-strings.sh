#!/usr/bin/env bash
# User-facing-strings lint.
#
# Every word a user can read lives in one strings file, so that a second
# language is one more static and a wording change is a change to one
# file: faraday-strings' `en.rs` (CLAUDE.md). This lint fails when
# Faraday's screen code holds a literal with wording in it (whose kinds of
# allowed literal are listed below), and when a string in that file is
# untidy (the last check).
#
# Run from the repository root; `just lint` does.
set -u
cd "$(dirname "$0")/.."

FARADAY_STRINGS="faraday/faraday-strings/src/en.rs"

status=0

# Faraday's screen code: faraday-core (every module but the test kit:
# the screens, and the modules whose toasts, errors and labels they
# show), the two crates split from it that hold no wording
# (faraday-learn, faraday-intake), the Starter's faraday-starter,
# Faraday's drawing in faraday-ui, and the two
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
#     the like, to the end of the call; and attributes, whose only text
#     is a lint's `reason` (`#[expect(…, reason = "…")]`, or its
#     `reason = "…"` line when rustfmt splits one)
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
#     `ms1`, hex (`0x…`, a fingerprint) and PDF operators
#   - the names of standards, formats, units and keys, which a translation
#     keeps as they are: BIP-322, BIP-137, BIP-84 and the other presets,
#     SHA-256, HASH160, BIP-32, PNG, PDF, PK, KEK, db, codex32, MuSig2,
#     wsh, tr, sat, BTC (the kinds above). A name that is a label of its
#     own beside a sibling already in the strings file (HOTP beside TOTP)
#     goes there with it
# A value formatted as Debug prints it (`{:?}`) fails, wherever it is not
# a record line or marked `// not shown: <why>`: an enum's or an error's
# code name is never wording, and the screen maps the kind to a string.
# The vault crate's errors are such kinds, mapped by the app
# (`vault/errors.rs`), as are the FAT crate's (`faraday_files::fat_text`)
# and Secure Boot's (`secure_boot::error_text`); their sources are read
# here so that no wording comes back into them. A literal in them that
# is data, not wording (a certificate's common name, a format's fixed
# field), is marked `// not wording: <why>`, on its line or the comment
# line above it.
# The Learn pages (docs/learn, faraday-learn's pages) and the self-test's check names
# are outside this lint, as CLAUDE.md says.
FARADAY_SCREENS="$(find faraday/faraday-core/src -name '*.rs' -not -path '*/drawing/*' -not -path '*/work/*' | sort | grep -v '/testkit\.rs$\|/names\.rs$')
faraday/faraday-starter/src/*.rs
faraday/faraday-ui/src/*.rs
faraday/faraday-learn/src/*.rs
faraday/faraday-guides/src/*.rs
faraday/faraday-intake/src/*.rs
faraday/faraday-vault/src/lib.rs
faraday/faraday-vault/src/records.rs
faraday/faraday-fat/src/*.rs
faraday/faraday-sb/src/*.rs
faraday/shells/desktop/src/main.rs
faraday/shells/desktop/src/lib.rs
faraday/shells/starter/src/main.rs"

# shellcheck disable=SC2086
hits=$(awk '
FNR == 1 { intest = 0; dev = 0; mark = 0 }
/^#\[cfg\(test\)\]/ { intest = 1 }
intest { next }
/^[[:space:]]*\/\// { mark = ($0 ~ /\/\/ not wording: /); next }
/^[[:space:]]*#!?\[|^[[:space:]]*reason = "/ { next }
dev > 0 { dev += gsub(/\(/, "(") - gsub(/\)/, ")"); next }
/\.expect\(|panic!\(|unreachable!\(|assert(_eq|_ne)?!\(|todo!\(|eprintln!\(|eprint!\(|println!\(|print!\(|debug_assert/ {
    dev = gsub(/\(/, "(") - gsub(/\)/, ")")
    if (dev < 0) dev = 0
    next
}
/\.(starts_with|ends_with|strip_prefix|strip_suffix|trim_start_matches|trim_end_matches|split_once|contains|contains_key)\(|std::env::var|\.join\(|PathBuf::from\(|Path::new\(|Command::new\(|\.args?\(|include_str!|include_bytes!|json_strings?\(/ { next }
/tags: &\[|(vendors|families): &\[/ { next }
/bsms_record\(/ { next }
# A line marked `// not wording: <why>`, on it or on the comment line
# just above, holds data, not wording: a name a certificate carries, a
# fixed field of a format.
mark || /\/\/ not wording: / { mark = 0; next }
{
    line = $0
    sub(/[[:space:]]\/\/ .*$/, "", line)
    gsub(/(^|[^A-Za-z0-9_])br?#*"([^"\\]|\\.)*"#*/, " ", line)
    while (match(line, /r#*"([^"\\]|\\.)*"#*|"([^"\\]|\\.)*"/)) {
        lit = substr(line, RSTART, RLENGTH)
        line = substr(line, RSTART + RLENGTH)
        sub(/^r?#*"/, "", lit); sub(/"#*$/, "", lit)
        # A value as Debug prints it (`{:?}`, `{e:?}`) is the code name
        # of a kind, never wording: the screen maps the kind to a string
        # of the strings file. A record line, or a line marked
        # `// not shown: <why>` (a log line, a key compared), passes.
        if (lit ~ /\{[A-Za-z0-9_.]*:#?\?\}/ && lit !~ /\\[nt]/ && $0 !~ /\/\/ not shown: /) {
            printf "%s:%d: %s (Debug text)\n", FILENAME, FNR, lit
            continue
        }
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
    if (s ~ /[\/\\]/ || b ~ /^[.-]/ || s ~ /^[A-Za-z0-9_{}-]+\.([a-z0-9]{2,6}|[A-Z0-9]{2,6}|\{[a-z]*\})$/) return 1
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

for file in "$FARADAY_STRINGS"; do
    if [ ! -f "$file" ]; then
        echo "lint-strings: missing $file"
        status=1
    else
        # Every string in en.rs is tidy: not empty, no double or edge spaces,
        # and none of the filler that Faraday's strings rules forbid. The literals are what sits
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

# Every string is used: a key no Rust file outside en.rs names as `.key`
# is dead wording, and goes (STRUCTURE-PASS §7, A11).
keys=$(grep -oE '^    [a-z0-9_]+: "' "$FARADAY_STRINGS" | sed -E 's/^    ([a-z0-9_]+): "/\1/' | sort -u)
used=$(find faraday core tools -name '*.rs' -not -path '*/target/*' -not -path "$FARADAY_STRINGS" -print0 \
    | xargs -0 grep -hoE '\.[a-z0-9_]+\b' | sed 's/^\.//' | sort -u)
unused=$(comm -23 <(printf '%s\n' "$keys") <(printf '%s\n' "$used"))
if [ -n "$unused" ]; then
    echo "lint-strings: strings no code uses; remove them from $FARADAY_STRINGS"
    echo "$unused" | sed 's/^/  /'
    status=1
fi

if [ "$status" -eq 0 ]; then
    echo "lint-strings: ok"
fi
exit "$status"
