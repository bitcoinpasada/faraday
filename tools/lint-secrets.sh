#!/usr/bin/env bash
# Secret-hygiene lint for the core crates and the secret state in
# faraday/.
#
# Rule 1 is applied to every file under core/*/src/, to every
# file under faraday/faraday-core/src/, faraday/faraday-plan/src/,
# faraday/faraday-paper/src/ and faraday/faraday-host/src/, to the Bitcoin Core wallet reader's
# (faraday/faraday-core-wallet/src/, whose records carry a wallet's keys on
# their way to faraday-keys); rule 3 to the secret files listed below, which a
# file's use of a secret type also selects:
#   1. No output or logging macros: println!, eprintln!, print!, eprint!,
#      dbg!, log::, tracing::. Tests that need to print live under tests/
#      (integration tests), or use assert! only.
#   2. A secret type derives none of Clone, PartialEq or Debug and has no
#      Display impl: held by costs-lint's secret-types check
#      (tools/costs-lint/src/secret_types.rs, docs/SECRET-LEAKS.md §6), which
#      reads the secret type names from tools/secret-names.txt as this file
#      does.
# Applied to the files that handle secret material, currently
# core/faraday-crypto/src/secret.rs (secrets, seeds and mnemonics),
# core/faraday-bip/src/bip39.rs (mnemonics, entropy, seeds),
#   core/faraday-bip/src/slip39.rs (SLIP-39 shares and master secrets),
#   core/faraday-bip/src/codex32/ (mod.rs: codex32 secrets, shares and master
#     seeds; worksheet.rs: the worksheets of a set's strings; fresh.rs: a set
#     made from fresh entropy; correct.rs: a damaged string and the
#     correction proposed for it),
#   core/faraday-bip/src/silent.rs (a silent payments wallet's scan private
#     key, the label tweaks and the shared secrets it derives, and the
#     BIP-392 key expression that carries the scan key as text),
# core/faraday-bip/src/keys.rs,
# core/faraday-bip/src/bip85.rs (the WIF, extended private key, raw bytes and
#   passwords BIP-85 derives, which are as secret as the key they came
#   from) and
# core/faraday-bip/src/xkey.rs (private keys), core/faraday-bip/src/wif.rs (a
# single private key read from a WIF), core/faraday-entropy/src/ (one file a source)
# (rolls, flips, raw entropy), core/faraday-bip/src/aez.rs and aezeed.rs (an
# LND seed's words and entropy), core/faraday-bip/src/vanity.rs (the
# candidate passphrase each grind tests); in faraday/faraday-core/src/,
# word_entry.rs (the words typed on a list's own keyboard), quiz.rs (quiz
# candidates) and grind.rs (the counter a vanity grind appends to a
# passphrase, which is part of one); and
# faraday/faraday-vault/src/layouts.rs (the payloads a vault record holds),
# faraday/faraday-vault/src/contents.rs and contents/tree.rs (the TOML a vault
# holds: seed words, passphrases and notes as wiped text, and its keys),
# faraday/faraday-vault/src/qr.rs (a QR vault's item and its passphrase),
# faraday/faraday-vault/src/kdbx/keyfile.rs (a key file's key),
# faraday/faraday-pgp/src/paperkey.rs faraday/faraday-pgp/src/secret.rs (a GPG key's paperkey read back: its
#   lines, its data and the secret parts in it):
#   3. No heap text: the tokens `String`, `Arc<str>`, `Box<str>`, `alloc::string`, `to_string`,
#      `to_owned`, `format!` and `.into_owned()` may not appear, in code or
#      comments, and neither may the calls that build a String from other
#      text: `.join(` with an argument (not `.join()`), `<[_]>::join(`,
#      `.concat()`, `.to_lowercase(`, `.to_uppercase(`, `str::to_lowercase`,
#      `str::to_uppercase`, `.to_ascii_lowercase(`, `.to_ascii_uppercase(`,
#      `.replace(` and `.replacen(` with a pattern and a replacement, and
#      `.repeat(`; a line whose call makes no text says `// not text:` and
#      what it makes, and a receiver named c, ch, b, i, j, x, d or index
#      or written `char::from(..)` is a char or a byte (see BUILDERS). Words
#      are u16 indices; text is `&str` borrowed from the embedded lists.
#      One exception, because it is not Rust's type at all: a byte-string
#      literal holding the KDBX XML element `<String>`, which the vault's
#      xml/mod.rs writes into a document. Another: in
#      core/faraday-bip/src/json.rs, the name of the reader's variant
#      `Value::String` (its declaration `String(Text),` and its uses as
#      `Value::String(`), a name of the public API whose payload is wiping
#      Text; a `String` type there is still a hit.
#      Other files in faraday-bip (accounts, descriptors, SLIP-132) hold only
#      public data and may use String freely. A file is a secret file when
#      it names a secret type. SECRET_FILES is worked out below, not listed:
#      the non-test source files under core/ and faraday/ (type_files) that
#      name a secret type (the names in tools/secret-names.txt, the key boundary's key types, `zeroize`),
#      less UNHELD, plus NAMELESS, so a new module is found without anyone
#      adding it. The wiping text of faraday_text counts as a secret type: a
#      file naming it is held or, with a reason that is true, on UNHELD
#      (below), the one place such a file may stay out of rule 3. A file on
#      UNHELD names no other secret type. NAMELESS (with its reasons) holds
#      the files that hold a secret and name no secret type; one that now
#      names one is reported. These two are the only hand lists. The
#      files type_files leaves out (the shells, tools, fuzz and the Starter)
#      are not read by this check.
#
# Rule 4, the key boundary (clippy's, below), rule 5, the dev card's reports, and rule 6, the
# test allocator, are described where they are checked, below.
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
    { find core -path '*/src/*' -name '*.rs'; find faraday/faraday-core/src -name '*.rs'; find faraday/faraday-plan/src -name '*.rs'; find faraday/faraday-paper/src -name '*.rs'; find faraday/faraday-host/src -name '*.rs'; find faraday/faraday-core-wallet/src -name '*.rs'; } | sort
}

