# The check. Recipes for tests, fmt and the lints come with the code they check.
default: check clippy

check:
    cargo check --workspace --all-targets --locked

clippy:
    cargo clippy --workspace --all-targets --locked -- -D warnings
