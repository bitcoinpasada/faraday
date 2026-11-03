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
# Every outside tool the run needs, before anything is built; a missing one
# stops the run and is named (tools/crosscheck/judges.py).
if ! gone=$(python3 -I -B tools/crosscheck/judges.py check 2>&1); then
    echo "$gone" | while read -r line; do say "$line"; done
    exit 2
fi
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
# btcd's BIP-322 over tools/reference/btcd-bip322/ and a released btcd's
# hdkeychain and control-block check over tools/reference/btcd/ (offline: the
# vendor folders only, the installed toolchain only). A missing or older Go stops
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
say "build: btcd's answer program (go build -mod=vendor, tools/reference/btcd)"
if ! GOFLAGS=-mod=vendor GOTOOLCHAIN=local GOPROXY=off GOTELEMETRY=off CGO_ENABLED=0 \
        go build -C tools/reference/btcd -trimpath -o "$PWD/$progress/btcd-ref" . \
        2>"$progress/go-build-btcd.log"; then
    cat "$progress/go-build-btcd.log" >&2
    say "build: FAILED: btcd's answer program did not build (the Go version go.mod names)"
    exit 2
fi
say "build: btcd's answer program done"

# The second pass of the aezeed (area lightning): Faraday's command built
# with the aes crate's software round (--cfg aes_backend="soft", for this
# build only, in a target folder of its own) and LND's program built without
# assembly (tags noasm, purego: Yawning Angel's AEZ then runs its bitsliced
# round). soft.py stops the run when either is not what it says.
case " $* " in
    *" --area "*|*" --area="*)
        case " $* " in *" lightning "*|*"=lightning "*) soft=1 ;; *) soft= ;; esac ;;
    *) soft=1 ;;
esac
if [ -n "$soft" ]; then
    say "build: the Rust command with the software AES round (cargo build, --cfg aes_backend=\"soft\")"
    soft_exe=$(RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }--cfg aes_backend=\"soft\"" \
        CARGO_TARGET_DIR="$PWD/$progress/soft-target" \
        cargo build --frozen -q -p faraday-crosscheck --message-format=json-render-diagnostics \
        | python3 -I -c '
import json, sys
for line in sys.stdin:
    m = json.loads(line)
    if m.get("reason") == "compiler-artifact" and m.get("executable") \
            and m["target"]["name"] == "faraday-crosscheck":
        print(m["executable"])
' | tail -n 1)
    if [ -z "$soft_exe" ]; then
        say "build: FAILED: the Rust command with the software AES round did not build"
        exit 2
    fi
    if ! GOFLAGS=-mod=vendor GOTOOLCHAIN=local GOPROXY=off GOTELEMETRY=off CGO_ENABLED=0 \
            go build -C tools/reference/lnd -trimpath -tags noasm,purego \
            -o "$PWD/$progress/lnd-ref-soft" . 2>"$progress/go-build-soft.log"; then
        cat "$progress/go-build-soft.log" >&2
        say "build: FAILED: LND's answer program without assembly did not build"
        exit 2
    fi
    if ! why=$(python3 -I -B tools/crosscheck/soft.py check "$exe" "$soft_exe" \
            "$PWD/$progress/lnd-ref" "$PWD/$progress/lnd-ref-soft" 2>&1); then
        echo "$why" | while read -r line; do say "$line"; done
        exit 2
    fi
    say "build: the software-round pass: $why"
    export FARADAY_SOFT_RUST="$soft_exe" FARADAY_LND_SOFT_REF="$PWD/$progress/lnd-ref-soft"
fi

# The judged test files and the OVMF Secure Boot run (ESTABLISHED §9.2,
# rule 4): the tests that skip without an outside tool under plain `just`
# fail without it here, and QEMU boots OVMF's Secure Boot firmware over
# faraday-sb's keys. Not for one area or the identity run.
case " $* " in
    *" --area "*|*" --area="*|*" --identity "*) ;;
    *)
        say "judges: the judged tests (FARADAY_JUDGES_REQUIRED=1: faraday-fat, faraday-files, faraday-sb, the Starter's stick tests)"
        if ! FARADAY_JUDGES_REQUIRED=1 cargo nextest run --frozen --no-fail-fast \
                -p faraday-fat -p faraday-files -p faraday-sb -p faraday-starter-shell \
                -E 'binary(judged) | binary(formatted) | binary(foreign) | (package(faraday-starter-shell) & test(/^(ids|image_finish|stick_layout)::/))' \
                >"$progress/judged-tests.log" 2>&1; then
            tail -n 40 "$progress/judged-tests.log" >&2
            say "judges: FAILED: a judged test failed or its tool is missing (out/crosscheck/judged-tests.log)"
            exit 2
        fi
        say "judges: the judged tests done"
        say "judges: Secure Boot in OVMF (faraday/tools/sb-ovmf-check.py)"
        mkdir -p "$progress/sb-ovmf"
        if ! python3 -I -B faraday/tools/sb-ovmf-check.py "$PWD/$progress/sb-ovmf" \
                >"$progress/sb-ovmf.log" 2>&1; then
            tail -n 40 "$progress/sb-ovmf.log" >&2
            say "judges: FAILED: the OVMF Secure Boot run (out/crosscheck/sb-ovmf.log)"
            exit 2
        fi
        say "judges: the OVMF run done"
        ;;
esac

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
