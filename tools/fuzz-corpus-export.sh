#!/usr/bin/env bash
# Each target's corpus, minimized, into `out/fuzz-corpus/<target>/`: what a
# machine's runs have kept (`tools/fuzz-release.sh`'s store) together with
# the committed seeds, reduced by `cargo fuzz cmin` to the inputs that
# between them reach everything the whole set reaches. The folder is ready
# to commit to the repository that publishes the corpora after a release
# (`tools/README.md`, "Before a release"), and the store keeps the same
# minimized set in place of what it held.
#
#   tools/fuzz-corpus-export.sh        (`just fuzz-corpus-export`)
#
# Run it on the machine whose store the run grew, after `just fuzz-release`.
# `FARADAY_FUZZ_JOBS` is how many targets are minimized at once, and
# `FARADAY_FUZZ_PARTITION` which, as for that run: a run split into
# partitions exports each part where it ran.

set -euo pipefail

# shellcheck source=tools/fuzz-common.sh
. "$(dirname "${BASH_SOURCE[0]}")/fuzz-common.sh"

out="$root/out/fuzz-corpus"

export_one() {
    dir="$1"
    name="$2"
    to="$out/$name"
    mkdir -p "$to" "$dir/artifacts/$name"
    [ -d "$store/$name" ] && find "$store/$name" -type f -exec cp --update=none {} "$to/" \;
    [ -d "$dir/corpus/$name" ] && find "$dir/corpus/$name" -type f -exec cp --update=none {} "$to/" \;
    # An input that crashes is written beside the run's own
    # (fuzz/artifacts/<target>/), never into the working directory.
    # shellcheck disable=SC2046
    cargo fuzz cmin --fuzz-dir "$dir" "$name" "$to" -- -rss_limit_mb=2048 \
        -artifact_prefix="$dir/artifacts/$name/" $(options "$name") \
        >"$out/$name.log" 2>&1 || {
        echo "fuzz-corpus-export: $name failed, see out/fuzz-corpus/$name.log" >&2
        return 1
    }
    rm -f "$out/$name.log"
    # The store keeps the minimized set in place of what it held: it
    # reaches what the store reached, and a slow target, which runs every
    # kept input before it fuzzes, loads it in about a third of the time
    # (app_scanned's 2,027 inputs took 77 s, its 1,124 after this 28 s).
    if [ -d "$store/$name" ]; then
        rm -rf "$store/$name.min"
        cp -r "$to" "$store/$name.min"
        rm -rf "$store/$name"
        mv "$store/$name.min" "$store/$name"
    fi
    echo "fuzz-corpus-export: $name: $(find "$to" -type f | wc -l) inputs"
}
export -f export_one options
export store out

echo "fuzz-corpus-export: building every target with $nightly"
list="$(mktemp)"
build_all >"$list"
select_targets "$list" >"$list.picked" || { rm -f "$list" "$list.picked"; exit 2; }
mapfile -t jobs_list <"$list.picked"
rm -f "$list" "$list.picked"
rm -rf "$out"
mkdir -p "$out"
printf '%s\n' "${jobs_list[@]}" | xargs -P "$cores" -L1 bash -c 'export_one "$@"' _
echo "fuzz-corpus-export: ${#jobs_list[@]} targets in out/fuzz-corpus/"
