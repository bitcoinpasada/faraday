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
# Linked by LLD (F297), with uxtheme.dll and dwmapi.dll delay-loaded
# (faraday/shells/window/build.rs makes their import libraries), the
# load configuration's DependentLoadFlags set by the linker too, and GNU
# ld's 2 MiB reserve for the main thread's stack, which LLD halves.
RUSTFLAGS="$RUSTFLAGS -C link-arg=-fuse-ld=lld -C link-arg=-Wl,--delayload=uxtheme.dll"
RUSTFLAGS="$RUSTFLAGS -C link-arg=-Wl,--delayload=dwmapi.dll -C link-arg=-Wl,--stack,2097152"
RUSTFLAGS="$RUSTFLAGS -C link-arg=-Wl,--dependent-load-flag=0x800"
export RUSTFLAGS
# The symbols are not needed to run it, and are most of its size.
export CARGO_PROFILE_RELEASE_STRIP=symbols

cargo build --release --locked --target "$target" -p "$package" --bin "$bin"
install -m 0644 "$CARGO_TARGET_DIR/$target/release/$bin.exe" "/out/$bin.exe"
exe="/out/$bin.exe"

# Only Windows' own libraries, from a fixed list: nothing of mingw-w64's
# (libgcc_s, libwinpthread, libstdc++) may be needed beside it, and a new
# import is a decision, made here. Windows looks for a library that is not
# one of its KnownDLLs in the program's own folder first, and a folder
# such as Downloads holds files from anywhere, so the list holds no library
# that is not a KnownDLL and is mapped at start: none is. uxtheme.dll and
# dwmapi.dll are delay-loaded instead (below), loaded at first use, after
# main's SetDefaultDllDirectories, so from System32 only; a missing one
# stops the program. What libraries load in turn is not checked here.
allowed='advapi32 api-ms-win-core-synch-l1-2-0 bcryptprimitives gdi32 imm32 kernel32 msvcrt ntdll ole32 oleaut32 setupapi shell32 user32'
sections="$(x86_64-w64-mingw32-objdump -h "$exe")"
base="$(x86_64-w64-mingw32-objdump -p "$exe" | sed -n 's/^ImageBase\s*//p')"
dumped="$(x86_64-w64-mingw32-objdump -p "$exe")"
dlls="$(printf '%s\n' "$dumped" | sed -n 's/^\s*DLL Name: //p' | sort -u)"
echo "$bin.exe imports: $(echo "$dlls" | tr '\n' ' ')"
bad="$(echo "$dlls" | grep -iE '^(lib|mingw)' || true)"
[ -z "$bad" ] || die "$bin.exe needs a library Windows does not have: $bad"
for dll in $dlls; do
    name="$(echo "${dll%.dll}" | tr 'A-Z' 'a-z')"
    case " $allowed " in *" $name "*) ;; *) die "$bin.exe imports $dll, which is not in the list of allowed libraries" ;; esac
done
# The delay-loaded libraries are exactly dwmapi.dll and uxtheme.dll, and
# each one's import address table and module handle slot, which mingw-w64's
# delay-load helper writes at first use, lie in a writable section. (The
# readobj output is read into a variable first: a failure there stops the
# build, where a failing command inside a for list would run the loop zero
# times.)
readobj="$(llvm-readobj --coff-imports "$exe")"
delayed="$(printf '%s\n' "$readobj" | awk '/^DelayImport/ {d = 1} /^Import \{/ {d = 0} d && $1 == "Name:" {print $2}' | tr 'A-Z' 'a-z' | sort -u | tr '\n' ' ')"
[ "$delayed" = "dwmapi.dll uxtheme.dll " ] || die "$bin.exe delay-loads: ${delayed:-nothing}, not dwmapi.dll and uxtheme.dll"
slots="$(printf '%s\n' "$readobj" | awk '/^DelayImport/ {d = 1} /^Import \{/ {d = 0} d && ($1 == "ModuleHandle:" || $1 == "ImportAddressTable:") {sub(/^0x/, "", $2); print $2}')"
[ "$(printf '%s\n' "$slots" | grep -c . || true)" -eq 4 ] || die "$bin.exe's delay imports do not show two module handles and two address tables"
for slot in $slots; do
    w="$(printf '%s\n' "$sections" | while read -r idx name size vma rest; do
        case "$idx" in [0-9]*) ;; *) continue ;; esac
        read -r flags
        r=$((16#$vma - 16#$base))
        if [ $((16#$slot)) -ge $r ] && [ $((16#$slot)) -lt $((r + 16#$size)) ]; then
            case "$flags" in *READONLY*) echo "ro $name" ;; *) echo "w $name" ;; esac
        fi
    done)"
    case "$w" in "w "*) ;; *) die "$bin.exe's delay-import slot at 0x$slot is not in a writable section" ;; esac
done
echo "$bin.exe delay-loads: $delayed"

# DependentLoadFlags is LOAD_LIBRARY_SEARCH_SYSTEM32 (0x800). It does not
# apply to the .exe's own imports (KnownDLLs and API sets only, above); it
# is kept for what the libraries loaded later load in turn. Read from
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
echo "$bin.exe: load configuration Size 0x50, DependentLoadFlags LOAD_LIBRARY_SEARCH_SYSTEM32"

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
