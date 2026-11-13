#!/usr/bin/env bash
# Secret-hygiene lint for the core crates and the secret state in
# faraday/.
#
# Rules 1 and 2 are applied to every file under core/*/src/, to every
# file under faraday/faraday-core/src/, faraday/faraday-plan/src/,
# faraday/faraday-paper/src/ and faraday/faraday-host/src/, to the Bitcoin Core wallet reader's
# (faraday/faraday-core-wallet/src/, whose records carry a wallet's keys on
# their way to faraday-keys) and to the Argon QR Vault library's
# (aqrv/aqrv/src/, whose items and keys are as secret as Faraday's; its
# text is zeroized strings, so rule 3 does not apply to it); rule 2's
# derives also to every other non-test source file under core/, faraday/ and
# aqrv/ except the Starter's, the shells' and tools' (docs/SECRET-LEAKS.md
# §6); rule 3 to the secret files listed below, which a file's use of a secret
# type also selects:
#   1. No output or logging macros: println!, eprintln!, print!, eprint!,
#      dbg!, log::, tracing::. Tests that need to print live under tests/
#      (integration tests), or use assert! only.
#   2. A secret type derives none of Clone, PartialEq or Debug and has no
#      Display impl. A secret type is a struct or enum whose body names a
#      secret type (Secret, SecretText, Zeroizing, SeedBytes, MnemonicBytes,
#      a faraday-keys Held*, rule 4a's key types) as a field's type, that derives
#      Zeroize, or that has an `impl Drop`, `Zeroize` or `ZeroizeOnDrop`
#      for it. A
#      struct or enum of the same crate whose body names a secret type found
#      this way, and that has no Drop of its own, is secret too, repeated
#      until nothing new is found (a name the crate defines once). A copy is a function of the type (`duplicate`), a comparison
#      one that does not branch on the bytes, a Debug one that prints a
#      redaction. Types named Secret, Mnemonic, MnemonicBytes, Seed,
#      Entropy, MasterKey, DerivedKey, PinEntry, Session, Key or Item
#      derive no Debug either (checked on the item that follows the
#      derive); those that hold no secret are excepted one by one in
#      ALLOWED below, and a secret type that keeps a derive is excepted by
#      file and name in ALLOWED_TYPES, each with its reason.
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
#   core/faraday-bip/src/dleq.rs (the secret and the nonce of a BIP-374
#     proof) and core/faraday-psbt/src/silent.rs (the input keys a BIP-375
#     signer multiplies its ECDH shares out of),
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
# faraday/faraday-vault/src/kdbx/keyfile.rs (a key file's key and a YubiKey's secret),
# faraday/faraday-pgp/src/paperkey.rs faraday/faraday-pgp/src/secret.rs (a GPG key's paperkey read back: its
#   lines, its data and the secret parts in it):
#   3. No heap text: the tokens `String`, `alloc::string`, `to_string`,
#      `to_owned` and `format!` may not appear, in code or comments. Words
#      are u16 indices; text is `&str` borrowed from the embedded lists.
#      One exception, because it is not Rust's type at all: a byte-string
#      literal holding the KDBX XML element `<String>`, which the vault's
#      xml/mod.rs writes into a document.
#      Other files in faraday-bip (accounts, descriptors, SLIP-132) hold only
#      public data and may use String freely. A file is a secret file when
#      it is in SECRET_FILES below (paths are relative to the repository
#      root and need not be under core/) or in tools/lint-secrets-excepted.txt,
#      which names each non-test source file under core/, faraday/ or aqrv/
#      that uses a secret type and is not yet held by rule 3, with its
#      reason. A file that uses a secret type (the names of rule 2, 4a's
#      key types, `zeroize`) and is in neither list is reported, so a new
#      module is found without anyone adding it; so is a listed file that
#      uses none (unless it is in NAMELESS, with its reason), an excepted
#      file that has no heap text (it moves to SECRET_FILES), and one
#      listed twice. An excepted file leaves that list only into
#      SECRET_FILES or because it is gone, since a file that holds wiping
#      text (Text, not a secret name) is read by rule 3 only when listed.
#
# Rule 4, the key boundary, rule 5, the dev card's reports, and rule 6, the
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
    { find core -path '*/src/*' -name '*.rs'; find faraday/faraday-core/src -name '*.rs'; find faraday/faraday-plan/src -name '*.rs'; find faraday/faraday-paper/src -name '*.rs'; find faraday/faraday-host/src -name '*.rs'; find faraday/faraday-core-wallet/src -name '*.rs'; find aqrv/aqrv/src -name '*.rs'; } | sort
}

