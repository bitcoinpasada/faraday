#!/usr/bin/env bash
# The compile of one check job's dependencies, for check.yml's `warm` job (a
# push to main whose head already passed, which saves the result to main's cache)
# and its `warm-commands` job (every dispatch, which runs the same commands and
# saves nothing, so a command cargo refuses fails the check before main, F265).
# Each is the compile of that job's own command, without its run:
# doc-tests: cargo refuses --doc with --no-run; --list builds the libraries
# and dev-dependencies the doc tests link against and runs none of them.
# lint-clippy's deps are cargo check's, as clippy builds a dependency with
# plain rustc; lint-deps compiles nothing in target/ (the fuzz crates have
# their own folders), so its cache is only saved.
# Usage: tools/warm-compile.sh JOB   (RUNNER_TEMP names where an archive goes)
set -euo pipefail
job="${1:?usage: warm-compile.sh JOB}"
archive="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/warm.tar.zst"
case "$job" in
  build-tests)        cargo nextest archive --locked --workspace --archive-file "$archive" ;;
  doc-tests)          cargo test --locked --workspace --doc -- --list ;;
  lint-clippy)        cargo check --locked --workspace --all-targets ;;
  lint-deps)          ;;
  lint-repo)          cargo build --locked -p costs-lint ;;
  lint-touch-targets) cargo nextest archive --locked -p faraday-snapshot -p faraday-starter --archive-file "$archive" ;;
  lint-op-cost)       cargo build --locked --release -p op-cost ;;
  *) echo "unknown job $job" >&2; exit 1 ;;
esac