# Every non-test source file that can name a secret type: core/ and faraday/,
# but not tests/, examples/, benches/ and fuzz/, the Starter (which runs
# on an online computer and holds test data), the shells and tools.
type_files() {
    find core faraday -path '*/src/*' -name '*.rs' \
        -not -path '*/tests/*' -not -path '*/examples/*' -not -path '*/benches/*' \
        -not -path 'faraday/faraday-starter*' -not -path 'faraday/shells/*' \
        -not -path 'faraday/tools/*' -not -path 'faraday/fuzz/*' | sort
}
# A secret type, by name: the wiping types, faraday-keys' holders (Held,
# HeldSet, HeldItem, HeldAezeed), the key boundary's key types and anything that
# wipes itself. The type names are tools/secret-names.txt, which costs-lint's
# secret-types check reads too: one list.
SECRET_TYPES=$(grep -vE '^[[:space:]]*(#|$)' tools/secret-names.txt | sed -E 's/\*$/[A-Za-z]*/' | paste -sd'|')
SECRET_NAMES="\\b(${SECRET_TYPES}|Zeroize|ZeroizeOnDrop)\\b|\\bzeroize\\b|\\bfaraday_text\\b"
# The same without the wiping text: a file on UNHELD may name only that.
OTHER_SECRET_NAMES="\\b(${SECRET_TYPES}|Zeroize|ZeroizeOnDrop)\\b|\\bzeroize\\b"

# Rule 1: output and logging macros.
report "output/logging macro in core source (rule 1)" \
    "$(src_files | xargs grep -nE '\b(println|eprintln|print|eprint|dbg)!|\b(log|tracing)::')"

# Rule 2, a secret type derives no Clone, PartialEq or Debug and has no
# Display impl, is held by costs-lint (tools/costs-lint/src/secret_types.rs),
# which reads the secret type names from tools/secret-names.txt as this file
# does.

