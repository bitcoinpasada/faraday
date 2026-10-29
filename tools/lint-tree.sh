#!/usr/bin/env bash
# Tree lint: the repository holds what it says it holds, and nothing left
# behind by an editor, a merge or a session. Four checks, over every file
# git tracks or would track (tracked, or untracked and not ignored):
#
# 1. The top level holds only the entries in TOP below. A new top-level
#    file or folder is added to TOP, with what it is, in the change that
#    adds it, and README.md's "Repository layout" names it.
# 2. No leftovers anywhere: an editor's or a merge's backup (`*.orig`,
#    `*.rej`, `*.bak`, `*.swp`, `*~`), an operating system's folder files
#    (`.DS_Store`, `Thumbs.db`, `desktop.ini`), a log or temporary file
#    (`*.log`, `*.tmp`), or a copy kept "before" a change (`*.before-*`,
#    `* (copy)*`, `*.old`). History is git's.
# 3. No file over 2 MiB: a larger one is build output, or data that
#    belongs in a release or a repository of its own.
#
# 4. No metadata in an image: every tracked png, jpg, jpeg, webp or svg
#    passes `tools/guide-pictures.py --check` (a PNG's text, EXIF, ICC and
#    time chunks and their kin in the other formats). `--strip` removes it.
#
# Run from the repository root; `just lint` does.
set -u
cd "$(dirname "$0")/.." || exit 2

# What each top-level entry is.
TOP=(
    .claude              # the agent definitions a change may use
    .github              # issue templates; no workflows (no hosted CI)
    .gitignore
    CLAUDE.md            # the rules every change follows
    CONTRIBUTING.md      # how to report and how to send a change
    Cargo.lock
    Cargo.toml
    LICENSE
    README.md
    SECURITY.md
    core                 # the libraries with no flow and no wording
    deny.toml            # cargo deny's policy
    design               # the mark's drafts
    docs                 # what is true now or still to be built
    faraday              # Faraday itself
    fuzz                 # the shared crates' fuzz targets
    justfile
    rust-toolchain.toml
    supply-chain         # cargo vet's audits and exemptions
    third_party          # vendored crates, each with its patches
    tools                # lints, references, vectors and scripts
)
MAX_BYTES=$((2 * 1024 * 1024))

status=0
mapfile -d '' FILES < <(git ls-files -z --cached --others --exclude-standard)

declare -A allowed=()
for t in "${TOP[@]}"; do allowed[$t]=1; done
declare -A reported=()
for f in "${FILES[@]}"; do
    top="${f%%/*}"
    if [ -z "${allowed[$top]:-}" ] && [ -z "${reported[$top]:-}" ]; then
        reported[$top]=1
        echo "lint-tree: $top is not one of the top level's entries; put it in a folder, or add it to TOP in tools/lint-tree.sh and README.md's \"Repository layout\""
        status=1
    fi
done

regular=()
for f in "${FILES[@]}"; do
    base="${f##*/}"
    case "$base" in
        *.orig | *.rej | *.bak | *.swp | *~ | .DS_Store | Thumbs.db | desktop.ini | *.log | *.tmp | *.before-* | *" (copy)"* | *.old)
            echo "lint-tree: $f is a leftover; remove it (git keeps the history)"
            status=1
            ;;
    esac
    if [ -f "$f" ] && [ ! -L "$f" ]; then
        regular+=("$f")
    fi
done

# Every size in one stat: a process per file takes seconds on a slow fork.
if [ "${#regular[@]}" -gt 0 ]; then
    while IFS=$'\t' read -r -d '' size f; do
        if [ "$size" -gt "$MAX_BYTES" ]; then
            echo "lint-tree: $f is $size bytes, over 2 MiB; build output stays out of the repository"
            status=1
        fi
    done < <(printf '%s\0' "${regular[@]}" | xargs -0 stat --printf '%s\t%n\0' --)
fi

images=()
for f in "${FILES[@]}"; do
    case "${f,,}" in
        *.png | *.jpg | *.jpeg | *.webp | *.svg) [ -f "$f" ] && images+=("$f") ;;
    esac
done
if [ "${#images[@]}" -gt 0 ] \
    && ! python3 -I -B tools/guide-pictures.py --check "${images[@]}"; then
    echo "lint-tree: an image carries metadata; strip it with: python3 -I -B tools/guide-pictures.py --strip <image>"
    status=1
fi

if [ "$status" -eq 0 ]; then
    echo "lint-tree: ok"
fi
exit "$status"
