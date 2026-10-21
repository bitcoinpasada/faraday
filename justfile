# The task runner. Everything runs locally; there is no hosted CI.

default: test lint

# Run every unit, integration and doc test in the workspace. nextest runs
# the test binaries in parallel where `cargo test` runs them one after
# another (docs/PLANNING.md §16.119); it does not run doc tests, so those
# follow. `cargo install cargo-nextest --locked` puts it on a machine.
test:
    cargo nextest run --workspace
    cargo test --workspace --doc

# Clippy with warnings as errors, formatting check, the secret-hygiene
# lint, the user-facing-strings lint, the design-token lint, and the
# Starter's macOS code checked for both Mac targets without a Mac
# (tools/check-macos.sh).
lint:
    cargo clippy --workspace --all-targets -- -D warnings
    # Advisories, licences, duplicates and sources, against deny.toml.
    # `cargo install --locked cargo-deny` puts it on a machine (README,
    # "Faraday Desktop, for developers").
    cargo deny check
    cargo fmt --all --check
    tools/lint-secrets.sh
    tools/lint-strings.sh
    tools/lint-tokens.sh
    tools/check-macos.sh
    python3 tools/learn/sync.py check

# BIP 445's reference implementation as the second share of the
# threshold fixture spend (docs/PLANNING.md §16.103). Not part of the
# default `just`, because it is Python and the default recipe is Rust;
# the reference and its curve library are vendored under
# tools/reference/bip445/ and nothing is fetched.
threshold-reference:
    python3 tools/scripts/threshold-reference.py

# Regenerate the embedded BIP-39 wordlists from the BIP repository and
# the SLIP-39 list from the SLIPs repository (needs network).
wordlists:
    python3 tools/wordlists/gen.py
    python3 tools/wordlists/gen-slip39.py
    cargo fmt --all

# Regenerate the embedded EFF diceware wordlists from the lists committed
# under tools/vectors/eff/ (offline).
diceware:
    python3 tools/diceware/gen.py
    cargo fmt --all

# Re-extract the glyph outlines from tools/fonts/ into core/osk-ui (offline).
fonts:
    cargo run -p osk-fontbake --release
    cargo fmt --all

# Rewrite the one PSBT per policy warning, and the registered-wallet
# policy and transaction, in tools/vectors/psbt/ (offline).
psbt-fixtures:
    cargo run -p osk-psbt --release --example warnings -- tools/vectors/psbt
    cargo run -p osk-psbt --release --example wallet -- tools/vectors/psbt
    cargo run -p osk-psbt --release --example musig -- tools/vectors/psbt
    cargo run -p osk-psbt --release --example threshold -- tools/vectors/psbt
    cargo run -p osk-psbt --release --example tapmulti -- tools/vectors/psbt

# --- Fuzzing (fuzz/, docs/PLANNING.md §12) --------------------------------
#
# Local only: no hosted CI runs this, and no release depends on it. It
# needs a nightly toolchain and cargo-fuzz, both at user level:
#
#   rustup toolchain install nightly --profile minimal
#   cargo install cargo-fuzz --locked
#
#   just fuzz-list             the targets
#   just fuzz classify         one target for the default time
#   just fuzz decode_luma 300  one target for 300 seconds
#
# The fuzz crate is its own workspace (the root Cargo.toml excludes it),
# so a sanitizer build never shares this workspace's target directory or
# lock file.

fuzz_seconds := "60"

# The fuzz targets, one per untrusted-input parser.
fuzz-list:
    cd fuzz && cargo +nightly fuzz list

# Run one target. SECONDS defaults to fuzz_seconds; give decode_luma and
# psbt_parse longer, since they are the widest surfaces.
fuzz TARGET seconds=fuzz_seconds:
    cd fuzz && cargo +nightly fuzz run {{TARGET}} -- -max_total_time={{seconds}}

# The Learn pages as Markdown: docs/learn/ is where the text is edited,
# en.rs is what the device reads; `lint` fails while the two differ.
learn-export:
    python3 tools/learn/sync.py export

learn-import:
    python3 tools/learn/sync.py import
    cargo fmt --all

# Faraday's own recipes: faraday/faraday.just.
import? 'faraday/faraday.just'
