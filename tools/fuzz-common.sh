# Shared by tools/fuzz-release.sh and tools/fuzz-corpus-export.sh (sourced,
# not run): which nightly the fuzz crates build with, the libFuzzer options
# of each target, and where a machine keeps the corpus its runs have grown.
#
# The nightly is pinned in `fuzz/rust-toolchain.toml` and
# `faraday/fuzz/rust-toolchain.toml` (the two must agree), so a release's
# fuzz run is reproduced from its commit. `cargo fuzz` is told the
# toolchain by name from the repository root; the files make a build
# started inside either directory use the same one.

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

pinned() {
    sed -n 's/^channel *= *"\(.*\)"$/\1/p' "$1/rust-toolchain.toml"
}
nightly="$(pinned fuzz)"
if [ -z "$nightly" ] || [ "$nightly" != "$(pinned faraday/fuzz)" ]; then
    echo "fuzz: fuzz/rust-toolchain.toml and faraday/fuzz/rust-toolchain.toml must name one nightly" >&2
    exit 2
fi
if ! rustup toolchain list | grep -q "^$nightly-"; then
    echo "fuzz: the pinned nightly is not installed: rustup toolchain install $nightly --profile minimal" >&2
    exit 2
fi
export RUSTUP_TOOLCHAIN="$nightly"

store="${FARADAY_FUZZ_STORE:-$HOME/.cache/faraday-fuzz}"
cores="${FARADAY_FUZZ_JOBS:-$(nproc)}"
host="$(rustc -vV | sed -n 's/^host: //p')"

# The libFuzzer options of a target beyond the common ones.
options() {
    case "$1" in
        # Any file of a stick, whole: up to a megabyte, and a slow one is
        # a finding (the 18 MiB read limit is the real bound).
        classify_file) echo "-max_len=1048576 -timeout=10" ;;
        read_wallet | json | policy) echo "-max_len=1048576 -timeout=30" ;;
        inbox_found | app_scanned | qr_assembler) echo "-max_len=262144 -timeout=60" ;;
        png_qr | pe_image | vault_header | vault_entries) echo "-max_len=262144 -timeout=30" ;;
        *) echo "-timeout=30" ;;
    esac
}

# A target's dictionary, when its directory has one (`dict/<target>.dict`):
# the words of the format it reads, which libFuzzer splices into inputs.
dictionary() {
    [ -f "$1/dict/$2.dict" ] && echo "-dict=$1/dict/$2.dict"
    return 0
}

# Builds both fuzz crates and prints "DIR TARGET" for each target. The
# lock files are committed: a lock that is out of date fails here
# (`cargo fuzz` has no --locked of its own), and a build then changes none.
build_all() {
    for dir in fuzz faraday/fuzz; do
        cargo metadata --locked --format-version 1 --manifest-path "$dir/Cargo.toml" >/dev/null
        cargo fuzz build --fuzz-dir "$dir" >&2
        cargo fuzz list --fuzz-dir "$dir" | while read -r name; do
            [ -n "$name" ] && echo "$dir $name"
        done
    done
}

# The "DIR TARGET" lines of the file $1 in the partitions
# `FARADAY_FUZZ_PARTITION` names, or all of them when it is unset. As
# cargo-nextest's `--partition count:M/N`: the targets sorted by name
# (bytewise), the i-th (from 0) is in partition i mod N + 1. `M/N` takes
# partition M of N, and `M1,M2,.../N` several of them. A run split into partitions is joined
# by `tools/fuzz-join.py`.
select_targets() {
    local p="${FARADAY_FUZZ_PARTITION:-}"
    if [ -z "$p" ]; then
        cat "$1"
        return 0
    fi
    if ! [[ "$p" =~ ^[0-9]+(,[0-9]+)*/[0-9]+$ ]] || ! LC_ALL=C sort -k2,2 "$1" | awk -v spec="$p" '
        BEGIN {
            split(spec, a, "/"); n = a[2] + 0; k = split(a[1], m, ",")
            if (n < 1) exit 3
            for (i = 1; i <= k; i++) {
                if (m[i] + 0 < 1 || m[i] + 0 > n || (m[i] + 0) in want) exit 3
                want[m[i] + 0] = 1
            }
        }
        ((NR - 1) % n + 1) in want'; then
        echo "fuzz: FARADAY_FUZZ_PARTITION is M/N or M1,M2,.../N, each M from 1 to N once, not $p" >&2
        return 2
    fi
}
