# The task runner. Everything runs locally; there is no hosted CI.

[group('test')]
default: test lint

# Run every unit, integration and doc test in the workspace. nextest runs
# the test binaries in parallel where `cargo test` runs them one after
# another; it does not run doc tests, so those
# follow. `cargo install cargo-nextest --locked` puts it on a machine.
[group('test')]
test:
    cargo nextest run --locked --workspace
    cargo test --locked --workspace --doc

# Clippy with warnings as errors, and failing on a warning about its own
# configuration (tools/clippy.py), formatting check, the secret-hygiene
# lint, the user-facing-strings lint, the design-token lint, the costs
# lint (who takes, mints and keeps the compute token, docs/COMPUTE.md
# §8), the check that no shipped binary compiles the token that counts
# nothing (docs/COMPUTE.md §3), and the Starter's macOS code checked for
# both Mac targets without a Mac (tools/check-macos.sh), the docs lint
# (a Kind and Status header on every document, every cited path real,
# tools/lint-docs.sh), and the layout lint (the code map, file sizes and
# long functions, counts that only fall, tools/lint-layout.sh), the tree
# lint (the top level's entries, no leftovers, no file over 2 MiB,
# tools/lint-tree.sh), and cargo
# vet: every dependency trusted, audited or exempted (F71), and the guide
# lint (bold names are the app's strings, links resolve, tools/lint-guide.py).
[group('lint')]
lint:
    python3 tools/clippy.py --locked --workspace --all-targets
    # Advisories, licences, duplicates and sources, against deny.toml.
    # `cargo install --locked cargo-deny` puts it on a machine (README,
    # "Faraday Demo, for developers").
    cargo deny check
    # Every dependency trusted, audited or exempted, against the committed
    # imports, offline (F71, docs/STRUCTURE-PASS.md §6). `cargo install
    # --locked cargo-vet` puts it on a machine; `just vet-refresh` updates
    # the imports when chosen.
    cargo vet --locked
    # The two fuzz crates are workspaces of their own: vetted against the
    # same store (docs/deps/libfuzzer-sys.md).
    cargo vet --locked --manifest-path fuzz/Cargo.toml --store-path supply-chain
    cargo vet --locked --manifest-path faraday/fuzz/Cargo.toml --store-path supply-chain
    cargo fmt --all --check
    tools/lint-secrets.sh
    tools/lint-strings.sh
    tools/lint-tokens.sh
    tools/lint-docs.sh
    tools/lint-layout.sh
    tools/lint-tree.sh
    python3 -I -B tools/lint-builds.py
    cargo run -q -p costs-lint
    tools/lint-unchecked.sh
    tools/lint-crosscheck.sh
    # Each cross-check area's facts are stated, their paths resolve, and the
    # README's table and known differences are what they say.
    python3 -I -B tools/crosscheck/facts.py check
    # The colour tables are what the vendored material-color-utilities
    # makes (docs/THEMES.md §11.3).
    tools/lint-themes.sh
    tools/lint-keys-tests.sh
    tools/check-macos.sh
    python3 tools/learn/sync.py check
    # The guide's bold names are the app's strings, its links resolve.
    python3 -B tools/lint-guide.py --self-test
    python3 -B tools/lint-guide.py
    # The registry of what a change brings with it reads, and matches the
    # tree and this recipe (docs/IMPACT.md §2.5).
    python3 -I -B tools/impact.py lint
    # No pair of the registry left unmet in the range from main (F149).
    python3 -I -B tools/impact.py check --quiet
    # docs/CHECKS.md is what the registry makes (`tools/impact.py docs` writes it).
    python3 -I -B tools/impact.py docs --check

# Fetch the audit sets supply-chain/config.toml imports, write them to
# imports.lock, and drop exemptions they now cover (F71), keeping those
# the fuzz workspaces still need; then vet all three offline
# (tools/vet-refresh.py). Online; run it when chosen, and when a trust
# entry is renewed before its end date.
[group('lint')]
vet-refresh:
    python3 tools/vet-refresh.py

# Regenerate the embedded BIP-39 wordlists from the BIP repository and
# the SLIP-39 list from the SLIPs repository (needs network).
[group('dev')]
wordlists:
    python3 tools/wordlists/gen.py
    python3 tools/wordlists/gen-slip39.py
    cargo fmt --all

# Regenerate the embedded EFF diceware wordlists from the lists committed
# under tools/vectors/eff/ (offline).
[group('dev')]
diceware:
    python3 tools/diceware/gen.py
    cargo fmt --all

# Regenerate the colour tables, faraday/faraday-ui/src/pal/schemes.rs,
# from the themes' seeds with the vendored material-color-utilities
# (docs/THEMES.md §10.2). Offline; needs Node 22 or later.
[group('dev')]
themes:
    node tools/themes/gen.mjs

# Re-extract the glyph outlines from tools/fonts/ into core/faraday-draw (offline).
[group('dev')]
fonts:
    cargo run -p faraday-fontbake --release
    cargo fmt --all