# Every non-test source file that can name a secret type: core/, faraday/ and
# aqrv/, but not tests/, examples/, benches/ and fuzz/, the Starter (which runs
# on an online computer and holds test data), the shells and tools.
type_files() {
    find core faraday aqrv -path '*/src/*' -name '*.rs' \
        -not -path '*/tests/*' -not -path '*/examples/*' -not -path '*/benches/*' \
        -not -path 'faraday/faraday-starter*' -not -path 'faraday/shells/*' \
        -not -path 'faraday/tools/*' -not -path 'faraday/fuzz/*' | sort
}
# A secret type, by name: the wiping types, faraday-keys' holders (Held,
# HeldSet, HeldItem, HeldAezeed), rule 4a's key types and anything that
# wipes itself.
SECRET_NAMES='\b(Secret|SecretText|Zeroizing|SeedBytes|MnemonicBytes|Held[A-Za-z]*|Zeroize|ZeroizeOnDrop|MasterKey|SecretKey|Xpriv|Keypair|PrivateKey|DerivedKey|SecNonce|SlotKey)\b|\bzeroize\b'

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
ALLOWED="core/faraday-bip/src/slip132.rs:Key core/faraday-draw/src/fonts/mod.rs:Key core/faraday-shell-api/src/lib.rs:Key faraday/faraday-core/src/backup_flow/mod.rs:Sealed faraday/faraday-plan/src/lib.rs:Seed faraday/faraday-plan/src/lib.rs:Item"
rule2() {
    for f in $(src_files); do
        awk -v file="$f" -v allowed="$ALLOWED" '
            /#\[derive\(/ { derive = $0; derive_line = NR; next }
            derive != "" && /^[[:space:]]*(#\[|\/\/)/ { next }
            derive != "" {
                if (derive ~ /Debug/ && $0 ~ /(^|[^A-Za-z0-9_])(struct|enum)[[:space:]]+(Secret|Mnemonic|MnemonicBytes|Seed|Entropy|MasterKey|DerivedKey|PinEntry|Session|Key|Item)([^A-Za-z0-9_]|$)/) {
                    line = $0
                    sub(/^.*(struct|enum)[[:space:]]+/, "", line)
                    sub(/[^A-Za-z0-9_].*$/, "", line)
                    if (index(" " allowed " ", " " file ":" line " ") == 0) {
                        printf "%s:%d: %s\n", file, derive_line, derive
                    }
                }
                derive = ""
            }
        ' "$f" 2>&1 || echo "$f: awk failed (exit $?)"
    done
}
report "Debug derived on a secret type (rule 2)" "$(rule2)"

# Rule 2, by type: a struct or enum that holds a secret (its body names a
# secret type as a field's type, it derives Zeroize, or it has an `impl Drop`,
# `Zeroize` or `ZeroizeOnDrop`) derives none of Clone, PartialEq or Debug and
# has no Display impl. The ones that keep a derive are named here by file and
# type, then the derives it excepts (Clone, PartialEq, Debug, Display, comma
# separated; only those, any other derive on it is still flagged), then the
# reason ("fixed in a later change": the type's callers copy, compare or print
# it and are not yet changed to a function of the type that does).
ALLOWED_TYPES=$(cat <<'EOF'
faraday/faraday-core/src/codex32/mod.rs:DieRow Clone,PartialEq,Debug one die's two values, a Copy element of wiped buffers that the tour and the tests compare and print
core/faraday-shell-api/src/lib.rs:EntropyBytes Clone Event is Clone and the snapshot and replay shells copy events
core/faraday-shell-api/src/lib.rs:Event Clone,PartialEq its only wiping part is EntropyBytes, which wipes on drop and compares without a branch on a byte, so the derived copy and comparison are its own; the snapshot and replay shells copy events and the tests compare them
faraday/faraday-core/src/backup_flow/mod.rs:Live Debug holds Writing, whose Debug is its own and prints the names and sizes of what it holds; its copy is its own
faraday/faraday-core/src/backup_flow/mod.rs:BackupFlow Clone,Debug holds Live, whose copy is its own and whose Debug is Writing's own
faraday/faraday-core/src/dice_entry/screen.rs:Ask Clone,Copy for it is Copy: it borrows the dice entry and holds no copy of it
faraday/faraday-core/src/transfer/mod.rs:TransferArea Debug holds SendText, whose Debug is its own and prints lengths only
faraday/faraday-keys/src/vault/mod.rs:Records Clone,Copy for it is Copy: a view of an open vault's records that holds references, not bytes
faraday/faraday-pgp/src/message.rs:Message Debug holds Body, whose Debug is its own and prints a length only
faraday/faraday-vault/src/records.rs:Record Debug holds Field, whose Debug is its own and prints a number and a length only
faraday/faraday-vault/src/contents.rs:Refusal Debug,PartialEq holds KeyName, whose Debug and PartialEq are its own, written by hand over the name as text, not derived over its bytes
faraday/faraday-vault/src/qr.rs:Error Debug,PartialEq holds Refusal, whose Debug and PartialEq come from KeyName's own
EOF
)
ALLOWED_TYPES_LIST=$(echo "$ALLOWED_TYPES" | awk '{print $1 "=" $2}' | tr '\n' ' ')
rule2_types() {
    for f in $(type_files); do
        names=$(grep -ohE 'impl(<[^>]*>)? +(Drop|Zeroize|ZeroizeOnDrop|zeroize::Zeroize|zeroize::ZeroizeOnDrop) +for +[A-Za-z0-9_]+' "$f" | sed -E 's/.* for //' | sort -u | tr '\n' ' ')
        awk -v file="$f" -v allowed="$ALLOWED_TYPES_LIST" -v names="$names" '
            function inbody(b,   re) {
                re = "([:(<,&[]|&mut)[[:space:]]*(pub(\\([a-z]+\\))?[[:space:]]+)?([a-z_]+::)*(Secret|SecretText|Zeroizing|SeedBytes|MnemonicBytes|Held[A-Za-z]*|MasterKey|SecretKey|Xpriv|Keypair|PrivateKey|DerivedKey|SecNonce|SlotKey)([^A-Za-z0-9_(]|$)"
                return b ~ re
            }
            BEGIN { OFS = "\t" }
            # The line without its comments and with each string or char
            # literal emptied, so that a bracket or a type name in one is
            # not counted. A block comment or a string may run on to later
            # lines (cdepth, instr); a lifetime is not a char literal.
            function strip(s,   out, i, n, c, d, p, j) {
                if (!cdepth && !instr && s !~ /["\047\/]/) return s
                out = ""; n = length(s); i = 1
                while (i <= n) {
                    c = substr(s, i, 1); d = substr(s, i + 1, 1)
                    if (cdepth) {
                        if (c == "/" && d == "*") { cdepth++; i += 2 }
                        else if (c == "*" && d == "/") { cdepth--; i += 2; if (!cdepth) out = out " " }
                        else i++
                        continue
                    }
                    if (instr == 1) {
                        if (c == "\\") i += 2
                        else if (c == "\"") { instr = 0; out = out "\"\""; i++ }
                        else i++
                        continue
                    }
                    if (instr == 2) {
                        if (c == "\"" && substr(s, i + 1, length(rawh)) == rawh) { instr = 0; out = out "\"\""; i += 1 + length(rawh) }
                        else i++
                        continue
                    }
                    p = substr(s, i - 1, 1)
                    if (c == "/" && d == "/") { sub(/[[:space:]]+$/, "", out); break }
                    if (c == "/" && d == "*") { cdepth = 1; i += 2; continue }
                    if (c == "\"") { instr = 1; i++; continue }
                    if (c == "r" && (p !~ /[A-Za-z0-9_]/ || (p == "b" && substr(s, i - 2, 1) !~ /[A-Za-z0-9_]/)) && match(substr(s, i + 1), /^#*"/)) {
                        instr = 2; rawh = substr(s, i + 1, RLENGTH - 1); i += 1 + RLENGTH; continue
                    }
                    if (c == "\047") {
                        if (d == "\\") {
                            j = index(substr(s, i + 3), "\047")
                            if (j) { out = out "\047\047"; i += 3 + j; continue }
                        } else if (substr(s, i + 2, 1) == "\047") { out = out "\047\047"; i += 3; continue }
                        else if (d != "" && d !~ /[ -~]/) {
                            j = index(substr(s, i + 1, 6), "\047")
                            if (j > 1) { out = out "\047\047"; i += 1 + j; continue }
                        }
                    }
                    out = out c; i++
                }
                return out
            }
            function flush(   toks, t, n, i) {
                if (name != "") {
                    # One record a type: what the final pass needs to follow
                    # nesting. Its tokens are the words of its body.
                    t = body; toks = ""
                    while (match(t, /[A-Za-z_][A-Za-z0-9_]*/)) {
                        if (substr(t, RSTART, 1) ~ /[A-Z]/) toks = toks " " substr(t, RSTART, RLENGTH)
                        t = substr(t, RSTART + RLENGTH)
                    }
                    print "T", file, dline, name, derive, ((inbody(body) || derive ~ /Zeroize/) ? 1 : 0), (index(" " names, " " name " ") ? 1 : 0), toks
                }
                name = ""; body = ""
            }
            collecting { pend = pend " " $0; if ($0 ~ /\)\]/) collecting = 0; next }
            /^[[:space:]]*#\[derive\(/ { pend = $0; pline = FNR; if ($0 !~ /\)\]/) collecting = 1; next }
            pend != "" && inattr { if ($0 ~ /\][[:space:]]*$/) inattr = 0; next }
            pend != "" && /^[[:space:]]*#\[/ { if ($0 !~ /\][[:space:]]*$/) inattr = 1; next }
            pend != "" && /^[[:space:]]*\/\// { next }
            /^[[:space:]]*impl(<[^>]*>)?[[:space:]]+([A-Za-z_:]*::)?Display[[:space:]]+for[[:space:]]+[A-Za-z_]/ {
                l = $0; sub(/^.*Display[[:space:]]+for[[:space:]]+/, "", l); sub(/[^A-Za-z0-9_].*$/, "", l)
                disp[l] = FNR
            }
            { $0 = strip($0); if ($0 == "") $0 = "|" }
            name != "" {
                # The name of an enum variant is not a type it holds; a line
                # inside the parentheses of a wrapped tuple variant is a type.
                if (is_enum && depth == 1 && pdepth == 0) sub(/^[[:space:]]*[A-Z][A-Za-z0-9_]*/, " _")
                body = body " " $0
                pdepth += gsub(/\(/, "(") - gsub(/\)/, ")")
                if (depth_open) { n = gsub(/\{/, "{"); m = gsub(/\}/, "}"); depth += n - m; if (depth <= 0) flush() }
                else if ($0 ~ /;/) flush()
                next
            }
            /(^|[^A-Za-z0-9_])(struct|enum)[[:space:]]+[A-Za-z_]/ && !/\/\// {
                derive = pend; dline = pline; pend = ""
                l = $0; sub(/^.*(struct|enum)[[:space:]]+/, "", l); sub(/[^A-Za-z0-9_].*$/, "", l); name = l
                is_enum = ($0 ~ /(^|[^A-Za-z0-9_])enum[[:space:]]/); pdepth = 0
                body = $0
                if ($0 ~ /\{/) { depth_open = 1; n = gsub(/\{/, "{"); m = gsub(/\}/, "}"); depth = n - m; if (depth <= 0) flush() }
                else { depth_open = 0; if ($0 ~ /;/) flush() }
                next
            }
            { pend = "" }
            END {
                for (t in disp) print "D", file, disp[t], t
                if (cdepth || instr) print file ": a string or block comment is not closed"
            }
        ' "$f" 2>&1 || echo "$f: awk failed (exit $?)"
    done | awk -F'\t' -v allowed="$ALLOWED_TYPES_LIST" '
        # A type is secret when it holds a secret type directly, derives
        # Zeroize or has a Drop of its own; and, repeated until nothing new is
        # found, when its body names a type already found secret and it has no
        # Drop of its own (the Drop is what wipes it). Types are matched by
        # name within a crate, and only a name the crate defines once, since
        # two types of one name cannot be told apart here.
        function excepts(key,   i, n, a) {
            n = split(allowed, a, " ")
            for (i = 1; i <= n; i++)
                if (index(a[i], key "=") == 1) return substr(a[i], length(key) + 2)
            return ""
        }
        BEGIN { split_d[1] = "Clone"; split_d[2] = "PartialEq"; split_d[3] = "Debug" }
        $1 == "T" {
            n++; file[n] = $2; dline[n] = $3; name[n] = $4; derive[n] = $5; drop[n] = $7; toks[n] = $8
            krate[n] = $2; sub(/\/src\/.*$/, "", krate[n])
            defs[krate[n] SUBSEP $4]++
            if ($6 == 1 || $7 == 1) { sec[n] = 1; secname[krate[n] SUBSEP $4] = 1 }
            next
        }
        $1 == "D" { nd++; dfile[nd] = $2; dl[nd] = $3; dname[nd] = $4; next }
        { print }
        END {
            do {
                changed = 0
                for (i = 1; i <= n; i++) {
                    if (sec[i] || drop[i]) continue
                    m = split(toks[i], w, " ")
                    for (j = 1; j <= m; j++)
                        if (((krate[i] SUBSEP w[j]) in secname) && defs[krate[i] SUBSEP w[j]] == 1 && w[j] != name[i]) { sec[i] = 1; secname[krate[i] SUBSEP name[i]] = 1; changed = 1; nested[i] = w[j]; break }
                }
            } while (changed)
            for (i = 1; i <= n; i++) {
                if (!sec[i]) continue
                ex = excepts(file[i] ":" name[i])
                for (k = 1; k <= 3; k++)
                    if (derive[i] ~ ("(^|[^A-Za-z0-9_])" split_d[k] "([^A-Za-z0-9_]|$)") && index("," ex ",", "," split_d[k] ",") == 0) {
                        printf "%s:%d: %s: %s%s\n", file[i], dline[i], name[i], derive[i], (i in nested ? " (holds " nested[i] ")" : "")
                        break
                    }
                secfile[file[i] SUBSEP name[i]] = 1
            }
            for (i = 1; i <= nd; i++)
                if (((dfile[i] SUBSEP dname[i]) in secfile) && index("," excepts(dfile[i] ":" dname[i]) ",", ",Display,") == 0)
                    printf "%s:%d: %s: Display implemented\n", dfile[i], dl[i], dname[i]
        }'
}
report "Clone, PartialEq, Debug derived or Display implemented on a secret type (rule 2)" "$(rule2_types)"

# Rule 3: no heap text in the files that hold secret material.
SECRET_FILES="faraday/faraday-vault/src/contents.rs
faraday/faraday-vault/src/contents/tree.rs
faraday/faraday-vault/src/kdbx/mod.rs
faraday/faraday-vault/src/kdbx/keys.rs
faraday/faraday-vault/src/kdbx/keyfile.rs
faraday/faraday-vault/src/kdbx/blocks.rs
faraday/faraday-vault/src/kdbx/inner.rs
faraday/faraday-vault/src/kdbx/xml/mod.rs
faraday/faraday-vault/src/kdbx/xml/read.rs
faraday/faraday-vault/src/kdbx/items/mod.rs
faraday/faraday-vault/src/kdbx/items/write.rs
faraday/faraday-vault/src/kdbx/items/forms.rs
faraday/faraday-vault/src/kdbx/gzip.rs
faraday/faraday-vault/src/file.rs
core/faraday-crypto/src/secret.rs
core/faraday-bip/src/bip39.rs
core/faraday-bip/src/slip39.rs
core/faraday-bip/src/keys.rs
core/faraday-bip/src/bip85.rs
core/faraday-bip/src/xkey.rs
core/faraday-bip/src/wif.rs
core/faraday-entropy/src/lib.rs
core/faraday-entropy/src/dice.rs
core/faraday-entropy/src/booklet.rs
core/faraday-entropy/src/coins.rs
core/faraday-entropy/src/hex.rs
core/faraday-entropy/src/xor.rs
core/faraday-entropy/src/cards.rs
core/faraday-entropy/src/camera.rs
core/faraday-entropy/src/device.rs
core/faraday-entropy/src/mix.rs
faraday/faraday-vault/src/layouts.rs
core/faraday-bip/src/codex32/mod.rs
core/faraday-bip/src/codex32/worksheet.rs
core/faraday-bip/src/codex32/fresh.rs
core/faraday-bip/src/codex32/correct.rs
core/faraday-bip/src/silent.rs
core/faraday-bip/src/dleq.rs
core/faraday-psbt/src/silent.rs
core/faraday-bip/src/aez.rs
core/faraday-bip/src/aezeed.rs
core/faraday-bip/src/vanity.rs
core/faraday-bip/src/account.rs
core/faraday-bip/src/bsms.rs
core/faraday-bip/src/multisig_config.rs
core/faraday-bip/src/musig.rs
faraday/faraday-core/src/word_entry.rs
faraday/faraday-core/src/quiz.rs
faraday/faraday-core/src/grind.rs
faraday/faraday-pgp/src/paperkey.rs
faraday/faraday-pgp/src/message.rs
faraday/faraday-keys/src/job.rs
faraday/faraday-keys/src/hsm_secret.rs
faraday/faraday-keys/src/slip39_set.rs
faraday/faraday-pgp/src/secret.rs
faraday/faraday-vault/src/qr.rs
faraday/faraday-kef/src/lib.rs
faraday/faraday-core/src/worker/kef.rs
faraday/faraday-core/src/worker.rs
faraday/faraday-keys/src/kef.rs
faraday/faraday-keys/src/qr_vault.rs
faraday/faraday-keys/src/core_wallet.rs
faraday/faraday-vault/src/records.rs
faraday/faraday-vault/src/kdbx/tests.rs
faraday/faraday-ui/src/press.rs
faraday/faraday-scanner/src/lib.rs
faraday/faraday-samples/src/data.rs
faraday/faraday-pgp/src/message/tests.rs
faraday/faraday-keys/src/vault/seal.rs
faraday/faraday-keys/src/vault/edits.rs
faraday/faraday-keys/src/psbt.rs
faraday/faraday-keys/src/parts.rs
faraday/faraday-keys/src/lib.rs
faraday/faraday-host/src/scroll.rs
faraday/faraday-guide/src/how.rs
faraday/faraday-guide/src/block.rs
faraday/faraday-core/src/wallet_backup/shown.rs
faraday/faraday-core/src/screens/spend.rs
faraday/faraday-core/src/screens/files.rs
faraday/faraday-core/src/wallet.rs
faraday/faraday-core/src/wallet/state.rs
faraday/faraday-core/src/vault/screen/secret.rs
faraday/faraday-core/src/vault/area.rs
faraday/faraday-core/src/vault/mod.rs
faraday/faraday-core/src/vault/create_key.rs
faraday/faraday-core/src/vault/dice.rs
faraday/faraday-core/src/vault/more.rs
faraday/faraday-core/src/vault/out.rs
faraday/faraday-core/src/vault/screen/codes.rs
faraday/faraday-core/src/vault/screen.rs
faraday/faraday-core/src/vault/seal.rs
faraday/faraday-core/src/vault/stretch.rs
faraday/faraday-core/src/vault/unlock_keys.rs
faraday/faraday-core/src/nav/mod.rs
faraday/faraday-core/src/fresh/roller/five.rs
faraday/faraday-core/src/codex32/walk.rs
faraday/faraday-core/src/codex32/tracker.rs
faraday/faraday-core/src/scan_key.rs
faraday/faraday-core/src/decode/check.rs
faraday/faraday-core/src/decode/mod.rs
faraday/faraday-core/src/silent/mod.rs
faraday/faraday-core/src/codex32/mod.rs
faraday/faraday-core/src/codex32/screen.rs
faraday/faraday-core/src/codex32_correct/mod.rs
faraday/faraday-core/src/codex32_correct/screen.rs
faraday/faraday-core/src/spend_backup/mod.rs
faraday/faraday-core/src/spend_backup/screen.rs
faraday/faraday-core/src/backups/mod.rs
faraday/faraday-core/src/backups/screen.rs
core/faraday-draw/src/tokens.rs
core/faraday-selftest/src/lib.rs
core/faraday-shell-api/src/lib.rs
faraday/faraday-strings/src/en.rs
core/faraday-crypto/src/pinned.rs
core/faraday-crypto/src/lib.rs
core/faraday-crypto/src/hash.rs
core/faraday-crypto/src/hex.rs
core/faraday-crypto/src/argon2.rs
core/faraday-codec/src/seedqr.rs
core/faraday-codec/src/lib.rs
core/faraday-codec/src/decode.rs
core/faraday-codec/src/png.rs
core/faraday-codec/src/classify.rs
core/faraday-codec/src/qr.rs
core/faraday-codec/src/ur/fountain.rs
core/faraday-codec/src/ur/mod.rs
core/faraday-crypto/src/blake2b.rs
core/faraday-crypto/src/scrypt.rs
core/faraday-crypto/src/wipe.rs
core/faraday-crypto/src/work.rs
core/faraday-entropy/src/pointer_keys.rs
core/faraday-psbt/src/context.rs
core/faraday-psbt/src/lib.rs
core/faraday-psbt/src/musig.rs
core/faraday-psbt/src/message.rs
core/faraday-psbt/src/sign.rs
core/faraday-psbt/src/verify.rs
faraday/faraday-core/src/fresh/page.rs
faraday/faraday-core/src/work/budget.rs
faraday/faraday-core/src/work/budget/codex32.rs
faraday/faraday-core/src/work/budget/identities.rs
faraday/faraday-core/src/work/budget/moments.rs
faraday/faraday-core/src/work/budget/presses.rs
faraday/faraday-core/src/work/budget/shown.rs
faraday/faraday-core/src/work.rs
faraday/faraday-core/src/drawing/allowed.rs
faraday/faraday-core/src/explore/shown.rs
faraday/faraday-core/src/paper.rs
faraday/faraday-keys/src/message.rs
faraday/faraday-keys/src/vanity.rs
faraday/faraday-sb/src/rng.rs
faraday/faraday-strings/src/bip.rs
core/faraday-text/src/lib.rs
core/faraday-text/src/text.rs
core/faraday-text/src/num.rs
faraday/faraday-intake/src/secret_buf.rs
aqrv/aqrv/src/codex32.rs
aqrv/aqrv/src/envelope.rs
aqrv/aqrv/src/item.rs
aqrv/aqrv-app/src/draw.rs
aqrv/aqrv-app/src/lib.rs
aqrv/aqrv-app/src/samples.rs
aqrv/aqrv-app/src/state.rs
aqrv/aqrv-app/src/worker.rs
aqrv/aqrv-cli/src/main.rs
aqrv/aqrv-cli/src/tty.rs
faraday/faraday-disk/src/lib.rs
faraday/faraday-disk/src/main.rs
faraday/faraday-fat/src/lib.rs
faraday/faraday-files/src/lib.rs
faraday/faraday-files/src/proto.rs
faraday/faraday-sqlite/src/lib.rs
faraday/faraday-storage/src/lib.rs
faraday/faraday-core/src/testkit.rs
faraday/faraday-core/src/samples/mod.rs
faraday/faraday-core/src/review.rs
faraday/faraday-core/src/convert/mod.rs
faraday/faraday-core/src/convert/set.rs
faraday/faraday-core/src/core_wallet/tool.rs
faraday/faraday-core/src/ots_check/hash.rs
faraday/faraday-core/src/ots_check/mod.rs
faraday/faraday-core/src/boot_import/mod.rs
faraday/faraday-core/src/boot_import/text_ask.rs
faraday/faraday-core/src/compact/mod.rs
faraday/faraday-core/src/forms.rs
faraday/faraday-core/src/stick/pills.rs
faraday/faraday-core/src/bip85/mod.rs
faraday/faraday-qr/src/bbqr.rs
faraday/faraday-qr/src/send.rs
faraday/faraday-intake/src/lib.rs
faraday/faraday-intake/src/wallet.rs
faraday/faraday-core-wallet/src/lib.rs
faraday/faraday-core-wallet/src/normal.rs
faraday/faraday-sb/src/x509.rs
faraday/faraday-host/src/lib.rs
faraday/faraday-plan/src/codex32.rs
faraday/faraday-plan/src/lib.rs
faraday/faraday-plan/src/text.rs
faraday/faraday-vault/src/kdbx/items/text.rs
faraday/faraday-vault/src/lib.rs
faraday/faraday-vault/src/entries.rs
faraday/faraday-vault/src/totp.rs
faraday/faraday-pgp/src/certificate.rs
faraday/faraday-pgp/src/lib.rs
faraday/faraday-core/src/qr_view.rs
faraday/faraday-core/src/qr_tool/item.rs
faraday/faraday-core/src/qr_tool/mod.rs
faraday/faraday-core/src/qr_tool/offers.rs
faraday/faraday-core/src/qr_tool/open.rs
faraday/faraday-core/src/scan_read.rs
faraday/faraday-core/src/transfer/mod.rs
faraday/faraday-core/src/transfer/screen.rs
faraday/faraday-core/src/transfer/text.rs
faraday/faraday-core/src/lightning/mod.rs
faraday/faraday-core/src/lightning/screen.rs
faraday/faraday-core/src/chart_edit.rs
faraday/faraday-core/src/create.rs
faraday/faraday-core/src/keygen/area.rs
faraday/faraday-core/src/keygen/mod.rs
faraday/faraday-core/src/keygen/screen.rs
faraday/faraday-core/src/seeds/kef.rs
faraday/faraday-core/src/seeds/mod.rs
faraday/faraday-core/src/seeds/read.rs
faraday/faraday-core/src/seeds/sweep.rs
faraday/faraday-core/src/backup_flow/steps.rs
faraday/faraday-core/src/backup_flow/gpg_restore.rs
faraday/faraday-core/src/wallet_backup.rs
faraday/faraday-core/src/wallet_backup/kef.rs
faraday/faraday-core/src/compare/mod.rs
faraday/faraday-core/src/tools/mod.rs
faraday/faraday-core/src/codes/mod.rs
faraday/faraday-core/src/copy_check.rs
faraday/faraday-core/src/item.rs
faraday/faraday-keys/src/bip85.rs
faraday/faraday-keys/src/codex32.rs
faraday/faraday-keys/src/gpg.rs
faraday/faraday-keys/src/hsm_node.rs
faraday/faraday-keys/src/identity.rs
faraday/faraday-keys/src/lightning.rs
faraday/faraday-keys/src/operations.rs
faraday/faraday-keys/src/session.rs
faraday/faraday-keys/src/silent.rs
faraday/faraday-keys/src/texts.rs
faraday/faraday-keys/src/totp.rs
faraday/faraday-keys/src/vault/mod.rs
faraday/faraday-keys/src/vault/reveals.rs
faraday/faraday-keys/src/vault/tests.rs
faraday/faraday-core/src/screens/entry.rs
faraday/faraday-core/src/screens/mod.rs
faraday/faraday-core/src/screens/parts.rs
faraday/faraday-core/src/screens/sheets.rs
faraday/faraday-core/src/screens/visit.rs
faraday/faraday-core/src/restore.rs
faraday/faraday-core/src/gpg.rs
faraday/faraday-core/src/signing.rs
faraday/faraday-core/src/signing/collect.rs
faraday/faraday-core/src/signing/read.rs
faraday/faraday-core/src/secrets.rs
faraday/faraday-core/src/codex32_backup/mod.rs
faraday/faraday-core/src/glance/mod.rs
faraday/faraday-core/src/glance/sheet.rs
faraday/faraday-core/src/inbox.rs
faraday/faraday-core/src/visit_taken.rs
faraday/faraday-core/src/storage_api.rs
faraday/faraday-core/src/backup_flow/mod.rs
faraday/faraday-core/src/backup_flow/wallet.rs
faraday/faraday-core/src/kef_form/mod.rs
faraday/faraday-core/src/plan/cards.rs
faraday/faraday-core/src/explore/screen.rs
faraday/faraday-core/src/picture.rs
faraday/faraday-core/src/inputs.rs
faraday/faraday-core/src/fresh.rs
faraday/faraday-core/src/fresh/roller/mod.rs
faraday/faraday-core/src/edit.rs
faraday/faraday-core/src/dice_entry/mod.rs
faraday/faraday-core/src/kef_tool/mod.rs
core/faraday-draw/src/canvas/list.rs
faraday/faraday-core/src/qr_tool/screen.rs
faraday/faraday-qr/src/specter.rs
faraday/faraday-core/src/backup_flow/wallet_files.rs
faraday/faraday-core/src/compute.rs
faraday/faraday-core/src/worker/kef_make.rs
faraday/faraday-core/src/held.rs"
# One path per line above; the checks below match a path between spaces.
SECRET_FILES=$(echo $SECRET_FILES)
for f in $SECRET_FILES; do
    [ -f "$f" ] || { echo "lint-secrets: missing $f (update SECRET_FILES)"; status=1; }
done
# Files in SECRET_FILES that hold a secret without naming a secret type, each
# with why:
#   codex32/fresh.rs   makes shares from entropy through the types of mod.rs
#   vault/qr.rs        takes the passphrase as text for the PGP layer's seal
#   worker/kef.rs      holds the KEF worker's answers behind faraday-keys calls
#   keys/totp.rs       holds the shown one-time code in wiping text
#   vault/dice.rs, vault/screen.rs   hold only public text and wiping text of the vault and its draw
#   faraday-text lib   declares and documents the wiping text type of text.rs
#   codex32_backup     holds the codex32 strings typed back and made as wiping text
#   glance/sheet.rs    shows a passphrase it reads from the store as wiping text
#   dice_entry/mod.rs holds a diceware passphrase and the rolls as wiping text
#   kef_tool/mod.rs    holds a KEF plaintext as wiping text
#   restore.rs         holds the passphrase and the typed backup text of a restore as wiping text
#   canvas/list.rs     draws the list's masked text rows from wiping text
#   qr_tool/screen.rs  holds the QR vault's passphrase and note as wiping text
#   specter.rs         holds the Specter payload it reads or makes as wiping text
#   wallet_files.rs    holds the wallet files' text as wiping text
#   held.rs            lends a key's revealed passphrase into the vault record it writes
#   codes/mod.rs       holds the entry's account and title as wiping text and the shown one-time code
#   spend_backup/      read a share or a key typed or scanned as wiped text (Text)
#                      and hand it on to the seeds area
NAMELESS="core/faraday-bip/src/codex32/fresh.rs faraday/faraday-vault/src/qr.rs faraday/faraday-core/src/worker/kef.rs core/faraday-text/src/lib.rs faraday/faraday-keys/src/totp.rs faraday/faraday-core/src/vault/dice.rs faraday/faraday-core/src/vault/screen.rs faraday/faraday-core/src/restore.rs faraday/faraday-core/src/codex32_backup/mod.rs faraday/faraday-core/src/glance/sheet.rs faraday/faraday-core/src/spend_backup/mod.rs faraday/faraday-core/src/spend_backup/screen.rs faraday/faraday-core/src/dice_entry/mod.rs faraday/faraday-core/src/kef_tool/mod.rs core/faraday-draw/src/canvas/list.rs faraday/faraday-core/src/qr_tool/screen.rs faraday/faraday-qr/src/specter.rs faraday/faraday-core/src/backup_flow/wallet_files.rs faraday/faraday-core/src/held.rs faraday/faraday-core/src/codes/mod.rs"
EXCEPTED_FILE=tools/lint-secrets-excepted.txt
EXCEPTED=$(sed -n 's/^\([^#:][^:]*\): .*$/\1/p' "$EXCEPTED_FILE")
names_secret() { grep -qE "$SECRET_NAMES" "$1"; }
has_heap_text() { grep -E '\bString\b|alloc::string|\bto_string\b|\bto_owned\b|\bformat!' "$1" | grep -qvE 'b"[^"]*</?String>|^ +[a-z0-9_]+: "String",$'; }
unlisted=$(for f in $(type_files); do
    names_secret "$f" || continue
    case " $SECRET_FILES " in *" $f "*) continue ;; esac
    grep -qxF "$f" <<< "$EXCEPTED" || echo "$f"
done)
report "a file that uses a secret type is in neither SECRET_FILES nor $EXCEPTED_FILE (rule 3)" "$unlisted"
nameless=$(for f in $SECRET_FILES; do
    [ -f "$f" ] || continue
    names_secret "$f" && continue
    case " $NAMELESS " in *" $f "*) ;; *) echo "$f" ;; esac
