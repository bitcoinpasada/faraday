#!/usr/bin/env bash
# Prints the Faraday build ID: the string that goes after `-faraday-` in
# the kernel release string (docs/DECISIONS.md F5).
#
#   0.2.0                                a release
#   0.1.0+ef24784b48d8.test              a test build (the default)
#   0.1.0+ef24784b48d8.dev               a dev build
#
# A tree that differs from its commit inserts `.dirty-XXXXXXXX` before the
# kind, e.g. `0.1.0+ef24784b48d8.dirty-3fa9c1d2.test`.
#
# Runs from the repository root. Inputs, from the environment:
#
#   FARADAY_RELEASE   0/1, default 0. 1 prints the version alone, and
#                     refuses a dev build, a version override or a dirty
#                     tree.
#   FARADAY_DEV       0/1, default 0. 1 appends `.dev` instead of `.test`.
#   FARADAY_VERSION   overrides the `version = "…"` read from
#                     faraday/faraday-core/Cargo.toml; must look like
#                     X.Y.Z. Empty (the default) uses the crate's own
#                     version.
set -euo pipefail

release="${FARADAY_RELEASE:-0}"
dev="${FARADAY_DEV:-0}"
version_override="${FARADAY_VERSION:-}"

if [ -n "$version_override" ] && ! [[ "$version_override" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "build-id.sh: FARADAY_VERSION must look like X.Y.Z, got '$version_override'" >&2
    exit 1
fi

if [ "$release" = "1" ] && [ "$dev" != "0" ]; then
    echo "build-id.sh: a release is not a dev image" >&2
    exit 1
fi
if [ "$release" = "1" ] && [ -n "$version_override" ]; then
    echo "build-id.sh: a release carries its own version" >&2
    exit 1
fi

version="$version_override"
if [ -z "$version" ]; then
    version="$(sed -n 's/^version = "\(.*\)"$/\1/p' faraday/faraday-core/Cargo.toml | head -1)"
fi

dirty=0
git diff --quiet HEAD -- || dirty=1
untracked=0
if [ -n "$(git ls-files -o --exclude-standard)" ]; then
    untracked=1
fi
if [ "$dirty" = "1" ] || [ "$untracked" = "1" ]; then
    dirty=1
fi

if [ "$release" = "1" ] && [ "$dirty" = "1" ]; then
    echo "build-id.sh: a release is built from a committed tree, and this one is dirty" >&2
    git status --short >&2
    exit 1
fi

if [ "$release" = "1" ]; then
    printf '%s\n' "$version"
    exit 0
fi

commit="$(git rev-parse --short=12 HEAD)"
out="$version+$commit"
if [ "$dirty" = "1" ]; then
    fp="$(
        {
            git diff HEAD --binary
            git ls-files -o --exclude-standard -z | LC_ALL=C sort -z | while IFS= read -r -d '' f; do
                printf '%s\0' "$f"
                cat "$f"
            done
        } | sha256sum | cut -c1-8
    )"
    out="$out.dirty-$fp"
fi
if [ "$dev" != "0" ]; then
    out="$out.dev"
else
    out="$out.test"
fi
printf '%s\n' "$out"