# Rewrite the one PSBT per policy warning, and the registered-wallet
# policy and transaction, in tools/vectors/psbt/ (offline).
[group('dev')]
psbt-fixtures:
    cargo run -p faraday-psbt --release --example warnings -- tools/vectors/psbt
    cargo run -p faraday-psbt --release --example wallet -- tools/vectors/psbt
    cargo run -p faraday-psbt --release --example musig -- tools/vectors/psbt
    cargo run -p faraday-psbt --release --example tapmulti -- tools/vectors/psbt

# --- Fuzzing (fuzz/, faraday/fuzz/) -----------------
#
# Local only: no hosted CI runs this. It needs cargo-fuzz and the nightly
# the fuzz crates pin (fuzz/rust-toolchain.toml, a dated one, so a run is
# reproduced from its commit), both at user level:
#
#   cargo install cargo-fuzz --locked
#   rustup toolchain install nightly-2026-10-13 --profile minimal
#
#   just fuzz-list                  the targets of both directories
#   just fuzz classify              one target for the default time
#   just fuzz decode_luma 300       one target for 300 seconds
#   just fuzz-release               every target, 10 minutes each (before a release)
#   just fuzz-corpus-export         every target's corpus, minimized, in out/fuzz-corpus/
#
# Each fuzz crate is its own workspace (the root Cargo.toml excludes
# them), so a sanitizer build never shares this workspace's target
# directory or lock file; each has its own committed Cargo.lock. cargo-fuzz
# has no --locked, so a recipe checks the lock with `cargo metadata
# --locked` first: a lock that is out of date fails instead of changing.
# `fuzz` finds a target's directory itself.

fuzz_seconds := "60"
fuzz_nightly := `sed -n 's/^channel *= *"\(.*\)"$/\1/p' fuzz/rust-toolchain.toml`

# The fuzz targets, one per untrusted-input parser: `fuzz/` first (the
# shared crates), then `faraday/fuzz/` (Faraday's own).
[group('test')]
fuzz-list:
    cargo +{{fuzz_nightly}} fuzz list --fuzz-dir fuzz
    cargo +{{fuzz_nightly}} fuzz list --fuzz-dir faraday/fuzz

# Run one target of either directory. SECONDS defaults to fuzz_seconds;
# give decode_luma and psbt_parse longer, since they are the widest
# surfaces.
[group('test')]
fuzz TARGET seconds=fuzz_seconds:
    #!/usr/bin/env bash
    set -euo pipefail
    for dir in fuzz faraday/fuzz; do
        listing="$(cargo +{{fuzz_nightly}} fuzz list --fuzz-dir "$dir" 2>/dev/null)"
        if grep -qx '{{TARGET}}' <<<"$listing"; then
            cargo +{{fuzz_nightly}} metadata --locked --format-version 1 --manifest-path "$dir/Cargo.toml" >/dev/null
            exec cargo +{{fuzz_nightly}} fuzz run --fuzz-dir "$dir" '{{TARGET}}' -- -max_total_time={{seconds}}
        fi
    done
    echo "just fuzz: no target named {{TARGET}} (just fuzz-list)" >&2
    exit 1

# The fuzz run before a release (docs: tools/README.md, "Before a
# release"): every target of both directories for SECONDS each (600), as
# many at once as there are cores, each keeping its corpus between runs.
# Any crash fails it. It writes out/fuzz-release.txt for the release
# notes. Run it on the build machine and take the record from there.
# FARADAY_FUZZ_CORPUS=DIR starts from a published corpus.
[group('test')]
fuzz-release seconds="600":
    tools/fuzz-release.sh {{seconds}}

# Each target's kept corpus and seeds, minimized with `cargo fuzz cmin`
# into out/fuzz-corpus/<target>/, ready to commit to the corpus
# repository after a release. On the machine that ran fuzz-release.
[group('test')]
fuzz-corpus-export:
    tools/fuzz-corpus-export.sh

# The Learn pages as Markdown: docs/learn/ is where the text is edited,
# en.rs is what the device reads; `lint` fails while the two differ.
[group('dev')]
learn-export:
    python3 tools/learn/sync.py export

[group('dev')]
learn-import:
    python3 tools/learn/sync.py import
    cargo fmt --all

# The cryptographic actions' manifest (docs/CRYPTO-LEARN.md, docs/crypto/check.py).
# `crypto-review ID` prints an action's entry, what changed in its code and
# every place to read with it, then writes its fingerprint; it is the only
# writer of one, and refuses while the entry is untrue.
[group('dev')]
crypto-review id:
    python3 -I -B docs/crypto/check.py review {{id}}

# The actions whose fingerprint changed since a tag, for the release notes
# and the deep cross-check before a release.
[group('dev')]
crypto-changes tag:
    python3 -I -B docs/crypto/check.py changes {{tag}}

# Faraday's own recipes: faraday/faraday.just.
import 'faraday/faraday.just'
import 'tools/crosscheck/crosscheck.just'
