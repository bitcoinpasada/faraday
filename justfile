# The check. Recipes for tests and the lints come with the code they check.
default: check clippy fmt deny vet test lint

check:
    cargo check --workspace --all-targets --locked

clippy:
    python3 -I -B tools/clippy.py --workspace --all-targets --locked -- -D warnings

fmt:
    cargo fmt --all --check

# Every test of the workspace, each run to its end (07 TS21).
test:
    cargo nextest run --workspace --locked --no-fail-fast

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
    # The registry of what a change brings with it reads, and matches the tree (08 section 6).
    python3 -I -B tools/impact.py lint
    just lint-tests

# The lints' own tests: each rule refused on a fixture tree.
lint-tests:
    python3 -I -B -m unittest discover -s tools/tests -p 'test_*.py'

# The mapping check (10 §4, MG4): every file of `main` at its tag has a row. Not in `lint` yet:
# it fails until the mapping is whole (the rows of the ported and rewritten crates are not written).
lint-migration:
    python3 -I -B tools/migration.py check