done)
report "a file in SECRET_FILES that uses no secret type (rule 3; remove it, or name it in NAMELESS with why)" "$nameless"
stale=$(for f in $EXCEPTED; do
    if [ ! -f "$f" ]; then echo "$f: missing"
    elif ! has_heap_text "$f"; then echo "$f: has no heap text: move it to SECRET_FILES"
    elif case " $SECRET_FILES " in *" $f "*) true ;; *) false ;; esac; then echo "$f: also in SECRET_FILES"
    fi
done; sort <<< "$EXCEPTED" | uniq -d | sed 's/$/: listed twice/')
report "$EXCEPTED_FILE is out of date (rule 3)" "$stale"
report "heap text in secret-handling source (rule 3)" \
    "$(grep -nE '\bString\b|alloc::string|\bto_string\b|\bto_owned\b|\bformat!' $SECRET_FILES \
        | grep -vE 'b"[^"]*</?String>|^[^:]+:[0-9]+: +[a-z0-9_]+: "String",$')"

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
#       open, open_with, create, derive, seal, Opened; and the file API,
#       file::open, seal, create, measure, Opened, kdbx::open, seal and
#       the QR vault's qr::open, seal), a KEF code opened or sealed
#       (faraday_kef open, seal, their _with_key forms, Opened), a PSBT signed
#       (faraday_psbt::sign, and with a WIF key, faraday_psbt::sweep::sign),
#       a WIF read into a key (faraday_bip::wif), BIP-352's receiver, an
#       aezeed decoded;
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
# renders and test data); faraday-core's testkit.rs and testkit/ (the `testkit`
# feature, which tools/lint-keys-tests.sh keeps out of every shipped
# binary); faraday-guide's how.rs, which the manifest generates and which
# names the functions as text (docs/CRYPTO-LEARN.md §3.1) without calling any.
# faraday-intake is exempt from 4b only: it reads text as it
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
        -not -path 'faraday/faraday-core/src/testkit.rs' -not -path 'faraday/faraday-core/src/testkit/*' \
        -not -path 'faraday/faraday-guide/src/how.rs' | sort
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
# enum variant work.rs declares for it. faraday-strings' bip.rs words
# MuSig2's error kind `SecNonce` (a nonce missing or used twice), which
# names the error, not a nonce in hand.
report "a key or seed outside faraday-keys (rule 4a)" \
    "$(in_code "$secret_types" "${BOUNDARY[@]}" \
        | sed -E 's/\b(Work|C)::MasterKey\b//g' \
        | grep -E "$secret_types" \
        | grep -vE '^faraday/faraday-core/src/work\.rs:[0-9]+:[[:space:]]*MasterKey,$' \
        | grep -vE '^faraday/faraday-strings/src/bip\.rs:[0-9]+:[[:space:]]*E::SecNonce => EN\.bip_musig_sec_nonce,$')"
report "seed words made outside faraday_keys::words (rule 4b)" \
    "$(in_code 'Mnemonic::(parse|from_indices|from_entropy|from_phrase)|seedqr::from_(entropy|digits)|slip39::split|records::words_of' "${WORDS_FILES[@]}")"
report "a primitive's secret-taking function outside faraday-keys (rule 4c)" \
    "$(in_code 'faraday_pgp::(Key|paperkey|restore|Restored|certificate::(restore|Restored))\b|faraday_sb::(make|enrolment|Keys|Pair|x509::Pair|pe::sign)\b|faraday_vault::(open|open_with|create|derive|seal|Opened)\b|faraday_vault::file::(open|seal|create|measure|Opened)\b|faraday_vault::kdbx::(open|seal)\b|\b(fv|faraday_vault)::qr\b|use faraday_vault::\{[^}]*\bqr\b|use faraday_vault::file::(open|seal|create|measure|Opened)\b|use faraday_vault::kdbx::(open|seal)\b|use faraday_vault::qr::(open|seal)\b|faraday_kef::(open|open_with_key|seal|seal_with_key|Opened)\b|use faraday_kef::\{[^}]*\b(open|open_with_key|seal|seal_with_key|Opened)\b|use faraday_kef::(open|open_with_key|seal|seal_with_key|Opened)\b|faraday_psbt::sign\(|faraday_psbt::sweep::sign\(|faraday_bip::wif\b|faraday_bip::silent::Receiver|aezeed::(decode|node_key)|use faraday_(pgp|sb|vault)::\{[^}]*\b(Key|paperkey|restore|Restored|make|enrolment|Keys|Pair|open|open_with|create|derive|seal|Opened)\b' "${BOUNDARY[@]}")"
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
