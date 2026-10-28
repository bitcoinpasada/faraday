#!/usr/bin/env bash
# Secret-hygiene lint for the core crates and the secret state in
# faraday/.
#
# Rules 1 and 2 are applied to every file under core/*/src/ and to every
# file under faraday/faraday-core/src/; rule 3 only to the secret files
# listed below:
#   1. No output or logging macros: println!, eprintln!, print!, eprint!,
#      dbg!, log::, tracing::. Tests that need to print live under tests/
#      (integration tests), or use assert! only.
#   2. No `#[derive(... Debug ...)]` on a type named Secret, Sealed,
#      SessionKey, Mnemonic, MnemonicBytes, Seed, Entropy, MasterKey,
#      DerivedKey, PinEntry, Session, Key or Item (checked on the item
#      that follows the derive). Types of those names that hold no secret
#      are excepted one by one in ALLOWED below.
# Applied to the files that handle secret material, currently
# core/faraday-crypto/src/sealed.rs (the session key and sealed values),
# core/faraday-bip/src/bip39.rs (mnemonics, entropy, seeds),
#   core/faraday-bip/src/slip39.rs (SLIP-39 shares and master secrets),
#   core/faraday-bip/src/codex32/ (mod.rs: codex32 secrets, shares and master
#     seeds; worksheet.rs: the worksheets of a set's strings; fresh.rs: a set
#     made from fresh entropy; correct.rs: a damaged string and the
#     correction proposed for it),
#   core/faraday-bip/src/silent.rs (a silent payments wallet's scan private
#     key, the label tweaks and the shared secrets it derives, and the
#     BIP-392 key expression that carries the scan key as text),
#   core/faraday-bip/src/dleq.rs (the secret and the nonce of a BIP-374
#     proof) and core/faraday-psbt/src/silent.rs (the input keys a BIP-375
#     signer multiplies its ECDH shares out of),
# core/faraday-bip/src/keys.rs,
# core/faraday-bip/src/bip85.rs (the WIF, extended private key, raw bytes and
#   passwords BIP-85 derives, which are as secret as the key they came
#   from) and
# core/faraday-bip/src/xkey.rs (private keys), core/faraday-entropy/src/ (one file a source)
# (rolls, flips, raw entropy), core/faraday-bip/src/aez.rs and aezeed.rs (an
# LND seed's words and entropy), core/faraday-bip/src/vanity.rs (the
# candidate passphrase each grind tests); in faraday/faraday-core/src/,
# word_entry.rs (the words typed on a list's own keyboard), quiz.rs (quiz
# candidates) and grind.rs (the counter a vanity grind appends to a
# passphrase, which is part of one); and
# faraday/faraday-vault/src/layouts.rs (the payloads a vault record holds),
# faraday/faraday-pgp/src/paperkey.rs (a GPG key's paperkey read back: its
#   lines, its data and the secret parts in it) and
# core/faraday-backup/src/kdbx.rs (the KDBX 4 writer, which holds the words
# and the seed in the clear until the cipher runs):
#   3. No heap text: the tokens `String`, `alloc::string`, `to_string`,
#      `to_owned` and `format!` may not appear, in code or comments. Words
#      are u16 indices; text is `&str` borrowed from the embedded lists.
#      One exception, because it is not Rust's type at all: a byte-string
#      literal holding the KDBX XML element `<String>`, which kdbx.rs
#      writes into a document.
#      Other files in faraday-bip (accounts, descriptors, SLIP-132) hold only
#      public data and may use String freely. A new module that touches
#      secrets must be added to SECRET_FILES below; paths are relative to
#      the repository root and need not be under core/.
#
# Rule 4, the key boundary, and rule 5, the dev card's reports, are
# described where they are checked, below.
#
# Exits non-zero on any hit and prints each offending line. Run from the
# repository root; `just lint` does.
set -u
cd "$(dirname "$0")/.."

status=0

