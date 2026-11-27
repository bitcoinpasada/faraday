# The check. Recipes for tests and the lints come with the code they check.
default: check clippy fmt deny vet

check:
    cargo check --workspace --all-targets --locked

clippy:
    cargo clippy --workspace --all-targets --locked -- -D warnings

fmt:
    cargo fmt --all --check

# Advisories, licences, duplicates and sources, against deny.toml.
deny:
    cargo deny check

# Every dependency trusted, audited or exempted, against the committed imports, offline.
vet:
    cargo vet --locked