# Rule 3: no heap text in the files that hold secret material. A file holds
# secret material when it names a secret type (SECRET_NAMES, the wiping text
# included): SECRET_FILES is worked out below from the files type_files lists,
# less UNHELD, plus NAMELESS. The two hand lists are NAMELESS and UNHELD.
# Files that hold a secret without naming a secret type or the wiping text,
# each with why:
#   codex32/fresh.rs   makes shares from entropy through the types of mod.rs
#   vault/qr.rs        takes the passphrase as text for the PGP layer's seal
#   keys/parts.rs      handles a key file's bytes and builds the Components that hold its key
#   faraday-text lib   declares and documents the wiping text type of text.rs
NAMELESS="core/faraday-bip/src/codex32/fresh.rs faraday/faraday-vault/src/qr.rs faraday/faraday-keys/src/parts.rs core/faraday-text/src/lib.rs"
# Files that name the wiping text or hold test text and are not held (the
# unheld files), each with why. This list is the one place such a file may
# stay out of SECRET_FILES; a file listed here that is missing, or that names a
# secret type other than the wiping text, is reported:
#   testkit/plain.rs  the test kit's key texts as plain text: public fixtures and
#                     published dummy data, built only with the `testkit` feature
#                     for tests, tools and the online Starter, never in the device app;
#                     and Plain, which copies a frame's drawn text out for a test
#   faraday/faraday-core/src/screens/backup.rs
#       builds no heap text of a secret; draws the revealed words, the typed passphrase and the word check by borrow from held text (the reveal's Text), and its other text is titles, counts and file names
#   faraday/faraday-core/src/compact/screen.rs
#       builds no heap text of a secret; draws the typed word, its number and the passphrase field by borrow from held text (the entry's Text and the held keyed_head)
#   faraday/faraday-core/src/board/mod.rs
#       the board's place names, chip labels, fingerprints and checksums, public text
#   faraday/faraday-core/src/board/screen.rs
#       the board's chip names and marks as drawn, public text
#   faraday/faraday-core/src/board/codex32.rs
#       a codex32 set's string names by index and its counts, never a string's characters
#   faraday/faraday-core/src/public_files.rs
#       a wallet's public files, their names and the descriptor's text
#   faraday/faraday-core/src/file_menu.rs
#       file names, kinds, sizes, SHA-256 groups and renames, public text
#   faraday/faraday-core/src/backup_flow/screen.rs
#       the backup flow cards' titles, place and vault names and counts
#   faraday/faraday-core/src/backup_flow/area.rs
#       the gate's kinds, ids and names, and the Power sheet's lines
#   faraday/faraday-core/src/backup_flow/wallet_screen.rs
#       a wallet backup's missing names and counts
#   faraday/faraday-core/src/memory/mod.rs
#       the kept settings, amounts, hashes, file names and summaries, which hold no secret
#   faraday/faraday-core/src/vault/load.rs
#       the Load sheet's wallet names, fingerprints, labels and the "Passphrase for key {fp}" lines, never a passphrase
#   faraday/faraday-core/src/vault/load_job.rs
#       a wallet's description and name, public
#   faraday/faraday-core/src/vault/summaries.rs
#       each vault's summary as kept, ids, fingerprints and names
#   faraday/faraday-core/src/vault/seals.rs
#       seal counts and times
#   faraday/faraday-core/src/vault/plan_store.rs
#       a plan record's places, vault names and labels
#   faraday/faraday-core/src/vault/keep.rs
#       the vault names a save goes into
#   faraday/faraday-core/src/stick/wipe.rs
#       the files chosen to delete, percentages and minutes
#   faraday/faraday-core/src/stick/mod.rs
#       stick problems and the stick's settings text
#   faraday/faraday-core/src/stick/settings.rs
#       the stick settings file's lines
#   faraday/faraday-core/src/secure_boot/mod.rs
#       the owner GUID, certificate names, image hashes and refusals; the keys stay in the record
#   faraday/faraday-core/src/register.rs
#       a wallet's typed name, shape, paths and refusals
#   faraday/faraday-core/src/create_from.rs
#       key lines, vault names and wallet checksums
#   faraday/faraday-core/src/decode/check_screen.rs
#       Check a signer's card titles, file names and txids
#   faraday/faraday-core/src/multisig_check/mod.rs
#       a setup's public descriptor text, its cosigners' fingerprints, paths and keys, and refusals; a private key is refused before it is read
#   faraday/faraday-core/src/multisig_check/screen.rs
#       a setup's public rows and the seed check's answer lines
#   faraday/faraday-core/src/wallet/confirm.rs
#       a wallet's public keys, fingerprints and paths on the confirmation sheet and Check a multisig setup
#   faraday/faraday-core/src/wallet/nonce.rs
#       the nonce check's counts and headlines
#   faraday/faraday-core/src/wallet/account.rs
#       an account's name, BIP number and source
#   faraday/faraday-core/src/wallet/own_name.rs
#       a wallet's name, a key's label and fingerprint
#   faraday/faraday-core/src/screens/wallets.rs
#       vault names, fingerprints and counts
#   faraday/faraday-core/src/screens/check_message.rs
#       a signed message checked and its refusal
#   faraday/faraday-core/src/visible.rs
#       a message to sign, drawn with its hidden characters as marks
#   faraday/faraday-core/src/drawing/memo.rs
#       the memo's backup lines and button labels
#   faraday/faraday-core/src/samples/put.rs
#       Practice's published sample keys and their public key text (the SeedQR digits at :107 are a sample's); a typed cosigner's origin is read by borrow
#   faraday/faraday-core/src/vanity/mod.rs
#       script names, a found address's head, fingerprints and refusals
#   faraday/faraday-core/src/explore/addresses.rs
#       addresses and the address searched for
#   faraday/faraday-core/src/explore/make.rs
#       the refusal line a Make a wallet press leaves
#   faraday/faraday-core/src/explore/mod.rs
#       a loaded key's name, the account path and the made wallet's name and refusals, public text
#   faraday/faraday-core/src/address_find/accounts.rs
#       a loaded key's name, fingerprint and standard account paths, public text
#   faraday/faraday-core/src/address_find/card.rs
#       how far a search looked, in how many wallets, and a found wallet's name, chain and index
#   faraday/faraday-core/src/address_find/look.rs
#       a found wallet's name, as it was at the find
#   faraday/faraday-core/src/address_check/mod.rs
#       the address typed or pasted, held in the wiping text and emptied when a secret is pasted, and its refusal lines
#   faraday/faraday-core/src/address_check/file.rs
#       a file's addresses and their answers, public text, read once the file is not a secret
#   faraday/faraday-core/src/address_check/screen.rs
#       the count over a file's addresses, drawn
#   faraday/faraday-core/src/codes/screen.rs
#       the seconds left on a code, never the code
#   faraday/faraday-core/src/work/kept.rs
#       test code (cfg(test)), wallet files and the test kit's dummy words
#   faraday/faraday-core/src/work/budget/scaling.rs
#       test code, counts and file names
#   faraday/faraday-core/src/work/budget/fee_bump.rs
#       test code, test file names
#   faraday/faraday-core/src/work/budget/signer.rs
#       test code, test file names
#   faraday/faraday-ui/src/lib.rs
#       SVG path data and test lines
#   faraday/faraday-ui/src/nav/app_bar.rs
#       a page title split to fit
#   faraday/faraday-ui/src/sheet.rs
#       a sheet's short labels
#   faraday/faraday-host/src/labels.rs
#       the strip's control labels and menu rows, never typed text
#   faraday/faraday-sb/src/lib.rs
#       the enrolment files' names and certificate common names
#   core/faraday-psbt/src/psbt.rs
#       a PSBT's parse refusals and its base64
#   core/faraday-bip/src/coldcard.rs, faraday/faraday-intake/src/electrum.rs
#       the import's own messages and public fields; only a key line is wiping text
UNHELD="faraday/faraday-core/src/testkit/plain.rs
faraday/faraday-core/src/screens/backup.rs
faraday/faraday-core/src/compact/screen.rs
faraday/faraday-core/src/board/mod.rs
faraday/faraday-core/src/board/screen.rs
faraday/faraday-core/src/board/codex32.rs
faraday/faraday-core/src/public_files.rs
faraday/faraday-core/src/file_menu.rs
faraday/faraday-core/src/backup_flow/screen.rs
faraday/faraday-core/src/backup_flow/area.rs
faraday/faraday-core/src/backup_flow/wallet_screen.rs
faraday/faraday-core/src/memory/mod.rs
faraday/faraday-core/src/vault/load.rs
faraday/faraday-core/src/vault/load_job.rs
faraday/faraday-core/src/vault/summaries.rs
faraday/faraday-core/src/vault/seals.rs
faraday/faraday-core/src/vault/plan_store.rs
faraday/faraday-core/src/vault/keep.rs
faraday/faraday-core/src/stick/wipe.rs
faraday/faraday-core/src/stick/mod.rs
faraday/faraday-core/src/stick/settings.rs
faraday/faraday-core/src/secure_boot/mod.rs
faraday/faraday-core/src/register.rs
faraday/faraday-core/src/create_from.rs
faraday/faraday-core/src/decode/check_screen.rs
faraday/faraday-core/src/multisig_check/mod.rs
faraday/faraday-core/src/multisig_check/screen.rs
faraday/faraday-core/src/wallet/confirm.rs
faraday/faraday-core/src/wallet/nonce.rs
faraday/faraday-core/src/wallet/account.rs
faraday/faraday-core/src/wallet/own_name.rs
faraday/faraday-core/src/screens/wallets.rs
faraday/faraday-core/src/screens/check_message.rs
faraday/faraday-core/src/visible.rs
faraday/faraday-core/src/drawing/memo.rs
faraday/faraday-core/src/samples/put.rs
faraday/faraday-core/src/vanity/mod.rs
faraday/faraday-core/src/explore/addresses.rs
faraday/faraday-core/src/explore/make.rs
faraday/faraday-core/src/explore/mod.rs
faraday/faraday-core/src/address_find/accounts.rs
faraday/faraday-core/src/address_find/card.rs
faraday/faraday-core/src/address_find/look.rs
faraday/faraday-core/src/address_check/mod.rs
faraday/faraday-core/src/address_check/file.rs
faraday/faraday-core/src/address_check/screen.rs
faraday/faraday-core/src/codes/screen.rs
faraday/faraday-core/src/work/kept.rs
faraday/faraday-core/src/work/budget/scaling.rs
faraday/faraday-core/src/work/budget/fee_bump.rs
faraday/faraday-core/src/work/budget/signer.rs
faraday/faraday-ui/src/lib.rs
faraday/faraday-ui/src/nav/app_bar.rs
faraday/faraday-ui/src/sheet.rs
faraday/faraday-host/src/labels.rs
faraday/faraday-sb/src/lib.rs
core/faraday-psbt/src/psbt.rs
core/faraday-bip/src/coldcard.rs
faraday/faraday-intake/src/electrum.rs"
UNHELD=$(echo $UNHELD)
names_secret() { grep -qE "$SECRET_NAMES" "$1"; }
SECRET_FILES=$({ for f in $(type_files); do
    names_secret "$f" || continue
    case " $UNHELD " in *" $f "*) ;; *) echo "$f" ;; esac
