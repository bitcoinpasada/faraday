#!/usr/bin/env bash
# `just crosscheck`: builds Faraday's side of the crypto cross-check
# offline and runs the Python side with no network (F79,
# docs/CROSSCHECK.md §10.4). Arguments go to crosscheck.py:
#
#   tools/crosscheck/run.sh                 the stamp run
#   tools/crosscheck/run.sh --fresh         before a release
#   tools/crosscheck/run.sh --area musig    one area, no stamp
#   tools/crosscheck/run.sh --jobs 4        four workers (default: one a core)
#   tools/crosscheck/run.sh --identity      the same report under 1 and 4 workers
#                                           (and PyPy when found): identity.py
#
# The run's progress goes to out/crosscheck/ (progress.log from the build
# on, progress.json, report.txt); `just crosscheck-status` reads it.
set -euo pipefail
cd "$(dirname "$0")/../.."

progress=out/crosscheck
mkdir -p "$progress"
rm -f "$progress/progress.json" "$progress/report.txt"
say() { echo "$(date +%H:%M:%S) $*" | tee -a "$progress/progress.log" >&2; }
: > "$progress/progress.log"
say "build: the Rust command (cargo build --frozen -p faraday-crosscheck)"

exe=$(cargo build --frozen -q -p faraday-crosscheck --message-format=json-render-diagnostics \
    | python3 -I -c '
import json, sys
for line in sys.stdin:
    m = json.loads(line)
    if m.get("reason") == "compiler-artifact" and m.get("executable") \
            and m["target"]["name"] == "faraday-crosscheck":
        print(m["executable"])
' | tail -n 1)
if [ -z "$exe" ]; then
    say "build: FAILED: the Rust command did not build"
    exit 2
fi
say "build: done"

# A network namespace of its own, with loopback only and that down:
# whatever tries to fetch fails instead of downloading.
if ! unshare --user --map-root-user --net true 2>/dev/null; then
    say "crosscheck: this machine does not let a user make a network namespace"
    echo "(unshare --user --net); see tools/crosscheck/README.md" >&2
    exit 2
fi
if [ "${1:-}" = --identity ]; then
    shift
    exec unshare --user --map-root-user --net \
        python3 -I -B tools/crosscheck/identity.py --rust "$exe" "$@"
fi
exec unshare --user --map-root-user --net \
    python3 -I -B tools/crosscheck/crosscheck.py --rust "$exe" --progress "$progress" "$@"
