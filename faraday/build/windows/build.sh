#!/usr/bin/env bash
# Builds a Windows program (the Starter, or the desktop app) inside the
# container (faraday/build/windows/Containerfile) for x86_64-pc-windows-gnu,
# and checks it. `just faraday-windows-bin` starts it; there is nothing
# here to run on the host.
#
#   build.sh [build [PACKAGE BIN]]  the release build, into /out/BIN.exe
#   build.sh clippy [PACKAGE]       clippy for the Windows target, warnings
#                                   as errors
#
# PACKAGE and BIN default to the Starter: faraday-starter-shell and
# faraday-starter.
#
# Container paths: /src is the checkout (read-only), /out and /build are
# bind mounts, /cargo the registry cache. FARADAY_BUILD comes from the
# host (faraday/image/build-id.sh).
set -euo pipefail

target=x86_64-pc-windows-gnu
die() { printf 'build.sh: %s\n' "$1" >&2; exit 1; }

cd /src
want="$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)"
have="$(rustc --version | cut -d' ' -f2)"
[ "$want" = "$have" ] || die "rust-toolchain.toml says $want, the container has $have"

what="${1:-build}"
package="${2:-faraday-starter-shell}"
bin="${3:-faraday-starter}"
case "$what" in build | clippy) ;; *) die "unknown action: $what" ;; esac

if [ "$what" = clippy ]; then
    exec cargo clippy --locked --target "$target" -p "$package" -- -D warnings
fi

: "${FARADAY_BUILD:?FARADAY_BUILD is not set}"
export FARADAY_BUILD

# No local path in the binary: the same rewriting as every other build of
# something published (faraday/image/path-remap.sh), with this container's
# home (/root), cargo home and source tree; and the target directory,
# where build scripts write what is compiled in.
export HOME=/root RUST_TARGET="$target"
. /src/faraday/image/path-remap.sh
export RUSTFLAGS="$RUSTFLAGS --remap-path-prefix=$CARGO_TARGET_DIR=/target"
# The symbols are not needed to run it, and are most of its size.
export CARGO_PROFILE_RELEASE_STRIP=symbols

cargo build --release --locked --target "$target" -p "$package" --bin "$bin"
install -m 0644 "$CARGO_TARGET_DIR/$target/release/$bin.exe" "/out/$bin.exe"
exe="/out/$bin.exe"

