#!/usr/bin/env bash
# Builds Faraday Companion's unsigned release APK inside the container
# (faraday/build/android/Containerfile) and checks it. `just
# companion-release-apk` starts it; there is nothing here to run on the host.
#
#   build.sh [apk]                    the release APK, into /out
#   build.sh verification-metadata    Gradle's checksums of every download of
#                                     that build, into /out/verification-metadata.xml
#                                     (`just companion-gradle-verification`
#                                     copies it into the shell's gradle/)
#
# Container paths: /src is the checkout (read-only), /out and /build are
# bind mounts, /work the copy Gradle builds in (emptied first), /cargo the
# registry cache and /gradle Gradle's home. SOURCE_DATE_EPOCH comes from the
# host (the commit's time).
#
# The APK is /out/faraday-companion-X.Y.Z-android-unsigned.apk, X.Y.Z being
# the version in faraday/faraday-companion/Cargo.toml (this script never
# changes it), and /out/jniLibs/<abi>/libfaraday_companion.so beside it, the
# libraries it packages, for comparing two builds.
set -euo pipefail

what="${1:-apk}"
case "$what" in apk | verification-metadata) ;; *) printf 'build.sh: unknown action %s\n' "$what" >&2; exit 1 ;; esac
die() { printf 'build.sh: %s\n' "$1" >&2; exit 1; }

cd /src
want="$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)"
have="$(rustc --version | cut -d' ' -f2)"
[ "$want" = "$have" ] || die "rust-toolchain.toml says $want, the container has $have"
: "${SOURCE_DATE_EPOCH:?SOURCE_DATE_EPOCH is not set}"
export SOURCE_DATE_EPOCH

version="$(sed -n 's/^version = "\([0-9]*\.[0-9]*\.[0-9]*\)"$/\1/p' faraday/faraday-companion/Cargo.toml | head -1)"
[ -n "$version" ] || die "no version in faraday/faraday-companion/Cargo.toml"
apk_name="faraday-companion-$version-android-unsigned.apk"

# The core for both ABIs, linked by the NDK's clang for API 26 (the app's
# minSdk). No local path in the library: the same rewriting as every other
# build of something published (faraday/image/path-remap.sh), with this
# container's home (/root), cargo home and source tree; and the target
# directory, where build scripts write what is compiled in.
export HOME=/root
. /src/faraday/image/path-remap.sh
export RUSTFLAGS="$RUSTFLAGS --remap-path-prefix=$CARGO_TARGET_DIR=/target"
# The symbols are not needed to run it, and are most of its size.
export CARGO_PROFILE_RELEASE_STRIP=symbols
tc="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin"
export CC_aarch64_linux_android="$tc/aarch64-linux-android26-clang" \
    AR_aarch64_linux_android="$tc/llvm-ar" \
    CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$tc/aarch64-linux-android26-clang" \
    CC_x86_64_linux_android="$tc/x86_64-linux-android26-clang" \
    AR_x86_64_linux_android="$tc/llvm-ar" \
    CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$tc/x86_64-linux-android26-clang"

for pair in arm64-v8a:aarch64-linux-android x86_64:x86_64-linux-android; do
    triple="${pair#*:}"
    cargo build --release --locked --target "$triple" -p faraday-companion-ffi
done

# Gradle builds in a copy of the shell, so /src stays read-only and nothing
# a developer left in the checkout (build/, .gradle/, jniLibs/) is in it.
tree=/work/tree
find /work -mindepth 1 -maxdepth 1 -exec rm -rf {} +
mkdir -p "$tree/faraday/shells" "$tree/faraday/faraday-companion"
tar -C faraday/shells --exclude=./android/build --exclude=./android/app/build \
    --exclude=./android/.gradle --exclude=./android/.kotlin \
    --exclude=./android/app/src/main/jniLibs -cf - android | tar -C "$tree/faraday/shells" -xf -
cp faraday/faraday-companion/Cargo.toml "$tree/faraday/faraday-companion/Cargo.toml"
for pair in arm64-v8a:aarch64-linux-android x86_64:x86_64-linux-android; do
    abi="${pair%%:*}"
    triple="${pair#*:}"
    mkdir -p "$tree/faraday/shells/android/app/src/main/jniLibs/$abi" "/out/jniLibs/$abi"
    cp "$CARGO_TARGET_DIR/$triple/release/libfaraday_companion_ffi.so" \
        "$tree/faraday/shells/android/app/src/main/jniLibs/$abi/libfaraday_companion.so"
    cp "$CARGO_TARGET_DIR/$triple/release/libfaraday_companion_ffi.so" "/out/jniLibs/$abi/libfaraday_companion.so"
done

cd "$tree/faraday/shells/android"
gradle=(./gradlew --no-daemon --no-build-cache --console=plain -q)
if [ "$what" = verification-metadata ]; then
    "${gradle[@]}" --write-verification-metadata sha256 clean assembleRelease
    cp gradle/verification-metadata.xml /out/verification-metadata.xml
    echo "Wrote /out/verification-metadata.xml"
    exit 0
fi
"${gradle[@]}" clean assembleRelease
built=app/build/outputs/apk/release/app-release-unsigned.apk
[ -f "$built" ] || die "Gradle left no $built"
install -m 0644 "$built" "/out/$apk_name"

# 16 KiB page alignment of the libraries and 4-byte alignment of the rest.
zipalign -c -P 16 4 "/out/$apk_name" || die "$apk_name is not aligned (zipalign -c -P 16 4)"
echo "zipalign -c -P 16 4: aligned"

# Nothing of the container in the libraries or the code: the same words
# stage-release.sh looks for in a published file, for this container's paths.
check="$(mktemp -d)"
unzip -q "/out/$apk_name" -d "$check"
for f in "$check"/lib/*/*.so "$check"/classes*.dex; do
    # (grep -m1 would end the pipe early, and pipefail would take that for a
    # failure.)
    hit="$(strings -a "$f" | grep -E '(^|[^[:alnum:]_.-])/(src|work|root|gradle|opt)/|/home/|/Users/' | head -1 || true)"
    [ -z "$hit" ] || die "${f#"$check"/} carries a path: $hit"
done
rm -rf "$check"

# The size is printed, not limited: Faraday signs an APK of any size
# (docs/APK-SIGNING.md §9, docs/LARGE-FILES.md).
printf 'size: %s bytes\n' "$(stat -c %s "/out/$apk_name")"
printf 'sha256: %s  %s\n' "$(sha256sum < "/out/$apk_name" | cut -d' ' -f1)" "$apk_name"
