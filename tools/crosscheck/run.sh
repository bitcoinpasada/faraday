#!/usr/bin/env bash
# `just crosscheck`: builds Faraday's side of the crypto cross-check
# offline and runs the Python side with no network (F79,
# docs/CROSSCHECK.md §10.4). Arguments go to crosscheck.py:
#
#   tools/crosscheck/run.sh                 the stamp run
#   tools/crosscheck/run.sh --fresh         before a release
#   tools/crosscheck/run.sh --combine R     a release's run made of two (F239): copy
#                                           the record and report.txt aside first,
#                                           as this clears both
#   tools/crosscheck/run.sh --area musig    one area, no stamp
#   tools/crosscheck/run.sh --jobs 4        four workers (default: one a core)
#   tools/crosscheck/run.sh --identity      the same report under 1 and 4 workers
#                                           (and PyPy when found): identity.py
#   tools/crosscheck/run.sh --sweep-core-wallet [--shapes plain,blank] [--jobs N]
#                                           the core_wallet area's sweep instead
#                                           (sweep_core_wallet.py)
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
if [ "${1:-}" = "--sweep-core-wallet" ]; then
    shift
    exec unshare --user --map-root-user --net \
        python3 -I -B tools/crosscheck/sweep_core_wallet.py --rust "$exe" "$@"
fi

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
# Core Lightning's answer program (tools/reference/cln): ccan's HKDF-SHA256 from
# its files and its hsm_secret functions over the system libsodium.
say "build: Core Lightning's answer program (cc, tools/reference/cln, libsodium)"
if ! command -v cc >/dev/null || ! command -v pkg-config >/dev/null \
        || ! pkg-config --exists libsodium; then
    say "build: FAILED: no C compiler, pkg-config or libsodium's headers (docs/DEVELOPING.md §1)"
    exit 2
fi
cln=tools/reference/cln
# shellcheck disable=SC2046
if ! cc -O2 -Wall -Werror -I "$cln" -I "$cln/ccan" \
        -o "$progress/cln-ref" "$cln/main.c" \
        "$cln"/ccan/ccan/crypto/sha256/sha256.c \
        "$cln"/ccan/ccan/crypto/hmac_sha256/hmac_sha256.c \
        "$cln"/ccan/ccan/crypto/hkdf_sha256/hkdf_sha256.c \
        $(pkg-config --cflags --libs libsodium) 2>"$progress/cc-cln.log"; then
    cat "$progress/cc-cln.log" >&2
    say "build: FAILED: Core Lightning's answer program did not build"
    exit 2
fi
say "build: Core Lightning's answer program done"
# Core Lightning's offer decoder (area lightning, tools/reference/bolt12): its
# own devtools/bolt12-cli, built from the pinned release archive by its own
# configure and make (the packages it needs are in docs/DEVELOPING.md §1; the
# archive is fetched once by fetch_cln.py).
say "build: Core Lightning's offer decoder (tools/crosscheck/build_bolt12_cli.sh)"
if ! tools/crosscheck/build_bolt12_cli.sh "$progress" 2>"$progress/bolt12-build.log"; then
    cat "$progress/bolt12-build.log" >&2
    say "build: FAILED: Core Lightning's offer decoder did not build (docs/DEVELOPING.md §1)"
    exit 2
fi
say "build: Core Lightning's offer decoder done"
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

# keepass-rs as the vault file's outside reader and writer (area vault;
# F193 point 1): its own workspace, tools/crosscheck/keepass-rs/, built
# here and never reaching an app.
say "build: keepass-rs's program (cargo build --frozen, tools/crosscheck/keepass-rs)"
if ! cargo build --frozen -q --release --manifest-path tools/crosscheck/keepass-rs/Cargo.toml \
        --target-dir "$PWD/$progress/keepass-rs-target" 2>"$progress/keepass-rs-build.log"; then
    cat "$progress/keepass-rs-build.log" >&2
    say "build: FAILED: keepass-rs's program did not build"
    exit 2
fi
say "build: keepass-rs's program done"

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

# The Rust answer programs of the codex32 area, built with --frozen from the
# Cargo.lock of their own (outside the workspace, never shipped): rust-codex32's over
# tools/reference/rust-codex32/ and volvelle-website's checksum worksheet over
# tools/vectors/codex32-volvelle/harness/.
for prog in "rust-codex32-ref tools/reference/rust-codex32 rust-codex32-answers" \
        "volvelle-ref tools/vectors/codex32-volvelle/harness volvelle-cells"; do
    read -r name dir bin <<<"$prog"
    say "build: $name (cargo build --frozen --manifest-path $dir/Cargo.toml)"
    if ! CARGO_TARGET_DIR="$PWD/$progress/rust-target-$name" cargo build --frozen -q \
            --manifest-path "$dir/Cargo.toml" 2>"$progress/cargo-$name.log"; then
        cat "$progress/cargo-$name.log" >&2
        say "build: FAILED: $name did not build"
        exit 2
    fi
    cp "$progress/rust-target-$name/debug/$bin" "$progress/$name"
    say "build: $name done"
