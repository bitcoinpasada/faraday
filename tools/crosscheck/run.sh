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
# A --fresh run also writes out/crosscheck-record.txt (record.py), which the
# release carries beside SHA256SUMS.
# The run's progress goes to out/crosscheck/ (progress.log from the build
# on, progress.json, report.txt); `just crosscheck-status` reads it.
set -euo pipefail
cd "$(dirname "$0")/../.."

progress=out/crosscheck
mkdir -p "$progress"
rm -f "$progress/progress.json" "$progress/report.txt" out/crosscheck-record.txt
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

# The Go answer programs, LND's over the vendored tree of tools/reference/lnd/
# and btcd's BIP-322 over tools/reference/btcd-bip322/ (offline: the vendor
# folders only, the installed toolchain only). A missing or older Go stops
# the run.
say "build: LND's answer program (go build -mod=vendor, tools/reference/lnd)"
if ! command -v go >/dev/null; then
    say "build: FAILED: no Go toolchain on the PATH (docs/DEVELOPING.md §1)"
    exit 2
fi
if ! GOFLAGS=-mod=vendor GOTOOLCHAIN=local GOPROXY=off GOTELEMETRY=off CGO_ENABLED=0 \
        go build -C tools/reference/lnd -trimpath -o "$PWD/$progress/lnd-ref" . \
        2>"$progress/go-build.log"; then
    cat "$progress/go-build.log" >&2
    say "build: FAILED: LND's answer program did not build (the Go version go.mod names)"
    exit 2
fi
say "build: LND's answer program done"
say "build: btcd's BIP-322 program (go build -mod=vendor, tools/reference/btcd-bip322)"
if ! GOFLAGS=-mod=vendor GOTOOLCHAIN=local GOPROXY=off GOTELEMETRY=off CGO_ENABLED=0 \
        go build -C tools/reference/btcd-bip322 -trimpath -o "$PWD/$progress/btcd-bip322-ref" . \
        2>"$progress/go-build-bip322.log"; then
    cat "$progress/go-build-bip322.log" >&2
    say "build: FAILED: btcd's BIP-322 program did not build (the Go version go.mod names)"
    exit 2
fi
say "build: btcd's BIP-322 program done"

# A network namespace of its own, with loopback only: whatever tries to
# fetch fails instead of downloading. crosscheck.py brings `lo` up for
# Bitcoin Core's RPC on loopback (core_node.py) and refuses to run if
# any other interface is there.
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
