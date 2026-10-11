#!/usr/bin/env bash
# Secret-hygiene lint for the OpenSignerKit core crates and the OpenSigner
# application core (docs/PLANNING.md §5.3).
#
# Rules, applied to every file under core/*/src/ and
# opensigner/opensigner-core/src/:
#   1. No output or logging macros: println!, eprintln!, print!, eprint!,
#      dbg!, log::, tracing::. Tests that need to print live under tests/
#      (integration tests), or use assert! only.
#   2. No `#[derive(... Debug ...)]` on a type named Secret, Sealed,
#      SessionKey, Mnemonic, MnemonicBytes, Seed, Entropy, MasterKey,
#      DerivedKey, PinEntry or Session (checked on the item that follows
#      the derive).
# Applied to the files that handle secret material, currently
# core/osk-crypto/src/sealed.rs (the session key and sealed values),
# core/osk-bip/src/bip39.rs (mnemonics, entropy, seeds),
#   core/osk-bip/src/slip39.rs (SLIP-39 shares and master secrets),
#   core/osk-bip/src/codex32.rs (codex32 secrets, shares and master seeds),
#   core/osk-bip/src/silent.rs (a silent payments wallet's scan private
#     key, the label tweaks and the shared secrets it derives, and the
#     BIP-392 key expression that carries the scan key as text),
# core/osk-bip/src/keys.rs,
# core/osk-bip/src/bip85.rs (the WIF, extended private key, raw bytes and
#   passwords BIP-85 derives, which are as secret as the key they came
#   from) and
# core/osk-bip/src/xkey.rs (private keys), core/osk-bip/src/frost.rs (FROST
# secret shares and secret nonces), core/osk-psbt/src/threshold.rs (the
# carry file's stored secret nonces), core/osk-entropy/src/lib.rs
# (rolls, flips, raw entropy) and, in opensigner/opensigner-core/src/,
# load.rs (the Load wizard's words, loaded keys), create.rs (the Create
# wizard's entries and words), finish.rs (the passphrase and the PIN),
# session.rs (the session key and PIN), quiz.rs (quiz candidates),
# notes.rs (a note, which is what a person wrote and may be a secret),
# keep.rs (the kept-key exchange and its PIN pads),
# lightning.rs (the aezeed words typed into the node-key tool, its
#   passphrase, the entropy it opens and the node private key),
# vanity.rs (the counter a vanity grind appends to a passphrase, which
#   is part of one; core/osk-bip/src/vanity.rs holds the candidate
#   passphrase each grind tests),
# backup.rs (the Backup flow), codex32.rs (the codex32 string being
# typed, the strings it has gathered, and the plan, randomness and
# strings this device writes), shares.rs (the SLIP-39 split's plan,
# randomness and shares), pass_entry.rs (the backup passphrase) and
# explore.rs (the words and passphrase typed into Explore); and
# core/osk-backup/src/oskb.rs (the format it seals words under),
# core/osk-backup/src/kdbx.rs (the KDBX 4 writer, which holds the words
# and the seed in the clear until the cipher runs),
# core/osk-keep/src/lib.rs (the kept-key blob: words, seeds and the
# PIN's derived keys) and core/osk-keep/src/notes.rs (the notes and
# recovery sheets kept in the blob, which are the same text as a note):
#   3. No heap text: the tokens `String`, `alloc::string`, `to_string`,
#      `to_owned` and `format!` may not appear, in code or comments. Words
#      are u16 indices; text is `&str` borrowed from the embedded lists.
#      One exception, because it is not Rust's type at all: a byte-string
#      literal holding the KDBX XML element `<String>`, which kdbx.rs
#      writes into a document.
#      Other files in osk-bip (accounts, descriptors, SLIP-132) hold only
#      public data and may use String freely. A new module that touches
#      secrets must be added to SECRET_FILES below; paths are relative to
#      the repository root and need not be under core/.
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
    find core opensigner/opensigner-core -path '*/src/*' -name '*.rs' | sort
}

# Rule 1: output and logging macros.
report "output/logging macro in core source (rule 1)" \
    "$(src_files | xargs grep -nE '\b(println|eprintln|print|eprint|dbg)!|\b(log|tracing)::')"

# Rule 2: Debug derives on secret types. Pair each derive with the item that
# follows it (skipping other attributes and doc comments).
rule2() {
    for f in $(src_files); do
        awk -v file="$f" '
            /#\[derive\(/ { derive = $0; derive_line = NR; next }
            derive != "" && /^[[:space:]]*(#\[|\/\/)/ { next }
            derive != "" {
                if (derive ~ /Debug/ && $0 ~ /(^|[^A-Za-z0-9_])(struct|enum)[[:space:]]+(Secret|Sealed|SessionKey|Mnemonic|MnemonicBytes|Seed|Entropy|MasterKey|DerivedKey|PinEntry|Session)([^A-Za-z0-9_]|$)/) {
                    printf "%s:%d: %s\n", file, derive_line, derive
                }
                derive = ""
            }
        ' "$f"
    done
}
report "Debug derived on a secret type (rule 2)" "$(rule2)"

# Rule 3: no heap text in the files that hold secret material.
SECRET_FILES="core/osk-crypto/src/sealed.rs core/osk-bip/src/bip39.rs core/osk-bip/src/slip39.rs core/osk-bip/src/frost.rs core/osk-psbt/src/threshold.rs core/osk-bip/src/keys.rs core/osk-bip/src/bip85.rs core/osk-bip/src/xkey.rs core/osk-entropy/src/lib.rs opensigner/opensigner-core/src/load.rs opensigner/opensigner-core/src/create.rs opensigner/opensigner-core/src/finish.rs opensigner/opensigner-core/src/session.rs opensigner/opensigner-core/src/quiz.rs opensigner/opensigner-core/src/backup.rs opensigner/opensigner-core/src/pass_entry.rs core/osk-backup/src/oskb.rs core/osk-backup/src/kdbx.rs opensigner/opensigner-core/src/explore.rs opensigner/opensigner-core/src/keep.rs core/osk-keep/src/lib.rs opensigner/opensigner-core/src/threshold.rs opensigner/opensigner-core/src/shares.rs opensigner/opensigner-core/src/codex32.rs core/osk-bip/src/codex32.rs core/osk-bip/src/silent.rs opensigner/opensigner-core/src/notes.rs core/osk-keep/src/notes.rs core/osk-bip/src/aez.rs core/osk-bip/src/aezeed.rs opensigner/opensigner-core/src/lightning.rs core/osk-bip/src/vanity.rs opensigner/opensigner-core/src/vanity.rs"
for f in $SECRET_FILES; do
    [ -f "$f" ] || { echo "lint-secrets: missing $f (update SECRET_FILES)"; status=1; }
done
report "heap text in secret-handling source (rule 3)" \
    "$(grep -nE '\bString\b|alloc::string|\bto_string\b|\bto_owned\b|\bformat!' $SECRET_FILES \
        | grep -vE 'b"[^"]*</?String>')"

if [ "$status" -eq 0 ]; then
    echo "lint-secrets: ok"
fi
exit "$status"
