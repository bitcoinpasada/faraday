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
# starts from them. The store belongs to the machine the run is on;
# `tools/fuzz-corpus-export.sh` leaves it minimized. A target with a
# dictionary (`dict/<target>.dict` beside its corpus) runs with it.
#
# `FARADAY_FUZZ_CORPUS=DIR` names a folder of corpora, one folder per
# target (a checkout of the published corpus, or `out/fuzz-corpus/` from
# `tools/fuzz-corpus-export.sh`): before a target runs, what in
# `DIR/<target>/` adds coverage is merged into its kept corpus. Several
# folders, separated by `:`, are merged in turn (the last release's corpus
# and an older one).
#
# `FARADAY_FUZZ_PARTITION=M/N` (or `M1,M2,.../N`) runs only the targets of
# those partitions (`tools/fuzz-common.sh`), so a long run can be split, for
# example across CI jobs or computers: each part runs with the same SECONDS
# and starting corpus, and `tools/fuzz-join.py` makes the one record and
# corpus of the parts. A part's record names its partitions.
#
# A full run (every target, no partition) ends by making the coverage map
# (F241): an instrumented build of every target run over the corpus the
# run kept, then, for each target, the repository's files in which that
# corpus executes code, in `out/fuzz-map.txt`, naming the commit. A run
# split into partitions makes it after the join, from the joined corpus:
#
#   tools/fuzz-release.sh map [CORPUS]     (CORPUS a `fuzz-corpus/` folder;
#                                           the store when it is left out)
#
# A later run of the same release (F241) takes the earlier full run's
# folder, one holding its `fuzz-release.txt`, `fuzz-map.txt` and
# `fuzz-corpus/`: `FARADAY_FUZZ_EARLIER=DIR` runs only the targets
# `tools/fuzz-plan.py` says the changes since the earlier run's commit
# reach (it starts from `DIR/fuzz-corpus` unless FARADAY_FUZZ_CORPUS names
# another), and `tools/fuzz-join.py --later` joins the two records.
# `FARADAY_FUZZ_TARGETS="a b"` runs only the named targets, the way a list
# worked out elsewhere is given; the record then has a `Selected` line.
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

mode=run
if [ "${1:-}" = map ]; then
    mode=map
    shift
    seconds=0
else
    seconds="${1:-600}"
fi
case "$seconds" in
    '' | *[!0-9]*) echo "fuzz-release: SECONDS must be a number" >&2; exit 2 ;;
esac

# shellcheck source=tools/fuzz-common.sh
. "$(dirname "${BASH_SOURCE[0]}")/fuzz-common.sh"

# One target's part of the map: its instrumented binary built and run over
# its corpus (the folder $3/NAME when $3 is not empty, else the store's, and
# the target's seeds) by `cargo fuzz coverage`, then what it covered
# exported to `$4/NAME.json`, with the binary's dep-info beside it as
# `NAME.d`. Costs the build of the instrumented binary, a run over the
# corpus and an export.
map_target() {
    local dir="$1" name="$2" from="$3" mapwork="$4" dirs=() bin="" tgt
    if [ -n "$from" ]; then
        [ -d "$from/$name" ] && dirs+=("$from/$name")
    else
        [ -d "$store/$name" ] && dirs+=("$store/$name")
    fi
    [ -d "$dir/corpus/$name" ] && dirs+=("$dir/corpus/$name")
    cargo fuzz coverage --fuzz-dir "$dir" "$name" "${dirs[@]}" >"$mapwork/$name.log" 2>&1 \
        || { echo "fuzz-release: the coverage run of $name failed (out/fuzz-map/$name.log)" >&2; return 1; }
    for tgt in "$dir/target" "$root/target"; do
        if [ -x "$tgt/$host/coverage/$host/release/$name" ]; then
            bin="$tgt/$host/coverage/$host/release/$name"
            break
        fi
    done
    [ -n "$bin" ] || { echo "fuzz-release: no coverage binary for $name" >&2; return 1; }
    cp "$bin.d" "$mapwork/$name.d" 2>/dev/null || cp "$dir/target/$host/release/$name.d" "$mapwork/$name.d"
    "$llvmcov" export -summary-only -instr-profile="$dir/coverage/$name/coverage.profdata" "$bin" \
        >"$mapwork/$name.json" || { echo "fuzz-release: llvm-cov failed for $name" >&2; return 1; }
}

