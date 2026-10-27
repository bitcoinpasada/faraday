# The crypto cross-check (F79, docs/CROSSCHECK.md §10;
# tools/crosscheck/README.md): Faraday's Rust against the vendored Python
# references, offline, over every published vector and a seeded random
# corpus, about 3 to 4 minutes. A passing run writes tools/crosscheck/stamp,
# which `just lint` holds the crypto to. `--fresh` before a release (a new
# seed, ten times the cases); `--area NAME` for one area.
[group('test')]
crosscheck *args:
    tools/crosscheck/run.sh {{args}}

# The harness's own tests (tools/crosscheck/test_harness.py): its
# shortcuts give the references' answers. No Rust, no network, a second.
[group('test')]
crosscheck-test:
    python3 -I -B tools/crosscheck/test_harness.py
