# The check. Recipes for tests and the lints come with the code they check.
default: check clippy fmt deny vet lint

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

# Every lint of `tools/`, each over the whole workspace, and the lints' own tests. One command per line.
lint:
    python3 -I -B tools/lint-layers.py
    python3 -I -B tools/lint-test-headers.py
    just lint-tests

# The lints' own tests: each rule refused on a fixture tree.
lint-tests:
    python3 -I -B -m unittest discover -s tools/tests -p 'test_*.py'