# The load configuration (faraday/build/windows/resources.rs) is in the
# .exe as the section .lcfg; mingw-w64's linker does not point the image's
# load configuration directory (entry 10 of the data directory, in the
# optional header after the PE signature and the file header) at it, so
# this does: the section's RVA and size, eight bytes. The same bytes every
# time; the check below reads the result back. The entry must be empty
# before: a linker that starts writing one is a change to look at here.
le32() { printf "$(printf '\\x%02x\\x%02x\\x%02x\\x%02x' $(($1 & 255)) $((($1 >> 8) & 255)) $((($1 >> 16) & 255)) $((($1 >> 24) & 255)))"; }
sections="$(x86_64-w64-mingw32-objdump -h "$exe")"
base="$(x86_64-w64-mingw32-objdump -p "$exe" | sed -n 's/^ImageBase\s*//p')"
size="$(printf '%s\n' "$sections" | awk '$2 == ".lcfg" { print $3 }')"
vma="$(printf '%s\n' "$sections" | awk '$2 == ".lcfg" { print $4 }')"
[ -n "$base" ] && [ -n "$vma" ] || die "$bin.exe has no .lcfg section"
nt="$(od -An -tu4 -j60 -N4 "$exe" | tr -d ' ')"
entry=$((nt + 24 + 112 + 10 * 8))
[ "$(od -An -tu4 -j$entry -N8 "$exe" | tr -s ' ' | tr -d '\n')" = " 0 0" ] \
    || die "$bin.exe already has a load configuration directory"
{ le32 $((16#$vma - 16#$base)); le32 $((16#$size)); } \
    | dd of="$exe" bs=1 seek=$entry conv=notrunc status=none

# Only Windows' own libraries, from a fixed list: nothing of mingw-w64's
# (libgcc_s, libwinpthread, libstdc++) may be needed beside it, and a new
# import is a decision, made here. Windows looks for a library that is not
# one of its KnownDLLs in the program's own folder first, and a folder
# such as Downloads holds files from anywhere. The load configuration set
# above makes Windows look in System32 only for every library below (what
# they load in turn is main's SetDefaultDllDirectories, not checked here),
# and the check after the list reads that back.
allowed='advapi32 api-ms-win-core-synch-l1-2-0 bcryptprimitives dwmapi gdi32 imm32 kernel32 msvcrt ntdll ole32 oleaut32 propsys setupapi shell32 user32 uxtheme'
dumped="$(x86_64-w64-mingw32-objdump -p "$exe")"
dlls="$(printf '%s\n' "$dumped" | sed -n 's/^\s*DLL Name: //p' | sort -u)"
echo "$bin.exe imports: $(echo "$dlls" | tr '\n' ' ')"
bad="$(echo "$dlls" | grep -iE '^(lib|mingw)' || true)"
[ -z "$bad" ] || die "$bin.exe needs a library Windows does not have: $bad"
for dll in $dlls; do
    name="$(echo "${dll%.dll}" | tr 'A-Z' 'a-z')"
    case " $allowed " in *" $name "*) ;; *) die "$bin.exe imports $dll, which is not in the list of allowed libraries" ;; esac
done
# A delay-loaded library is searched for when first used, not by the
# list above: none is allowed. (grep -c, not -q, for the pipefail reason
# below.)
[ "$(printf '%s\n' "$dumped" | grep -c '^Entry d 0000000000000000 00000000 ' || true)" -eq 1 ] \
    || die "$bin.exe delay-loads a library"

# DependentLoadFlags is LOAD_LIBRARY_SEARCH_SYSTEM32 (0x800): read from
# the load configuration (the flags at offset 0x4e), whose Size (offset 0)
# must reach 0x50, the end of the flags, and so must the directory's size.
cfg="$(printf '%s\n' "$dumped" | sed -n 's/^Entry a \([0-9a-f]*\) .*Load Configuration.*/\1/p')"
[ -n "$cfg" ] && [ "$((16#$cfg))" -ne 0 ] || die "$bin.exe has no load configuration"
dirsize="$(printf '%s\n' "$dumped" | sed -n 's/^Entry a [0-9a-f]* \([0-9a-f]*\) .*Load Configuration.*/\1/p')"
[ -n "$dirsize" ] && [ "$((16#$dirsize))" -ge 80 ] || die "$bin.exe's load configuration directory is under 0x50 bytes"
at=$((16#$base + 16#$cfg + 16#4e))
flags="$(x86_64-w64-mingw32-objdump -s --start-address="$at" --stop-address=$((at + 2)) "$exe" \
    | sed -n 's/^ *[0-9a-f]* \([0-9a-f]\{4\}\) .*/\1/p')"
at=$((16#$base + 16#$cfg))
structsize="$(x86_64-w64-mingw32-objdump -s --start-address="$at" --stop-address=$((at + 4)) "$exe" \
    | sed -n 's/^ *[0-9a-f]* \([0-9a-f]\{8\}\) .*/\1/p')"
[ "$structsize" = 50000000 ] || die "$bin.exe's load configuration Size is not 0x50 (found: ${structsize:-none})"
[ "$flags" = 0008 ] || die "$bin.exe's DependentLoadFlags is not LOAD_LIBRARY_SEARCH_SYSTEM32 (found: ${flags:-none})"
echo "$bin.exe: libraries are loaded from System32 only"

# Its resources: the manifest asks for no elevation.
# (Not grep -q: it would end the pipe early, and pipefail would take that
# for a failure.)
[ "$(strings -a "$exe" | grep -c 'level="asInvoker"' || true)" -gt 0 ] || die "$bin.exe carries no asInvoker manifest"

# Nothing of the container's own paths, nor a home folder, may be left in
# it. (grep -m1 would end the pipe early, and pipefail would take that for
# a failure.) Both 8-bit and 16-bit little-endian strings.
hit="$( { strings -a "$exe"; strings -a -el "$exe"; } \
    | grep -E '(^|[^[:alnum:]_.-])/(src|root|payload)/|/build/target|/cargo/registry|/home/|/Users/|[A-Za-z]:\\Users\\' \
    | head -1 || true)"
[ -z "$hit" ] || die "$bin.exe carries a path: $hit"
echo "$bin.exe: $(stat -c %s "$exe") bytes, no path in its strings"