report() {
    # $1 = rule description, $2 = matching lines (empty when clean)
    if [ -n "$2" ]; then
        echo "lint-secrets: $1"
        echo "$2"
        status=1
    fi
}

src_files() {
    { find core -path '*/src/*' -name '*.rs'; find faraday/faraday-core/src -name '*.rs'; } | sort
}

# Rule 1: output and logging macros.
report "output/logging macro in core source (rule 1)" \
    "$(src_files | xargs grep -nE '\b(println|eprintln|print|eprint|dbg)!|\b(log|tracing)::')"

# Rule 2: Debug derives on secret types. Pair each derive with the item that
# follows it (skipping other attributes and doc comments).
#
# These types carry the name of a secret type and hold none; each is
# public data, and the exception is that file and name alone:
#   slip132.rs Key         an extended public key, decoded
#   fonts/mod.rs Key       a glyph cache key (family, size, code point)
#   shell-api Key          a physical keyboard key
#   backup_flow Sealed     the hash and name of a sealed vault file
#   plan Seed              a seed as the plan sees it: name and flags
#   plan Item              one line of the checklist
ALLOWED="core/faraday-bip/src/slip132.rs:Key core/faraday-draw/src/fonts/mod.rs:Key core/faraday-shell-api/src/lib.rs:Key faraday/faraday-core/src/backup_flow/mod.rs:Sealed faraday/faraday-core/src/plan/mod.rs:Seed faraday/faraday-core/src/plan/mod.rs:Item"
rule2() {
    for f in $(src_files); do
        awk -v file="$f" -v allowed="$ALLOWED" '
            /#\[derive\(/ { derive = $0; derive_line = NR; next }
            derive != "" && /^[[:space:]]*(#\[|\/\/)/ { next }
            derive != "" {
                if (derive ~ /Debug/ && $0 ~ /(^|[^A-Za-z0-9_])(struct|enum)[[:space:]]+(Secret|Sealed|SessionKey|Mnemonic|MnemonicBytes|Seed|Entropy|MasterKey|DerivedKey|PinEntry|Session|Key|Item)([^A-Za-z0-9_]|$)/) {
                    line = $0
                    sub(/^.*(struct|enum)[[:space:]]+/, "", line)
                    sub(/[^A-Za-z0-9_].*$/, "", line)
                    if (index(" " allowed " ", " " file ":" line " ") == 0) {
                        printf "%s:%d: %s\n", file, derive_line, derive
                    }
                }
                derive = ""
            }
        ' "$f"
    done
}
report "Debug derived on a secret type (rule 2)" "$(rule2)"

# Rule 3: no heap text in the files that hold secret material.
SECRET_FILES="core/faraday-crypto/src/sealed.rs core/faraday-bip/src/bip39.rs core/faraday-bip/src/slip39.rs core/faraday-bip/src/keys.rs core/faraday-bip/src/bip85.rs core/faraday-bip/src/xkey.rs core/faraday-entropy/src/lib.rs core/faraday-entropy/src/dice.rs core/faraday-entropy/src/booklet.rs core/faraday-entropy/src/coins.rs core/faraday-entropy/src/hex.rs core/faraday-entropy/src/xor.rs core/faraday-entropy/src/cards.rs core/faraday-entropy/src/camera.rs core/faraday-entropy/src/device.rs core/faraday-entropy/src/mix.rs faraday/faraday-vault/src/layouts.rs core/faraday-backup/src/kdbx.rs core/faraday-bip/src/codex32/mod.rs core/faraday-bip/src/codex32/worksheet.rs core/faraday-bip/src/codex32/fresh.rs core/faraday-bip/src/codex32/correct.rs core/faraday-bip/src/silent.rs core/faraday-bip/src/dleq.rs core/faraday-psbt/src/silent.rs core/faraday-bip/src/aez.rs core/faraday-bip/src/aezeed.rs core/faraday-bip/src/vanity.rs faraday/faraday-core/src/word_entry.rs faraday/faraday-core/src/quiz.rs faraday/faraday-core/src/grind.rs faraday/faraday-pgp/src/paperkey.rs"
for f in $SECRET_FILES; do
    [ -f "$f" ] || { echo "lint-secrets: missing $f (update SECRET_FILES)"; status=1; }
done
report "heap text in secret-handling source (rule 3)" \
    "$(grep -nE '\bString\b|alloc::string|\bto_string\b|\bto_owned\b|\bformat!' $SECRET_FILES \
        | grep -vE 'b"[^"]*</?String>')"

# Rule 4: the key boundary (docs/KEYS.md, F70). Key material is used
# only in faraday/faraday-keys. Everywhere else under faraday/ these are
# refused, in code (comments are skipped):
#   4a. a key or a seed in hand: MasterKey, SecretKey, Xpriv, Keypair,
#       PrivateKey, DerivedKey, SecNonce, SlotKey, to_seed;
#   4b. seed words made into a Mnemonic (parse, from_indices,
#       from_entropy), a SeedQR decoded into one, a SLIP-39 split, a
#       vault record's words read (records::words_of): faraday_keys::words
#       does these;
#   4c. the primitive crates' secret-taking functions and types: a GPG
#       key (faraday_pgp::Key), a paperkey read back or a key restored
#       from one (faraday_pgp::paperkey, restore, Restored: the restore
#       set, faraday_keys::gpg::Restore, does these), Secure Boot keys made or used
#       (faraday_sb make, enrolment, Keys, Pair, pe::sign), a vault
#       opened, sealed, created or its slot key derived (faraday_vault
#       open, open_with, create, derive, seal, Opened), a PSBT signed
#       (faraday_psbt::sign), BIP-352's receiver, an aezeed decoded;
#   4d. a held secret taken out of its guard (Held::release, release(),
#       into_record()), except at the sites in RELEASES;
#   4e. a codex32 set made, split or recovered, or a worksheet of one's
#       strings (fresh_set, FreshSet, ChecksumWorksheet,
#       TranslationWorksheet, Codex32::from_seed, ::split, ::recover,
#       ::derive_share): faraday_keys::codex32::WorkSet and
#       Keys::codex32_shares do these, and lend their results only as a
#       paper form.
# Exempt, as they are not shipped or are the boundary's own: faraday-keys;
# the primitive crates faraday-pgp, -sb and -vault; tests/,
# examples/ and fuzz/; the snapshot shell and faraday/tools/ (test
# renders and test data); faraday-core's testkit.rs (the `testkit`
# feature, which tools/lint-keys-tests.sh keeps out of every shipped
# binary). faraday-intake is exempt from 4b only: it reads text as it
# arrives (transient input, KEYS §2) and keeps none of it, and
# faraday-keys depends on it.
#
# The one place a held secret leaves its guard outside faraday-keys:
#   vault/screen.rs   a GPG key's paperkey text, drawn while it is held
#                     (Reveal::GpgPaperkey)
RELEASES="faraday/faraday-core/src/vault/screen.rs"
boundary_files() {
    find faraday -name '*.rs' \
        -not -path 'faraday/faraday-keys/*' \
        -not -path 'faraday/faraday-pgp/*' -not -path 'faraday/faraday-sb/*' \
        -not -path 'faraday/faraday-vault/*' \
        -not -path '*/tests/*' -not -path '*/examples/*' -not -path 'faraday/fuzz/*' \
        -not -path 'faraday/shells/snapshot/*' -not -path 'faraday/tools/*' \
        -not -path 'faraday/faraday-core/src/testkit.rs' | sort
}
in_code() {
    # $1 = pattern; the files follow. Comment lines are skipped.
    local re=$1
    shift
    grep -nE "$re" "$@" | grep -vE '^[^:]+:[0-9]+:[[:space:]]*//'
}
mapfile -t BOUNDARY < <(boundary_files)
mapfile -t WORDS_FILES < <(printf '%s\n' "${BOUNDARY[@]}" | grep -v '^faraday/faraday-intake/')
mapfile -t RELEASE_FILES < <(printf '%s\n' "${BOUNDARY[@]}" | grep -vxF -f <(tr ' ' '\n' <<< "$RELEASES"))
secret_types='\b(MasterKey|SecretKey|Xpriv|Keypair|PrivateKey|DerivedKey|SecNonce|SlotKey)\b|\bto_seed\b'
# `Work::MasterKey` is a kind of costly work, not the type, and so is the
# enum variant work.rs declares for it.
report "a key or seed outside faraday-keys (rule 4a)" \
    "$(in_code "$secret_types" "${BOUNDARY[@]}" \
        | sed -E 's/\b(Work|C)::MasterKey\b//g' \
        | grep -E "$secret_types" \
        | grep -vE '^faraday/faraday-core/src/work\.rs:[0-9]+:[[:space:]]*MasterKey,$')"
report "seed words made outside faraday_keys::words (rule 4b)" \
    "$(in_code 'Mnemonic::(parse|from_indices|from_entropy|from_phrase)|seedqr::from_(entropy|digits)|slip39::split|records::words_of' "${WORDS_FILES[@]}")"
report "a primitive's secret-taking function outside faraday-keys (rule 4c)" \
    "$(in_code 'faraday_pgp::(Key|paperkey|restore|Restored|certificate::(restore|Restored))\b|faraday_sb::(make|enrolment|Keys|Pair|x509::Pair|pe::sign)\b|faraday_vault::(open|open_with|create|derive|seal|Opened)\b|faraday_psbt::sign\(|faraday_bip::silent::Receiver|aezeed::(decode|node_key)|use faraday_(pgp|sb|vault)::\{[^}]*\b(Key|paperkey|restore|Restored|make|enrolment|Keys|Pair|open|open_with|create|derive|seal|Opened)\b' "${BOUNDARY[@]}")"
report "a codex32 set made, split or recovered outside faraday-keys (rule 4e)" \
    "$(in_code 'fresh_set|FreshSet|ChecksumWorksheet|TranslationWorksheet|Codex32::(from_seed|split|recover|derive_share)\b' "${BOUNDARY[@]}")"
report "a held secret taken out of its guard outside the named sites (rule 4d)" \
    "$(in_code 'Held::release|\.release\(\)|\.into_record\(\)' "${RELEASE_FILES[@]}")"

# Rule 5: the dev card's reports. faraday-boot-report.txt and
# faraday-timings.txt are written by the files below, and hold the kernel's
# log and files, the shell's own clock and the core's counts of costly
# work, never what a person typed or chose. In code (comments are
# skipped), these files
#   5a. do not name the app, its storage requests or its events, a key or
#       a vault (Faraday, StorageCommand, StorageEvent, faraday_shell_api,
#       faraday_keys, faraday_vault): they never hold the app's state;
#   5b. reach faraday-core only through its counts of costly work
#       (faraday_core::work, whose vault notes carry a cost and an
#       outcome and nothing else) and its fade counts (FadeCounts).
REPORT_FILES="faraday/shells/stick/src/bootreport.rs faraday/shells/stick/src/timings.rs faraday/shells/stick/src/costly.rs faraday/shells/stick/src/system.rs faraday/shells/stick/src/radios.rs"
for f in $REPORT_FILES; do
    [ -f "$f" ] || { echo "lint-secrets: missing $f (update REPORT_FILES)"; status=1; }
done
# shellcheck disable=SC2086 # the list is split on purpose
report "the app's state in a dev report's writer (rule 5a)" \
    "$(in_code '\bFaraday\b|\bStorage(Command|Event)\b|\bfaraday_(shell_api|keys|vault)\b' $REPORT_FILES)"
# shellcheck disable=SC2086
report "faraday-core reached past its work counts in a dev report's writer (rule 5b)" \
    "$(in_code '\bfaraday_core::' $REPORT_FILES | grep -vE '\bfaraday_core::(work|FadeCounts)\b')"

if [ "$status" -eq 0 ]; then
    echo "lint-secrets: ok"
fi
exit "$status"
