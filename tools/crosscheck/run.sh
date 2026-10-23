#!/usr/bin/env bash
# `just crosscheck`: builds Faraday's side of the crypto cross-check
# offline and runs the Python side with no network (F79,
# docs/MULTIVENDOR.md §10.4). Arguments go to crosscheck.py:
#
#   tools/crosscheck/run.sh                 the stamp run
#   tools/crosscheck/run.sh --fresh         before a release
#   tools/crosscheck/run.sh --area musig    one area, no stamp
set -euo pipefail
cd "$(dirname "$0")/../.."

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
    echo "crosscheck: the Rust command did not build" >&2
    exit 2
fi

# A network namespace of its own, with loopback only and that down:
# whatever tries to fetch fails instead of downloading.
if ! unshare --user --map-root-user --net true 2>/dev/null; then
    echo "crosscheck: this machine does not let a user make a network namespace" >&2
    echo "(unshare --user --net); see tools/crosscheck/README.md" >&2
    exit 2
fi
exec unshare --user --map-root-user --net \
    python3 -I -B tools/crosscheck/crosscheck.py --rust "$exe" "$@"