# The coverage map of the corpus in folder $1 (a `fuzz-corpus/`, one folder
# a target), or of the store when $1 is empty, at the checked-out commit:
# `out/fuzz-map.txt` (F241). Costs an instrumented build of every target, a
# run of each over its corpus and an export, as many targets at once as the
# machine has cores.
map_step() {
    local from="$1" out="${FARADAY_FUZZ_MAP_FILE:-$root/out/fuzz-map.txt}"
    local mapwork="$root/out/fuzz-map" dir name pid failed=0
    llvmcov="$(rustc --print sysroot)/lib/rustlib/$host/bin/llvm-cov"
    if [ ! -x "$llvmcov" ]; then
        echo "fuzz-release: the map needs llvm-cov: rustup component add llvm-tools-preview --toolchain $nightly" >&2
        return 2
    fi
    if ! git diff --quiet HEAD -- .; then
        echo "fuzz-release: the map is of a commit: the checkout has uncommitted changes" >&2
        return 2
    fi
    rm -rf "$mapwork"
    mkdir -p "$mapwork" "$(dirname "$out")"
    echo "fuzz-release: the coverage map"
    for dir in fuzz faraday/fuzz; do
        cargo metadata --locked --format-version 1 --manifest-path "$dir/Cargo.toml" >/dev/null
    done
    while read -r dir name; do
        map_target "$dir" "$name" "$from" "$mapwork" &
        if [ "$(jobs -r | wc -l)" -ge "$cores" ]; then
            wait -n || failed=1
        fi
    done < <(FARADAY_FUZZ_PREBUILT=1 build_all)
    for pid in $(jobs -p); do
        wait "$pid" || failed=1
    done
    [ "$failed" = 0 ] || return 1
    {
        echo "Commit: $(git rev-parse HEAD)"
        echo "Toolchain: $(rustc -V)"
    } >"$mapwork/header"
    python3 -I -B "$root/tools/fuzz-plan.py" map "$mapwork" "$out"
    echo "fuzz-release: wrote $out"
}

if [ "$mode" = map ]; then
    map_step "${1:-}"
    exit $?
fi

record="${FARADAY_FUZZ_RECORD:-$root/out/fuzz-release.txt}"
work="$root/out/fuzz-release"
earlier="${FARADAY_FUZZ_EARLIER:-}"
selected="${FARADAY_FUZZ_TARGETS:-}"
start_corpus="${FARADAY_FUZZ_CORPUS:-}"
if [ -n "$earlier" ]; then
    if [ -n "${FARADAY_FUZZ_PARTITION:-}" ]; then
        echo "fuzz-release: FARADAY_FUZZ_EARLIER and FARADAY_FUZZ_PARTITION are not used together" >&2
        exit 2
    fi
    [ -n "$start_corpus" ] || start_corpus="$earlier/fuzz-corpus"
fi
if [ -z "$earlier" ] && [ -z "${FARADAY_FUZZ_PARTITION:-}" ] && [ -z "$selected" ]; then
    # A full run ends with the map: check what it needs before the hour.
    if [ ! -x "$(rustc --print sysroot)/lib/rustlib/$host/bin/llvm-cov" ]; then
        echo "fuzz-release: the run's map needs llvm-cov: rustup component add llvm-tools-preview --toolchain $nightly" >&2
        exit 2
    fi
fi
start_dirs=()
[ -n "$start_corpus" ] && IFS=':' read -r -a start_dirs <<<"$start_corpus"
for d in "${start_dirs[@]}"; do
    if [ ! -d "$d" ]; then
        echo "fuzz-release: FARADAY_FUZZ_CORPUS names $d, which is not a folder" >&2
        exit 2
    fi
