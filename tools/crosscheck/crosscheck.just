# The crypto cross-check (F79, docs/CROSSCHECK.md §10;
# tools/crosscheck/README.md): Faraday's Rust against the vendored Python
# references, offline, over every published vector and a seeded random
# corpus, on one worker a core. A passing run writes tools/crosscheck/stamp,
# which `just lint` holds the crypto to. `--fresh` before a release (a new
# seed, ten times the cases); `--area NAME` for one area; `--jobs N` for N
# workers.
[group('test')]
crosscheck *args:
    tools/crosscheck/run.sh {{args}}

# How far a running cross-check has got, from another terminal:
# `--follow` until it ends, `--log` for its log (tools/crosscheck/status.py).
[group('test')]
crosscheck-status *args:
    python3 -I -B tools/crosscheck/status.py {{args}}

# The harness's own tests (tools/crosscheck/test_harness.py): its
# shortcuts give the references' answers, and the report is the same for
# any number of workers. No Rust, no network, about 20 seconds.
[group('test')]
crosscheck-test:
    python3 -I -B tools/crosscheck/test_harness.py

# The real areas under 1 and 4 workers (and PyPy when found) give the same
# report (tools/crosscheck/identity.py), at a small scale.
[group('test')]
crosscheck-identity *args:
    tools/crosscheck/run.sh --identity {{args}}