done; echo $NAMELESS | tr ' ' '\n'; } | sort -u)
# One path per line above; the checks below match a path between spaces.
SECRET_FILES=$(echo $SECRET_FILES)
# A character is never encoded into a bare stack array in a held file: the
# array is not wiped, so a seed word's or a passphrase's last character would
# stay on the stack. Text::from_char and the wiping text's own push do it.
for f in $SECRET_FILES; do
    [ -f "$f" ] || continue
    if grep -nE 'encode_utf8\(&mut \[' "$f" | grep -v '^[0-9]*:[[:space:]]*//' | grep -q .; then
        echo "lint-secrets: $f encodes a character into an unwiped stack array (use Text::from_char)"
        status=1
    fi
done
unheld=$(for f in $UNHELD; do
    [ -f "$f" ] || { echo "$f: missing"; continue; }
    grep -qE "$OTHER_SECRET_NAMES" "$f" && echo "$f: names a secret type other than the wiping text; it cannot be on UNHELD"
    grep -qE '\bfaraday_text\b' "$f" || echo "$f: no longer names the wiping text; remove it from UNHELD"
done)
report "UNHELD is out of date (rule 3)" "$unheld"
missing=$(for f in $NAMELESS; do
    [ -f "$f" ] || echo "$f: missing"
done)
report "NAMELESS is out of date (rule 3)" "$missing"
redundant=$(for f in $NAMELESS; do
    [ -f "$f" ] && names_secret "$f" && echo "$f"
done)
report "a file in NAMELESS that now names a secret type (rule 3; remove it from NAMELESS)" "$redundant"
# The text builders: the type, its macros and the calls that make a String.
HEAP_TEXT='\bString\b|\b(Arc|Box)<str>|alloc::string|\bto_string\b|\bto_owned\b|\bformat!|\.into_owned\(\)'
# The calls that build a String (or a Vec) from other text. A line that holds
# one of them but not text (a char or a byte, a Vec of bytes, an Option, a
# path) says so with a `// not text: <what>` comment on the line; a file that
# builds only paths with `.join(` is in PATH_FILES, with its reason.
BUILDERS='\.join\([^)]|<\[_\]>::join\(|\.concat\(\)|\bstr::to_(lower|upper)case\b|\.to_(lower|upper)case\(|\.to_ascii_(lower|upper)case\(|\.replacen?\([^,()]*(\([^()]*\))?[^,()]*,|\.repeat\('
# `.to_ascii_lowercase()` on a char or a byte makes no text: a receiver named
# c, ch, b, i, j, x, d or index, or `char::from(..)`, is one.
CHARS='(^|[^A-Za-z0-9_.])(c|ch|b|i|j|x|d|index)\.to_ascii_(lower|upper)case\(|char::from\([^()]*\)\.to_ascii_(lower|upper)case\('
# Files whose `.join(` is a path's, each with why:
#   faraday-disk lib.rs, faraday-files lib.rs, faraday-storage lib.rs   build file system paths
PATH_FILES="faraday/faraday-disk/src/lib.rs faraday/faraday-files/src/lib.rs faraday/faraday-storage/src/lib.rs"
builders=$(grep -nE "$BUILDERS" $SECRET_FILES | grep -v '// not text:' | grep -vE "$CHARS" \
    | while IFS=: read -r f n rest; do
        case " $PATH_FILES " in *" $f "*) case "$rest" in *'.join('*) ! grep -qE '\.(concat|repeat|replacen?|to_[a-z_]*case)\(' <<< "$rest" && continue ;; esac ;; esac
        echo "$f:$n:$rest"
    done)