done

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
    from=()
    [ -n "$start_corpus" ] && IFS=':' read -r -a from <<<"$start_corpus"
    for d in "${from[@]}"; do
        [ -d "$d/$name" ] || continue
        # Only what adds coverage to the kept corpus comes across.
        # shellcheck disable=SC2046
        "$bin" -merge=1 -rss_limit_mb=2048 $(options "$name") \
            "$store/$name" "$d/$name" >>"$work/$name.log" 2>&1 || true
    done
    start="$(date +%s)"
    set +e
    # shellcheck disable=SC2046
    "$bin" "${corpus[@]}" -max_total_time="$seconds" -rss_limit_mb=2048 \
        -artifact_prefix="$dir/artifacts/$name/" $(options "$name") $(dictionary "$dir" "$name") \
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
export -f run_one options dictionary
export seconds store work host start_corpus

echo "fuzz-release: building every target with $nightly"
list="$(mktemp)"
build_all >"$list"
all_targets="$(wc -l <"$list")"
if [ -n "$earlier" ]; then
    selected="$(python3 -I -B "$root/tools/fuzz-plan.py" plan "$earlier" "$list" | tr '\n' ' ')" \
        || { rm -f "$list"; exit 2; }
    if [ "$(wc -w <<<"$selected")" -eq "$all_targets" ]; then
        selected=""
        earlier=""    # every target runs: a full run, with its map
    elif [ -z "${selected// /}" ]; then
        selected="none"
    fi
fi
if [ -n "$selected" ]; then
    # Only the named targets run ("none": no target).
    names=" ${selected//,/ } "
    for n in $names; do
        [ "$n" = none ] || grep -q " $n\$" "$list" || { echo "fuzz-release: no target named $n" >&2; rm -f "$list"; exit 2; }
    done
    FARADAY_FUZZ_TARGETS="$selected" only_named "$list"
fi
total="$(wc -l <"$list")"
select_targets "$list" >"$list.picked" || { rm -f "$list" "$list.picked"; exit 2; }
mapfile -t jobs_list <"$list.picked"
rm -f "$list" "$list.picked"

rm -rf "$work"
mkdir -p "$work"
echo "fuzz-release: ${#jobs_list[@]} targets, $seconds s each, $cores at once"
if [ "${#jobs_list[@]}" -gt 0 ]; then
    printf '%s\n' "${jobs_list[@]}" \
        | xargs -P "$cores" -L1 bash -c 'run_one "$@"' _
fi
results() { find "$work" -name '*.result' -exec cat {} +; }

mkdir -p "$(dirname "$record")"
commit="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
if [ -z "$start_corpus" ]; then
    started="none (the committed seeds and what the store holds)"
else
    started=""
    for d in "${start_dirs[@]}"; do
        one="$(basename "$d")"
        if c="$(git -C "$d" rev-parse --short HEAD 2>/dev/null)"; then
            one="$one, commit $c"
        fi
        started="${started:+$started; }$one"
    done
fi
{
    echo "Fuzz run before a release"
    echo "Date: $(date -u +%Y-%m-%d)"
    echo "Commit: $commit"
    echo "Toolchain: $(rustc -V)"
    echo "cargo-fuzz: $(cargo fuzz --version | awk '{ print $2 }')"
    echo "Starting corpus: $started"
    echo "Time per target: $seconds s"
    if [ -n "$selected" ]; then
        echo "Selected: $total of $all_targets targets"
    fi
    if [ -n "${FARADAY_FUZZ_PARTITION:-}" ]; then
        echo "Partition: ${FARADAY_FUZZ_PARTITION%/*} of ${FARADAY_FUZZ_PARTITION#*/}, ${#jobs_list[@]} of $total targets"
    fi
    echo
    printf '%-16s %8s %12s %9s %12s  %s\n' target seconds executions corpus bytes result
    results | sort | while IFS=$'\t' read -r name elapsed runs entries bytes result; do
        printf '%-16s %8s %12s %9s %12s  %s\n' "$name" "$elapsed" "$runs" "$entries" "$bytes" "$result"
    done
    echo
} >"$record"

crashed="$(results | grep -c 'CRASH' || true)"
if [ "$crashed" -ne 0 ]; then
    echo "Result: $crashed target(s) crashed" >>"$record"
    cat "$record"
    exit 1
fi
echo "Result: ${#jobs_list[@]} targets, no crashes" >>"$record"
if [ -z "$selected" ] && [ -z "$earlier" ] && [ -z "${FARADAY_FUZZ_PARTITION:-}" ]; then
    map_step "" || { cat "$record"; exit 1; }
fi
cat "$record"
