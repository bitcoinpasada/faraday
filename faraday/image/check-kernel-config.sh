#!/bin/bash
# The kernel this device boots is the drivers it needs and nothing else.
# This script is what makes that a rule the build enforces rather than a
# claim in a document.
#
#   check-kernel-config.sh <path to a kernel .config> <part directory>...
#
# build.sh runs it after Buildroot has generated the kernel's .config and
# before the kernel is built, with the same part directories the rest of
# the image is assembled from — common, the board, the panel and, in a dev
# image, the variant — so a forbidden option fails the build in seconds
# instead of appearing in an image nobody reads the config of.
#
# Each part may hold two files:
#
#   kernel.forbidden  options that must not be built: an option set to y
#                     or m here fails
#   kernel.required   options the device does not work without: an option
#                     missing or unset here fails
#   kernel.cmdline    words the kernel's built-in command line
#                     (CONFIG_CMDLINE) must carry, for a board whose
#                     command line is compiled in; the format is the same
#                     (a word, then the reason). A word that is missing
#                     fails, and so does a config with no CONFIG_CMDLINE.
#
# A line in either is a symbol without its CONFIG_ prefix, then the reason
# it is in the list. The reasons are the point of the files; an entry
# without one is a rule nobody can check later. Blank lines and lines
# starting with # are comments.
#
# The lists are per part because the devices differ. A board with a
# soldered-on touch panel and one USB port for a camera forbids HID and
# USB storage; a laptop booted from a USB stick requires both. That is two
# board files and not one shared list with exceptions in it: a list with an
# exception stops being readable as the rule it states.
set -euo pipefail

config="${1:-}"
[ -n "$config" ] || { echo "usage: check-kernel-config.sh <.config> <part dir>..." >&2; exit 2; }
[ -f "$config" ] || { echo "no such kernel config: $config" >&2; exit 2; }
shift
[ "$#" -gt 0 ] || { echo "check-kernel-config.sh: no part directories given" >&2; exit 2; }

forbidden=()
required=()
cmdline_words=()

read_list() {
    local file="$1" name="$2" sym reason line
    [ -f "$file" ] || return 0
    while IFS= read -r line; do
        case "$line" in ''|'#'*) continue ;; esac
        sym="${line%% *}"
        reason="$(echo "${line#"$sym"}" | sed 's/^ *//')"
        # A symbol is upper-case letters, digits and underscores, without
        # CONFIG_: anything else (a prefix, a value, a typo with a stray
        # character) would match nothing in the config and so pass.
        if [ "$name" != cmdline ] && ! [[ "$sym" =~ ^[A-Z0-9_]+$ && "$sym" != CONFIG_* ]]; then
            echo "$file: $sym is not a kernel option name (no CONFIG_ prefix, no value)" >&2
            exit 2
        fi
        if [ -z "$reason" ]; then
            echo "$file: $sym has no reason" >&2
            exit 2
        fi
        case "$name" in
            forbidden) forbidden+=("$sym|$reason|$file") ;;
            cmdline) cmdline_words+=("$sym|$reason|$file") ;;
            *) required+=("$sym|$reason|$file") ;;
        esac
    done < "$file"
}

for part in "$@"; do
    read_list "$part/kernel.forbidden" forbidden
    read_list "$part/kernel.required" required
    read_list "$part/kernel.cmdline" cmdline
done

fail=0

# A later part may require what an earlier one forbids — a board with a
# keyboard requires the HID layer the common list has no opinion on — but
# the same symbol in both lists at once is two rules that cannot both
# hold, and an image built from them would be whichever ran last.
for r in ${required[@]+"${required[@]}"}; do
    rsym="${r%%|*}"
    for f in ${forbidden[@]+"${forbidden[@]}"}; do
        if [ "${f%%|*}" = "$rsym" ]; then
            echo "kernel config: CONFIG_$rsym is both required and forbidden:" >&2
            echo "               ${r##*|} and ${f##*|}" >&2
            fail=1
        fi
    done
done

for entry in ${forbidden[@]+"${forbidden[@]}"}; do
    sym="${entry%%|*}"
    rest="${entry#*|}"
    line="$(grep -E "^CONFIG_$sym=[ym]\$" "$config" || true)"
    # An option the config does not mention at all, set or unset, is
    # either misspelt or not offered on this board (the kernel leaves out
    # an option whose dependencies are not met): said, so that a typo
    # does not stay a rule that checks nothing.
    if ! grep -qE "^(CONFIG_$sym=|# CONFIG_$sym is not set\$)" "$config"; then
        echo "kernel config: note: forbidden CONFIG_$sym is not in this config at all (misspelt, or not offered here)" >&2
    fi
    if [ -n "$line" ]; then
        echo "kernel config: $line" >&2
        echo "               must not be built: ${rest%%|*}" >&2
        echo "               (${rest##*|})" >&2
        fail=1
    fi
done

for entry in ${required[@]+"${required[@]}"}; do
    sym="${entry%%|*}"
    rest="${entry#*|}"
    if ! grep -qE "^CONFIG_$sym=y\$" "$config"; then
        echo "kernel config: CONFIG_$sym is not set" >&2
        echo "               must be built: ${rest%%|*}" >&2
        echo "               (${rest##*|})" >&2
        fail=1
    fi
done

if [ "${#cmdline_words[@]}" -gt 0 ]; then
    built_in="$(sed -n 's/^CONFIG_CMDLINE="\(.*\)"$/\1/p' "$config")"
    for entry in "${cmdline_words[@]}"; do
        word="${entry%%|*}"
        rest="${entry#*|}"
        case " $built_in " in
            *" $word "*) ;;
            *)
                echo "kernel config: CONFIG_CMDLINE lacks $word" >&2
                echo "               must be on the command line: ${rest%%|*}" >&2
                echo "               (${rest##*|})" >&2
                fail=1
                ;;
        esac
    done
fi

if [ "$fail" != 0 ]; then
    echo "kernel config: $config does not describe this device" >&2
    exit 1
fi

echo "    kernel config: ${#forbidden[@]} options out, ${#required[@]} in, ${#cmdline_words[@]} command-line words, $(grep -c '=y$' "$config") built in"