report "heap text in secret-handling source (rule 3)" \
    "$( { grep -nE "$HEAP_TEXT" $SECRET_FILES \
        | grep -vE 'b"[^"]*</?String>|^[^:]+:[0-9]+: +[a-z0-9_]+: "String",$' \
        | grep -vE '^core/faraday-bip/src/json\.rs:[0-9]+: +String\(Text\),$' \
        | while IFS= read -r hit; do
            # json.rs names its own variant `Value::String(`: that token
            # alone is no heap text (serde_json's, written with a path
            # before it, is); a line still a hit without it stays one.
            case "$hit" in
                core/faraday-bip/src/json.rs:*)
                    grep -qE "$HEAP_TEXT" <<< "$(sed -E 's/(^|[^:])Value::String\(/\1/g' <<< "${hit#*:*:}")" || continue ;;
            esac
            echo "$hit"
        done; echo "$builders"; } | grep .)"

# Rule 4, the key boundary (docs/KEYS.md), is clippy's: the `disallowed-types`
# and `disallowed-methods` entries marked "key boundary" in faraday/clippy.toml
# (and faraday-kef's copy of them) refuse key material, seed words, the
# primitive crates' secret-taking functions, codex32 sets and a held secret
# taken out of its guard in every crate under faraday/ but the ones that make
# them (faraday-keys, -vault, -pgp and -sb, whose own clippy.toml has none).
# A legitimate use carries `#[expect(clippy::disallowed_types, reason = ...)]`
# or `#[expect(clippy::disallowed_methods, reason = ...)]`; tools/clippy.py
# fails when the copies of the entries differ.
in_code() {
    # $1 = pattern; the files follow. Comment lines are skipped.
    local re=$1
    shift
    grep -nE "$re" "$@" | grep -vE '^[^:]+:[0-9]+:[[:space:]]*//'
}

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