done

# rPGP as the QR vault's outside sealer and reader (area qrvault; F193
# point 1): its own workspace, tools/crosscheck/rpgp/, built here from its
# Cargo.lock and never reaching an app. Its crates are fetched beforehand
# (cargo fetch --locked --manifest-path tools/crosscheck/rpgp/Cargo.toml).
say "build: rPGP's program (cargo build --frozen, tools/crosscheck/rpgp)"
if ! CARGO_TARGET_DIR="$PWD/$progress/rust-target-rpgp-ref" cargo build --frozen -q \
        --manifest-path tools/crosscheck/rpgp/Cargo.toml 2>"$progress/cargo-rpgp-ref.log"; then
    cat "$progress/cargo-rpgp-ref.log" >&2
    say "build: FAILED: rPGP's program did not build (fetch its crates first: docs/DEVELOPING.md)"
    exit 2
fi
cp "$progress/rust-target-rpgp-ref/debug/faraday-crosscheck-rpgp" "$progress/rpgp-ref"
say "build: rPGP's program done"

# The judged test files and the OVMF Secure Boot run (ESTABLISHED §9.2,
# rule 4): the tests that skip without an outside tool under plain `just`
# fail without it here, and QEMU boots OVMF's Secure Boot firmware over
# faraday-sb's keys. Not for one area or the identity run. The judged tests
# run first; the OVMF run starts beside the areas (below) and the run waits
# for it before it ends.
ovmf=
case " $* " in
    *" --area "*|*" --area="*|*" --identity "*) ;;
    *)
        say "judges: the judged tests (FARADAY_JUDGES_REQUIRED=1: faraday-fat, faraday-files, faraday-sb, the Starter's stick tests)"
        if ! FARADAY_JUDGES_REQUIRED=1 cargo nextest run --frozen --no-fail-fast \
                -p faraday-fat -p faraday-files -p faraday-sb -p faraday-starter-shell \
                -E 'binary(judged) | binary(formatted) | binary(foreign) | (package(faraday-starter-shell) & test(/^(ids|image_finish|outside|stick_layout)::/))' \
                >"$progress/judged-tests.log" 2>&1; then
            tail -n 40 "$progress/judged-tests.log" >&2
            say "judges: FAILED: a judged test failed or its tool is missing (out/crosscheck/judged-tests.log)"
            exit 2
        fi
        say "judges: the judged tests done"
        ovmf=1
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
# A signal to this script ends what it started: the OVMF run has a process group
# of its own, so its QEMU goes with it, and crosscheck.py ends its own workers.
ovmf_pid=
check_pid=
end_started() {
    trap - INT TERM HUP
    [ -z "$ovmf_pid" ] || kill -TERM -- "-$ovmf_pid" 2>/dev/null || true
    [ -z "$check_pid" ] || kill -TERM "$check_pid" 2>/dev/null || true
    wait 2>/dev/null || true
    say "crosscheck: stopped by a signal"
    exit "$1"
}
trap 'end_started 129' HUP
trap 'end_started 130' INT
trap 'end_started 143' TERM
if [ -n "$ovmf" ]; then
    say "judges: Secure Boot in OVMF (faraday/tools/sb-ovmf-check.py), beside the areas"
    mkdir -p "$progress/sb-ovmf"
    setsid python3 -I -B faraday/tools/sb-ovmf-check.py "$PWD/$progress/sb-ovmf" \
        >"$progress/sb-ovmf.log" 2>&1 &
    ovmf_pid=$!
fi
unshare --user --map-root-user --net \
    python3 -I -B tools/crosscheck/crosscheck.py --rust "$exe" --progress "$progress" "$@" &
check_pid=$!
if [ -n "$ovmf_pid" ]; then
    ovmf_rc=0
    wait "$ovmf_pid" || ovmf_rc=$?
    if [ "$ovmf_rc" -ne 0 ]; then
        kill "$check_pid" 2>/dev/null || true
        wait "$check_pid" 2>/dev/null || true
        # The areas may have agreed and written the record: this run did not.
        if [ -f out/crosscheck-record.txt ]; then
            sed -i 's/^result: every answer agreed$/result: the OVMF Secure Boot run failed/' \
                out/crosscheck-record.txt
        fi
        tail -n 40 "$progress/sb-ovmf.log" >&2
        say "judges: FAILED: the OVMF Secure Boot run (out/crosscheck/sb-ovmf.log)"
        exit 2
    fi
    say "judges: the OVMF run done"
fi
check_rc=0
wait "$check_pid" || check_rc=$?
exit "$check_rc"
