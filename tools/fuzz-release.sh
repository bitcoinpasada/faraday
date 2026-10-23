#!/usr/bin/env bash
# The fuzz run before a release: every target of `fuzz/` and
# `faraday/fuzz/`, each for SECONDS (600 by default), as many at once as
# the machine has cores. Any crash, out-of-memory or timeout fails the
# run (exit 1) and names the input.
#
#   tools/fuzz-release.sh [SECONDS]        (`just fuzz-release [SECONDS]`)
#
# Each target keeps its corpus between runs in a store outside the
# checkout, `$FARADAY_FUZZ_STORE` (default `~/.cache/faraday-fuzz`), one
# folder per target: a checkout is replaced when it is synced, and the
# corpus a run has grown is worth more than the seeds. The committed seeds
# are read from `corpus/` beside the target as well, so a new target
# starts from them. The store belongs to the machine the run is on.
#
# `FARADAY_FUZZ_CORPUS=DIR` names a folder of corpora, one folder per
# target (a checkout of the published corpus, or `out/fuzz-corpus/` from
# `tools/fuzz-corpus-export.sh`): before a target runs, what in
# `DIR/<target>/` adds coverage is merged into its kept corpus.
#
# It writes `out/fuzz-release.txt`: the date, the commit, the nightly and
# cargo-fuzz it used, the corpus it started from, and for each target its
# seconds, executions, corpus size and result. The release notes quote it.
# Nothing in it names a path or a machine.
#
# The nightly is the one `fuzz/rust-toolchain.toml` pins and the lock
# files are committed, so the run is reproduced from the release's commit:
# check it out, install that nightly and the recorded cargo-fuzz version,
# and run this again with the release's corpus in FARADAY_FUZZ_CORPUS.
#
# A hostile input can be large or slow, so the files a stick holds are
# fuzzed with a larger input limit and a short timeout (`options` in
# `tools/fuzz-common.sh`), and every run has a 2 GiB memory limit.

set -euo pipefail

seconds="${1:-600}"
case "$seconds" in
    '' | *[!0-9]*) echo "fuzz-release: SECONDS must be a number" >&2; exit 2 ;;
esac

# shellcheck source=tools/fuzz-common.sh
. "$(dirname "${BASH_SOURCE[0]}")/fuzz-common.sh"

record="${FARADAY_FUZZ_RECORD:-$root/out/fuzz-release.txt}"
work="$root/out/fuzz-release"
start_corpus="${FARADAY_FUZZ_CORPUS:-}"
if [ -n "$start_corpus" ] && [ ! -d "$start_corpus" ]; then
    echo "fuzz-release: FARADAY_FUZZ_CORPUS is not a folder" >&2
    exit 2
fi

# One target: its binary is already built.
run_one() {
    dir="$1"
    name="$2"
    bin="$dir/target/$host/release/$name"
    seeds="$dir/corpus/$name"
    mkdir -p "$store/$name" "$dir/artifacts/$name" "$work"
    corpus=("$store/$name")
    [ -d "$seeds" ] && corpus+=("$seeds")
    : >"$work/$name.log"
    if [ -n "$start_corpus" ] && [ -d "$start_corpus/$name" ]; then
        # Only what adds coverage to the kept corpus comes across.
        # shellcheck disable=SC2046
        "$bin" -merge=1 -rss_limit_mb=2048 $(options "$name") \
            "$store/$name" "$start_corpus/$name" >>"$work/$name.log" 2>&1 || true
    fi
    start="$(date +%s)"
    set +e
    # shellcheck disable=SC2046
    "$bin" "${corpus[@]}" -max_total_time="$seconds" -rss_limit_mb=2048 \
        -artifact_prefix="$dir/artifacts/$name/" $(options "$name") \
        >>"$work/$name.log" 2>&1
    status=$?
    set -e
    elapsed=$(($(date +%s) - start))
    runs="$(sed -n 's/^Done \([0-9]*\) runs.*/\1/p' "$work/$name.log" | tail -1)"
    entries="$(find "$store/$name" -type f | wc -l)"
    bytes="$(find "$store/$name" -type f -printf '%s\n' | awk '{ n += $1 } END { print n + 0 }')"
    if [ "$status" -eq 0 ]; then
        result="no crashes"
    else
        found="$(sed -n 's/^artifact_prefix=.*Test unit written to //p' "$work/$name.log" | tail -1)"
        result="CRASH (exit $status) ${found:-see $work/$name.log}"
    fi
    printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$name" "$elapsed" "${runs:-0}" "$entries" "$bytes" "$result" >"$work/$name.result"
}
export -f run_one options
export seconds store work host start_corpus

echo "fuzz-release: building every target with $nightly"
list="$(mktemp)"
build_all >"$list"
mapfile -t jobs_list <"$list"
rm -f "$list"

rm -rf "$work"
mkdir -p "$work"
echo "fuzz-release: ${#jobs_list[@]} targets, $seconds s each, $cores at once"
printf '%s\n' "${jobs_list[@]}" \
    | xargs -P "$cores" -L1 bash -c 'run_one "$@"' _

mkdir -p "$(dirname "$record")"
commit="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
if [ -z "$start_corpus" ]; then
    started="none (the committed seeds and what the store holds)"
else
    started="$(basename "$start_corpus")"
    if c="$(git -C "$start_corpus" rev-parse --short HEAD 2>/dev/null)"; then
        started="$started, commit $c"
    fi
fi
{
    echo "Fuzz run before a release"
    echo "Date: $(date -u +%Y-%m-%d)"
    echo "Commit: $commit"
    echo "Toolchain: $(rustc -V)"
    echo "cargo-fuzz: $(cargo fuzz --version | awk '{ print $2 }')"
    echo "Starting corpus: $started"
    echo "Time per target: $seconds s"
    echo
    printf '%-16s %8s %12s %9s %12s  %s\n' target seconds executions corpus bytes result
    cat "$work"/*.result | sort | while IFS=$'\t' read -r name elapsed runs entries bytes result; do
        printf '%-16s %8s %12s %9s %12s  %s\n' "$name" "$elapsed" "$runs" "$entries" "$bytes" "$result"
    done
    echo
} >"$record"

crashed="$(cat "$work"/*.result | grep -c 'CRASH' || true)"
if [ "$crashed" -ne 0 ]; then
    echo "Result: $crashed target(s) crashed" >>"$record"
    cat "$record"
    exit 1
fi
echo "Result: ${#jobs_list[@]} targets, no crashes" >>"$record"
cat "$record"