# Rule 6: the test allocator stays out of every shipped crate
# (docs/SECRET-LEAKS.md §8 and §10, F181, F250). No Cargo.toml under the
# repository (except target/ and out/) names faraday-leak-scan outside a
# dev-dependencies table, `[target.'...'.dev-dependencies]` and
# `[dev-dependencies.faraday-leak-scan]` included. The crate's own [package]
# and the workspace's [workspace] members list are not a dependency.
leak_scan_outside_dev() {
    awk '
        /^[[:space:]]*\[/ {
            table = $0
            sub(/^[[:space:]]*/, "", table)
            sub(/[[:space:]]*(#.*)?$/, "", table)
            next
        }
        /faraday-leak-scan/ {
            if (table ~ /^\[workspace\]$/ || table ~ /^\[package\]$/) next
            if (table ~ /dev-dependencies(\.faraday-leak-scan)?\]$/) next
            if ($0 ~ /^[[:space:]]*#/) next
            printf "%s: faraday-leak-scan in %s\n", FILENAME, (table == "" ? "the top of the file" : table)
        }
    ' "$@"
}
leak_scan_headers() {
    grep -nE '^[[:space:]]*\[.*faraday-leak-scan' "$@" \
        | grep -vE 'dev-dependencies\.faraday-leak-scan\]' \
        | sed -E 's/^([^:]*):[0-9]+:[[:space:]]*(.*)$/\1: faraday-leak-scan in \2/'
}
# shellcheck disable=SC2046 # the manifest list is split on purpose
report "the test allocator faraday-leak-scan outside a dev-dependencies table (rule 6)" \
    "$({ leak_scan_outside_dev $(find . \( -path ./target -o -path ./out -o -path ./.git -o -path ./.claude \) -prune -o -name Cargo.toml -print | sort); leak_scan_headers $(find . \( -path ./target -o -path ./out -o -path ./.git -o -path ./.claude \) -prune -o -name Cargo.toml -print | sort); } | sort -u)"

if [ "$status" -eq 0 ]; then
    echo "lint-secrets: ok"
fi
exit "$status"
